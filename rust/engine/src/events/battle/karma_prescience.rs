use crate::prelude::*;

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct KarmaPrescience {}

impl EventRules for KarmaPrescience {
    fn validate(&self, _game: &Game, _by: By) -> Result<(), Rejection> {
        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        let me = by.player();
        let card = Karma::valid_karma_cards(game, me).into_iter().next();
        game.play_karma(me, card, vec!["see the entire enemy battle plan".into()]);
        game.player_mut(me).special_karma_power_used = true;
        game.stone(Milestone::Karma);
        game.fight_mut().atreides_karma = true;
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        express!("Using ", TreacheryCardType::Karma, ", ", by.faction, " see the entire enemy battle plan")
    }
}
