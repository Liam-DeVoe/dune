use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct IxianSelectedStartingCard {
    #[canon(name = "_cardId")]
    pub card_id: i32,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for IxianSelectedStartingCard {
    fn default() -> Self {
        IxianSelectedStartingCard {
            card_id: Default::default(),
            base: Default::default(),
        }
    }
}

impl IxianSelectedStartingCard {
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

    pub fn validate(&self, _game: &Game) -> Option<Message> {
        None
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) {
        let me = self.get_player(game, Some(self.initiator())).nre();
        let card = self.card();
        // C# parity: Validate accepts any id. With an unknown id C# adds null to the hand
        // (`Card!` does not throw), and `Items.Remove(null)` removes nothing.
        game.player_mut(me).treachery_cards.push(card.or_null());
        cs::list_remove(&mut game.starting_treachery_cards.as_mut().nre().items, card);
        self.log(game);
        game.starting_treachery_cards.as_mut().nre().shuffle();
        game.stone(Milestone::Shuffled);
        game.deal_remaining_starting_treachery_cards_to_non_ixian();
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!(self.initiator(), " pick their starting treachery card")
    }

}

impl GameEventMethods for IxianSelectedStartingCard {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::IxianSelectedStartingCard
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::IxianSelectedStartingCard(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        IxianSelectedStartingCard::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) {
        IxianSelectedStartingCard::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        IxianSelectedStartingCard::get_message(self, game)
    }

}
