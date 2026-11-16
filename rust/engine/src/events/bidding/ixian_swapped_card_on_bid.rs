use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct IxianSwappedCardOnBid {
    pub card: Option<TreacheryCard>,
    // ---- inherited from PassableGameEvent ----
    pub passed: bool,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    pub base: GameEventBase,
}

impl Default for IxianSwappedCardOnBid {
    fn default() -> Self {
        IxianSwappedCardOnBid {
            card: TreacheryCardManager::get(0),
            passed: Default::default(),
            base: Default::default(),
        }
    }
}

impl IxianSwappedCardOnBid {
    pub fn new(game: &Game, initiator: impl Into<Option<Faction>>) -> Self {
        let e = Self::default();
        e.initialize(game, initiator.into());
        e
    }

    pub fn validate(&self, _game: &Game) -> Option<Message> {
        None
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        if !self.passed && self.card.is_some() {
            let card = self.card.required("card")?;
            game.bidding.ixian_swapped_card_on_bid = true;
            let me = self.player();
            crate::game::remove_card(&mut game.player_mut(me).treachery_cards, Some(card));
            let drawn = game.bidding.cards_on_auction.as_mut().required("cards on auction")?.try_draw()?;
            game.player_mut(me).treachery_cards.push(drawn);

            // The lazy Where re-runs HasBiddingPrescience per player; UnregisterKnown does not change it.
            let players: Vec<PlayerIndex> = game.player_indices().filter(|p| !game.has_bidding_prescience(*p)).collect();
            for p in players {
                let my_cards = game.player(me).treachery_cards.clone();
                Game::unregister_all_known_by_player(game, p, &my_cards);
            }

            game.bidding.cards_on_auction.as_mut().required("cards on auction")?.put_on_top(card);
            game.register_known_by_faction(Faction::Ixian, card);
            game.stone(Milestone::CardOnBidSwapped);
            self.log(game);
        }

        game.continue_after_ixian_card_swap_decision();
        Ok(())
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        if !self.passed {
            return express!(self.initiator(), " swap a card from hand with the next card on bid");
        }
        express!(self.initiator(), " don't swap a card from hand with the next card on bid")
    }

}

impl GameEventMethods for IxianSwappedCardOnBid {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::IxianSwappedCardOnBid
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::IxianSwappedCardOnBid(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        IxianSwappedCardOnBid::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        IxianSwappedCardOnBid::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        IxianSwappedCardOnBid::get_message(self, game)
    }

}

impl PassableGameEventMethods for IxianSwappedCardOnBid {
    fn field_passed(&self) -> &bool {
        &self.passed
    }

}
