use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct IxianSwappedCardOnBid {
    #[canon(name = "_cardId")]
    pub card_id: i32,
    // ---- inherited from PassableGameEvent ----
    pub passed: bool,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for IxianSwappedCardOnBid {
    fn default() -> Self {
        IxianSwappedCardOnBid {
            card_id: Default::default(),
            passed: Default::default(),
            base: Default::default(),
        }
    }
}

impl IxianSwappedCardOnBid {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn card(&self) -> Option<TreacheryCard> {
        TreacheryCardManager::get(self.card_id)
    }

    pub fn set_card(&mut self, value: Option<TreacheryCard>) {
        let value = value.and_then(crate::compat::cs::CsNull::opt);
        self.card_id = TreacheryCardManager::get_id(value);
    }

    pub fn validate(&self, _game: &Game) -> Option<Message> {
        None
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) {
        if !self.passed && self.card().is_some() {
            let card = self.card().nre();
            game.ixian_swapped_card_on_bid = true;
            let me = self.player();
            crate::game::remove_card(&mut game.player_mut(me).treachery_cards, Some(card));
            let drawn = game.cards_on_auction.as_mut().nre().draw();
            game.player_mut(me).treachery_cards.push(drawn);

            // The lazy Where re-runs HasBiddingPrescience per player; UnregisterKnown does not change it.
            let players: Vec<PlayerIndex> = game.player_indices().filter(|p| !game.has_bidding_prescience(*p)).collect();
            for p in players {
                let my_cards = game.player(me).treachery_cards.clone();
                Game::unregister_known_player_i_enumerable_treachery_card(game, p, &my_cards);
            }

            game.cards_on_auction.as_mut().nre().put_on_top(card);
            game.register_known_faction_treachery_card(Faction::Ixian, card);
            game.stone(Milestone::CardOnBidSwapped);
            self.log(game);
        }

        game.continue_after_ixian_card_swap_decision();
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

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) {
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
