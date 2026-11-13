use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct KarmaChoamDiscard {
    #[canon(name = "_cardToUse")]
    pub card_to_use: i32,
    #[canon(name = "_cardIds")]
    pub card_ids: Option<String>,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for KarmaChoamDiscard {
    fn default() -> Self {
        KarmaChoamDiscard {
            card_to_use: Default::default(),
            card_ids: Default::default(),
            base: Default::default(),
        }
    }
}

macro_rules! elems {
    ($($e:expr),* $(,)?) => { vec![$(ExpressionElement::from($e)),*] };
}

/// The type name shown when the expressed `Cards` collection is rendered as text.
/// Kept deliberately so the message text stays the same.
const CARDS_SELECT_TYPE: &str =
    "System.Linq.Enumerable+ListSelectIterator`2[Treachery.Shared.TreacheryCard,Treachery.Shared.MessagePart]";

impl KarmaChoamDiscard {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn card_to_use(&self) -> Option<TreacheryCard> {
        TreacheryCardManager::lookup().find(self.card_to_use)
    }

    pub fn set_card_to_use(&mut self, value: Option<TreacheryCard>) {
        self.card_to_use = TreacheryCardManager::lookup().get_id(value);
    }

    pub fn cards(&self) -> Vec<TreacheryCard> {
        GameEvent::id_string_to_objects(self.card_ids.as_deref(), &TreacheryCardManager::lookup())
    }

    pub fn set_cards(&mut self, value: &[TreacheryCard]) {
        self.card_ids = Some(GameEvent::objects_to_id_string(value, &TreacheryCardManager::lookup()));
    }

    /// `Cards.Select(c => MessagePart.Express(" ", c, " "))` as one message element.
    fn cards_parts(cards: &[TreacheryCard]) -> ExpressionElement {
        ExpressionElement::collection(CARDS_SELECT_TYPE, cards.iter().map(|c| express_part!(" ", *c, " ")))
    }

    pub fn validate(&self, _game: &Game) -> Option<Message> {
        let Some(karma_card_to_use) = self.card_to_use() else {
            return Some(express!("You must select a ", TreacheryCardType::Karma, " card to use"));
        };

        if self.cards().contains(&karma_card_to_use) {
            return Some(express!("You can't discard the card you need to play to use this power"));
        }

        None
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        let me = self.player();
        // C# parity: plays the first valid Karama card, not the validated CardToUse (hazards 18).
        let card = Karma::valid_karma_cards(game, me).into_iter().next();
        let cards = self.cards();
        let parts = Self::cards_parts(&cards);
        let amount = self.cards().len() as i32 * 3;
        game.play_karma(me, card, elems!["discard ", parts, "to get ", Payment::of(amount)]);

        for card in self.cards() {
            game.discard_player_treachery_card_boolean(Some(me), Some(card), true);
        }

        let gain = self.cards().len() as i32 * 3;
        let p = game.player_mut(me);
        p.resources += gain;
        p.special_karma_power_used = true;
        Ok(())
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        // C# parity: no space before "Using", so the log reads "ChoamUsing".
        let cards = self.cards();
        express!(
            self.initiator(),
            "Using ",
            TreacheryCardType::Karma,
            ", ",
            self.initiator(),
            " discard ",
            Self::cards_parts(&cards),
            "to get ",
            Payment::of(self.cards().len() as i32 * 3)
        )
    }
}

impl GameEventMethods for KarmaChoamDiscard {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::KarmaChoamDiscard
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::KarmaChoamDiscard(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        KarmaChoamDiscard::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        KarmaChoamDiscard::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        KarmaChoamDiscard::get_message(self, game)
    }

}
