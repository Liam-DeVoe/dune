/*
 * Copyright (C) 2020-2025 Ronald Ossendrijver (admin@treachery.online)
 * This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License
 * as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version. This
 * program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details. You should have
 * received a copy of the GNU General Public License along with this program. If not, see <http://www.gnu.org/licenses/>.
 */

namespace Treachery.Shared.Model;

public class Player(Game game, Faction faction = Faction.None) : ICloneable
{
    #region Properties
    
    private Game Game { get; } = game;

    public Faction Faction { get; set; } = faction;

    public string Name => Game.GetPlayerName(this);

    public int Seat { get; set; } = -1;

    public int Resources { get; set; }

    public int Extortion { get; set; }

    public int Bribes { get; set; }

    public int ResourcesAfterBidding { get; set; }

    public int BankedResources { get; set; }

    public int TransferableResources { get; set; }

    public List<TreacheryCard> TreacheryCards { get; set; } = [];

    public HashSet<TreacheryCard> KnownCards { get; } = [];

    public List<IHero> Traitors { get; set; } = [];

    public HashSet<IHero> RevealedTraitors { get; } = [];

    public HashSet<IHero> ToldTraitors { get; } = [];

    public HashSet<IHero> ToldNonTraitors { get; } = [];

    public HashSet<IHero> KnownNonTraitors { get; } = [];

    public HashSet<IHero> DiscardedTraitors { get; } = [];

    public List<IHero> FaceDancers { get; private set; } = [];

    public HashSet<IHero> RevealedFaceDancers { get; private set; } = [];

    public HashSet<IHero> ToldFaceDancers { get; } = [];

    public HashSet<IHero> ToldNonFaceDancers { get; } = [];

    public List<Leader> Leaders { get; private set; } = [];

    public int ForcesInReserve => HomeWorlds.Sum(ForcesIn);

    public int SpecialForcesInReserve => HomeWorlds.Sum(SpecialForcesIn);

    public int AnyForcesInReserves => ForcesInReserve + SpecialForcesInReserve;

    public Dictionary<Location, Battalion> ForcesInLocations { get; private set; } = [];

    public Dictionary<Location, Battalion> ForcesOnPlanet => ForcesInLocations.Where(kvp => kvp.Key is not Homeworld).ToDictionary(kvp => kvp.Key, kvp => kvp.Value);

    public List<Ambassador> Ambassadors { get; set; } = [];

    public Faction PredictedFaction { get; set; }

    public int PredictedTurn { get; set; }

    public int ForcesKilled { get; set; }

    public int SpecialForcesKilled { get; set; }

    public int TotalForcesKilledInBattle { get; private set; }
    
    public Faction Ally { get; set; }

    public bool SpecialKarmaPowerUsed { get; set; }

    public List<TechToken> TechTokens { get; private set; } = [];

    public bool NoFieldIsActive => Faction == Faction.Richese && ForcesInLocations.Any(locationWithForces => locationWithForces.Value.AmountOfSpecialForces > 0);

    public Leader? MostRecentlyRevivedLeader { get; set; }

    public List<LeaderSkill> SkillsToChooseFrom { get; } = [];

    public List<Homeworld> HomeWorlds { get; } = [];

    public Faction Nexus { get; set; } = Faction.None;

    

    #endregion Properties

    #region Forces
    
    public Homeworld GetHomeworld(bool specialForces) => 
        HomeWorlds.First(hw => specialForces ? hw.IsHomeOfSpecialForces : hw.IsHomeOfNormalForces);

    private Battalion GetAndCreateIfNeeded(Location location)
    {
        if (ForcesInLocations.TryGetValue(location, out var result)) 
            return result;
        
        result = new Battalion(Faction, 0, 0, location);
        ForcesInLocations.Add(location, result);

        return result;
    }

    private void ChangeForces(Location location, int nrOfForces)
    {
        var result = GetAndCreateIfNeeded(location);
        result.ChangeForces(nrOfForces);
        if (result.TotalAmountOfForces == 0) ForcesInLocations.Remove(location);
    }

    private void ChangeSpecialForces(Location location, int nrOfForces)
    {
        var result = GetAndCreateIfNeeded(location);
        result.ChangeSpecialForces(nrOfForces);
        if (result.TotalAmountOfForces == 0) ForcesInLocations.Remove(location);
        CheckIfSalusaSecundusThresholdWasPassed();
    }

