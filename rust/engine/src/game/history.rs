use crate::prelude::*;

/// The events played so far, the events undone, and what each event's moment and timing were.
pub struct History {
    pub entries: Vec<GameEvent>,
    pub moments: Vec<Moment>,
    pub timers: IndexMap<Faction, Timer<MainPhase>>,
    pub recently_undone_event_count: i32,
    pub recently_undone_events: Vec<GameEvent>,
    /// The game as it was before the last `undo`, which `restore_recently_undone` returns to.
    pub previous_game_state: Option<Box<Game>>,
}

impl History {
    pub fn new() -> Self {
        History {
            entries: Vec::new(),
            moments: Vec::new(),
            timers: Default::default(),
            recently_undone_event_count: Default::default(),
            recently_undone_events: Vec::new(),
            previous_game_state: None,
        }
    }
}
