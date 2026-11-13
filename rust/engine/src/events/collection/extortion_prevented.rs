use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct ExtortionPrevented {
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for ExtortionPrevented {
    fn default() -> Self {
        ExtortionPrevented {
            base: Default::default(),
        }
    }
}

impl ExtortionPrevented {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn validate(&self, _game: &Game) -> Option<Message> {
        None
    }

    pub fn can_be_played(g: &Game, p: PlayerIndex) -> bool {
        g.player(p).faction != Faction::Moritani
            && g.player(p).resources >= 3
            && g.extortion_to_be_returned
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        self.log(game);
        game.extortion_to_be_returned = false;
        let p = game.player_mut(self.player());
        p.resources -= 3;
        let moritani = self.get_player(game, Some(Faction::Moritani)).required("get player")?;
        let c = game.player_mut(moritani);
        c.resources += 3;
        Ok(())
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!(
            self.initiator(),
            " pay ",
            Payment::of(3),
            " to prevent ",
            Faction::Moritani,
            " from regaining ",
            TerrorType::Extortion
        )
    }
}

impl GameEventMethods for ExtortionPrevented {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::ExtortionPrevented
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::ExtortionPrevented(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        ExtortionPrevented::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        ExtortionPrevented::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        ExtortionPrevented::get_message(self, game)
    }

}
