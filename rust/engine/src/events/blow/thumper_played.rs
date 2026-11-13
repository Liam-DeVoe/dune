use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct ThumperPlayed {
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    pub base: GameEventBase,
}

impl Default for ThumperPlayed {
    fn default() -> Self {
        ThumperPlayed {
            base: Default::default(),
        }
    }
}

impl ThumperPlayed {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn validate(&self, _game: &Game) -> Option<Message> {
        None
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        game.discard(self.player(), TreacheryCardType::Thumper)?;
        self.log(game);
        game.stone(Milestone::Thumper);
        game.thumper_called_shai_hulud = game.current_turn > 1;
        game.enter_blow_a();
        Ok(())
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!(self.initiator(), " use a ", TreacheryCardType::Thumper, " to attract ", Concept::Monster)
    }

}

impl GameEventMethods for ThumperPlayed {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::ThumperPlayed
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::ThumperPlayed(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        ThumperPlayed::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        ThumperPlayed::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        ThumperPlayed::get_message(self, game)
    }

}
