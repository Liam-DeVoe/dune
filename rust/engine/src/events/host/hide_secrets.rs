use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct HideSecrets {
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for HideSecrets {
    fn default() -> Self {
        HideSecrets {
            base: Default::default(),
        }
    }
}

impl HideSecrets {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn validate(&self, _game: &Game) -> Option<Message> {
        None
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        game.secrets_remain_hidden.push(self.initiator());
        Ok(())
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!(self.initiator(), " hide their secrets at end of game")
    }

}

impl GameEventMethods for HideSecrets {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::HideSecrets
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::HideSecrets(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        HideSecrets::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        HideSecrets::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        HideSecrets::get_message(self, game)
    }

}
