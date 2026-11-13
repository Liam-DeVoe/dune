use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct EmperorDiscarded {
    pub card_id: i32,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    pub base: GameEventBase,
}

impl Default for EmperorDiscarded {
    fn default() -> Self {
        EmperorDiscarded {
            card_id: Default::default(),
            base: Default::default(),
        }
    }
}

impl EmperorDiscarded {
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
        if self.card().is_none() {
            return Some(express!("Choose a card to discard"));
        }
        if !game.player(self.player()).has(self.card()) {
            return Some(express!("Invalid card"));
        }

        None
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        self.log(game);
        let me = self.player();
        {
            let p = game.player_mut(me);
            p.resources -= 2;
        }
        game.discard_player_treachery_card_boolean(Some(me), self.card(), true);
        Ok(())
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!(self.initiator(), " pay ", Payment::of(2), " to discard ", self.card())
    }

}

impl GameEventMethods for EmperorDiscarded {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::EmperorDiscarded
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::EmperorDiscarded(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        EmperorDiscarded::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        EmperorDiscarded::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        EmperorDiscarded::get_message(self, game)
    }

}
