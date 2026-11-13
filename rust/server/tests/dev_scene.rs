//! `POST /dev/scene`: a game from a fixture with the scene user seated as one faction.

mod common;

use common::*;
use serde_json::{json, Value};

async fn scene(s: &mut TestServer, fixture: &str, faction: &str) -> Value {
    let json = s.server().create_scene(fixture, faction).await.unwrap();
    let c = s.connect();
    let token = json["userToken"].as_str().unwrap().to_string();
    let game = json["gameId"].as_str().unwrap().to_string();
    let (reply, received) = s.call_raw(c, "RequestReconnectGame", json!([token, game])).await;
    assert_eq!(reply["result"]["Success"], true, "{reply}");
    views(&received).pop().unwrap()
}

#[test]
fn the_scene_user_plays_the_faction() {
    rt().block_on(async {
        let mut s = TestServer::start_with(&[("TREACHERY_DEV_SCENE", "true")]).await;
        let view = scene(&mut s, "alliance", "Emperor").await;
        assert_eq!(view["me"]["faction"], "Emperor");
        assert!(actions(&view).contains(&"AllianceOffered".to_string()));
        // Again, as another faction: the earlier scene closes.
        let view = scene(&mut s, "alliance", "Atreides").await;
        assert_eq!(view["me"]["faction"], "Atreides");
        assert!(s.server().create_scene("alliance", "Nobody").await.is_err());
        assert!(s.server().create_scene("../secret", "Emperor").await.is_err());
        // The scene's other seats never act.
        s.advance(std::time::Duration::from_secs(120)).await;

        let mut off = TestServer::start().await;
        assert!(off.server().create_scene("alliance", "Emperor").await.is_err());
    });
}
