//! Option builders for the ship-and-move forms. Each returns the results of the engine helpers
//! the form needs (`Shipment::*`, the placement helpers and others) for the inputs in the draft.
//! Inputs that other choices depend on (the forces selected for a move, the shipment's
//! destination and type, the hero picked for a Tleilaxu ambassador) are read from the draft, so
//! a draft query (protocol.md, Dependent inputs) returns the dependent options.
//!
//! A form also adjusts its inputs: an input that stops applying is dropped (a No-Field value,
//! smuggled forces, a Karama card or an ally payment the shipment no longer allows), the Bene
//! Gesserit advisor flag follows the destination, and a required choice falls back to its first
//! value. [`normalize`] makes those changes to a draft, as `revival.rs` does, so the options and
//! validation are those of the adjusted draft, and the options carry it back to the client
//! (`draft`). What the client still does itself: which sectors it can draw, and the changes a
//! click makes (a new map pick, the amounts it starts with).

use serde_json::Value;
use treachery_engine::compat::cs;
use treachery_engine::events::GameEventType;
use treachery_engine::json;
use treachery_engine::messages::expression::ExpressionElement;
use treachery_engine::compat::net_enum::NetEnum;
use treachery_engine::prelude::*;

use crate::build::{event, validate};
use crate::describe::Describer;
use crate::expr::message;
use crate::schema::*;
use crate::shield::hero_ref;
use super::widgets::{draft_json, required};

fn name<T: ToString>(v: T) -> String {
    v.to_string()
}

fn ids(ls: impl IntoIterator<Item = Location>) -> Vec<i32> {
    ls.into_iter().map(|l| l.id()).collect()
}

/// `SectorsOf(territories)`: every sector of the territories.
fn sectors_of(ts: impl IntoIterator<Item = Territory>) -> Vec<i32> {
    ts.into_iter().flat_map(|t| t.locations().iter().map(|l| l.id())).collect()
}

fn forces_at(game: &Game, p: PlayerIndex, l: Location) -> ForcesAt {
    let pl = game.player(p);
    ForcesAt { location: l.id(), forces: pl.forces_in(Some(l)), special_forces: pl.special_forces_in(Some(l)) }
}

/// The draft a form sends before the player touches it: its starting inputs, after its widgets
/// adjusted them (a required image choice picks the first value).
pub fn default_draft(game: &Game, p: PlayerIndex, faction: Faction, t: GameEventType) -> Option<GameEvent> {
    let draft = initial_draft(game, p, faction, t)?;
    Some(normalized(game, p, &draft))
}

