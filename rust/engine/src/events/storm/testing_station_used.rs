use crate::prelude::*;

/// `params object?[]` element list for `Log(...)`.
macro_rules! elems {
    ($($e:expr),* $(,)?) => { vec![$(ExpressionElement::from($e)),*] };
}

#[derive(Clone, Debug, Canon)]
pub struct TestingStationUsed {
    pub value_added: i32,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for TestingStationUsed {
    fn default() -> Self {
        TestingStationUsed {
            value_added: Default::default(),
            base: Default::default(),
        }
    }
}

impl TestingStationUsed {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn validate(&self, _game: &Game) -> Option<Message> {
        if self.value_added != -1 && self.value_added != 1 {
            return Some(express!("Invalid amount"));
        }

        None
    }

    pub fn can_be_played(g: &Game, p: PlayerIndex) -> bool {
        g.current_phase == Phase::MetheorAndStormSpell
            && g.player(p).occupies(g, Some(g.map.testing_station().location()))
            && !g.current_testing_station_used
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        game.stone(Milestone::WeatherControlled);
        game.current_testing_station_used = true;
        game.next_storm_moves = game.next_storm_moves.wrapping_add(self.value_added);
        let next = game.next_storm_moves;
        self.log_objects(
            game,
            elems![self.initiator(), " use ", DiscoveryToken::TestingStation, ": storm ", if self.value_added > 0 { "increases" } else { "weakens" }, " to ", next],
        );
        Ok(())
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!(self.initiator(), " use ", DiscoveryToken::TestingStation, " to influence Storm movement")
    }

}

impl GameEventMethods for TestingStationUsed {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::TestingStationUsed
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::TestingStationUsed(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        TestingStationUsed::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        TestingStationUsed::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        TestingStationUsed::get_message(self, game)
    }

}
