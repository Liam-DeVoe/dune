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
        game.in_bidding_mut().richese_occupier_specified_card = false;
        let m = self.describe(game, by);
        game.log_event(m);

        game.in_bidding_mut().number_of_cards_on_auction -= 1;

        if !self.first && game.in_bidding().number_of_cards_on_auction > 0 {
            game.in_bidding_mut().richese_auction_should_still_happen = true;
        }

        if game.in_bidding().number_of_cards_on_auction == 0 {
            game.in_bidding_mut().regular_bidding_is_done = true;
        }

        let condition = self.first || game.in_bidding().number_of_cards_on_auction == 0;
        if condition {
            game.enter_bidding(BiddingStep::RicheseSpecifyingAuction);
        } else {
            game.start_regular_bidding();
        }
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        express!(by.faction, " will auction a card from their cache ", if self.first { "first" } else { "last" })
    }
}
