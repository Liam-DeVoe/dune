//! Playing: events, game views and the view calls, undo, pauses between phases, bots,
//! async-play mail.

mod common;

use std::time::Duration;

use common::*;
use serde_json::{json, Value};

mod error {
    pub const NOT_LOGGED_IN: &str = "notLoggedIn";
    pub const USER_NOT_IN_GAME: &str = "userNotInGame";
    pub const GAME_NOT_FOUND: &str = "gameNotFound";
    pub const NO_HOST: &str = "noHost";
    pub const INVALID_GAME_EVENT: &str = "invalidGameEvent";
}

fn ally(view: &Value, faction: &str) -> String {
    player(view, faction)["ally"].as_str().unwrap().to_string()
}

fn event_number(view: &Value) -> i64 {
    view["eventNumber"].as_i64().unwrap()
}

/// Alice hosts the alliance fixture as the Emperor, Bobby plays the Atreides.
async fn alliance_game(s: &mut TestServer) -> (User, User, String) {
    let alice = s.user("alice").await;
    let bobby = s.user("bobby").await;
    let game = s.fixture_game(&alice, "alliance").await;
    let (emperor, atreides) = alliance_seats();
    s.ok(alice.conn, "assignSeats", json!({"game": game, "seats": [[alice.id, emperor]]})).await;
    s.ok(alice.conn, "openOrCloseSeat", json!({"game": game, "seat": atreides})).await;
    s.ok(bobby.conn, "joinGame", json!({"game": game, "passwordHash": "", "seat": atreides})).await;
    (alice, bobby, game)
}

#[test]
fn two_players_make_an_alliance() {
    rt().block_on(async {
        let mut s = TestServer::start().await;
        let (alice, bobby, game) = alliance_game(&mut s).await;
        let alice_view = s.latest_view(alice.conn).unwrap();
        assert_eq!(alice_view["viewer"]["faction"], "Emperor");
        assert!(actions(&alice_view).contains(&"AllianceOffered".to_string()));
        let start = event_number(&alice_view);
        s.received(bobby.conn);

        // Drafts are checked against the caller's actions.
        let draft = s.ok(alice.conn, "queryDraft", json!({"game": game, "eventNumber": start, "draft": offer(faction::EMPEROR)})).await;
        assert!(!draft["validation"].is_null(), "offering to oneself is invalid: {draft}");
        let draft = s.ok(alice.conn, "queryDraft", json!({"game": game, "eventNumber": start, "draft": offer(faction::ATREIDES)})).await;
        assert!(draft["validation"].is_null(), "{draft}");
        assert_eq!(draft["eventType"], "AllianceOffered");
        let r = s.call(alice.conn, "queryDraft", json!({"game": game, "eventNumber": start, "draft": {"type": "StormDialled", "amount": 1}})).await;
        assert_eq!(code(&r), error::INVALID_GAME_EVENT, "not an action of this view");

        // Each event sends every member their own view, the submitter's before its reply.
        let (reply, before) = s.call_raw(alice.conn, "submitEvent", json!({"game": game, "event": offer(faction::ATREIDES)})).await;
        assert!(is_ok(&reply), "{reply}");
        assert_eq!(event_number(&views(&before).pop().unwrap()), start + 1);
        assert_eq!(views(&s.received(bobby.conn)).len(), 1);
        s.ok(bobby.conn, "submitEvent", json!({"game": game, "event": offer(faction::EMPEROR)})).await;
        let bobby_view = s.latest_view(bobby.conn).unwrap();
        let alice_view = s.latest_view(alice.conn).unwrap();
        assert_eq!(ally(&bobby_view, "Atreides"), "Emperor");
        assert_eq!(ally(&alice_view, "Emperor"), "Atreides");
        assert_eq!(event_number(&alice_view), event_number(&bobby_view));
        assert_eq!(bobby_view["me"]["faction"], "Atreides");

        // An invalid event is refused with the validation message and changes nothing.
        let (reply, before) = s.call_raw(bobby.conn, "submitEvent", json!({"game": game, "event": offer(faction::EMPEROR)})).await;
        assert_eq!(code(&reply), error::INVALID_GAME_EVENT);
        assert!(!reply["err"]["message"].as_str().unwrap().is_empty());
        assert!(views(&before).is_empty());
        assert!(views(&s.received(alice.conn)).is_empty());

        let anonymous = s.connect();
        assert_eq!(s.err(anonymous, "submitEvent", json!({"game": game, "event": offer(faction::EMPEROR)})).await, error::NOT_LOGGED_IN);
        assert_eq!(s.err(bobby.conn, "submitEvent", json!({"game": "no game", "event": offer(faction::EMPEROR)})).await, error::GAME_NOT_FOUND);
        let (reply, _) = s.call_raw(bobby.conn, "submitEvent", json!({"game": game, "event": {"type": "NoSuchEvent"}})).await;
        assert_eq!(code(&reply), error::INVALID_GAME_EVENT, "{reply}");

        // Reports, filtered for the caller.
        let reports = alice_view["log"]["reports"].as_array().unwrap().len() as i64;
        let lines = s.ok(bobby.conn, "report", json!({"game": game, "index": reports - 1})).await;
        assert_eq!(lines["index"], reports - 1);
        assert_eq!(s.err(bobby.conn, "report", json!({"game": game, "index": reports})).await, error::INVALID_GAME_EVENT);

        // Saving is for hosts: the live game as a savegame.
        assert_eq!(s.err(bobby.conn, "saveGame", json!({"game": game})).await, error::NO_HOST);
        let saved = s.ok(alice.conn, "saveGame", json!({"game": game})).await;
        let saved: Value = serde_json::from_str(saved.as_str().unwrap()).unwrap();
        assert_eq!(saved["events"].as_array().unwrap().len() as i64, event_number(&alice_view));
    });
}

