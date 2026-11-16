//! Development scenes: `POST /dev/scene` opens a game from a fixture savegame with the user
//! "Scene" seated as one faction and hosting, for comparing the web client's screens.
//! The other seats have no user and never act, so the scene stays as the fixture left it.
//! Only with `TREACHERY_DEV_SCENE` on.

use treachery_protocol::UserId;
use treachery_engine::enumerations::Faction;

use super::games::load_savegame;
use super::{RunningGame, Server};
use crate::db::UserRow;
use crate::ids;
use crate::participation::Participation;

pub const SCENE_USER: &str = "scene";

/// A faction by name (`"Atreides"`).
pub fn parse_faction(name: &str) -> Option<Faction> {
    Faction::INFO.declared.iter().find(|(n, _)| n.eq_ignore_ascii_case(name)).and_then(|(_, v)| <Faction as treachery_engine::enum_type::GameEnum>::from_value(*v))
}

impl Server {
    /// Opens the scene and returns `{"userToken", "gameId", "userId"}`.
    pub async fn create_scene(&mut self, fixture: &str, faction: &str) -> Result<serde_json::Value, String> {
        let Some(settings) = self.services.settings.dev_scene.clone() else { return Err("dev scenes are off".into()) };
        if fixture.is_empty() || !fixture.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
            return Err(format!("bad fixture name '{fixture}'"));
        }
        let faction = parse_faction(faction).ok_or_else(|| format!("unknown faction '{faction}'"))?;
        let path = settings.fixtures.join(format!("{fixture}.json"));
        let savegame = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;

        let user = match self.users.values().find(|u| u.name == SCENE_USER) {
            Some(u) => u.id,
            None => {
                let row = UserRow {
                    id: UserId(0),
                    name: SCENE_USER.into(),
                    password_hash: crate::passwords::hash(&ids::new_token()),
                    email: String::new(),
                    player_name: "Scene".into(),
                    last_login: self.now(),
                    reset_token_digest: None,
                    reset_token_created: None,
                };
                let id = self.services.db.insert_user(&row).await.map_err(|e| e.to_string())?;
                self.users.insert(id, UserRow { id, ..row });
                id
            }
        };
        let earlier: Vec<String> = self.games.values().filter(|g| g.creator == user).map(|g| g.id.clone()).collect();
        for game in earlier {
            self.end_game(&game).await.map_err(|e| e.to_string())?;
        }

        let mut participation = Participation::new();
        let mut game = load_savegame(&savegame, &participation, false)?;
        let seat = game.get_player(Some(faction)).map(|p| game.player(p).seated()).ok_or_else(|| format!("{faction} is not in {fixture}"))?;
        participation.add_player(&mut game, user, "Scene", Some(seat));
        participation.set_or_unset_host(user);
        let id = ids::new_id();
        let mut running = RunningGame::new(id.clone(), fixture.to_string(), user, game, participation, self.now());
        running.scene = true;
        running.generation = self.new_generation();
        self.games.insert(id.clone(), running);
        self.changed(&id);
        let token = self.create_session(user).await.map_err(|e| e.to_string())?;
        Ok(serde_json::json!({ "userToken": token, "gameId": id, "userId": user }))
    }
}
