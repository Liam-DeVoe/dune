use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct ThumperPlayed {
    /// The fields of `Event` (Initiator, Time, Game, Player).
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
    pub fn new(game: &Game, initiator: impl Into<Option<Faction>>) -> Self {
        let e = Self::default();
        e.initialize(game, initiator.into());
        e
    }

    pub fn validate(&self, _game: &Game) -> Option<Message> {
        None
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        game.discard(self.player(), TreacheryCardType::Thumper)?;
        self.log(game);
        game.stone(Milestone::Thumper);
        game.blow.thumper_called_shai_hulud = game.turn.current_turn > Turn::FIRST;
        game.enter_blow_a()?;
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

    fn event_type(&self) -> EventType {
        EventType::ThumperPlayed
    }

    fn into_event(self: Rc<Self>) -> Event {
        Event::ThumperPlayed(self)
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
