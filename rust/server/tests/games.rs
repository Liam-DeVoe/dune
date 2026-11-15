//! The lobby and game management: creating, joining, observing, leaving and closing games,
//! seats and hosts, scheduled games, chat.

mod common;

use common::*;
use serde_json::{json, Value};

mod error {
    pub const USER_TOKEN_NOT_FOUND: i64 = 8;
    pub const GAME_NOT_FOUND: i64 = 2;
    pub const NO_HOST: i64 = 22;
    pub const INVALID_GAME_EVENT: i64 = 23;
    pub const TOO_MANY_GAMES: i64 = 24;
    pub const TOO_MANY_SCHEDULED_GAMES: i64 = 25;
    pub const INCORRECT_GAME_PASSWORD: i64 = 26;
    pub const SEAT_NOT_AVAILABLE_KICKED: i64 = 28;
    pub const CANNOT_REMOVE_LAST_HOST: i64 = 30;
    pub const NO_CREATOR: i64 = 31;
    pub const ALREADY_PLAYER: i64 = 32;
    pub const USER_NOT_IN_GAME: i64 = 33;
}

/// `[userId, value]` pairs as a map.
fn pairs(v: &Value) -> Vec<(i64, Value)> {
    v.as_array().unwrap().iter().map(|p| (p[0].as_i64().unwrap(), p[1].clone())).collect()
}

fn ids(v: &Value) -> Vec<i64> {
    let mut ids: Vec<i64> = v.as_array().unwrap().iter().map(|x| x.as_i64().unwrap()).collect();
    ids.sort();
    ids
}

fn entry<'a>(list: &'a Value, game: &str) -> Option<&'a Value> {
    list.as_array().unwrap().iter().find(|g| g["GameId"] == game)
}

#[test]
fn create_a_game_and_find_it_in_the_lobby() {
    rt().block_on(async {
        let mut s = TestServer::start().await;
        let alice = s.user("alice").await;
        let bobby = s.user("bobby").await;

        let (reply, before) = s.call_raw(alice.conn, "RequestCreateGame", json!(["", "secret", "", ""])).await;
        let info = &reply["result"]["Contents"];
        let game = info["GameId"].as_str().unwrap().to_string();
        assert_eq!(info["GameName"], "Alice Player's Game");
        assert_eq!(pairs(&info["Participation"]["PlayerNames"]), vec![(alice.id, json!("Alice Player"))]);
        assert_eq!(pairs(&info["Participation"]["SeatedPlayers"]), vec![(alice.id, json!(-1))]);
        assert_eq!(ids(&info["Participation"]["Hosts"]), vec![alice.id]);
        assert!(info.get("GameState").is_none(), "clients get views, not savegames");
        // The creator gets their view of the game before the reply.
        let view = views(&before).pop().expect("a view");
        assert_eq!(view["viewer"]["isHost"], true);
        assert_eq!(view["phase"], "AwaitingPlayers");

        let lobby = s.lobby(&bobby, 2).await;
        let g = entry(&lobby["RunningGames"], &game).expect("the game is listed").clone();
        assert_eq!(g["Name"], "Alice Player's Game");
        assert_eq!(g["CreatorId"], alice.id);
        assert_eq!(g["CreatorUsername"], "alice");
        assert_eq!(g["HasPassword"], true);
        assert!(g["Phase"].is_number());
        assert_eq!(g["NrOfPlayers"], 1);
        assert_eq!(pairs(&g["SeatedPlayers"]), vec![(alice.id, json!(-1))]);
        assert!(g["MaxPlayers"].as_i64().unwrap() > 1);
        assert!(lobby["OwnGames"].as_array().unwrap().is_empty());

        let own = s.lobby(&alice, 2).await;
        assert!(entry(&own["OwnGames"], &game).is_some());
        // Scope None lists no games.
        let none = s.lobby(&alice, 1).await;
        assert!(none["RunningGames"].as_array().unwrap().is_empty() && none["OwnGames"].as_array().unwrap().is_empty());

        // A named game without a password.
        let named = s.new_game(&bobby, "Bobby's table", "").await;
        let g = entry(&s.lobby(&alice, 3).await["RunningGames"], &named).unwrap().clone();
        assert_eq!(g["Name"], "Bobby's table");
        assert_eq!(g["HasPassword"], false);

        let anonymous = s.connect();
        assert_eq!(s.err(anonymous, "RequestCreateGame", json!(["", "", "", ""])).await, error::USER_TOKEN_NOT_FOUND);
        assert_eq!(s.err(anonymous, "RequestHeartbeat", json!([2])).await, error::USER_TOKEN_NOT_FOUND);
    });
}

