//! Rules bugs fixed on purpose (rust/docs/behavior-changes.md). Each test
//! replays a design fixture (scripts/fixtures), adjusts the state where the fixture lacks the
//! situation, and submits events with validation, as the server does.

use std::path::PathBuf;

use treachery_engine::game::Intrusion;
use treachery_engine::prelude::*;

fn fixture(name: &str) -> Game {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scripts/fixtures").join(format!("{name}.json"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let state = treachery_engine::game::GameState::load(&text).expect("savegame");
    let mut game = Game::of_version(state.version);
    for e in state.events.clone() {
        let event_name = e.event_type().name();
        assert!(game.play_for_message(e, false, false).is_none(), "{name}: {} failed", event_name);
    }
    game
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
    let sequence = PlayerSequence::new(&g, false, 1, Some(emperor), false);
    g.ship_move_mut().expect("the fixture is in the shipment and move phase").sequence = sequence;
    assert_eq!(g.current_shipment_and_move_player(), Some(emperor));
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
    g.alliances.shipment_permissions.insert(Faction::Emperor, ShipmentPermission::TO_HOMEWORLD);

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

    g.turn.intrusions.push(Intrusion { initiator: Faction::Emperor, to: Some(intruded), type_: IntrusionType::BeneGesseritIntrusion, alliance_offered: false });
    g.turn.last_shipment_or_movement = Some(ShipmentOrMovement { to: Some(elsewhere) });
    g.interrupt(Interrupt::intrusion(IntrusionType::BeneGesseritIntrusion, IntrusionCause::NonGuildMove));

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

    let mut permission = AllyPermission::default();
    permission.emperor_will_pay_for_extra_revival = 3;
    assert_eq!(submit_by(&mut g, Faction::Emperor, |_| Event::AllyPermission(permission)), None);

    let revival = |own: u16, by_emperor: u16, _g: &Game| {
        let mut r = Revival::default();
        r.hero = None;
        r.amount_of_special_forces = own;
        r.extra_special_forces_paid_by_emperor = by_emperor;
        Event::Revival(r)
    };
    assert_eq!(submit_by(&mut g, Faction::Fremen, |g| revival(0, 1, g)), None);
    assert_eq!(g.player(fremen).special_forces_killed, 1);
    assert_eq!(submit_by(&mut g, Faction::Fremen, |g| revival(1, 0, g)).as_deref(), Some("You already revived a Fremen this turn"));
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

    let monsters = g.board.number_of_monsters;
    let target = KarmaMonster::valid_targets(&g).next().unwrap();
    assert_eq!(submit_by(&mut g, Faction::Fremen, |g| monster(Some(target), g)), None);
    assert_eq!(karama_cards(&g), 0);
    assert!(g.player(fremen).special_karma_power_used);
    assert_eq!(g.board.number_of_monsters, monsters + 1);
}

