//! The lobby and game management: creating, joining, observing, leaving and closing games,
//! seats and hosts, scheduled games, chat.

mod common;

use common::*;
use serde_json::{json, Value};

mod error {
    pub const NOT_LOGGED_IN: &str = "notLoggedIn";
    pub const GAME_NOT_FOUND: &str = "gameNotFound";
    pub const NO_HOST: &str = "noHost";
    pub const INVALID_GAME_EVENT: &str = "invalidGameEvent";
    pub const TOO_MANY_GAMES: &str = "tooManyGames";
    pub const TOO_MANY_SCHEDULED_GAMES: &str = "tooManyScheduledGames";
    pub const INCORRECT_GAME_PASSWORD: &str = "incorrectGamePassword";
    pub const SEAT_NOT_AVAILABLE_KICKED: &str = "seatNotAvailableKicked";
    pub const CANNOT_REMOVE_LAST_HOST: &str = "cannotRemoveLastHost";
    pub const NO_CREATOR: &str = "noCreator";
    pub const ALREADY_PLAYER: &str = "alreadyPlayer";
    pub const USER_NOT_IN_GAME: &str = "userNotInGame";
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
    list.as_array().unwrap().iter().find(|g| g["gameId"] == game)
}

#[test]
fn create_a_game_and_find_it_in_the_lobby() {
    rt().block_on(async {
        let mut s = TestServer::start().await;
        let alice = s.user("alice").await;
        let bobby = s.user("bobby").await;

        let (reply, before) = s.call_raw(alice.conn, "createGame", json!({"name": "", "passwordHash": "secret", "savegame": "", "skin": ""})).await;
        let info = &reply["ok"];
        let game = info["gameId"].as_str().unwrap().to_string();
        assert_eq!(info["gameName"], "Alice Player's Game");
        assert_eq!(pairs(&info["participation"]["playerNames"]), vec![(alice.id, json!("Alice Player"))]);
        assert_eq!(pairs(&info["participation"]["seatedPlayers"]), vec![(alice.id, json!(-1))]);
        assert_eq!(ids(&info["participation"]["hosts"]), vec![alice.id]);
        assert!(info.get("GameState").is_none(), "clients get views, not savegames");
        // The creator gets their view of the game before the reply.
        let view = views(&before).pop().expect("a view");
        assert_eq!(view["viewer"]["isHost"], true);
        assert_eq!(view["phase"], "AwaitingPlayers");

        let lobby = s.lobby(&bobby, "Active").await;
        let g = entry(&lobby["runningGames"], &game).expect("the game is listed").clone();
        assert_eq!(g["name"], "Alice Player's Game");
        assert_eq!(g["creatorId"], alice.id);
        assert_eq!(g["creatorUserName"], "alice");
        assert_eq!(g["hasPassword"], true);
        assert_eq!(g["phase"], "AwaitingPlayers");
        assert_eq!(g["mainPhase"], "Started");
        assert_eq!(g["players"], 1);
        assert_eq!(pairs(&g["seatedPlayers"]), vec![(alice.id, json!(-1))]);
        assert!(g["maxPlayers"].as_i64().unwrap() > 1);
        assert!(lobby["ownGames"].as_array().unwrap().is_empty());

        let own = s.lobby(&alice, "Active").await;
        assert!(entry(&own["ownGames"], &game).is_some());
        // Scope None lists no games.
        let none = s.lobby(&alice, "None").await;
        assert!(none["runningGames"].as_array().unwrap().is_empty() && none["ownGames"].as_array().unwrap().is_empty());

        // A named game without a password.
        let named = s.new_game(&bobby, "Bobby's table", "").await;
        let g = entry(&s.lobby(&alice, "All").await["runningGames"], &named).unwrap().clone();
        assert_eq!(g["name"], "Bobby's table");
        assert_eq!(g["hasPassword"], false);

        let anonymous = s.connect();
        assert_eq!(s.err(anonymous, "createGame", json!({"name": "", "passwordHash": "", "savegame": "", "skin": ""})).await, error::NOT_LOGGED_IN);
        assert_eq!(s.err(anonymous, "heartbeat", json!({"scope": "Active"})).await, error::NOT_LOGGED_IN);
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
        let active = s.lobby(&bobby, "Active").await;
        assert!(entry(&active["runningGames"], &game).is_none());
        let all = s.lobby(&bobby, "All").await;
        assert!(entry(&all["runningGames"], &game).is_some());
        // Players still see their own games.
        let mine = s.lobby(&alice, "Active").await;
        assert!(entry(&mine["runningGames"], &game).is_some());
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
        assert_eq!(s.err(alice.conn, "createGame", json!({"name": "", "passwordHash": "", "savegame": "", "skin": ""})).await, error::TOO_MANY_GAMES);
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

        assert_eq!(s.err(bobby.conn, "joinGame", json!({"game": "no such game", "passwordHash": "secret", "seat": null})).await, error::GAME_NOT_FOUND);
        assert_eq!(s.err(bobby.conn, "joinGame", json!({"game": game, "passwordHash": "wrong", "seat": null})).await, error::INCORRECT_GAME_PASSWORD);
        assert_eq!(s.err(bobby.conn, "reconnectGame", json!({"game": game})).await, error::USER_NOT_IN_GAME);
        assert_eq!(s.err(bobby.conn, "gameState", json!({"game": game})).await, error::USER_NOT_IN_GAME);

        let (reply, before) = s.call_raw(bobby.conn, "joinGame", json!({"game": game, "passwordHash": "secret", "seat": null})).await;
        let info = &reply["ok"];
        assert_eq!(info["gameName"], "Alice Player's Game");
        assert_eq!(ids(&info["participation"]["hosts"]), vec![alice.id]);
        assert!(pairs(&info["participation"]["seatedPlayers"]).iter().any(|(id, _)| *id == bobby.id));
        assert_eq!(info["phaseControl"]["active"], false);
        assert!(!views(&before).is_empty(), "the joiner gets a view");

        let to_alice = s.received(alice.conn);
        let joined = find(&to_alice, "joinGame").expect("joinGame");
        assert_eq!(joined, &json!({"user": bobby.id, "name": "Bobby Player", "seat": -1}));
        assert!(!views(&to_alice).is_empty(), "members get a new view");

        // Observers don't need the password unless the game says so.
        assert_eq!(s.err(bobby.conn, "observeGame", json!({"game": game, "passwordHash": ""})).await, error::ALREADY_PLAYER);
        let (reply, before) = s.call_raw(carol.conn, "observeGame", json!({"game": game, "passwordHash": ""})).await;
        assert!(is_ok(&reply), "{reply}");
        assert_eq!(ids(&reply["ok"]["participation"]["observers"]), vec![carol.id]);
        assert_eq!(views(&before).pop().unwrap()["viewer"]["isObserver"], true);
        assert_eq!(find(&s.received(alice.conn), "observeGame").unwrap(), &json!({"user": carol.id, "name": "Carol Player"}));

        // A new connection rejoins the game's group with reconnectGame.
        s.disconnect(bobby.conn);
        let again = s.connect();
        assert_eq!(s.err(again, "reconnectGame", json!({"game": game})).await, error::NOT_LOGGED_IN);
        s.ok(again, "authenticate", json!({"token": bobby.token})).await;
        let (reply, before) = s.call_raw(again, "reconnectGame", json!({"game": game})).await;
        assert_eq!(reply["ok"]["gameId"], game.as_str());
        assert_eq!(reply["ok"]["gameName"], "Alice Player's Game");
        assert!(!views(&before).is_empty());
        let (reply, before) = s.call_raw(again, "gameState", json!({"game": game})).await;
        assert_eq!(reply["ok"]["gameId"], game.as_str());
        assert!(!views(&before).is_empty());
        let bobby = User { conn: again, ..bobby };

        // Group messages reach every member, and only members.
        s.received(carol.conn);
        s.ok(alice.conn, "setSkin", json!({"game": game, "skin": "{}"})).await;
        for c in [alice.conn, bobby.conn, carol.conn] {
            assert_eq!(find(&s.received(c), "setSkin").expect("setSkin"), &json!({"skin": "{}"}));
        }
        let dave = s.user("david").await;
        s.ok(alice.conn, "setTimer", json!({"game": game, "value": 30})).await;
        assert!(find(&s.received(dave.conn), "setTimer").is_none());
        assert_eq!(find(&s.received(bobby.conn), "setTimer").unwrap(), &json!({"value": 30}));
        assert_eq!(s.err(bobby.conn, "setTimer", json!({"game": game, "value": 30})).await, error::NO_HOST);
        assert_eq!(s.err(bobby.conn, "setSkin", json!({"game": game, "skin": "{}"})).await, error::NO_HOST);
    });
}

