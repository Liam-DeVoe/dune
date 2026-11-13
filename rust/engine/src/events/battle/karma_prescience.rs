use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct KarmaPrescience {
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for KarmaPrescience {
    fn default() -> Self {
        KarmaPrescience {
            base: Default::default(),
        }
    }
}

impl KarmaPrescience {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn validate(&self, _game: &Game) -> Option<Message> {
        None
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) {
        let me = self.player();
        let card = Karma::valid_karma_cards(game, me).into_iter().next();
        game.play_karma(me, card, vec!["see the entire enemy battle plan".into()]);
        game.player_mut(me).special_karma_power_used = true;
        game.stone(Milestone::Karma);
        game.atreides_karma = true;
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!("Using ", TreacheryCardType::Karma, ", ", self.initiator(), " see the entire enemy battle plan")
    }
}

impl GameEventMethods for KarmaPrescience {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::KarmaPrescience
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::KarmaPrescience(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        KarmaPrescience::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) {
        KarmaPrescience::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        KarmaPrescience::get_message(self, game)
    }

}
