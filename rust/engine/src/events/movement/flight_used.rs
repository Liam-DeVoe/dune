use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct FlightUsed {
    pub move_three_territories: bool,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for FlightUsed {
    fn default() -> Self {
        FlightUsed {
            move_three_territories: Default::default(),
            base: Default::default(),
        }
    }
}

impl FlightUsed {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn extra_move(&self) -> bool {
        !self.move_three_territories
    }

    pub fn validate(&self, _game: &Game) -> Option<Message> {
        None
    }

    pub fn is_available(game: &Game, p: PlayerIndex) -> bool {
        game.player(p).treachery_cards.iter().any(|c| c.type_() == TreacheryCardType::Flight)
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        self.log(game);
        game.discard(self.player(), TreacheryCardType::Flight)?;
        game.current_flight_used = Some(self.clone());
        Ok(())
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        // C# parity: the doubled spaces are in the C# text (hazards 11).
        express!(
            self.initiator(),
            " use an ",
            TreacheryCardType::Flight,
            " to ",
            if self.extra_move() { " move an additional group of forces " } else { " gain movement speed" }
        )
    }

}

impl GameEventMethods for FlightUsed {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::FlightUsed
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::FlightUsed(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        FlightUsed::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        FlightUsed::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        FlightUsed::get_message(self, game)
    }

}
