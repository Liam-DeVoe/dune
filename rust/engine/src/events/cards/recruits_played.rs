use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct RecruitsPlayed {
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    pub base: GameEventBase,
}

impl Default for RecruitsPlayed {
    fn default() -> Self {
        RecruitsPlayed {
            base: Default::default(),
        }
    }
}

impl RecruitsPlayed {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn validate(&self, _game: &Game) -> Option<Message> {
        None
    }

    pub fn is_applicable(g: &Game, p: PlayerIndex) -> bool {
        g.current_phase == Phase::BeginningOfResurrection && g.player(p).has_treachery_card_type(TreacheryCardType::Recruits)
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        self.log(game);
        game.current_recruits_played = Some(self.clone());
        game.discard(self.player(), TreacheryCardType::Recruits)?;
        Ok(())
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!(self.initiator(), " use ", TreacheryCardType::Recruits, " to double free revivals and set revival limits to ", 7)
    }

}

impl GameEventMethods for RecruitsPlayed {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::RecruitsPlayed
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::RecruitsPlayed(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        RecruitsPlayed::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        RecruitsPlayed::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        RecruitsPlayed::get_message(self, game)
    }

}
