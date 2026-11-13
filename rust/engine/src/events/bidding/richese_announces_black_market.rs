use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct RicheseAnnouncesBlackMarket {
    pub card_id: i32,
    pub auction_type: AuctionType,
    pub direction: i32,
    // ---- inherited from PassableGameEvent ----
    pub passed: bool,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    pub base: GameEventBase,
}

impl Default for RicheseAnnouncesBlackMarket {
    fn default() -> Self {
        RicheseAnnouncesBlackMarket {
            card_id: Default::default(),
            auction_type: Default::default(),
            direction: Default::default(),
            passed: Default::default(),
            base: Default::default(),
        }
    }
}

impl RicheseAnnouncesBlackMarket {
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

    pub fn validate(&self, game: &Game) -> Option<Message> {
        if self.passed {
            return None;
        }

        if self.auction_type != AuctionType::BlackMarketNormal && self.auction_type != AuctionType::BlackMarketOnceAround && self.auction_type != AuctionType::BlackMarketSilent {
            return Some(express!("Invalid auction type"));
        }
        if !RicheseAnnouncesBlackMarket::valid_cards(game, self.player()).iter().any(|c| Some(*c) == self.card()) {
            return Some(express!("Invalid card"));
        }

        None
    }

    pub fn valid_cards(game: &Game, p: PlayerIndex) -> Vec<TreacheryCard> {
        game.player(p).treachery_cards.clone()
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        self.log(game);

        if !self.passed {
            game.enter(Phase::BlackMarketBidding, true);
            let me = self.player();
            let card = self.card();
            crate::game::remove_card(&mut game.player_mut(me).treachery_cards, card);
            // C# puts a null card on the deck when executed without validation.
            let card = card.required("card")?;
            game.cards_on_auction.as_mut().required("cards on auction")?.put_on_top(card);
            game.register_known_faction_treachery_card(self.initiator(), card);
            game.bids.clear();
            game.current_bid = None;
            game.start_bid_sequence_and_auction_type(self.auction_type, self.player_opt(), self.direction);
        } else {
            game.enter_richese_bidding();
        }
        Ok(())
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        let mut direction_text = "";
        if self.auction_type == AuctionType::BlackMarketOnceAround {
            if self.direction == 1 {
                direction_text = " (counter-clockwise)";
            } else {
                direction_text = " (clockwise)";
            }
        }

        if !self.passed {
            return express!(self.initiator(), " put a card on the black market by ", self.auction_type, " auction", direction_text);
        }
        express!(self.initiator(), " don't put a card on the black market")
    }

}

impl GameEventMethods for RicheseAnnouncesBlackMarket {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::RicheseAnnouncesBlackMarket
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::RicheseAnnouncesBlackMarket(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        RicheseAnnouncesBlackMarket::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        RicheseAnnouncesBlackMarket::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        RicheseAnnouncesBlackMarket::get_message(self, game)
    }

}

impl PassableGameEventMethods for RicheseAnnouncesBlackMarket {
    fn field_passed(&self) -> &bool {
        &self.passed
    }

}
