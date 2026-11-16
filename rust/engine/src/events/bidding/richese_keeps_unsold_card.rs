use crate::prelude::*;

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct RicheseKeepsUnsoldCard {
    #[serde(skip)]
    pub passed: bool,
}

impl EventRules for RicheseKeepsUnsoldCard {
    fn validate(&self, _game: &Game, _by: By) -> Result<(), Rejection> {
        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        let m = self.describe(game, by);
        game.log_event(m);
        let card = game.in_bidding_mut().cards_on_auction.as_mut().required("cards on auction")?.try_draw()?;
        game.register_won_card_as_known(card);

        if !self.passed {
            let me = by.player();
            game.player_mut(me).treachery_cards.push(card);
            game.log_to(by.required_faction()?, vec!["You get: ".into(), card.into()]);
            game.finish_bid(by.player, None, card, false);
        } else {
            game.decks.removed_treachery_cards.push(card);
            game.finish_bid(None, None, card, false);
        }
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        if !self.passed {
            return express!(by.faction, " keep the card no faction bid on");
        }
        express!(by.faction, " remove the card no faction bid on from the game")
    }
}