#[test]
fn undo_restore_and_dismiss() {
    rt().block_on(async {
        let mut s = TestServer::start().await;
        let (alice, bobby, game) = alliance_game(&mut s).await;
        s.ok(alice.conn, "submitEvent", json!({"game": game, "event": offer(faction::ATREIDES)})).await;
        s.ok(bobby.conn, "submitEvent", json!({"game": game, "event": offer(faction::EMPEROR)})).await;
        let count = event_number(&s.latest_view(alice.conn).unwrap());
        s.received(bobby.conn);

        // The undo list and preview are for hosts.
        assert_eq!(s.err(bobby.conn, "undoList", json!({"game": game})).await, error::NO_HOST);
        let list = s.ok(alice.conn, "undoList", json!({"game": game})).await;
        let list = list.as_array().unwrap();
        assert_eq!(list.len() as i64, count);
        assert_eq!(list.last().unwrap()["eventNumber"], count - 1);
        assert_eq!(s.err(bobby.conn, "undoPreview", json!({"game": game, "gap": count - 2})).await, error::NO_HOST);
        let (reply, before) = s.call_raw(alice.conn, "undoPreview", json!({"game": game, "gap": count - 2})).await;
        assert!(views(&before).is_empty(), "a preview sends no views");
        let preview = reply["ok"].clone();
        assert_eq!(event_number(&preview), count - 2);
        assert_eq!(ally(&preview, "Emperor"), "None");
        assert_eq!(preview["actions"], json!([]));
        let now = s.ok(alice.conn, "undoPreview", json!({"game": game, "gap": count + 5})).await;
        assert_eq!(event_number(&now), count, "a gap past the end is the live game");

        assert_eq!(s.err(bobby.conn, "undo", json!({"game": game, "until": count - 2})).await, error::NO_HOST);
        s.ok(alice.conn, "undo", json!({"game": game, "until": count - 2})).await;
        let undone = s.latest_view(alice.conn).unwrap();
        assert_eq!(event_number(&undone), count - 2);
        assert_eq!(ally(&undone, "Emperor"), "None");
        let strip = |v: &Value| {
            let mut v = v.clone();
            v["actions"] = json!([]);
            if let Some(hand) = v["me"]["hand"].as_array_mut() {
                hand.iter_mut().for_each(|c| c["playableWith"] = json!([]));
            }
            v
        };
        assert_eq!(strip(&undone), strip(&preview), "the preview showed what the undo gives");
        assert_eq!(event_number(&s.latest_view(bobby.conn).unwrap()), count - 2);

        // Restore brings the undone events back.
        assert_eq!(s.err(bobby.conn, "restoreUndone", json!({"game": game})).await, error::NO_HOST);
        s.ok(alice.conn, "restoreUndone", json!({"game": game})).await;
        let restored = s.latest_view(bobby.conn).unwrap();
        assert_eq!(event_number(&restored), count);
        assert_eq!(ally(&restored, "Atreides"), "Emperor");

        // Undo again and dismiss: the events stay gone.
        s.ok(alice.conn, "undo", json!({"game": game, "until": count - 1})).await;
        assert_eq!(s.err(bobby.conn, "dismissUndone", json!({"game": game})).await, error::NO_HOST);
        s.ok(alice.conn, "dismissUndone", json!({"game": game})).await;
        assert_eq!(event_number(&s.latest_view(alice.conn).unwrap()), count - 1);
        s.ok(alice.conn, "restoreUndone", json!({"game": game})).await;
        assert_eq!(event_number(&s.latest_view(alice.conn).unwrap()), count - 1);
    });
}

