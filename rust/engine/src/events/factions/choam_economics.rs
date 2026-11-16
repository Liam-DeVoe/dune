use crate::prelude::*;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ChoamEconomics {
    pub status: Option<ChoamEconomicsStatus>,
}

impl EventRules for ChoamEconomics {
    fn validate(&self, _game: &Game, _by: By) -> Result<(), Rejection> {
        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        game.charity.economics_status = self.status;
        game.stone(Milestone::Economics);
        let m = self.describe(game, by);
        game.log_event(m);
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        express!(by.faction, " play Inflation: ", self.status)
    }
}
