use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct RicheseSpecifiesAuction {
    pub auction_type: Option<AuctionType>,
    pub direction: i32,
    pub card_id: i32,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    pub base: GameEventBase,
}

impl Default for RicheseSpecifiesAuction {
    fn default() -> Self {
        RicheseSpecifiesAuction {
            auction_type: Default::default(),
            direction: Default::default(),
            card_id: Default::default(),
            base: Default::default(),
        }
    }
}

impl RicheseSpecifiesAuction {
    pub fn new(game: &Game, initiator: impl Into<Option<Faction>>) -> Self {
        let e = Self::default();
        e.initialize(game, initiator.into());
        e
    }

    pub fn card(&self) -> Option<TreacheryCard> {
        TreacheryCardManager::get(self.card_id)
    }

    pub fn set_card(&mut self, value: Option<TreacheryCard>) {
        self.card_id = TreacheryCardManager::get_id(value);
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        if !game.bidding.richese_occupier_specified_card && !RicheseSpecifiesAuction::valid_cards(game).iter().any(|c| Some(*c) == self.card()) {
            return Some(express!("Invalid card"));
        }

        if self.auction_type != Some(AuctionType::RicheseOnceAround) && self.auction_type != Some(AuctionType::RicheseSilent) {
            return Some(express!("Invalid auction type"));
        }

        None
    }

    pub fn valid_cards(g: &Game) -> Vec<TreacheryCard> {
        g.decks.richese_cache.clone()
    }

    pub fn may_specify_card(g: &Game, p: PlayerIndex) -> bool {
        let occupier_of_richese_homeworld = g.occupier_of(World::Richese);
        (occupier_of_richese_homeworld.is_none() && g.player(p).faction == Some(Faction::Richese))
            || (occupier_of_richese_homeworld.is_some() && !g.bidding.richese_occupier_specified_card && g.player(occupier_of_richese_homeworld.expect("occupier of richese homeworld")).is(g.player(p).faction))
    }

    pub fn may_specify_type_of_auction(g: &Game, p: PlayerIndex) -> bool {
        let occupier_of_richese_homeworld = g.occupier_of(World::Richese);
        (occupier_of_richese_homeworld.is_none() && g.player(p).faction == Some(Faction::Richese))
            || (occupier_of_richese_homeworld.is_some() && g.bidding.richese_occupier_specified_card && g.player(p).faction == Some(Faction::Richese))
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        if !game.bidding.richese_occupier_specified_card {
            game.bidding.richese_auction_should_still_happen = false;
            let card = self.card().required("card")?;
            game.bidding.cards_on_auction.as_mut().required("cards on auction")?.put_on_top(card);
            crate::game::remove_card(&mut game.decks.richese_cache, Some(card));
            game.drop_copy_in_treachery_deck(card);
            game.register_known(card);

            let occupier_of_richese_homeworld = game.occupier_of(World::Richese);
            if occupier_of_richese_homeworld.is_none() {
                self.log(game);
                game.start_bid_sequence_and_auction_type(self.auction_type, self.player_opt(), self.direction);
                game.start_bidding_round();
            } else {
                self.log_objects(game, vec![self.initiator().into(), " put ".into(), card.into(), " on auction".into()]);
                game.bidding.richese_occupier_specified_card = true;
            }
        } else {
            let mut direction_text = "";
            if self.auction_type == Some(AuctionType::RicheseOnceAround) {
                if self.direction == 1 {
                    direction_text = " (counter-clockwise)";
                } else {
                    direction_text = " (clockwise)";
                }
            }

            self.log_objects(game, vec![self.initiator().into(), " select a ".into(), self.auction_type.into(), " auction".into(), direction_text.into()]);

            game.start_bid_sequence_and_auction_type(self.auction_type, self.player_opt(), self.direction);
            game.start_bidding_round();
        }
        Ok(())
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        let mut direction_text = "";
        if self.auction_type == Some(AuctionType::RicheseOnceAround) {
            if self.direction == 1 {
                direction_text = " (counter-clockwise)";
            } else {
                direction_text = " (clockwise)";
            }
        }

        express!(self.initiator(), " put a ", Faction::Richese, " card on ", self.auction_type, " auction", direction_text)
    }

}

impl GameEventMethods for RicheseSpecifiesAuction {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::RicheseSpecifiesAuction
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::RicheseSpecifiesAuction(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        RicheseSpecifiesAuction::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        RicheseSpecifiesAuction::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        RicheseSpecifiesAuction::get_message(self, game)
    }

}
