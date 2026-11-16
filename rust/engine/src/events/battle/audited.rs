use crate::prelude::*;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Audited {}

impl EventRules for Audited {
    fn validate(&self, _game: &Game, _by: By) -> Result<(), Rejection> {
        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        let m = self.describe(game, by);
        game.log_event(m);
        game.stone(Milestone::Audited);
        // RegisterKnown only adds to the player's KnownCards, so the list does not change.
        let audited = game.battle.audited_cards.clone();
        for card in audited {
            Game::register_known_by_player(game, by.player(), Some(card));
        }
        let winner_is_none = game.battle.battle_winner == Some(None);
        let harkonnen_must_decide = game.harkonnen_must_decide_to_capture();
        // Enter(condition, FinishBattle, ...), with FinishBattle's failure passed on.
        if winner_is_none {
            game.finish_battle()?;
        } else {
            game.enter_either(harkonnen_must_decide, Phase::CaptureDecision, Phase::BattleConclusion);
        }
        Ok(())
    }

    fn describe(&self, _game: &Game, _by: By) -> Message {
        express!(Faction::Choam, " finish their audit")
    }
}
