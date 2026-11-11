//! `POST /dev/scene`: the Rust side of the C# client's `/scene` page (client-plan.md, Comparing
//! with the C# client). Development only; off unless the `DevScene` setting is `true`.
//!
//! It calls `Connect` as a client would, then loads `<DevSceneFixtures>/<fixture>.json` as a
//! new game through the normal hub methods, as the user "Scene" (created on first use), seats
//! that user as `faction` and makes them host.
//! The other seats stay without a user, so they count as bots that never move, as on the C#
//! scene page. Events submitted to the game afterwards run through the normal server path.
//!
//! The hub methods need a connection (`AddToGroup`), so the scene uses a connection id that no
//! socket has, and leaves the group again at the end. The web client then connects and calls
//! `RequestReconnectGame` like any client.
//!
//! `DevSceneClock` (an ISO 8601 time, such as `2027-01-01T00:00:00Z`) stops the server's clock at
//! that time, as the comparison harness stops the pages' clocks (`scripts/ui/compare.py`,
//! `FIXED_TIME`). Events submitted to a scene game are then stamped with the page's time, as the
//! C# scene page stamps them, so the clocks that show the time since an event agree. It is read
//! only when `DevScene` is on; without it, or with `DevScene` off, the server uses the real clock.

use treachery_engine::enumerations::Faction;
use treachery_engine::game::Game;

use std::rc::Rc;

use treachery_engine::date_time_offset::DateTimeOffset;

use crate::clients::ConnectionId;
use crate::clock::{Clock, ManualClock, SystemClock};
use crate::config::Config;
use crate::hub::{CallContext, GameHub};

/// The scene user's name, as on the C# scene page.
pub const SCENE_USER: &str = "Scene";
/// The scene user's password hash. Any value works; the scene user is for local development.
const SCENE_PASSWORD_HASH: &str = "scene";

/// Whether the endpoint is on (`DevScene`).
pub fn enabled(config: &Config) -> bool {
    config.get("DevScene").is_some_and(|v| v.eq_ignore_ascii_case("true"))
}

/// The fixture directory: `DevSceneFixtures`, by default the repository's `scripts/fixtures`.
pub fn fixtures_dir(config: &Config) -> std::path::PathBuf {
    match config.get("DevSceneFixtures") {
        Some(dir) if !dir.is_empty() => dir.into(),
        _ => std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/fixtures"),
    }
}

/// The fixed time `DevSceneClock` sets, when `DevScene` is on. An unreadable value is an error,
/// so a typo can't silently leave the real clock running.
pub fn fixed_time(config: &Config) -> Result<Option<DateTimeOffset>, String> {
    if !enabled(config) {
        return Ok(None);
    }
    match config.get("DevSceneClock") {
        Some(v) if !v.is_empty() => DateTimeOffset::parse_json(v).map(Some).ok_or_else(|| format!("DevSceneClock: can't read '{v}' as a time")),
        _ => Ok(None),
    }
}

/// The server's clock: stopped at `DevSceneClock` on a dev scene server, else the real one.
pub fn clock(config: &Config) -> Result<Rc<dyn Clock>, String> {
    Ok(match fixed_time(config)? {
        Some(t) => Rc::new(ManualClock::new(t)),
        None => Rc::new(SystemClock),
    })
}

/// What the endpoint returns: `{"userToken":..,"gameId":..,"userId":..}`.
pub struct SceneGame {
    pub user_token: String,
    pub game_id: String,
    pub user_id: i32,
}

impl SceneGame {
    pub fn to_json(&self) -> String {
        serde_json::json!({ "userToken": self.user_token, "gameId": self.game_id, "userId": self.user_id }).to_string()
    }
}

/// A faction by its C# member name (`"Atreides"`).
pub fn parse_faction(name: &str) -> Option<Faction> {
    Faction::INFO.declared.iter().find(|(n, _)| n.eq_ignore_ascii_case(name)).map(|(_, v)| Faction(*v))
}

fn check<T>(what: &str, r: &crate::model::result::CallResult<T>) -> Result<(), String> {
    if r.success {
        Ok(())
    } else {
        Err(format!("{what}: {:?} {}", r.error, r.error_details))
    }
}

