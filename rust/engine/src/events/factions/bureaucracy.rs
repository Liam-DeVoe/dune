use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct Bureaucracy {
    // ---- inherited from PassableGameEvent ----
    pub passed: bool,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for Bureaucracy {
    fn default() -> Self {
        Bureaucracy {
            passed: Default::default(),
            base: Default::default(),
        }
    }
}

impl Bureaucracy {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn validate(&self, _game: &Game) -> Option<Message> {
        None
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        let m = self.get_dynamic_message(game);
        self.log_objects(game, vec![m.into()]);

        if !self.passed {
            game.stone(Milestone::Bureaucracy);
            game.bureaucrat_was_used_this_phase = true;
            let victim = game.get_player(Some(game.target_of_bureaucracy)).required("get player")?;
            let v = game.player_mut(victim);
            v.resources = v.resources.wrapping_sub(2);
            game.was_victim_of_bureaucracy = game.target_of_bureaucracy;
        }

        let phase = game.phase_before_bureaucrat_was_activated;
        game.enter(phase, true);
        game.target_of_bureaucracy = Faction::None;
        Ok(())
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        if self.passed {
            return express!(self.initiator(), " don't apply Bureaucracy");
        }
        express!(self.initiator(), " apply Bureaucracy")
    }

    pub fn get_dynamic_message(&self, game: &Game) -> Message {
        if self.passed {
            return express!(self.initiator(), " don't apply Bureaucracy");
        }
        express!(self.initiator(), " apply Bureaucracy → ", game.target_of_bureaucracy, " lose ", Payment::of(2))
    }

}

impl GameEventMethods for Bureaucracy {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::Bureaucracy
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::Bureaucracy(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        Bureaucracy::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        Bureaucracy::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        Bureaucracy::get_message(self, game)
    }

}

impl PassableGameEventMethods for Bureaucracy {
    fn field_passed(&self) -> &bool {
        &self.passed
    }

}
