//! Deals: buying and selling prescience, and withdrawing offers that no longer hold.

use crate::prelude::*;

/// An offer made to the Fremen alone.
fn is_only_to_fremen(deal: &DealOffered) -> bool {
    deal.to.as_deref().nre() == [Faction::Fremen]
}

impl ClassicBot {
    /// The first offer of bidding prescience until `end_phase` that I can pay for.
    fn affordable_bidding_prescience_offer(&self, game: &Game, end_phase: Phase) -> Option<Rc<DealOffered>> {
        DealAccepted::acceptable_deals(game, Some(self.player))
            .find(|d| d.type_ == DealType::ShareBiddingPrescience && d.end_phase == end_phase && d.price <= self.resources(game))
    }

    /// Buy bidding prescience when it is cheap enough for what I can spend.
    pub fn determine_deal_accepted(&self, game: &Game) -> Option<Rc<DealAccepted>> {
        let me = self.player;
        if !DealAccepted::may_deal(game, Some(me), 1) || game.applicable(Rule::DisableResourceTransfers) {
            return None;
        }

        let we_have_prescience = || {
            game.has_bidding_prescience(me) || (self.ally(game) != Faction::None && game.has_bidding_prescience(self.allied_player(game).or_null()))
        };

        if game.current_phase == Phase::ClaimingCharity
            && game.player(me).tech_tokens.iter().any(|&tt| tt == TechToken::Resources)
            && self.resources(game) <= 3
            && !we_have_prescience()
        {
            if let Some(d) = self.affordable_bidding_prescience_offer(game, Phase::BiddingReport) {
                return d.acceptance(game, self.faction(game));
            }
        }

        if game.current_phase != Phase::Bidding || we_have_prescience() {
            return None;
        }

        let for_the_phase = self.affordable_bidding_prescience_offer(game, Phase::BiddingReport);
        if let Some(d) = &for_the_phase {
            if self.faction(game) == Faction::Fremen
                && game.current_turn == 1
                && game.applicable(Rule::TechTokens)
                && game.current_phase <= Phase::ClaimingCharity
            {
                return d.acceptance(game, self.faction(game));
            }
            if game.current_main_phase == MainPhase::Bidding
                && game.card_number == 1
                && (d.price as f32) < 1.4 * game.cards_on_auction.as_ref().nre().items.len() as f32
                && self.resources_including_ally_contribution(game) - d.price > 16
            {
                return d.acceptance(game, self.faction(game));
            }
        }

        let for_one_card = self.affordable_bidding_prescience_offer(game, Phase::Bidding);
        let rich_enough = if self.faction(game) == Faction::Emperor {
            self.resources_including_ally_contribution(game) > 6
        } else {
            self.resources_including_ally_contribution(game) > 16
        };
        let max_price_to_pay = if rich_enough && game.player(me).has_room_for_cards(game) { 1 } else { 0 };
        match for_one_card {
            Some(d) if game.current_main_phase == MainPhase::Bidding && d.price <= max_price_to_pay => d.acceptance(game, self.faction(game)),
            _ => None,
        }
    }

    pub fn determine_deal_cancelled(&self, game: &Game) -> Option<Rc<DealOffered>> {
        self.determine_outdated_deal_offers(game)
    }

    pub fn determine_deal_offered(&self, game: &Game) -> Option<Rc<DealOffered>> {
        if self.last_turn(game) || game.current_main_phase <= MainPhase::Setup || game.applicable(Rule::DisableResourceTransfers) {
            return None;
        }

        // The first offer that applies; later ones are only considered (and may draw) when earlier ones gave nothing.
        self.determine_deal_offered_bidding_prescience_entire_phase_fremen_turn1(game)
            .or_else(|| self.determine_deal_offered_bidding_prescience_entire_phase(game))
            .or_else(|| self.determine_deal_offered_bidding_prescience_one_card(game))
            .or_else(|| self.determine_deal_offered_storm_prescience(game))
            .or_else(|| self.determine_deal_offered_resource_deck_prescience(game))
            .or_else(|| self.determine_deal_offered_tell_discarded_traitors(game))
    }

