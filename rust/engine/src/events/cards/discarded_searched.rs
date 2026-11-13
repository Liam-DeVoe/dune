use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct DiscardedSearched {
    #[canon(name = "_cardId")]
    pub card_id: i32,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for DiscardedSearched {
    fn default() -> Self {
        DiscardedSearched {
            card_id: Default::default(),
            base: Default::default(),
        }
    }
}

impl DiscardedSearched {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn card(&self) -> Option<TreacheryCard> {
        TreacheryCardManager::get(self.card_id)
    }

    pub fn set_card(&mut self, value: Option<TreacheryCard>) {
        let value = value.and_then(crate::compat::cs::CsNull::opt);
        self.card_id = TreacheryCardManager::get_id(value);
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        // List.Contains(null) is false.
        let valid_cards = DiscardedSearched::valid_cards(game);
        if !valid_cards.contains(&self.card().or_null()) {
            return Some(express!("Invalid card"));
        }

        None
    }

    pub fn valid_cards(g: &Game) -> Vec<TreacheryCard> {
        g.treachery_discard_pile.as_ref().nre().items.clone()
    }

    pub fn can_be_played(game: &Game, p: PlayerIndex) -> bool {
        game.player(p).has_treachery_card_type(TreacheryCardType::SearchDiscarded)
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) {
        self.log(game);

        for i in 0..game.players.len() {
            let items = game.treachery_discard_pile.as_ref().nre().items.clone();
            Game::unregister_known_player_i_enumerable_treachery_card(game, game.player_at(i), &items);
        }

        let card = self.card();
        cs::list_remove(&mut game.treachery_discard_pile.as_mut().nre().items, card);
        let me = self.player();
        // C# adds `Card!` even when it is null (only reachable without validation).
        game.player_mut(me).treachery_cards.push(card.or_null());
        game.treachery_discard_pile.as_mut().nre().shuffle();
        game.discard(me, TreacheryCardType::SearchDiscarded);
        let phase = game.phase_before_searching_discarded;
        game.enter(phase, true);
        game.stone(Milestone::Shuffled);
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!(self.initiator(), " take one card and the Treachery Discard Pile is then shuffled")
    }

}

impl GameEventMethods for DiscardedSearched {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::DiscardedSearched
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::DiscardedSearched(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        DiscardedSearched::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) {
        DiscardedSearched::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        DiscardedSearched::get_message(self, game)
    }

}
