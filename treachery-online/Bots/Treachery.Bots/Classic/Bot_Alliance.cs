/*
 * Copyright (C) 2020-2025 Ronald Ossendrijver (admin@treachery.online)
 * This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License
 * as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version. This
 * program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details. You should have
 * received a copy of the GNU General Public License along with this program. If not, see <http://www.gnu.org/licenses/>.
 */

namespace Treachery.Bots;

using Shared.Model;

public partial class ClassicBot
{
    protected virtual AllianceByTerror DetermineAllianceByTerror()
    {
        var moritani = Game.GetPlayer(Faction.Moritani);
        var accept = !Player.HasAlly || PlayerStanding(moritani) > 0.75f * PlayerStanding(AlliedPlayer!);
        return new AllianceByTerror(Game, Faction) { Passed = !accept };
    }

    protected virtual AllianceByAmbassador DetermineAllianceByAmbassador()
    {
        var ecaz = Game.GetPlayer(Faction.Ecaz);
        var accept = !Player.HasAlly || PlayerStanding(ecaz) > 0.75f * PlayerStanding(AlliedPlayer!);
        return new AllianceByAmbassador(Game, Faction) { Passed = !accept };
    }

    private NexusVoted DetermineNexusVoted()
    {
        return new NexusVoted(Game, Faction) { Passed = false };
    }

    protected virtual AllianceOffered? DetermineAllianceOffered()
    {
        var nrOfPlayers = Game.Players.Count;
        var nrOfUnAlliedBots = Game.Players.Count(p => Game.IsBot(p) && !p.HasAlly);
        var nrOfUnalliedHumans = Game.Players.Count(p => !Game.IsBot(p) && !p.HasAlly);
        var offer = Game.CurrentAllianceOffers
                        .Where(offer => offer.Target == Faction && !Game.IsBot(offer.Player))
                        .HighestOrDefault(_random, offer => PlayerStanding(offer.Player))
                    ?? Game.CurrentAllianceOffers
                        .Where(allianceOffered => allianceOffered.Target == Faction)
                        .HighestOrDefault(_random, allianceOffered => PlayerStanding(allianceOffered.Player));

        if (offer != null) 
            return new AllianceOffered(Game, Faction) { Target = offer.Initiator };

        if (nrOfPlayers <= 2 ||
            Game.Applicable(Rule.BotsCannotAlly) ||
            (nrOfUnalliedHumans != 0 && nrOfUnalliedHumans >= nrOfUnAlliedBots - 1) ||
            Game.CurrentAllianceOffers.Any(o =>
                o.Initiator == Faction && Game.GetPlayer(o.Target).Ally == Faction.None)) return null;
        {
            var mostInterestingOpponentBotWithoutAlly = Game.Players
                .Where(p => p != Player && Game.IsBot(p) && !p.HasAlly && AllianceOffered.ValidTargets(Game, Player).Contains(p.Faction))
                .HighestOrDefault(_random, p => PlayerStanding(p));

            if (mostInterestingOpponentBotWithoutAlly != null) 
                return new AllianceOffered(Game, Faction) { Target = mostInterestingOpponentBotWithoutAlly.Faction };
        }

        return null;
    }

    protected virtual AlliancePassed? DetermineAlliancePassed()
    {
        //Keep waiting while one of our offers is still open; it is withdrawn when the other faction passes
        if (Game.CurrentAllianceOffers.Any(o => o.Initiator == Faction)) return null;

        //Humans may still offer us an alliance until they are done
        if (Game.Players.Any(p => !Game.IsBot(p) && !Game.HasActedOrPassed.Contains(p.Faction))) return null;

        return new AlliancePassed(Game, Faction);
    }

    private int PlayerStanding(Player p)
    {
        return 2 * p.TreacheryCards.Count() +
               p.Resources +
               6 * p.LocationsWithAnyForces.Count(l => l.Territory.IsStronghold) +
               p.ForcesOnPlanet.Sum(b => b.Value.TotalAmountOfForces) +
               p.ForcesInReserve +
               2 * p.SpecialForcesInReserve +
               2 * p.Leaders.Count(l => Game.IsAlive(l));
    }

    protected virtual AllianceBroken? DetermineAllianceBroken()
    {
        var offer = Game.CurrentAllianceOffers
            .Where(offer => offer.Target == Faction && !Game.IsBot(offer.Player))
            .HighestOrDefault(_random, offer => PlayerStanding(offer.Player));

        if (offer != null && Player.HasAlly && PlayerStanding(offer.Player) > PlayerStanding(AlliedPlayer!)) 
            return new AllianceBroken(Game, Faction);

        return null;
    }

    protected virtual NexusCardDrawn DetermineNexusCardDrawn()
    {
        if (NexusCardDrawn.MayDraw(Game, Player))
            if (Player.Nexus == Faction.None ||
                (Faction == Faction.Emperor && Player.Nexus == Faction.Emperor && !Game.Applicable(Rule.EmperorSpecialForces)) ||
                (Faction == Faction.BeneGesserit && Player.Nexus == Faction.BeneGesserit && !Game.Applicable(Rule.BeneGesseritAdvisors)) ||
                (Faction == Faction.Ixian && Player.Nexus == Faction.Ixian && !Game.Applicable(Rule.AdvancedCombat)))
                return new NexusCardDrawn(Game, Faction) { Passed = false };

        return new NexusCardDrawn(Game, Faction) { Passed = true };
    }

