//! The server decides who may do what in a game, from the user the connection is logged in as:
//! a user who isn't in the game, an observer and a player of another faction are refused
//! every call that changes the game or acts in it, and an event is always the caller's own,
//! whatever its `Initiator` says.

mod common;

use common::*;
use serde_json::{json, Value};

mod error {
    pub const NO_HOST: i64 = 22;
    pub const INVALID_GAME_EVENT: i64 = 23;
    pub const SEAT_NOT_AVAILABLE_NOT_OPEN: i64 = 27;
    pub const NO_CREATOR: i64 = 31;
    pub const USER_NOT_IN_GAME: i64 = 33;
}

/// The alliance fixture: Alice hosts as the Emperor, Bobby plays the Atreides, Carol observes,
/// and Dave has nothing to do with the game.
struct Table {
    alice: User,
    bobby: User,
    carol: User,
    dave: User,
    game: String,
}

async fn table(s: &mut TestServer) -> Table {
    let alice = s.user("alice").await;
    let bobby = s.user("bobby").await;
    let carol = s.user("carol").await;
    let dave = s.user("david").await;
    let game = s.fixture_game(&alice, "alliance").await;
    s.ok(alice.conn, "RequestAssignSeats", json!([game, [[alice.id, 0]]])).await;
    s.ok(alice.conn, "RequestOpenOrCloseSeat", json!([game, 3])).await;
    s.ok(bobby.conn, "RequestJoinGame", json!([game, "", 3])).await;
    s.ok(carol.conn, "RequestObserveGame", json!([game, ""])).await;
    for c in [alice.conn, bobby.conn, carol.conn, dave.conn] {
        s.received(c);
    }
    Table { alice, bobby, carol, dave, game }
}

fn offer(target: i64, initiator: i64) -> Value {
    json!({"$type": "AllianceOffered", "Target": target, "Initiator": initiator})
}

/// What the host sees of the game: the savegame and the participation.
async fn state(s: &mut TestServer, t: &Table) -> (Value, Value) {
    let saved = s.ok(t.alice.conn, "RequestSaveGame", json!([t.game])).await;
    let info = s.ok(t.alice.conn, "RequestGameState", json!([t.game])).await;
    s.received(t.alice.conn);
    (serde_json::from_str(saved.as_str().unwrap()).unwrap(), info["Participation"].clone())
}

#[test]
fn outsiders_observers_and_other_players_are_refused() {
    rt().block_on(async {
        let mut s = TestServer::start().await;
        let t = table(&mut s).await;
        let g = t.game.clone();
        let before = state(&mut s, &t).await;
        let settings = establish_players(4, &[], false, 0)["Settings"].clone();
        let emperor_offer = offer(faction::ATREIDES, faction::EMPEROR);

        // Calls only a player may make: outsiders and observers are refused.
        let player_calls: Vec<(&str, Value)> = vec![
            ("SubmitEvent", json!([g, emperor_offer])),
            ("QueryDraft", json!([g, 0, emperor_offer])),
            ("RequestPhaseDone", json!([g])),
            ("RequestPhaseHold", json!([g])),
            ("RequestPhaseRelease", json!([g])),
            ("RequestNudgeBots", json!([g])),
            ("SendChatMessage", json!([g, {"TargetUserId": -1, "SourceUserId": t.alice.id, "Body": "hi"}])),
        ];
        for (method, args) in &player_calls {
            for who in [&t.dave, &t.carol] {
                assert_eq!(s.err(who.conn, method, args.clone()).await, error::USER_NOT_IN_GAME, "{method} by {}", who.name);
            }
        }
        // Calls for anyone in the game, observers too: outsiders are refused.
        for (method, args) in [
            ("RequestReport", json!([g, 0])),
            ("RequestGameState", json!([g])),
            ("RequestReconnectGame", json!([g])),
            ("RequestLeaveGame", json!([g])),
        ] {
            assert_eq!(s.err(t.dave.conn, method, args).await, error::USER_NOT_IN_GAME, "{method}");
        }
        // Host calls: everyone but the host is refused, the other player included.
        let host_calls: Vec<(&str, Value)> = vec![
            ("RequestUndo", json!([g, 1])),
            ("RequestRestoreRecentlyUndone", json!([g])),
            ("RequestDismissRecentlyUndone", json!([g])),
            ("RequestUndoList", json!([g])),
            ("RequestUndoPreview", json!([g, 1])),
            ("RequestSaveGame", json!([g])),
            ("RequestLoadGame", json!([g, fixture_text("alliance"), null])),
            ("RequestUpdateSettings", json!([g, settings])),
            ("SetTimer", json!([g, 10])),
            ("RequestSetSkin", json!([g, "{}"])),
            ("RequestSetBotSpeed", json!([g, 0])),
            ("RequestPhaseContinue", json!([g, 0])),
            ("RequestSetOrUnsetHost", json!([g, t.bobby.id])),
            ("RequestOpenOrCloseSeat", json!([g, 1])),
            ("RequestKick", json!([g, t.alice.id])),
            ("RequestAssignSeats", json!([g, [[t.bobby.id, 0], [t.alice.id, 3]]])),
        ];
        for (method, args) in &host_calls {
            for who in [&t.dave, &t.carol, &t.bobby] {
                assert_eq!(s.err(who.conn, method, args.clone()).await, error::NO_HOST, "{method} by {}", who.name);
            }
        }
        for who in [&t.dave, &t.carol, &t.bobby] {
            assert_eq!(s.err(who.conn, "RequestCloseGame", json!([g])).await, error::NO_CREATOR, "by {}", who.name);
        }

        // Nothing changed, and nobody but the members heard anything.
        assert_eq!(state(&mut s, &t).await, before);
        assert!(s.notifications(t.dave.conn).is_empty());
    });
}

