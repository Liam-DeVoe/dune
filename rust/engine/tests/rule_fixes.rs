//! Rules bugs copied from C# and fixed on purpose (rust/docs/behavior-changes.md). Each test
//! replays a design fixture (scripts/fixtures), adjusts the state where the fixture lacks the
//! situation, and submits events with validation, as the server does.

use std::path::PathBuf;
use std::rc::Rc;

use treachery_engine::game::{GameState, Intrusion};
use treachery_engine::prelude::*;

fn fixture(name: &str) -> Game {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scripts/fixtures").join(format!("{name}.json"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let state = GameState::load(&text).expect("savegame");
    let mut game = Game::of_version(state.version);
    for e in state.events() {
        let event_name = e.event_type().name();
        assert!(game.play_for_message(e, false, false).is_none(), "{name}: {} failed", event_name);
    }
    game
}

/// Submits `e` with validation; the refusal, if any.
fn submit(g: &mut Game, make: impl FnOnce(&Game) -> Event) -> Option<String> {
    let e = make(g);
    g.play_for_message(HistoryEntry::legacy(e), true, false).map(|m| m.plain_text())
}

/// Submits `e` by `faction` with validation; the refusal, if any.
fn submit_by(g: &mut Game, faction: impl Into<Option<Faction>>, make: impl FnOnce(&Game) -> Event) -> Option<String> {
    let e = make(g);
    g.play_for_message(HistoryEntry::new(faction, e), true, false).map(|m| m.plain_text())
}

fn idx(g: &Game, f: Faction) -> PlayerId {
    g.get_player(Some(f)).expect("faction in game")
}

fn location(g: &Game, id: i32) -> Location {
    g.board.map.location_lookup().find(id).expect("location")
}

fn ally(g: &mut Game, a: Faction, b: Faction) {
    let (pa, pb) = (idx(g, a), idx(g, b));
    g.player_mut(pa).ally = Some(b);
    g.player_mut(pb).ally = Some(a);
}

/// Emperor ships 3 forces and 2 Sardaukar back to reserves. Before the fix the Sardaukar were
/// paid for and stayed on the planet.
#[test]
fn shipping_back_to_reserves_moves_special_forces_too() {
    let mut g = fixture("ship");
    let emperor = idx(&g, Faction::Emperor);
    // Make it Emperor's turn to ship, if it isn't in the fixture.
    g.ship_move.shipment_and_move_sequence = Some(PlayerSequence::new(&g, false, 1, Some(emperor), false));
    assert_eq!(g.ship_move.shipment_and_move_sequence.as_ref().unwrap().current_player(&g), Some(emperor));
    // The fixture has no Emperor forces on the planet: put some on an empty sand sector.
    let sand = g
        .board.map
        .locations(false)
        .find(|l| {
            !l.territory().is_stronghold()
                && l.sector(&g.board.map) != g.board.sector_in_storm
                && g.players.iter().all(|p| p.any_forces_in(Some(*l)) == 0)
        })
        .unwrap();
    let home = g.player(emperor).home_worlds[0];
    Player::move_forces(&mut g, emperor, home.location(), sand, 3).unwrap();
    let sardaukar_home = g.player(emperor).home_worlds.iter().copied().find(|hw| g.player(emperor).special_forces_in(Some(hw.location())) > 0).unwrap();
    Player::move_special_forces(&mut g, emperor, sardaukar_home.location(), sand, 2).unwrap();
    // As if the Guild had given Emperor the permission (SetShipmentPermission).
    g.ship_move.shipment_permissions.insert(Faction::Emperor, ShipmentPermission::TO_HOMEWORLD);

    let (reserve, special_reserve, spice) = {
        let pl = g.player(emperor);
        (pl.forces_in_reserve(), pl.special_forces_in_reserve(), i32::from(pl.resources))
    };
    let shipment = |forces: i32, specials: i32, _g: &Game| {
        let mut s = Shipment::default();
        s.shipment_type = ShipmentType::ShipmentBack;
        s.karma_card = None;
        s.set_no_field_value(-1);
        s.set_cunning_no_field_value(-1);
        s.to = Some(sand);
        s.force_amount = forces;
        s.special_force_amount = specials;
        Event::Shipment(s)
    };

    // More than there are, or amounts with the wrong sign, are refused.
    assert_eq!(submit_by(&mut g, Faction::Emperor, |g| shipment(-3, -3, g)).as_deref(), Some("Not enough Emperor to ship back to reserves"));
    assert_eq!(submit_by(&mut g, Faction::Emperor, |g| shipment(-4, 0, g)).as_deref(), Some("Not enough Emperor to ship back to reserves"));
    assert_eq!(submit_by(&mut g, Faction::Emperor, |g| shipment(2, -2, g)).as_deref(), Some("Can't ship forces from reserves when shipping back to reserves"));

    assert_eq!(submit_by(&mut g, Faction::Emperor, |g| shipment(-3, -2, g)), None);
    let pl = g.player(emperor);
    assert_eq!(pl.any_forces_in(Some(sand)), 0, "nothing stays behind");
    assert_eq!(pl.forces_in_reserve(), reserve + 3);
    assert_eq!(pl.special_forces_in_reserve(), special_reserve + 2);
    assert_eq!(i32::from(pl.resources), spice - 3, "half of 5 forces, rounded up");
}

