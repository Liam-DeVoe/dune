/*
 * Copyright (C) 2020-2025 Ronald Ossendrijver (admin@treachery.online)
 * This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License
 * as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version. This
 * program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details. You should have
 * received a copy of the GNU General Public License along with this program. If not, see <http://www.gnu.org/licenses/>.
 */

namespace Treachery.Shared;

public class Revival : GameEvent, ILocationEvent
{
    #region Construction

    public Revival(Game game, Faction initiator) : base(game, initiator)
    {
    }

    public Revival()
    {
    }

    #endregion Construction

    #region Properties

    public int AmountOfForces { get; set; }

    public int AmountOfSpecialForces { get; set; }

    public int ExtraForcesPaidByEmperor { get; set; }

    public int ExtraSpecialForcesPaidByEmperor { get; set; }

    public int NumberOfForcesInLocation { get; set; }

    public int NumberOfSpecialForcesInLocation { get; set; }

    public int _locationId = -1;

    [JsonIgnore]
    public Location? Location
    {
        get => Game.Map.LocationLookup.Find(_locationId);
        set => _locationId = Game.Map.LocationLookup.GetId(value);
    }

    [JsonIgnore]
    public Location? To => Location;

    public int _heroId;

    [JsonIgnore]
    public IHero? Hero
    {
        get => LeaderManager.HeroLookup.Find(_heroId);
        set => _heroId = LeaderManager.HeroLookup.GetId(value);
    }

    public bool AssignSkill { get; set; }

    public bool UsesEmperorSecretAlly { get; set; }

    [JsonIgnore]
    public int TotalAmountOfForcesAddedToLocation => NumberOfForcesInLocation + NumberOfSpecialForcesInLocation;

    [JsonIgnore]
    public int ForcesAddedToLocation => NumberOfForcesInLocation;

    [JsonIgnore]
    public int SpecialForcesAddedToLocation => NumberOfSpecialForcesInLocation;

    #endregion Properties

    #region Validation

    public override Message? Validate()
    {
        var p = Player;

        if (AmountOfForces + ExtraForcesPaidByEmperor <= 0 && AmountOfSpecialForces + ExtraSpecialForcesPaidByEmperor <= 0 && Hero == null) return Message.Express("Select forces or a leader to revive");

        if (AmountOfForces + ExtraForcesPaidByEmperor > p.ForcesKilled) return Message.Express("You can't revive that many ", p.Force);
        if (AmountOfSpecialForces + ExtraSpecialForcesPaidByEmperor > p.SpecialForcesKilled) return Message.Express("You can't revive that many ", p.SpecialForce);

        if (Initiator != Faction.Ixian && AmountOfSpecialForces + ExtraSpecialForcesPaidByEmperor > 1) return Message.Express("You can't revive more than one ", p.SpecialForce, " per turn");
        if (AmountOfSpecialForces + ExtraSpecialForcesPaidByEmperor > 0 && Initiator != Faction.Ixian && Game.FactionsThatRevivedSpecialForcesThisTurn.Contains(Initiator)) return Message.Express("You already revived a ", p.SpecialForce, " this turn");

        if (Game.Version >= 124)
        {
            var limit = Game.GetMaxRevivals(Game, p, UsesEmperorSecretAlly);
            if (Game.Version < 180 && UsesEmperorSecretAlly) limit += 3;
            if (AmountOfForces + AmountOfSpecialForces > limit) return Message.Express("You can't revive more than ", limit);

            var allyLimit = ValidMaxRevivalsByEmperor(Game, p);
            if (ExtraForcesPaidByEmperor + ExtraSpecialForcesPaidByEmperor > ValidMaxRevivalsByEmperor(Game, p)) return Message.Express("Your ally won't revive more than ", allyLimit);
        }
        else
        {
            var emperorRevivals = ValidMaxRevivalsByEmperor(Game, p);
            var limit = Game.GetMaxRevivals(Game, p, UsesEmperorSecretAlly);
            if (AmountOfForces + AmountOfSpecialForces > limit + emperorRevivals) Message.Express("You can't revive that many");
        }
        
        if (Game.Version >= 187 && Initiator is not Faction.Tleilaxu && Game.LeaderRevivalsThisTurn.GetValueOrDefault(Initiator, 0) > 0) return Message.Express("You cannot revive more leaders this turn");

        var costOfRevival = DetermineCost(Game, p, Hero, AmountOfForces, AmountOfSpecialForces, ExtraForcesPaidByEmperor, ExtraSpecialForcesPaidByEmperor, UsesEmperorSecretAlly);
        if (costOfRevival.TotalCostForPlayer > p.Resources) return Message.Express("You can't pay that many");

        if (AssignSkill && Hero == null) return Message.Express("You must revive a leader to assign a skill to");
        if (AssignSkill && !MayAssignSkill(Game, p, Hero)) return Message.Express("You can't assign a skill to this leader");

        if (UsesEmperorSecretAlly && !MayUseEmperorSecretAlly(Game, Player)) return Message.Express("you can't use ", Faction.Emperor, " cunning");

        if (Location != null)
        {
            if (!ValidRevivedForceLocations(Game, Player).Contains(Location)) return Message.Express("You can't place revived forces there");

            if (NumberOfForcesInLocation > NumberOfForcesThatMayBePlacedOnPlanet(Game, Player, UsesEmperorSecretAlly, AmountOfForces + ExtraForcesPaidByEmperor) ||
                NumberOfSpecialForcesInLocation > NumberOfSpecialForcesThatMayBePlacedOnPlanet(Player, AmountOfSpecialForces + ExtraSpecialForcesPaidByEmperor))
                return Message.Express("You can't place that many forces directly on the planet");
        }

        return null;
    }

