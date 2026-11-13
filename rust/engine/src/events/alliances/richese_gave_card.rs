use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct RicheseGaveCard {
    #[canon(name = "_cardId")]
    pub card_id: i32,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for RicheseGaveCard {
    fn default() -> Self {
        RicheseGaveCard {
            card_id: -1,
            base: Default::default(),
        }
    }
}

impl RicheseGaveCard {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn card(&self) -> Option<TreacheryCard> {
        TreacheryCardManager::lookup().find(self.card_id)
    }

    pub fn set_card(&mut self, value: Option<TreacheryCard>) {
        let value = value.and_then(crate::compat::cs::CsNull::opt);
        self.card_id = TreacheryCardManager::lookup().get_id(value);
    }

    pub fn validate(&self, _game: &Game) -> Option<Message> {
        None
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach (docs/lazy-queries.md).
    pub fn valid_cards(game: &Game, p: PlayerIndex) -> impl Iterator<Item = TreacheryCard> {
        game.player(p).treachery_cards.iter().copied().filter(move |c| c.rules().contains(&Rule::RicheseTreacheryCards))
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        let me = self.player();
        let target = game.player(me).allied_player(game);

        let card = self.card();
        crate::game::remove_card(&mut game.player_mut(me).treachery_cards, card);
        Game::register_known_player_treachery_card(game, me, card);
        // C# adds a null card to the ally's hand when the card id is unknown.
        let target = target.required("target")?;
        game.player_mut(target).treachery_cards.push(card.or_null());

        let others: Vec<PlayerIndex> = game.player_indices().filter(|p| *p != me && *p != target).collect();
        for p in others {
            let my_cards = game.player(me).treachery_cards.clone();
            Game::unregister_known_player_i_enumerable_treachery_card(game, p, &my_cards);
            let target_cards = game.player(target).treachery_cards.clone();
            Game::unregister_known_player_i_enumerable_treachery_card(game, p, &target_cards);
        }

        self.log(game);
        Ok(())
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!(self.initiator(), " give a card to their ally")
    }

}

impl GameEventMethods for RicheseGaveCard {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::RicheseGaveCard
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::RicheseGaveCard(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        RicheseGaveCard::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        RicheseGaveCard::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        RicheseGaveCard::get_message(self, game)
    }

}