/// A Bene Gesserit intrusion waits in one territory while the latest move went elsewhere (a
/// terror sneak attack, for example, doesn't update the latest move). The flip must happen where
/// the prompt and log say: in the intruded territory.
#[test]
fn bene_gesserit_flip_happens_in_the_intruded_territory() {
    let mut g = fixture("bene_gesserit");
    let bg = idx(&g, Faction::BeneGesserit);
    // Two locations, in different territories, with Bene Gesserit advisors and no fighters.
    let advisors: Vec<(Location, i32)> = g
        .player(bg)
        .forces_on_planet()
        .iter()
        .filter(|(_, b)| b.amount_of_forces == 0 && b.amount_of_special_forces > 0)
        .map(|(l, b)| (*l, b.special_forces()))
        .collect();
    let (intruded, n_intruded) = advisors[0];
    let (elsewhere, n_elsewhere) = *advisors.iter().find(|(l, _)| l.territory() != intruded.territory()).expect("advisors in two territories");

    g.ship_move.intrusions.push(Intrusion { initiator: Faction::Emperor, to: Some(intruded), type_: IntrusionType::BeneGesseritIntrusion });
    g.ship_move.last_shipment_or_movement = Some(ShipmentOrMovement { to: Some(elsewhere) });
    g.turn.current_phase = Phase::BeneGesseritIntrudedByNonGuildMove;

    let mut flip = BeneGesseritFlip::default();
    flip.as_advisors = false;
    assert_eq!(submit_by(&mut g, Faction::BeneGesserit, |_| Event::BeneGesseritFlip(flip)), None);
    let log = g.current_report().messages.iter().map(|m| m.plain_text()).collect::<Vec<_>>();
    assert!(log.iter().any(|m| m.contains("become")), "{log:?}");
    let pl = g.player(bg);
    assert_eq!((pl.forces_in(Some(intruded)), pl.special_forces_in(Some(intruded))), (n_intruded, 0), "flipped where intruded");
    assert_eq!((pl.forces_in(Some(elsewhere)), pl.special_forces_in(Some(elsewhere))), (0, n_elsewhere), "left alone");
}

/// Fremen, allied with Emperor, revive one Fedaykin with Emperor's money. That is their one
/// special force revival this turn, so a second one is refused.
#[test]
fn special_force_paid_by_emperor_counts_toward_the_limit() {
    let mut g = fixture("revival-planet");
    let fremen = idx(&g, Faction::Fremen);
    ally(&mut g, Faction::Fremen, Faction::Emperor);
    g.player_mut(fremen).special_forces_killed = 2;

    let mut permission = AllyPermission::new(&g, Faction::Emperor);
    permission.emperor_will_pay_for_extra_revival = 3;
    assert_eq!(submit(&mut g, |_| Event::AllyPermission(Rc::new(permission))), None);

    let revival = |own: u16, by_emperor: u16, g: &Game| {
        let mut r = Revival::new(g, Faction::Fremen);
        r.hero = None;
        r.amount_of_special_forces = own;
        r.extra_special_forces_paid_by_emperor = by_emperor;
        Event::Revival(Rc::new(r))
    };
    assert_eq!(submit(&mut g, |g| revival(0, 1, g)), None);
    assert_eq!(g.player(fremen).special_forces_killed, 1);
    assert_eq!(submit(&mut g, |g| revival(1, 0, g)).as_deref(), Some("You already revived a Fremen this turn"));
    assert_eq!(g.player(fremen).special_forces_killed, 1);
}

