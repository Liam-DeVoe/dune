use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct PoisonToothCancelled {
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for PoisonToothCancelled {
    fn default() -> Self {
        PoisonToothCancelled {
            base: Default::default(),
        }
    }
}

impl PoisonToothCancelled {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn validate(&self, _game: &Game) -> Option<Message> {
        None
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) {
        game.poison_tooth_cancelled = true;
        self.log(game);
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!(self.initiator(), " don't use their ", TreacheryCardType::PoisonTooth)
    }
}

impl GameEventMethods for PoisonToothCancelled {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::PoisonToothCancelled
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::PoisonToothCancelled(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        PoisonToothCancelled::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) {
        PoisonToothCancelled::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        PoisonToothCancelled::get_message(self, game)
    }

}
