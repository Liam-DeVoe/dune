//! Alliances: offering, accepting and breaking them, and what to share with an ally.

use crate::prelude::*;

/// A yes/no permission a faction can give its ally.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum SharedAdvantage {
    AtreidesSharesPrescience,
    FremenSharesPrescience,
    FremenWillProtectFromMonster,
    FremenAllowsThreeFreeRevivals,
    FremenRefundsBattleDial,
    GuildAllowsShippingDiscount,
    BeneGesseritAllowsUseOfVoice,
    IxianAllowsReplacingCards,
    TleilaxuAllowsRevivalDiscount,
    RicheseAllowsUseOfNoField,
    MoritaniAllowsKeepingCards,
    EcazSharesAmbassadors,
}

impl SharedAdvantage {
    /// What `faction` shares with its ally.
    fn of(faction: Faction) -> &'static [SharedAdvantage] {
        use SharedAdvantage::*;
        match faction {
            Faction::Atreides => &[AtreidesSharesPrescience],
            Faction::Fremen => &[FremenSharesPrescience, FremenWillProtectFromMonster, FremenAllowsThreeFreeRevivals, FremenRefundsBattleDial],
            Faction::Guild => &[GuildAllowsShippingDiscount],
            Faction::BeneGesserit => &[BeneGesseritAllowsUseOfVoice],
            Faction::Ixian => &[IxianAllowsReplacingCards],
            Faction::Tleilaxu => &[TleilaxuAllowsRevivalDiscount],
            Faction::Richese => &[RicheseAllowsUseOfNoField],
            Faction::Moritani => &[MoritaniAllowsKeepingCards],
            Faction::Ecaz => &[EcazSharesAmbassadors],
            _ => &[],
        }
    }

    fn grant(self, permission: &mut AllyPermission) {
        use SharedAdvantage::*;
        match self {
            AtreidesSharesPrescience => permission.atreides_shares_prescience = true,
            FremenSharesPrescience => permission.fremen_shares_prescience = true,
            FremenWillProtectFromMonster => permission.fremen_will_protect_from_monster = true,
            FremenAllowsThreeFreeRevivals => permission.fremen_allows_three_free_revivals = true,
            FremenRefundsBattleDial => permission.fremen_refunds_battle_dial = true,
            GuildAllowsShippingDiscount => permission.set_guild_allows_shipping_discount(true),
            BeneGesseritAllowsUseOfVoice => permission.bene_gesserit_allows_use_of_voice = true,
            IxianAllowsReplacingCards => permission.set_ixian_allows_replacing_cards(true),
            TleilaxuAllowsRevivalDiscount => permission.set_tleilaxu_allows_revival_discount(true),
            RicheseAllowsUseOfNoField => permission.richese_allows_use_of_no_field = true,
            MoritaniAllowsKeepingCards => permission.moritani_allows_keeping_cards = true,
            EcazSharesAmbassadors => permission.ecaz_shares_ambassadors = true,
        }
    }

    fn granted(self, game: &Game) -> bool {
        use SharedAdvantage::*;
        match self {
            AtreidesSharesPrescience => game.atreides_shares_prescience,
            FremenSharesPrescience => game.fremen_shares_prescience,
            FremenWillProtectFromMonster => game.fremen_will_protect_from_monster,
            FremenAllowsThreeFreeRevivals => game.fremen_allows_three_free_revivals,
            FremenRefundsBattleDial => game.fremen_refunds_battle_dial,
            GuildAllowsShippingDiscount => game.guild_allows_shipping_discount,
            BeneGesseritAllowsUseOfVoice => game.bene_gesserit_allows_use_of_voice,
            IxianAllowsReplacingCards => game.ixian_allows_replacing_cards,
            TleilaxuAllowsRevivalDiscount => game.tleilaxu_allows_revival_discount,
            RicheseAllowsUseOfNoField => game.richese_allows_use_of_no_field,
            MoritaniAllowsKeepingCards => game.moritani_allows_keeping_cards,
            EcazSharesAmbassadors => game.ecaz_shares_ambassadors,
        }
    }
}

impl ClassicBot {
    pub fn determine_alliance_by_terror(&self, game: &Game) -> Rc<AllianceByTerror> {
        let mut result = AllianceByTerror::new(game, self.faction(game));
        result.passed = !self.accepts_forced_alliance_with(game, Faction::Moritani);
        Rc::new(result)
    }

    pub fn determine_alliance_by_ambassador(&self, game: &Game) -> Rc<AllianceByAmbassador> {
        let mut result = AllianceByAmbassador::new(game, self.faction(game));
        result.passed = !self.accepts_forced_alliance_with(game, Faction::Ecaz);
        Rc::new(result)
    }