/// Fremen's Karama worm needs a territory the worm can reach. Before the fix, confirming without
/// one spent the card and Fremen's Karama power, and no worm appeared.
#[test]
fn karama_worm_needs_a_territory() {
    let mut g = fixture("karma-monster");
    let fremen = idx(&g, Faction::Fremen);
    let monster = |t: Option<Territory>, _g: &Game| {
        let mut m = KarmaMonster::default();
        m.territory = t;
        Event::KarmaMonster(m)
    };
    let karama_cards = |g: &Game| g.player(fremen).treachery_cards.iter().filter(|c| c.type_() == TreacheryCardType::Karma).count();
    if karama_cards(&g) == 0 {
        let held: Vec<TreacheryCard> = g.players.iter().flat_map(|p| p.treachery_cards.clone()).collect();
        let karama = TreacheryCardManager::items().iter().copied().find(|c| c.type_() == TreacheryCardType::Karma && !held.contains(c)).unwrap();
        g.player_mut(fremen).treachery_cards.push(karama);
    }
    assert_eq!(karama_cards(&g), 1);

    assert_eq!(submit_by(&mut g, Faction::Fremen, |g| monster(None, g)).as_deref(), Some("No territory selected"));
    let stronghold = g.board.map.territories(false).find(|t| t.is_stronghold()).unwrap();
    assert_eq!(submit_by(&mut g, Faction::Fremen, |g| monster(Some(stronghold), g)).as_deref(), Some("Selected territory is protected"));
    assert_eq!(karama_cards(&g), 1);
    assert!(!g.player(fremen).special_karma_power_used);

    let monsters = g.blow.number_of_monsters;
    let target = KarmaMonster::valid_targets(&g).next().unwrap();
    assert_eq!(submit_by(&mut g, Faction::Fremen, |g| monster(Some(target), g)), None);
    assert_eq!(karama_cards(&g), 0);
    assert!(g.player(fremen).special_karma_power_used);
    assert_eq!(g.blow.number_of_monsters, monsters + 1);
}

/// Ally permissions are stored under the ally. Without an ally there is nothing to store (the
/// event isn't applicable then, but savegames replay events without validation).
#[test]
fn ally_permissions_without_an_ally_store_nothing() {
    let mut g = fixture("ship");
    let mut permission = AllyPermission::new(&g, Faction::Guild);
    permission.permitted_resources = 3;
    let e = HistoryEntry::new(Faction::Guild, Rc::new(permission));
    assert!(g.play_for_message(e, false, false).is_none());
    assert!(g.alliances.permitted_use_of_ally_spice.is_empty(), "{:?}", g.alliances.permitted_use_of_ally_spice);
}

/// Rule::RicheseTreacheryCards puts Richese's cards in the treachery deck. With Richese playing
/// they are also its cache, so a card could be sold in a normal auction and then again by
/// Richese, and the next `TreacheryCalled` failed with "Card 50 is in more than one hand"
/// (observe bots seed 1 game 163). A card taken from one of the two now leaves the other.
#[test]
fn a_richese_card_taken_from_the_deck_or_the_cache_leaves_the_other() {
    let mut g = Game::of_version(Game::LATEST_VERSION);
    let mut s = GameSettings::new();
    s.number_of_players = 3;
    s.allowed_factions_in_play = Some(vec![Faction::Richese, Faction::Atreides, Faction::Emperor]);
    s.initial_rules = Some(vec![Rule::RicheseTreacheryCards]);
    let mut establish = EstablishPlayers::new(&g, None);
    establish.seed = 7;
    establish.settings = Some(s);
    assert!(g.play_for_message(HistoryEntry::new(None, Rc::new(establish)), false, true).is_none());
    let end = HistoryEntry::new(None, Rc::new(EndPhase::new(&g, None)));
    assert!(g.play_for_message(end, false, true).is_none());

    let in_deck = |g: &Game, c: TreacheryCard| g.decks.treachery_deck.as_ref().unwrap().items.contains(&c);
    let both: Vec<TreacheryCard> = g.decks.richese_cache.iter().copied().filter(|c| in_deck(&g, *c)).collect();
    assert!(both.len() >= 2, "Richese cards in the deck and the cache: {both:?}");
    let (drawn, sold) = (both[0], both[1]);

    // Drawn from the deck (for an auction, or as a starting card): Richese no longer has it.
    let items = &mut g.decks.treachery_deck.as_mut().unwrap().items;
    let i = items.iter().position(|c| *c == drawn).unwrap();
    items.remove(i);
    g.decks.treachery_deck.as_mut().unwrap().put_on_top(drawn);
    assert_eq!(g.draw_treachery_card(), Some(drawn));
    assert!(!g.decks.richese_cache.contains(&drawn));

    // Sold from the cache: it leaves the deck.
    g.bidding.cards_on_auction = Some(Deck::empty(None));
    g.turn.current_phase = Phase::RicheseSpecifyingAuction;
    let mut auction = RicheseSpecifiesAuction::default();
    auction.card = Some(sold);
    auction.auction_type = Some(AuctionType::RicheseOnceAround);
    auction.direction = 1;
    assert!(g.play_for_message(HistoryEntry::new(Faction::Richese, auction), false, false).is_none());
    assert!(!g.decks.richese_cache.contains(&sold));
    assert!(!in_deck(&g, sold));
}

