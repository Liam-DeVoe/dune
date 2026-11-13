//! The ship-and-move forms: for every fixture state and every state of the sample bot games
//! where one of the 25 event types is applicable, the form's options equal the engine's
//! helpers, for the default draft and for the
//! drafts a player makes on the way (a source picked, a destination pinned, a shipment type or
//! No-Field chosen). The draft query path (JSON in, options out) is checked on the fixtures.
//!
//! The options carry the draft after the form's adjustments (`check_adjustments`): drafts with
//! inputs that no longer apply come back with those inputs dropped, and the validation is the
//! adjusted draft's. The expected adjustments are written out here, not by calling the view's
//! `normalize`.

mod common;

use common::{load, viewer_of};
use treachery_engine::compat::cs::Shared;
use treachery_engine::events::GameEventType;
use treachery_engine::messages::expression::ExpressionElement;
use treachery_engine::json;
use treachery_engine::compat::net_enum::NetEnum;
use treachery_engine::game::Participation;
use treachery_engine::prelude::*;
use treachery_view::actions::ship_move::{default_draft, options, options_and_validation};
use treachery_view::build::{event, validate};
use treachery_view::describe::{default_describer, Describer};
use treachery_view::replica::ViewReplica;
use treachery_view::schema::*;
use treachery_view::shield::hero_ref;

const TYPES: &[GameEventType] = &[
    GameEventType::AmbassadorActivated,
    GameEventType::AmbassadorPlaced,
    GameEventType::BeneGesseritAccompanies,
    GameEventType::BeneGesseritFlip,
    GameEventType::ChoamMovePrevention,
    GameEventType::ChoamRemoveForce,
    GameEventType::Caravan,
    GameEventType::DiscoveryEntered,
    GameEventType::DiscoveryRevealed,
    GameEventType::FlightUsed,
    GameEventType::KarmaHmsMovement,
    GameEventType::KarmaShipmentPrevention,
    GameEventType::Move,
    GameEventType::PerformBeneGesseritPlacement,
    GameEventType::PerformMoritaniSetup,
    GameEventType::PerformHmsMovement,
    GameEventType::PerformHmsPlacement,
    GameEventType::PerformFremenSetup,
    GameEventType::SetShipmentPermission,
    GameEventType::Shipment,
    GameEventType::TerrorPlanted,
    GameEventType::TerrorRevealed,
    GameEventType::TestingStationUsed,
    GameEventType::FremenRidesMonster,
    GameEventType::FremenSentMonster,
];

fn ids(ls: impl IntoIterator<Item = Location>) -> Vec<i32> {
    ls.into_iter().map(|l| l.id()).collect()
}

fn sectors(ts: impl IntoIterator<Item = Territory>) -> Vec<i32> {
    ts.into_iter().flat_map(|t| t.locations().iter().map(|l| l.id())).collect()
}

fn names<T: ToString>(v: impl IntoIterator<Item = T>) -> Vec<String> {
    v.into_iter().map(|x| x.to_string()).collect()
}

fn opts(game: &Game, p: PlayerIndex, draft: &GameEvent, d: &dyn Describer) -> ActionOptions {
    options(game, p, draft, d).expect("a ship-and-move type").expect("the builder does not throw")
}

fn forces_at(game: &Game, p: PlayerIndex, l: Location) -> ForcesAt {
    ForcesAt { location: l.id(), forces: game.player(p).forces_in(Some(l)), special_forces: game.player(p).special_forces_in(Some(l)) }
}