    /// Whether to ally with `faction` when it offers through terror or an ambassador: when I have
    /// no ally, or `faction` stands better than 3/4 of my ally.
    fn accepts_forced_alliance_with(&self, game: &Game, faction: Faction) -> bool {
        let offering = game.get_player(Some(faction));
        !game.player(self.player).has_ally()
            || self.player_standing(game, offering.expect("offering")) as f32 > 0.75 * self.player_standing(game, self.allied_player(game).expect("allied player")) as f32
    }

    pub fn determine_nexus_voted(&self, game: &Game) -> Rc<NexusVoted> {
        let mut result = NexusVoted::new(game, self.faction(game));
        result.passed = false;
        Rc::new(result)
    }

    pub fn determine_alliance_offered(&self, game: &Game) -> Option<Rc<AllianceOffered>> {
        let faction = self.faction(game);
        let best_offer_to_me = |from_humans_only: bool| {
            game.current_alliance_offers
                .iter()
                .filter(|offer| offer.target == faction && !(from_humans_only && game.is_bot(offer.player())))
                .cloned()
                .highest_or_default(&self.random, |offer| self.player_standing(game, offer.player()))
        };
        // Accept the best offer from a human, else the best from a bot.
        if let Some(offer) = best_offer_to_me(true).or_else(|| best_offer_to_me(false)) {
            let mut result = AllianceOffered::new(game, faction);
            result.target = offer.initiator();
            return Some(Rc::new(result));
        }

        let nr_of_players = game.players.len();
        let nr_of_unallied_bots = game.player_indices().filter(|&p| game.is_bot(p) && !game.player(p).has_ally()).count() as i32;
        let nr_of_unallied_humans = game.player_indices().filter(|&p| !game.is_bot(p) && !game.player(p).has_ally()).count() as i32;
        let my_offer_is_open = game
            .current_alliance_offers
            .iter()
            .any(|o| o.initiator() == faction && game.player(game.get_player(Some(o.target)).expect("player")).ally == Faction::None);
        if nr_of_players <= 2
            || game.applicable(Rule::BotsCannotAlly)
            || (nr_of_unallied_humans != 0 && nr_of_unallied_humans >= nr_of_unallied_bots - 1)
            || my_offer_is_open
        {
            return None;
        }

        let me = self.player;
        let most_interesting_bot_without_ally = game
            .player_indices()
            .filter(|&p| {
                p != me
                    && game.is_bot(p)
                    && !game.player(p).has_ally()
                    && AllianceOffered::valid_targets(game, me).any(|f| f == game.player(p).faction)
            })
            .highest_or_default(&self.random, |&p| self.player_standing(game, p))?;

        let mut result = AllianceOffered::new(game, faction);
        result.target = game.player(most_interesting_bot_without_ally).faction;
        Some(Rc::new(result))
    }

    pub fn determine_alliance_passed(&self, game: &Game) -> Option<Rc<AlliancePassed>> {
        let faction = self.faction(game);

        // Keep waiting while one of our offers is still open; it is withdrawn when the other faction passes
        if game.current_alliance_offers.iter().any(|o| o.initiator() == faction) {
            return None;
        }

        // Humans may still offer us an alliance until they are done
        if game.player_indices().any(|p| !game.is_bot(p) && !game.has_acted_or_passed.contains(&game.player(p).faction)) {
            return None;
        }

        Some(Rc::new(AlliancePassed::new(game, faction)))
    }

    /// How strong `p` is: cards, spice, strongholds, forces and living leaders.
    pub fn player_standing(&self, game: &Game, p: PlayerIndex) -> i32 {
        let p = game.player(p);
        2 * p.treachery_cards.len() as i32
            + p.resources
            + 6 * p.locations_with_any_forces().iter().filter(|l| l.territory().is_stronghold()).count() as i32
            + p.forces_on_planet().values().map(|b| b.total_amount_of_forces()).sum::<i32>()
            + p.forces_in_reserve()
            + 2 * p.special_forces_in_reserve()
            + 2 * p.leaders.iter().filter(|&&l| game.is_alive(Hero::Leader(l))).count() as i32
    }

    /// Break my alliance for a human whose offer stands better than my ally.
    pub fn determine_alliance_broken(&self, game: &Game) -> Option<Rc<AllianceBroken>> {
        let faction = self.faction(game);
        let offer = game
            .current_alliance_offers
            .iter()
            .filter(|offer| offer.target == faction && !game.is_bot(offer.player()))
            .cloned()
            .highest_or_default(&self.random, |offer| self.player_standing(game, offer.player()))?;

        let better_than_my_ally = game.player(self.player).has_ally()
            && self.player_standing(game, offer.player()) > self.player_standing(game, self.allied_player(game).expect("allied player"));
        better_than_my_ally.then(|| Rc::new(AllianceBroken::new(game, faction)))
    }

