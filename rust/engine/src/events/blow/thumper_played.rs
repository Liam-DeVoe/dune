use crate::prelude::*;

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct ThumperPlayed {
}

impl EventRules for ThumperPlayed {
    fn validate(&self, _game: &Game, _by: By) -> Result<(), Rejection> {
        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        // Only reached outside the phase when events are replayed without validation.
        if game.blow().is_none() {
            game_error!("Not in the spice blow phase");
        }
        game.discard(by.player(), TreacheryCardType::Thumper)?;
        let m = self.describe(game, by);
        game.log_event(m);
        game.stone(Milestone::Thumper);
        game.blow_state().thumper_called_shai_hulud = game.turn.current_turn > Turn::FIRST;
        game.enter_blow_a()?;
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        express!(by.faction, " use a ", TreacheryCardType::Thumper, " to attract ", Concept::Monster)
    }
}
