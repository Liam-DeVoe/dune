use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct DistransUsed {
    pub target: Option<Faction>,
    pub card: Option<TreacheryCard>,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    pub base: GameEventBase,
}

impl Default for DistransUsed {
    fn default() -> Self {
        DistransUsed {
            target: Default::default(),
            card: None,
            base: Default::default(),
        }
    }
}

impl DistransUsed {
    pub fn new(game: &Game, initiator: impl Into<Option<Faction>>) -> Self {
        let e = Self::default();
        e.initialize(game, initiator.into());
        e
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        if !DistransUsed::valid_cards(game, self.player()).any(|c| Some(c) == self.card) {
            return Some(express!("Invalid card"));
        }
        if !DistransUsed::valid_targets(game, self.player()).any(|v| Some(v) == self.target) {
            return Some(express!("Invalid target"));
        }
        None
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn valid_cards(game: &Game, p: PlayerId) -> impl Iterator<Item = TreacheryCard> {
        game.player(p).treachery_cards.iter().copied().filter(move |c| c.type_() != TreacheryCardType::Distrans)
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn valid_targets(g: &Game, p: PlayerId) -> impl Iterator<Item = Faction> {
        g.players
            .iter()
            .enumerate()
            .filter(move |(i, target)| g.player_at(*i) != p && target.has_room_for_cards(g))
            .filter_map(move |(_, target)| target.faction)
    }

    pub fn can_be_played(game: &Game, player: PlayerId) -> bool {
        game.player(player).treachery_cards.iter().any(|c| c.type_() == TreacheryCardType::Distrans)
            && !DistransUsed::valid_cards(game, player).next().is_none()
            && !DistransUsed::valid_targets(game, player).next().is_none()
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        let target = self.get_player(game, self.target);
        let target_had_room_for_cards = game.player(target.required("target")?).has_room_for_cards(game);
        let target = target.required("target")?;
        let me = self.player();
        game.discard(me, TreacheryCardType::Distrans)?;
        let card = self.card;
        crate::game::remove_card(&mut game.player_mut(me).treachery_cards, card);
        Game::register_known_by_player(game, me, card);
        game.player_mut(target).treachery_cards.push(card.required("card")?);

        if !game.player(me).treachery_cards.is_empty() {
            let initiator = self.initiator();
            let others: Vec<PlayerId> = game.player_indices().filter(|p| !game.player(*p).is(initiator) && *p != target).collect();
            for p in others {
                let my_cards = game.player(me).treachery_cards.clone();
                Game::unregister_all_known_by_player(game, p, &my_cards);
                let target_cards = game.player(target).treachery_cards.clone();
                Game::unregister_all_known_by_player(game, p, &target_cards);
            }
        }

        self.log(game);
        self.check_if_bidding_for_player_should_be_skipped(game, target, target_had_room_for_cards);
        Ok(())
    }

    pub fn check_if_bidding_for_player_should_be_skipped(&self, game: &mut Game, player: PlayerId, had_room_for_cards: bool) {
        // C# parity: during black market bidding this also executes a regular Bid, not a BlackMarketBid.
        // Nested Execute: pre/post-event tasks run again and the Bid is added to History.
        if game.turn.current_phase == Phase::BlackMarketBidding && had_room_for_cards && !game.player(player).has_room_for_cards(game) && BlackMarketBid::may_be_played(game, player) {
            let mut bid = Bid::new(game, game.player(player).faction.expect("faction"));
            bid.passed = true;
            Rc::new(bid).execute(game, false, false);
        } else if game.turn.current_phase == Phase::Bidding && had_room_for_cards && !game.player(player).has_room_for_cards(game) && Bid::may_be_played(game, Some(player)) {
            let mut bid = Bid::new(game, game.player(player).faction.expect("faction"));
            bid.passed = true;
            Rc::new(bid).execute(game, false, false);
        }
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!(self.initiator(), " use ", TreacheryCardType::Distrans, " to give a card to ", self.target)
    }

}

impl GameEventMethods for DistransUsed {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::DistransUsed
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::DistransUsed(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        DistransUsed::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        DistransUsed::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        DistransUsed::get_message(self, game)
    }

}
