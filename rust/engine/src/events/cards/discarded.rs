use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct Discarded {
    pub card: Option<TreacheryCard>,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    pub base: GameEventBase,
}

impl Default for Discarded {
    fn default() -> Self {
        Discarded {
            card: TreacheryCardManager::get(0),
            base: Default::default(),
        }
    }
}

impl Discarded {
    pub fn new(game: &Game, initiator: impl Into<Option<Faction>>) -> Self {
        let e = Self::default();
        e.initialize(game, initiator.into());
        e
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        if self.card.is_none() {
            return Some(express!("Choose a card to discard"));
        }
        if !game.player(self.player()).has(self.card) {
            return Some(express!("Invalid card"));
        }

        None
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        let initiator = self.initiator();
        if let Some(i) = game.turn.factions_that_must_discard.iter().position(|f| Some(*f) == initiator) {
            game.turn.factions_that_must_discard.remove(i);
        }
        let m = self.get_message(game);
        let mut fact = BattleFact::new(BattleFactKind::Discard, initiator.required("faction")?);
        fact.card = self.card;
        let m = m.with_fact(fact).with_log(AsideLog::aside());
        self.log_objects(game, vec![m.into()]);
        game.discard_card_of(self.player_opt(), self.card, false);

        if game.turn.factions_that_must_discard.is_empty() {
            let phase = game.turn.phase_before_discarding.required("phase before discarding")?;
            game.enter(phase, true);
        }
        Ok(())
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!(self.initiator(), " discard ", self.card)
    }

}

impl GameEventMethods for Discarded {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::Discarded
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::Discarded(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        Discarded::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        Discarded::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        Discarded::get_message(self, game)
    }

}
