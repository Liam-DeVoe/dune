use crate::prelude::*;

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct RecruitsPlayed {}

impl RecruitsPlayed {
    pub fn is_applicable(g: &Game, p: PlayerId) -> bool {
        g.phase() == Phase::BeginningOfResurrection && g.player(p).has_treachery_card_type(TreacheryCardType::Recruits)
    }
}

impl EventRules for RecruitsPlayed {
    fn validate(&self, _game: &Game, _by: By) -> Result<(), Rejection> {
        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        let m = self.describe(game, by);
        game.log_event(m);
        game.revival.current_recruits_played = true;
        game.discard(by.player(), TreacheryCardType::Recruits)?;
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        express!(by.faction, " use ", TreacheryCardType::Recruits, " to double free revivals and set revival limits to ", 7)
    }
}
