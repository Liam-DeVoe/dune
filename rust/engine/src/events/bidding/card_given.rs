use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct CardGiven {
    // ---- inherited from PassableGameEvent ----
    pub passed: bool,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for CardGiven {
    fn default() -> Self {
        CardGiven {
            passed: Default::default(),
            base: Default::default(),
        }
    }
}

impl CardGiven {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn validate(&self, _game: &Game) -> Option<Message> {
        None
    }

    pub fn is_applicable(game: &Game, p: PlayerIndex) -> bool {
        game.player(p).has(game.card_that_must_be_kept_or_given_to_ally)
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) {
        self.log(game);

        if !self.passed && game.card_that_must_be_kept_or_given_to_ally.is_some() && game.player(self.player()).allied_player(game).is_some() {
            let card = game.card_that_must_be_kept_or_given_to_ally;
            let me = self.player();
            crate::game::remove_card(&mut game.player_mut(me).treachery_cards, card);
            let ally = game.player(me).allied_player(game).nre();
            game.player_mut(ally).treachery_cards.push(card.or_null());
        }

        game.card_that_must_be_kept_or_given_to_ally = None;
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        if self.passed {
            return express!(self.initiator(), " don't give the card to their ally");
        }
        express!(self.initiator(), " give the card to their ally")
    }

}

impl GameEventMethods for CardGiven {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::CardGiven
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::CardGiven(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        CardGiven::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) {
        CardGiven::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        CardGiven::get_message(self, game)
    }

}

impl PassableGameEventMethods for CardGiven {
    fn field_passed(&self) -> &bool {
        &self.passed
    }

}