/// The form's starting inputs, before any adjustment.
fn initial_draft(game: &Game, p: PlayerIndex, faction: Faction, t: GameEventType) -> Option<GameEvent> {
    Some(match t {
        GameEventType::Shipment => {
            let mut e = Shipment::new(game, faction);
            e.shipment_type = ShipmentType::ShipmentNormal;
            e.set_no_field_value(-1);
            e.set_cunning_no_field_value(-1);
            e.set_from(game, None);
            e.set_to(game, None);
            e.set_karma_card(None);
            event(e)
        }
        GameEventType::Move => {
            let mut e = Move::new(game, faction);
            e.set_to(game, None);
            event(e)
        }
        GameEventType::Caravan => {
            let mut e = Caravan::new(game, faction);
            e.set_to(game, None);
            event(e)
        }
        GameEventType::FremenRidesMonster => {
            let mut e = FremenRidesMonster::new(game, faction);
            e.set_to(game, None);
            event(e)
        }
        GameEventType::PerformFremenSetup => event(PerformFremenSetup::new(game, faction)),
        GameEventType::DiscoveryEntered => {
            let mut e = DiscoveryEntered::new(game, faction);
            e.set_to(game, None);
            event(e)
        }
        GameEventType::AmbassadorActivated => {
            let mut e = AmbassadorActivated::new(game, faction);
            let ambassador = AmbassadorActivated::get_ambassador(game);
            // BeneGesserit: the required image choice picks the first ambassador.
            let actual = if ambassador == Ambassador::BeneGesserit {
                AmbassadorActivated::get_valid_bene_gesserit_ambassadors(game).first().copied().unwrap_or(Ambassador::None)
            } else {
                ambassador
            };
            e.bene_gesserit_selected_ambassador = actual;
            e.set_choam_cards(&[]);
            if actual == Ambassador::Ixian {
                e.set_ixian_card(AmbassadorActivated::get_valid_ixian_cards(game, p).first().copied());
            } else {
                e.set_ixian_card(None);
            }
            e.set_fremen_or_guild_to(game, None);
            e.set_fremen_force_locations(game, &NetDictionary::new());
            e.set_tleilaxu_hero(None);
            event(e)
        }
        GameEventType::AmbassadorPlaced => {
            let mut e = AmbassadorPlaced::new(game, faction);
            e.ambassador = AmbassadorPlaced::valid_ambassadors(game, p).first().copied().unwrap_or(Ambassador::None);
            e.set_stronghold(game, None);
            event(e)
        }
        GameEventType::BeneGesseritAccompanies => {
            let mut e = BeneGesseritAccompanies::new(game, faction);
            e.set_location(game, None);
            e.accompanies = true;
            event(e)
        }
        GameEventType::BeneGesseritFlip => event(BeneGesseritFlip::new(game, faction)),
        GameEventType::ChoamMovePrevention => {
            let mut e = ChoamMovePrevention::new(game, faction);
            e.set_territory(game, None);
            event(e)
        }
        GameEventType::ChoamRemoveForce => {
            let mut e = ChoamRemoveForce::new(game, faction);
            e.set_location(game, None);
            event(e)
        }
        GameEventType::DiscoveryRevealed => {
            let mut e = DiscoveryRevealed::new(game, faction);
            let first = DiscoveryRevealed::get_locations(game, p).next();
            e.set_location(game, first);
            event(e)
        }
        GameEventType::FlightUsed => {
            let mut e = FlightUsed::new(game, faction);
            e.move_three_territories = true;
            event(e)
        }
        GameEventType::KarmaHmsMovement => {
            let mut e = KarmaHmsMovement::new(game, faction);
            e.set_target(game, None);
            event(e)
        }
        GameEventType::KarmaShipmentPrevention => {
            let mut e = KarmaShipmentPrevention::new(game, faction);
            e.target = KarmaShipmentPrevention::get_valid_targets(game, p).next().unwrap_or(Faction::None);
            event(e)
        }
        GameEventType::PerformBeneGesseritPlacement => {
            let mut e = PerformBeneGesseritPlacement::new(game, faction);
            e.set_target(game, None);
            event(e)
        }
        GameEventType::PerformMoritaniSetup => {
            let mut e = PerformMoritaniSetup::new(game, faction);
            e.set_target(game, None);
            event(e)
        }
        GameEventType::PerformHmsMovement => {
            let mut e = PerformHmsMovement::new(game, faction);
            e.set_target(game, None);
            event(e)
        }
        GameEventType::PerformHmsPlacement => {
            let mut e = PerformHmsPlacement::new(game, faction);
            e.set_target(game, None);
            event(e)
        }
        GameEventType::SetShipmentPermission => {
            let mut e = SetShipmentPermission::new(game, faction);
            e.factions = Some(vec![]);
            event(e)
        }
        GameEventType::TerrorPlanted => {
            let mut e = TerrorPlanted::new(game, faction);
            // Removing: the required select picks the first type. Placing: the type is
            // chosen in the pinned panel, which only shows once a stronghold is picked.
            if TerrorPlanted::may_remove_tokens(game, p) {
                e.type_ = TerrorPlanted::valid_terror_types(game, true).next().unwrap_or(TerrorType::None);
            }
            e.set_stronghold(game, None);
            event(e)
        }
        GameEventType::TerrorRevealed => {
            let mut e = TerrorRevealed::new(game, faction);
            e.type_ = TerrorRevealed::get_types(game).next().unwrap_or(TerrorType::None);
            e.set_card_to_give_in_sabotage(None);
            e.set_sneak_attack_to(game, None);
            event(e)
        }
        GameEventType::TestingStationUsed => {
            let mut e = TestingStationUsed::new(game, faction);
            // The required radio over [-1, 1] replaces the initial 0 with -1.
            e.value_added = -1;
            event(e)
        }
        GameEventType::FremenSentMonster => {
            let mut e = FremenSentMonster::new(game, faction);
            e.set_territory(game, None);
            event(e)
        }
        _ => return None,
    })
}

/// The options of the form for `draft`, or `None` when the draft is not one of this module's
/// event types. A helper that panics for the draft leaves the form without options.
pub fn options(game: &Game, p: PlayerIndex, draft: &GameEvent, d: &dyn Describer) -> Option<Option<ActionOptions>> {
    options_and_validation(game, p, draft, d).map(|(o, _)| o)
}

/// The options of the form for `draft` after [`normalize`], carrying the adjusted draft, and the
/// validation of the adjusted draft. `None` when the draft is not one of this module's types.
pub fn options_and_validation(game: &Game, p: PlayerIndex, draft: &GameEvent, d: &dyn Describer) -> Option<(Option<ActionOptions>, Option<RichText>)> {
    if !handles(draft.event_type()) {
        return None;
    }
    let draft = normalized(game, p, draft);
    let options = cs::catch(|| build(game, p, &draft, d)).ok().map(|mut o| {
        set_draft(&mut o, draft_json(&draft));
        o
    });
    Some((options, validate(game, draft, d)))
}

fn set_draft(o: &mut ActionOptions, draft: Value) {
    use ActionOptions as O;
    let slot = match o {
        O::Shipment(x) => &mut x.draft,
        O::Move(x) | O::Caravan(x) | O::FremenRidesMonster(x) => &mut x.draft,
        O::PerformFremenSetup(x) => &mut x.draft,
        O::DiscoveryEntered(x) => &mut x.draft,
        O::AmbassadorActivated(x) => &mut x.draft,
        O::AmbassadorPlaced(x) => &mut x.draft,
        O::BeneGesseritAccompanies(x) => &mut x.draft,
        O::BeneGesseritFlip(x) => &mut x.draft,
        O::ChoamMovePrevention(x) => &mut x.draft,
        O::ChoamRemoveForce(x) => &mut x.draft,
        O::DiscoveryRevealed(x) => &mut x.draft,
        O::FlightUsed(x) | O::TestingStationUsed(x) => &mut x.draft,
        O::KarmaHmsMovement(x) | O::PerformHmsMovement(x) => &mut x.draft,
        O::KarmaShipmentPrevention(x) => &mut x.draft,
        O::PerformBeneGesseritPlacement(x) | O::PerformMoritaniSetup(x) | O::PerformHmsPlacement(x) | O::FremenSentMonster(x) => &mut x.draft,
        O::SetShipmentPermission(x) => &mut x.draft,
        O::TerrorPlanted(x) => &mut x.draft,
        O::TerrorRevealed(x) => &mut x.draft,
        _ => unreachable!("build() returns one of this module's payloads"),
    };
    *slot = draft;
}

