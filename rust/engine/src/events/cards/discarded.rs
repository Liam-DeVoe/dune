use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct Discarded {
    pub card_id: i32,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    pub base: GameEventBase,
}

impl Default for Discarded {
    fn default() -> Self {
        Discarded {
            card_id: Default::default(),
            base: Default::default(),
        }
    }
}

impl Discarded {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn card(&self) -> Option<TreacheryCard> {
        TreacheryCardManager::get(self.card_id)
    }

    pub fn set_card(&mut self, value: Option<TreacheryCard>) {
        self.card_id = TreacheryCardManager::get_id(value);
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        if self.card().is_none() {
            return Some(express!("Choose a card to discard"));
        }
        if !game.player(self.player()).has(self.card()) {
            return Some(express!("Invalid card"));
        }

        None
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        let initiator = self.initiator();
        if let Some(i) = game.factions_that_must_discard.iter().position(|f| *f == initiator) {
            game.factions_that_must_discard.remove(i);
        }
        let m = self.get_message(game);
        let mut fact = BattleFact::new(BattleFactKind::Discard, initiator);
        fact.card = self.card();
        let m = m.with_fact(fact).with_log(AsideLog::aside());
        self.log_objects(game, vec![m.into()]);
        game.discard_player_treachery_card_boolean(self.player_opt(), self.card(), false);

        if game.factions_that_must_discard.is_empty() {
            let phase = game.phase_before_discarding;
            game.enter(phase, true);
        }
        Ok(())
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!(self.initiator(), " discard ", self.card())
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
