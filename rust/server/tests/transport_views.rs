//! The transport and the game-view layer together, through `runtime::receive`: the same path
//! a WebSocket message takes, without the socket. Two users load the alliance fixture, take
//! seats and play alliance events; each must get their own view after each change, before the
//! reply, and the view calls must answer from the replica.

use std::rc::Rc;

use serde_json::{json, Value};
use tokio::sync::mpsc;
use treachery_server::clock::SystemClock;
use treachery_server::config::Config;
use treachery_server::db::Db;
use treachery_server::hub::{GameHub, SequentialIds, Services};
use treachery_server::mail::RecordingMailer;
use treachery_server::runtime::{self, Outgoing};
use treachery_server::timers::RecordingTimers;
use treachery_server::views::GameViews;

struct Server {
    hub: GameHub,
    views: GameViews,
    outgoing: Outgoing,
    queues: Vec<mpsc::UnboundedReceiver<String>>,
    next_id: u64,
}

const FACTION_EMPEROR: i32 = 40;
const FACTION_ATREIDES: i32 = 20;

impl Server {
    async fn new() -> Server {
        let services = Services {
            db: Db::open_in_memory().await.unwrap(),
            config: Config::default(),
            clock: Rc::new(SystemClock),
            timers: Rc::new(RecordingTimers::default()),
            mailer: Rc::new(RecordingMailer::default()),
            ids: Rc::new(SequentialIds::default()),
        };
        let mut s = Server { hub: GameHub::new(services), views: GameViews::new(), outgoing: Outgoing::new(), queues: vec![], next_id: 0 };
        for connection in 0..2u64 {
            let (tx, rx) = mpsc::unbounded_channel();
            runtime::connect(&mut s.hub, &s.views, &mut s.outgoing, connection, tx);
            s.queues.push(rx);
        }
        s
    }

    fn drain(&mut self, connection: usize) -> Vec<Value> {
        let mut out = vec![];
        while let Ok(text) = self.queues[connection].try_recv() {
            out.push(serde_json::from_str(&text).unwrap_or_else(|e| panic!("{e}: {text}")));
        }
        out
    }

    /// Sends a call and returns everything the caller received, the reply last.
    async fn call(&mut self, connection: usize, method: &str, args: Value) -> Vec<Value> {
        self.next_id += 1;
        let text = json!({"type": "call", "id": self.next_id, "method": method, "args": args}).to_string();
        runtime::receive(&mut self.hub, &mut self.views, &mut self.outgoing, connection as u64, &text).await;
        let received = self.drain(connection);
        let reply = received.last().unwrap_or_else(|| panic!("{method}: no reply"));
        assert_eq!(reply["type"], "reply", "{method}: the reply comes last: {received:?}");
        assert_eq!(reply["id"], self.next_id);
        received
    }

    /// A call that must succeed: its `Contents`, and the notifications before the reply.
    async fn ok(&mut self, connection: usize, method: &str, args: Value) -> (Value, Vec<Value>) {
        let mut received = self.call(connection, method, args).await;
        let reply = received.pop().unwrap();
        assert_eq!(reply["result"]["Success"], true, "{method}: {reply}");
        (reply["result"]["Contents"].clone(), received)
    }
}

fn views(notifications: &[Value]) -> Vec<&Value> {
    notifications.iter().filter(|n| n["name"] == "GameView").map(|n| &n["args"][0]).collect()
}

fn names(notifications: &[Value]) -> Vec<String> {
    notifications.iter().map(|n| n["name"].as_str().unwrap_or_default().to_string()).collect()
}

fn fixture() -> String {
    std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../scripts/fixtures/alliance.json")).unwrap()
}

fn ally(view: &Value, faction: &str) -> String {
    view["players"].as_array().unwrap().iter().find(|p| p["faction"] == faction).unwrap()["ally"].as_str().unwrap().to_string()
}