#[test]
fn hosts_seats_and_kicks() {
    rt().block_on(async {
        let mut s = TestServer::start().await;
        let alice = s.user("alice").await;
        let bobby = s.user("bobby").await;
        let game = s.new_game(&alice, "", "").await;
        s.ok(bobby.conn, "joinGame", json!({"game": game, "passwordHash": "", "seat": null})).await;
        s.received(alice.conn);
        s.received(bobby.conn);

        assert_eq!(s.err(bobby.conn, "setOrUnsetHost", json!({"game": game, "user": bobby.id})).await, error::NO_HOST);
        assert_eq!(s.err(alice.conn, "setOrUnsetHost", json!({"game": game, "user": alice.id})).await, error::CANNOT_REMOVE_LAST_HOST);
        s.ok(alice.conn, "setOrUnsetHost", json!({"game": game, "user": bobby.id})).await;
        assert_eq!(find(&s.received(bobby.conn), "setOrUnsetHost").unwrap(), &json!({"user": bobby.id}));
        // Now there are two hosts, and either may step down.
        s.ok(bobby.conn, "setOrUnsetHost", json!({"game": game, "user": bobby.id})).await;
        assert_eq!(s.err(bobby.conn, "openOrCloseSeat", json!({"game": game, "seat": 2})).await, error::NO_HOST);

        s.received(bobby.conn);
        // Before the game starts there are no seats to open.
        assert_eq!(s.err(alice.conn, "openOrCloseSeat", json!({"game": game, "seat": 2})).await, error::INVALID_GAME_EVENT);

        // A kicked user leaves the group and may not come back.
        assert_eq!(s.err(bobby.conn, "kick", json!({"game": game, "user": alice.id})).await, error::NO_HOST);
        s.ok(alice.conn, "kick", json!({"game": game, "user": bobby.id})).await;
        assert_eq!(find(&s.received(bobby.conn), "removeUser").unwrap(), &json!({"user": bobby.id, "kick": true}));
        assert_eq!(find(&s.received(alice.conn), "removeUser").unwrap(), &json!({"user": bobby.id, "kick": true}));
        s.ok(alice.conn, "setTimer", json!({"game": game, "value": 5})).await;
        assert!(s.notifications(bobby.conn).is_empty(), "no longer in the group");
        assert_eq!(s.err(bobby.conn, "joinGame", json!({"game": game, "passwordHash": "", "seat": null})).await, error::SEAT_NOT_AVAILABLE_KICKED);
        assert_eq!(s.err(bobby.conn, "observeGame", json!({"game": game, "passwordHash": ""})).await, error::SEAT_NOT_AVAILABLE_KICKED);
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
        s.ok(bobby.conn, "joinGame", json!({"game": game, "passwordHash": "", "seat": null})).await;
        s.ok(carol.conn, "observeGame", json!({"game": game, "passwordHash": ""})).await;
        s.received(bobby.conn);

        let status = s.ok(alice.conn, "leaveGame", json!({"game": game})).await;
        assert!(status["runningGames"].is_array());
        let to_bobby = s.received(bobby.conn);
        assert_eq!(find(&to_bobby, "setOrUnsetHost").unwrap(), &json!({"user": bobby.id}));
        assert_eq!(find(&to_bobby, "removeUser").unwrap(), &json!({"user": alice.id, "kick": false}));
        let state = s.ok(bobby.conn, "gameState", json!({"game": game})).await;
        assert_eq!(ids(&state["participation"]["hosts"]), vec![bobby.id], "observers don't become hosts");
        assert!(pairs(&state["participation"]["playerNames"]).iter().all(|(id, _)| *id != alice.id));
        assert_eq!(s.err(alice.conn, "gameState", json!({"game": game})).await, error::USER_NOT_IN_GAME);
    });
}

