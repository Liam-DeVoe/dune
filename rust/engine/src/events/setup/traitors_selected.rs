use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct TraitorsSelected {
    #[canon(name = "_traitorId")]
    pub traitor_id: i32,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for TraitorsSelected {
    fn default() -> Self {
        TraitorsSelected {
            traitor_id: Default::default(),
            base: Default::default(),
        }
    }
}

impl TraitorsSelected {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn selected_traitor(&self) -> Option<Hero> {
        LeaderManager::hero_lookup().find(self.traitor_id)
    }

    pub fn set_selected_traitor(&mut self, value: Option<Hero>) {
        let value = value.and_then(crate::compat::cs::CsNull::opt);
        self.traitor_id = LeaderManager::hero_lookup().get_id(value);
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        let selected = self.selected_traitor();
        if !game.player(self.player()).traitors.iter().any(|t| Some(*t) == selected) {
            return Some(express!("Invalid traitor"));
        }
        None
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        let me = self.player();
        let selected = self.selected_traitor();
        // Snapshot (ToList), hazards 4.1.
        let to_remove: Vec<Hero> = game.player(me).traitors.iter().copied().filter(|l| Some(*l) != selected).collect();

        for l in to_remove {
            game.traitor_deck.as_mut().required("traitor deck")?.items.push(l);
            let traitors = &mut game.player_mut(me).traitors;
            if let Some(i) = traitors.iter().position(|t| *t == l) {
                traitors.remove(i);
            }
            game.player_mut(me).discarded_traitors.insert(l);
            game.player_mut(me).known_non_traitors.insert(l);
        }

        game.has_acted_or_passed.push(self.initiator());
        self.log(game);

        if game.everyone_acted_or_passed() {
            game.assign_face_dancers();
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
