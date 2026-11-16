use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct KarmaRicheseBuy {
    pub card: Option<TreacheryCard>,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    pub base: GameEventBase,
}

impl Default for KarmaRicheseBuy {
    fn default() -> Self {
        KarmaRicheseBuy {
            card: TreacheryCardManager::get(0),
            base: Default::default(),
        }
    }
}

impl KarmaRicheseBuy {
    pub fn new(game: &Game, initiator: impl Into<Option<Faction>>) -> Self {
        let e = Self::default();
        e.initialize(game, initiator.into());
        e
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        if i32::from(game.player(self.player()).resources) < 3 {
            return Some(express!("You can't pay ", Payment::of(3)));
        }
        if !game.decks.richese_cache.iter().any(|c| Some(*c) == self.card) {
            return Some(express!("Invalid card"));
        }

        None
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        let me = self.player();
        let karma_card = Karma::valid_karma_cards(game, me).into_iter().next();
        game.play_karma(me, karma_card, vec!["buy a card from their cache for ".into(), Payment::of(3).into()]);
        let card = self.card;
        game.player_mut(me).treachery_cards.push(card.required("card")?);
        crate::game::remove_card(&mut game.decks.richese_cache, card);
        if let Some(card) = card {
            game.drop_copy_in_treachery_deck(card);
        }
        let p = game.player_mut(me);
        p.resources -= 3;
        p.special_karma_power_used = true;
        Ok(())
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!(self.initiator(), " use ", TreacheryCardType::Karma, " to buy a card from their cache for ", Payment::of(3))
    }

}

impl GameEventMethods for KarmaRicheseBuy {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::KarmaRicheseBuy
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::KarmaRicheseBuy(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        KarmaRicheseBuy::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        KarmaRicheseBuy::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        KarmaRicheseBuy::get_message(self, game)
    }

}
