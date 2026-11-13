//! Rules bugs copied from C# and fixed on purpose (rust/docs/behavior-changes.md). Each test
//! replays a design fixture (scripts/fixtures), adjusts the state where the fixture lacks the
//! situation, and submits events with validation, as the server does.

use std::path::PathBuf;
use std::rc::Rc;

use treachery_engine::game::{GameState, Intrusion, Participation};
use treachery_engine::prelude::*;
use treachery_engine::shared::Shared;

fn fixture(name: &str) -> Game {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scripts/fixtures").join(format!("{name}.json"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let state = GameState::load(&text).expect("savegame");
    let mut game = Game::new_int32_participation(state.version, &Shared::new(Participation::default()));
    for e in state.events() {
        e.initialize_game(&game);
        assert!(e.execute(&mut game, false, false).is_none(), "{name}: {} failed", e.event_type().name());
    }
    game
}

/// Submits `e` with validation; the refusal, if any.
fn submit(g: &mut Game, make: impl FnOnce(&Game) -> GameEvent) -> Option<String> {
    let e = make(g);
    e.execute(g, true, false).map(|m| m.to_cs_string())
}

fn idx(g: &Game, f: Faction) -> PlayerIndex {
    g.get_player(Some(f)).expect("faction in game")
}

fn location(g: &Game, id: i32) -> Location {
    g.map.location_lookup().find(id).expect("location")
}

fn ally(g: &mut Game, a: Faction, b: Faction) {
    let (pa, pb) = (idx(g, a), idx(g, b));
    g.player_mut(pa).ally = b;
    g.player_mut(pb).ally = a;
}

/// Emperor ships 3 forces and 2 Sardaukar back to reserves. Before the fix the Sardaukar were
/// paid for and stayed on the planet.
#[test]
fn shipping_back_to_reserves_moves_special_forces_too() {
    let mut g = fixture("ship");
    let emperor = idx(&g, Faction::Emperor);
    assert_eq!(g.shipment_and_move_sequence.as_ref().unwrap().current_player(&g), Some(emperor));
    // The fixture has no Emperor forces on the planet: put some on an empty sand sector.
    let sand = g
        .map
        .locations(false)
        .find(|l| {
            !l.territory().is_stronghold()
                && l.sector(&g.map) != g.sector_in_storm
                && g.players.iter().all(|p| p.any_forces_in(Some(*l)) == 0)
        })
        .unwrap();
    let home = g.player(emperor).home_worlds[0];
    Player::move_forces(&mut g, emperor, home.location(), sand, 3);
    let sardaukar_home = g.player(emperor).home_worlds.iter().copied().find(|hw| g.player(emperor).special_forces_in(Some(hw.location())) > 0).unwrap();
    Player::move_special_forces(&mut g, emperor, sardaukar_home.location(), sand, 2);
    // As if the Guild had given Emperor the permission (SetShipmentPermission).
    g.shipment_permissions.insert(Faction::Emperor, ShipmentPermission::ToHomeworld);

    let (reserve, special_reserve, spice) = {
        let pl = g.player(emperor);
        (pl.forces_in_reserve(), pl.special_forces_in_reserve(), pl.resources)
    };
    let shipment = |forces: i32, specials: i32, g: &Game| {
        let mut s = Shipment::new(g, Faction::Emperor);
        s.shipment_type = ShipmentType::ShipmentBack;
        s.set_karma_card(None);
        s.set_no_field_value(-1);
        s.set_cunning_no_field_value(-1);
        s.set_to(g, Some(sand));
        s.force_amount = forces;
        s.special_force_amount = specials;
        GameEvent::Shipment(Rc::new(s))
    };

    // More than there are, or amounts with the wrong sign, are refused.
    assert_eq!(submit(&mut g, |g| shipment(-3, -3, g)).as_deref(), Some("Not enough Emperor to ship back to reserves"));
    assert_eq!(submit(&mut g, |g| shipment(-4, 0, g)).as_deref(), Some("Not enough Emperor to ship back to reserves"));
    assert_eq!(submit(&mut g, |g| shipment(2, -2, g)).as_deref(), Some("Can't ship forces from reserves when shipping back to reserves"));

    assert_eq!(submit(&mut g, |g| shipment(-3, -2, g)), None);
    let pl = g.player(emperor);
    assert_eq!(pl.any_forces_in(Some(sand)), 0, "nothing stays behind");
    assert_eq!(pl.forces_in_reserve(), reserve + 3);
    assert_eq!(pl.special_forces_in_reserve(), special_reserve + 2);
    assert_eq!(pl.resources, spice - 3, "half of 5 forces, rounded up");
}