/// [`normalize`], or the draft itself when a helper it calls throws (the options builder then
/// reports the failure).
fn normalized(game: &Game, p: PlayerIndex, draft: &GameEvent) -> GameEvent {
    cs::catch(|| normalize(game, p, draft)).unwrap_or_else(|_| draft.clone())
}

/// The draft after one round of the form's adjustments, repeated until it stops changing (each
/// step only clears an input or picks a valid value, so this ends at once).
pub fn normalize(game: &Game, p: PlayerIndex, draft: &GameEvent) -> GameEvent {
    let mut current = draft.clone();
    for _ in 0..8 {
        let next = normalize_once(game, p, &current);
        if json::serialize(&next) == json::serialize(&current) {
            return next;
        }
        current = next;
    }
    current
}

fn normalize_once(game: &Game, p: PlayerIndex, draft: &GameEvent) -> GameEvent {
    let pl = game.player(p);
    let faction = pl.faction;
    match draft {
        // The pinned panel, drawn while the shipment has a destination or goes back to reserves
        // (and Karama does not stop it).
        GameEvent::Shipment(s) => {
            let mut s = (**s).clone();
            let cross = Shipment::may_ship_cross_planet(game, p);
            let prevented = faction != Faction::Fremen && game.prevented_from_shipping(faction) && !cross;
            let ty = s.shipment_type;
            let back = ty == ShipmentType::ShipmentBack;
            let destination = if back { None } else { s.to(game) };
            if !prevented && (destination.is_some() || back) {
                let from_reserves = matches!(ty, ShipmentType::ShipmentNormal | ShipmentType::ShipmentRicheseSecretAlly);
                let (normal_worlds, special_worlds) = if from_reserves {
                    (Shipment::homeworlds_to_ship_from(game, p, false).len(), Shipment::homeworlds_to_ship_from(game, p, true).len())
                } else {
                    (0, 0)
                };
                if normal_worlds <= 1 && special_worlds <= 1 {
                    s.set_force_locations(game, &NetDictionary::new());
                }
                if !Shipment::may_use_no_field(game, p) {
                    s.set_no_field_value(-1);
                    s.set_cunning_no_field_value(-1);
                }
                if !Shipment::may_smuggle(game, p, s.to(game)) || !from_reserves || faction == Faction::Fremen {
                    s.smuggled_amount = 0;
                    s.smuggled_special_amount = 0;
                }
                let may_karma = Shipment::can_karma(game, p) && (faction != Faction::Fremen || ty == ShipmentType::ShipmentSiteToSite) && !cross;
                if !may_karma {
                    s.set_karma_card(None);
                }
                let cost = Shipment::determine_cost(game, p, &s);
                let ally_can_pay = if pl.ally != Faction::None && cost > 0 { game.resources_your_ally_can_pay(p) } else { 0 };
                if ally_can_pay == 0 {
                    s.ally_contribution_amount = 0;
                }
            }
            event(s)
        }
        // The pinned panel, drawn while there is a destination.
        GameEvent::Move(e) => {
            let mut e = (**e).clone();
            if let Some(to) = e.to(game) {
                e.as_advisors = as_advisors(game, p, &e.force_locations(game), to, e.as_advisors);
            }
            event(e)
        }
        GameEvent::Caravan(e) => {
            let mut e = (**e).clone();
            if let Some(to) = e.to(game) {
                e.as_advisors = as_advisors(game, p, &e.force_locations(game), to, e.as_advisors);
            }
            event(e)
        }
        // OnPickChanged: the forces that may enter are those next to the pinned stronghold, all
        // of them when first picked. The client clears them when it pins another stronghold.
        GameEvent::DiscoveryEntered(e) => {
            let mut e = (**e).clone();
            let current = e.force_locations(game);
            let mut forces = NetDictionary::new();
            if let Some(ds) = e.to(game).and_then(|l| l.as_discovered_location()) {
                for l in DiscoveryEntered::valid_sources(game, p, ds) {
                    let b = current.get(&l).cloned().unwrap_or_else(|| Battalion::new(faction, pl.forces_in(Some(l)), pl.special_forces_in(Some(l)), l));
                    forces.add(l, b);
                }
            }
            e.set_force_locations(game, &forces);
            event(e)
        }
        GameEvent::AmbassadorActivated(e) => {
            let mut e = (**e).clone();
            let ambassador = AmbassadorActivated::get_ambassador(game);
            let actual = if ambassador == Ambassador::BeneGesserit {
                required(e.bene_gesserit_selected_ambassador, &AmbassadorActivated::get_valid_bene_gesserit_ambassadors(game), Ambassador::None)
            } else {
                ambassador
            };
            e.bene_gesserit_selected_ambassador = actual;
            match actual {
                Ambassador::Ecaz => {
                    if !AmbassadorActivated::vidal_can_be_taken(game, p) {
                        e.ecaz_take_vidal = false;
                    }
                    if !e.ecaz_take_vidal && AmbassadorActivated::alliance_can_be_offered(game, p) {
                        if !(e.ecaz_offer_alliance && AmbassadorActivated::vidal_can_be_offered_to_new_ally(game, p)) {
                            e.ecaz_give_vidal_to_ally = false;
                        }
                    } else {
                        e.ecaz_offer_alliance = false;
                        e.ecaz_give_vidal_to_ally = false;
                    }
                }
                Ambassador::Ixian => {
                    let cards = AmbassadorActivated::get_valid_ixian_cards(game, p);
                    let card = e.ixian_card();
                    if !card.is_some_and(|c| cards.contains(&c)) {
                        e.set_ixian_card(cards.first().copied());
                    }
                }
                Ambassador::Guild => {
                    if AmbassadorActivated::valid_guild_targets(game, p).next().is_none() {
                        e.guild_force_amount = 0;
                    }
                }
                Ambassador::Tleilaxu => {
                    let max = AmbassadorActivated::valid_tleilaxu_max_amount(game, p);
                    e.tleilaxu_amount_of_forces = if max > 0 { e.tleilaxu_amount_of_forces.clamp(0, max) } else { 0 };
                    if !e.tleilaxu_hero().is_some_and(|h| Revival::may_assign_skill(game, p, Some(h))) {
                        e.tleilaxu_assign_skill = false;
                    }
                }
                _ => {}
            }
            event(e)
        }
        GameEvent::AmbassadorPlaced(e) => {
            let mut e = (**e).clone();
            e.ambassador = required(e.ambassador, &AmbassadorPlaced::valid_ambassadors(game, p), Ambassador::None);
            event(e)
        }
        // PinContent, drawn while a location is pinned.
        GameEvent::BeneGesseritAccompanies(e) => {
            let mut e = (**e).clone();
            if let Some(l) = e.location(game) {
                if !BeneGesseritAccompanies::may_send_extra_advisor(game, p, l) {
                    e.extra_advisor = false;
                }
            }
            event(e)
        }
        // OnPickChanged and SetTarget: the faction and force type follow the pinned location.
        GameEvent::ChoamRemoveForce(e) => {
            let mut e = (**e).clone();
            if let Some(l) = e.location(game) {
                let factions: Vec<Faction> = ChoamRemoveForce::valid_factions(game, Some(p), Some(l)).collect();
                e.target = required(e.target, &factions, Faction::None);
                let choices = ChoamRemoveForce::valid_special_force_choices(game, Some(l), e.target);
                e.special_force = required(e.special_force, &choices, false);
            }
            event(e)
        }
        // The selected location falls back to the first location.
        GameEvent::DiscoveryRevealed(e) => {
            let mut e = (**e).clone();
            let locations: Vec<Location> = DiscoveryRevealed::get_locations(game, p).collect();
            if !e.location(game).is_some_and(|l| locations.contains(&l)) {
                e.set_location(game, locations.first().copied());
            }
            event(e)
        }
        GameEvent::KarmaShipmentPrevention(e) => {
            let mut e = (**e).clone();
            let targets: Vec<Faction> = KarmaShipmentPrevention::get_valid_targets(game, p).collect();
            e.target = required(e.target, &targets, Faction::None);
            event(e)
        }
        // Removing: the required select. Placing: the pinned panel, drawn while a stronghold is
        // pinned.
        GameEvent::TerrorPlanted(e) => {
            let mut e = (**e).clone();
            if TerrorPlanted::may_remove_tokens(game, p) {
                let types: Vec<TerrorType> = TerrorPlanted::valid_terror_types(game, true).collect();
                e.type_ = required(e.type_, &types, TerrorType::None);
            } else if e.stronghold(game).is_some() {
                let types: Vec<TerrorType> = TerrorPlanted::valid_terror_types(game, false).collect();
                e.type_ = required(e.type_, &types, TerrorType::None);
            }
            event(e)
        }
        GameEvent::TerrorRevealed(e) => {
            let mut e = (**e).clone();
            let types: Vec<TerrorType> = TerrorRevealed::get_types(game).collect();
            e.type_ = required(e.type_, &types, TerrorType::None);
            if e.type_ != TerrorType::Robbery {
                e.robbery_takes_card = false;
            }
            if e.type_ != TerrorType::Sabotage {
                e.set_card_to_give_in_sabotage(None);
            }
            if e.type_ != TerrorType::SneakAttack || TerrorRevealed::valid_sneak_attack_targets(game, p).next().is_none() {
                e.forces_in_sneak_attack = 0;
            }
            event(e)
        }
        // The required radio over [-1, 1].
        GameEvent::TestingStationUsed(e) => {
            let mut e = (**e).clone();
            e.value_added = required(e.value_added, &[-1, 1], 0);
            event(e)
        }
        _ => draft.clone(),
    }
}

