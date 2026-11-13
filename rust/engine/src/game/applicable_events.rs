use crate::prelude::*;

impl Game {
    pub fn get_applicable_events(&mut self, player: Option<PlayerIndex>, is_host: bool) -> Vec<GameEventType> {
        let player = player.and_then(crate::compat::cs::CsNull::opt);
        let mut result: Vec<GameEventType> = Vec::new();

        if self.current_phase == Phase::SelectingFactions && (player.is_none() || self.player(player.nre()).faction == Faction::None) {
            result.push(GameEventType::FactionSelected);
        } else if let Some(p) = player {
            if self.player(p).faction != Faction::None {
                self.add_player_actions(p, is_host, &mut result);
            }
        }

        if is_host {
            self.add_host_actions(&mut result);
        }

        result
    }

    pub fn add_host_actions(&self, result: &mut Vec<GameEventType>) {
        match self.current_phase {
            Phase::AwaitingPlayers => result.push(GameEventType::EstablishPlayers),
            Phase::CustomizingDecks => result.push(GameEventType::CardsDetermined),
            Phase::PerformCustomSetup => result.push(GameEventType::PerformSetup),
            Phase::HmsPlacement => {
                if !self.is_playing(Faction::Ixian) && self.applicable(Rule::HmSwithoutIxian) {
                    result.push(GameEventType::PerformHmsPlacement);
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
            | Phase::CancellingTraitor => result.push(GameEventType::EndPhase),
            Phase::ReplacingCardJustWon => result.push(GameEventType::EndPhase),
            _ => {}
        }
    }

    pub fn add_player_actions(&mut self, player: PlayerIndex, is_host: bool, result: &mut Vec<GameEventType>) {
        let faction = self.player(player).faction;
        let has_caravan = |g: &Game| g.player(player).treachery_cards.iter().any(|c| c.type_() == TreacheryCardType::Caravan);

        match self.current_phase {
            Phase::Discarding => {
                if self.factions_that_must_discard.contains(&faction) {
                    result.push(GameEventType::Discarded);
                }
            }
            Phase::DiscardingTraitor => {
                if faction == self.faction_that_must_discard_traitor {
                    result.push(GameEventType::TraitorDiscarded);
                }
            }
            Phase::AllianceByTerror => {
                if faction == self.alliance_by_terror_offered_to {
                    result.push(GameEventType::AllianceByTerror);
                }
            }
            Phase::AllianceByAmbassador => {
                if faction == self.alliance_by_ambassador_offered_to {
                    result.push(GameEventType::AllianceByAmbassador);
                }
            }
            Phase::Bureaucracy => {
                if Some(player) == self.player_skilled_as(LeaderSkill::Bureaucrat) {
                    result.push(GameEventType::Bureaucracy);
                }
            }
            Phase::AssigningInitialSkills | Phase::AssigningSkill => {
                if !self.player(player).skills_to_choose_from.is_empty() {
                    result.push(GameEventType::SkillAssigned);
                }
            }
            Phase::SelectingFactions => {
                if self.player(player).faction == Faction::None {
                    result.push(GameEventType::FactionSelected);
                }
            }
            Phase::DiallingStorm => {
                let f = self.player(player).faction;
                if self.has_battle_wheel.contains(&f) && !self.has_acted_or_passed.contains(&f) {
                    result.push(GameEventType::StormDialled);
                }
            }
            Phase::TradingFactions => {
                if self.players.len() > 1 {
                    result.push(GameEventType::FactionTradeOffered);
                }
            }
            Phase::PerformCustomSetup => {}
            Phase::BeneGesseritPredicting => {
                if faction == Faction::BeneGesserit {
                    result.push(GameEventType::BeneGesseritPrediction);
                }
            }
            Phase::FremenSettingUp => {
                if faction == Faction::Fremen {
                    result.push(GameEventType::PerformFremenSetup);
                }
            }
            Phase::BeneGesseritSettingUp => {
                if faction == Faction::BeneGesserit {
                    result.push(GameEventType::PerformBeneGesseritPlacement);
                }
            }
            Phase::MoritaniSettingUp => {
                if faction == Faction::Moritani {
                    result.push(GameEventType::PerformMoritaniSetup);
                }
            }
            Phase::HarkonnenMulligan => {
                if faction == Faction::Harkonnen {
                    result.push(GameEventType::MulliganPerformed);
                }
            }
            Phase::SelectingTraitors => {
                if faction != Faction::Harkonnen && faction != Faction::Tleilaxu && !self.has_acted_or_passed.contains(&faction) {
                    result.push(GameEventType::TraitorsSelected);
                }
            }
            Phase::IxianSelectingCard => {
                if faction == Faction::Ixian {
                    result.push(GameEventType::IxianSelectedStartingCard);
                }
            }
            Phase::HmsPlacement => {
                if faction == Faction::Ixian {
                    result.push(GameEventType::PerformHmsPlacement);
                }
            }
            Phase::BeginningOfStorm => {
                if !DiscoveryEntered::valid_targets(self, player).next().is_none() {
                    result.push(GameEventType::DiscoveryEntered);
                }
            }
            Phase::HmsMovement => {
                if faction == Faction::Ixian {
                    result.push(GameEventType::PerformHmsMovement);
                }
            }
            Phase::MetheorAndStormSpell => {
                if self.player(player).has_treachery_card_type(TreacheryCardType::StormSpell) && self.current_turn > 1 {
                    result.push(GameEventType::StormSpellPlayed);
                }
                if MetheorPlayed::may_play_metheor(self, player) {
                    result.push(GameEventType::MetheorPlayed);
                }
                if TestingStationUsed::can_be_played(self, player) {
                    result.push(GameEventType::TestingStationUsed);
                }
            }
            Phase::Thumper => {
                if self.player(player).has_treachery_card_type(TreacheryCardType::Thumper) {
                    result.push(GameEventType::ThumperPlayed);
                }
            }
            Phase::VoteAllianceA | Phase::VoteAllianceB => {
                if self.nexus_votes.iter().all(|v| v.initiator() != faction) {
                    result.push(GameEventType::NexusVoted);
                }
            }
            Phase::HarvesterA | Phase::HarvesterB => {
                if self.player(player).has_treachery_card_type(TreacheryCardType::Harvester) {
                    result.push(GameEventType::HarvesterPlayed);
                }
            }
            Phase::StormLosses => {
                if faction == TakeLosses::losses_to_take(self).faction {
                    result.push(GameEventType::TakeLosses);
                }
            }
            Phase::FremenSendingMonsterA | Phase::FremenSendingMonsterB => {
                if faction == Faction::Fremen {
                    result.push(GameEventType::FremenSentMonster);
                }
            }
            Phase::FremenRidingMonsterA | Phase::FremenRidingMonsterB => {
                if faction == Faction::Fremen {
                    result.push(GameEventType::FremenRidesMonster);
                }
            }
            Phase::AllianceA | Phase::AllianceB => {
                if !self.has_acted_or_passed.contains(&faction) {
                    if self.player(player).ally == Faction::None && self.players.len() > 1 {
                        result.push(GameEventType::AllianceOffered);
                    }
                    if self.player(player).ally != Faction::None {
                        result.push(GameEventType::AllianceBroken);
                    }
                    result.push(GameEventType::AlliancePassed);
                }
            }
            Phase::NexusCards => {
                if NexusCardDrawn::applicable(self, player) {
                    result.push(GameEventType::NexusCardDrawn);
                }
            }
            Phase::BlowReport => {}
            Phase::ClaimingCharity => {
                if !is_host && faction == Faction::Atreides {
                    result.push(GameEventType::EndPhase);
                }
                if CharityClaimed::can_be_played(self, player) {
                    result.push(GameEventType::CharityClaimed);
                }
            }
            Phase::BlackMarketAnnouncement => {
                if faction == Faction::Richese {
                    result.push(GameEventType::RicheseAnnouncesBlackMarket);
                }
            }
            Phase::BlackMarketBidding => {
                if BlackMarketBid::may_be_played(self, player) {
                    result.push(GameEventType::BlackMarketBid);
                }
                if faction == Faction::Emperor && self.applicable(Rule::EmperorSupportingNonAllyBids) {
                    result.push(GameEventType::EmperorBidSupport);
                }
            }
            Phase::RicheseAnnouncingAuction => {
                if faction == Faction::Richese {
                    result.push(GameEventType::RicheseAnnouncesAuction);
                }
            }
            Phase::RicheseSpecifyingAuction => {
                if RicheseSpecifiesAuction::may_specify_card(self, player) || RicheseSpecifiesAuction::may_specify_type_of_auction(self, player) {
                    result.push(GameEventType::RicheseSpecifiesAuction);
                }
            }
            Phase::Bidding => {
                if Bid::may_be_played(self, Some(player)) {
                    result.push(GameEventType::Bid);
                }
                if faction == Faction::Emperor && self.applicable(Rule::EmperorSupportingNonAllyBids) {
                    result.push(GameEventType::EmperorBidSupport);
                }
            }
            Phase::RicheseKeepingUnsoldCard => {
                if faction == Faction::Richese {
                    result.push(GameEventType::RicheseKeepsUnsoldCard);
                }
            }
            Phase::IxianRemovingCardFromBid => {
                let occupier_of_ixian_homeworld = self.occupier_of(World::Ixian);
                if (occupier_of_ixian_homeworld.is_none() && faction == Faction::Ixian) || occupier_of_ixian_homeworld == Some(player) {
                    result.push(GameEventType::IxianRemovedCardFromAuction);
                }
            }
            Phase::IxianSwappingCard => {
                if faction == Faction::Ixian {
                    result.push(GameEventType::IxianSwappedCardOnBid);
                }
            }
            Phase::ReplacingCardJustWon => {
                if faction == self.faction_that_may_replace_bought_card {
                    result.push(GameEventType::ReplacedCardWon);
                }
            }
            Phase::WaitingForNextBiddingRound => {
                if !is_host && faction == Faction::Atreides {
                    result.push(GameEventType::EndPhase);
                }
            }
            Phase::BiddingReport => {
                if faction == Faction::Tleilaxu
                    && self.players.len() > 1
                    && !self.prevented(FactionAdvantage::TleilaxuIncreasingRevivalLimits)
                {
                    result.push(GameEventType::SetIncreasedRevivalLimits);
                }
                if faction == Faction::Emperor && self.player(player).has_high_threshold_0(self) && self.player(player).resources >= 2 {
                    result.push(GameEventType::EmperorDiscarded);
                }
            }

            Phase::BeginningOfResurrection => {
                if faction == Faction::Tleilaxu
                    && self.players.len() > 1
                    && !self.prevented(FactionAdvantage::TleilaxuIncreasingRevivalLimits)
                {
                    result.push(GameEventType::SetIncreasedRevivalLimits);
                }
                if RecruitsPlayed::is_applicable(self, player) {
                    result.push(GameEventType::RecruitsPlayed);
                }
            }

            Phase::Resurrection => {
                if self.is_playing(Faction::Tleilaxu)
                    && faction != Faction::Tleilaxu
                    && !self.killed_heroes(player).is_empty()
                    && Revival::normally_revivable_heroes(self, player).is_empty()
                {
                    result.push(GameEventType::RequestTleilaxuRevival);
                }

                if faction == Faction::Tleilaxu && (!self.current_revival_requests.is_empty() || !self.early_revivals_offers.is_empty()) {
                    result.push(GameEventType::AcceptOrCancelTleilaxuRevival);
                }
                if self.has_something_to_revive(player) && !self.prevented_from_reviving(faction) {
                    result.push(GameEventType::Revival);
                }
                if faction == Faction::Tleilaxu
                    && self.players.len() > 1
                    && !self.prevented(FactionAdvantage::TleilaxuIncreasingRevivalLimits)
                {
                    result.push(GameEventType::SetIncreasedRevivalLimits);
                }
            }

            Phase::ResurrectionReport => {
                if faction == Faction::Ecaz && AmbassadorPlaced::is_applicable(self, player) {
                    result.push(GameEventType::AmbassadorPlaced);
                }
            }

            Phase::GuildShip => {
                if faction == Faction::Guild {
                    if self.guild_may_delay() {
                        result.push(GameEventType::GuildDelay);
                    }
                    result.push(GameEventType::Shipment);
                    if has_caravan(self) && !PlacementEvent::valid_movement_sources(self, player).is_empty() {
                        result.push(GameEventType::Caravan);
                    }
                }
            }
            Phase::BeneGesseritAccompaniesGuildShip | Phase::BeneGesseritAccompaniesNonGuildShip => {
                if faction == Faction::BeneGesserit {
                    result.push(GameEventType::BeneGesseritAccompanies);
                }
            }
            Phase::NonGuildShip => {
                if self.is_current_shipment_and_move_player(player) {
                    result.push(GameEventType::Shipment);
                    if has_caravan(self) {
                        result.push(GameEventType::Caravan);
                    }
                }
            }
            Phase::GuildMove => {
                if faction == Faction::Guild {
                    self.add_move_actions(player, result);
                }
            }
            Phase::NonGuildMove => {
                if self.is_current_shipment_and_move_player(player) {
                    self.add_move_actions(player, result);
                }
            }

            Phase::BeneGesseritIntrudedByGuildShip
            | Phase::BeneGesseritIntrudedByNonGuildShip
            | Phase::BeneGesseritIntrudedByGuildMove
            | Phase::BeneGesseritIntrudedByNonGuildMove
            | Phase::BeneGesseritIntrudedByFremenRidingMonsterA
            | Phase::BeneGesseritIntrudedByFremenRidingMonsterB
            | Phase::BeneGesseritIntrudedByCaravan
            | Phase::BeneGesseritIntrudedByRevival => {
                if faction == Faction::BeneGesserit {
                    result.push(GameEventType::BeneGesseritFlip);
                }
            }

            Phase::TerrorTriggeredByGuildShip
            | Phase::TerrorTriggeredByBeneGesseritAccompaniesGuildShip
            | Phase::TerrorTriggeredByNonGuildShip
            | Phase::TerrorTriggeredByBeneGesseritAccompaniesNonGuildShip
            | Phase::TerrorTriggeredByGuildMove
            | Phase::TerrorTriggeredByNonGuildMove
            | Phase::TerrorTriggeredByFremenRidingMonsterA
            | Phase::TerrorTriggeredByFremenRidingMonsterB
            | Phase::TerrorTriggeredByCaravan
            | Phase::TerrorTriggeredByRevival => {
                if faction == Faction::Moritani {
                    result.push(GameEventType::TerrorRevealed);
                }
            }

            Phase::AmbassadorTriggeredByGuildShip
            | Phase::AmbassadorTriggeredByBeneGesseritAccompaniesGuildShip
            | Phase::AmbassadorTriggeredByNonGuildShip
            | Phase::AmbassadorTriggeredByBeneGesseritAccompaniesNonGuildShip
            | Phase::AmbassadorTriggeredByGuildMove
            | Phase::AmbassadorTriggeredByNonGuildMove
            | Phase::AmbassadorTriggeredByFremenRidingMonsterA
            | Phase::AmbassadorTriggeredByFremenRidingMonsterB
            | Phase::AmbassadorTriggeredByCaravan
            | Phase::AmbassadorTriggeredByRevival => {
                if faction == Faction::Ecaz || (self.player(player).ally == Faction::Ecaz && self.ecaz_shares_ambassadors) {
                    result.push(GameEventType::AmbassadorActivated);
                }
            }

            Phase::ShipmentAndMoveConcluded => {
            }

            Phase::BattlePhase => {
                // NextPlayerToBattle mutates BattleSequence (hazards §4.3); only read when CurrentBattle is null.
                if self.current_battle.is_none() && Some(player) == self.next_player_to_battle() {
                    result.push(GameEventType::BattleInitiated);
                }

                if self.current_battle.is_some() && SwitchedSkilledLeader::can_be_played(self, player) {
                    result.push(GameEventType::SwitchedSkilledLeader);
                }

                if HMSAdvantageChosen::can_be_played(self, player) {
                    result.push(GameEventType::HMSAdvantageChosen);
                }

                if Voice::may_use_voice(self, player) {
                    result.push(GameEventType::Voice);
                }

                if Thought::may_be_used(self, player) {
                    result.push(GameEventType::Thought);
                }

                if Prescience::may_use_prescience(self, player) {
                    result.push(GameEventType::Prescience);
                }

                if let Some(cb) = self.current_battle.clone() {
                    if faction == cb.aggressor(self) && self.aggressor_plan.is_none() {
                        result.push(GameEventType::Battle);
                    } else if faction == cb.defender(self) && self.defender_plan.is_none() {
                        result.push(GameEventType::Battle);
                    }

                    if faction == cb.aggressor(self) && self.aggressor_plan.is_some() {
                        result.push(GameEventType::BattleRevision);
                    } else if faction == cb.defender(self) && self.defender_plan.is_some() {
                        result.push(GameEventType::BattleRevision);
                    }

                    if ResidualPlayed::may_play(self, player) {
                        result.push(GameEventType::ResidualPlayed);
                    }
                }

            }

            Phase::ClaimingBattle => {
                if self.has_low_threshold(Faction::Ecaz) {
                    let opponent = match &self.battle_about_to_start {
                        Some(b) => b.opponent_of_faction(self, Faction::Ecaz),
                        None => None,
                    };
                    if opponent.is_some_and(|o| self.player(o).is(faction)) {
                        result.push(GameEventType::BattleClaimed);
                    }
                } else if faction == Faction::Ecaz {
                    result.push(GameEventType::BattleClaimed);
                }
            }

            Phase::CallTraitorOrPass => {
                if let (Some(cb), Some(ap), Some(dp)) = (&self.current_battle, &self.aggressor_plan, &self.defender_plan) {
                    let traitors = &self.player(player).traitors;
                    let ally_of = |f: Faction| self.get_player(Some(f)).map(|p| self.player(p).ally);
                    if (self.aggressor_traitor_action.is_none() && faction == cb.aggressor(self))
                        || (self.aggressor_traitor_action.is_none()
                            && faction == Faction::Harkonnen
                            && ally_of(ap.initiator()) == Some(Faction::Harkonnen)
                            && dp.hero().is_some()
                            && traitors.contains(&dp.hero().nre()))
                        || (self.defender_traitor_action.is_none() && faction == cb.defender(self))
                        || (self.defender_traitor_action.is_none()
                            && faction == Faction::Harkonnen
                            && ally_of(dp.initiator()) == Some(Faction::Harkonnen)
                            && ap.hero().is_some()
                            && traitors.contains(&ap.hero().nre()))
                    {
                        result.push(GameEventType::TreacheryCalled);
                    }
                }

                if let Some(ap) = &self.aggressor_plan {
                    if faction == ap.initiator()
                        && ap.weapon().is_some_and(|w| w.type_() == TreacheryCardType::PoisonTooth)
                        && !self.poison_tooth_cancelled
                    {
                        result.push(GameEventType::PoisonToothCancelled);
                    }
                }

                if let Some(dp) = &self.defender_plan {
                    if faction == dp.initiator()
                        && dp.weapon().is_some_and(|w| w.type_() == TreacheryCardType::PoisonTooth)
                        && !self.poison_tooth_cancelled
                    {
                        result.push(GameEventType::PoisonToothCancelled);
                    }
                }

                if PortableAntidoteUsed::can_be_played(self, player) {
                    result.push(GameEventType::PortableAntidoteUsed);
                }

                if Diplomacy::can_be_played(self, player) {
                    result.push(GameEventType::Diplomacy);
                }
            }

            Phase::Retreating => {
                if Some(faction) == self.battle_loser {
                    result.push(GameEventType::Retreat);
                }
            }

            Phase::MeltingRock => {
                if RockWasMelted::can_be_played(self, player) {
                    result.push(GameEventType::RockWasMelted);
                }
            }

            Phase::CaptureDecision => {
                if faction == Faction::Harkonnen {
                    result.push(GameEventType::Captured);
                }
            }

            Phase::BattleConclusion => {
                if Some(faction) == self.battle_winner && !self.battle_was_concluded_by_winner {
                    result.push(GameEventType::BattleConcluded);
                }
                if LoserConcluded::is_applicable(self, Some(player)) {
                    result.push(GameEventType::LoserConcluded);
                }
            }

            Phase::AvoidingAudit => {
                if Some(player) == self.auditee() {
                    result.push(GameEventType::AuditCancelled);
                }
            }

            Phase::Auditing => {
                if faction == Faction::Choam {
                    result.push(GameEventType::Audited);
                }
            }

            Phase::RevealingFacedancer => {
                if faction == Faction::Tleilaxu {
                    result.push(GameEventType::FaceDancerRevealed);
                }
            }

            Phase::Facedancing => {
                if faction == Faction::Tleilaxu {
                    result.push(GameEventType::FaceDanced);
                }
            }

            Phase::BattleReport => {
                // NextPlayerToBattle mutates BattleSequence (hazards §4.3); only read when the
                // preceding conditions hold.
            }

            Phase::DividingCollectedResources => {
                if DivideResources::is_applicable(self, player) {
                    result.push(GameEventType::DivideResources);
                }
            }

            Phase::AcceptingResourceDivision => {
                if DivideResourcesAccepted::is_applicable(self, player) {
                    result.push(GameEventType::DivideResourcesAccepted);
                }
            }

            Phase::ReplacingFaceDancer => {
                if faction == Faction::Tleilaxu {
                    result.push(GameEventType::FaceDancerReplaced);
                }
            }

            Phase::Extortion => {
                if ExtortionPrevented::can_be_played(self, player) {
                    result.push(GameEventType::ExtortionPrevented);
                }
            }

            Phase::BeginningOfCollection | Phase::CollectionReport => {
                if faction == Faction::Moritani && TerrorPlanted::is_applicable(self, player) {
                    result.push(GameEventType::TerrorPlanted);
                }
            }

            Phase::Contemplate => {
                if faction == Faction::Choam
                    && !self.prevented(FactionAdvantage::ChoamEconomics)
                    && self.economics_status == ChoamEconomicsStatus::None
                {
                    result.push(GameEventType::ChoamEconomics);
                }
                if faction == Faction::Moritani && TerrorPlanted::is_applicable(self, player) {
                    result.push(GameEventType::TerrorPlanted);
                }
            }

            Phase::PerformingKarmaHandSwap => {
                if faction == Faction::Harkonnen && !self.karma_prevented(faction) {
                    result.push(GameEventType::KarmaHandSwap);
                }
            }

            Phase::TradingCards => {
                if Some(faction) == self.current_card_trade_offer.as_ref().map(|o| o.target) {
                    result.push(GameEventType::CardTraded);
                }
            }

            Phase::Clairvoyance => {
                if Some(faction) == self.latest_clairvoyance.as_ref().map(|c| c.target) {
                    result.push(GameEventType::ClairVoyanceAnswered);
                }
            }

            Phase::Thought => {
                // `CurrentBattle?.OpponentOf(CurrentThought!.Initiator)`: the argument (and its
                // NullReferenceException) is only evaluated when CurrentBattle is not null.
                let opponent = match &self.current_battle {
                    Some(cb) => cb.opponent_of_faction(self, self.current_thought.as_ref().nre().initiator()),
                    None => None,
                };
                if Some(player) == opponent {
                    result.push(GameEventType::ThoughtAnswered);
                }
            }

            Phase::SearchingDiscarded => {
                if DiscardedSearched::can_be_played(self, player) {
                    result.push(GameEventType::DiscardedSearched);
                }
            }

            _ => {}
        }

        if self.current_phase > Phase::TradingFactions {
            result.push(GameEventType::AutomationConfigured);
        }

        if self.current_main_phase > MainPhase::Setup
            && faction == Faction::Choam
            && self.player(player).has_low_threshold_0(self)
            && !self.is_occupied_world(World::Choam)
            && !ResourcesAudited::valid_factions(self, player).next().is_none()
        {
            result.push(GameEventType::ResourcesAudited);
        }

        if self.current_main_phase == MainPhase::ShipmentAndMove && SetShipmentPermission::is_applicable(self, player) {
            result.push(GameEventType::SetShipmentPermission);
        }

        if self.current_main_phase == MainPhase::Collection && DiscoveryRevealed::applicable(self, player) {
            result.push(GameEventType::DiscoveryRevealed);
        }

        if NexusPlayed::is_applicable(self, player) {
            result.push(GameEventType::NexusPlayed);
        }

        if self.player(player).ally != Faction::None {
            result.push(GameEventType::AllyPermission);
        }

        let is_after_setup = self.current_main_phase > MainPhase::Setup;
        let has_finalized_battle_plan_waiting_to_be_resolved = (self.current_phase == Phase::BattlePhase
            || self.current_phase == Phase::MeltingRock
            || self.current_phase == Phase::CallTraitorOrPass)
            && self.current_battle.as_ref().is_some_and(|cb| cb.plan_of(self, Some(player)).is_some());

        if JuicePlayed::can_be_played_by(self, player) {
            result.push(GameEventType::JuicePlayed);
        }

        if CardGiven::is_applicable(self, player) {
            result.push(GameEventType::CardGiven);
        }

        if faction == Faction::Ecaz
            && self.current_phase == Phase::CallTraitorOrPass
            && self.current_battle.is_some()
            && self.current_battle.as_ref().nre().is_aggressor_or_defender(self, player)
            && !self.karma_prevented(faction)
            && !self.player(player).special_karma_power_used
            && self.player(player).has_karma(self)
            && self.applicable(Rule::AdvancedKarama)
        {
            let plan_of_ecaz = self.current_battle.as_ref().nre().plan_of_faction(self, Faction::Ecaz);
            if plan_of_ecaz.is_some_and(|p| p.defense().is_none() && p.weapon().is_none()) {
                result.push(GameEventType::KarmaEcazDial);
            }
        }

        if is_after_setup && !has_finalized_battle_plan_waiting_to_be_resolved && !self.current_phase_cannot_be_interrupted() {
            let p = self.player(player);

            if self.current_main_phase < MainPhase::Battle && p.no_field_is_active() {
                result.push(GameEventType::RicheseRevealedNoField);
            }

            if faction == Faction::Choam
                && !self.prevented(FactionAdvantage::ChoamDiscarding)
                && self.consider_as_end_of_phase()
                && !ChoamDiscarded::valid_cards(self, player).next().is_none()
            {
                result.push(GameEventType::ChoamDiscarded);
            }

            if !result.contains(&GameEventType::AmalPlayed) && p.has_treachery_card_type(TreacheryCardType::Amal) && self.consider_as_start_of_phase() {
                result.push(GameEventType::AmalPlayed);
            }

            if DiscardedTaken::can_be_played(self, player) {
                result.push(GameEventType::DiscardedTaken);
            }

            if self.current_phase != Phase::SearchingDiscarded && DiscardedSearchedAnnounced::can_be_played(self, player) {
                result.push(GameEventType::DiscardedSearchedAnnounced);
            }

            if self.current_main_phase == MainPhase::ShipmentAndMove && ChoamMovePrevention::can_be_played_by(self, player) {
                result.push(GameEventType::ChoamMovePrevention);
            }

            if ChoamKarmaPrevention::can_be_played_by(self, player) {
                result.push(GameEventType::ChoamKarmaPrevention);
            }

            if self.current_main_phase == MainPhase::ShipmentAndMove && ChoamExtraMove::can_be_played_by(self, player) {
                result.push(GameEventType::ChoamExtraMove);
            }

            if self.current_main_phase == MainPhase::Resurrection && ChoamFreeRevivalPrevention::can_be_played_by(self, player) {
                result.push(GameEventType::ChoamFreeRevivalPrevention);
            }

            if self.current_main_phase == MainPhase::Contemplate && ChoamRemoveForce::can_be_played_by(self, player) {
                result.push(GameEventType::ChoamRemoveForce);
            }

            if ((faction == Faction::Choam && p.has_ally()) || p.ally == Faction::Choam)
                && self.consider_as_end_of_phase()
                && !self.player(p.allied_player(self).nre()).treachery_cards.is_empty()
                && !p.treachery_cards.is_empty()
                && self.last_turn_card_was_traded < self.current_turn
            {
                result.push(GameEventType::CardTraded);
            }

            if p.has_treachery_card_type(TreacheryCardType::RaiseDead) {
                result.push(GameEventType::RaiseDeadPlayed);
            }

            if p.has_treachery_card_type(TreacheryCardType::Clairvoyance)
                || (p.occupies(self, Some(self.map.shrine().location())) && p.has_karma(self))
            {
                result.push(GameEventType::ClairVoyancePlayed);
            }

            if !self.karma_prevented(faction) && p.has_karma(self) {
                result.push(GameEventType::Karma);
            }

            if self.players.len() > 1
                && faction == Faction::Harkonnen
                && !self.karma_prevented(faction)
                && !p.special_karma_power_used
                && p.has_karma(self)
                && self.current_main_phase == MainPhase::Bidding
                && self.applicable(Rule::AdvancedKarama)
            {
                result.push(GameEventType::KarmaHandSwapInitiated);
            }

            if faction == Faction::Guild
                && !self.karma_prevented(faction)
                && !p.special_karma_power_used
                && p.has_karma(self)
                && self.current_main_phase == MainPhase::ShipmentAndMove
                && self.applicable(Rule::AdvancedKarama)
            {
                result.push(GameEventType::KarmaShipmentPrevention);
            }

            if faction == Faction::Tleilaxu
                && !self.karma_prevented(faction)
                && !p.special_karma_power_used
                && p.has_karma(self)
                && self.current_main_phase == MainPhase::Resurrection
                && self.applicable(Rule::AdvancedKarama)
            {
                result.push(GameEventType::KarmaRevivalPrevention);
            }

            if faction == Faction::Choam
                && !self.karma_prevented(faction)
                && !p.special_karma_power_used
                && p.has_karma(self)
                && self.applicable(Rule::AdvancedKarama)
            {
                result.push(GameEventType::KarmaChoamDiscard);
            }

            if faction == Faction::Richese
                && !self.karma_prevented(faction)
                && !p.special_karma_power_used
                && p.has_karma(self)
                && self.applicable(Rule::AdvancedKarama)
                && !self.has_bid_to_pay(player)
            {
                result.push(GameEventType::KarmaRicheseBuy);
            }

            if faction == Faction::Emperor
                && !self.karma_prevented(faction)
                && !p.special_karma_power_used
                && p.has_karma(self)
                && self.current_main_phase == MainPhase::Resurrection
                && self.applicable(Rule::AdvancedKarama)
            {
                result.push(GameEventType::KarmaFreeRevival);
            }

            if faction == Faction::Ixian
                && ((!self.karma_prevented(faction) && !p.special_karma_power_used && p.has_karma(self)) || self.karma_hms_moves_left == 1)
                && self.current_main_phase == MainPhase::ShipmentAndMove
                && Some(player) == self.shipment_and_move_sequence.as_ref().nre().current_player(self)
                && p.any_forces_in(Some(self.map.hidden_mobile_stronghold().location())) > 0
                && self.applicable(Rule::AdvancedKarama)
            {
                result.push(GameEventType::KarmaHmsMovement);
            }

            if faction == Faction::Fremen
                && self.current_main_phase == MainPhase::Blow
                && self.current_turn > 1
                && !self.karma_prevented(faction)
                && !p.special_karma_power_used
                && p.has_karma(self)
                && self.applicable(Rule::AdvancedKarama)
            {
                result.push(GameEventType::KarmaMonster);
            }

            if faction == Faction::Atreides
                && self.current_main_phase == MainPhase::Battle
                && self.current_battle.is_some()
                && self.current_battle.as_ref().nre().is_involved(self, player)
                && !self.karma_prevented(faction)
                && !p.special_karma_power_used
                && p.has_karma(self)
                && self.applicable(Rule::AdvancedKarama)
            {
                result.push(GameEventType::KarmaPrescience);
            }

            if faction == Faction::BeneGesserit
                && self.current_phase > Phase::AllianceB
                && (self.current_main_phase < MainPhase::ShipmentAndMove
                    || (p.initiated(self.current_bene_gesserit_cunning.clone().map(|c| c.into_event()).as_ref())
                        && self.current_main_phase == MainPhase::ShipmentAndMove))
                && !BeneGesseritBattleAnnouncement::valid_territories(self, player).next().is_none()
            {
                result.push(GameEventType::BeneGesseritBattleAnnouncement);
            }

            if ResourcesTransferred::can_be_played(self, player) {
                result.push(GameEventType::ResourcesTransferred);
            }

            if self.players.len() > 1
                && !Donated::valid_targets(self, player).next().is_none()
                && (is_host || p.resources > 0)
                && Donated::may_donate(self, player)
                && self.aggressor_plan.as_ref().is_none_or(|ap| faction != ap.initiator())
                && self.defender_plan.as_ref().is_none_or(|dp| faction != dp.initiator())
            {
                result.push(GameEventType::Donated);
            }

            if faction == Faction::Richese
                && self.current_phase != Phase::BlackMarketBidding
                && self.current_phase != Phase::Bidding
                && !has_finalized_battle_plan_waiting_to_be_resolved
                && p.has_ally()
                && !RicheseGaveCard::valid_cards(self, player).next().is_none()
                && self.player(p.allied_player(self).nre()).has_room_for_cards(self)
            {
                result.push(GameEventType::RicheseGaveCard);
            }

            if self.current_phase != Phase::BlackMarketBidding
                && (self.current_phase != Phase::Bidding || self.current_bid.is_none())
                && !has_finalized_battle_plan_waiting_to_be_resolved
                && DistransUsed::can_be_played(self, player)
            {
                result.push(GameEventType::DistransUsed);
            }

            result.push(GameEventType::DealOffered);
            result.push(GameEventType::DealAccepted);
            result.push(GameEventType::DealRejected);
        }
    }

    /// `player == ShipmentAndMoveSequence?.CurrentPlayer` (Game_Events.cs:275/293)
    fn is_current_shipment_and_move_player(&self, player: PlayerIndex) -> bool {
        match &self.shipment_and_move_sequence {
            Some(seq) => seq.current_player(self) == Some(player),
            None => false,
        }
    }

    /// The identical bodies of the `GuildMove` and `NonGuildMove` cases (Game_Events.cs:285-289/295-299).
    fn add_move_actions(&self, player: PlayerIndex, result: &mut Vec<GameEventType>) {
        if self.player(player).treachery_cards.iter().any(|c| c.type_() == TreacheryCardType::Caravan) {
            result.push(GameEventType::Caravan);
        }
        result.push(GameEventType::Move);
        if FlightUsed::is_available(self, player) {
            result.push(GameEventType::FlightUsed);
        }
        if FlightDiscoveryUsed::is_available(self, player) {
            result.push(GameEventType::FlightDiscoveryUsed);
        }
        if Planetology::can_be_played(self, player) {
            result.push(GameEventType::Planetology);
        }
    }

    pub fn current_phase_cannot_be_interrupted(&self) -> bool {
        self.current_main_phase == MainPhase::Ended
            || self.current_phase == Phase::Discarding
            || self.current_phase == Phase::AssigningSkill
            || self.current_phase == Phase::Clairvoyance
            || self.current_phase == Phase::Thought
            || self.current_phase == Phase::TradingCards
            || self.current_phase == Phase::Bureaucracy
            || self.current_phase == Phase::SearchingDiscarded
            || self.current_phase == Phase::PerformingKarmaHandSwap
            || self.current_phase == Phase::ReplacingCardJustWon
            || self.current_phase == Phase::DividingCollectedResources
            || self.current_phase == Phase::AcceptingResourceDivision
    }

    pub fn consider_as_end_of_phase(&self) -> bool {
        // `a || b && c is X or Y` reads as `a || (b && (c is X or Y))`.
        self.current_moment == MainPhaseMoment::End
            || (self.current_moment == MainPhaseMoment::Start
                && matches!(self.current_main_phase, MainPhase::Bidding | MainPhase::ShipmentAndMove))
    }

    pub fn consider_as_start_of_phase(&self) -> bool {
        self.current_moment == MainPhaseMoment::Start
            || (self.current_main_phase == MainPhase::Blow && self.current_moment == MainPhaseMoment::End)
    }
}
