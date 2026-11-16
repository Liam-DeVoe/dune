use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct TraitorsSelected {
    pub selected_traitor: Option<Hero>,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    pub base: GameEventBase,
}

impl Default for TraitorsSelected {
    fn default() -> Self {
        TraitorsSelected {
            selected_traitor: TreacheryCardManager::get(0).map(Hero::TreacheryCard),
            base: Default::default(),
        }
    }
}

impl TraitorsSelected {
    pub fn new(game: &Game, initiator: impl Into<Option<Faction>>) -> Self {
        let e = Self::default();
        e.initialize(game, initiator.into());
        e
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        let selected = self.selected_traitor;
        if !game.player(self.player()).traitors.iter().any(|t| Some(*t) == selected) {
            return Some(express!("Invalid traitor"));
        }
        None
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        let me = self.player();
        let selected = self.selected_traitor;
        // Snapshot (ToList).
        let to_remove: Vec<Hero> = game.player(me).traitors.iter().copied().filter(|l| Some(*l) != selected).collect();

        for l in to_remove {
            game.decks.traitor_deck.as_mut().required("traitor deck")?.items.push(l);
            let traitors = &mut game.player_mut(me).traitors;
            if let Some(i) = traitors.iter().position(|t| *t == l) {
                traitors.remove(i);
            }
            game.player_mut(me).discarded_traitors.insert(l);
            game.player_mut(me).known_non_traitors.insert(l);
        }

        game.turn.has_acted_or_passed.push(self.initiator().required("faction")?);
        self.log(game);

        if game.everyone_acted_or_passed() {
            game.assign_face_dancers()?;
        }
        Ok(())
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!(self.initiator(), " pick their traitor")
    }

}

impl GameEventMethods for TraitorsSelected {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::TraitorsSelected
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::TraitorsSelected(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        TraitorsSelected::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        TraitorsSelected::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        TraitorsSelected::get_message(self, game)
    }

}
