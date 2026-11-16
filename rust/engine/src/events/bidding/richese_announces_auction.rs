use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct RicheseAnnouncesAuction {
    pub first: bool,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    pub base: GameEventBase,
}

impl Default for RicheseAnnouncesAuction {
    fn default() -> Self {
        RicheseAnnouncesAuction {
            first: Default::default(),
            base: Default::default(),
        }
    }
}

impl RicheseAnnouncesAuction {
    pub fn new(game: &Game, initiator: impl Into<Option<Faction>>) -> Self {
        let e = Self::default();
        e.initialize(game, initiator.into());
        e
    }

    pub fn validate(&self, _game: &Game) -> Option<Message> {
        None
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        game.bidding.richese_occupier_specified_card = false;
        self.log(game);

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

    pub fn get_message(&self, _game: &Game) -> Message {
        express!(self.initiator(), " will auction a card from their cache ", if self.first { "first" } else { "last" })
    }

}

impl GameEventMethods for RicheseAnnouncesAuction {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::RicheseAnnouncesAuction
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::RicheseAnnouncesAuction(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        RicheseAnnouncesAuction::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        RicheseAnnouncesAuction::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        RicheseAnnouncesAuction::get_message(self, game)
    }

}