    public static int DetermineCostOfForcesForEmperor(Game g, Player emperor, Faction ally, int forces, int specialForces)
    {
        return (int)Math.Ceiling(forces * GetPricePerForce(g, emperor) + specialForces * GetPricePerSpecialForce(g, emperor, ally));
    }

    public static RevivalCost DetermineCost(Game g, Player initiator, IHero? hero, int amountOfForces, int amountOfSpecialForces, int extraForcesPaidByEmperor, int extraSpecialForcesPaidByEmperor, bool usesEmperorSecretAlly)
    {
        var result = new RevivalCost();

        if (g.Version >= 124)
        {
            result.CostForEmperor = initiator.Ally == Faction.Emperor ? DetermineCostOfForcesForEmperor(g, initiator.AlliedPlayer!, initiator.Faction, extraForcesPaidByEmperor, extraSpecialForcesPaidByEmperor) : 0;
            result.CostForForceRevivalForPlayer = GetPriceOfForceRevival(g, initiator, amountOfForces, amountOfSpecialForces, usesEmperorSecretAlly, out var nrOfPaidSpecialForces, out var numberOfForcesRevivedForFree);
            result.IncludesCostsForSpecialForces = nrOfPaidSpecialForces > 0;
            result.NumberOfForcesRevivedForFree = numberOfForcesRevivedForFree;
        }
        else
        {
            var costForForceRevival = GetPriceOfForceRevival(g, initiator, amountOfForces, amountOfSpecialForces, usesEmperorSecretAlly, out var nrOfPaidSpecialForces, out var numberOfForcesRevivedForFree);
            var amountPaidForByEmperor = ValidMaxRevivalsByEmperor(g, initiator);
            var emperor = g.GetPlayer(Faction.Emperor);
            var emperorsSpice = emperor?.Resources ?? 0;

            result.CostForEmperor = DetermineCostForEmperor(g, initiator.Faction, costForForceRevival, amountOfForces, amountOfSpecialForces, emperorsSpice, amountPaidForByEmperor);
            result.CostForForceRevivalForPlayer = costForForceRevival - result.CostForEmperor;
            result.IncludesCostsForSpecialForces = nrOfPaidSpecialForces > 0;
            result.NumberOfForcesRevivedForFree = numberOfForcesRevivedForFree;
        }

        result.CostToReviveHero = GetPriceOfHeroRevival(g, initiator, hero);
        result.CanBePaid = initiator.Resources >= result.TotalCostForPlayer;

        return result;
    }