    pub fn determine_outdated_deal_offers(&self, game: &Game) -> Option<Rc<DealOffered>> {
        let mut outdated = self
            .find_invalid_deal_offers(game)
            .or_else(|| self.find_outdated_deal_offer(game, MainPhase::Bidding, DealType::ShareBiddingPrescience))
            .or_else(|| self.find_outdated_deal_offer(game, MainPhase::Battle, DealType::ShareResourceDeckPrescience))
            .or_else(|| self.find_outdated_deal_offer(game, MainPhase::Battle, DealType::ShareStormPrescience));

        if outdated.is_none() && self.last_turn(game) {
            outdated = game.deal_offers.first().cloned();
        }

        outdated.and_then(|o| o.cancellation())
    }

    pub fn find_invalid_deal_offers(&self, game: &Game) -> Option<Rc<DealOffered>> {
        let atreides = game.get_player(Some(Faction::Atreides));
        if let Some(atreides) = atreides {
            if game.current_main_phase == MainPhase::Bidding && !game.has_bidding_prescience(atreides) {
                return game
                    .deal_offers
                    .iter()
                    .find(|offer| offer.initiator() == Faction::Atreides && offer.type_ == DealType::ShareBiddingPrescience)
                    .cloned();
            }
        }

        if atreides.is_some() && game.current_main_phase == MainPhase::ShipmentAndMove && !game.has_resource_deck_prescience(atreides) {
            return game
                .deal_offers
                .iter()
                .find(|offer| offer.initiator() == Faction::Atreides && offer.type_ == DealType::ShareResourceDeckPrescience)
                .cloned();
        }

        let fremen = game.get_player(Some(Faction::Fremen));
        if fremen.is_some() && !game.has_storm_prescience(fremen) {
            return game
                .deal_offers
                .iter()
                .find(|offer| offer.initiator() == Faction::Fremen && offer.type_ == DealType::ShareStormPrescience)
                .cloned();
        }

        None
    }

    pub fn find_outdated_deal_offer(&self, game: &Game, after: MainPhase, type_: DealType) -> Option<Rc<DealOffered>> {
        if game.current_main_phase > after {
            return game
                .deal_offers
                .iter()
                .find(|offer| offer.initiator() == self.faction(game) && offer.type_ == type_)
                .cloned();
        }

        None
    }

    pub fn determine_deal_offered_tell_discarded_traitors(&self, game: &Game) -> Option<Rc<DealOffered>> {
        let faction = self.faction(game);
        if faction != Faction::Harkonnen
            && faction != Faction::Tleilaxu
            && !game.deal_offers.iter().any(|deal| deal.initiator() == faction && deal.type_ == DealType::TellDiscardedTraitors)
        {
            let mut offer = DealOffered::new(game, faction);
            offer.cancel = false;
            offer.type_ = DealType::TellDiscardedTraitors;
            offer.end_phase = Phase::TurnConcluded;
            offer.price = self.d(3, 3);
            offer.text = Some("tell which traitors I discarded".to_string());
            return Some(Rc::new(offer));
        }

        None
    }

    pub fn determine_deal_offered_resource_deck_prescience(&self, game: &Game) -> Option<Rc<DealOffered>> {
        let faction = self.faction(game);
        if faction == Faction::Atreides
            && (game.current_main_phase >= MainPhase::ShipmentAndMove && game.current_main_phase <= MainPhase::Battle)
            && game.has_resource_deck_prescience(Some(self.player))
            && !game.deal_offers.iter().any(|deal| {
                deal.initiator() == faction && deal.type_ == DealType::ShareResourceDeckPrescience && deal.end_phase == Phase::TurnConcluded
            })
        {
            let mut offer = DealOffered::new(game, faction);
            offer.cancel = false;
            offer.type_ = DealType::ShareResourceDeckPrescience;
            offer.end_phase = Phase::TurnConcluded;
            offer.price = self.d(1, 2);
            offer.text = Some("share spice card prescience Player turn".to_string());
            return Some(Rc::new(offer));
        }

        None
    }

