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
        self.card_id = TreacheryCardManager::get_id(value);
    }

    pub fn validate(&self, _game: &Game) -> Option<Message> {
        None
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        let me = self.get_player(game, Some(self.initiator())).required("get player")?;
        // C# parity: Validate accepts the id of any card, not only one of the starting cards.
        let card = self.card().required("card")?;
        game.player_mut(me).treachery_cards.push(card);
        crate::iter::remove_first(&mut game.starting_treachery_cards.as_mut().required("starting treachery cards")?.items, &card);
        self.log(game);
        game.starting_treachery_cards.as_mut().required("starting treachery cards")?.shuffle();
        game.stone(Milestone::Shuffled);
        game.deal_remaining_starting_treachery_cards_to_non_ixian();
        Ok(())
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

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        IxianSelectedStartingCard::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        IxianSelectedStartingCard::get_message(self, game)
    }

}
