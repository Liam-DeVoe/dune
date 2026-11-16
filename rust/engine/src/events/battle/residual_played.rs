use crate::prelude::*;

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct ResidualPlayed {}

impl ResidualPlayed {
    pub fn may_play(g: &Game, p: PlayerId) -> bool {
        g.current_battle().expect("current battle").is_aggressor_or_defender(g, p)
            && g.player(p).treachery_cards.iter().any(|c| c.type_() == TreacheryCardType::Residual)
    }
}

impl EventRules for ResidualPlayed {
    fn validate(&self, _game: &Game, _by: By) -> Result<(), Rejection> {
        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        game.discard(by.player(), TreacheryCardType::Residual)?;
        let battle = game.current_battle().cloned().required("current battle")?;
        let opponent = battle.opponent_of_faction(game, by.faction).required("opponent of faction")?;
        let candidates: Vec<Hero> = game
            .player(opponent)
            .leaders
            .iter()
            .map(|l| Hero::Leader(*l))
            .filter(|l| game.is_alive(*l) && game.can_join_current_battle(*l))
            .collect();
        let mut leaders_to_kill = Deck::new(candidates);
        leaders_to_kill.shuffle(&mut game.rng);

        if !leaders_to_kill.is_empty() {
            let to_kill = leaders_to_kill.try_draw()?;
            let opponent_plan = battle.plan_of(game, Some(opponent));
            if let Some(opponent_plan) = opponent_plan {
                if opponent_plan.hero == Some(to_kill) {
                    game.revoke_plan(&opponent_plan);
                }
            }
            game.kill_hero(to_kill);
            game.log(parts![TreacheryCardType::Residual, " kills ", to_kill]);
        } else {
            let faction = game.player(opponent).faction;
            game.log(parts![faction, " have no available leaders to kill"]);
        }
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        express!(by.faction, " use ", TreacheryCardType::Residual, " to kill a random opponent leader")
    }
}
