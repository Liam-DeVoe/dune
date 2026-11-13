use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct FlightDiscoveryUsed {
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for FlightDiscoveryUsed {
    fn default() -> Self {
        FlightDiscoveryUsed {
            base: Default::default(),
        }
    }
}

impl FlightDiscoveryUsed {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn validate(&self, _game: &Game) -> Option<Message> {
        None
    }

    pub fn is_available(g: &Game, p: PlayerIndex) -> bool {
        g.owner_of_flight_discovery == Some(g.player(p).faction)
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) {
        self.log(game);
        game.owner_of_flight_discovery = Some(Faction::None);
        game.current_flight_discovery_used = Some(self.clone());
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        // C# parity: "discovert" is the C# text (hazards 11).
        express!(self.initiator(), " use the ", DiscoveryToken::Flight, " discovert token to gain movement speed")
    }

}

impl GameEventMethods for FlightDiscoveryUsed {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::FlightDiscoveryUsed
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::FlightDiscoveryUsed(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        FlightDiscoveryUsed::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) {
        FlightDiscoveryUsed::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        FlightDiscoveryUsed::get_message(self, game)
    }

}
