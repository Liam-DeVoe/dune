use crate::prelude::*;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct BattleRevision {}

impl EventRules for BattleRevision {
    fn validate(&self, _game: &Game, _by: By) -> Result<(), Rejection> {
        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        let Some(current_battle) = game.battle.current_battle.clone() else {
            game_error!("BattleRevision event executed when there is no current battle");
        };

        let withdrawn = current_battle.plan_of(game, by.player);
        if let Some(withdrawn) = withdrawn {
            for aspect in Battle::aspects_seen_by_opponent(game, by.player()) {
                game.battle.prescience_seen_plans.insert(aspect, withdrawn.clone());
            }
        }

        if by.is(current_battle.aggressor(game)) {
            game.battle.previous_aggressor_plan = game.battle.aggressor_plan.take();
        } else if by.is(current_battle.defender(game)) {
            game.battle.previous_defender_plan = game.battle.defender_plan.take();
        }
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        express!(by.faction, " revise their battle plan")
    }
}
