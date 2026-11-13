use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct FaceDancerRevealed {
    // ---- inherited from PassableGameEvent ----
    pub passed: bool,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for FaceDancerRevealed {
    fn default() -> Self {
        FaceDancerRevealed {
            passed: Default::default(),
            base: Default::default(),
        }
    }
}

macro_rules! elems {
    ($($e:expr),* $(,)?) => { vec![$(ExpressionElement::from($e)),*] };
}

impl FaceDancerRevealed {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        if self.passed {
            return None;
        }
        if !FaceDanced::may_call_face_dancer_opt(game, self.player_opt()) {
            return Some(express!("You can't reveal a face dancer"));
        }
        None
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) {
        if !self.passed {
            // Game.WinnerHero.IsFaceDancer(f): read per element; null throws on the first one.
            let facedancer = game
                .player(self.player())
                .face_dancers
                .iter()
                .copied()
                .cs_first_or_default_where(|f| game.winner_hero().nre().is_face_dancer(*f));
            game.log(elems![self.initiator(), " reveal ", facedancer, " as one of their Face Dancers!"]);
            game.stone(Milestone::FaceDanced);
            game.enter(Phase::Facedancing, true);
        } else {
            game.log(elems![self.initiator(), " don't reveal a Face Dancer"]);
            game.finish_battle();
        }
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        if !self.passed {
            return express!(self.initiator(), " reveal a face dancer!");
        }
        express!(self.initiator(), " don't reveal a face dancer")
    }
}

impl GameEventMethods for FaceDancerRevealed {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::FaceDancerRevealed
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::FaceDancerRevealed(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        FaceDancerRevealed::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) {
        FaceDancerRevealed::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        FaceDancerRevealed::get_message(self, game)
    }

}

impl PassableGameEventMethods for FaceDancerRevealed {
    fn field_passed(&self) -> &bool {
        &self.passed
    }

}