    pub fn determine_deal_offered_storm_prescience(&self, game: &Game) -> Option<Rc<DealOffered>> {
        let faction = self.faction(game);
        if faction == Faction::Fremen
            && (game.current_main_phase > MainPhase::Storm && game.current_main_phase <= MainPhase::Battle)
            && game.has_storm_prescience(Some(self.player))
            && !game.deal_offers.iter().any(|deal| {
                deal.initiator() == faction && deal.type_ == DealType::ShareStormPrescience && deal.end_phase == Phase::TurnConcluded
            })
        {
            let mut offer = DealOffered::new(game, faction);
            offer.cancel = false;
            offer.type_ = DealType::ShareStormPrescience;
            offer.end_phase = Phase::TurnConcluded;
            offer.price = self.d(1, 2);
            offer.text = Some("share storm prescience Player turn".to_string());
            return Some(Rc::new(offer));
        }

        None
    }

    pub fn determine_deal_offered_bidding_prescience_entire_phase_fremen_turn1(&self, game: &Game) -> Option<Rc<DealOffered>> {
        let faction = self.faction(game);
        if faction == Faction::Atreides
            && game.is_playing(Faction::Fremen)
            && game.current_turn == 1
            && game.current_main_phase == MainPhase::Charity
            && !game.deal_offers.iter().any(|deal| {
                deal.initiator() == faction
                    && deal.type_ == DealType::ShareBiddingPrescience
                    && deal.end_phase == Phase::BiddingReport
                    && is_only_to_fremen(deal)
            })
        {
            let mut offer = DealOffered::new(game, faction);
            offer.cancel = false;
            offer.type_ = DealType::ShareBiddingPrescience;
            offer.end_phase = Phase::BiddingReport;
            offer.price = 3;
            offer.text = Some("share prescience (phase)".to_string());
            offer.to = Some(vec![Faction::Fremen]);
            return Some(Rc::new(offer));
        }

        None
    }

    pub fn determine_deal_offered_bidding_prescience_entire_phase(&self, game: &Game) -> Option<Rc<DealOffered>> {
        let faction = self.faction(game);
        if faction == Faction::Atreides
            && game.current_main_phase <= MainPhase::Bidding
            && game.has_bidding_prescience(self.player)
            && !game.deal_offers.iter().any(|deal| {
                deal.initiator() == faction
                    && deal.type_ == DealType::ShareBiddingPrescience
                    && deal.end_phase == Phase::BiddingReport
                    && !is_only_to_fremen(deal)
            })
        {
            let nr_of_cards = game
                .player_indices()
                .filter(|&p| (game.player(p).treachery_cards.len() as i32) < game.player(p).maximum_number_of_cards(game))
                .count() as i32;
            let factor: f32 = if self.ally(game) == Faction::None { 1.0 } else { 1.5 };
            let mut offer = DealOffered::new(game, faction);
            offer.cancel = false;
            offer.type_ = DealType::ShareBiddingPrescience;
            offer.end_phase = Phase::BiddingReport;
            // The first term is computed in f32, the second in f64.
            offer.price = ((factor * nr_of_cards as f32) as i32)
                + (0.1f64 * self.resources_including_ally_contribution(game) as f64).floor() as i32;
            offer.text = Some("share prescience (phase)".to_string());
            return Some(Rc::new(offer));
        }

        None
    }

    pub fn determine_deal_offered_bidding_prescience_one_card(&self, game: &Game) -> Option<Rc<DealOffered>> {
        let faction = self.faction(game);
        if faction == Faction::Atreides
            && game.current_main_phase == MainPhase::Bidding
            && game.current_auction_type == AuctionType::Normal
            && game.has_bidding_prescience(self.player)
            && !game.deal_offers.iter().any(|deal| {
                deal.initiator() == faction && deal.type_ == DealType::ShareBiddingPrescience && deal.end_phase == Phase::Bidding
            })
        {
            let mut offer = DealOffered::new(game, faction);
            offer.cancel = false;
            offer.type_ = DealType::ShareBiddingPrescience;
            offer.end_phase = Phase::Bidding;
            offer.price = 1i32
                + (0.05f64 * self.resources_including_ally_contribution(game) as f64).floor() as i32
                + if self.ally(game) == Faction::None { 0 } else { 1 };
            offer.text = Some("share prescience (1 card)".to_string());
            return Some(Rc::new(offer));
        }

        None
    }
}
