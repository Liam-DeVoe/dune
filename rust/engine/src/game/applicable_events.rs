use crate::prelude::*;

impl Game {
    pub fn get_applicable_events(&self, player: Option<PlayerId>, is_host: bool) -> Vec<EventType> {
        let mut result: Vec<EventType> = Vec::new();

        if self.phase() == Phase::SelectingFactions && (player.is_none() || self.player(player.expect("player")).faction == None) {
            result.push(EventType::FactionSelected);
        } else if let Some(p) = player {
            if self.player(p).faction != None {
                self.add_player_actions(p, is_host, &mut result);
            }
        }

        if is_host {
            self.add_host_actions(&mut result);
        }

        result
    }

    pub fn add_host_actions(&self, result: &mut Vec<EventType>) {
        match self.phase() {
            Phase::AwaitingPlayers => result.push(EventType::EstablishPlayers),
            Phase::CustomizingDecks => result.push(EventType::CardsDetermined),
            Phase::PerformCustomSetup => result.push(EventType::PerformSetup),
            Phase::HmsPlacement => {
                if !self.is_playing(Faction::Ixian) && self.applicable(Rule::HmSwithoutIxian) {
                    result.push(EventType::PerformHmsPlacement);
                }
            }
            Phase::SelectingFactions
            | Phase::TradingFactions
            | Phase::BeginningOfStorm
            | Phase::MetheorAndStormSpell
            | Phase::StormReport
            | Phase::Thumper
            | Phase::HarvesterA
            | Phase::HarvesterB
            | Phase::BlowReport
            | Phase::BeginningOfCharity
            | Phase::ClaimingCharity
            | Phase::CharityReport
            | Phase::BeginningOfBidding
            | Phase::WaitingForNextBiddingRound
            | Phase::BiddingReport
            | Phase::BeginningOfResurrection
            | Phase::Resurrection
            | Phase::ResurrectionReport
            | Phase::BeginningOfShipAndMove
            | Phase::ShipmentAndMoveConcluded
            | Phase::BeginningOfBattle
            | Phase::BattleReport
            | Phase::BeginningOfCollection
            | Phase::CollectionReport
            | Phase::Extortion
            | Phase::Contemplate
            | Phase::TurnConcluded
            | Phase::CancellingTraitor => result.push(EventType::EndPhase),
            Phase::ReplacingCardJustWon => result.push(EventType::EndPhase),
            _ => {}
        }
    }

