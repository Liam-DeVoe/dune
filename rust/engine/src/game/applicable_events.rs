use crate::prelude::*;

impl Game {
    pub fn get_applicable_events(&self, player: Option<PlayerId>, is_host: bool) -> Vec<EventType> {
        let mut result: Vec<EventType> = Vec::new();

        if self.turn.current_phase == Phase::SelectingFactions && (player.is_none() || self.player(player.expect("player")).faction == None) {
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
        match self.turn.current_phase {
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
        let has_caravan = |g: &Game| g.player(player).treachery_cards.iter().any(|c| c.type_() == TreacheryCardType::Caravan);

        match self.turn.current_phase {
            Phase::Discarding => {
                if self.turn.factions_that_must_discard.contains_opt(faction) {
                    result.push(EventType::Discarded);
                }
            }
            Phase::DiscardingTraitor => {
                if faction == self.turn.faction_that_must_discard_traitor {
                    result.push(EventType::TraitorDiscarded);
                }
            }
            Phase::AllianceByTerror => {
                if faction == self.ship_move.alliance_by_terror_offered_to {
                    result.push(EventType::AllianceByTerror);
                }
            }
            Phase::AllianceByAmbassador => {
                if faction == self.ship_move.alliance_by_ambassador_offered_to {
                    result.push(EventType::AllianceByAmbassador);
                }
            }
            Phase::Bureaucracy => {
                if Some(player) == self.player_skilled_as(LeaderSkill::Bureaucrat) {
                    result.push(EventType::Bureaucracy);
                }
            }
            Phase::AssigningInitialSkills | Phase::AssigningSkill => {
                if !self.player(player).skills_to_choose_from.is_empty() {
                    result.push(EventType::SkillAssigned);
                }
            }
            Phase::SelectingFactions => {
                if self.player(player).faction == None {
                    result.push(EventType::FactionSelected);
                }
            }
            Phase::DiallingStorm => {
                let f = self.player(player).faction;
                if self.storm.has_battle_wheel.contains_opt(f) && !self.turn.has_acted_or_passed.contains_opt(f) {
                    result.push(EventType::StormDialled);
                }
            }
            Phase::TradingFactions => {
                if self.players.len() > 1 {
                    result.push(EventType::FactionTradeOffered);
                }
            }
            Phase::PerformCustomSetup => {}
            Phase::BeneGesseritPredicting => {
                if faction == Some(Faction::BeneGesserit) {
                    result.push(EventType::BeneGesseritPrediction);
                }
            }
            Phase::FremenSettingUp => {
                if faction == Some(Faction::Fremen) {
                    result.push(EventType::PerformFremenSetup);
                }
            }
            Phase::BeneGesseritSettingUp => {
                if faction == Some(Faction::BeneGesserit) {
                    result.push(EventType::PerformBeneGesseritPlacement);
                }
            }
            Phase::MoritaniSettingUp => {
                if faction == Some(Faction::Moritani) {
                    result.push(EventType::PerformMoritaniSetup);
                }
            }
            Phase::HarkonnenMulligan => {
                if faction == Some(Faction::Harkonnen) {
                    result.push(EventType::MulliganPerformed);
                }
            }
            Phase::SelectingTraitors => {
                if faction != Some(Faction::Harkonnen) && faction != Some(Faction::Tleilaxu) && !self.turn.has_acted_or_passed.contains_opt(faction) {
                    result.push(EventType::TraitorsSelected);
                }
            }
            Phase::IxianSelectingCard => {
                if faction == Some(Faction::Ixian) {
                    result.push(EventType::IxianSelectedStartingCard);
                }
            }
            Phase::HmsPlacement => {
                if faction == Some(Faction::Ixian) {
                    result.push(EventType::PerformHmsPlacement);
                }
            }
            Phase::BeginningOfStorm => {
                if !DiscoveryEntered::valid_targets(self, player).next().is_none() {
                    result.push(EventType::DiscoveryEntered);
                }
            }
            Phase::HmsMovement => {
                if faction == Some(Faction::Ixian) {
                    result.push(EventType::PerformHmsMovement);
                }
            }
            Phase::MetheorAndStormSpell => {
                if self.player(player).has_treachery_card_type(TreacheryCardType::StormSpell) && self.turn.current_turn > Turn::FIRST {
                    result.push(EventType::StormSpellPlayed);
                }
                if MetheorPlayed::may_play_metheor(self, player) {
                    result.push(EventType::MetheorPlayed);
                }
                if TestingStationUsed::can_be_played(self, player) {
                    result.push(EventType::TestingStationUsed);
                }
            }
            Phase::Thumper => {
                if self.player(player).has_treachery_card_type(TreacheryCardType::Thumper) {
                    result.push(EventType::ThumperPlayed);
                }
            }
            Phase::VoteAllianceA | Phase::VoteAllianceB => {
                if self.blow.nexus_votes.iter().all(|v| v.initiator != faction) {
                    result.push(EventType::NexusVoted);
                }
            }
            Phase::HarvesterA | Phase::HarvesterB => {
                if self.player(player).has_treachery_card_type(TreacheryCardType::Harvester) {
                    result.push(EventType::HarvesterPlayed);
                }
            }
            Phase::StormLosses => {
                if faction == TakeLosses::losses_to_take(self).faction {
                    result.push(EventType::TakeLosses);
                }
            }
            Phase::FremenSendingMonsterA | Phase::FremenSendingMonsterB => {
                if faction == Some(Faction::Fremen) {
                    result.push(EventType::FremenSentMonster);
                }
            }
            Phase::FremenRidingMonsterA | Phase::FremenRidingMonsterB => {
                if faction == Some(Faction::Fremen) {
                    result.push(EventType::FremenRidesMonster);
                }
            }
            Phase::AllianceA | Phase::AllianceB => {
                if !self.turn.has_acted_or_passed.contains_opt(faction) {
                    if self.player(player).ally == None && self.players.len() > 1 {
                        result.push(EventType::AllianceOffered);
                    }
                    if self.player(player).ally != None {
                        result.push(EventType::AllianceBroken);
                    }
                    result.push(EventType::AlliancePassed);
                }
            }
            Phase::NexusCards => {
                if NexusCardDrawn::applicable(self, player) {
                    result.push(EventType::NexusCardDrawn);
                }
            }
            Phase::BlowReport => {}
            Phase::ClaimingCharity => {
                if !is_host && faction == Some(Faction::Atreides) {
                    result.push(EventType::EndPhase);
                }
                if CharityClaimed::can_be_played(self, player) {
                    result.push(EventType::CharityClaimed);
                }
            }
            Phase::BlackMarketAnnouncement => {
                if faction == Some(Faction::Richese) {
                    result.push(EventType::RicheseAnnouncesBlackMarket);
                }
            }
            Phase::BlackMarketBidding => {
                if BlackMarketBid::may_be_played(self, player) {
                    result.push(EventType::BlackMarketBid);
                }
                if faction == Some(Faction::Emperor) && self.applicable(Rule::EmperorSupportingNonAllyBids) {
                    result.push(EventType::EmperorBidSupport);
                }
            }
            Phase::RicheseAnnouncingAuction => {
                if faction == Some(Faction::Richese) {
                    result.push(EventType::RicheseAnnouncesAuction);
                }
            }
            Phase::RicheseSpecifyingAuction => {
                if RicheseSpecifiesAuction::may_specify_card(self, player) || RicheseSpecifiesAuction::may_specify_type_of_auction(self, player) {
                    result.push(EventType::RicheseSpecifiesAuction);
                }
            }
            Phase::Bidding => {
                if Bid::may_be_played(self, Some(player)) {
                    result.push(EventType::Bid);
                }
                if faction == Some(Faction::Emperor) && self.applicable(Rule::EmperorSupportingNonAllyBids) {
                    result.push(EventType::EmperorBidSupport);
                }
            }
            Phase::RicheseKeepingUnsoldCard => {
                if faction == Some(Faction::Richese) {
                    result.push(EventType::RicheseKeepsUnsoldCard);
                }
            }
            Phase::IxianRemovingCardFromBid => {
                let occupier_of_ixian_homeworld = self.occupier_of(World::Ixian);
                if (occupier_of_ixian_homeworld.is_none() && faction == Some(Faction::Ixian)) || occupier_of_ixian_homeworld == Some(player) {
                    result.push(EventType::IxianRemovedCardFromAuction);
                }
            }
            Phase::IxianSwappingCard => {
                if faction == Some(Faction::Ixian) {
                    result.push(EventType::IxianSwappedCardOnBid);
                }
            }
            Phase::ReplacingCardJustWon => {
                if faction == self.bidding.faction_that_may_replace_bought_card {
                    result.push(EventType::ReplacedCardWon);
                }
            }
            Phase::WaitingForNextBiddingRound => {
                if !is_host && faction == Some(Faction::Atreides) {
                    result.push(EventType::EndPhase);
                }
            }
            Phase::BiddingReport => {
                if faction == Some(Faction::Tleilaxu)
                    && self.players.len() > 1
                    && !self.prevented(FactionAdvantage::TleilaxuIncreasingRevivalLimits)
                {
                    result.push(EventType::SetIncreasedRevivalLimits);
                }
                // Kaitain's high threshold. Salusa Secundus has no discard.
                if faction == Some(Faction::Emperor) && self.player(player).has_high_threshold(self, World::Emperor) && i32::from(self.player(player).resources) >= 2 {
                    result.push(EventType::EmperorDiscarded);
                }
            }

            Phase::BeginningOfResurrection => {
                if faction == Some(Faction::Tleilaxu)
                    && self.players.len() > 1
                    && !self.prevented(FactionAdvantage::TleilaxuIncreasingRevivalLimits)
                {
                    result.push(EventType::SetIncreasedRevivalLimits);
                }
                if RecruitsPlayed::is_applicable(self, player) {
                    result.push(EventType::RecruitsPlayed);
                }
            }

            Phase::Resurrection => {
                if self.is_playing(Faction::Tleilaxu)
                    && faction != Some(Faction::Tleilaxu)
                    && !self.killed_heroes(player).is_empty()
                    && Revival::normally_revivable_heroes(self, player).is_empty()
                {
                    result.push(EventType::RequestTleilaxuRevival);
                }

                if faction == Some(Faction::Tleilaxu) && (!self.revival.current_revival_requests.is_empty() || !self.revival.early_revivals_offers.is_empty()) {
                    result.push(EventType::AcceptOrCancelTleilaxuRevival);
                }
                if self.has_something_to_revive(player) && !self.prevented_from_reviving(faction) {
                    result.push(EventType::Revival);
                }
                if faction == Some(Faction::Tleilaxu)
                    && self.players.len() > 1
                    && !self.prevented(FactionAdvantage::TleilaxuIncreasingRevivalLimits)
                {
                    result.push(EventType::SetIncreasedRevivalLimits);
                }
            }

            Phase::ResurrectionReport => {
                if faction == Some(Faction::Ecaz) && AmbassadorPlaced::is_applicable(self, player) {
                    result.push(EventType::AmbassadorPlaced);
                }
            }

            Phase::GuildShip => {
                if faction == Some(Faction::Guild) {
                    if self.guild_may_delay() {
                        result.push(EventType::GuildDelay);
                    }
                    result.push(EventType::Shipment);
                    if has_caravan(self) && !PlacementEvent::valid_movement_sources(self, player).is_empty() {
                        result.push(EventType::Caravan);
                    }
                }
            }
            Phase::BeneGesseritAccompaniesGuildShip | Phase::BeneGesseritAccompaniesNonGuildShip => {
                if faction == Some(Faction::BeneGesserit) {
                    result.push(EventType::BeneGesseritAccompanies);
                }
            }
            Phase::NonGuildShip => {
                if self.is_current_shipment_and_move_player(player) {
                    result.push(EventType::Shipment);
                    if has_caravan(self) {
                        result.push(EventType::Caravan);
                    }
                }
            }
            Phase::GuildMove => {
                if faction == Some(Faction::Guild) {
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
                if faction == Some(Faction::BeneGesserit) {
                    result.push(EventType::BeneGesseritFlip);
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
                if faction == Some(Faction::Moritani) {
                    result.push(EventType::TerrorRevealed);
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
                if faction == Some(Faction::Ecaz) || (self.player(player).ally == Some(Faction::Ecaz) && self.alliances.ecaz_shares_ambassadors) {
                    result.push(EventType::AmbassadorActivated);
                }
            }

            Phase::ShipmentAndMoveConcluded => {
            }

            Phase::BattlePhase => {
                if self.battle.current_battle.is_none() && Some(player) == self.next_player_to_battle() {
                    result.push(EventType::BattleInitiated);
                }

                if self.battle.current_battle.is_some() && SwitchedSkilledLeader::can_be_played(self, player) {
                    result.push(EventType::SwitchedSkilledLeader);
                }

                if HMSAdvantageChosen::can_be_played(self, player) {
                    result.push(EventType::HMSAdvantageChosen);
                }

                if Voice::may_use_voice(self, player) {
                    result.push(EventType::Voice);
                }

                if VoiceDeclined::may_decline(self, player) {
                    result.push(EventType::VoiceDeclined);
                }

                if Thought::may_be_used(self, player) {
                    result.push(EventType::Thought);
                }

                if Prescience::may_use_prescience(self, player) {
                    result.push(EventType::Prescience);
                }

                if let Some(cb) = self.battle.current_battle.clone() {
                    if faction == cb.aggressor(self) && self.battle.aggressor_plan.is_none() {
                        result.push(EventType::Battle);
                    } else if faction == cb.defender(self) && self.battle.defender_plan.is_none() {
                        result.push(EventType::Battle);
                    }

                    if faction == cb.aggressor(self) && self.battle.aggressor_plan.is_some() {
                        result.push(EventType::BattleRevision);
                    } else if faction == cb.defender(self) && self.battle.defender_plan.is_some() {
                        result.push(EventType::BattleRevision);
                    }

                    if ResidualPlayed::may_play(self, player) {
                        result.push(EventType::ResidualPlayed);
                    }
                }

            }

            Phase::ClaimingBattle => {
                if self.has_low_threshold(Faction::Ecaz) {
                    let opponent = match &self.battle.battle_about_to_start {
                        Some(b) => b.opponent_of_faction(self, Faction::Ecaz),
                        None => None,
                    };
                    if opponent.is_some_and(|o| self.player(o).is(faction)) {
                        result.push(EventType::BattleClaimed);
                    }
                } else if faction == Some(Faction::Ecaz) {
                    result.push(EventType::BattleClaimed);
                }
            }

            Phase::CallTraitorOrPass => {
                if let (Some(cb), Some(ap), Some(dp)) = (&self.battle.current_battle, &self.battle.aggressor_plan, &self.battle.defender_plan) {
                    let traitors = &self.player(player).traitors;
                    let ally_of = |f: Faction| self.get_player(Some(f)).map(|p| self.player(p).ally);
                    if (self.battle.aggressor_traitor_action.is_none() && faction == cb.aggressor(self))
                        || (self.battle.aggressor_traitor_action.is_none()
                            && faction == Some(Faction::Harkonnen)
                            && ally_of(ap.initiator().expect("faction")) == Some(Some(Faction::Harkonnen))
                            && dp.hero.is_some()
                            && traitors.contains(&dp.hero.expect("hero")))
                        || (self.battle.defender_traitor_action.is_none() && faction == cb.defender(self))
                        || (self.battle.defender_traitor_action.is_none()
                            && faction == Some(Faction::Harkonnen)
                            && ally_of(dp.initiator().expect("faction")) == Some(Some(Faction::Harkonnen))
                            && ap.hero.is_some()
                            && traitors.contains(&ap.hero.expect("hero")))
                    {
                        result.push(EventType::TreacheryCalled);
                    }
                }

                if let Some(ap) = &self.battle.aggressor_plan {
                    if faction == ap.initiator()
                        && ap.weapon().is_some_and(|w| w.type_() == TreacheryCardType::PoisonTooth)
                        && !self.battle.poison_tooth_cancelled
                    {
                        result.push(EventType::PoisonToothCancelled);
                    }
                }

                if let Some(dp) = &self.battle.defender_plan {
                    if faction == dp.initiator()
                        && dp.weapon().is_some_and(|w| w.type_() == TreacheryCardType::PoisonTooth)
                        && !self.battle.poison_tooth_cancelled
                    {
                        result.push(EventType::PoisonToothCancelled);
                    }
                }

                if PortableAntidoteUsed::can_be_played(self, player) {
                    result.push(EventType::PortableAntidoteUsed);
                }

                if Diplomacy::can_be_played(self, player) {
                    result.push(EventType::Diplomacy);
                }
            }

            Phase::Retreating => {
                if Some(faction) == self.battle.battle_loser {
                    result.push(EventType::Retreat);
                }
            }

            Phase::MeltingRock => {
                if RockWasMelted::can_be_played(self, player) {
                    result.push(EventType::RockWasMelted);
                }
            }

            Phase::CaptureDecision => {
                if faction == Some(Faction::Harkonnen) {
                    result.push(EventType::Captured);
                }
            }

            Phase::BattleConclusion => {
                if Some(faction) == self.battle.battle_winner && !self.battle.battle_was_concluded_by_winner {
                    result.push(EventType::BattleConcluded);
                }
                if LoserConcluded::is_applicable(self, Some(player)) {
                    result.push(EventType::LoserConcluded);
                }
            }

            Phase::AvoidingAudit => {
                if Some(player) == self.auditee() {
                    result.push(EventType::AuditCancelled);
                }
            }

            Phase::Auditing => {
                if faction == Some(Faction::Choam) {
                    result.push(EventType::Audited);
                }
            }

            Phase::RevealingFacedancer => {
                if faction == Some(Faction::Tleilaxu) {
                    result.push(EventType::FaceDancerRevealed);
                }
            }

            Phase::Facedancing => {
                if faction == Some(Faction::Tleilaxu) {
                    result.push(EventType::FaceDanced);
                }
            }

            Phase::BattleReport => {
                // NextPlayerToBattle mutates BattleSequence; only read when the
                // preceding conditions hold.
            }

            Phase::DividingCollectedResources => {
                if DivideResources::is_applicable(self, player) {
                    result.push(EventType::DivideResources);
                }
            }

            Phase::AcceptingResourceDivision => {
                if DivideResourcesAccepted::is_applicable(self, player) {
                    result.push(EventType::DivideResourcesAccepted);
                }
            }

            Phase::ReplacingFaceDancer => {
                if faction == Some(Faction::Tleilaxu) {
                    result.push(EventType::FaceDancerReplaced);
                }
            }

            Phase::Extortion => {
                if ExtortionPrevented::can_be_played(self, player) {
                    result.push(EventType::ExtortionPrevented);
                }
            }

            Phase::BeginningOfCollection | Phase::CollectionReport => {
                if faction == Some(Faction::Moritani) && TerrorPlanted::is_applicable(self, player) {
                    result.push(EventType::TerrorPlanted);
                }
            }

            Phase::Contemplate => {
                if faction == Some(Faction::Choam)
                    && !self.prevented(FactionAdvantage::ChoamEconomics)
                    && self.charity.economics_status == None
                {
                    result.push(EventType::ChoamEconomics);
                }
                if faction == Some(Faction::Moritani) && TerrorPlanted::is_applicable(self, player) {
                    result.push(EventType::TerrorPlanted);
                }
            }

            Phase::PerformingKarmaHandSwap => {
                if faction == Some(Faction::Harkonnen) && !self.karma_prevented(faction) {
                    result.push(EventType::KarmaHandSwap);
                }
            }

            Phase::TradingCards => {
                if Some(faction) == self.alliances.current_card_trade_offer.as_ref().map(|o| o.target) {
                    result.push(EventType::CardTraded);
                }
            }

            Phase::Clairvoyance => {
                if Some(faction) == self.turn.latest_clairvoyance.as_ref().map(|c| c.target) {
                    result.push(EventType::ClairVoyanceAnswered);
                }
            }

            Phase::Thought => {
                // `CurrentBattle?.OpponentOf(CurrentThought!.Initiator)`: the argument (and its
                // NullReferenceException) is only evaluated when CurrentBattle is not null.
                let opponent = match &self.battle.current_battle {
                    Some(cb) => cb.opponent_of_faction(self, self.battle.current_thought.as_ref().expect("current thought").initiator()),
                    None => None,
                };
                if Some(player) == opponent {
                    result.push(EventType::ThoughtAnswered);
                }
            }

            Phase::SearchingDiscarded => {
                if DiscardedSearched::can_be_played(self, player) {
                    result.push(EventType::DiscardedSearched);
                }
            }

            _ => {}
        }

        if self.turn.current_phase > Phase::TradingFactions {
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
        let has_finalized_battle_plan_waiting_to_be_resolved = (self.turn.current_phase == Phase::BattlePhase
            || self.turn.current_phase == Phase::MeltingRock
            || self.turn.current_phase == Phase::CallTraitorOrPass)
            && self.battle.current_battle.as_ref().is_some_and(|cb| cb.plan_of(self, Some(player)).is_some());

        if JuicePlayed::can_be_played_by(self, player) {
            result.push(EventType::JuicePlayed);
        }

        if CardGiven::is_applicable(self, player) {
            result.push(EventType::CardGiven);
        }

        if faction == Some(Faction::Ecaz)
            && self.turn.current_phase == Phase::CallTraitorOrPass
            && self.battle.current_battle.is_some()
            && self.battle.current_battle.as_ref().expect("current battle").is_aggressor_or_defender(self, player)
            && !self.karma_prevented(faction)
            && !self.player(player).special_karma_power_used
            && self.player(player).has_karma(self)
            && self.applicable(Rule::AdvancedKarama)
        {
            let plan_of_ecaz = self.battle.current_battle.as_ref().expect("current battle").plan_of_faction(self, Faction::Ecaz);
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

            if self.turn.current_phase != Phase::SearchingDiscarded && DiscardedSearchedAnnounced::can_be_played(self, player) {
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
                && self.battle.current_battle.is_some()
                && self.battle.current_battle.as_ref().expect("current battle").is_involved(self, player)
                && !self.karma_prevented(faction)
                && !p.special_karma_power_used
                && p.has_karma(self)
                && self.applicable(Rule::AdvancedKarama)
            {
                result.push(EventType::KarmaPrescience);
            }

            if faction == Some(Faction::BeneGesserit)
                && self.turn.current_phase > Phase::AllianceB
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
                && self.battle.aggressor_plan.as_ref().is_none_or(|ap| faction != ap.initiator())
                && self.battle.defender_plan.as_ref().is_none_or(|dp| faction != dp.initiator())
            {
                result.push(EventType::Donated);
            }

            if faction == Some(Faction::Richese)
                && self.turn.current_phase != Phase::BlackMarketBidding
                && self.turn.current_phase != Phase::Bidding
                && !has_finalized_battle_plan_waiting_to_be_resolved
                && p.has_ally()
                && !RicheseGaveCard::valid_cards(self, player).next().is_none()
                && self.player(p.allied_player(self).expect("allied player")).has_room_for_cards(self)
            {
                result.push(EventType::RicheseGaveCard);
            }

            if self.turn.current_phase != Phase::BlackMarketBidding
                && (self.turn.current_phase != Phase::Bidding || self.bidding.current_bid.is_none())
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
    fn is_current_shipment_and_move_player(&self, player: PlayerId) -> bool {
        match &self.ship_move.shipment_and_move_sequence {
            Some(seq) => seq.current_player(self) == Some(player),
            None => false,
        }
    }

    /// The actions shared by the `GuildMove` and `NonGuildMove` cases.
    fn add_move_actions(&self, player: PlayerId, result: &mut Vec<EventType>) {
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
            || self.turn.current_phase == Phase::Discarding
            || self.turn.current_phase == Phase::AssigningSkill
            || self.turn.current_phase == Phase::Clairvoyance
            || self.turn.current_phase == Phase::Thought
            || self.turn.current_phase == Phase::TradingCards
            || self.turn.current_phase == Phase::Bureaucracy
            || self.turn.current_phase == Phase::SearchingDiscarded
            || self.turn.current_phase == Phase::PerformingKarmaHandSwap
            || self.turn.current_phase == Phase::ReplacingCardJustWon
            || self.turn.current_phase == Phase::DividingCollectedResources
            || self.turn.current_phase == Phase::AcceptingResourceDivision
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
