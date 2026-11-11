//! Game identity around `Undo`, `RestoreRecentlyUndone` and `DismissRecentlyUndone`,
//! compared with C#. See engine-conventions.md, "Game identity and undo", and
//! docs/reviews/pkg-a.md items 1 and 2.
//!
//! Expected values (including full canonical state dumps) come from the C# reference
//! program, which runs the same scenarios:
//!
//!     cd rust/engine/csharp-ref && dotnet run -c Release -- undo-identity ../tests/golden/undo_identity.json

use std::rc::Rc;

use serde_json::Value;
use treachery_engine::canon;
use treachery_engine::cs::{self, Shared};
use treachery_engine::enumerations::{ClairvoyanceQuestion, Faction, Phase, TreacheryCardType};
use treachery_engine::game_status::GameStatus;
use treachery_engine::expression::ExpressionElement;
use treachery_engine::i_hero::Hero;
use treachery_engine::leader_manager::LeaderManager;
use treachery_engine::player::Player;
use treachery_engine::events::{Battle, BattleInitiated, Bid, ClairVoyancePlayed, DealOffered, Move, FlightUsed, GameEvent, GameEventMethods, NexusPlayed};
use treachery_engine::game::Game;
use treachery_engine::json;
use treachery_engine::participation::Participation;
use treachery_engine::treachery_card::TreacheryCard;
use treachery_engine::treachery_card_manager::TreacheryCardManager;

// The two events of ReviewA.cs (steps 1 and 2 of traces/sample/fuzz-2026-00018).
const ESTABLISH: &str = r#"{"$type":"EstablishPlayers","Settings":{"NumberOfPlayers":3,"MaximumTurns":8,"InitialRules":[301,300,65,60,20,1013,15,700,105,80,85,604,90,998,30,701,40,107,10,70,50,25,209,999,112],"AllowedFactionsInPlay":[50,10,120,60,40,100,20,90,70,80,30,110],"AutoOpenEmptySeats":true,"AsyncPlay":false,"AsyncPlayMessageIntervalMinutes":0},"Seed":1073548723,"MaximumNumberOfPlayers":3,"MaximumTurns":8,"ApplicableRules":[301,300,65,60,20,1013,15,700,105,80,85,604,90,998,30,701,40,107,10,70,50,25,209,999,112],"_players":"","_factionsInPlay":"Guild,Fremen,Moritani,BeneGesserit,Emperor,Richese,Atreides,Choam,Ixian,Tleilaxu,Harkonnen,Ecaz","Initiator":0,"Time":"2024-01-01T12:02:17+00:00"}"#;
const END_PHASE: &str = r#"{"$type":"EndPhase","Initiator":0,"Time":"2024-01-01T12:02:32+00:00"}"#;