    public static int GetPriceOfForceRevival(Game g, Player initiator, int amountOfForces, int amountOfSpecialForces, bool usesEmperorSecretAlly, out int nrOfPaidSpecialForces, out int numberOfForcesRevivedForFree)
    {
        var nrOfFreeRevivals = g.FreeRevivals(initiator, usesEmperorSecretAlly);
        nrOfPaidSpecialForces = initiator.Is(Faction.Emperor) && initiator.HasLowThreshold(World.SalusaSecundus) ? amountOfSpecialForces : Math.Max(0, amountOfSpecialForces - nrOfFreeRevivals);

        var nrOfFreeRevivalsLeft = nrOfFreeRevivals - (amountOfSpecialForces - nrOfPaidSpecialForces);
        var nrOfPaidNormalForces = Math.Max(0, amountOfForces - nrOfFreeRevivalsLeft);
        numberOfForcesRevivedForFree = amountOfForces + amountOfSpecialForces - nrOfPaidSpecialForces - nrOfPaidNormalForces;

        var priceOfSpecialForces = initiator.Is(Faction.Ixian) ? 3 : 2;
        var priceOfNormalForces = initiator.Is(Faction.Choam) && !g.Prevented(FactionAdvantage.ChoamRevival) ? 1 : 2;
        var cost = nrOfPaidSpecialForces * priceOfSpecialForces + nrOfPaidNormalForces * priceOfNormalForces;

        if (MayReviveWithDiscount(g, initiator)) cost = (int)Math.Ceiling(0.5 * cost);

        return cost;
    }

    private static int DetermineCostForEmperor(Game g, Faction initiator, int totalCostForForceRevival, int amountOfForces, int amountOfSpecialForces, int emperorsSpice, int amountPaidForByEmperor)
    {
        var priceOfSpecialForces = initiator == Faction.Ixian ? 3 : 2;
        var priceOfNormalForces = initiator == Faction.Choam && !g.Prevented(FactionAdvantage.ChoamRevival) && g.Version >= 122 ? 1 : 2;

        var specialForcesPaidByEmperor = 0;
        while (
            specialForcesPaidByEmperor + 1 <= amountOfSpecialForces &&
            (specialForcesPaidByEmperor + 1) * priceOfSpecialForces <= emperorsSpice &&
            specialForcesPaidByEmperor + 1 <= amountPaidForByEmperor)
            specialForcesPaidByEmperor++;

        var forcesPaidByEmperor = 0;
        while (
            forcesPaidByEmperor + 1 <= amountOfForces &&
            specialForcesPaidByEmperor * priceOfSpecialForces + (forcesPaidByEmperor + 1) * priceOfNormalForces <= emperorsSpice &&
            specialForcesPaidByEmperor + forcesPaidByEmperor + 1 <= amountPaidForByEmperor)
            forcesPaidByEmperor++;

        var costForEmperor = specialForcesPaidByEmperor * priceOfSpecialForces + forcesPaidByEmperor * priceOfNormalForces;
        return Math.Min(totalCostForForceRevival, Math.Min(costForEmperor, emperorsSpice));
    }

