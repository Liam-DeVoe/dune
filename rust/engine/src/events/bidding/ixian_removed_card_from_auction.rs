use crate::prelude::*;

#[derive(Clone, Debug, PartialEq)]
pub struct IxianRemovedCardFromAuction {
    pub put_on_top: bool,
    pub card: Option<TreacheryCard>,
}

impl Default for IxianRemovedCardFromAuction {
    fn default() -> Self {
        IxianRemovedCardFromAuction { put_on_top: Default::default(), card: TreacheryCardManager::get(0) }
    }
}

impl EventRules for IxianRemovedCardFromAuction {
    fn validate(&self, _game: &Game, _by: By) -> Result<(), Rejection> {
        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        let card = self.card;
        crate::game::remove_card(&mut game.bidding.cards_on_auction.as_mut().required("cards on auction")?.items, card);
        let card = card.required("card")?;

        if self.put_on_top {
            game.decks.treachery_deck.as_mut().required("treachery deck")?.put_on_top(card);
        } else {
            game.decks.treachery_deck.as_mut().required("treachery deck")?.put_on_bottom(card);
        }

        game.register_known_by_faction(Faction::Ixian, card);
        game.bidding.cards_on_auction.as_mut().required("cards on auction")?.shuffle(&mut game.rng);
        game.stone(Milestone::Shuffled);
        let m = self.describe(game, by);
        game.log_event(m);

        if game.ixian_may_swap_card_on_bid() {
            if !game.prevented(FactionAdvantage::IxianSwappingCard) {
                game.enter(Phase::IxianSwappingCard, true);
            } else {
                game.log_prevention_by_karma(FactionAdvantage::IxianSwappingCard);
                if !game.applicable(Rule::FullPhaseKarma) {
                    game.allow(FactionAdvantage::IxianSwappingCard);
                }
                game.start_bidding_round();
            }
        } else {
            game.start_bidding_round();
        }
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        if self.put_on_top {
            return express!(by.faction, " put a card on top of the Treachery Card deck");
        }
        express!(by.faction, " put a card at the bottom of the Treachery Card deck")
    }
}
