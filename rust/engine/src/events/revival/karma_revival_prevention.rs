use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct KarmaRevivalPrevention {
    pub target: Faction,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for KarmaRevivalPrevention {
    fn default() -> Self {
        KarmaRevivalPrevention {
            target: Default::default(),
            base: Default::default(),
        }
    }
}

impl KarmaRevivalPrevention {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        // PlayersOtherThan(null) only dereferences the player once there is a player to compare.
        if !Self::get_valid_targets(game, self.player_opt().unwrap_or(PlayerIndex::NULL)).any(|v| v == self.target) {
            return Some(express!("Invalid target"));
        }

        None
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach (docs/lazy-queries.md).
    pub fn get_valid_targets(g: &Game, p: PlayerIndex) -> impl Iterator<Item = Faction> {
        g.players_other_than(p)
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) {
        game.current_karma_revival_prevention = Some(self.clone());
        let me = self.player();
        let card = Karma::valid_karma_cards(game, me).into_iter().next();
        game.play_karma(me, card, vec!["prevent revival by ".into(), self.target.into()]);
        game.player_mut(me).special_karma_power_used = true;
        game.stone(Milestone::Karma);
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!("Using ", TreacheryCardType::Karma, ", ", self.initiator(), " prevent revival by ", self.target)
    }

}

impl GameEventMethods for KarmaRevivalPrevention {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::KarmaRevivalPrevention
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::KarmaRevivalPrevention(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        KarmaRevivalPrevention::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) {
        KarmaRevivalPrevention::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        KarmaRevivalPrevention::get_message(self, game)
    }

}
