use crate::prelude::*;

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct Planetology {
    pub add_one_to_movement: bool,
}

impl Planetology {
    pub fn move_from_two_territories(&self) -> bool {
        !self.add_one_to_movement
    }

    pub fn can_be_played(g: &Game, p: PlayerId) -> bool {
        g.has_skill_in_front_of_shield(p, LeaderSkill::Planetologist) && g.turn.current_planetology.is_none()
    }
}

impl EventRules for Planetology {
    fn validate(&self, _game: &Game, _by: By) -> Result<(), Rejection> {
        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        let m = self.describe(game, by);
        game.log_event(m);
        game.turn.current_planetology = Some(PlanetologyUse { initiator: by.faction, add_one_to_movement: self.add_one_to_movement });
        Ok(())
    }

    fn describe(&self, _game: &Game, _by: By) -> Message {
        if self.add_one_to_movement {
            return express!(LeaderSkill::Planetologist, " adds ", 1, " to force movement");
        }
        express!(LeaderSkill::Planetologist, " allows movement from ", 2, " different territories")
    }
}
