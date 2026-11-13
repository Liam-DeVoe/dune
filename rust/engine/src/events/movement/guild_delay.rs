use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct GuildDelay {
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for GuildDelay {
    fn default() -> Self {
        GuildDelay {
            base: Default::default(),
        }
    }
}

impl GuildDelay {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn validate(&self, _game: &Game) -> Option<Message> {
        None
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        game.beginning_of_shipment_and_move_phase = false;
        let m = self.get_message(game).with_log(GuildDelayed::default());
        self.log_objects(game, vec![m.into()]);
        game.enter(Phase::NonGuildShip, true);
        Ok(())
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!(self.initiator(), " delay their turn")
    }

}

impl GameEventMethods for GuildDelay {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::GuildDelay
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::GuildDelay(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        GuildDelay::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        GuildDelay::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        GuildDelay::get_message(self, game)
    }

}
