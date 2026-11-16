use crate::prelude::*;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PoisonToothCancelled {}

impl EventRules for PoisonToothCancelled {
    fn validate(&self, _game: &Game, _by: By) -> Result<(), Rejection> {
        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        game.battle.poison_tooth_cancelled = true;
        let m = self.describe(game, by);
        game.log_event(m);
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        express!(by.faction, " don't use their ", TreacheryCardType::PoisonTooth)
    }
}