    public void AddForces(Location location, int nrOfForces, bool fromReserves)
    {
        if (fromReserves)
        {
            var sourceWorld = HomeWorlds.FirstOrDefault(w => ForcesIn(w) >= nrOfForces);
            if (sourceWorld != null) MoveForces(sourceWorld, location, nrOfForces);
        }
        else
        {
            ChangeForces(location, nrOfForces);
        }
    }
    
    public void AddSpecialForces(Location location, int nrOfForces, bool fromReserves)
    {
        if (fromReserves)
        {
            var sourceWorld = HomeWorlds.FirstOrDefault(w => SpecialForcesIn(w) >= nrOfForces);
            if (sourceWorld != null) MoveSpecialForces(sourceWorld, location, nrOfForces);
        }
        else
        {
            ChangeSpecialForces(location, nrOfForces);
        }
    }

    public void AddForcesToReserves(int nrOfForces)
    {
        var sourceWorld = HomeWorlds.FirstOrDefault(w => w.IsHomeOfNormalForces);
        if (sourceWorld != null) ChangeForces(sourceWorld, nrOfForces);
    }

    private bool SalusaSecundusHomeworldIsOnLowThreshold { get; set; }
    private void CheckIfSalusaSecundusThresholdWasPassed()
    {
        if (Faction == Faction.Emperor && Game.Applicable(Rule.EmperorSpecialForces))
        {
            var homeWorld = HomeWorlds.FirstOrDefault(hw => hw.World == World.SalusaSecundus);

            if (homeWorld == null)
                return;
            
            var hasLessThanThreshold = Game.Version <= 163 ? AnyForcesIn(homeWorld) < homeWorld.Threshold : SpecialForcesIn(homeWorld) < homeWorld.Threshold;

            if (hasLessThanThreshold && !SalusaSecundusHomeworldIsOnLowThreshold)
            {
                SalusaSecundusHomeworldIsOnLowThreshold = true;
            }
            else if (!hasLessThanThreshold && SalusaSecundusHomeworldIsOnLowThreshold)
            {
                SalusaSecundusHomeworldIsOnLowThreshold = false;
            }
        }
    }

    public void AddSpecialForcesToReserves(int nrOfForces)
    {
        var sourceWorld = HomeWorlds.FirstOrDefault(w => w.IsHomeOfSpecialForces);
        if (sourceWorld != null) ChangeSpecialForces(sourceWorld, nrOfForces);
    }

    public void RemoveForces(Location location, int nrOfForces)
    {
        ChangeForces(location, -nrOfForces);
    }

    public void RemoveSpecialForces(Location location, int nrOfForces)
    {
        ChangeSpecialForces(location, -nrOfForces);
    }

    public int ForcesIn(Location? location)
    {
        if (location is not null && ForcesInLocations.TryGetValue(location, out var battalion))
            return battalion.AmountOfForces;
        
        return 0;
    }

    public int SpecialForcesIn(Location? location)
    {
        if (location is not null && ForcesInLocations.TryGetValue(location, out var battalion))
            return battalion.AmountOfSpecialForces;
        
        return 0;
    }

    public int AnyForcesIn(Location? location)
    {
        if (location is not null && ForcesInLocations.TryGetValue(location, out var battalion))
            return battalion.TotalAmountOfForces;
        
        return 0;
    }
    
    public int OccupyingForcesIn(Territory? t) => t is null ? 0 : t.Locations.Sum(OccupyingForcesIn);

    public int ForcesIn(Territory? t) => t is null ? 0 : t.Locations.Sum(ForcesIn);

    public int SpecialForcesIn(Territory? t) => t is null ? 0 : t.Locations.Sum(SpecialForcesIn);

    public int AnyForcesIn(Territory? t) => t is null ? 0 : t.Locations.Sum(AnyForcesIn);

    private void ForcesToReserves(Location location)
    {
        var battalion = ForcesInLocations[location];

        AddForcesToReserves(battalion.AmountOfForces);

        if (Faction == Faction.BeneGesserit)
            AddForcesToReserves(battalion.AmountOfSpecialForces);
        else if (Faction != Faction.Richese) AddSpecialForcesToReserves(battalion.AmountOfSpecialForces);

        ForcesInLocations.Remove(location);
        CheckIfSalusaSecundusThresholdWasPassed();
    }

