use crate::prelude::*;

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct KarmaEcazDial {}

impl EventRules for KarmaEcazDial {
    fn validate(&self, _game: &Game, _by: By) -> Result<(), Rejection> {
        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        let me = by.player();
        let karama = Karma::valid_karma_cards(game, me).into_iter().next();
        game.player_mut(me).special_karma_power_used = true;
        game.stone(Milestone::Karma);
        let battle = game.battle.current_battle.clone().required("current battle")?;
        let my_leader = battle.plan_of_faction(game, by.faction).required("plan of faction")?.hero;
        let opponent_leader = battle.plan_of_opponent(game, Some(me)).required("plan of opponent")?.hero;
        if let (Some(my_leader), Some(opponent_leader)) = (my_leader, opponent_leader) {
            game.battle.ecaz_karma_bonus =
                (my_leader.value() - opponent_leader.value_in_combat_against(Some(my_leader))).abs();
        }
        let bonus = game.battle.ecaz_karma_bonus;
        game.play_karma(me, karama, vec!["add ".into(), bonus.into(), " to their dial".into()]);
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        express!("Using ", TreacheryCardType::Karma, ", ", by.faction, " add the difference between leader discs to their dial")
    }
}
