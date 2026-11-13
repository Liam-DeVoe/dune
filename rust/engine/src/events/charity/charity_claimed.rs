use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct CharityClaimed {
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for CharityClaimed {
    fn default() -> Self {
        CharityClaimed {
            base: Default::default(),
        }
    }
}

impl CharityClaimed {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        let p = self.player();
        if game.player(p).resources > 1 {
            return Some(express!("You cannot claim charity"));
        }

        None
    }

    pub fn can_be_played(game: &Game, player: PlayerIndex) -> bool {
        game.player(player).resources <= 1
            && !game.has_acted_or_passed.contains(&game.player(player).faction)
            && !game.charity_is_cancelled()
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) {
        game.claim_charity(self.player());
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!(self.initiator(), " claim charity")
    }

}

impl GameEventMethods for CharityClaimed {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::CharityClaimed
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::CharityClaimed(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        CharityClaimed::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) {
        CharityClaimed::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        CharityClaimed::get_message(self, game)
    }

}
