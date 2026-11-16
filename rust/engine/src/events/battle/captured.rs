use crate::prelude::*;

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct Captured {
    #[serde(skip)]
    pub passed: bool,
}

impl EventRules for Captured {
    fn validate(&self, _game: &Game, _by: By) -> Result<(), Rejection> {
        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        let m = self.describe(game, by);
        game.log_event(m);

        if !self.passed {
            if game.prevented(FactionAdvantage::HarkonnenCaptureLeader) {
                game.log_prevention_by_karma(FactionAdvantage::HarkonnenCaptureLeader);
            } else {
                game.capture_leader();
            }
        }

        game.enter_battle(BattleStep::Conclusion);
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        if self.passed {
            return express!(by.faction, " won't capture or kill a leader");
        }
        express!(by.faction, " will capture or kill a random leader...")
    }
}