#[test]
fn an_event_is_always_the_callers_own() {
    rt().block_on(async {
        let mut s = TestServer::start().await;
        let t = table(&mut s).await;
        let events = |saved: &Value| saved["_events"].as_array().unwrap().len();
        let (before, _) = state(&mut s, &t).await;

        // Bobby offers in the Emperor's name: the server makes it the Atreides' offer, which
        // the Atreides can't make to themselves.
        let r = s.submit(&t.bobby, &t.game, offer(faction::ATREIDES, faction::EMPEROR)).await;
        assert_eq!(r["Error"], error::INVALID_GAME_EVENT, "{r}");
        let (after, _) = state(&mut s, &t).await;
        assert_eq!(events(&after), events(&before));

        // An offer that the Atreides can make is made by the Atreides, not the faction named.
        let r = s.submit(&t.bobby, &t.game, offer(faction::EMPEROR, faction::EMPEROR)).await;
        assert_eq!(r["Success"], true, "{r}");
        let (after, _) = state(&mut s, &t).await;
        let last = after["_events"].as_array().unwrap().last().unwrap().clone();
        assert_eq!(last["$type"], "AllianceOffered");
        assert_eq!(last["Initiator"], faction::ATREIDES);
        assert_eq!(last["Target"], faction::EMPEROR);

        // A draft is answered for the caller's faction too: the Emperor offering to the
        // Atreides is valid for Alice, but for Bobby it is an offer to himself.
        let draft = s.ok(t.bobby.conn, "QueryDraft", json!([t.game, 0, offer(faction::ATREIDES, faction::EMPEROR)])).await;
        assert!(!draft["validation"].is_null(), "{draft}");
    });
}

#[test]
fn only_a_host_starts_the_game_and_the_server_draws_the_seed() {
    rt().block_on(async {
        let mut s = TestServer::start().await;
        let alice = s.user("alice").await;
        let bobby = s.user("bobby").await;
        let carol = s.user("carol").await;
        let game = s.new_game(&alice, "", "").await;
        s.ok(bobby.conn, "RequestJoinGame", json!([game, "", -1])).await;
        s.ok(carol.conn, "RequestObserveGame", json!([game, ""])).await;
        let start = establish_players(2, &[], false, 0);
        assert_eq!(s.submit(&bobby, &game, start.clone()).await["Error"], error::NO_HOST);
        assert_eq!(s.submit(&carol, &game, start.clone()).await["Error"], error::USER_NOT_IN_GAME);
        assert_eq!(s.submit(&alice, &game, start.clone()).await["Success"], true);
        let saved = s.ok(alice.conn, "RequestSaveGame", json!([game])).await;
        let saved: Value = serde_json::from_str(saved.as_str().unwrap()).unwrap();
        let established = &saved["_events"][0];
        assert_eq!(established["$type"], "EstablishPlayers");
        assert_eq!(established["Seed"], GAME_SEED, "the server's seed, not the client's ({})", start["Seed"]);
    });
}

