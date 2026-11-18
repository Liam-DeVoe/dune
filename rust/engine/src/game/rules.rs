use crate::prelude::*;

/// The game's version, rules and settings, and the factions allowed to play.
#[derive(Debug)]
pub struct Rules {
    pub version: i32,
    pub ruleset: Option<Ruleset>,
    pub active: Vec<Rule>,
    pub rules_for_bots: Vec<Rule>,
    /// `EstablishPlayers` can assign a null `Settings.InitialRules`, hence the `Option`.
    pub all_rules: Option<Vec<Rule>>,
    /// `EstablishPlayers` can assign null, hence the `Option`.
    pub settings: Option<GameSettings>,
    pub factions_in_play: Vec<Faction>,
    pub automation_rules: Vec<AutomationRuleState>,
    pub secrets_remain_hidden: Vec<Faction>,
}

impl Rules {
    pub fn new(version: i32) -> Self {
        Rules {
            version: version,
            ruleset: Default::default(),
            active: Vec::new(),
            rules_for_bots: Vec::new(),
            all_rules: Some(Vec::new()),
            settings: Some(Default::default()),
            factions_in_play: Default::default(),
            automation_rules: Vec::new(),
            secrets_remain_hidden: Vec::new(),
        }
    }
}

impl Game {
    pub const EXPANSION_LEVEL: i32 = 3;

    pub fn get_custom_rules(&self) -> Vec<Rule> {
                self.rules.active
            .iter()
            .copied()
            .filter(|rule| {
                matches!(
                    Game::get_rule_group(*rule),
                    Some(RuleGroup::House)
                        | Some(RuleGroup::CoreBasicExceptions)
                        | Some(RuleGroup::CoreAdvancedExceptions)
                        | Some(RuleGroup::ExpansionIxAndBtBasicExceptions)
                        | Some(RuleGroup::ExpansionIxAndBtAdvancedExceptions)
                        | Some(RuleGroup::ExpansionChoamAndRicheseBasicExceptions)
                        | Some(RuleGroup::ExpansionChoamAndRicheseAdvancedExceptions)
                        | Some(RuleGroup::ExpansionEcazAndMoritaniAdvancedExceptions)
                        | Some(RuleGroup::ExpansionEcazAndMoritaniBasicExceptions)
                )
            })
            .collect()
    }

    pub fn determine_approximate_ruleset(factions: &[Faction], rules: &[Rule], expansion_level: i32) -> Ruleset {
                let has_expansion1 = factions.contains(&Faction::Tleilaxu)
            || factions.contains(&Faction::Ixian)
            || rules.contains(&Rule::ExpansionTreacheryCardsPBandSs)
            || rules.contains(&Rule::ExpansionTreacheryCardsAmal)
            || rules.contains(&Rule::ExpansionTreacheryCardsExceptPBandSSandAmal)
            || rules.contains(&Rule::TechTokens);

        let has_expansion2 = factions.contains(&Faction::Choam)
            || factions.contains(&Faction::Richese)
            || rules.contains(&Rule::LeaderSkills)
            || rules.contains(&Rule::Expansion2TreacheryCards)
            || rules.contains(&Rule::StrongholdBonus);

        let has_expansion3 = factions.contains(&Faction::Ecaz)
            || factions.contains(&Faction::Moritani)
            || rules.contains(&Rule::Expansion3TreacheryCards)
            || rules.contains(&Rule::GreatMaker)
            || rules.contains(&Rule::DiscoveryTokens)
            || rules.contains(&Rule::Homeworlds)
            || rules.contains(&Rule::NexusCards);

        if Game::advanced_rules_apply(rules) {
            if expansion_level == 3 && has_expansion1 && has_expansion2 && has_expansion3 {
                return Ruleset::AllExpansionsAdvancedGame;
            }
            if expansion_level == 2 && has_expansion1 && has_expansion2 {
                return Ruleset::AllExpansionsAdvancedGame;
            }
            if expansion_level == 1 && has_expansion1 {
                return Ruleset::AllExpansionsAdvancedGame;
            }
            if has_expansion3 {
                return Ruleset::Expansion3AdvancedGame;
            }
            if has_expansion2 {
                return Ruleset::Expansion2AdvancedGame;
            }
            if has_expansion1 {
                return Ruleset::ExpansionAdvancedGame;
            }
            return Ruleset::AdvancedGame;
        }

        if expansion_level == 3 && has_expansion1 && has_expansion2 && has_expansion3 {
            return Ruleset::AllExpansionsBasicGame;
        }
        if expansion_level == 2 && has_expansion1 && has_expansion2 {
            return Ruleset::AllExpansionsBasicGame;
        }
        if expansion_level == 1 && has_expansion1 {
            return Ruleset::AllExpansionsBasicGame;
        }
        if has_expansion3 {
            return Ruleset::Expansion3BasicGame;
        }
        if has_expansion2 {
            return Ruleset::Expansion2BasicGame;
        }
        if has_expansion1 {
            return Ruleset::ExpansionBasicGame;
        }
        Ruleset::BasicGame
    }