    public static IEnumerable<IHero> ValidRevivalHeroes(Game g, Player p)
    {
        var result = new List<IHero>();

        if (p.Faction != Faction.Tleilaxu)
        {
            if (g.Version >= 187 &&
                g.LeaderRevivalsThisTurn.ContainsKey(p.Faction))
                return [];

            result.AddRange(NormallyRevivableHeroes(g, p));
        }
        else if (p.Leaders.Count(g.IsAlive) < 5)
        {
            result.AddRange(UnrestrictedRevivableHeroes(g, p));
        } 
            
        if (result.Count == 0)
        {
            var tleilaxu = g.GetPlayer(Faction.Tleilaxu);
            var gholas = tleilaxu != null ? tleilaxu.Leaders.Where(l => l.Faction == p.Faction) : [];

            return g.KilledHeroes(p).Union(gholas).Where(h => IsAllowedEarlyRevival(g, h));
        }

        var livingLeaders = p.Leaders.Count(g.IsAlive);
        if (g.Version >= 139) livingLeaders += g.Players.Where(player => player != p).SelectMany(player => player.Leaders.Where(l => l.Faction == p.Faction)).Count();

        if (p.Is(Faction.Tleilaxu) && g.Applicable(Rule.TleilaxuGholas) && livingLeaders < 5 && (g.Version < 164 || !g.Prevented(FactionAdvantage.TleilaxuReviveGhola)))
            result.AddRange(
                g.LeaderState.Where(leaderAndState =>
                    leaderAndState.Key.Faction != Faction.Tleilaxu &&
                    leaderAndState.Key != LeaderManager.Messiah &&
                    leaderAndState.Key.HeroType != HeroType.Auditor &&
                    !leaderAndState.Value.Alive).Select(kvp => kvp.Key));

        return result;
    }

    private static bool IsAllowedEarlyRevival(Game g, IHero h)
    {
        return g.EarlyRevivalsOffers.TryGetValue(h, out var value) && value < int.MaxValue;
    }

    public static IEnumerable<IHero> NormallyRevivableHeroes(Game g, Player p)
    {
        var result = new List<IHero>();

        if (AllAvailableLeadersHaveDiedOnce(g, p) || AtLeastFiveLeadersHaveDiedOnce(g, p))
            if (p.Leaders.Any())
            {
                var lowestDeathCount = p.Leaders.Min(g.DeathCount);

                result.AddRange(p.Leaders.Where(l => g.DeathCount(l) == lowestDeathCount && !g.IsAlive(l)));

                if (p.Is(Faction.Atreides) && !g.IsAlive(LeaderManager.Messiah) && g.DeathCount(LeaderManager.Messiah) == lowestDeathCount) result.Add(LeaderManager.Messiah);
            }

        if (p.Faction == Faction.Choam)
        {
            var auditor = p.Leaders.FirstOrDefault(l => l.HeroType == HeroType.Auditor);
            if (auditor != null && !g.IsAlive(auditor) && !result.Contains(auditor)) result.Add(auditor);
        }

        if (p.Faction == Faction.Ecaz && g.Vidal != null)
            if (!g.IsAlive(g.Vidal) && !result.Contains(g.Vidal)) result.Add(g.Vidal);

        return result;

    }

    private static bool AtLeastFiveLeadersHaveDiedOnce(Game g, Player p)
    {
        return p.Leaders.Count(l => g.DeathCount(l) > 0) >= 5;
    }

    private static bool AllAvailableLeadersHaveDiedOnce(Game g, Player p)
    {
        return p.Leaders.All(l => g.DeathCount(l) > 0);
    }

    private static IEnumerable<IHero> UnrestrictedRevivableHeroes(Game g, Player p)
    {
        var result = new List<IHero>();

        result.AddRange(p.Leaders.Where(l => !g.IsAlive(l) && l.HeroType != HeroType.Auditor));

        if (p.Is(Faction.Atreides) && !g.IsAlive(LeaderManager.Messiah)) result.Add(LeaderManager.Messiah);

        return result;
    }

    public static int ValidMaxRevivals(Game g, Player p, bool specialForces, bool usingEmperorSecretAlly)
    {
        //var increasedRevivalDueToEmperorSecretAlly = usingEmperorSecretAlly ? 3 : 0;
        var normalForceRevivalLimit = g.GetMaxRevivals(g, p, usingEmperorSecretAlly);

        if (g.Version >= 124)
        {
            if (!specialForces)
                return Math.Min(normalForceRevivalLimit, p.ForcesKilled);
            
            return Math.Min(p.Is(Faction.Ixian) 
                ? normalForceRevivalLimit : 
                g.FactionsThatRevivedSpecialForcesThisTurn.Contains(p.Faction) 
                    ? 0 
                    : 1, p.SpecialForcesKilled);
        }
        // ReSharper disable once RedundantIfElseBlock
        else
        {
            var amountPaidByEmperor = EmperorExtraRevivalLimit(g, p);

            if (!specialForces)
                return Math.Min(normalForceRevivalLimit + amountPaidByEmperor, p.ForcesKilled);
        
            return Math.Min(p.Is(Faction.Ixian) 
                ? normalForceRevivalLimit + amountPaidByEmperor 
                : g.FactionsThatRevivedSpecialForcesThisTurn.Contains(p.Faction) 
                    ? 0 
                    : 1, p.SpecialForcesKilled);
        }
    }