    public void ForcesToReserves(Location location, int amount)
    {
        AddForcesToReserves(amount);
        ChangeForces(location, -amount);
    }

    public void SpecialForcesToReserves(Location location, int amount)
    {
        if (Faction == Faction.BeneGesserit)
            AddForcesToReserves(amount);
        else if (Faction != Faction.Richese) AddSpecialForcesToReserves(amount);

        ChangeSpecialForces(location, -amount);
    }

    public void ForcesToReserves(Territory t, int amount, bool special)
    {
        if (amount > 0)
        {
            var toRemoveInTotal = amount;
            foreach (var l in t.Locations.OrderBy(l => l.SpiceBlowAmount))
            {
                var forcesIn = special ? SpecialForcesIn(l) : ForcesIn(l);
                if (forcesIn > 0)
                {
                    var toRemoveInThisLocation = Math.Min(forcesIn, toRemoveInTotal);

                    if (special && Faction != Faction.BeneGesserit)
                        AddSpecialForcesToReserves(toRemoveInTotal);
                    else if (!special || Faction != Faction.Richese) AddForcesToReserves(toRemoveInTotal);

                    if (special)
                        ChangeSpecialForces(l, -toRemoveInTotal);
                    else
                        ChangeForces(l, -toRemoveInTotal);

                    toRemoveInTotal -= toRemoveInThisLocation;
                }

                if (toRemoveInTotal == 0) break;
            }
        }
    }

    public void ForcesToReserves(Territory t)
    {
        foreach (var l in t.Locations.Where(l => AnyForcesIn(l) > 0)) ForcesToReserves(l);
    }

    public int KillAllForces(Location location, bool inBattle)
    {
        if (ForcesInLocations.TryGetValue(location, out var battalion))
        {
            var killCount = battalion.AmountOfForces;
            ForcesKilled += killCount;

            var specialKillCount = battalion.AmountOfSpecialForces;
            if (Faction == Faction.BeneGesserit)
                ForcesKilled += specialKillCount;
            else
                SpecialForcesKilled += specialKillCount;

            ForcesInLocations.Remove(location);
            CheckIfSalusaSecundusThresholdWasPassed();

            if (inBattle) TotalForcesKilledInBattle += killCount + specialKillCount;

            return killCount + specialKillCount;
        }

        return 0;
    }

    public void KillAllForces(Territory t, bool inBattle)
    {
        foreach (var l in t.Locations.Where(l => AnyForcesIn(l) > 0)) KillAllForces(l, inBattle);
    }

    public int KillForces(Location location, int amountOfForces, int amountOfSpecialForces, bool inBattle)
    {
        ChangeForces(location, -amountOfForces);
        ChangeSpecialForces(location, -amountOfSpecialForces);
            
        ForcesKilled += amountOfForces;

        if (Faction == Faction.BeneGesserit)
            ForcesKilled += amountOfSpecialForces;
        else
            SpecialForcesKilled += amountOfSpecialForces;

        if (inBattle) TotalForcesKilledInBattle += amountOfForces + amountOfSpecialForces;

        return amountOfForces + amountOfSpecialForces;
    }

    public void KillForces(Territory t, int amount, bool special, bool inBattle)
    {
        var toKill = amount;
        foreach (var l in t.Locations.OrderBy(l => l.SpiceBlowAmount))
        {
            var forcesIn = special ? SpecialForcesIn(l) : ForcesIn(l);
            if (forcesIn > 0)
            {
                var toBeKilled = Math.Min(forcesIn, toKill);
                if (special)
                    KillForces(l, 0, toBeKilled, inBattle);
                else
                    KillForces(l, toBeKilled, 0, inBattle);
                toKill -= toBeKilled;
            }

            if (toKill == 0) break;
        }
    }

    public void ReviveForces(int amount)
    {
        ForcesKilled -= amount;
        AddForcesToReserves(amount);
    }

    public void ReviveSpecialForces(int amount)
    {
        SpecialForcesKilled -= amount;
        AddSpecialForcesToReserves(amount);
    }

    public void ShipForces(Location to, int amount)
    {
        AddForces(to, amount, true);
    }