    pub fn advanced_rules_apply(rules: &[Rule]) -> bool {
                rules.contains(&Rule::AdvancedCombat)
            || rules.contains(&Rule::IncreasedResourceFlow)
            || rules.contains(&Rule::AdvancedKarama)
            || rules.contains(&Rule::AtreidesMessiah)
            || rules.contains(&Rule::HarkonnenCapturesOrKillsLeaders)
            || rules.contains(&Rule::FremenSpecialForces)
            || rules.contains(&Rule::EmperorSpecialForces)
            || rules.contains(&Rule::GuildDetermineShipment)
            || rules.contains(&Rule::BeneGesseritAdvisors)
            || rules.contains(&Rule::IxianSwappingCardOnBid)
            || rules.contains(&Rule::TleilaxuGholas)
            || rules.contains(&Rule::ChoamAuditor)
            || rules.contains(&Rule::RicheseBlackMarket)
            || rules.contains(&Rule::MoritaniAssassinate)
            || rules.contains(&Rule::EcazLoyalty)
            || rules.contains(&Rule::StrongholdBonus)
    }

    /// A constant table, rebuilt on each call.
    pub fn ruleset_definition() -> IndexMap<Ruleset, Vec<Rule>> {
        // A constant table, built fresh on each call.
        const EXPANSION_LEVEL: i32 = Game::EXPANSION_LEVEL;
        let core_advanced = [
            Rule::AdvancedCombat,
            Rule::IncreasedResourceFlow,
            Rule::AdvancedKarama,
            Rule::FremenDeterminesStorm,
            Rule::FremenStormLosses,
            Rule::FremenSendingMonster,
            Rule::FremenSpecialForces,
            Rule::AtreidesMessiah,
            Rule::HarkonnenCapturesOrKillsLeaders,
            Rule::BeneGesseritAutoCharity,
            Rule::BeneGesseritWorthlessAsKarma,
            Rule::BeneGesseritAdvisors,
            Rule::GuildDetermineShipment,
            Rule::EmperorSpecialForces,
        ];
        fn concat(parts: &[&[Rule]]) -> Vec<Rule> {
            parts.iter().flat_map(|p| p.iter().copied()).collect()
        }
        let empty: &[Rule] = &[];

        let mut d = IndexMap::new();
        d.insert(Ruleset::BasicGame, vec![Rule::BasicTreacheryCards, Rule::HasCharityPhase]);

        d.insert(
            Ruleset::AdvancedGame,
            concat(&[
                &core_advanced,
                if EXPANSION_LEVEL < 1 { empty } else { &[Rule::IxianSwappingCardOnBid, Rule::TleilaxuGholas] },
                if EXPANSION_LEVEL < 2 { empty } else { &[Rule::ChoamAuditor, Rule::RicheseBlackMarket] },
                if EXPANSION_LEVEL < 3 { empty } else { &[Rule::MoritaniAssassinate, Rule::EcazLoyalty] },
            ]),
        );

        d.insert(
            Ruleset::AllExpansionsBasicGame,
            concat(&[
                if EXPANSION_LEVEL < 1 {
                    empty
                } else {
                    &[Rule::TechTokens, Rule::CheapHeroTraitor, Rule::ExpansionTreacheryCards, Rule::SandTrout]
                },
                if EXPANSION_LEVEL < 2 { empty } else { &[Rule::LeaderSkills, Rule::Expansion2TreacheryCards] },
                if EXPANSION_LEVEL < 3 {
                    empty
                } else {
                    &[Rule::Expansion3TreacheryCards, Rule::DiscoveryTokens, Rule::Homeworlds, Rule::NexusCards, Rule::GreatMaker]
                },
            ]),
        );

        d.insert(
            Ruleset::ExpansionBasicGame,
            vec![Rule::TechTokens, Rule::CheapHeroTraitor, Rule::ExpansionTreacheryCards, Rule::SandTrout],
        );

        d.insert(Ruleset::Expansion2BasicGame, vec![Rule::LeaderSkills, Rule::Expansion2TreacheryCards]);

        d.insert(
            Ruleset::Expansion3BasicGame,
            vec![Rule::Expansion3TreacheryCards, Rule::DiscoveryTokens, Rule::Homeworlds, Rule::NexusCards, Rule::GreatMaker],
        );

        d.insert(
            Ruleset::AllExpansionsAdvancedGame,
            concat(&[
                &core_advanced,
                if EXPANSION_LEVEL < 1 {
                    empty
                } else {
                    &[
                        Rule::TechTokens,
                        Rule::CheapHeroTraitor,
                        Rule::ExpansionTreacheryCards,
                        Rule::SandTrout,
                        Rule::IxianSwappingCardOnBid,
                        Rule::TleilaxuGholas,
                    ]
                },
                if EXPANSION_LEVEL < 2 {
                    empty
                } else {
                    &[
                        Rule::LeaderSkills,
                        Rule::Expansion2TreacheryCards,
                        Rule::StrongholdBonus,
                        Rule::ChoamAuditor,
                        Rule::RicheseBlackMarket,
                    ]
                },
                if EXPANSION_LEVEL < 3 {
                    empty
                } else {
                    &[
                        Rule::Expansion3TreacheryCards,
                        Rule::DiscoveryTokens,
                        Rule::Homeworlds,
                        Rule::NexusCards,
                        Rule::GreatMaker,
                        Rule::MoritaniAssassinate,
                        Rule::EcazLoyalty,
                    ]
                },
            ]),
        );

        d.insert(
            Ruleset::ExpansionAdvancedGame,
            concat(&[
                &core_advanced,
                if EXPANSION_LEVEL < 1 {
                    empty
                } else {
                    &[
                        Rule::TechTokens,
                        Rule::CheapHeroTraitor,
                        Rule::ExpansionTreacheryCards,
                        Rule::SandTrout,
                        Rule::IxianSwappingCardOnBid,
                        Rule::TleilaxuGholas,
                        Rule::ChoamAuditor,
                        Rule::RicheseBlackMarket,
                        Rule::MoritaniAssassinate,
                        Rule::EcazLoyalty,
                    ]
                },
            ]),
        );

        d.insert(
            Ruleset::Expansion2AdvancedGame,
            concat(&[
                &core_advanced,
                if EXPANSION_LEVEL < 2 {
                    empty
                } else {
                    &[
                        Rule::LeaderSkills,
                        Rule::Expansion2TreacheryCards,
                        Rule::StrongholdBonus,
                        Rule::IxianSwappingCardOnBid,
                        Rule::TleilaxuGholas,
                        Rule::ChoamAuditor,
                        Rule::RicheseBlackMarket,
                        Rule::MoritaniAssassinate,
                        Rule::EcazLoyalty,
                    ]
                },
            ]),
        );

        d.insert(
            Ruleset::Expansion3AdvancedGame,
            concat(&[
                &core_advanced,
                if EXPANSION_LEVEL < 3 {
                    empty
                } else {
                    &[
                        Rule::DiscoveryTokens,
                        Rule::GreatMaker,
                        Rule::NexusCards,
                        Rule::Expansion3TreacheryCards,
                        Rule::Homeworlds,
                        Rule::IxianSwappingCardOnBid,
                        Rule::TleilaxuGholas,
                        Rule::ChoamAuditor,
                        Rule::RicheseBlackMarket,
                        Rule::MoritaniAssassinate,
                        Rule::EcazLoyalty,
                    ]
                },
            ]),
        );

        d.insert(
            Ruleset::ServerClassic,
            vec![
                Rule::AdvancedCombat,
                Rule::IncreasedResourceFlow,
                Rule::AdvancedKarama,
                Rule::FremenDeterminesStorm,
                Rule::FremenStormLosses,
                Rule::FremenSendingMonster,
                Rule::FremenSpecialForces,
                Rule::AtreidesMessiah,
                Rule::HarkonnenCapturesOrKillsLeaders,
                Rule::BeneGesseritAutoCharity,
                Rule::BeneGesseritWorthlessAsKarma,
                Rule::BeneGesseritAdvisors,
                Rule::GuildDetermineShipment,
                Rule::EmperorSpecialForces,
                Rule::GuildShipmentContributionsFlowBack,
                Rule::FullPhaseKarma,
                Rule::BeneGesseritVoiceMustNameSpecialCards,
                Rule::BattlesUnderStorm,
                Rule::AdvisorsDontConflictWithAlly,
                Rule::FremenMayMoveIntoStorm,
                Rule::Ssw,
                Rule::HarkonnenMulligan,
            ],
        );

        d
    }