/// The placement form's pinned panel: Bene Gesserit move as advisors where they can't choose and
/// may; everyone else never does.
fn as_advisors(game: &Game, p: PlayerIndex, forces: &NetDictionary<Location, Battalion>, to: Location, current: bool) -> bool {
    if game.player(p).faction != Faction::BeneGesserit {
        return false;
    }
    let advisors = forces.values().any(|b| b.amount_of_special_forces > 0);
    let may_move_as_advisors = PlacementEvent::may_move_as_advisors(game, p, to.territory(), advisors);
    let choice = may_move_as_advisors && PlacementEvent::may_move_as_bene_gesserit_fighters(game, p, to.territory());
    if choice { current } else { may_move_as_advisors }
}

fn handles(t: GameEventType) -> bool {
    use GameEventType as T;
    matches!(
        t,
        T::AmbassadorActivated
            | T::AmbassadorPlaced
            | T::BeneGesseritAccompanies
            | T::BeneGesseritFlip
            | T::ChoamMovePrevention
            | T::ChoamRemoveForce
            | T::Caravan
            | T::DiscoveryEntered
            | T::DiscoveryRevealed
            | T::FlightUsed
            | T::KarmaHmsMovement
            | T::KarmaShipmentPrevention
            | T::Move
            | T::PerformBeneGesseritPlacement
            | T::PerformMoritaniSetup
            | T::PerformHmsMovement
            | T::PerformHmsPlacement
            | T::PerformFremenSetup
            | T::SetShipmentPermission
            | T::Shipment
            | T::TerrorPlanted
            | T::TerrorRevealed
            | T::TestingStationUsed
            | T::FremenRidesMonster
            | T::FremenSentMonster
    )
}