    pub fn determine_nexus_card_drawn(&self, game: &Game) -> Rc<NexusCardDrawn> {
        let faction = self.faction(game);
        let nexus = game.player(self.player).nexus;
        let draw = NexusCardDrawn::may_draw(game, self.player)
            && (nexus == Faction::None
                || (faction == Faction::Emperor && nexus == Faction::Emperor && !game.applicable(Rule::EmperorSpecialForces))
                || (faction == Faction::BeneGesserit && nexus == Faction::BeneGesserit && !game.applicable(Rule::BeneGesseritAdvisors))
                || (faction == Faction::Ixian && nexus == Faction::Ixian && !game.applicable(Rule::AdvancedCombat)));
        let mut result = NexusCardDrawn::new(game, faction);
        result.passed = !draw;
        Rc::new(result)
    }

    /// Share spice, a Karama card and my faction's advantages with my ally, when they changed.
    pub fn determine_alliance_permissions(&self, game: &Game) -> Option<Rc<AllyPermission>> {
        let ally = self.ally(game);
        if ally == Faction::None
            || !matches!(game.current_main_phase, MainPhase::Blow | MainPhase::Bidding | MainPhase::ShipmentAndMove | MainPhase::Battle)
        {
            return None;
        }

        let allowed_resources = self.determine_allowed_resources(game);
        let allowed_karma_card = game.player(self.player).treachery_cards.iter().copied().find(|c| c.type_() == TreacheryCardType::Karma);
        let mut permission = AllyPermission::new(game, self.faction(game));
        permission.permitted_resources = allowed_resources;
        permission.set_permitted_karma_card(allowed_karma_card);

        let advantages = SharedAdvantage::of(self.faction(game));
        for advantage in advantages {
            advantage.grant(&mut permission);
        }
        let advantages_changed = advantages.iter().any(|a| !a.granted(game));

        let mut extra_revivals_changed = false;
        if self.faction(game) == Faction::Emperor {
            permission.emperor_will_pay_for_extra_revival = 3;
            extra_revivals_changed = game.emperor_will_pay_for_extra_revival != 3;
        }

        let changed = advantages_changed
            || extra_revivals_changed
            || game.get_permitted_use_of_ally_resources(ally) != allowed_resources
            || game.get_permitted_use_of_ally_karma(ally) != allowed_karma_card;
        changed.then(|| Rc::new(permission))
    }

    /// The spice my ally may use: all of it in the phases my faction doesn't need it, otherwise
    /// what is left above a reserve of 10 (5 once I have shipped).
    pub fn determine_allowed_resources(&self, game: &Game) -> i32 {
        let faction = self.faction(game);
        let resources = self.resources(game);
        let phase = game.current_main_phase;
        let shipped = phase == MainPhase::ShipmentAndMove && game.has_acted_or_passed.contains(&faction);
        let above_reserve = (resources - if shipped { 5 } else { 10 }).max(0);
        match faction {
            Faction::Atreides | Faction::BeneGesserit | Faction::Richese | Faction::Harkonnen | Faction::Ecaz | Faction::Moritani | Faction::Ixian => {
                above_reserve
            }
            Faction::Choam if phase == MainPhase::Battle => resources,
            Faction::Choam => above_reserve,
            Faction::Guild if phase == MainPhase::ShipmentAndMove => resources,
            Faction::Fremen if phase == MainPhase::ShipmentAndMove || game.player(self.player).treachery_cards.len() == 4 => resources,
            Faction::Guild | Faction::Fremen => (resources - 10).max(0),
            Faction::Emperor if phase == MainPhase::Bidding => resources,
            Faction::Emperor => above_reserve,
            Faction::Tleilaxu if phase == MainPhase::Resurrection => resources,
            Faction::Tleilaxu => above_reserve,
            _ => 0,
        }
    }

    /// Answer a card trade: the card asked for, or else my worst card.
    pub fn determine_card_traded(&self, game: &Game) -> Option<Rc<CardTraded>> {
        let offer = game.current_card_trade_offer.as_ref()?;

        let mut result = CardTraded::new(game, self.faction(game));
        result.target = offer.initiator();
        if offer.requested_card().is_some() {
            result.set_card(offer.requested_card());
        } else {
            let worst = game
                .player(self.player)
                .treachery_cards
                .iter()
                .copied()
                .min_by_key(|c| self.card_quality(game, *c, Some(self.player)));
            result.set_card(worst);
        }
        result.set_requested_card(None);
        Some(Rc::new(result))
    }
}
