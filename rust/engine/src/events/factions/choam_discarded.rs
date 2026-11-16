use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct ChoamDiscarded {
    pub card: Option<TreacheryCard>,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    pub base: GameEventBase,
}

impl Default for ChoamDiscarded {
    fn default() -> Self {
        ChoamDiscarded {
            card: TreacheryCardManager::get(0),
            base: Default::default(),
        }
    }
}

impl ChoamDiscarded {
    pub fn new(game: &Game, initiator: impl Into<Option<Faction>>) -> Self {
        let e = Self::default();
        e.initialize(game, initiator.into());
        e
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        let card = self.card;
        if !ChoamDiscarded::valid_cards(game, self.player()).into_iter().any(|c| Some(c) == card) {
            Some(express!("Invalid card"))
        } else {
            None
        }
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn valid_cards(game: &Game, p: PlayerId) -> impl Iterator<Item = TreacheryCard> {
        let pl = game.player(p);
        pl.treachery_cards
            .iter()
            .copied()
            .filter(move |c| {
                (c.type_() == TreacheryCardType::Useless && !pl.has_any_high_threshold(game))
                    || (c.type_() != TreacheryCardType::Projectile
                        && c.type_() != TreacheryCardType::Poison
                        && pl.treachery_cards.iter().filter(move |to_count| to_count.type_() == c.type_()).count() > 1)
            })
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        let Some(card) = self.card else {
            game_error!("Card is null");
        };

        game.discard_card(Some(card), true)?;
        self.log(game);

        let p = game.player_mut(self.player());
        if card.type_() == TreacheryCardType::Useless {
            p.resources += 2;
        } else {
            p.resources += 3;
        }

        game.stone(Milestone::ResourcesReceived);
        Ok(())
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        if self.card.map(|c| c.type_()) == Some(TreacheryCardType::Useless) {
            express!(self.initiator(), " get ", Payment::of(2), " by discarding a ", TreacheryCardType::Useless, " card")
        } else {
            express!(self.initiator(), " get ", Payment::of(3), " by discarding a duplicate ", self.card)
        }
    }
}

impl GameEventMethods for ChoamDiscarded {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::ChoamDiscarded
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::ChoamDiscarded(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        ChoamDiscarded::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        ChoamDiscarded::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        ChoamDiscarded::get_message(self, game)
    }

}