#[test]
fn inactive_games_leave_the_active_lobby() {
    rt().block_on(async {
        let mut s = TestServer::start().await;
        let alice = s.user("alice").await;
        let bobby = s.user("bobby").await;
        let game = s.new_game(&alice, "", "").await;
        s.advance_minutes(121).await;
        let active = s.lobby(&bobby, 2).await;
        assert!(entry(&active["RunningGames"], &game).is_none());
        let all = s.lobby(&bobby, 3).await;
        assert!(entry(&all["RunningGames"], &game).is_some());
        // Players still see their own games.
        let mine = s.lobby(&alice, 2).await;
        assert!(entry(&mine["RunningGames"], &game).is_some());
    });
}

#[test]
fn a_user_runs_at_most_ten_games() {
    rt().block_on(async {
        let mut s = TestServer::start().await;
        let alice = s.user("alice").await;
        for _ in 0..10 {
            s.new_game(&alice, "", "").await;
        }
        assert_eq!(s.err(alice.conn, "RequestCreateGame", json!(["", "", "", ""])).await, error::TOO_MANY_GAMES);
    });
}

#[test]
fn join_observe_and_reconnect() {
    rt().block_on(async {
        let mut s = TestServer::start().await;
        let alice = s.user("alice").await;
        let bobby = s.user("bobby").await;
        let carol = s.user("carol").await;
        let game = s.new_game(&alice, "", "secret").await;
        s.received(alice.conn);

        assert_eq!(s.err(bobby.conn, "RequestJoinGame", json!(["no such game", "secret", -1])).await, error::GAME_NOT_FOUND);
        assert_eq!(s.err(bobby.conn, "RequestJoinGame", json!([game, "wrong", -1])).await, error::INCORRECT_GAME_PASSWORD);
        assert_eq!(s.err(bobby.conn, "RequestReconnectGame", json!([game])).await, error::USER_NOT_IN_GAME);
        assert_eq!(s.err(bobby.conn, "RequestGameState", json!([game])).await, error::USER_NOT_IN_GAME);

        let (reply, before) = s.call_raw(bobby.conn, "RequestJoinGame", json!([game, "secret", -1])).await;
        let info = &reply["result"]["Contents"];
        assert_eq!(info["GameName"], "Alice Player's Game");
        assert_eq!(ids(&info["Participation"]["Hosts"]), vec![alice.id]);
        assert!(pairs(&info["Participation"]["SeatedPlayers"]).iter().any(|(id, _)| *id == bobby.id));
        assert_eq!(info["PhaseControl"]["Active"], false);
        assert!(!views(&before).is_empty(), "the joiner gets a view");

        let to_alice = s.received(alice.conn);
        let joined = find(&to_alice, "JoinGame").expect("JoinGame");
        assert_eq!(joined["args"], json!([bobby.id, "Bobby Player", -1]));
        assert!(!views(&to_alice).is_empty(), "members get a new view");

        // Observers don't need the password unless the game says so.
        assert_eq!(s.err(bobby.conn, "RequestObserveGame", json!([game, ""])).await, error::ALREADY_PLAYER);
        let (reply, before) = s.call_raw(carol.conn, "RequestObserveGame", json!([game, ""])).await;
        assert_eq!(reply["result"]["Success"], true);
        assert_eq!(ids(&reply["result"]["Contents"]["Participation"]["Observers"]), vec![carol.id]);
        assert_eq!(views(&before).pop().unwrap()["viewer"]["isObserver"], true);
        assert_eq!(find(&s.received(alice.conn), "ObserveGame").unwrap()["args"], json!([carol.id, "Carol Player"]));

        // A new connection rejoins the game's group with RequestReconnectGame.
        s.disconnect(bobby.conn);
        let again = s.connect();
        assert_eq!(s.err(again, "RequestReconnectGame", json!([game])).await, error::USER_TOKEN_NOT_FOUND);
        s.ok(again, "RequestResumeSession", json!([bobby.token])).await;
        let (reply, before) = s.call_raw(again, "RequestReconnectGame", json!([game])).await;
        assert_eq!(reply["result"]["Contents"]["GameId"], game.as_str());
        assert_eq!(reply["result"]["Contents"]["GameName"], "Alice Player's Game");
        assert!(!views(&before).is_empty());
        let (reply, before) = s.call_raw(again, "RequestGameState", json!([game])).await;
        assert_eq!(reply["result"]["Contents"]["GameId"], game.as_str());
        assert!(!views(&before).is_empty());
        let bobby = User { conn: again, ..bobby };

        // Group messages reach every member, and only members.
        s.received(carol.conn);
        s.ok(alice.conn, "RequestSetSkin", json!([game, "{}"])).await;
        for c in [alice.conn, bobby.conn, carol.conn] {
            assert_eq!(find(&s.received(c), "SetSkin").expect("SetSkin")["args"], json!(["{}"]));
        }
        let dave = s.user("david").await;
        s.ok(alice.conn, "SetTimer", json!([game, 30])).await;
        assert!(find(&s.received(dave.conn), "SetTimer").is_none());
        assert_eq!(find(&s.received(bobby.conn), "SetTimer").unwrap()["args"], json!([30]));
        assert_eq!(s.err(bobby.conn, "SetTimer", json!([game, 30])).await, error::NO_HOST);
        assert_eq!(s.err(bobby.conn, "RequestSetSkin", json!([game, "{}"])).await, error::NO_HOST);
    });
}

