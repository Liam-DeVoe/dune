//! `POST /dev/scene` (`dev.rs`): the scene user must be seated as the chosen faction, so the
//! web client that reconnects to the scene gets that faction's view and actions, on a fresh
//! server (the user is created, and the client's `Connect` is the first) and on later scenes
//! (the user logs in).

use std::rc::Rc;

use serde_json::{json, Value};
use tokio::sync::mpsc;
use treachery_server::clock::SystemClock;
use treachery_server::config::Config;
use treachery_server::db::Db;
use treachery_server::dev;
use treachery_server::hub::{GameHub, SequentialIds, Services};
use treachery_server::mail::RecordingMailer;
use treachery_server::runtime::{self, Outgoing};
use treachery_server::timers::RecordingTimers;
use treachery_server::views::GameViews;

/// A connection id no socket has, as the runtime gives `create_scene`.
const GHOST: u64 = 1000;

struct Server {
    hub: GameHub,
    views: GameViews,
    outgoing: Outgoing,
    queue: mpsc::UnboundedReceiver<String>,
    next_id: u64,
}

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
        let mut hub = GameHub::new(services);
        let views = GameViews::new();
        let mut outgoing = Outgoing::new();
        let (tx, queue) = mpsc::unbounded_channel();
        runtime::connect(&mut hub, &views, &mut outgoing, 0, tx);
        Server { hub, views, outgoing, queue, next_id: 0 }
    }

    /// What the runtime does for `POST /dev/scene`.
    async fn scene(&mut self, fixture: &str, faction: &str) -> dev::SceneGame {
        let scene = dev::create_scene(&mut self.hub, GHOST, fixture, faction).await.unwrap_or_else(|e| panic!("{fixture}/{faction}: {e}"));
        runtime::flush_views(&mut self.views, &self.hub, &self.outgoing);
        scene
    }

    /// Sends a call on connection 0; returns the notifications before the reply and the reply.
    async fn call(&mut self, method: &str, args: Value) -> (Vec<Value>, Value) {
        self.next_id += 1;
        let text = json!({"type": "call", "id": self.next_id, "method": method, "args": args}).to_string();
        runtime::receive(&mut self.hub, &mut self.views, &mut self.outgoing, 0, &text).await;
        let mut received = vec![];
        while let Ok(text) = self.queue.try_recv() {
            received.push(serde_json::from_str::<Value>(&text).unwrap());
        }
        let reply = received.pop().unwrap_or_else(|| panic!("{method}: no reply"));
        assert_eq!(reply["type"], "reply", "{method}: {reply}");
        (received, reply["result"].clone())
    }

    /// The web client's part (`WebSocketSource`): connect, check the token, reconnect to the
    /// scene, and take the view it is sent. Its `Connect` is the first on a fresh server, which
    /// restores the running games from the database.
    async fn open(&mut self, scene: &dev::SceneGame) -> (Value, Value) {
        let (_, connected) = self.call("Connect", json!([])).await;
        assert_eq!(connected["Success"], true, "{connected}");
        let (_, login) = self.call("GetLoginInfo", json!([scene.user_token])).await;
        assert_eq!(login["Contents"]["UserId"], scene.user_id, "{login}");
        let (notifications, result) = self.call("RequestReconnectGame", json!([scene.user_token, scene.game_id])).await;
        assert_eq!(result["Success"], true, "{result}");
        let views: Vec<&Value> = notifications.iter().filter(|n| n["name"] == "GameView").map(|n| &n["args"][0]).collect();
        assert_eq!(views.len(), 1, "{notifications:?}");
        (result["Contents"].clone(), views[0].clone())
    }
}

fn seat_of(view: &Value, faction: &str) -> i64 {
    view["players"].as_array().unwrap().iter().find(|p| p["faction"] == faction).unwrap_or_else(|| panic!("{faction} in {}", view["players"]))["seat"]
        .as_i64()
        .unwrap()
}

fn action_types(view: &Value) -> Vec<String> {
    view["actions"].as_array().unwrap().iter().map(|a| a["eventType"].as_str().unwrap_or_default().to_string()).collect()
}

async fn assert_seated(s: &mut Server, fixture: &str, faction: &str, action: &str) -> dev::SceneGame {
    let scene = s.scene(fixture, faction).await;
    let (info, view) = s.open(&scene).await;

    let seated = &info["Participation"]["SeatedPlayers"];
    // `[[userId, seat]]`: dictionaries go over the wire as pairs (protocol.md, Encoding).
    let pairs = seated.as_array().unwrap_or_else(|| panic!("{fixture}/{faction}: {seated}"));
    assert_eq!(pairs.len(), 1, "{fixture}/{faction}: {seated}");
    assert_eq!(pairs[0][0].as_i64(), Some(scene.user_id as i64), "{fixture}/{faction}: {seated}");
    let seat = pairs[0][1].as_i64().unwrap_or_else(|| panic!("{fixture}/{faction}: {seated}"));
    assert_eq!(seat, seat_of(&view, faction), "{fixture}/{faction}: {seated}");

    assert_eq!(view["viewer"]["faction"], faction, "{fixture}/{faction}: {}", view["viewer"]);
    assert_eq!(view["viewer"]["isHost"], true);
    assert_eq!(view["me"]["faction"], faction);
    let actions = action_types(&view);
    assert!(actions.iter().any(|a| a == action), "{fixture}/{faction}: {action} in {actions:?}");
    scene
}

#[tokio::test(flavor = "current_thread")]
async fn scene_user_is_seated_as_the_faction() {
    let mut s = Server::new().await;
    // The first scene on a fresh server creates the scene user.
    let first = assert_seated(&mut s, "alliance", "Emperor", "AllianceOffered").await;
    // Later scenes log the user in and close the earlier scene.
    let second = assert_seated(&mut s, "alliance", "Atreides", "AllianceOffered").await;
    assert_eq!(first.user_id, second.user_id);
    assert_ne!(first.game_id, second.game_id);
    let (_, closed) = s.call("RequestReconnectGame", json!([second.user_token, first.game_id])).await;
    assert_eq!(closed["Success"], false, "the earlier scene is closed: {closed}");
    assert_seated(&mut s, "bid", "Atreides", "Bid").await;
}
