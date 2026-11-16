use crate::prelude::*;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Bureaucracy {
    pub passed: bool,
}

impl Bureaucracy {
    pub fn get_dynamic_message(&self, game: &Game, by: By) -> Message {
        if self.passed {
            return express!(by.faction, " don't apply Bureaucracy");
        }
        express!(by.faction, " apply Bureaucracy → ", game.turn.target_of_bureaucracy, " lose ", Payment::of(2))
    }
}

impl EventRules for Bureaucracy {
    fn validate(&self, _game: &Game, _by: By) -> Result<(), Rejection> {
        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        let m = self.get_dynamic_message(game, by);
        game.log(parts![m]);

        if !self.passed {
            game.stone(Milestone::Bureaucracy);
            game.turn.bureaucrat_was_used_this_phase = true;
            let victim = game.get_player(game.turn.target_of_bureaucracy).required("get player")?;
            let v = game.player_mut(victim);
            v.resources -= 2;
            game.turn.was_victim_of_bureaucracy = game.turn.target_of_bureaucracy;
        }

        let phase = game.turn.phase_before_bureaucrat_was_activated.required("phase before bureaucrat was activated")?;
        game.enter(phase, true);
        game.turn.target_of_bureaucracy = None;
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        if self.passed {
            return express!(by.faction, " don't apply Bureaucracy");
        }
        express!(by.faction, " apply Bureaucracy")
    }
}