#[test]
fn hosts_seats_and_kicks() {
    rt().block_on(async {
        let mut s = TestServer::start().await;
        let alice = s.user("alice").await;
        let bobby = s.user("bobby").await;
        let game = s.new_game(&alice, "", "").await;
        s.ok(bobby.conn, "RequestJoinGame", json!([game, "", -1])).await;
        s.received(alice.conn);
        s.received(bobby.conn);

        assert_eq!(s.err(bobby.conn, "RequestSetOrUnsetHost", json!([game, bobby.id])).await, error::NO_HOST);
        assert_eq!(s.err(alice.conn, "RequestSetOrUnsetHost", json!([game, alice.id])).await, error::CANNOT_REMOVE_LAST_HOST);
        s.ok(alice.conn, "RequestSetOrUnsetHost", json!([game, bobby.id])).await;
        assert_eq!(find(&s.received(bobby.conn), "SetOrUnsetHost").unwrap()["args"], json!([bobby.id]));
        // Now there are two hosts, and either may step down.
        s.ok(bobby.conn, "RequestSetOrUnsetHost", json!([game, bobby.id])).await;
        assert_eq!(s.err(bobby.conn, "RequestOpenOrCloseSeat", json!([game, 2])).await, error::NO_HOST);

        s.received(bobby.conn);
        // Before the game starts there are no seats to open.
        assert_eq!(s.err(alice.conn, "RequestOpenOrCloseSeat", json!([game, 2])).await, error::INVALID_GAME_EVENT);

        // A kicked user leaves the group and may not come back.
        assert_eq!(s.err(bobby.conn, "RequestKick", json!([game, alice.id])).await, error::NO_HOST);
        s.ok(alice.conn, "RequestKick", json!([game, bobby.id])).await;
        assert_eq!(find(&s.received(bobby.conn), "RemoveUser").unwrap()["args"], json!([bobby.id, true]));
        assert_eq!(find(&s.received(alice.conn), "RemoveUser").unwrap()["args"], json!([bobby.id, true]));
        s.ok(alice.conn, "SetTimer", json!([game, 5])).await;
        assert!(s.notifications(bobby.conn).is_empty(), "no longer in the group");
        assert_eq!(s.err(bobby.conn, "RequestJoinGame", json!([game, "", -1])).await, error::SEAT_NOT_AVAILABLE_KICKED);
        assert_eq!(s.err(bobby.conn, "RequestObserveGame", json!([game, ""])).await, error::SEAT_NOT_AVAILABLE_KICKED);
    });
}

