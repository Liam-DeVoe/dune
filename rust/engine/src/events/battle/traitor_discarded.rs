use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct TraitorDiscarded {
    pub traitor: Option<Hero>,
    /// The fields of `Event` (Initiator, Time, Game, Player).
    pub base: GameEventBase,
}

impl Default for TraitorDiscarded {
    fn default() -> Self {
        TraitorDiscarded {
            traitor: TreacheryCardManager::get(0).map(Hero::TreacheryCard),
            base: Default::default(),
        }
    }
}

impl TraitorDiscarded {
    pub fn new(game: &Game, initiator: impl Into<Option<Faction>>) -> Self {
        let e = Self::default();
        e.initialize(game, initiator.into());
        e
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        let valid = Self::valid_traitors(game, self.player());
        // List<IHero>.Contains(null) is false: the list holds no nulls.
        let traitor = self.traitor;
        if !valid.iter().any(|t| Some(*t) == traitor) {
            return Some(express!("Invalid traitor"));
        }
        None
    }

    pub fn valid_traitors(game: &Game, p: PlayerId) -> Vec<Hero> {
        game.player(p).traitors.clone()
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        self.log(game);
        // C# parity: the message says the traitor is shuffled into the deck, but the deck is not shuffled.
        // A null Traitor (only reachable without validation) is added to the deck as null.
        let traitor = self.traitor;
        let traitor = traitor.required("traitor")?;
        game.decks.traitor_deck.as_mut().required("traitor deck")?.items.push(traitor);
        let me = self.player();
        crate::iter::remove_first(&mut game.player_mut(me).traitors, &traitor);
        game.turn.number_of_traitors_to_discard -= 1;
        if game.turn.number_of_traitors_to_discard == 0 {
            let phase = game.turn.phase_before_discarding_traitor.required("phase before discarding traitor")?;
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

    fn event_type(&self) -> EventType {
        EventType::TraitorDiscarded
    }

    fn into_event(self: Rc<Self>) -> Event {
        Event::TraitorDiscarded(self)
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