fn build(game: &Game, p: PlayerIndex, draft: &GameEvent, d: &dyn Describer) -> ActionOptions {
    use ActionOptions as O;
    match draft {
        GameEvent::Shipment(s) => O::Shipment(shipment(game, p, s, d)),
        GameEvent::Move(e) => {
            let forces = e.force_locations(game);
            let to = e.to(game);
            let mut o = placement(game, p, &forces, to, PlacementEvent::valid_movement_sources(game, p));
            o.inform_about_caravan = game.player(p).has_treachery_card_type(TreacheryCardType::Caravan);
            O::Move(o)
        }
        GameEvent::Caravan(e) => {
            let forces = e.force_locations(game);
            O::Caravan(placement(game, p, &forces, e.to(game), PlacementEvent::valid_movement_sources(game, p)))
        }
        GameEvent::FremenRidesMonster(e) => O::FremenRidesMonster(fremen_ride(game, p, e, d)),
        GameEvent::PerformFremenSetup(_) => {
            let pl = game.player(p);
            let m = &game.map;
            let mut targets = vec![m.sietch_tabr().id()];
            targets.extend(m.false_wall_south().locations().iter().map(|l| l.id()));
            targets.extend(m.false_wall_west().locations().iter().map(|l| l.id()));
            O::PerformFremenSetup(StagedPlacementOptions { draft: Value::Null,
                targets,
                max_total: 10,
                may_stage_special: pl.has_special_forces(game),
                forces_in_reserve: pl.forces_in_reserve(),
                special_forces_in_reserve: pl.special_forces_in_reserve(),
                places: vec![
                    message(&treachery_engine::express!(m.sietch_tabr()), d),
                    message(&treachery_engine::express!(m.false_wall_south()), d),
                    message(&treachery_engine::express!(m.false_wall_west()), d),
                ],
            })
        }
        GameEvent::DiscoveryEntered(e) => {
            let sources = match e.to(game).and_then(|l| l.as_discovered_location()) {
                Some(ds) => DiscoveryEntered::valid_sources(game, p, ds).map(|l| forces_at(game, p, l)).collect(),
                None => vec![],
            };
            O::DiscoveryEntered(DiscoveryEnteredOptions { draft: Value::Null,
                targets: DiscoveryEntered::valid_targets(game, p).map(|ds| ds.location().id()).collect(),
                sources,
            })
        }
        GameEvent::AmbassadorActivated(e) => O::AmbassadorActivated(ambassador_activated(game, p, e)),
        GameEvent::AmbassadorPlaced(_) => O::AmbassadorPlaced(AmbassadorPlacedOptions { draft: Value::Null,
            ambassadors: AmbassadorPlaced::valid_ambassadors(game, p).into_iter().map(name).collect(),
            targets: sectors_of(AmbassadorPlaced::valid_strongholds(game, p)),
            cost: game.ambassadors_placed_this_turn + 1,
        }),
        GameEvent::BeneGesseritAccompanies(e) => O::BeneGesseritAccompanies(BeneGesseritAccompaniesOptions { draft: Value::Null,
            targets: ids(BeneGesseritAccompanies::valid_targets(game, p)),
            may_send_extra_advisor: e.location(game).is_some_and(|l| BeneGesseritAccompanies::may_send_extra_advisor(game, p, l)),
        }),
        GameEvent::BeneGesseritFlip(_) => O::BeneGesseritFlip(BeneGesseritFlipOptions { draft: Value::Null, territory: BeneGesseritFlip::get_territory(game).id() }),
        GameEvent::ChoamMovePrevention(_) => O::ChoamMovePrevention(ChoamMovePreventionOptions { draft: Value::Null,
            card: ChoamMovePrevention::card_to_use(game, p).map(|c| c.id()),
            targets: sectors_of(ChoamMovePrevention::valid_territories(game, p)),
        }),
        GameEvent::ChoamRemoveForce(e) => {
            let l = e.location(game);
            O::ChoamRemoveForce(ChoamRemoveForceOptions { draft: Value::Null,
                card: ChoamRemoveForce::card_to_use(game, p).map(|c| c.id()),
                targets: ids(ChoamRemoveForce::valid_locations(game)),
                factions: ChoamRemoveForce::valid_factions(game, Some(p), l)
                    .collect::<Vec<_>>()
                    .into_iter()
                    .map(|f| ChoamRemoveForceFaction { faction: name(f), special_choices: ChoamRemoveForce::valid_special_force_choices(game, l, f) })
                    .collect(),
            })
        }
        GameEvent::DiscoveryRevealed(_) => O::DiscoveryRevealed(DiscoveryRevealedOptions { draft: Value::Null,
            locations: DiscoveryRevealed::get_locations(game, p)
                .map(|l| RevealableDiscovery {
                    location: l.id(),
                    territory: l.territory().id(),
                    token: game.discoveries_on_planet.get(&l).map(|disc| name(disc.token)).unwrap_or_default(),
                })
                .collect(),
        }),
        GameEvent::FlightUsed(_) => O::FlightUsed(NoOptions { draft: Value::Null }),
        GameEvent::TestingStationUsed(_) => O::TestingStationUsed(NoOptions { draft: Value::Null }),
        GameEvent::KarmaHmsMovement(_) => {
            O::KarmaHmsMovement(HmsMovementOptions { draft: Value::Null, targets: ids(KarmaHmsMovement::valid_locations(game)), moves_left: game.karma_hms_moves_left })
        }
        GameEvent::PerformHmsMovement(_) => {
            O::PerformHmsMovement(HmsMovementOptions { draft: Value::Null, targets: ids(PerformHmsMovement::valid_locations(game)), moves_left: game.hms_moves_left })
        }
        GameEvent::KarmaShipmentPrevention(_) => O::KarmaShipmentPrevention(FactionTargetsOptions { draft: Value::Null,
            targets: KarmaShipmentPrevention::get_valid_targets(game, p).map(name).collect(),
        }),
        GameEvent::PerformBeneGesseritPlacement(_) => {
            O::PerformBeneGesseritPlacement(LocationPickOptions { draft: Value::Null, targets: ids(PerformBeneGesseritPlacement::valid_locations(game)) })
        }
        GameEvent::PerformMoritaniSetup(_) => O::PerformMoritaniSetup(LocationPickOptions { draft: Value::Null, targets: ids(PerformMoritaniSetup::valid_locations(game)) }),
        GameEvent::PerformHmsPlacement(_) => O::PerformHmsPlacement(LocationPickOptions { draft: Value::Null, targets: ids(PerformHmsPlacement::valid_locations(game, p)) }),
        GameEvent::FremenSentMonster(_) => O::FremenSentMonster(LocationPickOptions { draft: Value::Null, targets: sectors_of(FremenSentMonster::valid_targets(game)) }),
        GameEvent::SetShipmentPermission(_) => O::SetShipmentPermission(SetShipmentPermissionOptions { draft: Value::Null,
            permissions: game
                .shipment_permissions
                .iter()
                .map(|(f, perm)| ShipmentPermissionRow {
                    faction: name(*f),
                    cross: perm.value() & ShipmentPermission::Cross.value() != 0,
                    to_homeworld: perm.value() & ShipmentPermission::ToHomeworld.value() != 0,
                    guild_rate: perm.value() & ShipmentPermission::GuildRate.value() != 0,
                })
                .collect(),
            targets: SetShipmentPermission::valid_targets(game, p).map(name).collect(),
        }),
        GameEvent::TerrorPlanted(_) => O::TerrorPlanted(TerrorPlantedOptions { draft: Value::Null,
            may_remove: TerrorPlanted::may_remove_tokens(game, p),
            remove_types: TerrorPlanted::valid_terror_types(game, true).map(name).collect(),
            place_types: TerrorPlanted::valid_terror_types(game, false).map(name).collect(),
            unplaced: game.unplaced_terror_tokens.iter().map(|t| name(*t)).collect(),
            targets: sectors_of(TerrorPlanted::valid_strongholds(game, p)),
        }),
        GameEvent::TerrorRevealed(_) => O::TerrorRevealed(TerrorRevealedOptions { draft: Value::Null,
            victim: name(TerrorRevealed::get_victim(game)),
            territory: TerrorRevealed::get_territory(game).map(|t| t.id()),
            types: TerrorRevealed::get_types(game).map(name).collect(),
            may_offer_alliance: TerrorRevealed::may_offer_alliance(game),
            may_pass: TerrorRevealed::may_pass(game),
            sneak_attack_targets: if TerrorRevealed::get_territory(game).is_some() { ids(TerrorRevealed::valid_sneak_attack_targets(game, p)) } else { vec![] },
            max_sneak_attack_forces: TerrorRevealed::max_amount_of_forces_in_sneak_attack(game, p),
            cards: game.player(p).treachery_cards.iter().map(|c| c.id()).collect(),
        }),
        _ => unreachable!("handles() lists the event types build() covers"),
    }
}