/// Undo keeps the game it replaces: each restore brings back exactly the views from before
/// the matching undo, also after events were played in between, and two undos restore in
/// order.
#[test]
fn restore_brings_back_the_views_from_before_the_undo() {
    rt().block_on(async {
        let mut s = TestServer::start().await;
        let (alice, bobby, game) = alliance_game(&mut s).await;
        s.ok(alice.conn, "submitEvent", json!({"game": game, "event": offer(faction::ATREIDES)})).await;
        let after_offer = s.latest_view(bobby.conn).unwrap();
        s.ok(bobby.conn, "submitEvent", json!({"game": game, "event": offer(faction::EMPEROR)})).await;
        let allied = s.latest_view(bobby.conn).unwrap();
        let count = event_number(&allied);
        assert_eq!(ally(&allied, "Atreides"), "Emperor");

        // A negative event number is refused and changes nothing.
        assert_eq!(s.err(alice.conn, "undo", json!({"game": game, "until": -1})).await, error::INVALID_GAME_EVENT);
        assert!(views(&s.received(bobby.conn)).is_empty());

        // Undo both offers, play a different one, then restore: the alliance is back.
        s.ok(alice.conn, "undo", json!({"game": game, "until": count - 2})).await;
        assert_eq!(event_number(&s.latest_view(bobby.conn).unwrap()), count - 2);
        s.ok(bobby.conn, "submitEvent", json!({"game": game, "event": offer(faction::EMPEROR)})).await;
        assert_eq!(event_number(&s.latest_view(bobby.conn).unwrap()), count - 1);
        s.ok(alice.conn, "restoreUndone", json!({"game": game})).await;
        assert_eq!(s.latest_view(bobby.conn).unwrap(), allied);

        // Two undos in a row restore in order.
        s.ok(alice.conn, "undo", json!({"game": game, "until": count - 1})).await;
        s.ok(alice.conn, "undo", json!({"game": game, "until": count - 2})).await;
        assert_eq!(event_number(&s.latest_view(bobby.conn).unwrap()), count - 2);
        s.ok(alice.conn, "restoreUndone", json!({"game": game})).await;
        let first = s.latest_view(bobby.conn).unwrap();
        assert_eq!(event_number(&first), count - 1);
        assert_eq!(first["log"], after_offer["log"]);
        s.ok(alice.conn, "restoreUndone", json!({"game": game})).await;
        assert_eq!(s.latest_view(bobby.conn).unwrap(), allied);

        // The restored game plays on.
        s.ok(alice.conn, "undo", json!({"game": game, "until": count - 1})).await;
        s.ok(alice.conn, "dismissUndone", json!({"game": game})).await;
        s.ok(bobby.conn, "submitEvent", json!({"game": game, "event": offer(faction::EMPEROR)})).await;
        let replayed = s.latest_view(bobby.conn).unwrap();
        assert_eq!(event_number(&replayed), count);
        assert_eq!(ally(&replayed, "Atreides"), "Emperor");
    });
}

