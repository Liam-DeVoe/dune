use crate::prelude::*;

/// A savegame: `{"version":191,"events":[HistoryEntry, ..]}`. Savegames of version 190 and
/// before have another format, which `treachery-legacy` reads.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GameState {
    pub version: i32,
    pub events: Vec<HistoryEntry>,
}

impl GameState {
    /// Reads a savegame. The error says what is wrong with the text.
    pub fn load(data: &str) -> Result<GameState, String> {
        serde_json::from_str(data).map_err(|e| e.to_string())
    }

    /// The savegame of `g`.
    pub fn of(g: &Game) -> GameState {
        GameState { version: g.rules.version, events: g.history.entries.clone() }
    }

    pub fn get_state_as_string(g: &Game) -> String {
        serde_json::to_string(&GameState::of(g)).expect("savegames serialize")
    }
}