    public static bool MayAssignSkill(Game g, Player p, IHero? h)
    {
        if (h is null) return false;
        
        var harkonnen = g.GetPlayer(Faction.Harkonnen);
        
        var capturedLeadersToConsider = harkonnen != null ? harkonnen.Leaders.Where(l => l.Faction == p.Faction) : [];

        return
            h is Leader &&
            (g.Version < 147 || h.Faction == p.Faction) &&
            h.HeroType != HeroType.Auditor &&
            g.Applicable(Rule.LeaderSkills) &&
            !g.CapturedLeaders.Any(cl => cl.Value == p.Faction && g.IsSkilled(cl.Key)) &&
            !(p.Is(Faction.Ecaz) && g.IsSkilled(g.Vidal)) &&
            !p.Leaders.Any(g.IsSkilled) &&
            !capturedLeadersToConsider.Any(g.IsSkilled);
    }

    private static bool MayReviveWithDiscount(Game g, Player p)
    {
        return (p.Is(Faction.Tleilaxu) && !g.Prevented(FactionAdvantage.TleilaxuRevivalDiscount)) ||
               (p.Ally == Faction.Tleilaxu && g.TleilaxuAllowsRevivalDiscount && !g.Prevented(FactionAdvantage.TleilaxuRevivalDiscountAlly));
    }


    public static int GetPriceOfHeroRevival(Game g, Player initiator, IHero? hero)
    {
        var tleilaxuDiscountPrevented = g.Prevented(FactionAdvantage.TleilaxuRevivalDiscount);

        if (hero == null) return 0;

        var price = hero.CostToRevive;

        if (g.Version < 102)
        {
            if (initiator.Is(Faction.Tleilaxu) && !tleilaxuDiscountPrevented) price = (int)Math.Ceiling(0.5 * price);

            if (!NormallyRevivableHeroes(g, initiator).Contains(hero) && g.EarlyRevivalsOffers.TryGetValue(hero, out var offer)) price = offer;
        }
        else
        {
            if (initiator.Is(Faction.Tleilaxu) && !tleilaxuDiscountPrevented)
            {
                price = (int)Math.Ceiling(0.5 * price);
            }
            else if (!NormallyRevivableHeroes(g, initiator).Contains(hero) && g.EarlyRevivalsOffers.TryGetValue(hero, out var offer))
            {
                price = offer;
            }
            else if (g.Version >= 176 && initiator.Ally is Faction.Tleilaxu && g.TleilaxuAllowsRevivalDiscount && !tleilaxuDiscountPrevented)
            {
                price = (int)Math.Ceiling(0.5 * price);
            }
        }

        return price;
    }

    private static float GetPricePerForce(Game g, Player revivingPlayer)
    {
        return (MayReviveWithDiscount(g, revivingPlayer) ? 0.5f : 1) *
               (revivingPlayer.Is(Faction.Choam) && !g.Prevented(FactionAdvantage.ChoamRevival) ? 1 : 2);
    }

    private static float GetPricePerSpecialForce(Game g, Player revivingPlayer, Faction ofRevivedForces)
    {
        return (MayReviveWithDiscount(g, revivingPlayer) ? 0.5f : 1) * (ofRevivedForces == Faction.Ixian ? 3 : 2);
    }

    private static int EmperorExtraRevivalLimit(Game g, Player p)
    {
        return p.Ally == Faction.Emperor && (g.Version < 113 || !g.Prevented(FactionAdvantage.EmperorLetAllyReviveExtraForces))
            ? g.EmperorWillPayForExtraRevival
            : 0;
    }