/// Alice creates a game, Bobby joins, and Alice starts it for two players.
async fn started_game(s: &mut TestServer, async_play: bool, interval: i64) -> (User, User, String) {
    let alice = s.user("alice").await;
    let bobby = s.user("bobby").await;
    let game = s.new_game(&alice, "Dune", "").await;
    s.ok(bobby.conn, "joinGame", json!({"game": game, "passwordHash": "", "seat": null})).await;
    let settings = establish_players(2, &[], async_play, interval)["settings"].clone();
    s.ok(alice.conn, "updateSettings", json!({"game": game, "settings": settings})).await;
    s.received(alice.conn);
    s.received(bobby.conn);
    s.ok(alice.conn, "submitEvent", json!({"game": game, "event": establish_players(2, &[], async_play, interval)})).await;
    (alice, bobby, game)
}

#[test]
fn starting_a_game_seats_the_players() {
    rt().block_on(async {
        let mut s = TestServer::start().await;
        let (alice, bobby, game) = started_game(&mut s, false, 0).await;
        let received = s.received(bobby.conn);
        let seats = find(&received, "assignSeats").expect("assignSeats")["seats"].clone();
        let seated: Vec<i64> = seats.as_array().unwrap().iter().map(|p| p[0].as_i64().unwrap()).collect();
        assert_eq!(seated, vec![alice.id, bobby.id]);
        let view = views(&received).pop().unwrap();
        assert_ne!(view["phase"], "AwaitingPlayers");
        assert!(!view["me"].is_null(), "Bobby has a faction");
        assert_eq!(view["players"].as_array().unwrap().len(), 2);
        assert!(view["players"].as_array().unwrap().iter().all(|p| p["isBot"] == false));

        let lobby = s.lobby(&alice, "Active").await;
        let g = lobby["runningGames"].as_array().unwrap().iter().find(|g| g["gameId"] == game.as_str()).unwrap().clone();
        assert_eq!(g["players"], 2);
        assert_eq!(g["bots"], 0);
        assert_eq!(g["factionsInPlay"].as_array().unwrap().len(), 2);
    });
}

#[test]
fn starting_with_invalid_settings_is_refused() {
    rt().block_on(async {
        let mut s = TestServer::start().await;
        let alice = s.user("alice").await;
        let bobby = s.user("bobby").await;
        let game = s.new_game(&alice, "Dune", "").await;
        s.ok(bobby.conn, "joinGame", json!({"game": game, "passwordHash": "", "seat": null})).await;
        s.received(bobby.conn);

        // One seat for two players: the start is refused with the reason, and nothing changes.
        let r = s.call(alice.conn, "submitEvent", json!({"game": game, "event": establish_players(1, &[], false, 0)})).await;
        assert_eq!(code(&r), error::INVALID_GAME_EVENT, "{r}");
        assert!(r.to_string().contains("At least two players required"), "{r}");
        let received = s.received(bobby.conn);
        assert!(find(&received, "assignSeats").is_none(), "{received:?}");
        assert!(views(&received).is_empty(), "the game didn't change");

        // A valid start still works.
        s.ok(alice.conn, "submitEvent", json!({"game": game, "event": establish_players(2, &[], false, 0)})).await;
        assert!(find(&s.received(bobby.conn), "assignSeats").is_some());
    });
}

/// The engine doesn't know who waits for a seat: the server refuses a start with fewer seats
/// than users waiting.
#[test]
fn starting_with_more_users_than_seats_is_refused() {
    rt().block_on(async {
        let mut s = TestServer::start().await;
        let alice = s.user("alice").await;
        let bobby = s.user("bobby").await;
        let carol = s.user("carol").await;
        let game = s.new_game(&alice, "Dune", "").await;
        s.ok(bobby.conn, "joinGame", json!({"game": game, "passwordHash": "", "seat": null})).await;
        s.ok(carol.conn, "joinGame", json!({"game": game, "passwordHash": "", "seat": null})).await;
        s.received(bobby.conn);

        let r = s.call(alice.conn, "submitEvent", json!({"game": game, "event": establish_players(2, &[], false, 0)})).await;
        assert_eq!(code(&r), error::INVALID_GAME_EVENT, "{r}");
        assert!(r.to_string().contains("More players than seats at the table"), "{r}");
        assert!(views(&s.received(bobby.conn)).is_empty(), "the game didn't change");

        s.ok(alice.conn, "submitEvent", json!({"game": game, "event": establish_players(3, &[], false, 0)})).await;
        assert!(find(&s.received(bobby.conn), "assignSeats").is_some());
    });
}

