use crate::prelude::*;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct RicheseAnnouncesBlackMarket {
    pub card: Option<TreacheryCard>,
    pub auction_type: Option<AuctionType>,
    pub direction: i32,
    #[serde(skip)]
    pub passed: bool,
}

impl Default for RicheseAnnouncesBlackMarket {
    fn default() -> Self {
        RicheseAnnouncesBlackMarket {
            card: TreacheryCardManager::get(0),
            auction_type: Default::default(),
            direction: Default::default(),
            passed: Default::default(),
        }
    }
}

impl RicheseAnnouncesBlackMarket {
    pub fn valid_cards(game: &Game, p: PlayerId) -> Vec<TreacheryCard> {
        game.player(p).treachery_cards.clone()
    }
}

impl EventRules for RicheseAnnouncesBlackMarket {
    fn validate(&self, game: &Game, by: By) -> Result<(), Rejection> {
        if self.passed {
            return Ok(());
        }

        if self.auction_type != Some(AuctionType::BlackMarketNormal) && self.auction_type != Some(AuctionType::BlackMarketOnceAround) && self.auction_type != Some(AuctionType::BlackMarketSilent) {
            return Err(Rejection(express!("Invalid auction type")));
        }
        if !RicheseAnnouncesBlackMarket::valid_cards(game, by.player()).iter().any(|c| Some(*c) == self.card) {
            return Err(Rejection(express!("Invalid card")));
        }

        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        let m = self.describe(game, by);
        game.log_event(m);

        if !self.passed {
            game.enter(Phase::BlackMarketBidding, true);
            let me = by.player();
            let card = self.card;
            crate::game::remove_card(&mut game.player_mut(me).treachery_cards, card);
            let card = card.required("card")?;
            game.bidding.cards_on_auction.as_mut().required("cards on auction")?.put_on_top(card);
            game.register_known_by_faction(by.required_faction()?, card);
            game.bidding.bids.clear();
            game.bidding.current_bid = None;
            game.start_bid_sequence_and_auction_type(self.auction_type, by.player, self.direction);
        } else {
            game.enter_richese_bidding();
        }
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        let mut direction_text = "";
        if self.auction_type == Some(AuctionType::BlackMarketOnceAround) {
            if self.direction == 1 {
                direction_text = " (counter-clockwise)";
            } else {
                direction_text = " (clockwise)";
            }
        }

        if !self.passed {
            return express!(by.faction, " put a card on the black market by ", self.auction_type, " auction", direction_text);
        }
        express!(by.faction, " don't put a card on the black market")
    }
}
