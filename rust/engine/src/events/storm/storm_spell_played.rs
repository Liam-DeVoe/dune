use crate::log::storm::StormMoved;
use crate::prelude::*;

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct StormSpellPlayed {
    pub move_amount: u16,
}

impl EventRules for StormSpellPlayed {
    fn validate(&self, _game: &Game, _by: By) -> Result<(), Rejection> {
        if self.move_amount > 10 {
            return Err(Rejection(express!("Invalid number of sectors")));
        }

        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        let card = game.player(by.player()).card(TreacheryCardType::StormSpell);
        game.discard(by.player(), TreacheryCardType::StormSpell)?;
        let msg = self.describe(game, by).with_log(StormMoved::new(game.board.sector_in_storm, i32::from(self.move_amount), by.faction, card));
        game.log(parts![msg]);
        game.move_storm_and_determine_next(i32::from(self.move_amount))?;
        game.end_storm_phase();
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        express!(by.faction, " use ", TreacheryCardType::StormSpell, " to move the storm ", i32::from(self.move_amount), " sectors")
    }
}
