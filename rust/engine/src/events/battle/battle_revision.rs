use crate::prelude::*;

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct BattleRevision {}

impl EventRules for BattleRevision {
    fn validate(&self, _game: &Game, _by: By) -> Result<(), Rejection> {
        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        let Some(current_battle) = game.current_battle().cloned() else {
            game_error!("BattleRevision event executed when there is no current battle");
        };

        let withdrawn = current_battle.plan_of(game, by.player);
        if let Some(withdrawn) = withdrawn {
            for aspect in Battle::aspects_seen_by_opponent(game, by.player()) {
                game.fight_mut().prescience_seen_plans.insert(aspect, withdrawn.clone());
            }
        }

        if by.is(current_battle.aggressor(game)) {
            let c = game.fight_mut();
            c.previous_aggressor_plan = c.aggressor_plan.take();
        } else if by.is(current_battle.defender(game)) {
            let c = game.fight_mut();
            c.previous_defender_plan = c.defender_plan.take();
        }
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        express!(by.faction, " revise their battle plan")
    }
}
