use crate::prelude::*;

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct RockWasMelted {
    pub kill: bool,
}

impl RockWasMelted {
    pub fn can_be_played(g: &Game, p: PlayerId) -> bool {
        let plan = g.current_battle().expect("current battle").plan_of(g, Some(p));
        match plan {
            Some(plan) => match plan.weapon() {
                Some(weapon) => weapon.is_rock_melter(),
                None => false,
            },
            None => false,
        }
    }
}

impl EventRules for RockWasMelted {
    fn validate(&self, _game: &Game, _by: By) -> Result<(), Rejection> {
        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        let m = self.describe(game, by);
        game.log_event(m);
        game.fight_mut().rock_was_melted = Some(RockMelted { submitter: by, kill: self.kill });
        game.enter_battle(BattleStep::CallingTraitor);
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        if self.kill {
            return express!(by.faction, " use their ", TreacheryCardType::Rockmelter, " to kill both leaders");
        }
        express!(by.faction, " use their ", TreacheryCardType::Rockmelter, " to reduce both leaders to 0 strength")
    }
}
