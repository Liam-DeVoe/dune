use crate::prelude::*;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct BattleClaimed {
    pub passed: bool,
}

impl BattleClaimed {
    pub fn determine_ecaz_contribution(&self, game: &mut Game) {
        let fighter = game.get_player(game.battle.current_ecaz_or_ally_fighter.flatten());

        if fighter.is_some()
            && game.battle.current_battle.is_some()
            && (game.player(fighter.expect("fighter")).is(Faction::Ecaz) || game.player(fighter.expect("fighter")).ally == Some(Faction::Ecaz))
        {
            let ecaz = game.get_player(Some(Faction::Ecaz));
            if let Some(ecaz) = ecaz {
                let territory = game.battle.current_battle.as_ref().expect("current battle").territory.expect("territory");
                let forces = game.player(ecaz).any_forces_in_territory(Some(territory));
                game.battle.current_ecaz_battle_contribution = half_rounded_up(forces);
            }
        } else {
            game.battle.current_ecaz_battle_contribution = 0;
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
        game.battle.current_ecaz_or_ally_fighter = if self.passed { Some(game.get_ally(Some(Faction::Ecaz))) } else { Some(Some(Faction::Ecaz)) };
        game.enter(Phase::BattlePhase, true);
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