    public static int ValidMaxRevivalsByEmperor(Game g, Player p)
    {
        if (p.Ally != Faction.Emperor) return 0;

        var emperor = g.GetPlayer(Faction.Emperor)!;

        var potentialMaximum = p.Ally == Faction.Emperor && (g.Version < 113 || !g.Prevented(FactionAdvantage.EmperorLetAllyReviveExtraForces)) ? g.EmperorWillPayForExtraRevival : 0;

        if (g.Version < 115) return potentialMaximum;

        var priceOfSpecialForces = p.Faction == Faction.Ixian ? 3 : 2;

        var specialForcesPaidByEmperor = 0;
        while (
            (specialForcesPaidByEmperor + 1) * priceOfSpecialForces <= emperor.Resources &&
            specialForcesPaidByEmperor + 1 <= potentialMaximum)
            specialForcesPaidByEmperor++;

        var forcesPaidByEmperor = 0;
        while (
            specialForcesPaidByEmperor * priceOfSpecialForces + (forcesPaidByEmperor + 1) * 2 <= emperor.Resources &&
            specialForcesPaidByEmperor + forcesPaidByEmperor + 1 <= potentialMaximum)
            forcesPaidByEmperor++;

        return specialForcesPaidByEmperor + forcesPaidByEmperor;
    }

    public static bool MayUseEmperorSecretAlly(Game game, Player player)
    {
        return player.Nexus == Faction.Emperor && NexusPlayed.CanUseSecretAlly(game, player);
    }

    public static int NumberOfForcesThatMayBePlacedOnPlanet(Game game, Player player, bool usesEmperorSecretAlly, int nrOfForcesToRevive)
    {
        return player.Is(Faction.Tleilaxu) && player.HasHighThreshold()
            ? Math.Min(nrOfForcesToRevive, game.FreeRevivals(player, usesEmperorSecretAlly))
            : 0;
    }

    public static int NumberOfSpecialForcesThatMayBePlacedOnPlanet(Player player, int nrOfSpecialForcesToRevive)
    {
        return player.Is(Faction.Fremen) && player.HasHighThreshold() ? Math.Min(nrOfSpecialForcesToRevive, 1) : 0;
    }

    public static IEnumerable<Location> ValidRevivedForceLocations(Game g, Player p)
    {
        return g.Map.Locations(false).Where(l =>
            (p.Faction == Faction.Fremen || l.Sector != g.SectorInStorm) &&
            (!l.Territory.IsStronghold || g.NrOfOccupantsExcludingFaction(l, p.Faction) < 2) &&
            l.Visible &&
            (!p.HasAlly || Equals(l, g.Map.PolarSink) || !p.AlliedPlayer.Occupies(l)) &&
            (p.Faction == Faction.Tleilaxu || (p.Faction == Faction.Fremen && p.AnyForcesIn(l.Territory) >= 1)));
    }

    #endregion Validation

    #region Execution