/// What was checked, to make sure the corpus reaches the forms.
#[derive(Default)]
struct Seen(std::collections::BTreeMap<&'static str, usize>);

impl Seen {
    fn add(&mut self, t: GameEventType) {
        *self.0.entry(t.name()).or_default() += 1;
    }
}

/// Checks the form of `t` for player `p` in this state: default draft and dependent drafts.
fn check(game: &Game, p: PlayerIndex, t: GameEventType, d: &dyn Describer, seen: &mut Seen) {
    let f = game.player(p).faction;
    let draft = default_draft(game, p, f, t).unwrap_or_else(|| panic!("{} has a default draft", t.name()));
    let o = opts(game, p, &draft, d);
    seen.add(t);
    // The default draft is already as the component shows it.
    assert_eq!(json::serialize(&adjusted(game, p, &draft, d)), json::serialize(&draft), "{}: the default draft is stable", t.name());
    check_adjustments(game, p, &draft, d);
    let pl = game.player(p);
    match (&draft, o) {
        (GameEvent::Shipment(s), ActionOptions::Shipment(o)) => check_shipment(game, p, s, &o, d),
        (GameEvent::Move(_), ActionOptions::Move(o)) => {
            assert_eq!(o.inform_about_caravan, pl.has_treachery_card_type(TreacheryCardType::Caravan));
            check_placement(game, p, &o, t);
        }
        (GameEvent::Caravan(_), ActionOptions::Caravan(o)) => check_placement(game, p, &o, t),
        (GameEvent::FremenRidesMonster(_), ActionOptions::FremenRidesMonster(o)) => check_ride(game, p, &o, d),
        (GameEvent::PerformFremenSetup(_), ActionOptions::PerformFremenSetup(o)) => {
            let m = &game.map;
            let mut expected = vec![m.sietch_tabr().id()];
            expected.extend(sectors([m.false_wall_south(), m.false_wall_west()]));
            assert_eq!(o.targets, expected);
            assert_eq!(o.max_total, 10);
            assert_eq!(o.may_stage_special, pl.has_special_forces(game));
            assert_eq!((o.forces_in_reserve, o.special_forces_in_reserve), (pl.forces_in_reserve(), pl.special_forces_in_reserve()));
        }
        (GameEvent::DiscoveryEntered(_), ActionOptions::DiscoveryEntered(o)) => {
            let targets: Vec<DiscoveredLocation> = DiscoveryEntered::valid_targets(game, p).collect();
            assert_eq!(o.targets, targets.iter().map(|ds| ds.location().id()).collect::<Vec<_>>());
            assert!(o.sources.is_empty(), "no sources before a stronghold is picked");
            for ds in targets {
                let mut e = DiscoveryEntered::new(game, f);
                e.set_to(game, Some(ds.location()));
                let ActionOptions::DiscoveryEntered(o) = opts(game, p, &event(e), d) else { panic!() };
                let expected: Vec<ForcesAt> = DiscoveryEntered::valid_sources(game, p, ds).map(|l| forces_at(game, p, l)).collect();
                assert_eq!(o.sources, expected);
            }
        }
        (GameEvent::AmbassadorActivated(_), ActionOptions::AmbassadorActivated(o)) => {
            assert_eq!(o.ambassador, AmbassadorActivated::get_ambassador(game).to_string());
            assert_eq!(o.victim, AmbassadorActivated::get_victim(game).to_string());
            assert_eq!(o.bene_gesserit_ambassadors, names(AmbassadorActivated::get_valid_bene_gesserit_ambassadors(game)));
            assert_eq!(o.choam_cards, AmbassadorActivated::get_valid_choam_cards(game, p).iter().map(|c| c.id()).collect::<Vec<_>>());
            assert_eq!(o.ixian_cards, AmbassadorActivated::get_valid_ixian_cards(game, p).iter().map(|c| c.id()).collect::<Vec<_>>());
            assert_eq!(o.vidal_can_be_taken, AmbassadorActivated::vidal_can_be_taken(game, p));
            assert_eq!(o.alliance_can_be_offered, AmbassadorActivated::alliance_can_be_offered(game, p));
            assert_eq!(o.vidal_can_be_offered_to_new_ally, AmbassadorActivated::vidal_can_be_offered_to_new_ally(game, p));
            assert_eq!(o.fremen_targets, ids(AmbassadorActivated::valid_fremen_targets(game, p)));
            assert_eq!(o.guild_targets, ids(AmbassadorActivated::valid_guild_targets(game, p)));
            assert_eq!(o.guild_max_forces, AmbassadorActivated::valid_guild_max_forces(game, p));
            assert_eq!(o.tleilaxu_max_amount, AmbassadorActivated::valid_tleilaxu_max_amount(game, p));
            assert_eq!(o.may_pass, AmbassadorActivated::may_pass(game, p));
            let heroes = AmbassadorActivated::valid_tleilaxu_heroes(game, p);
            assert_eq!(o.tleilaxu_heroes.iter().map(|h| h.id).collect::<Vec<_>>(), heroes.iter().map(|h| h.id()).collect::<Vec<_>>());
            assert!(!o.may_assign_skill, "no hero picked yet");
            for h in heroes {
                let mut e = AmbassadorActivated::new(game, f);
                e.set_tleilaxu_hero(Some(h));
                let ActionOptions::AmbassadorActivated(o) = opts(game, p, &event(e), d) else { panic!() };
                assert_eq!(o.may_assign_skill, Revival::may_assign_skill(game, p, Some(h)));
                assert!(o.tleilaxu_heroes.iter().any(|x| x.hero == hero_ref(h)));
            }
        }
        (GameEvent::AmbassadorPlaced(e), ActionOptions::AmbassadorPlaced(o)) => {
            let valid = AmbassadorPlaced::valid_ambassadors(game, p);
            assert_eq!(o.ambassadors, names(valid.clone()));
            assert_eq!(e.ambassador, valid.first().copied().unwrap_or(Ambassador::None), "the required picker starts on the first ambassador");
            assert_eq!(o.targets, sectors(AmbassadorPlaced::valid_strongholds(game, p)));
            assert_eq!(o.cost, game.ambassadors_placed_this_turn + 1);
        }
        (GameEvent::BeneGesseritAccompanies(_), ActionOptions::BeneGesseritAccompanies(o)) => {
            let targets = BeneGesseritAccompanies::valid_targets(game, p);
            assert_eq!(o.targets, ids(targets.clone()));
            assert!(!o.may_send_extra_advisor);
            for l in targets {
                let mut e = BeneGesseritAccompanies::new(game, f);
                e.set_location(game, Some(l));
                e.accompanies = true;
                let ActionOptions::BeneGesseritAccompanies(o) = opts(game, p, &event(e), d) else { panic!() };
                assert_eq!(o.may_send_extra_advisor, BeneGesseritAccompanies::may_send_extra_advisor(game, p, l));
            }
        }
        (GameEvent::BeneGesseritFlip(_), ActionOptions::BeneGesseritFlip(o)) => assert_eq!(o.territory, BeneGesseritFlip::get_territory(game).id()),
        (GameEvent::ChoamMovePrevention(_), ActionOptions::ChoamMovePrevention(o)) => {
            assert_eq!(o.card, ChoamMovePrevention::card_to_use(game, p).map(|c| c.id()));
            assert_eq!(o.targets, sectors(ChoamMovePrevention::valid_territories(game, p)));
        }
        (GameEvent::ChoamRemoveForce(_), ActionOptions::ChoamRemoveForce(o)) => {
            let locations: Vec<Location> = ChoamRemoveForce::valid_locations(game).collect();
            assert_eq!(o.targets, ids(locations.clone()));
            for l in locations {
                let mut e = ChoamRemoveForce::new(game, f);
                e.set_location(game, Some(l));
                let ActionOptions::ChoamRemoveForce(o) = opts(game, p, &event(e), d) else { panic!() };
                let factions: Vec<Faction> = ChoamRemoveForce::valid_factions(game, Some(p), Some(l)).collect();
                assert_eq!(o.factions.iter().map(|x| x.faction.clone()).collect::<Vec<_>>(), names(factions.clone()));
                for (x, f) in o.factions.iter().zip(factions) {
                    assert_eq!(x.special_choices, ChoamRemoveForce::valid_special_force_choices(game, Some(l), f));
                }
            }
        }
        (GameEvent::DiscoveryRevealed(e), ActionOptions::DiscoveryRevealed(o)) => {
            let locations: Vec<Location> = DiscoveryRevealed::get_locations(game, p).collect();
            assert_eq!(o.locations.iter().map(|x| x.location).collect::<Vec<_>>(), ids(locations.clone()));
            for (x, l) in o.locations.iter().zip(&locations) {
                assert_eq!(x.token, game.discoveries_on_planet.get(l).unwrap().token.to_string());
            }
            assert_eq!(e.location(game), locations.first().copied(), "the form starts on the first location");
        }
        (GameEvent::FlightUsed(e), ActionOptions::FlightUsed(_)) => assert!(e.move_three_territories),
        (GameEvent::TestingStationUsed(e), ActionOptions::TestingStationUsed(_)) => assert_eq!(e.value_added, -1),
        (GameEvent::KarmaHmsMovement(_), ActionOptions::KarmaHmsMovement(o)) => {
            assert_eq!(o.targets, ids(KarmaHmsMovement::valid_locations(game)));
            assert_eq!(o.moves_left, game.karma_hms_moves_left);
        }
        (GameEvent::PerformHmsMovement(_), ActionOptions::PerformHmsMovement(o)) => {
            assert_eq!(o.targets, ids(PerformHmsMovement::valid_locations(game)));
            assert_eq!(o.moves_left, game.hms_moves_left);
        }
        (GameEvent::KarmaShipmentPrevention(e), ActionOptions::KarmaShipmentPrevention(o)) => {
            let targets: Vec<Faction> = KarmaShipmentPrevention::get_valid_targets(game, p).collect();
            assert_eq!(o.targets, names(targets.clone()));
            assert_eq!(e.target, targets.first().copied().unwrap_or(Faction::None));
        }
        (GameEvent::PerformBeneGesseritPlacement(_), ActionOptions::PerformBeneGesseritPlacement(o)) => {
            assert_eq!(o.targets, ids(PerformBeneGesseritPlacement::valid_locations(game)))
        }
        (GameEvent::PerformMoritaniSetup(_), ActionOptions::PerformMoritaniSetup(o)) => assert_eq!(o.targets, ids(PerformMoritaniSetup::valid_locations(game))),
        (GameEvent::PerformHmsPlacement(_), ActionOptions::PerformHmsPlacement(o)) => assert_eq!(o.targets, ids(PerformHmsPlacement::valid_locations(game, p))),
        (GameEvent::FremenSentMonster(_), ActionOptions::FremenSentMonster(o)) => assert_eq!(o.targets, sectors(FremenSentMonster::valid_targets(game))),
        (GameEvent::SetShipmentPermission(_), ActionOptions::SetShipmentPermission(o)) => {
            assert_eq!(o.targets, names(SetShipmentPermission::valid_targets(game, p)));
            assert_eq!(o.permissions.len(), game.shipment_permissions.len());
            for (row, (f, perm)) in o.permissions.iter().zip(game.shipment_permissions.iter()) {
                assert_eq!(row.faction, f.to_string());
                assert_eq!(row.cross, perm.value() & 1 != 0);
                assert_eq!(row.to_homeworld, perm.value() & 2 != 0);
                assert_eq!(row.guild_rate, perm.value() & 4 != 0);
            }
        }
        (GameEvent::TerrorPlanted(_), ActionOptions::TerrorPlanted(o)) => {
            assert_eq!(o.may_remove, TerrorPlanted::may_remove_tokens(game, p));
            assert_eq!(o.remove_types, names(TerrorPlanted::valid_terror_types(game, true)));
            assert_eq!(o.place_types, names(TerrorPlanted::valid_terror_types(game, false)));
            assert_eq!(o.unplaced, names(game.unplaced_terror_tokens.iter().copied()));
            assert_eq!(o.targets, sectors(TerrorPlanted::valid_strongholds(game, p)));
        }
        (GameEvent::TerrorRevealed(_), ActionOptions::TerrorRevealed(o)) => {
            assert_eq!(o.victim, TerrorRevealed::get_victim(game).to_string());
            assert_eq!(o.territory, TerrorRevealed::get_territory(game).map(|t| t.id()));
            assert_eq!(o.types, names(TerrorRevealed::get_types(game)));
            assert_eq!(o.may_offer_alliance, TerrorRevealed::may_offer_alliance(game));
            assert_eq!(o.may_pass, TerrorRevealed::may_pass(game));
            assert_eq!(o.sneak_attack_targets, ids(TerrorRevealed::valid_sneak_attack_targets(game, p)));
            assert_eq!(o.max_sneak_attack_forces, TerrorRevealed::max_amount_of_forces_in_sneak_attack(game, p));
        }
        (draft, o) => panic!("{}: options {o:?} for draft {draft:?}", t.name()),
    }
}

/// The maximum forces and special forces a draft may ship.
fn shipment_max(game: &Game, p: PlayerIndex, s: &Shipment, special: bool) -> i32 {
    match s.shipment_type {
        ShipmentType::ShipmentNormal => Shipment::valid_max_normal_shipment_forces(game, p, special, s.no_field_value()),
        ShipmentType::ShipmentRicheseSecretAlly => Shipment::valid_max_secret_ally_shipment_forces(game, p, special),
        ShipmentType::ShipmentSiteToSite => Shipment::valid_max_shipment_site_to_site_forces(game, p, special, s.from(game)),
        ShipmentType::ShipmentBack => Shipment::valid_max_shipment_back_forces(game, p, special, s.to(game)),
        _ => 0,
    }
}

fn check_shipment_draft(game: &Game, p: PlayerIndex, s: &Shipment, o: &ShipmentOptions) {
    let pl = game.player(p);
    let cross = Shipment::may_ship_cross_planet(game, p);
    assert_eq!(o.may_ship_cross_planet, cross);
    assert_eq!(o.may_ship_to_reserves, Shipment::may_ship_to_reserves(game, p));
    let targets = match s.shipment_type {
        ShipmentType::ShipmentNormal => ids(Shipment::valid_shipment_locations(game, p, false, false)),
        ShipmentType::ShipmentRicheseSecretAlly => ids(Shipment::valid_shipment_locations(game, p, false, true)),
        ShipmentType::ShipmentSiteToSite => ids(Shipment::valid_shipment_locations(game, p, true, false)),
        _ => vec![],
    };
    assert_eq!(o.targets, targets);
    assert_eq!(o.max_forces, shipment_max(game, p, s, false));
    assert_eq!(o.max_special_forces, shipment_max(game, p, s, true));
    assert_eq!(o.cost, Shipment::determine_cost(game, p, s));
    assert_eq!(o.may_smuggle, Shipment::may_smuggle(game, p, s.to(game)));
    assert_eq!(o.cunning_no_field_values, Shipment::valid_cunning_no_field_values(game, p, s.no_field_value()));
    assert_eq!(o.default_no_field_special_force_amount, Shipment::default_no_field_special_force_amount(pl.faction, s.no_field_value()));
    let expected_ally = if pl.ally != Faction::None && o.cost > 0 { game.resources_your_ally_can_pay(p) } else { 0 };
    assert_eq!(o.ally_can_pay, expected_ally);
    let destination = if s.shipment_type == ShipmentType::ShipmentBack { None } else { s.to(game) };
    assert_eq!(o.conflicting_ally, destination.is_some_and(|l| game.contains_conflicting_ally(p, l)));
    assert_eq!(
        o.may_karma,
        Shipment::can_karma(game, p) && (pl.faction != Faction::Fremen || s.shipment_type == ShipmentType::ShipmentSiteToSite) && !cross
    );
}

fn shipment_draft(game: &Game, f: Faction, ty: ShipmentType, from: Option<Location>, to: Option<Location>, forces: i32, special: i32) -> Shipment {
    let mut s = Shipment::new(game, f);
    s.shipment_type = ty;
    s.set_no_field_value(-1);
    s.set_cunning_no_field_value(-1);
    s.set_from(game, from);
    s.set_to(game, to);
    s.set_karma_card(None);
    let direction = if ty == ShipmentType::ShipmentBack { -1 } else { 1 };
    s.force_amount = direction * forces;
    s.special_force_amount = direction * special;
    s.use_richese_secret_ally = ty == ShipmentType::ShipmentRicheseSecretAlly;
    s
}

fn check_shipment(game: &Game, p: PlayerIndex, s: &Shipment, o: &ShipmentOptions, d: &dyn Describer) {
    let pl = game.player(p);
    let f = pl.faction;
    let cross = Shipment::may_ship_cross_planet(game, p);
    let to_reserves = Shipment::may_ship_to_reserves(game, p);
    check_shipment_draft(game, p, s, o);
    assert_eq!(o.prevented_from_shipping, f != Faction::Fremen && game.prevented_from_shipping(f) && !cross);
    let sources: Vec<Location> = if cross || to_reserves { Shipment::valid_source_locations(game, p).collect() } else { vec![] };
    assert_eq!(o.sources, sources.iter().map(|l| forces_at(game, p, *l)).collect::<Vec<_>>());
    assert_eq!(o.may_use_richese_secret_ally, Shipment::may_use_richese_secret_ally(game, p));
    assert_eq!(o.may_use_no_field, Shipment::may_use_no_field(game, p));
    assert_eq!(o.no_field_values, Shipment::valid_no_field_values(game, p));
    assert_eq!(o.may_use_cunning_no_field, Shipment::may_use_cunning_no_field(game, p));
    assert_eq!(o.richese_high_threshold, game.has_high_threshold(Faction::Richese));
    let normal = Shipment::homeworlds_to_ship_from(game, p, false);
    let special = Shipment::homeworlds_to_ship_from(game, p, true);
    for w in &o.homeworlds {
        assert_eq!(w.normal, normal.iter().any(|h| h.location().id() == w.location));
        assert_eq!(w.special, special.iter().any(|h| h.location().id() == w.location));
    }
    assert_eq!(o.homeworlds.len(), normal.iter().chain(special.iter()).map(|h| h.location().id()).collect::<std::collections::BTreeSet<_>>().len());
    let karma = Shipment::valid_karma_cards(game, p);
    assert_eq!(o.karma_cards.iter().map(|k| (k.card, k.own)).collect::<Vec<_>>(), karma.iter().map(|c| (c.id(), pl.treachery_cards.contains(c))).collect::<Vec<_>>());
    assert_eq!(o.great_flat, game.map.the_great_flat().territory().id());
    assert_eq!(o.karma_name, d.describe(&ExpressionElement::from(TreacheryCardType::Karma)));

    // Each destination with one force, as InitializeAmounts opens the pinned panel.
    for to in Shipment::valid_shipment_locations(game, p, false, false).collect::<Vec<_>>() {
        let forces = 1.min(Shipment::valid_max_normal_shipment_forces(game, p, false, -1));
        let specials = if forces == 0 { 1.min(Shipment::valid_max_normal_shipment_forces(game, p, true, -1)) } else { 0 };
        let draft = shipment_draft(game, f, ShipmentType::ShipmentNormal, None, Some(to), forces, specials);
        let ActionOptions::Shipment(o) = opts(game, p, &event(draft.clone()), d) else { panic!() };
        check_shipment_draft(game, p, &draft, &o);
        for nf in Shipment::valid_no_field_values(game, p).into_iter().take(2) {
            let mut draft = draft.clone();
            draft.set_no_field_value(nf);
            let ActionOptions::Shipment(o) = opts(game, p, &event(draft.clone()), d) else { panic!() };
            check_shipment_draft(game, p, &draft, &o);
        }
    }
    // Each source: site-to-site and back to reserves.
    for from in sources {
        if cross {
            let draft = shipment_draft(game, f, ShipmentType::ShipmentSiteToSite, Some(from), None, 0, 0);
            let ActionOptions::Shipment(o) = opts(game, p, &event(draft.clone()), d) else { panic!() };
            check_shipment_draft(game, p, &draft, &o);
            let here = !game.prevented_from_shipping(f) && Shipment::valid_shipment_locations(game, p, false, false).any(|l| l == from);
            assert_eq!(o.may_ship_here_from_reserves, here);
        }
        if to_reserves {
            let draft = shipment_draft(game, f, ShipmentType::ShipmentBack, None, Some(from), 1, 0);
            let ActionOptions::Shipment(o) = opts(game, p, &event(draft.clone()), d) else { panic!() };
            check_shipment_draft(game, p, &draft, &o);
        }
    }
    if Shipment::may_use_richese_secret_ally(game, p) {
        let draft = shipment_draft(game, f, ShipmentType::ShipmentRicheseSecretAlly, None, None, 0, 0);
        let ActionOptions::Shipment(o) = opts(game, p, &event(draft.clone()), d) else { panic!() };
        check_shipment_draft(game, p, &draft, &o);
    }
}

/// The placement forms: sources, then each source with all its forces selected.
fn check_placement(game: &Game, p: PlayerIndex, o: &PlacementOptions, t: GameEventType) {
    let pl = game.player(p);
    let f = pl.faction;
    let sources: Vec<Location> = PlacementEvent::valid_movement_sources(game, p)
        .iter()
        .flat_map(|t| t.locations().iter().copied())
        .filter(|l| pl.any_forces_in(Some(*l)) > 0 && !game.is_in_storm(*l))
        .collect();
    assert_eq!(o.sources, sources.iter().map(|l| forces_at(game, p, *l)).collect::<Vec<_>>());
    assert!(o.targets.is_empty(), "no destinations before forces are selected");
    let planetology = game.current_planetology.as_ref();
    assert_eq!(o.may_move_from_two_territories, planetology.is_some_and(|x| x.move_from_two_territories() && x.initiator() == f));
    let d = default_describer();
    for l in sources {
        let mut forces: NetDictionary<Location, Battalion> = NetDictionary::new();
        forces.add(l, Battalion::new(f, pl.forces_in(Some(l)), pl.special_forces_in(Some(l)), l));
        let expected = ids(PlacementEvent::valid_targets_game_player_dictionary_location_battalion(game, p, &forces));
        let draft = placement_draft(game, f, t, &forces, None);
        let o = placement_options(opts(game, p, &draft, &*d));
        assert_eq!(o.targets, expected);
        // Pin each destination: the Bene Gesserit choice and the conflicting-ally warning.
        for to in expected.iter().take(6) {
            let to = game.map.location_lookup().find(*to).unwrap();
            let draft = placement_draft(game, f, t, &forces, Some(to));
            let o = placement_options(opts(game, p, &draft, &*d));
            assert_eq!(o.conflicting_ally, game.contains_conflicting_ally(p, to));
            if f == Faction::BeneGesserit {
                let advisors = pl.special_forces_in(Some(l)) > 0;
                let may = PlacementEvent::may_move_as_advisors(game, p, to.territory(), advisors);
                let choice = may && PlacementEvent::may_move_as_bene_gesserit_fighters(game, p, to.territory());
                assert_eq!(o.advisor_choice, choice);
                assert_eq!(o.as_advisors, !choice && may);
            } else {
                assert!(!o.advisor_choice && !o.as_advisors);
            }
        }
    }
}

fn placement_options(o: ActionOptions) -> PlacementOptions {
    match o {
        ActionOptions::Move(o) | ActionOptions::Caravan(o) | ActionOptions::FremenRidesMonster(o) => o,
        o => panic!("not a placement: {o:?}"),
    }
}

fn placement_draft(game: &Game, f: Faction, t: GameEventType, forces: &NetDictionary<Location, Battalion>, to: Option<Location>) -> GameEvent {
    match t {
        GameEventType::Move => {
            let mut e = Move::new(game, f);
            e.set_force_locations(game, forces);
            e.set_to(game, to);
            event(e)
        }
        GameEventType::Caravan => {
            let mut e = Caravan::new(game, f);
            e.set_force_locations(game, forces);
            e.set_to(game, to);
            event(e)
        }
        GameEventType::FremenRidesMonster => {
            let mut e = FremenRidesMonster::new(game, f);
            e.set_force_locations(game, forces);
            e.set_to(game, to);
            event(e)
        }
        _ => unreachable!(),
    }
}

fn check_ride(game: &Game, p: PlayerIndex, o: &PlacementOptions, d: &dyn Describer) {
    let pl = game.player(p);
    let to_ride = FremenRidesMonster::to_ride(game);
    let sources: Vec<Location> = if to_ride.is_some() { FremenRidesMonster::valid_sources(game).collect() } else { vec![] };
    let max = FremenRidesMonster::max_forces_from_reserves(game, p, false);
    let max_special = FremenRidesMonster::max_forces_from_reserves(game, p, true);
    let from_reserves = sources.is_empty() && (max > 0 || max_special > 0);
    assert_eq!(o.sources, sources.iter().map(|l| forces_at(game, p, *l)).collect::<Vec<_>>());
    assert_eq!((o.from_reserves, o.max_from_reserves, o.max_special_from_reserves), (from_reserves, max, max_special));
    let valid_targets = ids(FremenRidesMonster::valid_targets(game, p));
    assert_eq!(o.targets, if from_reserves { valid_targets.clone() } else { vec![] });
    assert!(o.title.is_some());
    for l in sources {
        let mut forces: NetDictionary<Location, Battalion> = NetDictionary::new();
        forces.add(l, Battalion::new(pl.faction, pl.forces_in(Some(l)), pl.special_forces_in(Some(l)), l));
        let o = placement_options(opts(game, p, &placement_draft(game, pl.faction, GameEventType::FremenRidesMonster, &forces, None), d));
        assert_eq!(o.targets, valid_targets);
    }
}

#[test]
fn fixtures() {
    let d = default_describer();
    let mut seen = Seen::default();
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scripts/fixtures");
    let mut fixtures: Vec<String> = std::fs::read_dir(&dir).unwrap().map(|e| e.unwrap().path().file_stem().unwrap().to_string_lossy().to_string()).collect();
    fixtures.sort();
    for name in fixtures {
        // Fixtures before the lowest version the scene page takes don't replay here either.
        let Ok(mut r) = std::panic::catch_unwind(|| load(&name, 0)) else { continue };
        let players: Vec<PlayerIndex> = r.game.player_indices().collect();
        for p in players {
            for t in r.game.get_applicable_events(Some(p), true) {
                if TYPES.contains(&t) {
                    check(&r.game, p, t, &*d, &mut seen);
                }
            }
        }
    }
    for t in ["Shipment", "Move", "Caravan", "BeneGesseritAccompanies", "PerformFremenSetup", "FremenRidesMonster", "FremenSentMonster"] {
        assert!(seen.0.contains_key(t), "the fixtures reach {t}: {:?}", seen.0);
    }
}

#[test]
fn bot_games() {
    let d = default_describer();
    let mut seen = Seen::default();
    for path in common::sample_bot_traces() {
        let trace = common::read_trace(&path);
        let participation = Shared::new(Participation::default());
        let mut r = ViewReplica::new(trace.version, &participation);
        for e in &trace.events {
            r.apply(e).unwrap();
            let players: Vec<PlayerIndex> = r.game.player_indices().collect();
            for p in players {
                for t in r.game.get_applicable_events(Some(p), false) {
                    if TYPES.contains(&t) {
                        check(&r.game, p, t, &*d, &mut seen);
                    }
                }
            }
        }
    }
    println!("checked: {:?}", seen.0);
    for t in ["Shipment", "Move", "Caravan", "SetShipmentPermission", "AmbassadorPlaced", "PerformHmsMovement", "TerrorPlanted"] {
        assert!(seen.0.contains_key(t), "the bot games reach {t}: {:?}", seen.0);
    }
}

/// The view's action carries the default draft's options and validation, and a draft query
/// (JSON, as the client sends it) answers the dependent options.
#[test]
fn view_and_draft_query() {
    let mut r = load("ship", 0);
    let viewer = viewer_of(&r, Faction::Emperor);
    let view = r.view(viewer);
    let a = view.actions.iter().find(|a| a.event_type == "Shipment").expect("Emperor ships");
    let Some(ActionOptions::Shipment(o)) = &a.options else { panic!("{:?}", a.options) };
    assert!(!o.targets.is_empty());
    assert!(a.validation.is_some(), "an empty shipment is not valid");

    let p = r.game.get_player(Some(Faction::Emperor)).unwrap();
    let to = o.targets[0];
    let draft = format!(
        r#"{{"$type":"Shipment","ShipmentType":1,"ForceAmount":2,"SpecialForceAmount":0,"SmuggledAmount":0,"SmuggledSpecialAmount":0,"AllyContributionAmount":0,"UseRicheseSecretAlly":false,"_toId":{to},"_fromId":-1,"_noFieldValue":0,"_cunningNoFieldValue":0,"_karmaCardId":-1,"_forceLocations":"","Passed":false,"Initiator":0}}"#
    );
    let answer = r.query(viewer, &draft).unwrap();
    let Some(ActionOptions::Shipment(q)) = answer.options else { panic!() };
    let loc = r.game.map.location_lookup().find(to).unwrap();
    let expected = shipment_draft(&r.game, Faction::Emperor, ShipmentType::ShipmentNormal, None, Some(loc), 2, 0);
    assert_eq!(q.cost, Shipment::determine_cost(&r.game, p, &expected));
    assert!(q.cost > 0);
    assert_eq!(answer.validation, None, "two forces from reserves to a valid destination");

    // Move: picking a source gives its destinations.
    let mut r = load("move", 0);
    let viewer = viewer_of(&r, Faction::Fremen);
    let view = r.view(viewer);
    let a = view.actions.iter().find(|a| a.event_type == "Move").unwrap();
    let Some(ActionOptions::Move(o)) = &a.options else { panic!() };
    let src = &o.sources[0];
    let draft = format!(
        r#"{{"$type":"Move","AsAdvisors":false,"_toId":-1,"_forceLocations":"{}:{}|{}","Passed":false,"Initiator":0}}"#,
        src.location, src.forces, src.special_forces
    );
    let answer = r.query(viewer, &draft).unwrap();
    let Some(ActionOptions::Move(q)) = answer.options else { panic!() };
    assert!(!q.targets.is_empty());
}

/// The draft the options carry for `draft`, read back as an event, after checking that the
/// validation next to the options is that draft's.
fn adjusted(game: &Game, p: PlayerIndex, draft: &GameEvent, d: &dyn Describer) -> GameEvent {
    let (o, validation) = options_and_validation(game, p, draft, d).expect("a ship-and-move type");
    let o = o.expect("the builder does not throw");
    let value = serde_json::to_value(&o).unwrap();
    let text = serde_json::to_string(&value["draft"]).unwrap();
    let e: GameEvent = json::deserialize(&text).unwrap_or_else(|err| panic!("the draft reads back: {err:?} {text}"));
    e.initialize(game, game.player(p).faction);
    assert_eq!(validation, validate(game, e.clone(), d), "the validation is the adjusted draft's");
    e
}

/// Sends drafts whose inputs the form would change, and checks the adjusted drafts.
fn check_adjustments(game: &Game, p: PlayerIndex, draft: &GameEvent, d: &dyn Describer) {
    let pl = game.player(p);
    let f = pl.faction;
    match draft {
        GameEvent::Shipment(_) => check_shipment_adjustments(game, p, d),
        GameEvent::Move(_) | GameEvent::Caravan(_) => {
            let t = draft.event_type();
            let sources: Vec<Location> = PlacementEvent::valid_movement_sources(game, p)
                .iter()
                .flat_map(|t| t.locations().iter().copied())
                .filter(|l| pl.any_forces_in(Some(*l)) > 0 && !game.is_in_storm(*l))
                .take(3)
                .collect();
            for l in sources {
                let mut forces: NetDictionary<Location, Battalion> = NetDictionary::new();
                forces.add(l, Battalion::new(f, pl.forces_in(Some(l)), pl.special_forces_in(Some(l)), l));
                let targets: Vec<Location> = PlacementEvent::valid_targets_game_player_dictionary_location_battalion(game, p, &forces);
                for to in targets.into_iter().take(4) {
                    for start in [false, true] {
                        let mut e = placement_draft(game, f, t, &forces, Some(to));
                        set_as_advisors(&mut e, start);
                        // PinContent: Bene Gesserit move as advisors where they may and can't choose;
                        // everyone else as fighters.
                        let expected = if f == Faction::BeneGesserit {
                            let may = PlacementEvent::may_move_as_advisors(game, p, to.territory(), pl.special_forces_in(Some(l)) > 0);
                            if may && PlacementEvent::may_move_as_bene_gesserit_fighters(game, p, to.territory()) { start } else { may }
                        } else {
                            false
                        };
                        assert_eq!(as_advisors_of(&adjusted(game, p, &e, d)), expected, "{}: AsAdvisors at {}", t.name(), to.id());
                    }
                }
                // No destination, no pinned panel: nothing changes.
                let mut e = placement_draft(game, f, t, &forces, None);
                set_as_advisors(&mut e, true);
                assert!(as_advisors_of(&adjusted(game, p, &e, d)));
            }
        }
        GameEvent::DiscoveryEntered(_) => {
            for ds in DiscoveryEntered::valid_targets(game, p).collect::<Vec<_>>() {
                let sources: Vec<Location> = DiscoveryEntered::valid_sources(game, p, ds).collect();
                let mut e = DiscoveryEntered::new(game, f);
                e.set_to(game, Some(ds.location()));
                let GameEvent::DiscoveryEntered(a) = adjusted(game, p, &event(e.clone()), d) else { panic!() };
                // OnPickChanged: every force next to the stronghold.
                let got = a.force_locations(game);
                assert_eq!(got.keys().copied().collect::<Vec<_>>(), sources);
                for l in &sources {
                    let b = got.get(l).unwrap();
                    assert_eq!((b.amount_of_forces, b.amount_of_special_forces), (pl.forces_in(Some(*l)), pl.special_forces_in(Some(*l))));
                }
                // SetAmount: a lowered amount stays.
                if let Some(l) = sources.first() {
                    let mut forces: NetDictionary<Location, Battalion> = NetDictionary::new();
                    forces.add(*l, Battalion::new(f, 0, 0, *l));
                    e.set_force_locations(game, &forces);
                    let GameEvent::DiscoveryEntered(a) = adjusted(game, p, &event(e), d) else { panic!() };
                    assert_eq!(a.force_locations(game).get(l).map(|b| b.amount_of_forces), Some(0));
                }
            }
        }
        GameEvent::AmbassadorActivated(_) => {
            let ambassador = AmbassadorActivated::get_ambassador(game);
            let actuals: Vec<Ambassador> = if ambassador == Ambassador::BeneGesserit {
                let mut v = AmbassadorActivated::get_valid_bene_gesserit_ambassadors(game);
                v.push(Ambassador::None);
                v
            } else {
                vec![ambassador]
            };
            let heroes = AmbassadorActivated::valid_tleilaxu_heroes(game, p);
            for actual in actuals {
                for hero in std::iter::once(None).chain(heroes.iter().copied().map(Some).take(3)) {
                    let mut e = AmbassadorActivated::new(game, f);
                    e.bene_gesserit_selected_ambassador = actual;
                    e.ecaz_take_vidal = true;
                    e.ecaz_offer_alliance = true;
                    e.ecaz_give_vidal_to_ally = true;
                    e.guild_force_amount = 3;
                    e.tleilaxu_amount_of_forces = 99;
                    e.set_tleilaxu_hero(hero);
                    e.tleilaxu_assign_skill = true;
                    e.set_ixian_card(None);
                    let GameEvent::AmbassadorActivated(a) = adjusted(game, p, &event(e), d) else { panic!() };
                    // BeneGesserit: the required image choice; otherwise the shown one.
                    let valid_bg = AmbassadorActivated::get_valid_bene_gesserit_ambassadors(game);
                    let expected_actual = if ambassador != Ambassador::BeneGesserit {
                        ambassador
                    } else if valid_bg.contains(&actual) {
                        actual
                    } else {
                        valid_bg.first().copied().unwrap_or(Ambassador::None)
                    };
                    assert_eq!(a.bene_gesserit_selected_ambassador, expected_actual);
                    let (mut take, mut offer, mut give) = (true, true, true);
                    let (mut guild, mut tleilaxu, mut skill) = (3, 99, true);
                    let mut ixian = None;
                    match expected_actual {
                        Ambassador::Ecaz => {
                            take = AmbassadorActivated::vidal_can_be_taken(game, p);
                            if !take && AmbassadorActivated::alliance_can_be_offered(game, p) {
                                give = AmbassadorActivated::vidal_can_be_offered_to_new_ally(game, p);
                            } else {
                                offer = false;
                                give = false;
                            }
                        }
                        Ambassador::Guild => {
                            if AmbassadorActivated::valid_guild_targets(game, p).next().is_none() {
                                guild = 0;
                            }
                        }
                        Ambassador::Ixian => ixian = AmbassadorActivated::get_valid_ixian_cards(game, p).first().copied(),
                        Ambassador::Tleilaxu => {
                            tleilaxu = AmbassadorActivated::valid_tleilaxu_max_amount(game, p).max(0);
                            skill = hero.is_some_and(|h| Revival::may_assign_skill(game, p, Some(h)));
                        }
                        _ => {}
                    }
                    assert_eq!((a.ecaz_take_vidal, a.ecaz_offer_alliance, a.ecaz_give_vidal_to_ally), (take, offer, give), "Ecaz checkboxes");
                    assert_eq!((a.guild_force_amount, a.tleilaxu_amount_of_forces, a.tleilaxu_assign_skill), (guild, tleilaxu, skill));
                    assert_eq!(a.ixian_card(), ixian);
                    assert_eq!(a.tleilaxu_hero(), hero, "the hero picker is not required");
                }
            }
        }
        GameEvent::AmbassadorPlaced(_) => {
            let valid = AmbassadorPlaced::valid_ambassadors(game, p);
            let mut e = AmbassadorPlaced::new(game, f);
            e.ambassador = Ambassador::None;
            let GameEvent::AmbassadorPlaced(a) = adjusted(game, p, &event(e), d) else { panic!() };
            let expected = if valid.contains(&Ambassador::None) { Ambassador::None } else { valid.first().copied().unwrap_or(Ambassador::None) };
            assert_eq!(a.ambassador, expected);
        }
        GameEvent::BeneGesseritAccompanies(_) => {
            for l in BeneGesseritAccompanies::valid_targets(game, p) {
                let mut e = BeneGesseritAccompanies::new(game, f);
                e.set_location(game, Some(l));
                e.accompanies = true;
                e.extra_advisor = true;
                let GameEvent::BeneGesseritAccompanies(a) = adjusted(game, p, &event(e), d) else { panic!() };
                assert_eq!(a.extra_advisor, BeneGesseritAccompanies::may_send_extra_advisor(game, p, l));
            }
            let mut e = BeneGesseritAccompanies::new(game, f);
            e.set_location(game, None);
            e.extra_advisor = true;
            let GameEvent::BeneGesseritAccompanies(a) = adjusted(game, p, &event(e), d) else { panic!() };
            assert!(a.extra_advisor, "no pinned panel without a location");
        }
        GameEvent::ChoamRemoveForce(_) => {
            for l in ChoamRemoveForce::valid_locations(game).collect::<Vec<_>>() {
                for start in game.players.iter().map(|x| x.faction).chain([Faction::None]).collect::<Vec<_>>() {
                    for special in [false, true] {
                        let mut e = ChoamRemoveForce::new(game, f);
                        e.set_location(game, Some(l));
                        e.target = start;
                        e.special_force = special;
                        let GameEvent::ChoamRemoveForce(a) = adjusted(game, p, &event(e), d) else { panic!() };
                        // OnPickChanged / SetTarget.
                        let factions: Vec<Faction> = ChoamRemoveForce::valid_factions(game, Some(p), Some(l)).collect();
                        let target = if factions.contains(&start) { start } else { factions.first().copied().unwrap_or(Faction::None) };
                        let choices = ChoamRemoveForce::valid_special_force_choices(game, Some(l), target);
                        let sf = if choices.contains(&special) { special } else { choices.first().copied().unwrap_or(false) };
                        assert_eq!((a.target, a.special_force), (target, sf));
                    }
                }
            }
        }
        GameEvent::DiscoveryRevealed(_) => {
            let locations: Vec<Location> = DiscoveryRevealed::get_locations(game, p).collect();
            for l in locations.iter().copied().map(Some).chain([None]) {
                let mut e = DiscoveryRevealed::new(game, f);
                e.set_location(game, l);
                let GameEvent::DiscoveryRevealed(a) = adjusted(game, p, &event(e), d) else { panic!() };
                assert_eq!(a.location(game), l.or(locations.first().copied()));
            }
        }
        GameEvent::KarmaShipmentPrevention(_) => {
            let targets: Vec<Faction> = KarmaShipmentPrevention::get_valid_targets(game, p).collect();
            let mut e = KarmaShipmentPrevention::new(game, f);
            e.target = f;
            let GameEvent::KarmaShipmentPrevention(a) = adjusted(game, p, &event(e), d) else { panic!() };
            assert_eq!(a.target, if targets.contains(&f) { f } else { targets.first().copied().unwrap_or(Faction::None) });
        }
        GameEvent::TerrorPlanted(_) => {
            let removing = TerrorPlanted::may_remove_tokens(game, p);
            let strongholds: Vec<Territory> = TerrorPlanted::valid_strongholds(game, p).take(2).collect();
            for stronghold in std::iter::once(None).chain(strongholds.into_iter().map(Some)) {
                for ty in [TerrorType::Atomics, TerrorType::Robbery, TerrorType::None] {
                    let mut e = TerrorPlanted::new(game, f);
                    e.type_ = ty;
                    e.set_stronghold(game, stronghold);
                    let GameEvent::TerrorPlanted(a) = adjusted(game, p, &event(e), d) else { panic!() };
                    let expected = if removing || stronghold.is_some() {
                        let types: Vec<TerrorType> = TerrorPlanted::valid_terror_types(game, removing).collect();
                        if types.contains(&ty) { ty } else { types.first().copied().unwrap_or(TerrorType::None) }
                    } else {
                        ty
                    };
                    assert_eq!(a.type_, expected);
                }
            }
        }
        GameEvent::TerrorRevealed(_) => {
            let types: Vec<TerrorType> = TerrorRevealed::get_types(game).collect();
            let possible = TerrorRevealed::valid_sneak_attack_targets(game, p).next().is_some();
            for ty in types.iter().copied().chain([TerrorType::None]) {
                let mut e = TerrorRevealed::new(game, f);
                e.type_ = ty;
                e.robbery_takes_card = true;
                e.set_card_to_give_in_sabotage(pl.treachery_cards.first().copied());
                e.forces_in_sneak_attack = 2;
                let GameEvent::TerrorRevealed(a) = adjusted(game, p, &event(e), d) else { panic!() };
                let ty = if types.contains(&ty) { ty } else { types[0] };
                assert_eq!(a.type_, ty);
                assert_eq!(a.robbery_takes_card, ty == TerrorType::Robbery);
                assert_eq!(a.card_to_give_in_sabotage(), if ty == TerrorType::Sabotage { pl.treachery_cards.first().copied() } else { None });
                assert_eq!(a.forces_in_sneak_attack, if ty == TerrorType::SneakAttack && possible { 2 } else { 0 });
            }
        }
        GameEvent::TestingStationUsed(_) => {
            for (v, expected) in [(0, -1), (1, 1), (-1, -1), (5, -1)] {
                let mut e = TestingStationUsed::new(game, f);
                e.value_added = v;
                let GameEvent::TestingStationUsed(a) = adjusted(game, p, &event(e), d) else { panic!() };
                assert_eq!(a.value_added, expected);
            }
        }
        _ => {}
    }
}

fn set_as_advisors(e: &mut GameEvent, v: bool) {
    match e {
        GameEvent::Move(m) => std::rc::Rc::make_mut(m).as_advisors = v,
        GameEvent::Caravan(c) => std::rc::Rc::make_mut(c).as_advisors = v,
        _ => unreachable!(),
    }
}

fn as_advisors_of(e: &GameEvent) -> bool {
    match e {
        GameEvent::Move(m) => m.as_advisors,
        GameEvent::Caravan(c) => c.as_advisors,
        _ => unreachable!(),
    }
}

/// The shipment's pinned panel: a draft with every optional input set comes back with the
/// inputs the panel would not offer dropped.
fn check_shipment_adjustments(game: &Game, p: PlayerIndex, d: &dyn Describer) {
    let pl = game.player(p);
    let f = pl.faction;
    let cross = Shipment::may_ship_cross_planet(game, p);
    let prevented = f != Faction::Fremen && game.prevented_from_shipping(f) && !cross;
    let karma_cards = Shipment::valid_karma_cards(game, p);
    let any_karma = karma_cards.first().copied().or(pl.treachery_cards.first().copied());
    let no_field = Shipment::valid_no_field_values(game, p).first().copied().unwrap_or(3);
    let worlds: Vec<Homeworld> = Shipment::homeworlds_to_ship_from(game, p, false);
    let mut drafts: Vec<Shipment> = vec![];
    for to in Shipment::valid_shipment_locations(game, p, false, false).take(3).map(Some).chain([None]) {
        drafts.push(shipment_draft(game, f, ShipmentType::ShipmentNormal, None, to, 1, 0));
    }
    let sources: Vec<Location> = Shipment::valid_source_locations(game, p).take(2).collect();
    for from in &sources {
        drafts.push(shipment_draft(game, f, ShipmentType::ShipmentBack, None, Some(*from), 1, 0));
        if let Some(to) = Shipment::valid_shipment_locations(game, p, true, false).next() {
            drafts.push(shipment_draft(game, f, ShipmentType::ShipmentSiteToSite, Some(*from), Some(to), 1, 0));
        }
    }
    for mut s in drafts {
        s.set_no_field_value(no_field);
        s.set_cunning_no_field_value(no_field);
        s.smuggled_amount = 1;
        s.smuggled_special_amount = 1;
        s.set_karma_card(any_karma);
        s.ally_contribution_amount = 2;
        if let Some(w) = worlds.first() {
            let mut origins: NetDictionary<Location, Battalion> = NetDictionary::new();
            origins.add(w.location(), Battalion::new(f, 1, 0, w.location()));
            s.set_force_locations(game, &origins);
        }
        let GameEvent::Shipment(a) = adjusted(game, p, &event(s.clone()), d) else { panic!() };
        let ty = s.shipment_type;
        let back = ty == ShipmentType::ShipmentBack;
        let pinned = !prevented && (back || s.to(game).is_some());
        let mut e = s.clone();
        if pinned {
            let from_reserves = matches!(ty, ShipmentType::ShipmentNormal | ShipmentType::ShipmentRicheseSecretAlly);
            let choose = from_reserves && (Shipment::homeworlds_to_ship_from(game, p, false).len() > 1 || Shipment::homeworlds_to_ship_from(game, p, true).len() > 1);
            if !choose {
                e.set_force_locations(game, &NetDictionary::new());
            }
            if !Shipment::may_use_no_field(game, p) {
                e.set_no_field_value(-1);
                e.set_cunning_no_field_value(-1);
            }
            if !Shipment::may_smuggle(game, p, s.to(game)) || !from_reserves || f == Faction::Fremen {
                e.smuggled_amount = 0;
                e.smuggled_special_amount = 0;
            }
            if !(Shipment::can_karma(game, p) && (f != Faction::Fremen || ty == ShipmentType::ShipmentSiteToSite) && !cross) {
                e.set_karma_card(None);
            }
            let cost = Shipment::determine_cost(game, p, &e);
            if !(pl.ally != Faction::None && cost > 0 && game.resources_your_ally_can_pay(p) > 0) {
                e.ally_contribution_amount = 0;
            }
        }
        assert_eq!(json::serialize(&event((*a).clone())), json::serialize(&event(e)), "shipment adjustments for {:?} to {:?}", ty, s.to(game).map(|l| l.id()));
        let ActionOptions::Shipment(o) = opts(game, p, &event(s), d) else { panic!() };
        check_shipment_draft(game, p, &a, &o);
    }
}