#[test]
fn settings_and_closing_belong_to_the_creator() {
    rt().block_on(async {
        let mut s = TestServer::start().await;
        let alice = s.user("alice").await;
        let bobby = s.user("bobby").await;
        let game = s.new_game(&alice, "", "").await;
        s.ok(bobby.conn, "joinGame", json!({"game": game, "passwordHash": "", "seat": null})).await;
        s.received(bobby.conn);

        let settings = establish_players(4, &[], false, 0)["settings"].clone();
        assert_eq!(s.err(bobby.conn, "updateSettings", json!({"game": game, "settings": settings})).await, error::NO_HOST);
        s.ok(alice.conn, "updateSettings", json!({"game": game, "settings": settings})).await;
        let update = find(&s.received(bobby.conn), "updateSettings").expect("updateSettings").clone();
        assert_eq!(update["settings"]["numberOfPlayers"], 4, "settings are the engine's GameSettings");
        let g = entry(&s.lobby(&bobby, "All").await["runningGames"], &game).unwrap().clone();
        assert_eq!(g["maxPlayers"], 4);

        assert_eq!(s.err(bobby.conn, "closeGame", json!({"game": game})).await, error::NO_CREATOR);
        s.received(bobby.conn);
        let status = s.ok(alice.conn, "closeGame", json!({"game": game})).await;
        assert!(entry(&status["runningGames"], &game).is_none());
        let removed: Vec<Value> = s.received(bobby.conn).into_iter().filter(|m| m["notify"] == "removeUser").map(|m| m["data"].clone()).collect();
        assert!(removed.contains(&json!({"user": alice.id, "kick": true})) && removed.contains(&json!({"user": bobby.id, "kick": true})), "{removed:?}");
        assert_eq!(s.err(bobby.conn, "gameState", json!({"game": game})).await, error::GAME_NOT_FOUND);
        assert!(entry(&s.lobby(&bobby, "All").await["runningGames"], &game).is_none());

        // and stays closed after a restart.
        let mut s = s.restart().await;
        let bobby = s.login("bobby").await;
        assert!(entry(&s.lobby(&bobby, "All").await["runningGames"], &game).is_none());
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

        assert_eq!(s.err(bobby.conn, "assignSeats", json!({"game": game, "seats": [[bobby.id, 0]]})).await, error::NO_HOST);
        s.ok(alice.conn, "assignSeats", json!({"game": game, "seats": [[alice.id, emperor]]})).await;
        let received = s.received(alice.conn);
        assert_eq!(find(&received, "assignSeats").unwrap(), &json!({"seats": [[alice.id, emperor]]}));
        assert_eq!(views(&received).pop().unwrap()["me"]["faction"], "Emperor");

        // An open seat can be taken; the seat's other players see it.
        s.received(bobby.conn);
        s.ok(alice.conn, "openOrCloseSeat", json!({"game": game, "seat": atreides})).await;
        assert!(find(&s.received(alice.conn), "openOrCloseSeats").is_some_and(|n| *n == json!({"seats": [atreides]})));
        let lobby = s.lobby(&bobby, "All").await;
        let g = entry(&lobby["runningGames"], &game).unwrap().clone();
        assert!(g["availableSeats"].as_array().unwrap().iter().any(|a| a["seat"] == atreides && a["faction"] == "Atreides"), "{g}");
        s.ok(bobby.conn, "joinGame", json!({"game": game, "passwordHash": "", "seat": atreides})).await;
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
        s.ok(bobby.conn, "observeGame", json!({"game": game, "passwordHash": ""})).await;
        s.received(bobby.conn);

        let savegame = fixture_text("alliance");
        assert_eq!(s.err(bobby.conn, "loadGame", json!({"game": game, "savegame": savegame, "skin": ""})).await, error::NO_HOST);
        let bad = s.call(alice.conn, "loadGame", json!({"game": game, "savegame": "{\"Version\":0,\"_events\":[]}", "skin": ""})).await;
        assert!(is_ok(&bad) || code(&bad) == error::INVALID_GAME_EVENT, "{bad}");

        s.ok(alice.conn, "loadGame", json!({"game": game, "savegame": savegame, "skin": "{}"})).await;
        let received = s.received(bobby.conn);
        let names = names(&received);
        assert!(names.contains(&"loadGame".to_string()) && names.contains(&"setSkin".to_string()), "{names:?}");
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
        let r = s.call_raw(alice.conn, "createGame", json!({"name": "", "passwordHash": "", "savegame": "not a savegame", "skin": ""})).await.0;
        assert_eq!(code(&r), error::INVALID_GAME_EVENT, "{r}");
        let lobby = s.lobby(&alice, "All").await;
        assert!(lobby["runningGames"].as_array().unwrap().is_empty());
    });
}

