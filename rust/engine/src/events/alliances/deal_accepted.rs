use crate::prelude::*;
use crate::messages::part::Part;

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct DealAccepted {
    pub bound_faction: Option<Faction>,
    #[serde(rename = "dealType")]
    pub type_: DealType,
    pub deal_parameter1: Option<String>,
    pub deal_parameter2: Option<String>,
    pub text: Option<String>,
    pub end: Option<Phase>,
    pub price: u16,
    pub benefit: u16,
}

impl DealAccepted {
    pub fn may_deal(g: &Game, p: Option<PlayerId>, price: i32) -> bool {
        g.turn.current_phase != Phase::Bidding
            || price <= 0
            || g.bidding.current_bid.is_none()
            || {
                // p.Faction dereferences a null player only here.
                let bid = g.bidding.current_bid.as_ref().expect("current bid");
                let p_faction = g.player(p.expect("player")).faction;
                bid.initiator != p_faction
                    && (bid.bid_ally_contribution_amount() == 0 || g.player(bid.player.expect("player")).ally != p_faction)
            }
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn acceptable_deals(g: &Game, p: Option<PlayerId>) -> impl Iterator<Item = &DealOffer> {
        use either::Either;
        use std::iter::empty;
        let Some(p) = p else { return Either::Left(empty()) };
        Either::Right(g.deals.offers.iter().filter(move |offer| DealAccepted::is_acceptable_deal(g, p, offer)))
    }

    /// The `AcceptableDeals` filter for one offer (Rust-only helper, see `acceptable_deals`).
    pub fn is_acceptable_deal(g: &Game, p: PlayerId, offer: &DealOffer) -> bool {
        let pl = g.player(p);
        offer.initiator != pl.faction
            && offer.initiator != pl.ally
            && {
                let to = offer.to.as_ref().expect("destination");
                to.len() == 0 || to.contains_opt(pl.faction)
            }
            && !g.has_responded_to_deal(offer.id, pl.faction)
            && (g.charity.economics_status != Some(ChoamEconomicsStatus::Double) || (i32::from(offer.benefit) == 0 && i32::from(offer.price) == 0))
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn cancellable_deals(g: &Game, p: Option<PlayerId>) -> impl Iterator<Item = &DealOffer> {
        use either::Either;
        use std::iter::empty;
        let Some(p) = p else { return Either::Left(empty()) };
        Either::Right(g.deals.offers.iter().filter(move |offer| offer.initiator == g.player(p).faction))
    }

    pub fn current_deals(g: &Game) -> Vec<Deal> {
        g.deals.agreed.clone()
    }

    /// The offer this accepts that `by` hasn't answered yet.
    pub fn open_offer<'a>(&self, game: &'a Game, by: By) -> Option<&'a DealOffer> {
        game.deals.offers.iter().find(|offer| offer.is_accepted_by(self, by.faction) && !game.has_responded_to_deal(offer.id, by.faction))
    }

    pub fn get_deal_description(&self, game: &Game) -> Message {
        Deal::describe_deal_contents(
            game,
            self.type_,
            self.text.as_deref(),
            i32::from(self.benefit),
            self.end,
            self.deal_parameter1.as_deref(),
        )
    }

    pub fn handle_accepted_bot_deal(&self, game: &mut Game, by: By, offer: &DealOffer) {
        if offer.type_ == DealType::TellDiscardedTraitors {
            // A copy of the traitors discarded so far: later discards don't change this message.
            let discarded = Part::list(game.player(offer.player()).discarded_traitors.iter().copied());
            game.log_to(by.faction.expect("faction"), parts![offer.initiator, " discarded: ", discarded]);
            game.log(parts![offer.initiator, " gave ", by.faction, " the agreed information"]);
        }
    }
}

