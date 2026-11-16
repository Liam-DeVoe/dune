use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct ChoamKarmaPrevention {
    pub target: Option<Faction>,
    /// The fields of `Event` (Initiator, Time, Game, Player).
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
    pub fn new(game: &Game, initiator: impl Into<Option<Faction>>) -> Self {
        let e = Self::default();
        e.initialize(game, initiator.into());
        e
    }

    pub fn validate(&self, _game: &Game) -> Option<Message> {
        None
    }

    pub fn can_be_played_by(g: &Game, p: PlayerId) -> bool {
        g.player(p).faction == Some(Faction::Choam)
            && ((!g.prevented(FactionAdvantage::ChoamDiscarding) && ChoamKarmaPrevention::card_to_use(g, p).is_some())
                || (NexusPlayed::can_use_cunning(g, p) && !g.player(p).treachery_cards.is_empty()))
    }

    pub fn card_to_use(game: &Game, p: PlayerId) -> Option<TreacheryCard> {
        game.player(p).treachery_cards.iter().copied().find(|c| c.id() == TreacheryCardManager::CARD_KULL_WAHAD)
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        self.log(game);

        let me = self.player();
        if NexusPlayed::can_use_cunning(game, me) {
            game.discard_nexus_card(me);
            game.stone(Milestone::NexusPlayed);
            game.let_player_discard_treachery_card_of_choice(self.initiator().required("faction")?);
        } else {
            let card = ChoamKarmaPrevention::card_to_use(game, me);
            game.discard_card(card, true)?;
        }

        game.turn.current_karma_prevention = Some(self.clone());
        game.stone(Milestone::SpecialUselessPlayed);
        Ok(())
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!(self.initiator(), " prevent ", self.target, " from using ", TreacheryCardType::Karma, " this phase")
    }
}

impl GameEventMethods for ChoamKarmaPrevention {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> EventType {
        EventType::ChoamKarmaPrevention
    }

    fn into_event(self: Rc<Self>) -> Event {
        Event::ChoamKarmaPrevention(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        ChoamKarmaPrevention::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        ChoamKarmaPrevention::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        ChoamKarmaPrevention::get_message(self, game)
    }

}
