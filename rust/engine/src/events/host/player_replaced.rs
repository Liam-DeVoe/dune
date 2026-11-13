use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct PlayerReplaced {
    pub to_replace: Faction,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    pub base: GameEventBase,
}

impl Default for PlayerReplaced {
    fn default() -> Self {
        PlayerReplaced {
            to_replace: Default::default(),
            base: Default::default(),
        }
    }
}

impl PlayerReplaced {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn validate(&self, _game: &Game) -> Option<Message> {
        None
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn valid_targets(g: &Game) -> impl Iterator<Item = Faction> {
        g.players.iter().map(move |p| p.faction)
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        let player = self.get_player(game, Some(self.to_replace));
        // IsBot(null) reads p.Seat: NullReferenceException.
        let is_bot = game.is_bot(player.required("player")?);
        let message = express!(self.to_replace, " will now be played by a ", if is_bot { "Bot" } else { "Human" })
            .with_log(AsideLog::aside());
        self.log_objects(game, vec![message.into()]);
        Ok(())
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!(self.to_replace, " player has been replaced")
    }

}

impl GameEventMethods for PlayerReplaced {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::PlayerReplaced
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::PlayerReplaced(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        PlayerReplaced::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        PlayerReplaced::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        PlayerReplaced::get_message(self, game)
    }

}