/// A Karama bid that also passes, or names no Karama card of the bidder's, passed validation and
/// then threw halfway through, after changing the game. The changes weren't in the history, so a
/// later undo replayed another game, one with a Bene Gesserit intrusion that was never answered
/// (observe fuzz seed 42 game 110). Both are refused now.
#[test]
fn a_karama_bid_needs_a_karama_card_and_no_pass() {
    let mut g = fixture("bid");
    let bidder = g.bidding.bid_sequence.as_ref().unwrap().current_player(&g).expect("a bidder");
    let faction = g.player(bidder).faction;
    let held: Vec<TreacheryCard> = g.players.iter().flat_map(|p| p.treachery_cards.clone()).collect();
    let karama = TreacheryCardManager::items().iter().copied().find(|c| c.type_() == TreacheryCardType::Karma && !held.contains(c)).unwrap();
    g.player_mut(bidder).treachery_cards.push(karama);
    let bid = |passed: bool, card: Option<TreacheryCard>, _g: &Game| {
        let mut b = Bid::default();
        b.passed = passed;
        b.karma_bid = true;
        b.karma_card = card;
        Event::Bid(b)
    };
    let history = g.history.entries.len();
    assert_eq!(submit_by(&mut g, faction, |g| bid(true, Some(karama), g)).as_deref(), Some("You can't pass and use Karma in the same bid"));
    assert_eq!(submit_by(&mut g, faction, |g| bid(false, None, g)).as_deref(), Some("Invalid Karma card"));
    assert_eq!(g.history.entries.len(), history);
    assert!(g.player(bidder).treachery_cards.contains(&karama));

    assert_eq!(submit_by(&mut g, faction, |g| bid(false, Some(karama), g)), None);
    assert!(!g.player(bidder).treachery_cards.contains(&karama), "the Karama card is spent");
}

/// An intrusion is answered right after the shipment or move that caused it. One left queued
/// when a new phase starts can't be answered any more; before, it came up after the next
/// shipment, in place of that shipment's own (observe fuzz seed 42 game 110).
#[test]
fn an_intrusion_left_over_from_an_earlier_phase_is_dropped() {
    let mut g = fixture("bene_gesserit");
    let mut m = Move::default();
    m.to = Some(location(&g, 5));
    g.ship_move.intrusions.push(Intrusion { initiator: Faction::Emperor, to: m.to, type_: IntrusionType::BeneGesseritIntrusion });
    g.main_phase_start(MainPhase::Battle, false);
    assert!(g.ship_move.intrusions.is_empty());
}

