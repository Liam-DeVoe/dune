use crate::prelude::*;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct FaceDancerRevealed {
    pub passed: bool,
}

impl EventRules for FaceDancerRevealed {
    fn validate(&self, game: &Game, by: By) -> Result<(), Rejection> {
        if self.passed {
            return Ok(());
        }
        if !FaceDanced::may_call_face_dancer_opt(game, by.player) {
            return Err(Rejection(express!("You can't reveal a face dancer")));
        }
        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        if !self.passed {
            let face_dancers = &game.player(by.player()).face_dancers;
            let facedancer = if face_dancers.is_empty() {
                None
            } else {
                let winner = game.winner_hero().required("battle winner")?;
                face_dancers.iter().copied().find(|f| winner.is_face_dancer(*f))
            };
            game.log(parts![by.faction, " reveal ", facedancer, " as one of their Face Dancers!"]);
            game.stone(Milestone::FaceDanced);
            game.enter(Phase::Facedancing, true);
        } else {
            game.log(parts![by.faction, " don't reveal a Face Dancer"]);
            game.finish_battle()?;
        }
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        if !self.passed {
            return express!(by.faction, " reveal a face dancer!");
        }
        express!(by.faction, " don't reveal a face dancer")
    }
}
