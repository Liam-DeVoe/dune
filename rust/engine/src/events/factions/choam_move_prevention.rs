use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct ChoamMovePrevention {
    #[canon(name = "_territoryId")]
    pub territory_id: i32,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for ChoamMovePrevention {
    fn default() -> Self {
        ChoamMovePrevention {
            territory_id: Default::default(),
            base: Default::default(),
        }
    }
}

impl ChoamMovePrevention {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn territory(&self, game: &Game) -> Option<Territory> {
        game.map.territory_lookup().find(self.territory_id)
    }

    pub fn set_territory(&mut self, game: &Game, value: Option<Territory>) {
        let value = value.and_then(crate::compat::cs::CsNull::opt);
        self.territory_id = game.map.territory_lookup().get_id(value);
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        let territory = self.territory(game);
        if !ChoamMovePrevention::valid_territories(game, self.player()).any(|v| v == territory.or_null()) {
            return Some(express!("Invalid territory"));
        }
        None
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach (docs/lazy-queries.md).
    pub fn valid_territories(game: &Game, p: PlayerIndex) -> impl Iterator<Item = Territory> {
        game.player(p).territories_with_forces(game)
    }

    pub fn can_be_played_by(g: &Game, p: PlayerIndex) -> bool {
        let pl = g.player(p);
        pl.faction == Faction::Choam
            && !ChoamMovePrevention::valid_territories(g, p).next().is_none()
            && ((!g.prevented(FactionAdvantage::ChoamDiscarding) && ChoamMovePrevention::card_to_use(g, p).is_some())
                || (NexusPlayed::can_use_cunning(g, p) && !pl.treachery_cards.is_empty()))
    }

    pub fn card_to_use(game: &Game, p: PlayerIndex) -> Option<TreacheryCard> {
        game.player(p).treachery_cards.iter().copied().find(|c| c.id() == TreacheryCardManager::CARD_BALISET)
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        self.log(game);

        let me = self.player();
        if NexusPlayed::can_use_cunning(game, me) {
            game.discard_nexus_card(me);
            game.stone(Milestone::NexusPlayed);
            game.let_player_discard_treachery_card_of_choice(self.initiator());
        } else {
            let card = ChoamMovePrevention::card_to_use(game, me);
            game.discard_treachery_card_boolean(card, true)?;
        }

        // `Territory!` does not throw: a null territory (only without validation) is stored.
        game.current_blocked_territories.push(self.territory(game).or_null());
        game.stone(Milestone::SpecialUselessPlayed);
        Ok(())
    }

    pub fn get_message(&self, game: &Game) -> Message {
        express!(self.initiator(), " prevent forces moving into ", self.territory(game))
    }

}

impl GameEventMethods for ChoamMovePrevention {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::ChoamMovePrevention
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::ChoamMovePrevention(self)
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