    protected override void ExecuteConcreteEvent()
    {
        if (UsesEmperorSecretAlly) Game.PlayNexusCard(Player, " revive ", 3, " additional forces beyond revival limits for free");

        //Payment
        var cost = DetermineCost(Game, Player, Hero, AmountOfForces, AmountOfSpecialForces, ExtraForcesPaidByEmperor, ExtraSpecialForcesPaidByEmperor, UsesEmperorSecretAlly);
        if (cost.CostForEmperor > 0)
        {
            var emperor = GetPlayer(Faction.Emperor);
            emperor!.Resources -= cost.CostForEmperor;
        }
        Player.Resources -= cost.TotalCostForPlayer;

        var highThresholdExtraSuboids = Initiator == Faction.Ixian && Player.HasHighThreshold() && (Game.Version < 169 || cost.IncludesCostsForSpecialForces) ? 
            Math.Max(0, Math.Min(2, Player.ForcesKilled - AmountOfForces - ExtraForcesPaidByEmperor)) : 
            0;

        //Force revival
        Player.ReviveForces(AmountOfForces + ExtraForcesPaidByEmperor + highThresholdExtraSuboids);
        Player.ReviveSpecialForces(AmountOfSpecialForces + ExtraSpecialForcesPaidByEmperor);

        if (AmountOfSpecialForces > 0) Game.FactionsThatRevivedSpecialForcesThisTurn.Add(Initiator);

        //Register free revival
        var firstFreeRevivalThisTurn = false;
        var nrOfFreeRevivals = Math.Min(Game.FreeRevivals(Player, UsesEmperorSecretAlly),
            AmountOfForces + AmountOfSpecialForces);
        
        if (nrOfFreeRevivals > 0)
        {
            var freeRevivalsThisTurn = Game.FreeRevivalsThisTurn.GetValueOrDefault(Initiator, 0);
            firstFreeRevivalThisTurn = Game.Version < 186 || freeRevivalsThisTurn == 0;
            Game.FreeRevivalsThisTurn[Initiator] = freeRevivalsThisTurn + nrOfFreeRevivals;
        }

        //Tech token activated?
        if (firstFreeRevivalThisTurn && Initiator != Faction.Tleilaxu) Game.RevivalTechTokenIncome = true;

        //Tleilaxu income
        var tleilaxu = GetPlayer(Faction.Tleilaxu);
        var totalProfitsForTleilaxu = 0;
        if (tleilaxu != null)
        {
            if (firstFreeRevivalThisTurn && !Game.TleilaxuStartedRevivalWithLowThreshold) totalProfitsForTleilaxu += 1;

            if (Initiator != Faction.Tleilaxu)
            {
                totalProfitsForTleilaxu += cost.TotalCostForForceRevival;
                totalProfitsForTleilaxu += cost.CostToReviveHero;
            }

            if (totalProfitsForTleilaxu > 0 && Game.Prevented(FactionAdvantage.TleilaxuReceiveRevive))
            {
                totalProfitsForTleilaxu = 0;
                Game.LogPreventionByKarma(FactionAdvantage.TleilaxuReceiveRevive);
                if (!Game.Applicable(Rule.FullPhaseKarma)) Game.Allow(FactionAdvantage.TleilaxuReceiveRevive);
            }

            tleilaxu.Resources += totalProfitsForTleilaxu;

            if (totalProfitsForTleilaxu >= 5) Game.ApplyBureaucracy(Initiator, Faction.Tleilaxu);

            if (Game.Version < 176 && cost.TotalCost - totalProfitsForTleilaxu >= 4) Game.ActivateBanker(Player);
        }
        
        if (Game.Version >= 176 && cost.TotalCost - totalProfitsForTleilaxu >= 4) Game.ActivateBanker(Player);
        
        //Hero revival
        var asGhola = false;
        if (Hero != null)
        {
            Game.LeaderRevivalsThisTurn[Initiator] = Game.LeaderRevivalsThisTurn.GetValueOrDefault(Initiator, 0) + 1;
            var leader = Hero as Leader;
            
            if (Initiator != Hero.Faction && leader != null)
            {
                asGhola = true;
                Game.Revive(Player, leader);
            }
            else if (tleilaxu != null && tleilaxu.Leaders.Contains(Hero) && Game.IsAlive(Hero) && leader != null)
            {
                //Transfer of ghola
                tleilaxu.Leaders.Remove(leader);
                Player.Leaders.Add(leader);
            }
            else
            {
                Game.Revive(Player, Hero);
            }

            if (AssignSkill) Game.PrepareSkillAssignmentToRevivedLeader(Player, leader);

            Game.EarlyRevivalsOffers.Remove(Hero);
        }

        //Logging
        Game.Stone(Milestone.Revival);
        RegisterAndLogRevival(cost, totalProfitsForTleilaxu, asGhola, highThresholdExtraSuboids);

        if (Location != null)
        {
            if (NumberOfSpecialForcesInLocation > 0 || NumberOfForcesInLocation > 0)
            {
                var normalForceMessage = Message.Express();
                if (NumberOfForcesInLocation > 0)
                {
                    Player.ShipForces(Location, NumberOfForcesInLocation);
                    normalForceMessage = Message.Express(NumberOfForcesInLocation, Player.Force);
                }
                
                var specialForceMessage = Message.Express();
                if (NumberOfSpecialForcesInLocation > 0)
                {
                    Player.ShipSpecialForces(Location, NumberOfSpecialForcesInLocation);
                    specialForceMessage = Message.Express(NumberOfSpecialForcesInLocation, Player.SpecialForce);
                }
                
                Log(Initiator, " place ", normalForceMessage, specialForceMessage, " in ", Location);
                Game.LastShipmentOrMovement = this;
            }

            if (Game.Version >= 161 && Game.CheckIntrusion(this))
            {
                Game.PhaseBeforeRevivalCausedIntrusion = Game.CurrentPhase;
                Game.Enter(Game.LastBeneGesseritIntrusion != null, Phase.BeneGesseritIntrudedByRevival, Game.LastTerrorTrigger != null, Phase.TerrorTriggeredByRevival, Phase.AmbassadorTriggeredByRevival);
            }
        }

        if (Game.Version < 179 && Initiator != Faction.Tleilaxu) Game.HasActedOrPassed.Add(Initiator);
    }

