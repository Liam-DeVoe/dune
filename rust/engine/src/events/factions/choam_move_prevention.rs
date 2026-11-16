use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct ChoamMovePrevention {
    pub territory: Option<Territory>,
    /// The fields of `Event` (Initiator, Time, Game, Player).
    pub base: GameEventBase,
}

impl Default for ChoamMovePrevention {
    fn default() -> Self {
        ChoamMovePrevention {
            territory: Territory::from_id(0),
            base: Default::default(),
        }
    }
}

impl ChoamMovePrevention {
    pub fn new(game: &Game, initiator: impl Into<Option<Faction>>) -> Self {
        let e = Self::default();
        e.initialize(game, initiator.into());
        e
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        let territory = self.territory;
        if !ChoamMovePrevention::valid_territories(game, self.player()).any(|v| Some(v) == territory) {
            return Some(express!("Invalid territory"));
        }
        None
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn valid_territories(game: &Game, p: PlayerId) -> impl Iterator<Item = Territory> {
        game.player(p).territories_with_forces(game)
    }

    pub fn can_be_played_by(g: &Game, p: PlayerId) -> bool {
        let pl = g.player(p);
        pl.faction == Some(Faction::Choam)
            && !ChoamMovePrevention::valid_territories(g, p).next().is_none()
            && ((!g.prevented(FactionAdvantage::ChoamDiscarding) && ChoamMovePrevention::card_to_use(g, p).is_some())
                || (NexusPlayed::can_use_cunning(g, p) && !pl.treachery_cards.is_empty()))
    }

    pub fn card_to_use(game: &Game, p: PlayerId) -> Option<TreacheryCard> {
        game.player(p).treachery_cards.iter().copied().find(|c| c.id() == TreacheryCardManager::CARD_BALISET)
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        self.log(game);

        let me = self.player();
        if NexusPlayed::can_use_cunning(game, me) {
            game.discard_nexus_card(me);
            game.stone(Milestone::NexusPlayed);
            game.let_player_discard_treachery_card_of_choice(self.initiator().required("faction")?);
        } else {
            let card = ChoamMovePrevention::card_to_use(game, me);
            game.discard_card(card, true)?;
        }

        // `Territory!` does not throw: a null territory (only without validation) is stored.
        game.ship_move.current_blocked_territories.push(self.territory.required("territory")?);
        game.stone(Milestone::SpecialUselessPlayed);
        Ok(())
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!(self.initiator(), " prevent forces moving into ", self.territory)
    }

}

impl GameEventMethods for ChoamMovePrevention {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> EventType {
        EventType::ChoamMovePrevention
    }

    fn into_event(self: Rc<Self>) -> Event {
        Event::ChoamMovePrevention(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        ChoamMovePrevention::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        ChoamMovePrevention::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        ChoamMovePrevention::get_message(self, game)
    }

}