    protected virtual AllyPermission? DetermineAlliancePermissions()
    {
        if (Ally == Faction.None) return null;

        if (Game.CurrentMainPhase is MainPhase.Blow or MainPhase.Bidding or MainPhase.ShipmentAndMove or MainPhase.Battle)
        {

            var allowedResources = DetermineAllowedResources();
            var allowedKarmaCard = Player.TreacheryCards.FirstOrDefault(c => c.Type == TreacheryCardType.Karma);
            var permission = new AllyPermission(Game, Faction) { PermittedResources = allowedResources, PermittedKarmaCard = allowedKarmaCard };
            var boolPermissionsNeedUpdate = BoolPermissionsNeedUpdate(permission);
            var specialPermissionsNeedUpdate = SpecialPermissionsNeedUpdate(permission);

            if (boolPermissionsNeedUpdate || specialPermissionsNeedUpdate || Game.GetPermittedUseOfAllyResources(Ally) != allowedResources || Game.GetPermittedUseOfAllyKarma(Ally) != allowedKarmaCard)
            {
                LogInfo("Updating permissions, allowing use of {0} spice and Karama: {1}", allowedResources, allowedKarmaCard);
                return permission;
            }
        }

        return null;
    }

    private bool SpecialPermissionsNeedUpdate(AllyPermission permission)
    {
        var result = false;

        switch (Faction)
        {
            case Faction.Emperor:
                permission.EmperorWillPayForExtraRevival = 3;
                if (Game.EmperorWillPayForExtraRevival != 3) result = true;
                break;
        }

        return result;
    }

    private int DetermineAllowedResources()
    {
        switch (Faction)
        {
            case Faction.Atreides:
            case Faction.BeneGesserit:
            case Faction.Richese:
            case Faction.Harkonnen:
            case Faction.Ecaz:
            case Faction.Moritani:
            case Faction.Ixian:

                if (Game.CurrentMainPhase == MainPhase.ShipmentAndMove && Game.HasActedOrPassed.Contains(Faction))
                    return Math.Max(Resources - 5, 0);
                return Math.Max(Resources - 10, 0);

            case Faction.Choam:
                if (Game.CurrentMainPhase == MainPhase.Battle)
                    return Resources;
                if (Game.CurrentMainPhase == MainPhase.ShipmentAndMove && Game.HasActedOrPassed.Contains(Faction))
                    return Math.Max(Resources - 5, 0);
                return Math.Max(Resources - 10, 0);

            case Faction.Guild:
                if (Game.CurrentMainPhase == MainPhase.ShipmentAndMove)
                    return Resources;
                return Math.Max(Resources - 10, 0);

            case Faction.Fremen:
                if (Game.CurrentMainPhase == MainPhase.ShipmentAndMove || Player.TreacheryCards.Count() == 4)
                    return Resources;
                return Math.Max(Resources - 10, 0);

            case Faction.Emperor:
                if (Game.CurrentMainPhase == MainPhase.Bidding)
                    return Resources;
                if (Game.CurrentMainPhase == MainPhase.ShipmentAndMove && Game.HasActedOrPassed.Contains(Faction))
                    return Math.Max(Resources - 5, 0);
                return Math.Max(Resources - 10, 0);

            case Faction.Tleilaxu:
                if (Game.CurrentMainPhase == MainPhase.Resurrection)
                    return Resources;
                if (Game.CurrentMainPhase == MainPhase.ShipmentAndMove && Game.HasActedOrPassed.Contains(Faction))
                    return Math.Max(Resources - 5, 0);
                return Math.Max(Resources - 10, 0);
        }

        return 0;
    }

    private bool BoolPermissionsNeedUpdate(AllyPermission permission)
    {
        var result = false;

        foreach (var p in GetPermissionProperties())
        {
            typeof(AllyPermission).GetProperty(p)?.SetValue(permission, true);

            if (!result)
            {
                var curValue = typeof(Game).GetProperty(p)?.GetValue(Game);
                if (!Equals(curValue, true)) 
                    result = true;
            }
        }

        return result;
    }

    private IEnumerable<string> GetPermissionProperties() => Faction switch
    {
        Faction.Atreides => ["AtreidesSharesPrescience"],
        Faction.Fremen => ["FremenSharesPrescience", "FremenWillProtectFromMonster", "FremenAllowsThreeFreeRevivals", "FremenRefundsBattleDial"],
        Faction.Guild => ["GuildAllowsShippingDiscount"],
        Faction.BeneGesserit => ["BeneGesseritAllowsUseOfVoice"],
        Faction.Ixian => ["IxianAllowsReplacingCards"],
        Faction.Tleilaxu => ["TleilaxuAllowsRevivalDiscount"],
        Faction.Richese => ["RicheseAllowsUseOfNoField"],
        Faction.Moritani => ["MoritaniAllowsKeepingCards"],
        Faction.Ecaz => ["EcazSharesAmbassadors"],
        _ => []
    };

    protected virtual CardTraded? DetermineCardTraded()
    {
        if (Game.CurrentCardTradeOffer == null) return null;

        return Game.CurrentCardTradeOffer.RequestedCard != null 
            ? new CardTraded(Game, Faction) { Target = Game.CurrentCardTradeOffer.Initiator, Card = Game.CurrentCardTradeOffer.RequestedCard, RequestedCard = null } 
            : new CardTraded(Game, Faction) { Target = Game.CurrentCardTradeOffer.Initiator, Card = Player.TreacheryCards.OrderBy(c => CardQuality(c, Player)).FirstOrDefault(), RequestedCard = null };
    }
}