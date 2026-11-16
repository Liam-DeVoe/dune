use crate::prelude::*;

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct KarmaRevivalPrevention {
    pub target: Option<Faction>,
}

impl KarmaRevivalPrevention {
    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn get_valid_targets(g: &Game, p: PlayerId) -> impl Iterator<Item = Faction> {
        g.players_other_than(p)
    }
}

impl EventRules for KarmaRevivalPrevention {
    fn validate(&self, game: &Game, by: By) -> Result<(), Rejection> {
        // PlayersOtherThan(null) only dereferences the player once there is a player to compare.
        if !Self::get_valid_targets(game, by.player()).any(|v| Some(v) == self.target) {
            return Err(Rejection(express!("Invalid target")));
        }

        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        game.turn.revivals.current_karma_revival_prevention = Some(RevivalPrevention { target: self.target });
        let me = by.player();
        let card = Karma::valid_karma_cards(game, me).into_iter().next();
        game.play_karma(me, card, vec!["prevent revival by ".into(), self.target.into()]);
        game.player_mut(me).special_karma_power_used = true;
        game.stone(Milestone::Karma);
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        express!("Using ", TreacheryCardType::Karma, ", ", by.faction, " prevent revival by ", self.target)
    }
}