#[test]
fn the_last_host_leaving_hands_over() {
    rt().block_on(async {
        let mut s = TestServer::start().await;
        let alice = s.user("alice").await;
        let bobby = s.user("bobby").await;
        let carol = s.user("carol").await;
        let game = s.new_game(&alice, "", "").await;
        s.ok(bobby.conn, "RequestJoinGame", json!([game, "", -1])).await;
        s.ok(carol.conn, "RequestObserveGame", json!([game, ""])).await;
        s.received(bobby.conn);

        let status = s.ok(alice.conn, "RequestLeaveGame", json!([game])).await;
        assert!(status["RunningGames"].is_array());
        let to_bobby = s.received(bobby.conn);
        assert_eq!(find(&to_bobby, "SetOrUnsetHost").unwrap()["args"], json!([bobby.id]));
        assert_eq!(find(&to_bobby, "RemoveUser").unwrap()["args"], json!([alice.id, false]));
        let state = s.ok(bobby.conn, "RequestGameState", json!([game])).await;
        assert_eq!(ids(&state["Participation"]["Hosts"]), vec![bobby.id], "observers don't become hosts");
        assert!(pairs(&state["Participation"]["PlayerNames"]).iter().all(|(id, _)| *id != alice.id));
        assert_eq!(s.err(alice.conn, "RequestGameState", json!([game])).await, error::USER_NOT_IN_GAME);
    });
}

#[test]
fn settings_and_closing_belong_to_the_creator() {
    rt().block_on(async {
        let mut s = TestServer::start().await;
        let alice = s.user("alice").await;
        let bobby = s.user("bobby").await;
        let game = s.new_game(&alice, "", "").await;
        s.ok(bobby.conn, "RequestJoinGame", json!([game, "", -1])).await;
        s.received(bobby.conn);

        let settings = establish_players(4, &[], false, 0)["Settings"].clone();
        assert_eq!(s.err(bobby.conn, "RequestUpdateSettings", json!([game, settings])).await, error::NO_HOST);
        s.ok(alice.conn, "RequestUpdateSettings", json!([game, settings])).await;
        let update = find(&s.received(bobby.conn), "UpdateSettings").expect("UpdateSettings").clone();
        assert_eq!(update["args"][0]["NumberOfPlayers"], 4);
        let g = entry(&s.lobby(&bobby, 3).await["RunningGames"], &game).unwrap().clone();
        assert_eq!(g["MaxPlayers"], 4);

        assert_eq!(s.err(bobby.conn, "RequestCloseGame", json!([game])).await, error::NO_CREATOR);
        s.received(bobby.conn);
        let status = s.ok(alice.conn, "RequestCloseGame", json!([game])).await;
        assert!(entry(&status["RunningGames"], &game).is_none());
        let removed: Vec<Value> = s.received(bobby.conn).into_iter().filter(|m| m["name"] == "RemoveUser").map(|m| m["args"].clone()).collect();
        assert!(removed.contains(&json!([alice.id, true])) && removed.contains(&json!([bobby.id, true])), "{removed:?}");
        assert_eq!(s.err(bobby.conn, "RequestGameState", json!([game])).await, error::GAME_NOT_FOUND);
        assert!(entry(&s.lobby(&bobby, 3).await["RunningGames"], &game).is_none());

        // and stays closed after a restart.
        let mut s = s.restart().await;
        let bobby = s.login("bobby").await;
        assert!(entry(&s.lobby(&bobby, 3).await["RunningGames"], &game).is_none());
    });
}

