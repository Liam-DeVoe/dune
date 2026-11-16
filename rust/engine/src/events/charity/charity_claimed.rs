use crate::prelude::*;

/// A player with at most 1 spice claims CHOAM charity.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct CharityClaimed {}

impl CharityClaimed {
    pub fn can_be_played(game: &Game, player: PlayerId) -> bool {
        i32::from(game.player(player).resources) <= 1
            && !game.turn.has_acted_or_passed.contains_opt(game.player(player).faction)
            && !game.charity_is_cancelled()
    }
}

impl EventRules for CharityClaimed {
    fn validate(&self, game: &Game, by: By) -> Result<(), Rejection> {
        if i32::from(game.player(by.player()).resources) > 1 {
            return Err(Rejection(express!("You cannot claim charity")));
        }

        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        game.claim_charity(by.player());
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        express!(by.faction, " claim charity")
    }
}
