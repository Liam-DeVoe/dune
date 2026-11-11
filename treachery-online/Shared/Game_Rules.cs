/*
 * Copyright (C) 2020-2025 Ronald Ossendrijver (admin@treachery.online)
 * This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License
 * as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version. This
 * program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details. You should have
 * received a copy of the GNU General Public License along with this program. If not, see <http://www.gnu.org/licenses/>.
 */

namespace Treachery.Shared;

public partial class Game
{
    public const int ExpansionLevel = 3;

    public IEnumerable<Rule> GetCustomRules()
    {
        return Rules.Where(rule => GetRuleGroup(rule) is
            RuleGroup.House or RuleGroup.CoreBasicExceptions or RuleGroup.CoreAdvancedExceptions or
            RuleGroup.ExpansionIxAndBtBasicExceptions or RuleGroup.ExpansionIxAndBtAdvancedExceptions or
            RuleGroup.ExpansionChoamAndRicheseBasicExceptions or RuleGroup.ExpansionChoamAndRicheseAdvancedExceptions or
            RuleGroup.ExpansionEcazAndMoritaniAdvancedExceptions or RuleGroup.ExpansionEcazAndMoritaniBasicExceptions);
    }

    public static Ruleset DetermineApproximateRuleset(List<Faction> factions, List<Rule> rules, int expansionLevel)
    {
        var hasExpansion1 = factions.Contains(Faction.Tleilaxu) || factions.Contains(Faction.Ixian) ||
                            rules.Contains(Rule.ExpansionTreacheryCardsPBandSs) ||
                            rules.Contains(Rule.ExpansionTreacheryCardsAmal) ||
                            rules.Contains(Rule.ExpansionTreacheryCardsExceptPBandSSandAmal) ||
                            rules.Contains(Rule.TechTokens);

        var hasExpansion2 = factions.Contains(Faction.Choam) || factions.Contains(Faction.Richese) ||
                            rules.Contains(Rule.LeaderSkills) ||
                            rules.Contains(Rule.Expansion2TreacheryCards) ||
                            rules.Contains(Rule.StrongholdBonus);

        var hasExpansion3 = factions.Contains(Faction.Ecaz) || factions.Contains(Faction.Moritani) ||
                            rules.Contains(Rule.Expansion3TreacheryCards) ||
                            rules.Contains(Rule.GreatMaker) ||
                            rules.Contains(Rule.DiscoveryTokens) ||
                            rules.Contains(Rule.Homeworlds) ||
                            rules.Contains(Rule.NexusCards);

        if (AdvancedRulesApply(rules))
        {
            if (expansionLevel == 3 && hasExpansion1 && hasExpansion2 && hasExpansion3)
                return Ruleset.AllExpansionsAdvancedGame;
            if (expansionLevel == 2 && hasExpansion1 && hasExpansion2) return Ruleset.AllExpansionsAdvancedGame;
            if (expansionLevel == 1 && hasExpansion1) return Ruleset.AllExpansionsAdvancedGame;
            if (hasExpansion3) return Ruleset.Expansion3AdvancedGame;
            if (hasExpansion2) return Ruleset.Expansion2AdvancedGame;
            if (hasExpansion1) return Ruleset.ExpansionAdvancedGame;
            return Ruleset.AdvancedGame;
        }

        if (expansionLevel == 3 && hasExpansion1 && hasExpansion2 && hasExpansion3)
            return Ruleset.AllExpansionsBasicGame;
        if (expansionLevel == 2 && hasExpansion1 && hasExpansion2) return Ruleset.AllExpansionsBasicGame;
        if (expansionLevel == 1 && hasExpansion1) return Ruleset.AllExpansionsBasicGame;
        if (hasExpansion3) return Ruleset.Expansion3BasicGame;
        if (hasExpansion2) return Ruleset.Expansion2BasicGame;
        if (hasExpansion1) return Ruleset.ExpansionBasicGame;
        return Ruleset.BasicGame;
    }

    private static bool AdvancedRulesApply(List<Rule> rules)
    {
        return
            rules.Contains(Rule.AdvancedCombat) ||
            rules.Contains(Rule.IncreasedResourceFlow) ||
            rules.Contains(Rule.AdvancedKarama) ||
            rules.Contains(Rule.AtreidesMessiah) ||
            rules.Contains(Rule.HarkonnenCapturesOrKillsLeaders) ||
            rules.Contains(Rule.FremenSpecialForces) ||
            rules.Contains(Rule.EmperorSpecialForces) ||
            rules.Contains(Rule.GuildDetermineShipment) ||
            rules.Contains(Rule.BeneGesseritAdvisors) ||
            rules.Contains(Rule.IxianSwappingCardOnBid) ||
            rules.Contains(Rule.TleilaxuGholas) ||
            rules.Contains(Rule.ChoamAuditor) ||
            rules.Contains(Rule.RicheseBlackMarket) ||
            rules.Contains(Rule.MoritaniAssassinate) ||
            rules.Contains(Rule.EcazLoyalty) ||
            rules.Contains(Rule.StrongholdBonus);
    }

