use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct TraitorDiscarded {
    #[canon(name = "_traitorId")]
    pub traitor_id: i32,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for TraitorDiscarded {
    fn default() -> Self {
        TraitorDiscarded {
            traitor_id: Default::default(),
            base: Default::default(),
        }
    }
}

impl TraitorDiscarded {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn traitor(&self) -> Option<Hero> {
        LeaderManager::hero_lookup().find(self.traitor_id)
    }

    pub fn set_traitor(&mut self, value: Option<Hero>) {
        let value = value.and_then(crate::compat::cs::CsNull::opt);
        self.traitor_id = LeaderManager::hero_lookup().get_id(value);
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        let valid = Self::valid_traitors(game, self.player());
        // List<IHero>.Contains(null) is false: the list holds no nulls.
        let traitor = self.traitor();
        if !valid.iter().any(|t| Some(*t) == traitor) {
            return Some(express!("Invalid traitor"));
        }
        None
    }

    pub fn valid_traitors(game: &Game, p: PlayerIndex) -> Vec<Hero> {
        game.player(p).traitors.clone()
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        self.log(game);
        // C# parity: the message says the traitor is shuffled into the deck, but the deck is not shuffled.
        // A null Traitor (only reachable without validation) is added to the deck as null.
        let traitor = self.traitor();
        game.traitor_deck.as_mut().required("traitor deck")?.items.push(traitor.or_null());
        let me = self.player();
        cs::list_remove(&mut game.player_mut(me).traitors, traitor);
        game.number_of_traitors_to_discard = game.number_of_traitors_to_discard.wrapping_sub(1);
        if game.number_of_traitors_to_discard == 0 {
            let phase = game.phase_before_discarding_traitor;
            game.enter(phase, true);
        }
        Ok(())
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!(self.initiator(), " shuffle a traitor into the Traitor deck")
    }
}

impl GameEventMethods for TraitorDiscarded {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::TraitorDiscarded
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::TraitorDiscarded(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        TraitorDiscarded::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        TraitorDiscarded::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        TraitorDiscarded::get_message(self, game)
    }

}
