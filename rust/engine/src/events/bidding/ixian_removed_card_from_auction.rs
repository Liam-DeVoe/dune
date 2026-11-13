use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct IxianRemovedCardFromAuction {
    pub put_on_top: bool,
    #[canon(name = "_cardId")]
    pub card_id: i32,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for IxianRemovedCardFromAuction {
    fn default() -> Self {
        IxianRemovedCardFromAuction {
            put_on_top: Default::default(),
            card_id: Default::default(),
            base: Default::default(),
        }
    }
}

impl IxianRemovedCardFromAuction {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn card(&self) -> Option<TreacheryCard> {
        TreacheryCardManager::get(self.card_id)
    }

    pub fn set_card(&mut self, value: Option<TreacheryCard>) {
        self.card_id = TreacheryCardManager::get_id(value);
    }

    pub fn validate(&self, _game: &Game) -> Option<Message> {
        None
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        let card = self.card();
        crate::game::remove_card(&mut game.cards_on_auction.as_mut().required("cards on auction")?.items, card);
        // C# puts a null card on the deck when the card id is unknown.
        let card = card.required("card")?;

        if self.put_on_top {
            game.treachery_deck.as_mut().required("treachery deck")?.put_on_top(card);
        } else {
            game.treachery_deck.as_mut().required("treachery deck")?.put_on_bottom(card);
        }

        game.register_known_faction_treachery_card(Faction::Ixian, card);
        game.cards_on_auction.as_mut().required("cards on auction")?.shuffle();
        game.stone(Milestone::Shuffled);
        self.log(game);

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

    pub fn get_message(&self, _game: &Game) -> Message {
        if self.put_on_top {
            return express!(self.initiator(), " put a card on top of the Treachery Card deck");
        }
        express!(self.initiator(), " put a card at the bottom of the Treachery Card deck")
    }

}

impl GameEventMethods for IxianRemovedCardFromAuction {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::IxianRemovedCardFromAuction
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::IxianRemovedCardFromAuction(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        IxianRemovedCardFromAuction::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        IxianRemovedCardFromAuction::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        IxianRemovedCardFromAuction::get_message(self, game)
    }

}