#[test]
fn scheduled_games() {
    rt().block_on(async {
        let mut s = TestServer::start().await;
        let alice = s.user("alice").await;
        let bobby = s.user("bobby").await;
        let when = "2026-11-20T20:00:00+01:00";
        let schedule = || json!({"starts": when, "ruleset": null, "players": 6, "maxTurns": 10, "factions": ["Atreides", "Harkonnen"], "asyncPlay": true});

        let status = s.ok(alice.conn, "scheduleGame", schedule()).await;
        let scheduled = status["scheduledGames"].as_array().unwrap();
        assert_eq!(scheduled.len(), 1);
        let g = scheduled[0].clone();
        let id = g["scheduledGameId"].as_str().unwrap().to_string();
        assert_eq!(g["creatorId"], alice.id);
        assert_eq!(g["creatorName"], "Alice Player");
        assert_eq!(g["players"], 6);
        assert_eq!(g["maxTurns"], 10);
        assert_eq!(g["ruleset"], Value::Null);
        assert_eq!(g["asyncPlay"], true);
        assert_eq!(g["allowedFactions"], json!(["Atreides", "Harkonnen"]));
        assert_eq!(g["subscribers"], json!([{"userId": alice.id, "userName": "alice", "subscription": "CertainAsHost"}]));
        assert!(g["starts"].as_str().unwrap().starts_with("2026-11-20T"));
        let mut unknown = schedule();
        unknown["factions"] = json!(["Arrakis"]);
        assert_eq!(code(&s.call_raw(alice.conn, "scheduleGame", unknown).await.0), "badCall");

        s.ok(alice.conn, "scheduleGame", schedule()).await;
        s.ok(alice.conn, "scheduleGame", schedule()).await;
        assert_eq!(s.err(alice.conn, "scheduleGame", schedule()).await, error::TOO_MANY_SCHEDULED_GAMES);

        let subscribed = json!({"userId": bobby.id, "userName": "bobby", "subscription": "CertainAsPlayer"});
        let status = s.ok(bobby.conn, "subscribe", json!({"scheduledGame": id, "subscription": "CertainAsPlayer"})).await;
        let g = status["scheduledGames"].as_array().unwrap().iter().find(|g| g["scheduledGameId"] == id.as_str()).unwrap().clone();
        assert!(g["subscribers"].as_array().unwrap().contains(&subscribed), "{g}");
        assert_eq!(s.err(bobby.conn, "subscribe", json!({"scheduledGame": "nope", "subscription": "CertainAsPlayer"})).await, error::GAME_NOT_FOUND);
        assert_eq!(s.err(bobby.conn, "cancelScheduledGame", json!({"scheduledGame": id})).await, error::NO_HOST);

        // Scheduled games survive a restart, subscriptions included.
        let mut s = s.restart().await;
        let alice = s.login("alice").await;
        let lobby = s.lobby(&alice, "Active").await;
        let g = lobby["scheduledGames"].as_array().unwrap().iter().find(|g| g["scheduledGameId"] == id.as_str()).expect("still scheduled").clone();
        assert!(g["subscribers"].as_array().unwrap().contains(&subscribed), "{g}");

        let status = s.ok(alice.conn, "cancelScheduledGame", json!({"scheduledGame": id})).await;
        assert_eq!(status["scheduledGames"].as_array().unwrap().len(), 2);

        // Games more than four hours in the past are dropped.
        s.advance_minutes(60 * 24 * 13).await;
        let lobby = s.lobby(&alice, "Active").await;
        assert_eq!(lobby["scheduledGames"].as_array().unwrap().len(), 0);
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
        s.ok(bobby.conn, "joinGame", json!({"game": game, "passwordHash": "", "seat": null})).await;
        s.ok(carol.conn, "joinGame", json!({"game": game, "passwordHash": "", "seat": null})).await;
        s.ok(watcher.conn, "observeGame", json!({"game": game, "passwordHash": ""})).await;
        for c in [alice.conn, bobby.conn, carol.conn, watcher.conn] {
            s.received(c);
        }

        let message = json!({"game": game, "to": null, "body": "hello"});
        assert_eq!(s.err(anonymous, "chat", message.clone()).await, error::NOT_LOGGED_IN);
        // Outsiders and observers can't chat in the game.
        assert_eq!(s.err(outsider.conn, "chat", message.clone()).await, error::USER_NOT_IN_GAME);
        assert_eq!(s.err(watcher.conn, "chat", message.clone()).await, error::USER_NOT_IN_GAME);
        s.ok(alice.conn, "chat", message.clone()).await;
        for c in [alice.conn, bobby.conn, carol.conn, watcher.conn] {
            assert_eq!(find(&s.received(c), "chatMessage").expect("chatMessage"), &json!({"from": alice.id, "to": null, "body": "hello"}));
        }
        assert!(find(&s.received(outsider.conn), "chatMessage").is_none());

        // The sender is always the caller: a message has no sender of its own to forge.
        s.ok(bobby.conn, "chat", json!({"game": game, "to": null, "body": "from alice, honest", "from": alice.id})).await;
        assert_eq!(find(&s.received(carol.conn), "chatMessage").unwrap()["from"], bobby.id);
        for c in [alice.conn, bobby.conn, watcher.conn] {
            s.received(c);
        }

        // A message to one user reaches only them and the sender.
        s.ok(alice.conn, "chat", json!({"game": game, "to": bobby.id, "body": "psst"})).await;
        assert_eq!(find(&s.received(bobby.conn), "chatMessage").expect("to Bobby"), &json!({"from": alice.id, "to": bobby.id, "body": "psst"}));
        assert!(find(&s.received(alice.conn), "chatMessage").is_some(), "the sender sees it");
        assert!(find(&s.received(carol.conn), "chatMessage").is_none());
        assert!(find(&s.received(watcher.conn), "chatMessage").is_none());
        assert_eq!(s.err(alice.conn, "chat", json!({"game": game, "to": outsider.id, "body": "hi"})).await, error::USER_NOT_IN_GAME);

        let global = json!({"body": "hi all"});
        assert_eq!(s.err(anonymous, "globalChat", global.clone()).await, error::NOT_LOGGED_IN);
        s.ok(bobby.conn, "globalChat", global.clone()).await;
        for c in [alice.conn, bobby.conn, outsider.conn, anonymous] {
            assert_eq!(find(&s.received(c), "globalChatMessage").expect("globalChatMessage"), &json!({"from": bobby.id, "body": "hi all"}));
        }
        s.ok(outsider.conn, "globalChat", global).await;
        assert_eq!(find(&s.received(alice.conn), "globalChatMessage").unwrap()["from"], outsider.id);
    });
}

