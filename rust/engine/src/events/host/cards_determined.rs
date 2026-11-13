use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct CardsDetermined {
    #[canon(name = "_treacheryCardIds")]
    pub treachery_card_ids: Option<String>,
    #[canon(name = "_richeseCardIds")]
    pub richese_card_ids: Option<String>,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for CardsDetermined {
    fn default() -> Self {
        CardsDetermined {
            treachery_card_ids: Some(String::new()),
            richese_card_ids: Some(String::new()),
            base: Default::default(),
        }
    }
}

impl CardsDetermined {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn treachery_cards(&self) -> Vec<TreacheryCard> {
        GameEvent::id_string_to_objects(self.treachery_card_ids.as_deref(), &TreacheryCardManager::lookup())
    }

    pub fn set_treachery_cards(&mut self, value: &[TreacheryCard]) {
        self.treachery_card_ids = Some(GameEvent::objects_to_id_string(value, &TreacheryCardManager::lookup()));
    }

    pub fn richese_cards(&self) -> Vec<TreacheryCard> {
        GameEvent::id_string_to_objects(self.richese_card_ids.as_deref(), &TreacheryCardManager::lookup())
    }

    pub fn set_richese_cards(&mut self, value: &[TreacheryCard]) {
        self.richese_card_ids = Some(GameEvent::objects_to_id_string(value, &TreacheryCardManager::lookup()));
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        let selected = (self.treachery_cards().len() as i32) + (self.richese_cards().len() as i32);
        let needed = dotnet_compat::linq::sum_i32(game.players.iter().map(|p| p.maximum_number_of_cards(game)));
        if selected < (needed + 1) {
            return Some(express!("Not enough cards selected"));
        }

        None
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        game.treachery_deck = Some(Deck::new(self.treachery_cards(), game.random.clone()));
        game.treachery_deck.as_mut().required("treachery deck")?.shuffle();
        game.stone(Milestone::Shuffled);
        game.richese_cache = self.richese_cards();
        let message = self.get_verbose_message();
        self.log_objects(game, vec![message.into()]);
        // Game.Enter(bool, Action, Action) is static in C#; inlined here.
        game.enter_setup_phase();
        Ok(())
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!("Card decks were customized.")
    }

    pub fn get_verbose_message(&self) -> Message {
        if !self.richese_cards().is_empty() {
            return express!("Treachery Cards: ", self.treachery_cards(), ". ", Faction::Richese, " Cards: ", self.richese_cards());
        }
        express!("Treachery Cards: ", self.treachery_cards())
    }

}

impl GameEventMethods for CardsDetermined {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::CardsDetermined
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::CardsDetermined(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        CardsDetermined::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        CardsDetermined::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        CardsDetermined::get_message(self, game)
    }

}