    public void ShipForces(Location to, Homeworld from, int amount)
    {
        MoveForces(from, to, amount);
    }
    
    public void ShipSpecialForces(Location to, int amount)
    {
        if (Faction is Faction.Richese)
        {
            ChangeSpecialForces(to, amount);
        }
        else
        {
            AddSpecialForces(to, amount, true);
        }
    }
    
    public void ShipSpecialForces(Location to, Homeworld from, int amount)
    {
        if (Faction is Faction.Richese)
        {
            ChangeSpecialForces(to, amount);
        }
        else
        {
            MoveSpecialForces(from, to, amount);
        }
    }

    public void ShipAdvisors(Location l, int amount)
    {
        AddForcesToReserves(-amount);
        ChangeSpecialForces(l, amount);
    }

    public void MoveForces(Location from, Location to, int amount)
    {
        ChangeForces(from, -amount);
        ChangeForces(to, amount);
    }

    public void MoveSpecialForces(Location from, Location to, int amount)
    {
        ChangeSpecialForces(from, -amount);
        ChangeSpecialForces(to, amount);
    }

    public void FlipForces(Location l, bool asAdvisors)
    {
        if (Faction == Faction.BeneGesserit)
        {
            if (asAdvisors)
            {
                var numberOfForces = ForcesIn(l);
                ChangeForces(l, -numberOfForces);
                ChangeSpecialForces(l, numberOfForces);
            }
            else
            {
                var numberOfSpecialForces = SpecialForcesIn(l);
                ChangeForces(l, numberOfSpecialForces);
                ChangeSpecialForces(l, -numberOfSpecialForces);
            }
        }
    }

    public void FlipForces(Territory t, bool asAdvisors)
    {
        foreach (var l in t.Locations) FlipForces(l, asAdvisors);
    }

    public int OccupyingForcesIn(Location l)
    {
        return ForcesIn(l) + (Faction == Faction.BeneGesserit ? 0 : SpecialForcesIn(l));
    }

    public bool Occupies(Location? l)
    {
        return l is not null && (OccupyingForcesIn(l) > 0 
                                 || Game.Version >= 164 && Ally is Faction.Ecaz && OccupyingForcesIn(l) > 0 && AlliedPlayer?.OccupyingForcesIn(l) > 0);
    }

    public bool Occupies(Territory? t)
    {
        return t != null && t.Locations.Any(Occupies);
    }

    public IEnumerable<Location> OccupiedLocations => Game.Map.Locations(true).Where(Occupies);

    public IEnumerable<Territory> OccupiedTerritories => Game.Map.Territories(true).Where(Occupies);

    public bool Controls(Game g, Location l, bool contestedStrongholdsCountAsControlled)
    {
        if (contestedStrongholdsCountAsControlled)
            return Occupies(l);
        return Occupies(l) && g.NrOfOccupantsExcludingFaction(l, Faction) == 0;
    }

    public bool Controls(Game g, Territory t, bool contestedStrongholdsCountAsOccupied)
    {
        if (contestedStrongholdsCountAsOccupied)
            return Occupies(t);
        return Occupies(t) && g.NrOfOccupantsExcludingFaction(t, Faction) == 0;
    }

    public IEnumerable<Territory> TerritoriesWithForces => Game.Map.Territories(true).Where(t => AnyForcesIn(t) > 0);

    public IEnumerable<Location> LocationsWithAnyForces => ForcesInLocations.Keys;

    public IEnumerable<Location> LocationsWithAnyForcesInTerritory(Territory t) => t.Locations.Where(l => AnyForcesIn(l) > 0);

    public FactionForce Force
    {
        get
        {
            return Faction switch
            {
                Faction.Atreides => FactionForce.Atreides,
                Faction.Harkonnen => FactionForce.Harkonnen,
                Faction.Fremen => FactionForce.Fremen,
                Faction.Emperor => FactionForce.Emperor,
                Faction.Guild => FactionForce.Guild,
                Faction.BeneGesserit => FactionForce.BeneGesserit,
                Faction.Ixian => FactionForce.Ixian,
                Faction.Tleilaxu => FactionForce.Tleilaxu,
                Faction.Choam => FactionForce.Choam,
                Faction.Richese => FactionForce.Richese,
                Faction.Ecaz => FactionForce.Ecaz,
                Faction.Moritani => FactionForce.Moritani,
                _ => FactionForce.None
            };
        }
    }

