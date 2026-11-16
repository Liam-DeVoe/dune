use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct GuildDelay {
    /// The fields of `Event` (Initiator, Time, Game, Player).
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
    pub fn new(game: &Game, initiator: impl Into<Option<Faction>>) -> Self {
        let e = Self::default();
        e.initialize(game, initiator.into());
        e
    }

    pub fn validate(&self, _game: &Game) -> Option<Message> {
        None
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        game.ship_move.beginning_of_shipment_and_move_phase = false;
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

    fn event_type(&self) -> EventType {
        EventType::GuildDelay
    }

    fn into_event(self: Rc<Self>) -> Event {
        Event::GuildDelay(self)
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
