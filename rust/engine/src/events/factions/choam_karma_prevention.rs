use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct ChoamKarmaPrevention {
    pub target: Faction,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for ChoamKarmaPrevention {
    fn default() -> Self {
        ChoamKarmaPrevention {
            target: Default::default(),
            base: Default::default(),
        }
    }
}

impl ChoamKarmaPrevention {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn validate(&self, _game: &Game) -> Option<Message> {
        None
    }

    pub fn can_be_played_by(g: &Game, p: PlayerIndex) -> bool {
        g.player(p).faction == Faction::Choam
            && ((!g.prevented(FactionAdvantage::ChoamDiscarding) && ChoamKarmaPrevention::card_to_use(g, p).is_some())
                || (NexusPlayed::can_use_cunning(g, p) && !g.player(p).treachery_cards.is_empty()))
    }

    pub fn card_to_use(game: &Game, p: PlayerIndex) -> Option<TreacheryCard> {
        game.player(p).treachery_cards.iter().copied().find(|c| c.id() == TreacheryCardManager::CARD_KULL_WAHAD)
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) {
        self.log(game);

        let me = self.player();
        if NexusPlayed::can_use_cunning(game, me) {
            game.discard_nexus_card(me);
            game.stone(Milestone::NexusPlayed);
            game.let_player_discard_treachery_card_of_choice(self.initiator());
        } else {
            let card = ChoamKarmaPrevention::card_to_use(game, me);
            game.discard_treachery_card_boolean(card, true);
        }

        game.current_karma_prevention = Some(self.clone());
        game.stone(Milestone::SpecialUselessPlayed);
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!(self.initiator(), " prevent ", self.target, " from using ", TreacheryCardType::Karma, " this phase")
    }
}

impl GameEventMethods for ChoamKarmaPrevention {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::ChoamKarmaPrevention
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::ChoamKarmaPrevention(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        ChoamKarmaPrevention::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) {
        ChoamKarmaPrevention::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        ChoamKarmaPrevention::get_message(self, game)
    }

}