    pub fn add_player_actions(&self, player: PlayerId, is_host: bool, result: &mut Vec<EventType>) {
        let faction = self.player(player).faction;

        match self.phase() {
            Phase::AssigningInitialSkills | Phase::AssigningSkill => {
                if !self.player(player).skills_to_choose_from.is_empty() {
                    result.push(EventType::SkillAssigned);
                }
            }

            _ => {}
        }

        // Each phase state has its own function, in the file of its topic. Only the one of the
        // current phase adds anything.
        self.add_setup_actions(player, is_host, result);
        self.add_storm_actions(player, is_host, result);
        self.add_blow_actions(player, is_host, result);
        self.add_charity_actions(player, is_host, result);
        self.add_bidding_actions(player, is_host, result);
        self.add_revival_actions(player, is_host, result);
        self.add_ship_move_actions(player, is_host, result);
        self.add_battle_actions(player, is_host, result);
        self.add_collection_actions(player, is_host, result);
        self.add_contemplate_actions(player, is_host, result);
        self.add_interrupt_actions(player, is_host, result);

        if self.phase() > Phase::TradingFactions {
            result.push(EventType::AutomationConfigured);
        }

        if self.turn.current_main_phase > MainPhase::Setup
            && faction == Some(Faction::Choam)
            && self.player(player).has_any_low_threshold(self)
            && !self.is_occupied_world(World::Choam)
            && !ResourcesAudited::valid_factions(self, player).next().is_none()
        {
            result.push(EventType::ResourcesAudited);
        }

        if self.turn.current_main_phase == MainPhase::ShipmentAndMove && SetShipmentPermission::is_applicable(self, player) {
            result.push(EventType::SetShipmentPermission);
        }

        if self.turn.current_main_phase == MainPhase::Collection && DiscoveryRevealed::applicable(self, player) {
            result.push(EventType::DiscoveryRevealed);
        }

        if NexusPlayed::is_applicable(self, player) {
            result.push(EventType::NexusPlayed);
        }

        if self.player(player).ally != None {
            result.push(EventType::AllyPermission);
        }

        let is_after_setup = self.turn.current_main_phase > MainPhase::Setup;
        let has_finalized_battle_plan_waiting_to_be_resolved = (self.phase() == Phase::BattlePhase
            || self.phase() == Phase::MeltingRock
            || self.phase() == Phase::CallTraitorOrPass)
            && self.current_battle().is_some_and(|cb| cb.plan_of(self, Some(player)).is_some());

        if JuicePlayed::can_be_played_by(self, player) {
            result.push(EventType::JuicePlayed);
        }

        if CardGiven::is_applicable(self, player) {
            result.push(EventType::CardGiven);
        }

        if faction == Some(Faction::Ecaz)
            && self.phase() == Phase::CallTraitorOrPass
            && self.current_battle().is_some()
            && self.current_battle().expect("current battle").is_aggressor_or_defender(self, player)
            && !self.karma_prevented(faction)
            && !self.player(player).special_karma_power_used
            && self.player(player).has_karma(self)
            && self.applicable(Rule::AdvancedKarama)
        {
            let plan_of_ecaz = self.current_battle().expect("current battle").plan_of_faction(self, Faction::Ecaz);
            if plan_of_ecaz.is_some_and(|p| p.defense().is_none() && p.weapon().is_none()) {
                result.push(EventType::KarmaEcazDial);
            }
        }

        if is_after_setup && !has_finalized_battle_plan_waiting_to_be_resolved && !self.current_phase_cannot_be_interrupted() {
            let p = self.player(player);

            if self.turn.current_main_phase < MainPhase::Battle && p.no_field_is_active() {
                result.push(EventType::RicheseRevealedNoField);
            }

            if faction == Some(Faction::Choam)
                && !self.prevented(FactionAdvantage::ChoamDiscarding)
                && self.consider_as_end_of_phase()
                && !ChoamDiscarded::valid_cards(self, player).next().is_none()
            {
                result.push(EventType::ChoamDiscarded);
            }

            if !result.contains(&EventType::AmalPlayed) && p.has_treachery_card_type(TreacheryCardType::Amal) && self.consider_as_start_of_phase() {
                result.push(EventType::AmalPlayed);
            }

            if DiscardedTaken::can_be_played(self, player) {
                result.push(EventType::DiscardedTaken);
            }

            if self.phase() != Phase::SearchingDiscarded && DiscardedSearchedAnnounced::can_be_played(self, player) {
                result.push(EventType::DiscardedSearchedAnnounced);
            }

            if self.turn.current_main_phase == MainPhase::ShipmentAndMove && ChoamMovePrevention::can_be_played_by(self, player) {
                result.push(EventType::ChoamMovePrevention);
            }

            if ChoamKarmaPrevention::can_be_played_by(self, player) {
                result.push(EventType::ChoamKarmaPrevention);
            }

            if self.turn.current_main_phase == MainPhase::ShipmentAndMove && ChoamExtraMove::can_be_played_by(self, player) {
                result.push(EventType::ChoamExtraMove);
            }

            if self.turn.current_main_phase == MainPhase::Resurrection && ChoamFreeRevivalPrevention::can_be_played_by(self, player) {
                result.push(EventType::ChoamFreeRevivalPrevention);
            }

            if self.turn.current_main_phase == MainPhase::Contemplate && ChoamRemoveForce::can_be_played_by(self, player) {
                result.push(EventType::ChoamRemoveForce);
            }

            if ((faction == Some(Faction::Choam) && p.has_ally()) || p.ally == Some(Faction::Choam))
                && self.consider_as_end_of_phase()
                && !self.player(p.allied_player(self).expect("allied player")).treachery_cards.is_empty()
                && !p.treachery_cards.is_empty()
                && self.alliances.last_turn_card_was_traded < Some(self.turn.current_turn)
            {
                result.push(EventType::CardTraded);
            }

            if p.has_treachery_card_type(TreacheryCardType::RaiseDead) {
                result.push(EventType::RaiseDeadPlayed);
            }

            if p.has_treachery_card_type(TreacheryCardType::Clairvoyance)
                || (p.occupies(self, Some(self.board.map.shrine().location())) && p.has_karma(self))
            {
                result.push(EventType::ClairVoyancePlayed);
            }

            if !self.karma_prevented(faction) && p.has_karma(self) {
                result.push(EventType::Karma);
            }

            if self.players.len() > 1
                && faction == Some(Faction::Harkonnen)
                && !self.karma_prevented(faction)
                && !p.special_karma_power_used
                && p.has_karma(self)
                && self.turn.current_main_phase == MainPhase::Bidding
                && self.applicable(Rule::AdvancedKarama)
            {
                result.push(EventType::KarmaHandSwapInitiated);
            }

            if faction == Some(Faction::Guild)
                && !self.karma_prevented(faction)
                && !p.special_karma_power_used
                && p.has_karma(self)
                && self.turn.current_main_phase == MainPhase::ShipmentAndMove
                && self.applicable(Rule::AdvancedKarama)
            {
                result.push(EventType::KarmaShipmentPrevention);
            }

            if faction == Some(Faction::Tleilaxu)
                && !self.karma_prevented(faction)
                && !p.special_karma_power_used
                && p.has_karma(self)
                && self.turn.current_main_phase == MainPhase::Resurrection
                && self.applicable(Rule::AdvancedKarama)
            {
                result.push(EventType::KarmaRevivalPrevention);
            }

            if faction == Some(Faction::Choam)
                && !self.karma_prevented(faction)
                && !p.special_karma_power_used
                && p.has_karma(self)
                && self.applicable(Rule::AdvancedKarama)
            {
                result.push(EventType::KarmaChoamDiscard);
            }

            if faction == Some(Faction::Richese)
                && !self.karma_prevented(faction)
                && !p.special_karma_power_used
                && p.has_karma(self)
                && self.applicable(Rule::AdvancedKarama)
                && !self.has_bid_to_pay(player)
            {
                result.push(EventType::KarmaRicheseBuy);
            }

            if faction == Some(Faction::Emperor)
                && !self.karma_prevented(faction)
                && !p.special_karma_power_used
                && p.has_karma(self)
                && self.turn.current_main_phase == MainPhase::Resurrection
                && self.applicable(Rule::AdvancedKarama)
            {
                result.push(EventType::KarmaFreeRevival);
            }

            if faction == Some(Faction::Ixian)
                && ((!self.karma_prevented(faction) && !p.special_karma_power_used && p.has_karma(self)) || self.storm.karma_hms_moves_left == 1)
                && self.turn.current_main_phase == MainPhase::ShipmentAndMove
                && Some(player) == self.ship_move.shipment_and_move_sequence.as_ref().expect("shipment and move sequence").current_player(self)
                && p.any_forces_in(Some(self.board.map.hidden_mobile_stronghold().location())) > 0
                && self.applicable(Rule::AdvancedKarama)
            {
                result.push(EventType::KarmaHmsMovement);
            }

            if faction == Some(Faction::Fremen)
                && self.turn.current_main_phase == MainPhase::Blow
                && self.turn.current_turn > Turn::FIRST
                && !self.karma_prevented(faction)
                && !p.special_karma_power_used
                && p.has_karma(self)
                && self.applicable(Rule::AdvancedKarama)
            {
                result.push(EventType::KarmaMonster);
            }

            if faction == Some(Faction::Atreides)
                && self.turn.current_main_phase == MainPhase::Battle
                && self.current_battle().is_some()
                && self.current_battle().expect("current battle").is_involved(self, player)
                && !self.karma_prevented(faction)
                && !p.special_karma_power_used
                && p.has_karma(self)
                && self.applicable(Rule::AdvancedKarama)
            {
                result.push(EventType::KarmaPrescience);
            }

            if faction == Some(Faction::BeneGesserit)
                && self.phase() > Phase::AllianceB
                && (self.turn.current_main_phase < MainPhase::ShipmentAndMove
                    || (p.initiated(self.turn.current_bene_gesserit_cunning.as_ref())
                        && self.turn.current_main_phase == MainPhase::ShipmentAndMove))
                && !BeneGesseritBattleAnnouncement::valid_territories(self, player).next().is_none()
            {
                result.push(EventType::BeneGesseritBattleAnnouncement);
            }

            if ResourcesTransferred::can_be_played(self, player) {
                result.push(EventType::ResourcesTransferred);
            }

            if self.players.len() > 1
                && !Donated::valid_targets(self, player).next().is_none()
                && (is_host || i32::from(p.resources) > 0)
                && Donated::may_donate(self, player)
                && self.aggressor_plan().is_none_or(|ap| faction != ap.initiator())
                && self.defender_plan().is_none_or(|dp| faction != dp.initiator())
            {
                result.push(EventType::Donated);
            }

            if faction == Some(Faction::Richese)
                && self.phase() != Phase::BlackMarketBidding
                && self.phase() != Phase::Bidding
                && !has_finalized_battle_plan_waiting_to_be_resolved
                && p.has_ally()
                && !RicheseGaveCard::valid_cards(self, player).next().is_none()
                && self.player(p.allied_player(self).expect("allied player")).has_room_for_cards(self)
            {
                result.push(EventType::RicheseGaveCard);
            }

            if self.phase() != Phase::BlackMarketBidding
                && (self.phase() != Phase::Bidding || self.current_bid().is_none())
                && !has_finalized_battle_plan_waiting_to_be_resolved
                && DistransUsed::can_be_played(self, player)
            {
                result.push(EventType::DistransUsed);
            }

            result.push(EventType::DealOffered);
            result.push(EventType::DealAccepted);
            result.push(EventType::DealRejected);
        }
    }

