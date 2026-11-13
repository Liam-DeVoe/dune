use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct ChoamEconomics {
    pub status: ChoamEconomicsStatus,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    pub base: GameEventBase,
}

impl Default for ChoamEconomics {
    fn default() -> Self {
        ChoamEconomics {
            status: Default::default(),
            base: Default::default(),
        }
    }
}

impl ChoamEconomics {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn validate(&self, _game: &Game) -> Option<Message> {
        None
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        game.economics_status = self.status;
        game.stone(Milestone::Economics);
        self.log(game);
        Ok(())
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!(self.initiator(), " play Inflation: ", self.status)
    }
}

impl GameEventMethods for ChoamEconomics {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::ChoamEconomics
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::ChoamEconomics(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        ChoamEconomics::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        ChoamEconomics::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        ChoamEconomics::get_message(self, game)
    }

}
