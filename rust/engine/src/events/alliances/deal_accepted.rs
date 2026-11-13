use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct DealAccepted {
    pub bound_faction: Faction,
    pub type_: DealType,
    pub deal_parameter1: Option<String>,
    pub deal_parameter2: Option<String>,
    pub text: Option<String>,
    pub end: Phase,
    pub price: i32,
    pub benefit: i32,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for DealAccepted {
    fn default() -> Self {
        DealAccepted {
            bound_faction: Default::default(),
            type_: Default::default(),
            deal_parameter1: Default::default(),
            deal_parameter2: Default::default(),
            text: Default::default(),
            end: Default::default(),
            price: Default::default(),
            benefit: Default::default(),
            base: Default::default(),
        }
    }
}

impl DealAccepted {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        if !DealAccepted::may_deal(game, self.player_opt(), self.price) {
            return Some(express!("You currently have an outstanding bid"));
        }

        let bound_player = game.get_player(Some(self.bound_faction));
        if bound_player.is_some() && !DealAccepted::may_deal(game, bound_player, self.benefit) {
            return Some(express!(self.bound_faction, " currently have an outstanding bid"));
        }
        if self.price > game.player(self.player()).resources {
            return Some(express!("You can't pay that much"));
        }

        if let Some(bound_player) = bound_player {
            if self.benefit > game.player(bound_player).resources {
                return Some(express!("Offer is not valid (anymore)"));
            }
        }

        if (self.price > 0 || self.benefit > 0) && game.applicable(Rule::DisableResourceTransfers) {
            return Some(express!(Concept::Resource, " transfers are disabled by house rule"));
        }
        if !game.deal_offers.iter().any(|offer| offer.is_accepted_by(self)) {
            return Some(express!("Offer is not valid (anymore)"));
        }
        if self.open_offer(game).is_none() {
            return Some(express!("You already answered this offer"));
        }

        None
    }

    pub fn may_deal(g: &Game, p: Option<PlayerIndex>, price: i32) -> bool {
        let p = p.and_then(crate::compat::cs::CsNull::opt);
        g.current_phase != Phase::Bidding
            || price <= 0
            || g.current_bid.is_none()
            || {
                // p.Faction dereferences a null player only here.
                let bid = g.current_bid.as_ref().nre();
                let p_faction = g.player(p.nre()).faction;
                bid.initiator() != p_faction
                    && (bid.i_bid_ally_contribution_amount() == 0 || g.player(bid.player_opt().nre()).ally != p_faction)
            }
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach (docs/lazy-queries.md).
    pub fn acceptable_deals(g: &Game, p: Option<PlayerIndex>) -> impl Iterator<Item = Rc<DealOffered>> {
        use either::Either;
        use std::iter::empty;
        let p = p.and_then(crate::compat::cs::CsNull::opt);
        let Some(p) = p else { return Either::Left(empty()) };
        Either::Right(g.deal_offers.iter().filter(move |offer| DealAccepted::is_acceptable_deal(g, p, offer)).cloned())
    }

    /// The `AcceptableDeals` filter for one offer (Rust-only helper, see `acceptable_deals`).
    pub fn is_acceptable_deal(g: &Game, p: PlayerIndex, offer: &Rc<DealOffered>) -> bool {
        let pl = g.player(p);
        offer.initiator() != pl.faction
            && offer.initiator() != pl.ally
            && {
                let to = offer.to.as_ref().nre();
                to.len() == 0 || to.contains(&pl.faction)
            }
            && !g.has_responded_to_deal(offer, pl.faction)
            && (g.economics_status != ChoamEconomicsStatus::Double || (offer.benefit == 0 && offer.price == 0))
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach (docs/lazy-queries.md).
    pub fn cancellable_deals(g: &Game, p: Option<PlayerIndex>) -> impl Iterator<Item = Rc<DealOffered>> {
        use either::Either;
        use std::iter::empty;
        let p = p.and_then(crate::compat::cs::CsNull::opt);
        let Some(p) = p else { return Either::Left(empty()) };
        Either::Right(g.deal_offers.iter().filter(move |offer| offer.initiator() == g.player(p).faction).cloned())
    }

    pub fn current_deals(g: &Game) -> Vec<Deal> {
        g.deals.clone()
    }

    pub fn open_offer(&self, game: &Game) -> Option<Rc<DealOffered>> {
        game.deal_offers
            .iter()
            .find(|offer| offer.is_accepted_by(self) && !game.has_responded_to_deal(offer, self.initiator()))
            .cloned()
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        let m = self.get_message(game).with_log(AsideLog::aside());
        self.log_objects(game, vec![m.into()]);
        // Saved games from before acceptances were recorded replay without
        // validation and may accept the same offer twice, hence the fallback.
        let offer = match self.open_offer(game) {
            Some(o) => Some(o),
            None => game.deal_offers.iter().find(|offer| offer.is_accepted_by(self)).cloned(),
        };

        if let Some(offer) = offer {
            game.record_deal_response(&offer, self.initiator(), true);

            let new_deal = Deal {
                bound_faction: offer.initiator(),
                consuming_faction: self.initiator(),
                deal_parameter1: self.deal_parameter1.clone(),
                deal_parameter2: self.deal_parameter2.clone(),
                end: self.end,
                text: self.text.clone(),
                benefit: self.benefit,
                type_: self.type_,
            };

            game.start_deal(&new_deal);
            let offer_initiator = self.get_player(game, Some(offer.initiator()));

            if let Some(offer_initiator) = offer_initiator {
                if self.price > 0 {
                    // Player is `from`: a null Player throws before anything changes.
                    game.exchange_resources_in_bribe(self.player(), offer_initiator, self.price);
                    game.stone(Milestone::Bribe);
                }
            }

            if let Some(offer_initiator) = offer_initiator {
                if self.benefit > 0 {
                    match self.player_opt() {
                        Some(me) => game.exchange_resources_in_bribe(offer_initiator, me, self.benefit),
                        None => {
                            // ExchangeResourcesInBribe(offerInitiator, null, Benefit): `from` pays,
                            // then dereferencing the null target throws.
                            let from = game.player_mut(offer_initiator);
                            from.resources -= self.benefit;
                            let _ = game.bribes_during_mentat();
                            game_error!("A required value is missing");
                        }
                    }
                    game.stone(Milestone::Bribe);
                }
            }

            if game.is_bot(offer.player()) {
                self.handle_accepted_bot_deal(game, &offer);
            }
        }
        Ok(())
    }

    pub fn handle_accepted_bot_deal(&self, game: &mut Game, offer: &Rc<DealOffered>) {
        if offer.type_ == DealType::TellDiscardedTraitors {
            // C# stores the player's DiscardedTraitors set itself in the message, so the state
            // dump shows its contents at dump time (docs/live-lists.md).
            let discarded = ExpressionElement::live(game, crate::messages::expression::LiveList::PlayerDiscardedTraitors(offer.player()));
            self.log_to(
                game,
                self.initiator(),
                vec![
                    offer.initiator().into(),
                    " discarded: ".into(),
                    discarded,
                ],
            );
            self.log_objects(game, vec![offer.initiator().into(), " gave ".into(), self.initiator().into(), " the agreed information".into()]);
        }
    }

    pub fn get_message(&self, game: &Game) -> Message {
        express!(self.initiator(), " accept ", self.bound_faction, " offer for ", Payment::of(self.price), ": ", self.get_deal_description(game))
    }

    pub fn get_short_message(&self, _game: &Game) -> Message {
        express!(self.initiator(), " accept a deal by ", self.bound_faction)
    }

    pub fn get_deal_description(&self, game: &Game) -> Message {
        Deal::deal_contents_description_game_deal_type_string_int32_phase_string(
            game,
            self.type_,
            self.text.as_deref(),
            self.benefit,
            self.end,
            self.deal_parameter1.as_deref(),
        )
    }

    pub fn get_deal_contents(&self) -> Message {
        match &self.text {
            Some(t) if t.len() > 0 => express!(t.clone()),
            _ => express!(self.type_),
        }
    }

}

impl GameEventMethods for DealAccepted {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::DealAccepted
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::DealAccepted(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        DealAccepted::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        DealAccepted::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        DealAccepted::get_message(self, game)
    }

    fn get_short_message(&self, game: &Game) -> Message {
        DealAccepted::get_short_message(self, game)
    }

}