    public static Dictionary<Ruleset, Rule[]> RulesetDefinition { get; private set; } = new()
    {
        [Ruleset.BasicGame] =
        [
            Rule.BasicTreacheryCards,
            Rule.HasCharityPhase
        ],

        [Ruleset.AdvancedGame] = new[]
        {
            Rule.AdvancedCombat,
            Rule.IncreasedResourceFlow,
            Rule.AdvancedKarama,
            Rule.FremenDeterminesStorm,
            Rule.FremenStormLosses,
            Rule.FremenSendingMonster,
            Rule.FremenSpecialForces,
            Rule.AtreidesMessiah,
            Rule.HarkonnenCapturesOrKillsLeaders,
            Rule.BeneGesseritAutoCharity,
            Rule.BeneGesseritWorthlessAsKarma,
            Rule.BeneGesseritAdvisors,
            Rule.GuildDetermineShipment,
            Rule.EmperorSpecialForces
        }.Concat(ExpansionLevel < 1
            ? []
            : [ Rule.IxianSwappingCardOnBid, Rule.TleilaxuGholas]
        ).Concat(ExpansionLevel < 2
            ? []
            : [ Rule.ChoamAuditor, Rule.RicheseBlackMarket ]
        ).Concat(ExpansionLevel < 3
            ? []
            : [ Rule.MoritaniAssassinate, Rule.EcazLoyalty]).ToArray(),

        [Ruleset.AllExpansionsBasicGame] = Array.Empty<Rule>()
            .Concat(ExpansionLevel < 1
                ? []
                : [ Rule.TechTokens, Rule.CheapHeroTraitor, Rule.ExpansionTreacheryCards, Rule.SandTrout]
            ).Concat(ExpansionLevel < 2
                ? []
                : [ Rule.LeaderSkills, Rule.Expansion2TreacheryCards])
            .Concat(ExpansionLevel < 3
                ? []
                : [ Rule.Expansion3TreacheryCards, Rule.DiscoveryTokens, Rule.Homeworlds, Rule.NexusCards, Rule.GreatMaker]).ToArray(),

        [Ruleset.ExpansionBasicGame] =
        [
            Rule.TechTokens,
            Rule.CheapHeroTraitor,
            Rule.ExpansionTreacheryCards,
            Rule.SandTrout
        ],

        [Ruleset.Expansion2BasicGame] =
        [
            Rule.LeaderSkills,
            Rule.Expansion2TreacheryCards
        ],

        [Ruleset.Expansion3BasicGame] =
        [
            Rule.Expansion3TreacheryCards,
            Rule.DiscoveryTokens,
            Rule.Homeworlds,
            Rule.NexusCards,
            Rule.GreatMaker
        ],

        [Ruleset.AllExpansionsAdvancedGame] = new[]
        {
            Rule.AdvancedCombat,
            Rule.IncreasedResourceFlow,
            Rule.AdvancedKarama,
            Rule.FremenDeterminesStorm,
            Rule.FremenStormLosses,
            Rule.FremenSendingMonster,
            Rule.FremenSpecialForces,
            Rule.AtreidesMessiah,
            Rule.HarkonnenCapturesOrKillsLeaders,
            Rule.BeneGesseritAutoCharity,
            Rule.BeneGesseritWorthlessAsKarma,
            Rule.BeneGesseritAdvisors,
            Rule.GuildDetermineShipment,
            Rule.EmperorSpecialForces
        }.Concat(ExpansionLevel < 1
            ? []
            : [ Rule.TechTokens, Rule.CheapHeroTraitor, Rule.ExpansionTreacheryCards, Rule.SandTrout, Rule.IxianSwappingCardOnBid, Rule.TleilaxuGholas ]
        ).Concat(ExpansionLevel < 2
            ? []
            : [Rule.LeaderSkills, Rule.Expansion2TreacheryCards, Rule.StrongholdBonus, Rule.ChoamAuditor, Rule.RicheseBlackMarket ]
        ).Concat(ExpansionLevel < 3
            ? []
            : [ Rule.Expansion3TreacheryCards, Rule.DiscoveryTokens, Rule.Homeworlds, Rule.NexusCards, Rule.GreatMaker, Rule.MoritaniAssassinate, Rule.EcazLoyalty]).ToArray(),

        [Ruleset.ExpansionAdvancedGame] = new[]
        {
            Rule.AdvancedCombat,
            Rule.IncreasedResourceFlow,
            Rule.AdvancedKarama,
            Rule.FremenDeterminesStorm,
            Rule.FremenStormLosses,
            Rule.FremenSendingMonster,
            Rule.FremenSpecialForces,
            Rule.AtreidesMessiah,
            Rule.HarkonnenCapturesOrKillsLeaders,
            Rule.BeneGesseritAutoCharity,
            Rule.BeneGesseritWorthlessAsKarma,
            Rule.BeneGesseritAdvisors,
            Rule.GuildDetermineShipment,
            Rule.EmperorSpecialForces
        }.Concat(ExpansionLevel < 1
            ? []
            : [
            
                Rule.TechTokens,
                Rule.CheapHeroTraitor,
                Rule.ExpansionTreacheryCards,
                Rule.SandTrout,
                Rule.IxianSwappingCardOnBid,
                Rule.TleilaxuGholas,
                Rule.ChoamAuditor,
                Rule.RicheseBlackMarket,
                Rule.MoritaniAssassinate,
                Rule.EcazLoyalty
            ]).ToArray(),

        [Ruleset.Expansion2AdvancedGame] = new[]
        {
            Rule.AdvancedCombat,
            Rule.IncreasedResourceFlow,
            Rule.AdvancedKarama,
            Rule.FremenDeterminesStorm,
            Rule.FremenStormLosses,
            Rule.FremenSendingMonster,
            Rule.FremenSpecialForces,
            Rule.AtreidesMessiah,
            Rule.HarkonnenCapturesOrKillsLeaders,
            Rule.BeneGesseritAutoCharity,
            Rule.BeneGesseritWorthlessAsKarma,
            Rule.BeneGesseritAdvisors,
            Rule.GuildDetermineShipment,
            Rule.EmperorSpecialForces
        }.Concat(ExpansionLevel < 2
            ? []
            : new[]
            {
                Rule.LeaderSkills,
                Rule.Expansion2TreacheryCards,
                Rule.StrongholdBonus,
                Rule.IxianSwappingCardOnBid,
                Rule.TleilaxuGholas,
                Rule.ChoamAuditor,
                Rule.RicheseBlackMarket,
                Rule.StrongholdBonus,
                Rule.MoritaniAssassinate,
                Rule.EcazLoyalty
            }).ToArray(),

        [Ruleset.Expansion3AdvancedGame] = new[]
        {
            Rule.AdvancedCombat,
            Rule.IncreasedResourceFlow,
            Rule.AdvancedKarama,
            Rule.FremenDeterminesStorm,
            Rule.FremenStormLosses,
            Rule.FremenSendingMonster,
            Rule.FremenSpecialForces,
            Rule.AtreidesMessiah,
            Rule.HarkonnenCapturesOrKillsLeaders,
            Rule.BeneGesseritAutoCharity,
            Rule.BeneGesseritWorthlessAsKarma,
            Rule.BeneGesseritAdvisors,
            Rule.GuildDetermineShipment,
            Rule.EmperorSpecialForces
        }.Concat(ExpansionLevel < 3
            ? []
            : [
                Rule.DiscoveryTokens,
                Rule.GreatMaker,
                Rule.NexusCards,
                Rule.Expansion3TreacheryCards,
                Rule.Homeworlds,
                Rule.IxianSwappingCardOnBid,
                Rule.TleilaxuGholas,
                Rule.ChoamAuditor,
                Rule.RicheseBlackMarket,
                Rule.MoritaniAssassinate,
                Rule.EcazLoyalty
            ]).ToArray(),

        [Ruleset.ServerClassic] =
        [
            Rule.AdvancedCombat,
            Rule.IncreasedResourceFlow,
            Rule.AdvancedKarama,
            Rule.FremenDeterminesStorm,
            Rule.FremenStormLosses,
            Rule.FremenSendingMonster,
            Rule.FremenSpecialForces,
            Rule.AtreidesMessiah,
            Rule.HarkonnenCapturesOrKillsLeaders,
            Rule.BeneGesseritAutoCharity,
            Rule.BeneGesseritWorthlessAsKarma,
            Rule.BeneGesseritAdvisors,
            Rule.GuildDetermineShipment,
            Rule.EmperorSpecialForces,
            Rule.GuildShipmentContributionsFlowBack,
            Rule.FullPhaseKarma,
            Rule.BeneGesseritVoiceMustNameSpecialCards,
            Rule.BattlesUnderStorm,
            Rule.AdvisorsDontConflictWithAlly,
            Rule.FremenMayMoveIntoStorm,
            Rule.Ssw,
            Rule.HarkonnenMulligan
        ]
    };

