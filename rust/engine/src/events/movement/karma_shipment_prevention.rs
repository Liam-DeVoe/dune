use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct KarmaShipmentPrevention {
    pub target: Faction,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for KarmaShipmentPrevention {
    fn default() -> Self {
        KarmaShipmentPrevention {
            target: Default::default(),
            base: Default::default(),
        }
    }
}

impl KarmaShipmentPrevention {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        if !KarmaShipmentPrevention::get_valid_targets(game, self.player()).any(|v| v == self.target) {
            return Some(express!("Invalid target"));
        }

        None
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach (docs/lazy-queries.md).
    pub fn get_valid_targets(g: &Game, p: PlayerIndex) -> impl Iterator<Item = Faction> {
        g.players_other_than(p).filter(|f| *f != Faction::Fremen)
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        game.current_karma_shipment_prevention = Some(self.clone());
        let me = self.player();
        let card = Karma::valid_karma_cards(game, me).into_iter().next();
        game.play_karma(me, card, vec!["prevent shipment by ".into(), self.target.into()]);
        game.player_mut(me).special_karma_power_used = true;
        game.stone(Milestone::Karma);
        Ok(())
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!("Using ", TreacheryCardType::Karma, ", ", self.initiator(), " prevent shipment by ", self.target)
    }

}

impl GameEventMethods for KarmaShipmentPrevention {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::KarmaShipmentPrevention
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::KarmaShipmentPrevention(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        KarmaShipmentPrevention::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        KarmaShipmentPrevention::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        KarmaShipmentPrevention::get_message(self, game)
    }

}
