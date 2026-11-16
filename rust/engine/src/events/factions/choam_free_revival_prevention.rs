use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct ChoamFreeRevivalPrevention {
    pub target: Option<Faction>,
    /// The fields of `Event` (Initiator, Time, Game, Player).
    pub base: GameEventBase,
}

impl Default for ChoamFreeRevivalPrevention {
    fn default() -> Self {
        ChoamFreeRevivalPrevention {
            target: Default::default(),
            base: Default::default(),
        }
    }
}

impl ChoamFreeRevivalPrevention {
    pub fn new(game: &Game, initiator: impl Into<Option<Faction>>) -> Self {
        let e = Self::default();
        e.initialize(game, initiator.into());
        e
    }

    pub fn validate(&self, _game: &Game) -> Option<Message> {
        None
    }

    pub fn can_be_played_by(g: &Game, p: PlayerId) -> bool {
        let pl = g.player(p);
        pl.faction == Some(Faction::Choam)
            && ((!g.prevented(FactionAdvantage::ChoamDiscarding) && Self::card_to_use(g, p).is_some())
                || (NexusPlayed::can_use_cunning(g, p) && !pl.treachery_cards.is_empty()))
    }

    pub fn card_to_use(game: &Game, p: PlayerId) -> Option<TreacheryCard> {
        game.player(p).treachery_cards.iter().copied().find(|c| c.id() == TreacheryCardManager::CARD_LALALA)
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        self.log(game);

        let me = self.player();
        if NexusPlayed::can_use_cunning(game, me) {
            game.discard_nexus_card(me);
            game.stone(Milestone::NexusPlayed);
            game.let_player_discard_treachery_card_of_choice(self.initiator().required("faction")?);
        } else {
            let card = Self::card_to_use(game, me);
            game.discard_card(card, true)?;
        }

        game.revival.current_free_revival_prevention = Some(self.clone());
        game.stone(Milestone::SpecialUselessPlayed);
        Ok(())
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!(self.initiator(), " prevent ", self.target, " from using free revival this phase")
    }

}

impl GameEventMethods for ChoamFreeRevivalPrevention {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> EventType {
        EventType::ChoamFreeRevivalPrevention
    }

    fn into_event(self: Rc<Self>) -> Event {
        Event::ChoamFreeRevivalPrevention(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        ChoamFreeRevivalPrevention::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        ChoamFreeRevivalPrevention::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        ChoamFreeRevivalPrevention::get_message(self, game)
    }

}