    public FactionSpecialForce SpecialForce
    {
        get
        {
            return Faction switch
            {
                Faction.Emperor => FactionSpecialForce.Emperor,
                Faction.Fremen => FactionSpecialForce.Fremen,
                Faction.BeneGesserit => FactionSpecialForce.BeneGesserit,
                Faction.Ixian => FactionSpecialForce.Ixian,
                Faction.Richese => FactionSpecialForce.Richese,
                _ => FactionSpecialForce.None
            };
        }
    }
    public bool HasSpecialForces =>
        (Game.Applicable(Rule.FremenSpecialForces) && Is(Faction.Fremen)) ||
        (Game.Applicable(Rule.EmperorSpecialForces) && Is(Faction.Emperor)) ||
        Is(Faction.Ixian);

    #endregion Forces

    #region Information

    public Player? AlliedPlayer => Game.GetPlayer(Ally);
    public int AlliedResources => AlliedPlayer?.Resources ?? 0;
    
    [MemberNotNullWhen(true, nameof(AlliedPlayer))]
    public bool HasAlly => Ally != Faction.None;

    public bool Has(TreacheryCard? card)
    {
        return card != null && TreacheryCards.Contains(card);
    }

    public bool Has(TreacheryCardType cardType)
    {
        return TreacheryCards.Any(c => c.Type == cardType);
    }
    
    public bool Has(Leader leader)
    {
        return Leaders.Contains(leader);
    }

    public bool Is(Faction f)
    {
        return Faction == f;
    }

    public bool OrAllyIs(Faction f)
    {
        return Faction == f || Ally == f;
    }

    public int MaximumNumberOfCards
    {
        get
        {
            var occupierOfChoamHomeworld = Game.OccupierOf(World.Choam);
            var occupationBonus = occupierOfChoamHomeworld != null && (occupierOfChoamHomeworld == this || occupierOfChoamHomeworld.Faction == Ally) ? 1 : 0;
            var atomicsPenalty = Game.AtomicsAftermath != null && (Faction == Faction.Moritani || (Game.Version >= 158 && Ally == Faction.Moritani)) ? 1 : 0;

            var amount = Faction switch
            {
                Faction.Harkonnen => 8,
                Faction.Choam => 5,
                _ => 4
            };

            return amount + occupationBonus - atomicsPenalty;
        }
    }
    
    public bool HasNoFieldIn(Territory territory) 
        => Faction == Faction.Richese && SpecialForcesIn(territory) > 0;

    public bool HasRoomForCards => TreacheryCards.Count < MaximumNumberOfCards;

    public bool HandSizeExceeded => TreacheryCards.Count > MaximumNumberOfCards;

    public int NumberOfTraitors => Faction == Faction.Harkonnen ? 4 : 1;

    public int NumberOfFaceDancers => Faction == Faction.Tleilaxu ? 3 : 0;
    
    public Dictionary<Location, Battalion> BattalionsIn(Territory territory)
    {
        var result = new Dictionary<Location, Battalion>();
        foreach (var kvp in ForcesInLocations.Where(kvp => kvp.Key.Territory == territory)) result.Add(kvp.Key, kvp.Value);
        return result;
    }

    public bool BattalionIn(Location location, out Battalion? battalion)
    {
        return ForcesInLocations.TryGetValue(location, out battalion);
    }

    public Battalion? BattalionIn(Location location)
    {
        return BattalionIn(location, out var result) 
            ? result 
            : new Battalion(Faction, 0, 0, location);
    }

    public void AssignLeaders(Game g)
    {
        Leaders = Faction switch
        {
            Faction.Choam => LeaderManager.GetLeaders(Faction.Choam).Where(l => g.Applicable(Rule.ChoamAuditor) || l.HeroType != HeroType.Auditor).ToList(),
            Faction.Ecaz => LeaderManager.GetLeaders(Faction.Ecaz).Where(l => l.HeroType != HeroType.Vidal).ToList(),
            _ => LeaderManager.GetLeaders(Faction).ToList()
        };
    }

    public TreacheryCard? Card(TreacheryCardType type) => TreacheryCards.FirstOrDefault(c => c.Type == type);