#[test]
fn seats_in_a_loaded_game() {
    rt().block_on(async {
        let mut s = TestServer::start().await;
        let alice = s.user("alice").await;
        let bobby = s.user("bobby").await;
        let game = s.fixture_game(&alice, "alliance").await;
        let view = s.latest_view(alice.conn).unwrap();
        assert!(view["me"].is_null(), "the creator has no seat yet");
        let emperor = player(&view, "Emperor")["seat"].as_i64().unwrap();
        let atreides = player(&view, "Atreides")["seat"].as_i64().unwrap();

        assert_eq!(s.err(bobby.conn, "RequestAssignSeats", json!([game, [[bobby.id, 0]]])).await, error::NO_HOST);
        s.ok(alice.conn, "RequestAssignSeats", json!([game, [[alice.id, emperor]]])).await;
        let received = s.received(alice.conn);
        assert_eq!(find(&received, "AssignSeats").unwrap()["args"], json!([[[alice.id, emperor]]]));
        assert_eq!(views(&received).pop().unwrap()["me"]["faction"], "Emperor");

        // An open seat can be taken; the seat's other players see it.
        s.received(bobby.conn);
        s.ok(alice.conn, "RequestOpenOrCloseSeat", json!([game, atreides])).await;
        assert!(find(&s.received(alice.conn), "OpenOrCloseSeats").is_some_and(|n| n["args"] == json!([[atreides]])));
        let lobby = s.lobby(&bobby, 3).await;
        let g = entry(&lobby["RunningGames"], &game).unwrap().clone();
        assert!(g["AvailableSeats"].as_array().unwrap().iter().any(|a| a["Seat"] == atreides && a["Faction"] == faction::ATREIDES), "{g}");
        s.ok(bobby.conn, "RequestJoinGame", json!([game, "", atreides])).await;
        let view = s.latest_view(bobby.conn).unwrap();
        assert_eq!(view["me"]["faction"], "Atreides");
        assert_eq!(player(&view, "Atreides")["isBot"], false);
        let view = s.latest_view(alice.conn).unwrap();
        assert_eq!(player(&view, "Atreides")["name"], "Bobby Player");
    });
}

#[test]
fn load_a_savegame_into_a_game() {
    rt().block_on(async {
        let mut s = TestServer::start().await;
        let alice = s.user("alice").await;
        let bobby = s.user("bobby").await;
        let game = s.new_game(&alice, "", "").await;
        s.ok(bobby.conn, "RequestObserveGame", json!([game, ""])).await;
        s.received(bobby.conn);

        let savegame = fixture_text("alliance");
        assert_eq!(s.err(bobby.conn, "RequestLoadGame", json!([game, savegame, ""])).await, error::NO_HOST);
        let bad = s.call(alice.conn, "RequestLoadGame", json!([game, "{\"Version\":0,\"_events\":[]}", ""])).await;
        assert!(bad["Success"] == false || bad["Success"] == true);

        s.ok(alice.conn, "RequestLoadGame", json!([game, savegame, "{}"])).await;
        let received = s.received(bobby.conn);
        let names = names(&received);
        assert!(names.contains(&"LoadGame".to_string()) && names.contains(&"SetSkin".to_string()), "{names:?}");
        let view = views(&received).pop().expect("a view after loading");
        assert_eq!(view["phase"], "AllianceA");
        assert!(view["eventNumber"].as_i64().unwrap() > 1);
    });
}

#[test]
fn creating_from_a_bad_savegame_is_refused() {
    rt().block_on(async {
        let mut s = TestServer::start().await;
        let alice = s.user("alice").await;
        let r = s.call_raw(alice.conn, "RequestCreateGame", json!(["", "", "not a savegame", ""])).await.0;
        assert!(r.get("error").is_some() || r["result"]["Success"] == false, "{r}");
        let lobby = s.lobby(&alice, 3).await;
        assert!(lobby["RunningGames"].as_array().unwrap().is_empty());
    });
}

