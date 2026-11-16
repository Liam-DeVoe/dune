use crate::prelude::*;

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct RicheseAnnouncesAuction {
    pub first: bool,
}

impl EventRules for RicheseAnnouncesAuction {
    fn validate(&self, _game: &Game, _by: By) -> Result<(), Rejection> {
        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        game.bidding.richese_occupier_specified_card = false;
        let m = self.describe(game, by);
        game.log_event(m);

        game.bidding.number_of_cards_on_auction -= 1;

        if !self.first && game.bidding.number_of_cards_on_auction > 0 {
            game.bidding.richese_auction_should_still_happen = true;
        }

        if game.bidding.number_of_cards_on_auction == 0 {
            game.bidding.regular_bidding_is_done = true;
        }

        let condition = self.first || game.bidding.number_of_cards_on_auction == 0;
        game.enter_or_else(condition, Phase::RicheseSpecifyingAuction, Game::start_regular_bidding);
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        express!(by.faction, " will auction a card from their cache ", if self.first { "first" } else { "last" })
    }
}