#[test]
fn the_protocol_envelope() {
    rt().block_on(async {
        let mut s = TestServer::start().await;
        let c = s.connect();

        let info = s.ok(c, "serverInfo", Value::Null).await;
        assert_eq!(info["adminName"], "admin");

        // A call the server doesn't know, or with arguments that don't decode, is a badCall.
        // Call names are matched exactly.
        for (call, args) in [
            ("requestFoo", Value::Null),
            ("ServerInfo", Value::Null),
            ("authenticate", Value::Null),
            ("authenticate", json!({"token": 12})),
            ("joinGame", json!({"game": "g", "seat": "three"})),
            // A seat is a number from 0 to 255. -1, for no seat, only pairs a user with a seat.
            ("joinGame", json!({"game": "g", "seat": -1})),
            ("openOrCloseSeat", json!({"game": "g", "seat": 256})),
            ("assignSeats", json!({"game": "g", "seats": [[1, -2]]})),
        ] {
            let (reply, _) = s.call_raw(c, call, args.clone()).await;
            assert_eq!(code(&reply), "badCall", "{call} {args}: {reply}");
            assert!(!reply["err"]["message"].as_str().unwrap().is_empty());
        }
        assert!(s.is_open(c));

        // A message that isn't an object with an id closes the connection.
        s.send_text(c, "[1,2]").await;
        assert!(!s.is_open(c));
        let d = s.connect();
        s.send_text(d, r#"{"call":"serverInfo"}"#).await;
        assert!(!s.is_open(d), "a call needs an id");
        let e = s.connect();
        s.send_text(e, r#"{"type":"ping"}"#).await;
        assert!(!s.is_open(e), "the old envelope is gone");
    });
}