fn shipment(game: &Game, p: PlayerIndex, s: &Shipment, d: &dyn Describer) -> ShipmentOptions {
    let pl = game.player(p);
    let faction = pl.faction;
    let cross = Shipment::may_ship_cross_planet(game, p);
    let to_reserves = Shipment::may_ship_to_reserves(game, p);
    let from = s.from(game);
    let to = s.to(game);
    let ty = s.shipment_type;
    let no_field = s.no_field_value();

    let sources = if cross || to_reserves { Shipment::valid_source_locations(game, p).map(|l| forces_at(game, p, l)).collect() } else { vec![] };
    let targets: Vec<i32> = match ty {
        ShipmentType::ShipmentNormal => ids(Shipment::valid_shipment_locations(game, p, false, false)),
        ShipmentType::ShipmentRicheseSecretAlly => ids(Shipment::valid_shipment_locations(game, p, false, true)),
        ShipmentType::ShipmentSiteToSite => ids(Shipment::valid_shipment_locations(game, p, true, false)),
        _ => vec![],
    };
    let max = |special: bool| match ty {
        ShipmentType::ShipmentNormal => Shipment::valid_max_normal_shipment_forces(game, p, special, no_field),
        ShipmentType::ShipmentRicheseSecretAlly => Shipment::valid_max_secret_ally_shipment_forces(game, p, special),
        ShipmentType::ShipmentSiteToSite => Shipment::valid_max_shipment_site_to_site_forces(game, p, special, from),
        ShipmentType::ShipmentBack => Shipment::valid_max_shipment_back_forces(game, p, special, to),
        _ => 0,
    };
    let may_ship_here_from_reserves = from.is_some_and(|f| {
        !game.prevented_from_shipping(faction) && Shipment::valid_shipment_locations(game, p, false, false).any(|l| l == f)
    });

    let normal_worlds = Shipment::homeworlds_to_ship_from(game, p, false);
    let special_worlds = Shipment::homeworlds_to_ship_from(game, p, true);
    // normalWorlds.Union(specialWorlds): first-seen order, no duplicates.
    let mut worlds: Vec<Homeworld> = vec![];
    for w in normal_worlds.iter().chain(special_worlds.iter()) {
        if !worlds.contains(w) {
            worlds.push(*w);
        }
    }
    let homeworlds = worlds
        .into_iter()
        .map(|w| ShipmentWorld {
            location: w.location().id(),
            label: d.describe(&ExpressionElement::from(w)),
            normal: normal_worlds.contains(&w),
            special: special_worlds.contains(&w),
            forces: pl.forces_in(Some(w.location())),
            special_forces: pl.special_forces_in(Some(w.location())),
        })
        .collect();

    let may_karma = Shipment::can_karma(game, p) && (faction != Faction::Fremen || ty == ShipmentType::ShipmentSiteToSite) && !cross;
    let cost = Shipment::determine_cost(game, p, s);
    // The destination is `to`, except for a shipment back to reserves.
    let destination = if ty == ShipmentType::ShipmentBack { None } else { to };

    ShipmentOptions { draft: Value::Null,
        may_ship_cross_planet: cross,
        may_ship_to_reserves: to_reserves,
        prevented_from_shipping: faction != Faction::Fremen && game.prevented_from_shipping(faction) && !cross,
        sources,
        targets,
        max_forces: max(false),
        max_special_forces: max(true),
        may_ship_here_from_reserves,
        may_use_richese_secret_ally: Shipment::may_use_richese_secret_ally(game, p),
        may_use_no_field: Shipment::may_use_no_field(game, p),
        no_field_values: Shipment::valid_no_field_values(game, p),
        may_use_cunning_no_field: Shipment::may_use_cunning_no_field(game, p),
        cunning_no_field_values: Shipment::valid_cunning_no_field_values(game, p, no_field),
        default_no_field_special_force_amount: Shipment::default_no_field_special_force_amount(faction, no_field),
        richese_high_threshold: game.has_high_threshold(Faction::Richese),
        homeworlds,
        may_smuggle: Shipment::may_smuggle(game, p, to),
        may_karma,
        karma_cards: Shipment::valid_karma_cards(game, p).into_iter().map(|c| KarmaChoice { card: c.id(), own: pl.treachery_cards.contains(&c) }).collect(),
        cost,
        ally_can_pay: if pl.ally != Faction::None && cost > 0 { game.resources_your_ally_can_pay(p) } else { 0 },
        conflicting_ally: destination.is_some_and(|l| game.contains_conflicting_ally(p, l)),
        great_flat: game.map.the_great_flat().territory().id(),
        karma_name: d.describe(&ExpressionElement::from(TreacheryCardType::Karma)),
    }
}

