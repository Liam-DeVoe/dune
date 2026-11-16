use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct AmalPlayed {
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    pub base: GameEventBase,
}

impl Default for AmalPlayed {
    fn default() -> Self {
        AmalPlayed {
            base: Default::default(),
        }
    }
}

impl AmalPlayed {
    pub fn new(game: &Game, initiator: impl Into<Option<Faction>>) -> Self {
        let e = Self::default();
        e.initialize(game, initiator.into());
        e
    }

    pub fn validate(&self, _game: &Game) -> Option<Message> {
        None
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        game.discard(self.player(), TreacheryCardType::Amal)?;
        self.log(game);

        for i in 0..game.players.len() {
            let p = game.player_at(i);
            // Half, rounded up.
            let resources_paid = game.player(p).resources.div_ceil(2);
            let pl = game.player_mut(p);
            pl.resources -= resources_paid;
            let resources_paid = i32::from(resources_paid);
            let faction = game.player(p).faction;
            self.log_objects(game, vec![faction.into(), " lose ".into(), Payment::of(resources_paid).into()]);
        }

        game.stone(Milestone::Amal);
        Ok(())
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!(self.initiator(), " perform ", TreacheryCardType::Amal)
    }
}

impl GameEventMethods for AmalPlayed {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::AmalPlayed
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::AmalPlayed(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        AmalPlayed::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        AmalPlayed::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        AmalPlayed::get_message(self, game)
    }

}