// Steps 0-6 of traces/sample/fuzz-2026-00084 (same text as UndoIdentity.cs). Step 0 throws
// after adding players, so a replay has fewer players than the original game.
const TRACE84: &[(&str, bool)] = &[
    (r#"{"$type":"EstablishPlayers","Settings":{"NumberOfPlayers":5,"MaximumTurns":3,"InitialRules":[600,107,603,400,65,1015,998,109,401,120,601,602,1010],"AllowedFactionsInPlay":[80,90,40,50,20,10,70,110,100,30],"AutoOpenEmptySeats":true,"AsyncPlay":false,"AsyncPlayMessageIntervalMinutes":0},"Seed":1922928721,"MaximumNumberOfPlayers":5,"MaximumTurns":3,"ApplicableRules":[600,107,603,400,65,1015,998,109,401,120,601,602,1010],"_players":"","_factionsInPlay":"None","Initiator":0,"Time":"2024-01-01T12:01:49+00:00"}"#, true),
    (r#"{"$type":"EstablishPlayers","Settings":{"NumberOfPlayers":5,"MaximumTurns":3,"InitialRules":[600,107,603,400,65,1015,998,109,401,120,601,602,1010],"AllowedFactionsInPlay":[80,90,40,50,20,10,70,110,100,30],"AutoOpenEmptySeats":true,"AsyncPlay":false,"AsyncPlayMessageIntervalMinutes":0},"Seed":1922928721,"MaximumNumberOfPlayers":5,"MaximumTurns":3,"ApplicableRules":[600,107,603,400,65,1015,998,109,401,120,601,602,1010],"_players":"","_factionsInPlay":"Tleilaxu,Choam,Emperor,Guild,Atreides,Fremen,Ixian,Ecaz,Richese,Harkonnen","Initiator":0,"Time":"2024-01-01T12:02:36+00:00"}"#, true),
    (r#"{"$type":"EndPhase","Initiator":0,"Time":"2024-01-01T12:04:30+00:00"}"#, true),
    (r#"{"$type":"TraitorsSelected","_traitorId":1045,"Initiator":20,"Time":"2024-01-01T12:05:41+00:00"}"#, false),
    (r#"{"$type":"TraitorsSelected","_traitorId":1049,"Initiator":0,"Time":"2024-01-01T12:06:10+00:00"}"#, false),
    (r#"{"$type":"TraitorsSelected","_traitorId":1026,"Initiator":100,"Time":"2024-01-01T12:06:29+00:00"}"#, false),
    (r#"{"$type":"TraitorsSelected","_traitorId":1049,"Initiator":90,"Time":"2024-01-01T12:07:08+00:00"}"#, false),
];

fn golden() -> serde_json::Map<String, Value> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/golden/undo_identity.json");
    let text = std::fs::read_to_string(path).expect("run the csharp-ref undo-identity mode");
    serde_json::from_str::<Value>(&text).unwrap().as_object().unwrap().clone()
}

fn expected(name: &str) -> String {
    golden().get(name).unwrap_or_else(|| panic!("{name} missing from undo_identity.json")).as_str().unwrap().to_string()
}

fn check(name: &str, actual: impl ToString) {
    assert_eq!(actual.to_string(), expected(name), "{name}");
}

/// Compares a canonical dump with C#'s, reporting the first difference with context.
fn check_dump(name: &str, g: &Game) {
    let actual = String::from_utf8(canon::dump_game(g)).unwrap();
    let want = expected(name);
    if actual != want {
        let at = actual.bytes().zip(want.bytes()).position(|(a, b)| a != b).unwrap_or(actual.len().min(want.len()));
        let from = at.saturating_sub(300);
        panic!(
            "{name}: dump differs at byte {at}\n  C#:   ...{}\n  Rust: ...{}",
            &want[from..(at + 200).min(want.len())],
            &actual[from..(at + 200).min(actual.len())]
        );
    }
}

fn opt_id(c: Option<TreacheryCard>) -> String {
    c.map(|c| c.id().to_string()).unwrap_or_else(|| "null".into())
}

/// As the oracle runs a step: initialize, then execute with validation; an exception
/// escaping `EstablishPlayers.Execute` keeps the partial state.
fn run(g: &mut Game, events: &[(&str, bool)]) {
    for (text, is_host) in events {
        let e: GameEvent = json::deserialize(text).expect("event json");
        e.initialize_game(g);
        let _ = cs::catch(|| e.execute(g, true, *is_host));
    }
}

fn review_participation() -> Participation {
    let mut p = Participation::new();
    p.player_names.insert(100, "Human100".to_string());
    p.seated_players.insert(100, 2);
    p
}

fn base() -> Game {
    let mut g = Game::new_int32_participation(187, &Shared::new(review_participation()));
    run(&mut g, &[(ESTABLISH, true), (END_PHASE, true)]);
    g
}

fn trace84() -> Game {
    let mut g = Game::new_int32_participation(115, &Shared::new(Participation::new()));
    run(&mut g, TRACE84);
    g
}

/// `game = game.Undo(n)` and friends, as values.
fn undone(mut g: Game, n: i32) -> Game {
    g.undo(n);
    g
}
fn restored(mut g: Game) -> Game {
    g.restore_recently_undone();
    g
}
fn dismissed(mut g: Game) -> Game {
    g.dismiss_recently_undone();
    g
}

fn first_card(t: TreacheryCardType) -> TreacheryCard {
    *TreacheryCardManager::items().iter().find(|c| c.type_() == t).unwrap()
}

/// Undone events keep the old game and its players: the dump writes `OtherGame` and
/// the old game's player in full.
#[test]
fn undo_keeps_undone_events_in_the_old_game() {
    let u = undone(base(), 1);
    check_dump("undo.dump", &u);
}

/// `RestoreRecentlyUndone` returns the old game object, whose history events now
/// belong to the undo game.
#[test]
fn restore_returns_the_old_game_with_events_of_the_undo_game() {
    let g = base();
    let g_id = g.id;
    let r = restored(undone(g, 1));
    check("restore.returns_old", r.id == g_id);
    check_dump("restore.dump", &r);
}

#[test]
fn dismiss_replays_into_a_new_game() {
    check_dump("dismiss.dump", &dismissed(undone(base(), 1)));
}

/// The branch of `RestoreRecentlyUndone` that replays history and the undone events.
#[test]
fn restore_without_previous_game_replays() {
    let mut u = undone(base(), 1);
    u.previous_game_state = None;
    check_dump("restore_replay.dump", &restored(u));
}

/// Each restore returns the game the matching undo was made from.
#[test]
fn two_undos_restore_in_order() {
    let g = base();
    let g_id = g.id;
    let u1 = undone(g, 2);
    let u1_id = u1.id;
    let u2 = undone(u1, 1);
    check_dump("undo2.dump", &u2);
    let r1 = restored(u2);
    check("undo2.restore1.is_u1", r1.id == u1_id);
    check_dump("undo2.restore1.dump", &r1);
    let r2 = restored(r1);
    check("undo2.restore2.is_g", r2.id == g_id);
    check_dump("undo2.restore2.dump", &r2);
}

/// `GameEvent.Initialize` only sets `Player` when the initiator is seated. A replayed
/// event whose initiator isn't seated in the new game keeps the old game's player,
/// and executing it changes that player. Before game identity, Rust read the new
/// game's player at the same index (fuzz-2026-00084: 4 extra random draws).
#[test]
fn replayed_event_keeps_the_old_games_player() {
    let g = trace84();
    check_dump("retained.original.dump", &g);
    let g_id = g.id;
    let u = undone(g, 4);
    check_dump("retained.undo.dump", &u);
    let p = u.history[3].player_opt().expect("player set");
    check("retained.undo.history3_player_is_old", p.game == g_id);
    check("retained.undo.history3_player_is_new", p.game == u.id);
    check_dump("retained.restore.dump", &restored(u));
    check_dump("retained.dismiss.dump", &dismissed(undone(trace84(), 4)));
}

/// The sites of pkg-a.md item 1: after a restore, an event the game holds can belong
/// to the undo game. Comparisons with its `Player` are false, and writes to its
/// `Player` change the undo game's player.
#[test]
fn stored_events_of_another_game_after_restore() {
    let g = base();
    let u = undone(g, 1);
    let u_id = u.id;
    let mut r = restored(u);
    let f = r.players.iter().find(|x| x.faction != Faction::Fremen && x.ally == Faction::None).unwrap().faction;
    let p = r.get_player(Some(f)).unwrap();

    // Game.cs:977 HasOrnithopters: CurrentFlightUsed.Player == p
    let mut flight = FlightUsed::new(r.other_game(u_id), f);
    flight.move_three_territories = true;
    r.current_flight_used = Some(Rc::new(flight));
    check("site.ornithopters.foreign", r.has_ornithopters(p));
    let mut flight = FlightUsed::new(&r, f);
    flight.move_three_territories = true;
    r.current_flight_used = Some(Rc::new(flight));
    check("site.ornithopters.own", r.has_ornithopters(p));
    r.current_flight_used = None;

    // Game_Blow.cs:508 ProtectedFromMonster, Game_Revival.cs:80 FreeRevivals
    r.current_fremen_secret_ally = Some(Rc::new(NexusPlayed::new(r.other_game(u_id), f)));
    check("site.protected_from_monster.foreign", r.protected_from_monster(p));
    check("site.free_revivals.foreign", r.free_revivals(p, false));
    r.current_fremen_secret_ally = Some(Rc::new(NexusPlayed::new(&r, f)));
    check("site.protected_from_monster.own", r.protected_from_monster(p));
    check("site.free_revivals.own", r.free_revivals(p, false));
    r.current_fremen_secret_ally = None;

    // Game_Bidding.cs:636 GetCardSetAsideForBid, Game_Bidding.cs:178 ReturnKarmaCardUsedForBid
    let karma = first_card(TreacheryCardType::Karma);
    let mut bid = Bid::new(&r, f);
    bid.set_karma_card(Some(karma));
    r.current_bid = Some(Rc::new(bid).into_event());
    check("site.card_set_aside.own", opt_id(r.get_card_set_aside_for_bid(p)));
    let mut bid = Bid::new(r.other_game(u_id), f);
    bid.set_karma_card(Some(karma));
    r.current_bid = Some(Rc::new(bid).into_event());
    check("site.card_set_aside.foreign", opt_id(r.get_card_set_aside_for_bid(p)));
    r.return_karma_card_used_for_bid();
    check_dump("site.return_karma.dump", &r);
    check("site.return_karma.own_has_card", r.player(p).treachery_cards.contains(&karma));
    r.current_bid = None;

    // Members of a stored BattleInitiated read its own Game (the undo game)
    let other = r.players.iter().find(|x| x.faction != f).unwrap().faction;
    r.aggressor_plan = Some(Rc::new(Battle::new(&r, f)));
    let mut battle = BattleInitiated::new(r.other_game(u_id), f);
    battle.target = other;
    let arrakeen = r.map.arrakeen().territory();
    battle.set_territory(r.other_game(u_id), Some(arrakeen));
    r.current_battle = Some(Rc::new(battle));
    let battle = r.current_battle.clone().unwrap();
    check("site.battle.territory_is_this_maps", battle.territory(&r) == Some(arrakeen));
    check("site.battle.aggressive_player_is_undo_games", battle.aggressive_player(&r).unwrap().game == u_id);
    check("site.battle.plan_of_is_null", battle.plan_of(&r, Some(p)).is_none());
    check_dump("site.battle.dump", &r);
    r.current_battle = None;
    r.aggressor_plan = None;

    // Game_Battle.cs:220-228 HandleReinforcements writes to plan.Player
    let plan = Battle::new(r.other_game(u_id), f);
    plan.set_weapon(Some(first_card(TreacheryCardType::Reinforcements)));
    let plan = Rc::new(plan);
    r.aggressor_plan = Some(plan.clone());
    r.handle_reinforcements(&plan);
    check_dump("site.reinforcements.dump", &r);
}

/// Map objects of another game's map that reach this game keep their map: a territory
/// of the undo game's map is not this game's, and dumps with `"foreign":true`.
#[test]
fn map_objects_handed_back_by_another_game() {
    let g = base();
    let mut u = undone(g, 1);
    let u_id = u.id;
    let up = u.player_at(0);
    let arrakeen = u.map.arrakeen();
    Player::add_forces(&mut u, up, arrakeen, 3, false);
    let mut r = restored(u);
    let l = *r.player(up).forces_in_locations.keys().next().unwrap();
    let own = treachery_engine::i_fetcher::IFetcher::find(&r.map.territory_lookup(), l.territory().id());
    check("handed_back.territory_is_this_maps", Some(l.territory()) == own);
    assert_eq!(l.map_game(), Some(u_id));
    let leader = Hero::Leader(LeaderManager::leaders()[0]);
    r.leader_state.index_mut(&leader).current_territory = Some(l.territory());
    check_dump("handed_back.dump", &r);
}

// Step 0 of fuzz seed 100 game 64 (same text as UndoIdentity.cs): eight players including BeneGesserit.
const BENE_GESSERIT_ESTABLISH: &str = r#"{"$type":"EstablishPlayers","Settings":{"NumberOfPlayers":8,"MaximumTurns":5,"InitialRules":[209,1013,600,110,998,300,10000,10001,212,70],"AllowedFactionsInPlay":[60,100,50,120,80,40,110,90,20,10],"AutoOpenEmptySeats":true,"AsyncPlay":false,"AsyncPlayMessageIntervalMinutes":0},"Seed":1450467322,"MaximumNumberOfPlayers":8,"MaximumTurns":5,"ApplicableRules":[209,1013,600,110,998,300,10000,10001,212,70],"_players":"","_factionsInPlay":"BeneGesserit,Richese,Guild,Moritani,Tleilaxu,Emperor,Ecaz,Choam,Atreides,Fremen","Initiator":0,"Time":"2024-01-01T12:00:55+00:00"}"#;

/// `BeneGesseritFlip` after an undo flips BeneGesserit in a territory of the undo game's map. Where the
/// flip empties a battalion, C# removes it and creates a new one keyed by that map's
/// location, so the dump writes `"foreign":true`; where it doesn't, the old key stays.
/// Rust iterated `Territory::locations()`, which drops the map (fuzz seed 100 game 64).
/// The other `Player` methods that take a territory and change forces run on it too.
#[test]
fn bene_gesserit_flip_in_a_territory_of_another_games_map() {
    let mut g = Game::new_int32_participation(187, &Shared::new(Participation::new()));
    run(&mut g, &[(BENE_GESSERIT_ESTABLISH, true), (END_PHASE, true)]);
    let u = undone(g, 1);
    let u_id = u.id;
    let mut r = restored(u);
    let bene_gesserit = r.get_player(Some(Faction::BeneGesserit)).expect("BeneGesserit is seated");
    let (arrakeen, carthag) = (r.map.arrakeen(), r.map.carthag());
    Player::add_forces(&mut r, bene_gesserit, arrakeen, 2, false);
    Player::flip_forces_territory_boolean(&mut r, bene_gesserit, arrakeen.territory().of_game_map(u_id), true);
    Player::add_forces(&mut r, bene_gesserit, carthag, 1, false);
    Player::add_special_forces(&mut r, bene_gesserit, carthag, 1, false);
    Player::flip_forces_territory_boolean(&mut r, bene_gesserit, carthag.territory().of_game_map(u_id), true);
    // The other Player methods that take a Territory and change forces.
    let foreign = |l: treachery_engine::location::Location| l.territory().of_game_map(u_id);
    Player::kill_forces_territory_int32_boolean_boolean(&mut r, bene_gesserit, foreign(carthag), 1, true, false);
    let (tueks, tabr, habbanya) = (r.map.tueks_sietch(), r.map.sietch_tabr(), r.map.habbanya_sietch());
    Player::add_forces(&mut r, bene_gesserit, tueks, 3, false);
    Player::forces_to_reserves_territory_int32_boolean(&mut r, bene_gesserit, foreign(tueks), 1, false);
    Player::kill_all_forces_territory_boolean(&mut r, bene_gesserit, foreign(tueks), true);
    Player::add_forces(&mut r, bene_gesserit, tabr, 2, false);
    Player::forces_to_reserves_territory(&mut r, bene_gesserit, foreign(tabr));
    Player::add_forces(&mut r, bene_gesserit, habbanya, 2, false);
    Player::kill_forces_territory_int32_boolean_boolean(&mut r, bene_gesserit, foreign(habbanya), 2, false, true);
    let keys: Vec<_> = r.player(bene_gesserit).forces_in_locations.keys().copied().collect();
    check("bene_gesserit_flip.arrakeen_key_is_this_maps", keys.iter().any(|l| l.is_same_object(arrakeen)));
    check("bene_gesserit_flip.carthag_key_is_this_maps", keys.iter().any(|l| l.is_same_object(carthag)));
    check_dump("bene_gesserit_flip.dump", &r);
}

/// A stored event of the undo game that kept a player of this game: reading the
/// event's members goes through the undo game and back to this game's player.
#[test]
fn event_of_another_game_with_a_player_of_this_game() {
    let g = trace84();
    let u = undone(g, 4);
    let u_id = u.id;
    let r = restored(u);
    let mut b = BattleInitiated::new(&r, Faction::Richese);
    b.target = Faction::Atreides;
    b.initialize_game(r.other_game(u_id));
    check("third.player_is_restored_games", b.player().game == r.id);
    check("third.is_involved_atreides", b.is_involved_faction(&r, Faction::Atreides));
    check("third.is_involved_richese", b.is_involved_faction(&r, Faction::Richese));
    check("third.opponent_of_atreides_is_null", b.opponent_of_faction(&r, Faction::Atreides).is_none());
}

/// Members of stored events that read their own `Game`: `ClairVoyancePlayed.Parameter1`,
/// `DealOffered.Acceptance`/`Rejection` and `Battle.ActivateDynamicWeapons`.
#[test]
fn stored_event_members_read_their_own_game() {
    let g = base();
    let u = undone(g, 1);
    let u_id = u.id;
    let mut r = restored(u);
    let f = r.players.iter().find(|x| x.faction != Faction::Fremen && x.ally == Faction::None).unwrap().faction;
    let other = r.players.iter().find(|x| x.faction != f).unwrap().faction;

    let arrakeen = r.map.arrakeen().territory();
    let mut cv = ClairVoyancePlayed::new(r.other_game(u_id), f);
    cv.question = ClairvoyanceQuestion::WillAttackX;
    cv.question_parameter1 = Some(arrakeen.id().to_string());
    check("event_game.parameter1_is_this_maps", matches!(cv.parameter1(&r), ExpressionElement::Territory(t) if t == arrakeen));

    let offer = DealOffered::new(r.other_game(u_id), f);
    check("event_game.acceptance_game_is_undo", offer.acceptance(&r, other).unwrap().game_ref().0 == u_id);
    check("event_game.rejection_game_is_undo", offer.rejection(&r, other).unwrap().game_ref().0 == u_id);
    check("event_game.acceptance_player_is_undo_games", offer.acceptance(&r, other).unwrap().player().game == u_id);

    let plan = Battle::new(r.other_game(u_id), f);
    plan.set_weapon(Some(first_card(TreacheryCardType::MirrorWeapon)));
    let plan = Rc::new(plan);
    r.aggressor_plan = Some(plan.clone());
    plan.activate_dynamic_weapons(&mut r, Hero::Leader(LeaderManager::leaders()[0]), Some(first_card(TreacheryCardType::Projectile)), None);
    check_dump("event_game.dynamic_weapons.dump", &r);
    check("event_game.dynamic_weapons.undo_report_messages", r.other_game(u_id).current_report().messages.len());
}

/// `GameStatus` reads `LastShipmentOrMovement.To` through the event's own game: the
/// highlighted territory and the BeneGesserit intrusion messages name the undo game's territory
/// (pkg-k.md item 3).
#[test]
fn status_reads_the_last_movement_through_its_own_game() {
    let g = base();
    let u = undone(g, 1);
    let u_id = u.id;
    let mut r = restored(u);
    let f = r.players.iter().find(|x| x.faction != Faction::Fremen && x.ally == Faction::None).unwrap().faction;
    let mut m = Move::new(r.other_game(u_id), f);
    m.to_id = r.map.arrakeen().id();
    r.last_shipment_or_movement = Some(Rc::new(m).into_event());
    for phase in [Phase::BeneGesseritIntrudedByFremenRidingMonsterA, Phase::BeneGesseritIntrudedByNonGuildMove] {
        r.current_phase = phase;
        let me = r.player_at(0);
        let status = GameStatus::determine_status(&mut r, Some(me), true);
        let text = String::from_utf8(canon::dump_detached(&status as &dyn canon::Canon, Some(&r))).unwrap();
        check(&format!("status.{phase}"), text);
    }
}

/// pkg-a.md item 4: an exception out of `Undo` leaves the caller with the old game.
#[test]
fn undo_that_throws_keeps_the_old_game() {
    let mut g = base();
    let e = cs::catch(|| g.undo(-1)).expect_err("C# throws");
    check("undo_throws.exception", format!("{}: {}", e.type_name, e.message));
    check_dump("undo_throws.dump", &g);
}

/// pkg-a.md item 2: the game keeps the caller's `Participation`, and every game made
/// by undo, restore and dismiss shares it.
#[test]
fn participation_is_shared() {
    let part = Shared::new(Participation::new());
    let g = Game::new_int32_participation(187, &part);
    part.borrow_mut().hosts.add(7);
    check("participation.caller_alias.is_host7", g.is_host(Some(7)));
    let mut u = undone(g, 0);
    u.add_player(5, "Late", 1);
    let r = restored(u);
    check("participation.restore.is_participant5", r.is_participant(5));
    check_dump("participation.restore.dump", &r);
    let mut d = dismissed(r);
    d.set_or_unset_host(9);
    check("participation.dismiss.caller_sees_host9", part.borrow().hosts.contains(&9));
}

/// C#'s garbage collector frees a game nothing references any more. Rust drops such
/// games after each undo, restore and dismiss, so a long game with many undos keeps a
/// bounded number of them (Game::drop_unreachable_games).
#[test]
fn repeated_undos_keep_a_bounded_number_of_games() {
    let mut g = base();
    for _ in 0..200 {
        g.undo(1);
        g.restore_recently_undone();
        g.undo(1);
        g.dismiss_recently_undone();
    }
    assert!(g.other_games.len() <= 4, "{} games kept", g.other_games.len());
    // The game still behaves: its dump matches a game that did one undo and dismiss.
    check_dump("dismiss.dump", &g);
}