#[test]
fn players_choose_only_their_own_faction() {
    rt().block_on(async {
        const PLAYERS_CHOOSE_FACTIONS: i64 = 1011;
        let mut s = TestServer::start().await;
        let alice = s.user("alice").await;
        let bobby = s.user("bobby").await;
        let game = s.new_game(&alice, "", "").await;
        s.ok(bobby.conn, "RequestJoinGame", json!([game, "", -1])).await;
        let start = establish_players(2, &[PLAYERS_CHOOSE_FACTIONS], false, 0);
        s.ok(alice.conn, "RequestUpdateSettings", json!([game, start["Settings"]])).await;
        assert_eq!(s.submit(&alice, &game, start).await["Success"], true);
        let view = s.latest_view(bobby.conn).unwrap();
        assert_eq!(view["phase"], "SelectingFactions");
        let seats: Vec<(i64, i64)> = s.ok(alice.conn, "RequestGameState", json!([game])).await["Participation"]["SeatedPlayers"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| (p[0].as_i64().unwrap(), p[1].as_i64().unwrap()))
            .collect();
        let alice_seat = seats.iter().find(|(u, _)| *u == alice.id).unwrap().1;

        // Bobby picks the Atreides for Alice's seat, in her name: the pick is his.
        let pick = json!({"$type": "FactionSelected", "Seat": alice_seat, "InitiatorPlayerName": "Alice Player", "Faction": faction::ATREIDES, "Initiator": 0});
        assert_eq!(s.submit(&bobby, &game, pick).await["Success"], true);
        let view = s.latest_view(bobby.conn).unwrap();
        let seat_of = |faction: &str| view["players"].as_array().unwrap().iter().find(|p| p["faction"] == faction).map(|p| p["seat"].as_i64().unwrap());
        assert_ne!(seat_of("Atreides"), Some(alice_seat));
        assert!(seat_of("Atreides").is_some());
    });
}

#[test]
fn hosts_act_only_on_members_and_real_seats() {
    rt().block_on(async {
        let mut s = TestServer::start().await;
        let t = table(&mut s).await;
        let (a, g) = (t.alice.conn, t.game.clone());
        // Hosts are made from the players, not observers or outsiders.
        assert_eq!(s.err(a, "RequestSetOrUnsetHost", json!([g, t.carol.id])).await, error::USER_NOT_IN_GAME);
        assert_eq!(s.err(a, "RequestSetOrUnsetHost", json!([g, t.dave.id])).await, error::USER_NOT_IN_GAME);
        // Kicks are for other members of the game.
        assert_eq!(s.err(a, "RequestKick", json!([g, t.dave.id])).await, error::USER_NOT_IN_GAME);
        assert_eq!(s.err(a, "RequestKick", json!([g, t.alice.id])).await, error::USER_NOT_IN_GAME);
        // Seats are assigned to players, on seats the game has, once each.
        assert_eq!(s.err(a, "RequestAssignSeats", json!([g, [[t.dave.id, 1]]])).await, error::USER_NOT_IN_GAME);
        assert_eq!(s.err(a, "RequestAssignSeats", json!([g, [[t.carol.id, 1]]])).await, error::USER_NOT_IN_GAME);
        assert_eq!(s.err(a, "RequestAssignSeats", json!([g, [[t.alice.id, 99]]])).await, error::INVALID_GAME_EVENT);
        assert_eq!(s.err(a, "RequestAssignSeats", json!([g, [[t.alice.id, 3], [t.bobby.id, 3]]])).await, error::INVALID_GAME_EVENT);
        assert_eq!(s.err(a, "RequestOpenOrCloseSeat", json!([g, 99])).await, error::INVALID_GAME_EVENT);
        // A user joins a particular seat only once the host opened it.
        assert_eq!(s.err(t.dave.conn, "RequestJoinGame", json!([g, "", 1])).await, error::SEAT_NOT_AVAILABLE_NOT_OPEN);
        s.ok(a, "RequestOpenOrCloseSeat", json!([g, 1])).await;
        s.ok(t.dave.conn, "RequestJoinGame", json!([g, "", 1])).await;
        // The host's own calls still work.
        s.ok(a, "RequestSetOrUnsetHost", json!([g, t.bobby.id])).await;
        s.ok(a, "RequestKick", json!([g, t.carol.id])).await;
        s.ok(a, "RequestAssignSeats", json!([g, [[t.alice.id, 0], [t.bobby.id, 3], [t.dave.id, -1]]])).await;
    });
}

#[test]
fn a_new_connection_is_nobody_until_it_logs_in() {
    rt().block_on(async {
        let mut s = TestServer::start().await;
        let t = table(&mut s).await;
        let c = s.connect();
        assert_eq!(s.err(c, "SubmitEvent", json!([t.game, offer(faction::ATREIDES, faction::EMPEROR)])).await, 8);
        assert_eq!(s.err(c, "RequestUndo", json!([t.game, 1])).await, 8);
        assert_eq!(s.err(c, "GetAdminInfo", json!([])).await, 8);
        // Logging in as Bobby gives Bobby's rights, not Alice's.
        s.ok(c, "RequestResumeSession", json!([t.bobby.token])).await;
        assert_eq!(s.err(c, "RequestUndo", json!([t.game, 1])).await, error::NO_HOST);
        s.ok(c, "RequestReconnectGame", json!([t.game])).await;
    });
}