    pub fn get_rules_in_group(group: Option<RuleGroup>, expansion_level: i32) -> Vec<Rule> {
                get_values::<Rule>()
            .filter(|r| Game::get_rule_group(*r) == group && Game::get_rule_expansion(*r) <= expansion_level)
            .collect()
    }

    pub fn get_available_rulesets() -> Vec<Ruleset> {
                get_values::<Ruleset>().filter(|rs| Game::get_ruleset_expansion(Some(*rs)) <= Game::EXPANSION_LEVEL).collect()
    }

    pub fn applicable(&self, rule: Rule) -> bool {
                self.rules.active.contains(&rule) || self.rules.rules_for_bots.contains(&rule)
    }

    pub fn get_rule_group(rule: Rule) -> Option<RuleGroup> {
                match rule {
            Rule::HasCharityPhase | Rule::BasicTreacheryCards => Some(RuleGroup::CoreBasic),
            Rule::AdvancedCombat
            | Rule::IncreasedResourceFlow
            | Rule::AdvancedKarama
            | Rule::FremenDeterminesStorm
            | Rule::FremenStormLosses
            | Rule::FremenSendingMonster
            | Rule::FremenSpecialForces
            | Rule::AtreidesMessiah
            | Rule::HarkonnenCapturesOrKillsLeaders
            | Rule::BeneGesseritAutoCharity
            | Rule::BeneGesseritWorthlessAsKarma
            | Rule::BeneGesseritAdvisors
            | Rule::GuildDetermineShipment
            | Rule::EmperorSpecialForces => Some(RuleGroup::CoreAdvanced),
            Rule::BribesAreImmediate
            | Rule::ContestedStongholdsCountAsOccupied
            | Rule::GuildShipmentContributionsFlowBack
            | Rule::FullPhaseKarma
            | Rule::BeneGesseritVoiceMustNameSpecialCards
            | Rule::BattlesUnderStorm
            | Rule::MovementBonusRequiresOccupationBeforeMovement => Some(RuleGroup::CoreBasicExceptions),
            Rule::AdvisorsDontConflictWithAlly | Rule::FremenMayMoveIntoStorm => Some(RuleGroup::CoreAdvancedExceptions),
            Rule::CustomInitialForcesAndResources
            | Rule::CustomDecks
            | Rule::HmSwithoutIxian
            | Rule::StormDeckWithoutFremen
            | Rule::Ssw
            | Rule::HarkonnenMulligan
            | Rule::ExtraKaramaCards
            | Rule::CardsCanBeTraded
            | Rule::PlayersChooseFactions
            | Rule::EmperorSupportingNonAllyBids
            | Rule::AssistedNotekeeping
            | Rule::AssistedNotekeepingForAtreides
            | Rule::ResourceBonusForStrongholds
            | Rule::BattleWithoutLeader
            | Rule::CapturedLeadersAreTraitorsToOwnFaction
            | Rule::DisableEndOfGameReport
            | Rule::DisableGuildSpecialVictory
            | Rule::DisableResourceTransfers
            | Rule::DisableNovaFlipping
            | Rule::FremenAllyGetsDialedResourcesRefunded
            | Rule::BreakAlliancesOnAlliancePhase => Some(RuleGroup::House),
            Rule::BotsCannotAlly => Some(RuleGroup::Bots),
            Rule::TechTokens | Rule::CheapHeroTraitor | Rule::ExpansionTreacheryCards | Rule::SandTrout => Some(RuleGroup::ExpansionIxAndBtBasic),
            Rule::IxianSwappingCardOnBid | Rule::TleilaxuGholas => Some(RuleGroup::ExpansionIxAndBtAdvanced),
            Rule::LeaderSkills | Rule::Expansion2TreacheryCards => Some(RuleGroup::ExpansionChoamAndRicheseBasic),
            Rule::ChoamAuditor | Rule::RicheseBlackMarket | Rule::StrongholdBonus => Some(RuleGroup::ExpansionChoamAndRicheseAdvanced),
            Rule::Expansion3TreacheryCards | Rule::DiscoveryTokens | Rule::Homeworlds | Rule::NexusCards | Rule::GreatMaker => {
                Some(RuleGroup::ExpansionEcazAndMoritaniBasic)
            }
            Rule::MoritaniAssassinate | Rule::EcazLoyalty => Some(RuleGroup::ExpansionEcazAndMoritaniAdvanced),
            _ => None,
        }
    }

