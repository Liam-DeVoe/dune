use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct Captured {
    // ---- inherited from PassableGameEvent ----
    pub passed: bool,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for Captured {
    fn default() -> Self {
        Captured {
            passed: Default::default(),
            base: Default::default(),
        }
    }
}

impl Captured {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn validate(&self, _game: &Game) -> Option<Message> {
        None
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) {
        self.log(game);

        if !self.passed {
            if game.prevented(FactionAdvantage::HarkonnenCaptureLeader) {
                game.log_prevention_by_karma(FactionAdvantage::HarkonnenCaptureLeader);
            } else {
                game.capture_leader();
            }
        }

        game.enter(Phase::BattleConclusion, true);
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        if self.passed {
            return express!(self.initiator(), " won't capture or kill a leader");
        }
        express!(self.initiator(), " will capture or kill a random leader...")
    }

}

impl GameEventMethods for Captured {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::Captured
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::Captured(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        Captured::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) {
        Captured::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        Captured::get_message(self, game)
    }

}

impl PassableGameEventMethods for Captured {
    fn field_passed(&self) -> &bool {
        &self.passed
    }

}