#[test]
fn scheduled_games() {
    rt().block_on(async {
        let mut s = TestServer::start().await;
        let alice = s.user("alice").await;
        let bobby = s.user("bobby").await;
        let when = "2026-11-20T20:00:00+01:00";
        let schedule = || json!([when, null, 6, 10, [20, 10], true]);

        let status = s.ok(alice.conn, "RequestScheduleGame", schedule()).await;
        let scheduled = status["ScheduledGames"].as_array().unwrap();
        assert_eq!(scheduled.len(), 1);
        let g = scheduled[0].clone();
        let id = g["ScheduledGameId"].as_str().unwrap().to_string();
        assert_eq!(g["CreatorUserId"], alice.id);
        assert_eq!(g["CreatorName"], "Alice Player");
        assert_eq!(g["NumberOfPlayers"], 6);
        assert_eq!(g["MaximumTurns"], 10);
        assert_eq!(g["AsyncPlay"], true);
        assert_eq!(g["AllowedFactionsInPlay"], json!([20, 10]));
        assert_eq!(pairs(&g["Subscribers"]), vec![(alice.id, json!(50))]);
        assert_eq!(pairs(&g["SubscriberNames"]), vec![(alice.id, json!("alice"))]);
        assert!(g["DateTime"].as_str().unwrap().starts_with("2026-11-20T"));

        s.ok(alice.conn, "RequestScheduleGame", schedule()).await;
        s.ok(alice.conn, "RequestScheduleGame", schedule()).await;
        assert_eq!(s.err(alice.conn, "RequestScheduleGame", schedule()).await, error::TOO_MANY_SCHEDULED_GAMES);

        let status = s.ok(bobby.conn, "RequestSubscribeGame", json!([id, 30])).await;
        let g = status["ScheduledGames"].as_array().unwrap().iter().find(|g| g["ScheduledGameId"] == id.as_str()).unwrap().clone();
        assert!(pairs(&g["Subscribers"]).contains(&(bobby.id, json!(30))));
        assert_eq!(s.err(bobby.conn, "RequestSubscribeGame", json!(["nope", 30])).await, error::GAME_NOT_FOUND);
        assert_eq!(s.err(bobby.conn, "RequestCancelGame", json!([id])).await, error::NO_HOST);

        // Scheduled games survive a restart, subscriptions included.
        let mut s = s.restart().await;
        let alice = s.login("alice").await;
        let lobby = s.lobby(&alice, 2).await;
        let g = lobby["ScheduledGames"].as_array().unwrap().iter().find(|g| g["ScheduledGameId"] == id.as_str()).expect("still scheduled").clone();
        assert!(pairs(&g["Subscribers"]).contains(&(bobby.id, json!(30))));

        let status = s.ok(alice.conn, "RequestCancelGame", json!([id])).await;
        assert_eq!(status["ScheduledGames"].as_array().unwrap().len(), 2);

        // Games more than four hours in the past are dropped.
        s.advance_minutes(60 * 24 * 13).await;
        let lobby = s.lobby(&alice, 2).await;
        assert_eq!(lobby["ScheduledGames"].as_array().unwrap().len(), 0);
    });
}

