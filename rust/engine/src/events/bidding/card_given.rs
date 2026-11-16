use crate::prelude::*;

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct CardGiven {
    #[serde(skip)]
    pub passed: bool,
}

impl CardGiven {
    pub fn is_applicable(game: &Game, p: PlayerId) -> bool {
        game.player(p).has(game.bidding.card_that_must_be_kept_or_given_to_ally)
    }
}

impl EventRules for CardGiven {
    fn validate(&self, _game: &Game, _by: By) -> Result<(), Rejection> {
        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        let m = self.describe(game, by);
        game.log_event(m);

        if !self.passed && game.bidding.card_that_must_be_kept_or_given_to_ally.is_some() && game.player(by.player()).allied_player(game).is_some() {
            let card = game.bidding.card_that_must_be_kept_or_given_to_ally;
            let me = by.player();
            crate::game::remove_card(&mut game.player_mut(me).treachery_cards, card);
            let ally = game.player(me).allied_player(game).required("allied player")?;
            game.player_mut(ally).treachery_cards.push(card.required("card")?);
        }

        game.bidding.card_that_must_be_kept_or_given_to_ally = None;
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        if self.passed {
            return express!(by.faction, " don't give the card to their ally");
        }
        express!(by.faction, " give the card to their ally")
    }
}