/// The placement form for a regular move or a Caravan: sources are the sectors of
/// `valid_movement_sources` that hold forces and are out of the storm.
fn placement(game: &Game, p: PlayerIndex, forces: &NetDictionary<Location, Battalion>, to: Option<Location>, valid_sources: Vec<Territory>) -> PlacementOptions {
    let pl = game.player(p);
    let sources = valid_sources
        .iter()
        .flat_map(|t| t.locations().iter().copied())
        .filter(|l| pl.any_forces_in(Some(*l)) > 0 && !game.is_in_storm(*l))
        .map(|l| forces_at(game, p, l))
        .collect();
    let any_forces = forces.values().map(|b| b.amount_of_forces + b.amount_of_special_forces).sum::<i32>() > 0;
    let targets = if any_forces { ids(PlacementEvent::valid_targets_game_player_dictionary_location_battalion(game, p, forces)) } else { vec![] };
    let planetology = game.current_planetology.as_ref();
    let mut o = PlacementOptions { draft: Value::Null,
        title: None,
        sources,
        targets,
        may_move_from_two_territories: planetology.is_some_and(|pl| pl.move_from_two_territories() && pl.initiator() == game.player(p).faction),
        inform_about_caravan: false,
        from_reserves: false,
        max_from_reserves: 0,
        max_special_from_reserves: 0,
        advisor_choice: false,
        as_advisors: false,
        conflicting_ally: false,
    };
    destination_options(game, p, forces, to, &mut o);
    o
}

