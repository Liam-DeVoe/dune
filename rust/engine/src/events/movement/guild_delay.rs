use crate::prelude::*;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct GuildDelay {}

impl EventRules for GuildDelay {
    fn validate(&self, _game: &Game, _by: By) -> Result<(), Rejection> {
        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        game.ship_move.beginning_of_shipment_and_move_phase = false;
        let m = self.describe(game, by).with_log(GuildDelayed::default());
        game.log(parts![m]);
        game.enter(Phase::NonGuildShip, true);
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        express!(by.faction, " delay their turn")
    }
}
