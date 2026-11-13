use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct RicheseKeepsUnsoldCard {
    // ---- inherited from PassableGameEvent ----
    pub passed: bool,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for RicheseKeepsUnsoldCard {
    fn default() -> Self {
        RicheseKeepsUnsoldCard {
            passed: Default::default(),
            base: Default::default(),
        }
    }
}

impl RicheseKeepsUnsoldCard {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn validate(&self, _game: &Game) -> Option<Message> {
        None
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) {
        self.log(game);
        let card = game.cards_on_auction.as_mut().nre().draw();
        game.register_won_card_as_known(card);

        if !self.passed {
            let me = self.player();
            game.player_mut(me).treachery_cards.push(card);
            self.log_to(game, self.initiator(), vec!["You get: ".into(), card.into()]);
            game.finish_bid(self.player_opt(), None, card, false);
        } else {
            game.removed_treachery_cards.push(card);
            game.finish_bid(None, None, card, false);
        }
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        if !self.passed {
            return express!(self.initiator(), " keep the card no faction bid on");
        }
        express!(self.initiator(), " remove the card no faction bid on from the game")
    }

}

impl GameEventMethods for RicheseKeepsUnsoldCard {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::RicheseKeepsUnsoldCard
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::RicheseKeepsUnsoldCard(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        RicheseKeepsUnsoldCard::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) {
        RicheseKeepsUnsoldCard::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        RicheseKeepsUnsoldCard::get_message(self, game)
    }

}

impl PassableGameEventMethods for RicheseKeepsUnsoldCard {
    fn field_passed(&self) -> &bool {
        &self.passed
    }

}
