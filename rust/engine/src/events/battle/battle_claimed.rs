use crate::prelude::*;

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct BattleClaimed {
    #[serde(skip)]
    pub passed: bool,
}

impl BattleClaimed {
    pub fn determine_ecaz_contribution(&self, game: &mut Game) {
        let fighter = game.get_player(game.fight().and_then(|c| c.ecaz_or_ally_fighter));

        if fighter.is_some()
            && game.current_battle().is_some()
            && (game.player(fighter.expect("fighter")).is(Faction::Ecaz) || game.player(fighter.expect("fighter")).ally == Some(Faction::Ecaz))
        {
            let ecaz = game.get_player(Some(Faction::Ecaz));
            if let Some(ecaz) = ecaz {
                let territory = game.current_battle().expect("current battle").territory.expect("territory");
                let forces = game.player(ecaz).any_forces_in_territory(Some(territory));
                game.fight_mut().ecaz_battle_contribution = half_rounded_up(forces);
            }
        } else {
            game.fight_mut().ecaz_battle_contribution = 0;
        }
    }
}

impl EventRules for BattleClaimed {
    fn validate(&self, _game: &Game, _by: By) -> Result<(), Rejection> {
        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        let m = self.describe(game, by);
        game.log_event(m);
        let fighter = if self.passed { game.get_ally(Some(Faction::Ecaz)) } else { Some(Faction::Ecaz) };
        game.fight_mut().ecaz_or_ally_fighter = fighter;
        game.enter_battle(BattleStep::Planning);
        game.initiate_battle();
        self.determine_ecaz_contribution(game);
        Ok(())
    }

    fn describe(&self, _game: &Game, _by: By) -> Message {
        if !self.passed {
            return express!(Faction::Ecaz, " will fight this battle");
        }
        express!(Faction::Ecaz, "'s ally will fight this battle")
    }
}