/// Ally permissions are stored under the ally. Without an ally there is nothing to store (the
/// event isn't applicable then, but savegames replay events without validation).
#[test]
fn ally_permissions_without_an_ally_store_nothing() {
    let mut g = fixture("ship");
    let mut permission = AllyPermission::default();
    permission.permitted_resources = 3;
    let e = HistoryEntry::new(Faction::Guild, permission);
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
    let mut establish = EstablishPlayers::default();
    establish.seed = 7;
    establish.settings = Some(s);
    assert!(g.play_for_message(HistoryEntry::new(None, establish), false, true).is_none());
    let end = HistoryEntry::new(None, EndPhase::default());
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
    g.enter_bidding(BiddingStep::RicheseSpecifyingAuction);
    g.bidding_mut().unwrap().cards_on_auction = Some(Deck::empty());
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
/// (observe fuzz seed 42 game 110). A bid without a card is refused now, and a pass is a pass.
#[test]
fn a_karama_bid_needs_a_karama_card_and_no_pass() {
    let mut g = fixture("bid");
    let bidder = g.bidding().unwrap().bid_sequence.as_ref().unwrap().current_player(&g).expect("a bidder");
    let faction = g.player(bidder).faction.expect("bidder has a faction");
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
    assert_eq!(submit_by(&mut g, faction, |g| bid(false, None, g)).as_deref(), Some("Invalid Karma card"));
    assert_eq!(g.history.entries.len(), history);
    assert!(g.player(bidder).treachery_cards.contains(&karama));

    // Since version 191 a pass carries none of its other fields (`Event::normalized`), so a pass
    // that names a Karama card is a plain pass, and the card stays.
    let mut passing = fixture("bid");
    passing.player_mut(bidder).treachery_cards.push(karama);
    assert_eq!(submit_by(&mut passing, faction, |g| bid(true, Some(karama), g)), None);
    assert_eq!(passing.history.entries.len(), history + 1);
    assert!(passing.player(bidder).treachery_cards.contains(&karama), "a pass spends no card");

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
    g.turn.intrusions.push(Intrusion { initiator: Faction::Emperor, to: m.to, type_: IntrusionType::BeneGesseritIntrusion, alliance_offered: false });
    g.main_phase_start(MainPhase::Battle, false);
    assert!(g.turn.intrusions.is_empty());
}

/// An ally pays for a shipment only spice it has and has offered. Without the check, a refitted
/// game shipped with spice the ally didn't have and left it at -1 (the `alliance-allied`
/// fixture, after the change to ChaCha8). Bid and Battle check both too.
#[test]
fn an_ally_pays_for_a_shipment_only_spice_it_has_and_offered() {
    let mut g = fixture("ship");
    let shipper = g.current_shipment_and_move_player().expect("a shipper");
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

    let revival = |_g: &Game| {
        let mut r = Revival::default();
        r.hero = Some(Hero::Leader(leader));
        Event::Revival(r)
    };
    let karama_revival = |g: &Game| {
        let mut r = KarmaFreeRevival::default();
        r.hero = Some(Hero::Leader(leader));
        r.validate(g, By::new(g, f)).err().map(|r| r.0.plain_text())
    };
    let refusal = Some("You can't revive a leader who isn't dead");
    assert_eq!(submit_by(&mut g, f, revival).as_deref(), refusal);
    assert_eq!(karama_revival(&g).as_deref(), refusal);
    assert!(g.is_alive(Hero::Leader(leader)));

    g.kill_hero(Hero::Leader(leader));
    assert_eq!(karama_revival(&g), None);
    assert_eq!(submit_by(&mut g, f, revival), None);
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

    let mut r = Revival::default();
    r.hero = Some(Hero::Leader(leader));
    assert_eq!(submit_by(&mut g, f, |_| Event::Revival(r)), None);
    assert!(g.is_alive(Hero::Leader(leader)));
    assert!(g.player(p).leaders.contains(&leader));
    assert!(!g.player(tleilaxu).leaders.contains(&leader));
}

/// The battle of the `battle` fixture, started again as a new battle at `step`, as when the next
/// battle of the phase begins.
fn start_next_battle(g: &mut Game, step: BattleStep) {
    let battle = g.current_battle().cloned().expect("a battle");
    let aggressor = battle.aggressor(g).expect("aggressor");
    let defender = battle.defender(g).expect("defender");
    let (aggressor_plan, defender_plan) = (Battle::default().plan(By::new(g, aggressor)), Battle::default().plan(By::new(g, defender)));
    g.enter_battle_state(step, Some(battle));
    g.fight_mut().aggressor_plan = Some(aggressor_plan);
    g.fight_mut().defender_plan = Some(defender_plan);
    g.fight_mut().aggressor_traitor_action = Some(TraitorCall { submitter: By::new(g, aggressor), traitor_called: false });
    g.fight_mut().defender_traitor_action = Some(TraitorCall { submitter: By::new(g, defender), traitor_called: false });
}

/// A Decipherer leader lets the winner replace a traitor in its own battle only. Before the fix
/// the permission stayed, so a later winner without a Decipherer could replace a traitor.
#[test]
fn a_decipherer_lets_only_its_own_battle_replace_a_traitor() {
    let mut g = fixture("battle");
    let battle = g.current_battle().cloned().expect("a battle");
    let winner = g.get_player(battle.aggressor(&g)).expect("aggressor");
    let faction = g.player(winner).faction;
    let leader = g.player(winner).leaders[0];
    g.set_skill(leader, Some(LeaderSkill::Decipherer));
    let mut plan = Battle::default().plan(By::new(&g, faction.expect("faction")));
    plan.hero = Some(Hero::Leader(leader));
    g.activate_decipherer_if_applicable(&plan);
    assert!(g.fight().unwrap().decipherer_may_replace_traitor, "the Decipherer's own battle allows it");
    g.finish_decipherer_if_applicable();
    g.set_skill(leader, None);

    start_next_battle(&mut g, BattleStep::Conclusion);
    g.fight_mut().winner = faction;
    let traitors = g.player(winner).traitors.clone();
    let old = *traitors.first().expect("the winner has a traitor");
    let new = g.decks.traitor_deck.as_ref().expect("traitor deck").items.first().copied().expect("a traitor in the deck");
    let refusal = submit_by(&mut g, faction, |_| Event::from(BattleConcluded { traitor_to_replace: Some(old), new_traitor: Some(new), ..Default::default() }));
    assert_eq!(refusal, None);
    assert_eq!(g.player(winner).traitors, traitors, "no traitor replaced");
}

/// A Moritani secret ally that let a loser keep a card plays the Nexus card in its own battle only.
/// Before the fix an unused permission stayed, so a later loser keeping a card played the Nexus card.
#[test]
fn a_moritani_secret_ally_lets_only_its_own_battle_keep_a_card() {
    let mut g = fixture("battle");
    assert!(!g.is_playing(Some(Faction::Moritani)));
    let battle = g.current_battle().cloned().expect("a battle");
    let loser = g.get_player(battle.defender(&g)).expect("defender");
    let faction = g.player(loser).faction;
    g.player_mut(loser).nexus = Some(Faction::Moritani);
    let plan = Battle::default().plan(By::new(&g, faction.expect("faction")));
    g.lose_cards(&plan, false).unwrap();
    assert!(g.fight().unwrap().secret_ally_allows_keeping_cards_after_losing_battle, "the battle it was lost in allows it");

    start_next_battle(&mut g, BattleStep::Conclusion);
    let kept = TreacheryCardManager::items().iter().copied().find(|c| c.type_() == TreacheryCardType::Shield).expect("a shield");
    g.player_mut(loser).treachery_cards.push(kept);
    g.fight_mut().loser = faction;
    g.fight_mut().cards_to_be_discarded_by_loser = vec![kept];
    let refusal = submit_by(&mut g, faction, |_| Event::from(LoserConcluded { kept_card: Some(kept), ..Default::default() }));
    assert_eq!(refusal, None);
    assert_eq!(g.player(loser).nexus, Some(Faction::Moritani), "the Nexus card wasn't played");
    assert!(g.player(loser).has(Some(kept)));
    let logged = g.log.report_list.iter().flat_map(|r| r.messages.iter()).any(|m| m.plain_text().contains(" keep "));
    assert!(logged, "the kept card is logged");
}

/// The host ends the current step, as the server's host does.
fn end_phase(g: &mut Game) {
    assert!(g.play_for_message(HistoryEntry::new(None, EndPhase::default()), true, true).is_none(), "EndPhase at {:?}", g.phase());
}

/// The `bid` fixture in its bidding report, with `set` applied to the bidding state, then moved
/// on to the revival phase.
fn bidding_then_revival(set: impl FnOnce(&mut BiddingState)) -> Game {
    let mut g = fixture("bid");
    set(g.bidding_mut().expect("in bidding"));
    g.enter_bidding(BiddingStep::Report);
    end_phase(&mut g);
    assert!(g.bidding().is_none() && g.revival().is_some());
    g
}

/// After a Richese auction, everyone had bidding prescience until the next bidding phase began,
/// so Atreides could offer to share it and bots valued it in the charity phase.
#[test]
fn bidding_prescience_of_a_richese_auction_ends_with_the_bidding_phase() {
    let g = fixture("bid");
    let p = g.player_indices().find(|p| !g.has_bidding_prescience(*p)).expect("a player without bidding prescience");
    let mut g = bidding_then_revival(|b| b.current_auction_type = Some(AuctionType::RicheseSilent));
    assert_eq!(g.current_auction_type(), None);
    assert!(!g.has_bidding_prescience(p));
    g.enter_bidding_phase();
    assert_eq!(g.current_auction_type(), None);
    assert!(!g.has_bidding_prescience(p));
}

/// When everyone passed on a card, the passes and the "Atreides or their ally passed" flag
/// stayed into the next bidding phase: the bidding view showed the old passes, Juice to go first
/// wasn't offered, and the automation rule "pass when Atreides or their ally passed" could pass
/// at once.
#[test]
fn the_passes_of_an_auction_everyone_passed_end_with_the_bidding_phase() {
    let g = fixture("bid");
    let (p, f) = g.player_indices().find_map(|p| g.player(p).faction.map(|f| (p, f))).expect("a faction");
    let pass = PlacedBid {
        number: 1,
        type_: EventType::Bid,
        initiator: Some(f),
        player: Some(p),
        amount: 0,
        ally_contribution_amount: 0,
        emperor_contribution_amount: 0,
        uses_emperor_secret_ally: false,
        karma_card: None,
        karma_bid: false,
        passed: true,
    };
    let mut g = bidding_then_revival(|b| {
        b.current_bid = None;
        b.bids.insert(f, pass);
        b.latest_bid_by_atreides_or_atreides_ally_was_passed = true;
    });
    assert!(g.bids().is_empty());
    g.enter_bidding_phase();
    let b = g.bidding().expect("in bidding");
    assert!(b.bids.is_empty());
    assert!(!b.latest_bid_by_atreides_or_atreides_ally_was_passed);
    assert!(JuicePlayed::valid_types(&g, p).contains(&JuiceType::GoFirst));
}

/// Richese's Nexus betrayal is played right after Richese sell a card. Whether their auction
/// just finished stayed set into the next bidding phase, so the betrayal could be played there
/// before any card was sold.
#[test]
fn a_finished_richese_auction_ends_with_the_bidding_phase() {
    let mut g = bidding_then_revival(|b| b.richese_bidding_just_finished = true);
    g.enter_bidding_phase();
    assert!(!g.bidding().expect("in bidding").richese_bidding_just_finished);
}

/// A winner offered to replace a card with the Ixian Nexus card who declined left the flag set,
/// and a later replacement through the Ixian alliance then used up the Nexus card too.
#[test]
fn a_declined_nexus_replacement_ends_with_the_bidding_phase() {
    let mut g = bidding_then_revival(|b| b.replacing_bought_card_using_nexus = true);
    g.enter_bidding_phase();
    assert!(!g.bidding().expect("in bidding").replacing_bought_card_using_nexus);
}

/// A request to Tleilaxu for an early leader revival that Tleilaxu didn't answer stayed until
/// the next revival phase, where Tleilaxu could still answer it, and a bot that made it wouldn't
/// revive.
#[test]
fn an_unanswered_request_to_tleilaxu_ends_with_the_revival_phase() {
    let (mut g, p) = revival_fixture("revival-planet");
    let f = g.player(p).faction;
    let tleilaxu = idx(&g, Faction::Tleilaxu);
    let leader = g.player(p).leaders[0];
    g.kill_hero(Hero::Leader(leader));
    let request = RevivalRequest { initiator: f, player: Some(p), hero: Some(Hero::Leader(leader)) };
    g.revival_mut().expect("in revival").current_revival_requests.push(request);
    assert!(g.get_applicable_events(Some(tleilaxu), false).contains(&EventType::AcceptOrCancelTleilaxuRevival));
    end_phase(&mut g);
    assert_eq!(g.phase(), Phase::ResurrectionReport);
    end_phase(&mut g);
    assert!(g.revival().is_none());
    g.enter_revival(RevivalStep::Reviving);
    assert!(g.revival().expect("in revival").current_revival_requests.is_empty());
    assert!(!g.get_applicable_events(Some(tleilaxu), false).contains(&EventType::AcceptOrCancelTleilaxuRevival));
}

/// Tleilaxu's Karama prevention of revival played in the revival report, after the revivals,
/// applied to the next turn's revivals.
#[test]
fn a_karama_revival_prevention_ends_with_the_revival_phase() {
    let (mut g, p) = revival_fixture("revival");
    let f = g.player(p).faction;
    end_phase(&mut g);
    assert_eq!(g.phase(), Phase::ResurrectionReport);
    g.revival_mut().expect("in revival").current_karma_revival_prevention = Some(RevivalPrevention { target: f });
    assert!(g.prevented_from_reviving(f));
    end_phase(&mut g);
    assert!(!g.prevented_from_reviving(f));
    g.enter_revival(RevivalStep::Reviving);
    assert!(!g.prevented_from_reviving(f));
    assert!(g.get_applicable_events(Some(p), false).contains(&EventType::Revival));
}

/// Tleilaxu's Karama prevention of revival was offered in the revival report too, after every
/// revival, where it did nothing but use up the card and the once-per-game Karama power.
#[test]
fn a_karama_revival_prevention_is_offered_only_while_factions_revive() {
    let (mut g, p) = revival_fixture("revival");
    let tleilaxu = g.player_indices().find(|t| *t != p).expect("another player");
    g.player_mut(tleilaxu).faction = Some(Faction::Tleilaxu);
    g.rules.active.push(Rule::AdvancedKarama);
    let held: Vec<TreacheryCard> = g.player_indices().flat_map(|q| g.player(q).treachery_cards.clone()).collect();
    let karama = TreacheryCardManager::items().iter().copied().find(|c| c.type_() == TreacheryCardType::Karma && !held.contains(c)).unwrap();
    g.player_mut(tleilaxu).treachery_cards.push(karama);
    assert_eq!(g.phase(), Phase::Resurrection);
    assert!(g.get_applicable_events(Some(tleilaxu), false).contains(&EventType::KarmaRevivalPrevention));
    end_phase(&mut g);
    assert_eq!(g.phase(), Phase::ResurrectionReport);
    assert!(!g.get_applicable_events(Some(tleilaxu), false).contains(&EventType::KarmaRevivalPrevention));
}
