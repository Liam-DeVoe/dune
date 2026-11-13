use crate::prelude::*;

pub trait IBot {
    fn determine_highest_priority_in_phase_action(&mut self, game: &mut Game, events: &[GameEventType]) -> Option<GameEvent>;
    fn determine_high_priority_in_phase_action(&mut self, game: &mut Game, events: &[GameEventType]) -> Option<GameEvent>;
    fn determine_middle_priority_in_phase_action(&mut self, game: &mut Game, events: &[GameEventType]) -> Option<GameEvent>;
    fn determine_low_priority_in_phase_action(&mut self, game: &mut Game, events: &[GameEventType]) -> Option<GameEvent>;
    fn determine_end_phase_action(&mut self, game: &mut Game, events: &[GameEventType]) -> Option<GameEvent>;
    fn set_game_and_player(&mut self, game: &Game, player: PlayerIndex);
}

impl IBot for ClassicBot {
    fn determine_highest_priority_in_phase_action(&mut self, game: &mut Game, events: &[GameEventType]) -> Option<GameEvent> {
        ClassicBot::determine_highest_priority_in_phase_action(self, game, events)
    }
    fn determine_high_priority_in_phase_action(&mut self, game: &mut Game, events: &[GameEventType]) -> Option<GameEvent> {
        ClassicBot::determine_high_priority_in_phase_action(self, game, events)
    }
    fn determine_middle_priority_in_phase_action(&mut self, game: &mut Game, events: &[GameEventType]) -> Option<GameEvent> {
        ClassicBot::determine_middle_priority_in_phase_action(self, game, events)
    }
    fn determine_low_priority_in_phase_action(&mut self, game: &mut Game, events: &[GameEventType]) -> Option<GameEvent> {
        ClassicBot::determine_low_priority_in_phase_action(self, game, events)
    }
    fn determine_end_phase_action(&mut self, game: &mut Game, events: &[GameEventType]) -> Option<GameEvent> {
        ClassicBot::determine_end_phase_action(self, game, events)
    }
    fn set_game_and_player(&mut self, game: &Game, player: PlayerIndex) {
        ClassicBot::set_game_and_player(self, game, player)
    }
}
