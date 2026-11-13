use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct ChoamExtraMove {
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for ChoamExtraMove {
    fn default() -> Self {
        ChoamExtraMove {
            base: Default::default(),
        }
    }
}

impl ChoamExtraMove {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn validate(&self, _game: &Game) -> Option<Message> {
        None
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        self.log(game);

        let me = self.player();
        if NexusPlayed::can_use_cunning(game, me) {
            game.discard_nexus_card(me);
            game.stone(Milestone::NexusPlayed);
            game.let_player_discard_treachery_card_of_choice(self.initiator());
        } else {
            let card = ChoamExtraMove::card_to_use(game, me);
            game.discard_treachery_card_boolean(card, true)?;
        }

        game.choam_has_extra_move = true;
        game.stone(Milestone::SpecialUselessPlayed);
        Ok(())
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!(self.initiator(), " gain extra movement")
    }

    pub fn can_be_played_by(g: &Game, p: PlayerIndex) -> bool {
        let pl = g.player(p);
        pl.faction == Faction::Choam
            && ((!g.prevented(FactionAdvantage::ChoamDiscarding) && ChoamExtraMove::card_to_use(g, p).is_some())
                || (NexusPlayed::can_use_cunning(g, p) && !pl.treachery_cards.is_empty()))
    }

    pub fn card_to_use(game: &Game, p: PlayerIndex) -> Option<TreacheryCard> {
        game.player(p).treachery_cards.iter().copied().find(|c| c.id() == TreacheryCardManager::CARD_KULON)
    }

}

impl GameEventMethods for ChoamExtraMove {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::ChoamExtraMove
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::ChoamExtraMove(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        ChoamExtraMove::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        ChoamExtraMove::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        ChoamExtraMove::get_message(self, game)
    }

}
