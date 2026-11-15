use crate::prelude::*;

/// The game log: one report per phase, plus the report of messages logged before the game started.
pub struct Log {
    pub report_list: Vec<Report>,
    pub pre_game_report: Report,
}

impl Log {
    pub fn new() -> Self {
        Log {
            report_list: Vec::new(),
            pre_game_report: Report::new(MainPhase::None, 0),
        }
    }
}
