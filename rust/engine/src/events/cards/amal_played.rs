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
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
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
            // (int)Math.Ceiling(0.5 * p.Resources): double arithmetic.
            let resources_paid = (0.5f64 * game.player(p).resources as f64).ceil() as i32;
            let pl = game.player_mut(p);
            pl.resources -= resources_paid;
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
