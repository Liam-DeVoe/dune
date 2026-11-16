use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct DiscardedTaken {
    pub card: Option<TreacheryCard>,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    pub base: GameEventBase,
}

impl Default for DiscardedTaken {
    fn default() -> Self {
        DiscardedTaken {
            card: TreacheryCardManager::get(0),
            base: Default::default(),
        }
    }
}

impl DiscardedTaken {
    pub fn new(game: &Game, initiator: impl Into<Option<Faction>>) -> Self {
        let e = Self::default();
        e.initialize(game, initiator.into());
        e
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        // ValidCards(Game, Player) is lazy: a null Player and a null discard pile are
        // dereferenced inside the predicate, per element, up to the first match.
        if !DiscardedTaken::valid_cards(game, self.player()).any(|v| Some(v) == self.card) {
            return Some(express!("Invalid card"));
        }

        None
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn valid_cards(g: &Game, p: PlayerIndex) -> impl Iterator<Item = TreacheryCard> {
        g.decks.recently_discarded
            .iter()
            .filter(move |(k, v)| Some(**v) != g.player(p).faction && g.decks.treachery_discard_pile.as_ref().expect("treachery discard pile").items.contains(*k))
            .map(move |(k, _)| *k)
    }

    pub fn can_be_played(g: &Game, p: PlayerIndex) -> bool {
        !g.current_phase_cannot_be_interrupted()
            && g.player(p).has_treachery_card_type(TreacheryCardType::TakeDiscarded)
            && DiscardedTaken::valid_cards(g, p).next().is_some()
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        self.log(game);
        // Dictionary.Remove(null) throws.
        let Some(card) = self.card else {
            game_error!("Value cannot be null. (Parameter 'key')")
        };
        game.decks.recently_discarded.shift_remove(&card);
        {
            let items = &mut game.decks.treachery_discard_pile.as_mut().required("treachery discard pile")?.items;
            if let Some(i) = items.iter().position(|c| *c == card) {
                items.remove(i);
            }
        }
        let me = self.player();
        game.player_mut(me).treachery_cards.push(card);
        game.discard(me, TreacheryCardType::TakeDiscarded)?;
        game.stone(Milestone::CardWonSwapped);
        Ok(())
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!(self.initiator(), " use ", TreacheryCardType::TakeDiscarded, " to acquire the discarded ", self.card)
    }

}

impl GameEventMethods for DiscardedTaken {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::DiscardedTaken
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::DiscardedTaken(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        DiscardedTaken::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        DiscardedTaken::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        DiscardedTaken::get_message(self, game)
    }

}
