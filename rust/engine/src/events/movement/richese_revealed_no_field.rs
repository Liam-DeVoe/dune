use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct RicheseRevealedNoField {
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for RicheseRevealedNoField {
    fn default() -> Self {
        RicheseRevealedNoField {
            base: Default::default(),
        }
    }
}

impl RicheseRevealedNoField {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn validate(&self, _game: &Game) -> Option<Message> {
        None
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) {
        game.reveal_current_no_field(self.player(), None);
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!(self.initiator(), " reveal a ", FactionSpecialForce::Richese)
    }
}

impl GameEventMethods for RicheseRevealedNoField {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::RicheseRevealedNoField
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::RicheseRevealedNoField(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        RicheseRevealedNoField::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) {
        RicheseRevealedNoField::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        RicheseRevealedNoField::get_message(self, game)
    }

}