    public static IEnumerable<Rule> GetRulesInGroup(RuleGroup group, int expansionLevel)
    {
        return Enumerations.GetValues<Rule>()
            .Where(r => GetRuleGroup(r) == group && GetRuleExpansion(r) <= expansionLevel);
    }

    public static IEnumerable<Ruleset> GetAvailableRulesets()
    {
        return Enumerations.GetValuesExceptDefault(Ruleset.None).Where(rs => GetRulesetExpansion(rs) <= ExpansionLevel);
    }

    public bool Applicable(Rule rule)
    {
        return Rules.Contains(rule) || RulesForBots.Contains(rule);
    }

    public static RuleGroup GetRuleGroup(Rule rule) => rule switch
    {
        Rule.HasCharityPhase or Rule.BasicTreacheryCards => RuleGroup.CoreBasic,
        Rule.AdvancedCombat or Rule.IncreasedResourceFlow or Rule.AdvancedKarama or Rule.FremenDeterminesStorm
            or Rule.FremenStormLosses or Rule.FremenSendingMonster or Rule.FremenSpecialForces or Rule.AtreidesMessiah
            or Rule.HarkonnenCapturesOrKillsLeaders or Rule.BeneGesseritAutoCharity or Rule.BeneGesseritWorthlessAsKarma
            or Rule.BeneGesseritAdvisors or Rule.GuildDetermineShipment or Rule.EmperorSpecialForces => RuleGroup.CoreAdvanced,
        Rule.BribesAreImmediate or Rule.ContestedStongholdsCountAsOccupied
            or Rule.GuildShipmentContributionsFlowBack or Rule.FullPhaseKarma or Rule.BeneGesseritVoiceMustNameSpecialCards
            or Rule.BattlesUnderStorm
            or Rule.MovementBonusRequiresOccupationBeforeMovement => RuleGroup.CoreBasicExceptions,
        Rule.AdvisorsDontConflictWithAlly or Rule.FremenMayMoveIntoStorm => RuleGroup.CoreAdvancedExceptions,
        Rule.CustomInitialForcesAndResources or Rule.CustomDecks or Rule.HmSwithoutIxian
            or Rule.StormDeckWithoutFremen or Rule.Ssw or Rule.HarkonnenMulligan or Rule.ExtraKaramaCards
            or Rule.CardsCanBeTraded or Rule.PlayersChooseFactions or Rule.EmperorSupportingNonAllyBids
            or Rule.AssistedNotekeeping or Rule.AssistedNotekeepingForAtreides or Rule.ResourceBonusForStrongholds
            or Rule.BattleWithoutLeader or Rule.CapturedLeadersAreTraitorsToOwnFaction
            or Rule.DisableEndOfGameReport or Rule.DisableGuildSpecialVictory or Rule.DisableResourceTransfers
            or Rule.DisableNovaFlipping or Rule.FremenAllyGetsDialedResourcesRefunded
            or Rule.BreakAlliancesOnAlliancePhase => RuleGroup.House,
        Rule.BotsCannotAlly => RuleGroup.Bots,
        Rule.TechTokens or Rule.CheapHeroTraitor or Rule.ExpansionTreacheryCards or Rule.SandTrout => RuleGroup
            .ExpansionIxAndBtBasic,
        Rule.IxianSwappingCardOnBid or Rule.TleilaxuGholas => RuleGroup.ExpansionIxAndBtAdvanced,
        Rule.LeaderSkills or Rule.Expansion2TreacheryCards => RuleGroup.ExpansionChoamAndRicheseBasic,
        Rule.ChoamAuditor or Rule.RicheseBlackMarket or Rule.StrongholdBonus => RuleGroup
            .ExpansionChoamAndRicheseAdvanced,
        Rule.Expansion3TreacheryCards or Rule.DiscoveryTokens or Rule.Homeworlds or Rule.NexusCards
            or Rule.GreatMaker => RuleGroup.ExpansionEcazAndMoritaniBasic,
        Rule.MoritaniAssassinate or Rule.EcazLoyalty => RuleGroup.ExpansionEcazAndMoritaniAdvanced,
        _ => RuleGroup.None
    };