#[tokio::test(flavor = "current_thread")]
async fn two_players_play_alliance_events() {
    let mut s = Server::new().await;
    let (alice, _) = s.ok(0, "RequestCreateUser", json!(["alice", "h1", null, "Alice"])).await;
    let (bob, _) = s.ok(1, "RequestCreateUser", json!(["bobby", "h2", null, "Bobby"])).await;
    let (a, b) = (alice["Token"].as_str().unwrap().to_string(), bob["Token"].as_str().unwrap().to_string());
    let (a_id, b_id) = (alice["UserId"].as_i64().unwrap(), bob["UserId"].as_i64().unwrap());

    // The creator gets a view of the loaded game; GameInitInfo comes without the savegame.
    let (info, before) = s.ok(0, "RequestCreateGame", json!(["Test", a, "", fixture(), ""])).await;
    assert!(info.get("GameState").is_none(), "{info}");
    let game = info["GameId"].as_str().unwrap().to_string();
    let v = views(&before);
    assert_eq!(v.len(), 1, "{:?}", names(&before));
    assert_eq!(v[0]["phase"], "AllianceA");
    assert_eq!(v[0]["viewer"]["isHost"], true);

    // Seats: Alice takes the Emperor's (0) by assignment, Bob joins the opened Atreides seat (3).
    let seats = json!([[a_id, 0]]);
    let (_, n) = s.ok(0, "RequestAssignSeats", json!([a, game, seats])).await;
    assert_eq!(names(&n), ["AssignSeats", "GameView"]);
    assert_eq!(views(&n)[0]["viewer"]["faction"], "Emperor");
    let (_, n) = s.ok(0, "RequestOpenOrCloseSeat", json!([a, game, 3])).await;
    assert_eq!(names(&n), ["OpenOrCloseSeats", "GameView"]);
    let (_, n) = s.ok(1, "RequestJoinGame", json!([b, game, "", 3])).await;
    let bob_view = views(&n);
    assert_eq!(bob_view.len(), 1, "one view for the joining caller: {:?}", names(&n));
    assert_eq!(bob_view[0]["viewer"]["faction"], "Atreides");
    assert!(bob_view[0]["me"]["faction"] == "Atreides");
    let alice_got = s.drain(0);
    assert_eq!(views(&alice_got)[0]["viewer"]["faction"], "Emperor", "Alice sees Bob join: {:?}", names(&alice_got));
    assert!(views(&alice_got)[0]["players"].as_array().unwrap().iter().any(|p| p["faction"] == "Atreides" && p["isBot"] == false));

    // A draft query is answered from the replica: offering to oneself is invalid, to Bob valid.
    let (draft, _) = s
        .ok(0, "QueryDraft", json!([a, game, 0, {"$type": "AllianceOffered", "Target": FACTION_EMPEROR, "Initiator": 0}]))
        .await;
    assert!(!draft["validation"].is_null(), "{draft}");
    let (draft, _) = s
        .ok(0, "querydraft", json!([a, game, 0, {"$type": "AllianceOffered", "Target": FACTION_ATREIDES, "Initiator": 0}]))
        .await;
    assert!(draft["validation"].is_null(), "{draft}");
    assert_eq!(draft["eventType"], "AllianceOffered");
    let r = s.call(0, "QueryDraft", json!([a, game, 0, {"$type": "StormDialled", "Amount": 1}])).await;
    assert_eq!(r.last().unwrap()["result"]["Error"], 23, "not an action of this view");

    // Alice offers, Bob offers back: an alliance. Both see it.
    let (_, n) = s
        .ok(0, "SubmitEvent", json!([a, game, {"$type": "AllianceOffered", "Target": FACTION_ATREIDES, "Initiator": FACTION_EMPEROR}]))
        .await;
    assert!(names(&n).contains(&"GameView".to_string()), "{:?}", names(&n));
    let bob_got = s.drain(1);
    assert_eq!(views(&bob_got).len(), 1, "{:?}", names(&bob_got));
    let (_, n) = s
        .ok(1, "SubmitEvent", json!([b, game, {"$type": "AllianceOffered", "Target": FACTION_EMPEROR, "Initiator": FACTION_ATREIDES}]))
        .await;
    let bob_view = views(&n).last().unwrap().to_owned().clone();
    assert_eq!(ally(&bob_view, "Atreides"), "Emperor", "{}", bob_view["players"]);
    let alice_view = views(&s.drain(0)).last().unwrap().to_owned().clone();
    assert_eq!(ally(&alice_view, "Emperor"), "Atreides");
    assert_eq!(alice_view["eventNumber"], bob_view["eventNumber"]);

    // Invalid events are refused with the rendered validation message, and send no view.
    let r = s.call(1, "SubmitEvent", json!([b, game, {"$type": "AllianceOffered", "Target": FACTION_EMPEROR, "Initiator": FACTION_ATREIDES}])).await;
    assert_eq!(r.len(), 1, "{r:?}");
    assert_eq!(r[0]["result"]["Error"], 23);

    // The undo list is for hosts; undo rebuilds the replica and sends both a view without
    // the alliance.
    let r = s.call(1, "RequestUndoList", json!([b, game])).await;
    assert_eq!(r[0]["result"]["Error"], 22, "NoHost");
    let (list, _) = s.ok(0, "RequestUndoList", json!([a, game])).await;
    let list = list.as_array().unwrap();
    let count = alice_view["eventNumber"].as_i64().unwrap();
    assert_eq!(list.len() as i64, count);
    assert_eq!(list.last().unwrap()["eventNumber"], count - 1);

    // The undo preview (package 5) is for hosts: the host's view at the gap, without actions.
    let r = s.call(1, "RequestUndoPreview", json!([b, game, count - 2])).await;
    assert_eq!(r[0]["result"]["Error"], 22, "NoHost");
    let (preview, n) = s.ok(0, "RequestUndoPreview", json!([a, game, count - 2])).await;
    assert!(views(&n).is_empty(), "a preview sends no views: {:?}", names(&n));
    assert_eq!(preview["eventNumber"], count - 2);
    assert_eq!(preview["viewer"]["faction"], "Emperor");
    assert_eq!(ally(&preview, "Emperor"), "None");
    assert_eq!(preview["actions"], json!([]));
    // A gap past now is clamped to now, and the live game is untouched.
    let (now, _) = s.ok(0, "RequestUndoPreview", json!([a, game, count + 5])).await;
    assert_eq!(now["eventNumber"], count);
    assert_eq!(ally(&now, "Emperor"), "Atreides");

    // Save game: hosts get the server's game as a savegame; others are refused.
    let r = s.call(1, "RequestSaveGame", json!([b, game])).await;
    assert_eq!(r[0]["result"]["Error"], 22, "NoHost");
    let (saved, _) = s.ok(0, "RequestSaveGame", json!([a, game])).await;
    let saved: Value = serde_json::from_str(saved.as_str().unwrap()).unwrap();
    assert_eq!(saved["_events"].as_array().unwrap().len() as i64, count);

    let (_, n) = s.ok(0, "RequestUndo", json!([a, game, count - 2])).await;
    let undone = views(&n).last().unwrap().to_owned().clone();
    assert_eq!(undone["eventNumber"], count - 2);
    assert_eq!(ally(&undone, "Emperor"), "None");
    // The preview was the view the undo gives, but for its actions.
    let strip = |v: &Value| {
        let mut v = v.clone();
        v["actions"] = json!([]);
        if let Some(hand) = v["me"]["hand"].as_array_mut() {
            hand.iter_mut().for_each(|c| c["playableWith"] = json!([]));
        }
        v
    };
    assert_eq!(strip(&undone), strip(&preview));
    assert_eq!(views(&s.drain(1)).last().unwrap()["eventNumber"], count - 2);

    // Reports come from the replica, filtered for the caller.
    let reports = undone["log"]["reports"].as_array().unwrap().len();
    let (lines, _) = s.ok(1, "RequestReport", json!([b, game, reports - 1])).await;
    assert_eq!(lines["index"], reports - 1);
    let r = s.call(1, "RequestReport", json!([b, game, reports])).await;
    assert_eq!(r[0]["result"]["Success"], false);

    // A reconnecting connection gets its view before the reply.
    let (_, n) = s.ok(1, "RequestReconnectGame", json!([b, game])).await;
    assert_eq!(views(&n).len(), 1);

    // Bad calls are error replies; a malformed message closes the connection.
    let r = s.call(0, "RequestNothing", json!([])).await;
    assert_eq!(r[0]["error"], "Unknown method 'RequestNothing'");
    runtime::receive(&mut s.hub, &mut s.views, &mut s.outgoing, 1, "not json").await;
    assert!(!s.outgoing.contains_key(&1));
    let _ = b_id;
}
