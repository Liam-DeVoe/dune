use crate::json::JsonError;
use crate::json::utilities::Utilities;
use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct GameState {
    pub version: i32,
    pub events: Option<Vec<HistoryEntry>>,
}

impl Default for GameState {
    fn default() -> Self {
        GameState {
            version: Default::default(),
            events: Some(Default::default()),
        }
    }
}

impl GameState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn events(&self) -> Vec<HistoryEntry> {
        // get => _events (enumerating a null array throws at the caller)
        self.events.clone().expect("events")
    }

    pub fn set_events(&mut self, value: &[HistoryEntry]) {
        self.events = Some(value.to_vec());
    }

    /// Reads a savegame.
    pub fn load(data: &str) -> Result<GameState, JsonError> {
        // Old savegames named their types "Treachery.online.*". The replacement also changes
        // any text in the savegame that contains the string, which players could see.
        let data = data.replace("Treachery.online", "Treachery");
        Utilities::deserialize::<GameState>(&data)?.ok_or_else(|| JsonError::new("JsonException", "Cannot load game state"))
    }

    pub fn get_state_as_string(g: &Game) -> String {
        let mut state = GameState::new();
        state.version = g.rules.version;
        state.set_events(&g.history.entries);
        Utilities::serialize(&state)
    }

}