    private static int GetRulesetExpansion(Ruleset ruleset)
        => ruleset switch
        {
            Ruleset.None or Ruleset.BasicGame or Ruleset.AdvancedGame or Ruleset.ServerClassic or Ruleset.Custom
                or Ruleset.AllExpansionsBasicGame or Ruleset.AllExpansionsAdvancedGame => 0,
            Ruleset.ExpansionBasicGame or Ruleset.ExpansionAdvancedGame => 1,
            Ruleset.Expansion2BasicGame or Ruleset.Expansion2AdvancedGame => 2,
            Ruleset.Expansion3BasicGame or Ruleset.Expansion3AdvancedGame => 3,
            _ => int.MaxValue
        };

    private static int GetRuleExpansion(Rule rule)
        => rule switch
        {
            Rule.HasCharityPhase or Rule.BasicTreacheryCards => 0,
            Rule.AdvancedCombat or Rule.IncreasedResourceFlow or Rule.AdvancedKarama or Rule.FremenDeterminesStorm
                or Rule.FremenStormLosses or Rule.FremenSendingMonster or Rule.FremenSpecialForces or Rule.AtreidesMessiah
                or Rule.HarkonnenCapturesOrKillsLeaders or Rule.BeneGesseritAutoCharity or Rule.BeneGesseritWorthlessAsKarma
                or Rule.BeneGesseritAdvisors or Rule.GuildDetermineShipment or Rule.EmperorSpecialForces => 0,
            Rule.BribesAreImmediate or Rule.ContestedStongholdsCountAsOccupied
                or Rule.GuildShipmentContributionsFlowBack or Rule.FullPhaseKarma or Rule.BeneGesseritVoiceMustNameSpecialCards
                or Rule.BattlesUnderStorm or Rule.MovementBonusRequiresOccupationBeforeMovement => 0,
            Rule.AdvisorsDontConflictWithAlly or Rule.FremenMayMoveIntoStorm => 0,
            Rule.CustomInitialForcesAndResources or Rule.CustomDecks or Rule.HmSwithoutIxian
                or Rule.StormDeckWithoutFremen or Rule.Ssw or Rule.HarkonnenMulligan or Rule.ExtraKaramaCards
                or Rule.CardsCanBeTraded or Rule.PlayersChooseFactions or Rule.EmperorSupportingNonAllyBids
                or Rule.AssistedNotekeeping or Rule.AssistedNotekeepingForAtreides or Rule.ResourceBonusForStrongholds
                or Rule.BattleWithoutLeader or Rule.CapturedLeadersAreTraitorsToOwnFaction
                or Rule.DisableEndOfGameReport or Rule.DisableGuildSpecialVictory or Rule.DisableResourceTransfers
                or Rule.FremenAllyGetsDialedResourcesRefunded or Rule.DisableNovaFlipping
                or Rule.BreakAlliancesOnAlliancePhase => 0,
            Rule.FillWithBots or Rule.BotsCannotAlly => 0,
            Rule.TechTokens or Rule.CheapHeroTraitor or Rule.ExpansionTreacheryCards or Rule.SandTrout => 1,
            Rule.IxianSwappingCardOnBid or Rule.TleilaxuGholas => 1,
            Rule.LeaderSkills or Rule.Expansion2TreacheryCards => 2,
            Rule.ChoamAuditor or Rule.RicheseBlackMarket or Rule.StrongholdBonus => 2,
            Rule.GreatMaker or Rule.DiscoveryTokens or Rule.Homeworlds or Rule.NexusCards
                or Rule.Expansion3TreacheryCards or Rule.MoritaniAssassinate or Rule.EcazLoyalty => 3,
            _ => int.MaxValue
        };

    public bool IsAutomated(AutomationRuleType rule, Player p)
        => AutomationRules.Any(x => x.Initiator == p.Faction && x.RuleType == rule);
    
    public bool IsAutoPassedBid(Faction f)
    {
        foreach (var rule in AutomationRules.Where(x => x.Initiator == f))
        {
            switch (rule.RuleType)
            {
                case AutomationRuleType.BiddingPassAboveAmount:
                    if (CurrentBid != null && CurrentBid.TotalAmount >= rule.BiddingAboveAmount) return true;
                    break;
                
                case AutomationRuleType.BiddingPassWhenAtreidesOrAtreidesAllyPassed:
                    if (LatestBidByAtreidesOrAtreidesAllyWasPassed && f is not Faction.Atreides) return true;
                    break;
                
                case AutomationRuleType.BiddingPassWhenHighestBidByFaction:
                    if (CurrentBid != null && CurrentBid.Initiator == rule.BiddingWinningFaction) return true;
                    break;    
            }
        } 
        
        return false;
    }
}