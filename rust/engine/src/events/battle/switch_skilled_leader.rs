use crate::prelude::*;

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct SwitchedSkilledLeader {}

impl SwitchedSkilledLeader {
    pub fn switchable_leader(game: &Game, player: PlayerId) -> Option<Leader> {
        let faction = game.player(player).faction;
        game.player(player).leaders.iter().copied().find(|l| {
            game.is_skilled(Some(Hero::Leader(*l)))
                && !game.leaders.captured_leaders.contains_key(l)
                && (faction == Some(Faction::Ecaz) || l.hero_type() != HeroType::Vidal)
        })
    }

    pub fn can_be_played(game: &Game, player: PlayerId) -> bool {
        if game.turn.current_phase != Phase::BattlePhase {
            return false;
        }
        let Some(battle) = game.battle.current_battle.as_ref() else {
            return false;
        };
        battle.is_aggressor_or_defender(game, player)
            && battle.plan_of(game, Some(player)).is_none()
            && Self::switchable_leader(game, player).is_some()
    }
}

impl EventRules for SwitchedSkilledLeader {
    fn validate(&self, _game: &Game, _by: By) -> Result<(), Rejection> {
        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        let leader = Self::switchable_leader(game, by.player());
        let value = !game.is_in_front_of_shield(leader.map(Hero::Leader));
        let leader = match leader {
            Some(l) => l,
            None => game_error!("There is no skilled leader to switch"),
        };
        game.set_in_front_of_shield(leader, value);
        let hero = Some(Hero::Leader(leader));
        let skill = game.skill(hero);
        let in_front = game.is_in_front_of_shield(hero);
        game.log(vec![
                by.faction.into(),
                " place ".into(),
                skill.into(),
                " ".into(),
                leader.into(),
                if in_front { " in front of" } else { " behind" }.into(),
                " their shield".into(),
            ],
        );
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        express!(by.faction, " switch their skilled leader")
    }
}