/// A Bene Gesserit intrusion waits in one territory while the latest move went elsewhere (a
/// terror sneak attack, for example, doesn't update the latest move). The flip must happen where
/// the prompt and log say: in the intruded territory.
#[test]
fn bene_gesserit_flip_happens_in_the_intruded_territory() {
    let mut g = fixture("bene_gesserit");
    let bg = idx(&g, Faction::BeneGesserit);
    // Bene Gesserit advisors are in locations 5 and 8 (two strongholds).
    let (intruded, elsewhere) = (location(&g, 5), location(&g, 8));
    assert_eq!(g.player(bg).special_forces_in(Some(intruded)), 4);
    assert_eq!(g.player(bg).special_forces_in(Some(elsewhere)), 6);

    let mv = |to: Location, g: &Game| {
        let mut m = Move::new(g, Faction::Emperor);
        m.set_to(g, Some(to));
        GameEvent::Move(Rc::new(m))
    };
    g.intrusions.push(Intrusion::new(&mv(intruded, &g), IntrusionType::BeneGesseritIntrusion));
    g.last_shipment_or_movement = Some(mv(elsewhere, &g));
    g.current_phase = Phase::BeneGesseritIntrudedByNonGuildMove;

    let mut flip = BeneGesseritFlip::new(&g, Faction::BeneGesserit);
    flip.as_advisors = false;
    assert_eq!(submit(&mut g, |_| GameEvent::BeneGesseritFlip(Rc::new(flip))), None);
    let log = g.current_report().messages.iter().map(|m| m.to_cs_string()).collect::<Vec<_>>();
    assert!(log.iter().any(|m| m.contains("become")), "{log:?}");
    let pl = g.player(bg);
    assert_eq!((pl.forces_in(Some(intruded)), pl.special_forces_in(Some(intruded))), (4, 0), "flipped where intruded");
    assert_eq!((pl.forces_in(Some(elsewhere)), pl.special_forces_in(Some(elsewhere))), (0, 6), "left alone");
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
    assert_eq!(submit(&mut g, |_| GameEvent::AllyPermission(Rc::new(permission))), None);

    let revival = |own: i32, by_emperor: i32, g: &Game| {
        let mut r = Revival::new(g, Faction::Fremen);
        r.set_hero(None);
        r.amount_of_special_forces = own;
        r.extra_special_forces_paid_by_emperor = by_emperor;
        GameEvent::Revival(Rc::new(r))
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
    let monster = |t: Option<Territory>, g: &Game| {
        let mut m = KarmaMonster::new(g, Faction::Fremen);
        m.set_territory(g, t);
        GameEvent::KarmaMonster(Rc::new(m))
    };
    let karama_cards = |g: &Game| g.player(fremen).treachery_cards.iter().filter(|c| c.type_() == TreacheryCardType::Karma).count();
    assert_eq!(karama_cards(&g), 1);

    assert_eq!(submit(&mut g, |g| monster(None, g)).as_deref(), Some("No territory selected"));
    let stronghold = g.map.territories(false).find(|t| t.is_stronghold()).unwrap();
    assert_eq!(submit(&mut g, |g| monster(Some(stronghold), g)).as_deref(), Some("Selected territory is protected"));
    assert_eq!(karama_cards(&g), 1);
    assert!(!g.player(fremen).special_karma_power_used);

    let monsters = g.number_of_monsters;
    let target = KarmaMonster::valid_targets(&g).next().unwrap();
    assert_eq!(submit(&mut g, |g| monster(Some(target), g)), None);
    assert_eq!(karama_cards(&g), 0);
    assert!(g.player(fremen).special_karma_power_used);
    assert_eq!(g.number_of_monsters, monsters + 1);
}

/// Ally permissions are stored under the ally. Without an ally there is nothing to store (the
/// event isn't applicable then, but savegames replay events without validation).
#[test]
fn ally_permissions_without_an_ally_store_nothing() {
    let mut g = fixture("ship");
    let mut permission = AllyPermission::new(&g, Faction::Guild);
    permission.permitted_resources = 3;
    let e = GameEvent::AllyPermission(Rc::new(permission));
    assert!(e.execute(&mut g, false, false).is_none());
    assert!(g.permitted_use_of_ally_spice.is_empty(), "{:?}", g.permitted_use_of_ally_spice);
}

/// Rule::RicheseTreacheryCards puts Richese's cards in the treachery deck. With Richese playing
/// they are also its cache, so a card could be sold in a normal auction and then again by
/// Richese, and the next `TreacheryCalled` failed with "Card 50 is in more than one hand"
/// (observe bots seed 1 game 163). A card taken from one of the two now leaves the other.
#[test]
fn a_richese_card_taken_from_the_deck_or_the_cache_leaves_the_other() {
    let mut g = Game::new_int32_participation(Game::LATEST_VERSION, &Shared::new(Participation::default()));
    let mut s = GameSettings::new();
    s.number_of_players = 3;
    s.allowed_factions_in_play = Some(Shared::new(vec![Faction::Richese, Faction::Atreides, Faction::Emperor]));
    s.initial_rules = Some(vec![Rule::RicheseTreacheryCards]);
    let mut establish = EstablishPlayers::new(&g, Faction::None);
    establish.seed = 7;
    establish.settings = Some(Shared::new(s));
    assert!(GameEvent::EstablishPlayers(Rc::new(establish)).execute(&mut g, false, true).is_none());
    let end = GameEvent::EndPhase(Rc::new(EndPhase::new(&g, Faction::None)));
    assert!(end.execute(&mut g, false, true).is_none());

    let in_deck = |g: &Game, c: TreacheryCard| g.treachery_deck.as_ref().unwrap().items.contains(&c);
    let both: Vec<TreacheryCard> = g.richese_cache.iter().copied().filter(|c| in_deck(&g, *c)).collect();
    assert!(both.len() >= 2, "Richese cards in the deck and the cache: {both:?}");
    let (drawn, sold) = (both[0], both[1]);

    // Drawn from the deck (for an auction, or as a starting card): Richese no longer has it.
    let items = &mut g.treachery_deck.as_mut().unwrap().items;
    let i = items.iter().position(|c| *c == drawn).unwrap();
    items.remove(i);
    g.treachery_deck.as_mut().unwrap().put_on_top(drawn);
    assert_eq!(g.draw_treachery_card(), Some(drawn));
    assert!(!g.richese_cache.contains(&drawn));

    // Sold from the cache: it leaves the deck.
    g.cards_on_auction = Some(Deck::empty(None));
    g.current_phase = Phase::RicheseSpecifyingAuction;
    let mut auction = RicheseSpecifiesAuction::new(&g, Faction::Richese);
    auction.set_card(Some(sold));
    auction.auction_type = AuctionType::RicheseOnceAround;
    auction.direction = 1;
    assert!(GameEvent::RicheseSpecifiesAuction(Rc::new(auction)).execute(&mut g, false, false).is_none());
    assert!(!g.richese_cache.contains(&sold));
    assert!(!in_deck(&g, sold));
}

/// A Karama bid that also passes, or names no Karama card of the bidder's, passed validation and
/// then threw halfway through, after changing the game. The changes weren't in the history, so a
/// later undo replayed another game, one with a Bene Gesserit intrusion that was never answered
/// (observe fuzz seed 42 game 110). Both are refused now.
#[test]
fn a_karama_bid_needs_a_karama_card_and_no_pass() {
    let mut g = fixture("bid");
    let bidder = g.bid_sequence.as_ref().unwrap().current_player(&g).expect("a bidder");
    let faction = g.player(bidder).faction;
    let held: Vec<TreacheryCard> = g.players.iter().flat_map(|p| p.treachery_cards.clone()).collect();
    let karama = TreacheryCardManager::items().iter().copied().find(|c| c.type_() == TreacheryCardType::Karma && !held.contains(c)).unwrap();
    g.player_mut(bidder).treachery_cards.push(karama);
    let bid = |passed: bool, card: Option<TreacheryCard>, g: &Game| {
        let mut b = Bid::new(g, faction);
        b.passed = passed;
        b.karma_bid = true;
        b.set_karma_card(card);
        GameEvent::Bid(Rc::new(b))
    };
    let history = g.history.len();
    assert_eq!(submit(&mut g, |g| bid(true, Some(karama), g)).as_deref(), Some("You can't pass and use Karma in the same bid"));
    assert_eq!(submit(&mut g, |g| bid(false, None, g)).as_deref(), Some("Invalid Karma card"));
    assert_eq!(g.history.len(), history);
    assert!(g.player(bidder).treachery_cards.contains(&karama));

    assert_eq!(submit(&mut g, |g| bid(false, Some(karama), g)), None);
    assert!(!g.player(bidder).treachery_cards.contains(&karama), "the Karama card is spent");
}

/// An intrusion is answered right after the shipment or move that caused it. One left queued
/// when a new phase starts can't be answered any more; before, it came up after the next
/// shipment, in place of that shipment's own (observe fuzz seed 42 game 110).
#[test]
fn an_intrusion_left_over_from_an_earlier_phase_is_dropped() {
    let mut g = fixture("bene_gesserit");
    let mut m = Move::new(&g, Faction::Emperor);
    m.set_to(&g, Some(location(&g, 5)));
    g.intrusions.push(Intrusion::new(&GameEvent::Move(Rc::new(m)), IntrusionType::BeneGesseritIntrusion));
    g.main_phase_start(MainPhase::Battle, false);
    assert!(g.intrusions.is_empty());
}