    public IEnumerable<IHero> UnrevealedTraitors => Traitors.Where(f => !RevealedTraitors.Contains(f));

    public IEnumerable<IHero> UnrevealedFaceDancers => FaceDancers.Where(f => !RevealedFaceDancers.Contains(f));

    public bool MessiahAvailable => Game.Applicable(Rule.AtreidesMessiah) && Is(Faction.Atreides) && TotalForcesKilledInBattle >= 7 && Game.IsAlive(LeaderManager.Messiah);
    public bool IsBot => Game.IsBot(this);
    public bool AllyIsBot => HasAlly && Game.IsBot(AlliedPlayer);
    public Homeworld PrimaryHomeworld => HomeWorlds.First();

    public bool HasKarma(Game g) => Karma.ValidKarmaCards(g, this).Any();

    public void InitializeHomeworld(Homeworld world, int initialNormalForces, int initialSpecialForces)
    {
        if (initialNormalForces > 0) AddForces(world, initialNormalForces, false);
        if (initialSpecialForces > 0) AddSpecialForces(world, initialSpecialForces, false);

        SalusaSecundusHomeworldIsOnLowThreshold = world.World == World.SalusaSecundus && initialSpecialForces < world.Threshold;

        HomeWorlds.Add(world);
    }

    public bool HasHighThreshold(World w)
    {
        if (!Game.Applicable(Rule.Homeworlds)) return false;

        var homeworld = HomeWorlds.FirstOrDefault(hw => hw.World == w);

        if (homeworld == null)
            return false;

        if (homeworld.World == World.SalusaSecundus) return !SalusaSecundusHomeworldIsOnLowThreshold;

        return AnyForcesIn(homeworld) >= homeworld.Threshold;
    }

    public bool HasHighThreshold()
    {
        if (!Game.Applicable(Rule.Homeworlds)) return false;
        return HomeWorlds.Any(w => HasHighThreshold(w.World));
    }

    public bool HasLowThreshold(World w)
    {
        if (!Game.Applicable(Rule.Homeworlds)) return false;

        var homeworld = HomeWorlds.FirstOrDefault(hw => hw.World == w);

        if (homeworld == null)
            return false;

        if (homeworld.World == World.SalusaSecundus) return SalusaSecundusHomeworldIsOnLowThreshold;

        return AnyForcesIn(homeworld) < homeworld.Threshold;
    }

    public bool HasLowThreshold() => Game.Applicable(Rule.Homeworlds) && HomeWorlds.Any(w => HasLowThreshold(w.World));


    public bool Initiated(GameEvent? e) => e != null && e.Initiator == Faction;

    public bool IsNative(Territory territory) => territory.Locations.Any(l => l is Homeworld hw && IsNative(hw));

    public bool IsNative(Homeworld hw) => HomeWorlds.Contains(hw);

    public bool HaveForcesOnEachOthersHomeWorlds(Player other)
    {
        return ForcesInLocations.Keys.Any(l => other.HomeWorlds.Contains(l)) ||
               other.ForcesInLocations.Keys.Any(l => HomeWorlds.Contains(l));
    }

    public int GetHomeworldBattleContributionAndLasgunShieldLimit(Territory whereBattleHappens)
    {
        if (!whereBattleHappens.IsHomeworld || !IsNative(whereBattleHappens)) return 0;

        var homeworld = whereBattleHappens.Locations.First() as Homeworld;

        if (homeworld == null)
            return 0;

        return HasHighThreshold(homeworld.World) ? homeworld.BattleBonusAndLasgunShieldLimitAtHighThreshold : homeworld.BattleBonusAndLasgunShieldLimitAtLowThreshold;
    }

    #endregion Information

    #region Support

    public object Clone()
    {
        var result = (Player)MemberwiseClone();

        result.TreacheryCards = [..TreacheryCards];
        result.Traitors = [..Traitors];
        result.FaceDancers = [..FaceDancers];
        result.RevealedFaceDancers = [..RevealedFaceDancers];
        result.Leaders = [..Leaders];
        result.ForcesInLocations = Utilities.CloneObjectDictionary(ForcesInLocations);
        result.TechTokens = [..TechTokens];

        return result;
    }

    #endregion Support
}