use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct ChoamDiscarded {
    #[canon(name = "_cardId")]
    pub card_id: i32,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for ChoamDiscarded {
    fn default() -> Self {
        ChoamDiscarded {
            card_id: Default::default(),
            base: Default::default(),
        }
    }
}

impl ChoamDiscarded {
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
        let card = self.card();
        if !ChoamDiscarded::valid_cards(game, self.player()).into_iter().any(|c| Some(c) == card) {
            Some(express!("Invalid card"))
        } else {
            None
        }
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach (docs/lazy-queries.md).
    pub fn valid_cards(game: &Game, p: PlayerIndex) -> impl Iterator<Item = TreacheryCard> {
        let pl = game.player(p);
        pl.treachery_cards
            .iter()
            .copied()
            .filter(move |c| {
                (c.type_() == TreacheryCardType::Useless && !pl.has_high_threshold_0(game))
                    || (c.type_() != TreacheryCardType::Projectile
                        && c.type_() != TreacheryCardType::Poison
                        && pl.treachery_cards.iter().filter(move |to_count| to_count.type_() == c.type_()).count() > 1)
            })
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) {
        let Some(card) = self.card() else {
            throw!(InvalidOperationException, "Card is null");
        };

        game.discard_treachery_card_boolean(Some(card), true);
        self.log(game);

        let p = game.player_mut(self.player());
        if card.type_() == TreacheryCardType::Useless {
            p.resources = p.resources.wrapping_add(2);
        } else {
            p.resources = p.resources.wrapping_add(3);
        }

        game.stone(Milestone::ResourcesReceived);
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        if self.card().map(|c| c.type_()) == Some(TreacheryCardType::Useless) {
            express!(self.initiator(), " get ", Payment::of(2), " by discarding a ", TreacheryCardType::Useless, " card")
        } else {
            express!(self.initiator(), " get ", Payment::of(3), " by discarding a duplicate ", self.card())
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

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) {
        ChoamDiscarded::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        ChoamDiscarded::get_message(self, game)
    }

}