#[test]
fn pauses_continue_when_every_player_is_done() {
    rt().block_on(async {
        let mut s = TestServer::start().await;
        let (alice, bobby, game) = started_game(&mut s, false, 0).await;
        let received = s.received(bobby.conn);
        let control = find(&received, "phaseControl").expect("the pause waits for the players").clone();
        assert_eq!(control["active"], true);
        assert_eq!(control["waitingFor"].as_array().unwrap().len(), 2);
        let start = event_number(&views(&received).pop().unwrap());

        let outsider = s.user("carol").await;
        let r = s.call(outsider.conn, "phaseDone", json!({"game": game})).await;
        assert_eq!(code(&r), error::USER_NOT_IN_GAME, "only players");

        s.ok(alice.conn, "phaseDone", json!({"game": game})).await;
        let received = s.received(bobby.conn);
        let control = find(&received, "phaseControl").expect("one fewer to wait for").clone();
        assert_eq!(control["waitingFor"].as_array().unwrap().len(), 1);
        assert!(views(&received).is_empty());

        s.ok(bobby.conn, "phaseDone", json!({"game": game})).await;
        let view = s.latest_view(alice.conn).expect("the server ended the pause");
        assert_eq!(event_number(&view), start + 1);

        // The host's Continue only applies to the moment it was given for.
        let now = event_number(&view);
        assert_eq!(s.err(bobby.conn, "phaseContinue", json!({"game": game, "eventNumber": now})).await, error::NO_HOST);
        s.ok(alice.conn, "phaseContinue", json!({"game": game, "eventNumber": now - 1})).await;
        assert!(s.latest_view(alice.conn).is_none(), "a stale Continue does nothing");
    });
}

#[test]
fn bots_take_the_empty_seats() {
    rt().block_on(async {
        let mut s = TestServer::start().await;
        let alice = s.user("alice").await;
        let watcher = s.connect();
        let game = s.new_game(&alice, "", "").await;
        let settings = establish_players(3, &[rule::FILL_WITH_BOTS], false, 0)["settings"].clone();
        s.ok(alice.conn, "updateSettings", json!({"game": game, "settings": settings})).await;
        s.ok(alice.conn, "submitEvent", json!({"game": game, "event": establish_players(3, &[rule::FILL_WITH_BOTS], false, 0)})).await;
        let view = s.latest_view(alice.conn).unwrap();
        let bots = view["players"].as_array().unwrap().iter().filter(|p| p["isBot"] == true).count();
        assert_eq!(bots, 2);
        let lobby = s.lobby(&alice, "Active").await;
        assert_eq!(lobby["runningGames"][0]["bots"], 2);

        // Speed 0 stops the bots; the game's members hear of a speed change, other connections don't.
        s.received(alice.conn);
        s.ok(alice.conn, "setBotSpeed", json!({"game": game, "speed": 0})).await;
        assert_eq!(find(&s.received(alice.conn), "botSpeed").expect("botSpeed"), &json!({"speed": 0}));
        assert!(find(&s.received(watcher), "botSpeed").is_none(), "a connection outside the game");
        s.ok(alice.conn, "phaseDone", json!({"game": game})).await;
        s.advance(Duration::from_secs(60)).await;
        let stopped = event_number(&s.latest_view(alice.conn).unwrap());
        s.advance(Duration::from_secs(120)).await;
        assert!(s.latest_view(alice.conn).is_none(), "nothing happens while the bots are stopped");

        // At speed 1 they act on their own after a delay.
        s.ok(alice.conn, "setBotSpeed", json!({"game": game, "speed": 1})).await;
        s.advance(Duration::from_secs(60)).await;
        let view = s.latest_view(alice.conn).expect("the bots acted");
        assert!(event_number(&view) > stopped);

        // The game's players may nudge them; nobody else.
        s.ok(alice.conn, "nudgeBots", json!({"game": game})).await;
        let carol = s.user("carol").await;
        assert_eq!(s.err(carol.conn, "nudgeBots", json!({"game": game})).await, error::USER_NOT_IN_GAME);
        assert_eq!(s.err(carol.conn, "nudgeBots", json!({"game": "no game"})).await, error::GAME_NOT_FOUND);
        assert_eq!(s.err(carol.conn, "setBotSpeed", json!({"game": game, "speed": 2})).await, error::NO_HOST);
        // Speeds are 0 to 4.
        assert_eq!(s.err(alice.conn, "setBotSpeed", json!({"game": game, "speed": 1000})).await, error::INVALID_GAME_EVENT);
        assert_eq!(s.err(alice.conn, "setBotSpeed", json!({"game": game, "speed": -1})).await, error::INVALID_GAME_EVENT);
    });
}