    private void RegisterAndLogRevival(RevivalCost cost, int tleilaxuReceivedResources, bool asGhola, int highThresholdExtraSuboids)
    {
        var totalAmountOfForces = Game.Version < 182 
            ? AmountOfForces + ExtraForcesPaidByEmperor + highThresholdExtraSuboids
            : AmountOfForces + ExtraForcesPaidByEmperor + AmountOfSpecialForces + ExtraSpecialForcesPaidByEmperor + highThresholdExtraSuboids;
        
        Game.TotalRevivalsThisTurn[Initiator] = Game.TotalRevivalsThisTurn.GetValueOrDefault(Initiator, 0) + totalAmountOfForces; 

        // Each kind of force counted once; Ixian's free extra suboids have their own part.
        var forces = AmountOfForces + ExtraForcesPaidByEmperor;
        var specialForces = AmountOfSpecialForces + ExtraSpecialForcesPaidByEmperor;
        var totalCost = cost.TotalCostForPlayer + cost.CostForEmperor;
        var paidByEmperor = ExtraForcesPaidByEmperor > 0 || ExtraSpecialForcesPaidByEmperor > 0;

        Log(Message.Express(
            Initiator,
            " revive ",
            MessagePart.ExpressIf(Hero != null, Hero),
            MessagePart.ExpressIf(asGhola, " as Ghola"),
            MessagePart.ExpressIf(Hero != null && forces + specialForces > 0, " and "),
            MessagePart.ExpressIf(forces > 0, forces, Player.Force),
            MessagePart.ExpressIf(specialForces > 0, specialForces, Player.SpecialForce),
            " for ",
            Payment.Of(totalCost),
            MessagePart.ExpressIf(paidByEmperor, " (", Payment.Of(cost.CostForEmperor, Faction.Emperor), ")"),
            MessagePart.ExpressIf(tleilaxuReceivedResources > 0, " → ", Faction.Tleilaxu, " get ", Payment.Of(tleilaxuReceivedResources)),
            MessagePart.ExpressIf(highThresholdExtraSuboids > 0, " + ", highThresholdExtraSuboids, FactionForce.Ixian, " for free"))
            .WithLog(new RevivalRow(Initiator, forces + highThresholdExtraSuboids, specialForces, totalCost)
            {
                Hero = Hero,
                AsGhola = asGhola,
                PaidByOther = paidByEmperor ? cost.CostForEmperor : 0,
                Payer = paidByEmperor ? Faction.Emperor : Faction.None,
                TleilaxuIncome = tleilaxuReceivedResources
            }));
    }

    public override Message GetMessage()
    {
        return Message.Express(Initiator, " perform revival");
    }

    #endregion Execution
}