/// The pinned panel's inputs for the pinned destination.
fn destination_options(game: &Game, p: PlayerIndex, forces: &NetDictionary<Location, Battalion>, to: Option<Location>, o: &mut PlacementOptions) {
    let Some(to) = to else { return };
    if game.player(p).faction == Faction::BeneGesserit {
        let advisors = forces.values().any(|b| b.amount_of_special_forces > 0);
        let may_move_as_advisors = PlacementEvent::may_move_as_advisors(game, p, to.territory(), advisors);
        o.advisor_choice = may_move_as_advisors && PlacementEvent::may_move_as_bene_gesserit_fighters(game, p, to.territory());
        if !o.advisor_choice {
            o.as_advisors = may_move_as_advisors;
        }
    }
    o.conflicting_ally = game.contains_conflicting_ally(p, to);
}

fn fremen_ride(game: &Game, p: PlayerIndex, e: &FremenRidesMonster, d: &dyn Describer) -> PlacementOptions {
    let to_ride = FremenRidesMonster::to_ride(game);
    let ride_sources: Vec<Location> = if to_ride.is_some() { FremenRidesMonster::valid_sources(game).collect() } else { vec![] };
    let max_from_reserves = FremenRidesMonster::max_forces_from_reserves(game, p, false);
    let max_special_from_reserves = FremenRidesMonster::max_forces_from_reserves(game, p, true);
    let from_reserves = ride_sources.is_empty() && (max_from_reserves > 0 || max_special_from_reserves > 0);
    let forces = e.force_locations(game);
    let title = match to_ride {
        None => treachery_engine::express!("Ride ", Concept::Monster, "?"),
        Some(m) if m.is_great_monster => treachery_engine::express!("Ride the ", Concept::GreatMonster, " with forces from your reserves?"),
        Some(m) => treachery_engine::express!("Ride the Worm that appeared in ", m.territory, "?"),
    };
    let targets = if from_reserves || !forces.is_empty() { ids(FremenRidesMonster::valid_targets(game, p)) } else { vec![] };
    let mut o = PlacementOptions { draft: Value::Null,
        title: Some(message(&title, d)),
        sources: ride_sources.iter().map(|l| forces_at(game, p, *l)).collect(),
        targets,
        may_move_from_two_territories: false,
        inform_about_caravan: false,
        from_reserves,
        max_from_reserves,
        max_special_from_reserves,
        advisor_choice: false,
        as_advisors: false,
        conflicting_ally: false,
    };
    destination_options(game, p, &forces, e.to(game), &mut o);
    // FremenRidesMonster has no AsAdvisors; only Fremen ride.
    o.advisor_choice = false;
    o.as_advisors = false;
    o
}

fn ambassador_activated(game: &Game, p: PlayerIndex, e: &AmbassadorActivated) -> AmbassadorActivatedOptions {
    let pl = game.player(p);
    AmbassadorActivatedOptions { draft: Value::Null,
        ambassador: name(AmbassadorActivated::get_ambassador(game)),
        victim: name(AmbassadorActivated::get_victim(game)),
        bene_gesserit_ambassadors: AmbassadorActivated::get_valid_bene_gesserit_ambassadors(game).into_iter().map(name).collect(),
        choam_cards: AmbassadorActivated::get_valid_choam_cards(game, p).iter().map(|c| c.id()).collect(),
        ixian_cards: AmbassadorActivated::get_valid_ixian_cards(game, p).iter().map(|c| c.id()).collect(),
        vidal_can_be_taken: AmbassadorActivated::vidal_can_be_taken(game, p),
        alliance_can_be_offered: AmbassadorActivated::alliance_can_be_offered(game, p),
        vidal_can_be_offered_to_new_ally: AmbassadorActivated::vidal_can_be_offered_to_new_ally(game, p),
        vidal: game.vidal().map(|l| l.id()),
        fremen_sources: AmbassadorActivated::valid_fremen_sources(game, p)
            .iter()
            .flat_map(|t| t.locations().iter().copied())
            .filter(|l| pl.any_forces_in(Some(*l)) > 0 && !game.is_in_storm(*l))
            .map(|l| forces_at(game, p, l))
            .collect(),
        fremen_targets: ids(AmbassadorActivated::valid_fremen_targets(game, p)),
        guild_targets: ids(AmbassadorActivated::valid_guild_targets(game, p)),
        guild_max_forces: AmbassadorActivated::valid_guild_max_forces(game, p),
        tleilaxu_max_amount: AmbassadorActivated::valid_tleilaxu_max_amount(game, p),
        tleilaxu_heroes: AmbassadorActivated::valid_tleilaxu_heroes(game, p).into_iter().map(|h| HeroOption { hero: hero_ref(h), id: h.id() }).collect(),
        may_assign_skill: e.tleilaxu_hero().is_some_and(|h| Revival::may_assign_skill(game, p, Some(h))),
        may_pass: AmbassadorActivated::may_pass(game, p),
    }
}