#[test]
fn async_play_mails_the_players() {
    rt().block_on(async {
        let mut s = TestServer::start().await;
        let (alice, bobby, game) = started_game(&mut s, true, 10).await;
        // Starting the game mails every seated player.
        let mails = s.mails();
        let mut to: Vec<String> = mails.iter().flat_map(|m| m.to.clone()).collect();
        to.sort();
        assert_eq!(to, vec!["alice@example.com", "bobby@example.com"]);
        assert!(mails[0].body.contains("The game was just started"), "{}", mails[0].body);

        // Events within the interval are mailed to every seated player with an address, once it
        // has passed.
        s.ok(bobby.conn, "updateUserInfo", json!({"passwordHash": "", "playerName": "Bobby Player", "email": ""})).await;
        s.advance_minutes(1).await;
        s.ok(bobby.conn, "phaseDone", json!({"game": game})).await;
        s.ok(alice.conn, "phaseDone", json!({"game": game})).await;
        assert!(s.mails().is_empty());
        s.advance_minutes(11).await;
        let mails = s.mails();
        assert_eq!(mails.len(), 1, "{mails:?}");
        let mail = &mails[0];
        assert_eq!(mail.to, vec!["alice@example.com"]);
        assert_eq!(mail.subject, "Update for Dune");
        // Who is waited for depends on the factions the game dealt.
        assert!(mail.body.contains("Waiting for: ") && mail.body.contains("Bobby Player"), "{}", mail.body);
        assert!(!mail.body.contains(&alice.token), "no session token in the mail");

        // The link logs Alice in once and opens the game, without her session token.
        let token = link_token(&mail.body);
        let c = s.connect();
        let (reply, _) = s.call_raw(c, "linkLogin", json!({"token": token})).await;
        let info = &reply["ok"];
        assert_eq!(info["gameId"], game.as_str());
        assert_eq!(info["session"]["account"]["userId"], alice.id);
        let session = info["session"]["token"].as_str().unwrap().to_string();
        assert_ne!(session, alice.token, "a new session");
        s.ok(c, "reconnectGame", json!({"game": game})).await;
        let d = s.connect();
        assert_eq!(s.err(d, "linkLogin", json!({"token": token})).await, error::NOT_LOGGED_IN, "a link works once");
        s.ok(d, "authenticate", json!({"token": session})).await;
    });
}

/// The token of the login link in an async-play mail.
fn link_token(body: &str) -> String {
    let marker = "/link/";
    let start = body.find(marker).unwrap_or_else(|| panic!("no link in {body}")) + marker.len();
    body[start..start + body[start..].find('"').unwrap()].to_string()
}

#[test]
fn login_links_expire_after_a_day_and_survive_a_restart() {
    rt().block_on(async {
        let mut s = TestServer::start().await;
        started_game(&mut s, true, 10).await;
        let links: Vec<String> = s.mails().iter().map(|m| link_token(&m.body)).collect();
        assert_eq!(links.len(), 2);
        assert_ne!(links[0], links[1]);
        let mut s = s.restart().await;
        s.advance_minutes(60 * 24 - 1).await;
        let c = s.connect();
        s.ok(c, "linkLogin", json!({"token": links[0]})).await;
        s.advance_minutes(2).await;
        let d = s.connect();
        assert_eq!(s.err(d, "linkLogin", json!({"token": links[1]})).await, error::NOT_LOGGED_IN);
    });
}

#[test]
fn games_without_async_play_send_no_mail() {
    rt().block_on(async {
        let mut s = TestServer::start().await;
        started_game(&mut s, false, 0).await;
        s.advance_minutes(30).await;
        assert!(s.mails().is_empty());
    });
}