/// An ally pays for a shipment only spice it has and has offered. C# checked neither, so a
/// refitted game shipped with spice the ally didn't have and left it at -1 (the `alliance-allied`
/// fixture, after the change to ChaCha8). Bid and Battle already checked both.
#[test]
fn an_ally_pays_for_a_shipment_only_spice_it_has_and_offered() {
    let mut g = fixture("ship");
    let shipper = g.ship_move.shipment_and_move_sequence.as_ref().unwrap().current_player(&g).expect("a shipper");
    let f = g.player(shipper).faction.unwrap();
    let other = g.players.iter().filter_map(|p| p.faction).find(|x| *x != f && *x != Faction::Guild).unwrap();
    ally(&mut g, f, other);
    let pally = idx(&g, other);
    let to = Shipment::valid_shipment_locations(&g, shipper, false, false).find(|l| l.territory().is_stronghold()).expect("a stronghold to ship to");
    let shipment = |_g: &Game, contribution: u16| {
        let mut s = Shipment::default();
        s.shipment_type = ShipmentType::ShipmentNormal;
        s.karma_card = None;
        s.set_no_field_value(-1);
        s.set_cunning_no_field_value(-1);
        s.to = Some(to);
        s.force_amount = 2;
        s.ally_contribution_amount = contribution;
        s
    };
    let cost = Shipment::determine_cost(&g, shipper, &shipment(&g, 0));
    assert!(cost >= 2, "cost {cost}");
    g.player_mut(shipper).resources = 0;

    // The ally offered enough but has less.
    g.alliances.permitted_use_of_ally_spice.insert(f, cost);
    g.player_mut(pally).resources = (cost - 1) as u16;
    assert_eq!(submit_by(&mut g, f, |g| Event::Shipment(shipment(g, cost as u16))).as_deref(), Some("Your ally won't pay that much"));
    // The ally has enough but offered less.
    g.alliances.permitted_use_of_ally_spice.insert(f, cost - 1);
    g.player_mut(pally).resources = cost as u16;
    assert_eq!(submit_by(&mut g, f, |g| Event::Shipment(shipment(g, cost as u16))).as_deref(), Some("Your ally won't pay that much"));

    g.alliances.permitted_use_of_ally_spice.insert(f, cost);
    assert_eq!(submit_by(&mut g, f, |g| Event::Shipment(shipment(g, cost as u16))), None);
    assert_eq!(i32::from(g.player(pally).resources), 0);
    assert_eq!(i32::from(g.player(shipper).resources), 0);
    assert_eq!(g.alliances.permitted_use_of_ally_spice[&f], 0);
}

/// A revival fixture and a faction other than the Tleilaxu that may revive, given 30 spice.
fn revival_fixture(name: &str) -> (Game, PlayerId) {
    let mut g = fixture(name);
    let candidates: Vec<PlayerId> = g.player_indices().filter(|p| g.player(*p).faction != Some(Faction::Tleilaxu)).collect();
    let p = candidates
        .into_iter()
        .find(|p| g.get_applicable_events(Some(*p), false).contains(&EventType::Revival))
        .expect("a faction that may revive");
    g.player_mut(p).resources = 30;
    (g, p)
}

/// Reviving a leader who was alive put them in the tanks, with no time of death. Before the fix
/// `Revival` and `KarmaFreeRevival` accepted it.
#[test]
fn only_a_dead_leader_can_be_revived() {
    let (mut g, p) = revival_fixture("revival");
    let f = g.player(p).faction;
    let leader = g.player(p).leaders[0];
    assert!(g.is_alive(Hero::Leader(leader)));

    let revival = |g: &Game| {
        let mut r = Revival::new(g, f);
        r.hero = Some(Hero::Leader(leader));
        Event::Revival(Rc::new(r))
    };
    let karama_revival = |g: &Game| {
        let mut r = KarmaFreeRevival::new(g, f);
        r.hero = Some(Hero::Leader(leader));
        r.validate(g).map(|m| m.plain_text())
    };
    let refusal = Some("You can't revive a leader who isn't dead");
    assert_eq!(submit(&mut g, revival).as_deref(), refusal);
    assert_eq!(karama_revival(&g).as_deref(), refusal);
    assert!(g.is_alive(Hero::Leader(leader)));

    g.kill_hero(Hero::Leader(leader));
    assert_eq!(karama_revival(&g), None);
    assert_eq!(submit(&mut g, revival), None);
    assert!(g.is_alive(Hero::Leader(leader)));
}

/// A living leader the Tleilaxu hold as a ghola isn't dead, but their own faction can still
/// take them back with a revival.
#[test]
fn a_living_ghola_can_be_taken_back() {
    let (mut g, p) = revival_fixture("revival-planet");
    let f = g.player(p).faction;
    let tleilaxu = idx(&g, Faction::Tleilaxu);
    let leader = g.player(p).leaders[0];
    g.player_mut(p).leaders.retain(|l| *l != leader);
    g.player_mut(tleilaxu).leaders.push(leader);

    let mut r = Revival::new(&g, f);
    r.hero = Some(Hero::Leader(leader));
    assert_eq!(submit(&mut g, |_| Event::Revival(Rc::new(r))), None);
    assert!(g.is_alive(Hero::Leader(leader)));
    assert!(g.player(p).leaders.contains(&leader));
    assert!(!g.player(tleilaxu).leaders.contains(&leader));
}