impl EventRules for DealAccepted {
    fn validate(&self, game: &Game, by: By) -> Result<(), Rejection> {
        if !DealAccepted::may_deal(game, by.player, i32::from(self.price)) {
            return Err(Rejection(express!("You currently have an outstanding bid")));
        }

        let bound_player = game.get_player(self.bound_faction);
        if bound_player.is_some() && !DealAccepted::may_deal(game, bound_player, i32::from(self.benefit)) {
            return Err(Rejection(express!(self.bound_faction, " currently have an outstanding bid")));
        }
        if i32::from(self.price) > i32::from(game.player(by.player()).resources) {
            return Err(Rejection(express!("You can't pay that much")));
        }

        if let Some(bound_player) = bound_player {
            if i32::from(self.benefit) > i32::from(game.player(bound_player).resources) {
                return Err(Rejection(express!("Offer is not valid (anymore)")));
            }
        }

        if (i32::from(self.price) > 0 || i32::from(self.benefit) > 0) && game.applicable(Rule::DisableResourceTransfers) {
            return Err(Rejection(express!(Concept::Resource, " transfers are disabled by house rule")));
        }
        if !game.deals.offers.iter().any(|offer| offer.is_accepted_by(self, by.faction)) {
            return Err(Rejection(express!("Offer is not valid (anymore)")));
        }
        if self.open_offer(game, by).is_none() {
            return Err(Rejection(express!("You already answered this offer")));
        }
        // Not in C#, which let a player give away spice they still had to pay for a bid
        // (escapes.md, "Forces below zero").
        if !game.may_give_away_spice(by.player(), i32::from(self.price)) {
            return Err(Rejection(express!("You need that spice to pay your bid")));
        }
        if bound_player.is_some_and(|b| !game.may_give_away_spice(b, i32::from(self.benefit))) {
            return Err(Rejection(express!(self.bound_faction, " need that spice to pay their bid")));
        }

        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        let m = self.describe(game, by).with_log(AsideLog::aside());
        game.log(parts![m]);
        // Saved games from before acceptances were recorded replay without
        // validation and may accept the same offer twice, hence the fallback.
        let offer = match self.open_offer(game, by) {
            Some(o) => Some(o.clone()),
            None => game.deals.offers.iter().find(|offer| offer.is_accepted_by(self, by.faction)).cloned(),
        };

        if let Some(offer) = offer {
            game.record_deal_response(offer.id, by.required_faction()?, true);

            let new_deal = Deal {
                bound_faction: offer.initiator,
                consuming_faction: by.faction,
                deal_parameter1: self.deal_parameter1.clone(),
                deal_parameter2: self.deal_parameter2.clone(),
                end: self.end,
                text: self.text.clone(),
                benefit: i32::from(self.benefit),
                type_: self.type_,
            };

            game.start_deal(&new_deal);
            let offer_initiator = game.get_player(offer.initiator);

            if let Some(offer_initiator) = offer_initiator {
                if i32::from(self.price) > 0 {
                    // Player is `from`: a null Player throws before anything changes.
                    game.exchange_resources_in_bribe(by.player(), offer_initiator, i32::from(self.price))?;
                    game.stone(Milestone::Bribe);
                }
            }

            if let Some(offer_initiator) = offer_initiator {
                if i32::from(self.benefit) > 0 {
                    match by.player {
                        Some(me) => game.exchange_resources_in_bribe(offer_initiator, me, i32::from(self.benefit))?,
                        None => {
                            // ExchangeResourcesInBribe(offerInitiator, null, Benefit): `from` pays,
                            // then dereferencing the null target throws.
                            let from = game.player_mut(offer_initiator);
                            crate::game::amounts::spend(&mut from.resources, i32::from(self.benefit), "spice")?;
                            let _ = game.bribes_during_mentat();
                            game_error!("A required value is missing");
                        }
                    }
                    game.stone(Milestone::Bribe);
                }
            }

            if game.is_bot(offer.player()) {
                self.handle_accepted_bot_deal(game, by, &offer);
            }
        }
        Ok(())
    }

    fn describe(&self, game: &Game, by: By) -> Message {
        express!(by.faction, " accept ", self.bound_faction, " offer for ", Payment::of(i32::from(self.price)), ": ", self.get_deal_description(game))
    }

    fn describe_short(&self, _game: &Game, by: By) -> Message {
        express!(by.faction, " accept a deal by ", self.bound_faction)
    }
}