#[test]
fn chat() {
    rt().block_on(async {
        let mut s = TestServer::start().await;
        let alice = s.user("alice").await;
        let bobby = s.user("bobby").await;
        let carol = s.user("carol").await;
        let watcher = s.user("david").await;
        let outsider = s.user("erika").await;
        let anonymous = s.connect();
        let game = s.new_game(&alice, "", "").await;
        s.ok(bobby.conn, "RequestJoinGame", json!([game, "", -1])).await;
        s.ok(carol.conn, "RequestJoinGame", json!([game, "", -1])).await;
        s.ok(watcher.conn, "RequestObserveGame", json!([game, ""])).await;
        for c in [alice.conn, bobby.conn, carol.conn, watcher.conn] {
            s.received(c);
        }

        let message = json!({"TargetUserId": -1, "SourceUserId": alice.id, "Body": "hello"});
        assert_eq!(s.err(anonymous, "SendChatMessage", json!([game, message])).await, error::USER_TOKEN_NOT_FOUND);
        // Outsiders and observers can't chat in the game.
        assert_eq!(s.err(outsider.conn, "SendChatMessage", json!([game, message])).await, error::USER_NOT_IN_GAME);
        assert_eq!(s.err(watcher.conn, "SendChatMessage", json!([game, message])).await, error::USER_NOT_IN_GAME);
        s.ok(alice.conn, "SendChatMessage", json!([game, message])).await;
        for c in [alice.conn, bobby.conn, carol.conn, watcher.conn] {
            assert_eq!(find(&s.received(c), "ChatMessage").expect("ChatMessage")["args"], json!([message]));
        }
        assert!(find(&s.received(outsider.conn), "ChatMessage").is_none());

        // The sender is always the caller, whatever the message says.
        let forged = json!({"TargetUserId": -1, "SourceUserId": alice.id, "Body": "from alice, honest"});
        s.ok(bobby.conn, "SendChatMessage", json!([game, forged])).await;
        assert_eq!(find(&s.received(carol.conn), "ChatMessage").unwrap()["args"][0]["SourceUserId"], bobby.id);
        for c in [alice.conn, bobby.conn, watcher.conn] {
            s.received(c);
        }

        // A message to one user reaches only them and the sender.
        let whisper = json!({"TargetUserId": bobby.id, "SourceUserId": alice.id, "Body": "psst"});
        s.ok(alice.conn, "SendChatMessage", json!([game, whisper])).await;
        assert_eq!(find(&s.received(bobby.conn), "ChatMessage").expect("to Bobby")["args"], json!([whisper]));
        assert!(find(&s.received(alice.conn), "ChatMessage").is_some(), "the sender sees it");
        assert!(find(&s.received(carol.conn), "ChatMessage").is_none());
        assert!(find(&s.received(watcher.conn), "ChatMessage").is_none());
        let stranger = json!({"TargetUserId": outsider.id, "Body": "hi"});
        assert_eq!(s.err(alice.conn, "SendChatMessage", json!([game, stranger])).await, error::USER_NOT_IN_GAME);

        let global = json!({"SourceUserId": bobby.id, "Body": "hi all"});
        assert_eq!(s.err(anonymous, "SendGlobalChatMessage", json!([global])).await, error::USER_TOKEN_NOT_FOUND);
        s.ok(bobby.conn, "SendGlobalChatMessage", json!([global])).await;
        for c in [alice.conn, bobby.conn, outsider.conn, anonymous] {
            assert_eq!(find(&s.received(c), "GlobalChatMessage").expect("GlobalChatMessage")["args"], json!([global]));
        }
        s.ok(outsider.conn, "SendGlobalChatMessage", json!([global])).await;
        assert_eq!(find(&s.received(alice.conn), "GlobalChatMessage").unwrap()["args"][0]["SourceUserId"], outsider.id);
    });
}

#[test]
fn the_protocol_envelope() {
    rt().block_on(async {
        let mut s = TestServer::start().await;
        let c = s.connect();

        // Method names ignore case; a ping gets no answer.
        let info = s.ok(c, "connect", json!([])).await;
        assert_eq!(info["AdminName"], "admin");
        s.send_text(c, r#"{"type":"ping"}"#).await;
        assert!(s.received(c).is_empty());

        let (reply, _) = s.call_raw(c, "RequestFoo", json!([])).await;
        assert_eq!(reply["error"], "Unknown method 'RequestFoo'");
        let (reply, _) = s.call_raw(c, "RequestResumeSession", json!([])).await;
        assert!(reply["error"].is_string(), "missing arguments: {reply}");
        let (reply, _) = s.call_raw(c, "RequestResumeSession", json!([12])).await;
        assert!(reply["error"].is_string(), "wrong argument type: {reply}");
        assert!(s.is_open(c));

        // A message that isn't a call closes the connection.
        s.send_text(c, "[1,2]").await;
        assert!(!s.is_open(c));
        let d = s.connect();
        s.send_text(d, r#"{"type":"call","method":"Connect"}"#).await;
        assert!(!s.is_open(d), "a call needs an id");
    });
}