    pub fn get_ruleset_expansion(ruleset: Option<Ruleset>) -> i32 {
                match ruleset {
            None
            | Some(Ruleset::BasicGame)
            | Some(Ruleset::AdvancedGame)
            | Some(Ruleset::ServerClassic)
            | Some(Ruleset::Custom)
            | Some(Ruleset::AllExpansionsBasicGame)
            | Some(Ruleset::AllExpansionsAdvancedGame) => 0,
            Some(Ruleset::ExpansionBasicGame) | Some(Ruleset::ExpansionAdvancedGame) => 1,
            Some(Ruleset::Expansion2BasicGame) | Some(Ruleset::Expansion2AdvancedGame) => 2,
            Some(Ruleset::Expansion3BasicGame) | Some(Ruleset::Expansion3AdvancedGame) => 3,
        }
    }

    pub fn get_rule_expansion(rule: Rule) -> i32 {
                match rule {
            Rule::HasCharityPhase | Rule::BasicTreacheryCards => 0,
            Rule::AdvancedCombat
            | Rule::IncreasedResourceFlow
            | Rule::AdvancedKarama
            | Rule::FremenDeterminesStorm
            | Rule::FremenStormLosses
            | Rule::FremenSendingMonster
            | Rule::FremenSpecialForces
            | Rule::AtreidesMessiah
            | Rule::HarkonnenCapturesOrKillsLeaders
            | Rule::BeneGesseritAutoCharity
            | Rule::BeneGesseritWorthlessAsKarma
            | Rule::BeneGesseritAdvisors
            | Rule::GuildDetermineShipment
            | Rule::EmperorSpecialForces => 0,
            Rule::BribesAreImmediate
            | Rule::ContestedStongholdsCountAsOccupied
            | Rule::GuildShipmentContributionsFlowBack
            | Rule::FullPhaseKarma
            | Rule::BeneGesseritVoiceMustNameSpecialCards
            | Rule::BattlesUnderStorm
            | Rule::MovementBonusRequiresOccupationBeforeMovement => 0,
            Rule::AdvisorsDontConflictWithAlly | Rule::FremenMayMoveIntoStorm => 0,
            Rule::CustomInitialForcesAndResources
            | Rule::CustomDecks
            | Rule::HmSwithoutIxian
            | Rule::StormDeckWithoutFremen
            | Rule::Ssw
            | Rule::HarkonnenMulligan
            | Rule::ExtraKaramaCards
            | Rule::CardsCanBeTraded
            | Rule::PlayersChooseFactions
            | Rule::EmperorSupportingNonAllyBids
            | Rule::AssistedNotekeeping
            | Rule::AssistedNotekeepingForAtreides
            | Rule::ResourceBonusForStrongholds
            | Rule::BattleWithoutLeader
            | Rule::CapturedLeadersAreTraitorsToOwnFaction
            | Rule::DisableEndOfGameReport
            | Rule::DisableGuildSpecialVictory
            | Rule::DisableResourceTransfers
            | Rule::FremenAllyGetsDialedResourcesRefunded
            | Rule::DisableNovaFlipping
            | Rule::BreakAlliancesOnAlliancePhase => 0,
            Rule::FillWithBots | Rule::BotsCannotAlly => 0,
            Rule::TechTokens | Rule::CheapHeroTraitor | Rule::ExpansionTreacheryCards | Rule::SandTrout => 1,
            Rule::IxianSwappingCardOnBid | Rule::TleilaxuGholas => 1,
            Rule::LeaderSkills | Rule::Expansion2TreacheryCards => 2,
            Rule::ChoamAuditor | Rule::RicheseBlackMarket | Rule::StrongholdBonus => 2,
            Rule::GreatMaker
            | Rule::DiscoveryTokens
            | Rule::Homeworlds
            | Rule::NexusCards
            | Rule::Expansion3TreacheryCards
            | Rule::MoritaniAssassinate
            | Rule::EcazLoyalty => 3,
            _ => i32::MAX,
        }
    }

    pub fn is_automated(&self, rule: AutomationRuleType, p: PlayerId) -> bool {
                let faction = self.player(p).faction;
        self.rules.automation_rules.iter().any(|x| x.initiator == faction && x.rule_type == rule)
    }

    pub fn is_auto_passed_bid(&self, f: Faction) -> bool {
                for rule in self.rules.automation_rules.iter().filter(|x| x.initiator == Some(f)) {
            match rule.rule_type {
                AutomationRuleType::BiddingPassAboveAmount => {
                    if let Some(bid) = self.current_bid() {
                        if bid.bid_total_amount() >= i32::from(rule.bidding_above_amount) {
                            return true;
                        }
                    }
                }

                AutomationRuleType::BiddingPassWhenAtreidesOrAtreidesAllyPassed => {
                    if self.bidding().is_some_and(|b| b.latest_bid_by_atreides_or_atreides_ally_was_passed) && f != Faction::Atreides {
                        return true;
                    }
                }

                AutomationRuleType::BiddingPassWhenHighestBidByFaction => {
                    if let Some(bid) = self.current_bid() {
                        if Some(bid.bid_initiator()) == rule.bidding_winning_faction {
                            return true;
                        }
                    }
                }

                _ => {}
            }
        }

        false
    }

}