/// Creates the scene game. `connection` is an id no socket uses.
pub async fn create_scene(hub: &mut GameHub, connection: ConnectionId, fixture: &str, faction: &str) -> Result<SceneGame, String> {
    if fixture.is_empty() || !fixture.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        return Err(format!("bad fixture name '{fixture}'"));
    }
    let faction = parse_faction(faction).ok_or_else(|| format!("unknown faction '{faction}'"))?;
    let path = fixtures_dir(&hub.services.config).join(format!("{fixture}.json"));
    let savegame = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let hub_error = |e: crate::hub::HubError| e.to_string();

    // Connect first, as every client does. On a fresh server `Connect` restores the running
    // games from the database, replacing each with its saved copy. Without it here, the web
    // client's own `Connect` would replace the scene game with the copy `RequestCreateGame`
    // saved before the seat was assigned (`RequestAssignSeats` saves only every few minutes),
    // and the scene user would be unseated.
    let connected = hub.connect().await.map_err(hub_error)?;
    check("Connect", &connected)?;

    // Log in as the scene user, creating it the first time.
    let mut login = hub.request_login(Game::LATEST_VERSION, SCENE_USER, SCENE_PASSWORD_HASH).await.map_err(hub_error)?;
    if !login.success {
        let created = hub.request_create_user(SCENE_USER, SCENE_PASSWORD_HASH, None, SCENE_USER).await.map_err(hub_error)?;
        check("RequestCreateUser", &created)?;
        login = created;
    }
    let login = login.contents.ok_or("login without contents")?;

    // Close the scene user's earlier games: a user may run only ten.
    let earlier: Vec<String> =
        hub.running_games_by_game_id.iter().filter(|(_, g)| g.borrow().creator_user_id == login.user_id).map(|(id, _)| id.clone()).collect();
    for game_id in earlier {
        hub.request_close_game(&login.token, &game_id).await.map_err(hub_error)?;
    }

    let cx = CallContext { connection_id: Some(connection) };
    let created = hub.request_create_game(&cx, Some(fixture), &login.token, None, Some(&savegame), None).await.map_err(hub_error)?;
    check("RequestCreateGame", &created)?;
    let game_id = created.contents.and_then(|c| c.game_id).ok_or("RequestCreateGame without a game id")?;
    // The C# scene page has no bots; the seats the scene user doesn't take must not play either.
    hub.scene_games.insert(game_id.clone());

    let seat = {
        let game = hub.running_games_by_game_id.get(&game_id).ok_or("the scene game is gone")?.clone();
        let mg = game.borrow();
        let player = mg.game.get_player(Some(faction)).ok_or_else(|| format!("{faction} is not in {fixture}"))?;
        mg.game.player(player).seat
    };
    let mut assignment = dotnet_compat::NetDictionary::new();
    assignment.add(login.user_id, seat);
    let assigned = hub.request_assign_seats(&login.token, &game_id, assignment).await.map_err(hub_error)?;
    check("RequestAssignSeats", &assigned)?;

    hub.clients.remove_from_group(connection, &game_id);
    if let Some(info) = hub.connection_info_by_user_id.get_mut(&login.user_id) {
        if info.try_get_connection_id(&game_id) == Some(connection) {
            info.remove_connection_id(&game_id);
        }
    }
    Ok(SceneGame { user_token: login.token, game_id, user_id: login.user_id })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_clock_stops_only_on_a_dev_scene_server() {
        let at = "2027-01-01T00:00:00Z";
        let fixed = DateTimeOffset::parse_json(at).unwrap();
        assert_eq!(fixed_time(&Config::from_pairs([("DevSceneClock", at)])), Ok(None));
        assert_eq!(fixed_time(&Config::from_pairs([("DevScene", "false"), ("DevSceneClock", at)])), Ok(None));
        assert_eq!(fixed_time(&Config::from_pairs([("DevScene", "true")])), Ok(None));
        assert_eq!(fixed_time(&Config::from_pairs([("DevScene", "true"), ("DevSceneClock", at)])), Ok(Some(fixed)));
        assert!(fixed_time(&Config::from_pairs([("DevScene", "true"), ("DevSceneClock", "soon")])).is_err());
        assert_eq!(clock(&Config::from_pairs([("DevScene", "true"), ("DevSceneClock", at)])).unwrap().now(), fixed);
    }
}
