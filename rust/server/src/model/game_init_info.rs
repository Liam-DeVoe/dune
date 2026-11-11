//! Port of `Model/GameInitInfo.cs`.
//!
//! The hub fills `game_state` exactly as C# does, so the hub layer stays comparable with C#.
//! The transport leaves it out: clients get per-player views instead (protocol.md).

use treachery_engine::cs::Shared;
use treachery_engine::json::{JsonWriter, ToJson};
use treachery_engine::participation::Participation;

use super::phase_control_info::PhaseControlInfo;

#[derive(Clone, Debug)]
pub struct GameInitInfo {
    /// C# `string` without an initializer: null unless set.
    pub game_id: Option<String>,
    pub game_state: Option<String>,
    pub game_name: Option<String>,
    /// C# stores the game's own `Participation` object; it is serialized when the result is
    /// sent, after the hub method returned.
    pub participation: Option<Shared<Participation>>,
    /// What the current pause between phases waits for.
    pub phase_control: Option<PhaseControlInfo>,
}

impl ToJson for GameInitInfo {
    fn write_json(&self, w: &mut JsonWriter) {
        w.object_value();
        w.member("GameId", &self.game_id);
        w.member("GameState", &self.game_state);
        w.member("GameName", &self.game_name);
        w.key("Participation");
        match &self.participation {
            Some(p) => crate::json::write_participation(w, &p.borrow()),
            None => w.null(),
        }
        w.member("PhaseControl", &self.phase_control);
        w.end_object();
    }
}
