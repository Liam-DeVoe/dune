use crate::prelude::*;

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct PortableAntidoteUsed {}

impl PortableAntidoteUsed {
    pub fn can_be_played(g: &Game, p: PlayerId) -> bool {
        let card = g.player(p).card(TreacheryCardType::PortableAntidote);
        if let Some(card) = card {
            let plan = g.battle.current_battle.as_ref().and_then(|b| b.plan_of(g, Some(p)));
            if let Some(plan) = plan {
                if plan.defense().is_none()
                    && g.battle.current_portable_antidote_used.is_none()
                    && Battle::valid_defenses(g, p, plan.weapon(), g.battle.current_battle.as_ref().and_then(|b| b.territory), false)
                        .contains(&Some(card))
                {
                    return true;
                }
            }
        }

        false
    }
}

impl EventRules for PortableAntidoteUsed {
    fn validate(&self, _game: &Game, _by: By) -> Result<(), Rejection> {
        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        let m = self.describe(game, by);
        game.log_event(m);
        game.battle.current_portable_antidote_used = Some(AntidoteEquipped { submitter: by });
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        express!(by.faction, " equip a ", TreacheryCardType::PortableAntidote)
    }
}
