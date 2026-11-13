use crate::json::utilities::Utilities;
use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct GameState {
    pub version: i32,
    #[canon(name = "_events")]
    pub events: Option<Vec<GameEvent>>,
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

    pub fn events(&self) -> Vec<GameEvent> {
        // get => _events (enumerating a null array throws at the caller)
        self.events.clone().nre()
    }

    pub fn set_events(&mut self, value: &[GameEvent]) {
        self.events = Some(value.to_vec());
    }

    pub fn load(data: &str) -> GameState {
        let fixed_state_data = GameState::fix_game_state_string(data);
        match Utilities::deserialize::<GameState>(&fixed_state_data) {
            Some(state) => state,
            None => throw!(InvalidOperationException, "Cannot load game state"),
        }
    }

    pub fn get_state_as_string(g: &Game) -> String {
        let mut state = GameState::new();
        state.version = g.version;
        state.set_events(&g.history);
        Utilities::serialize(&state)
    }

    pub fn fix_game_state_string(state: &str) -> String {
        state.replace("Treachery.online", "Treachery")
    }

}