    /// Whether `player` is the current player of `ShipmentAndMoveSequence` (false if there is none).
    pub(crate) fn is_current_shipment_and_move_player(&self, player: PlayerId) -> bool {
        match &self.ship_move.shipment_and_move_sequence {
            Some(seq) => seq.current_player(self) == Some(player),
            None => false,
        }
    }

    /// The actions shared by the `GuildMove` and `NonGuildMove` cases.
    pub(crate) fn add_move_actions(&self, player: PlayerId, result: &mut Vec<EventType>) {
        if self.player(player).treachery_cards.iter().any(|c| c.type_() == TreacheryCardType::Caravan) {
            result.push(EventType::Caravan);
        }
        result.push(EventType::Move);
        if FlightUsed::is_available(self, player) {
            result.push(EventType::FlightUsed);
        }
        if FlightDiscoveryUsed::is_available(self, player) {
            result.push(EventType::FlightDiscoveryUsed);
        }
        if Planetology::can_be_played(self, player) {
            result.push(EventType::Planetology);
        }
    }

    pub fn current_phase_cannot_be_interrupted(&self) -> bool {
        self.turn.current_main_phase == MainPhase::Ended
            || self.phase() == Phase::Discarding
            || self.phase() == Phase::AssigningSkill
            || self.phase() == Phase::Clairvoyance
            || self.phase() == Phase::Thought
            || self.phase() == Phase::TradingCards
            || self.phase() == Phase::Bureaucracy
            || self.phase() == Phase::SearchingDiscarded
            || self.phase() == Phase::PerformingKarmaHandSwap
            || self.phase() == Phase::ReplacingCardJustWon
            || self.phase() == Phase::DividingCollectedResources
            || self.phase() == Phase::AcceptingResourceDivision
    }

    pub fn consider_as_end_of_phase(&self) -> bool {
        // `a || b && c is X or Y` reads as `a || (b && (c is X or Y))`.
        self.turn.current_moment == Some(MainPhaseMoment::End)
            || (self.turn.current_moment == Some(MainPhaseMoment::Start)
                && matches!(self.turn.current_main_phase, MainPhase::Bidding | MainPhase::ShipmentAndMove))
    }

    pub fn consider_as_start_of_phase(&self) -> bool {
        self.turn.current_moment == Some(MainPhaseMoment::Start)
            || (self.turn.current_main_phase == MainPhase::Blow && self.turn.current_moment == Some(MainPhaseMoment::End))
    }
}

