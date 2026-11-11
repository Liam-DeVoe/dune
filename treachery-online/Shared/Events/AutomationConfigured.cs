/*
 * Copyright (C) 2020-2025 Ronald Ossendrijver (admin@treachery.online)
 * This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License
 * as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version. This
 * program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details. You should have
 * received a copy of the GNU General Public License along with this program. If not, see <http://www.gnu.org/licenses/>.
 */

namespace Treachery.Shared;

public class AutomationConfigured : GameEvent
{
    #region Construction

    public AutomationConfigured(Game game, Faction initiator) : base(game, initiator)
    {
    }

    public AutomationConfigured()
    {
    }

    #endregion Construction

    #region Properties

    public int AutomationRuleId { get; set; } = -1;
    
    public ItemAction Action { get; set; }
    
    public AutomationRuleType RuleType { get; set; }

    // CharityAutoClaim = 201, *
    
    // BiddingPassWhenAtreidesOrAtreidesAllyPassed = 301,
    
    // BiddingPassAboveAmount = 302,
    public int BiddingAboveAmount { get; set; }
    
    // BiddingPassWhenHighestBidByFaction = 303,
    public Faction BiddingWinningFaction { get; set; }
    
    // RevivalAutoClaimFreeRevival = 401,
    
    // ShipmentGuildAutoDelay = 502,
    
    #endregion Properties

    #region Validation

    public override Message? Validate()
    {
        return null;
    }

    #endregion Validation

    #region Execution

    protected override void ExecuteConcreteEvent()
    {
        switch (Action)
        {
            case ItemAction.Create:
                AutomationRuleId = Game.AutomationRules.Count == 0 
                    ? 0 
                    : Game.AutomationRules.Max(x => x.AutomationRuleId) + 1;
            
                Game.AutomationRules.Add(this);
                return;

            case ItemAction.Delete:
            {
                // Delete rule
                var rule = Game.AutomationRules.First(x => x.AutomationRuleId == AutomationRuleId);
                Game.AutomationRules.Remove(rule);
                return;
            }
            
            case ItemAction.Update:
            {
                // var rule = Game.AutomationRules.First(x => x.AutomationRuleId == AutomationRuleId);
                // Update rule
                // existingRule
                break;
            }
        }
    }

    public override Message GetMessage()
    {
        return Message.Express(Initiator, " change ally permissions");
    }

    #endregion Execution

    public static List<AutomationRuleType> GetAvailableRuleTypes(Game g, Player p)
    {
        var atreidesIsPlaying = g.IsPlaying(Faction.Atreides);
        return Enumerations.GetValuesExceptDefault(AutomationRuleType.Unknown).Where(x =>
            (x != AutomationRuleType.BiddingPassWhenAtreidesOrAtreidesAllyPassed || atreidesIsPlaying)
            && (x != AutomationRuleType.CharityAutoClaim || !p.Is(Faction.BeneGesserit))
            && (x != AutomationRuleType.ShipmentGuildAutoDelay || p.Is(Faction.Guild) && g.Applicable(Rule.GuildDetermineShipment))
            && (x != AutomationRuleType.BiddingPassWhenHighestBidByFaction || GetValidBiddingFactions(g, p).Count > 0)).ToList();
    }
    
    public static List<Faction> GetValidBiddingFactions(Game g, Player p) 
        => g.Players.Where(x => x.Faction != p.Faction).Select(x => x.Faction).ToList();

    public Message? GetDescription()
    {
        switch (RuleType)
        {
            case AutomationRuleType.CharityAutoClaim:
                return Message.Express("Auto claim charity when possible");

            case AutomationRuleType.BiddingPassAboveAmount:
                return Message.Express("Auto pass if bid is equal to or higher than ",
                    Payment.Of(BiddingAboveAmount));

            case AutomationRuleType.BiddingPassWhenHighestBidByFaction:
                return Message.Express("Auto pass if highest bid by ", BiddingWinningFaction);

            case AutomationRuleType.BiddingPassWhenAtreidesOrAtreidesAllyPassed:
                var atreides = Game.GetPlayer(Faction.Atreides);
                return Message.Express("Auto pass if most recent bid by ", Faction.Atreides,
                    MessagePart.ExpressIf(atreides is { HasAlly: true }, atreides?.Ally),
                    " was passed");

            case AutomationRuleType.RevivalAutoClaimFreeRevival:
                if (Player.HasSpecialForces)
                    return Message.Express("Auto claim free revival, prioritizing ", Player.SpecialForce);

                return Message.Express("Auto claim free revival");

            case AutomationRuleType.ShipmentGuildAutoDelay:
                return Message.Express("Auto delay shipment until last");
        }

        return null;
    }
}