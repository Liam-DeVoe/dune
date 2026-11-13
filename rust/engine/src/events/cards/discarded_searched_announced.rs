use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct DiscardedSearchedAnnounced {
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for DiscardedSearchedAnnounced {
    fn default() -> Self {
        DiscardedSearchedAnnounced {
            base: Default::default(),
        }
    }
}

impl DiscardedSearchedAnnounced {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn validate(&self, _game: &Game) -> Option<Message> {
        None
    }

    pub fn can_be_played(g: &Game, p: PlayerIndex) -> bool {
        !g.current_phase_cannot_be_interrupted()
            && g.player(p).resources >= 2
            && g.player(p).has_treachery_card_type(TreacheryCardType::SearchDiscarded)
            && DiscardedSearched::valid_cards(g).len() > 0
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        self.log(game);
        game.phase_before_searching_discarded = game.current_phase;
        let me = self.player();
        let p = game.player_mut(me);
        p.resources = p.resources.wrapping_sub(2);
        game.enter(Phase::SearchingDiscarded, true);
        game.stone(Milestone::CardWonSwapped);
        Ok(())
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!(
            self.initiator(),
            " use ",
            TreacheryCardType::SearchDiscarded,
            " and pay ",
            Payment::of(2),
            " to search a card in the Treachery Discard Pile"
        )
    }

}

impl GameEventMethods for DiscardedSearchedAnnounced {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::DiscardedSearchedAnnounced
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::DiscardedSearchedAnnounced(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        DiscardedSearchedAnnounced::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        DiscardedSearchedAnnounced::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        DiscardedSearchedAnnounced::get_message(self, game)
    }

}
