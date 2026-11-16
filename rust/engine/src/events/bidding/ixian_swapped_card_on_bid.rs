use crate::prelude::*;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct IxianSwappedCardOnBid {
    pub card: Option<TreacheryCard>,
    #[serde(skip)]
    pub passed: bool,
}

impl Default for IxianSwappedCardOnBid {
    fn default() -> Self {
        IxianSwappedCardOnBid { card: TreacheryCardManager::get(0), passed: Default::default() }
    }
}

impl EventRules for IxianSwappedCardOnBid {
    fn validate(&self, _game: &Game, _by: By) -> Result<(), Rejection> {
        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        if !self.passed && self.card.is_some() {
            let card = self.card.required("card")?;
            game.bidding.ixian_swapped_card_on_bid = true;
            let me = by.player();
            crate::game::remove_card(&mut game.player_mut(me).treachery_cards, Some(card));
            let drawn = game.bidding.cards_on_auction.as_mut().required("cards on auction")?.try_draw()?;
            game.player_mut(me).treachery_cards.push(drawn);

            // The lazy Where re-runs HasBiddingPrescience per player; UnregisterKnown does not change it.
            let players: Vec<PlayerId> = game.player_indices().filter(|p| !game.has_bidding_prescience(*p)).collect();
            for p in players {
                let my_cards = game.player(me).treachery_cards.clone();
                Game::unregister_all_known_by_player(game, p, &my_cards);
            }

            game.bidding.cards_on_auction.as_mut().required("cards on auction")?.put_on_top(card);
            game.register_known_by_faction(Faction::Ixian, card);
            game.stone(Milestone::CardOnBidSwapped);
            let m = self.describe(game, by);
            game.log_event(m);
        }

        game.continue_after_ixian_card_swap_decision();
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        if !self.passed {
            return express!(by.faction, " swap a card from hand with the next card on bid");
        }
        express!(by.faction, " don't swap a card from hand with the next card on bid")
    }
}
