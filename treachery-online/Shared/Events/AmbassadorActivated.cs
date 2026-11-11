/*
 * Copyright (C) 2020-2025 Ronald Ossendrijver (admin@treachery.online)
 * This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License
 * as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version. This
 * program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details. You should have
 * received a copy of the GNU General Public License along with this program. If not, see <http://www.gnu.org/licenses/>.
 */

namespace Treachery.Shared;

public class AmbassadorActivated : PassableGameEvent, ILocationEvent, IPlacement
{
    #region Construction

    public AmbassadorActivated(Game game, Faction initiator) : base(game, initiator)
    {
    }

    public AmbassadorActivated()
    {
    }

    #endregion Construction

    #region Properties

    public Ambassador BeneGesseritSelectedAmbassador { get; set; }

    public string _choamCardIds = string.Empty;

    [JsonIgnore]
    public IEnumerable<TreacheryCard> ChoamCards
    {
        get => IdStringToObjects(_choamCardIds, TreacheryCardManager.Lookup);
        set => _choamCardIds = ObjectsToIdString(value, TreacheryCardManager.Lookup);
    }

    public bool EcazOfferAlliance { get; set; }

    public bool EcazGiveVidalToAlly { get; set; }

    public bool EcazTakeVidal { get; set; }

    public int _fremenOrGuildToId;

    [JsonIgnore]
    public Location? FremenOrGuildTo
    {
        get => Game.Map.LocationLookup.Find(_fremenOrGuildToId);
        set => _fremenOrGuildToId = Game.Map.LocationLookup.GetId(value);
    }

    [JsonIgnore]
    public Location? To => FremenOrGuildTo;

    public string _fremenForceLocations = "";

    [JsonIgnore]
    public Dictionary<Location, Battalion> FremenForceLocations
    {
        get => PlacementEvent.ParseForceLocations(Game, Player.Faction, _fremenForceLocations);
        set => _fremenForceLocations = PlacementEvent.ForceLocationsString(Game, value);
    }

    public int _ixianCardId;

    [JsonIgnore]
    public TreacheryCard? IxianCard
    {
        get => TreacheryCardManager.Get(_ixianCardId);
        set => _ixianCardId = TreacheryCardManager.GetId(value);
    }

    public int GuildForceAmount { get; set; }

    public int TleilaxuAmountOfForces { get; set; }

    public int _tleilaxuHeroId = -1;

    [JsonIgnore]
    public IHero? TleilaxuHero
    {
        get => LeaderManager.HeroLookup.Find(_tleilaxuHeroId);
        set => _tleilaxuHeroId = LeaderManager.HeroLookup.GetId(value);
    }

    public bool TleilaxuAssignSkill { get; set; }

    [JsonIgnore]
    public int TotalAmountOfForcesAddedToLocation => FremenForceLocations.Values.Sum(b => b.TotalAmountOfForces);

    [JsonIgnore]
    public int ForcesAddedToLocation => FremenForceLocations.Values.Sum(b => b.AmountOfForces);

    [JsonIgnore]
    public int SpecialForcesAddedToLocation => FremenForceLocations.Values.Sum(b => b.AmountOfSpecialForces);

    [JsonIgnore]
    public Dictionary<Location, Battalion> ForceLocations => FremenForceLocations;

    #endregion Properties

    #region Validation

    public override Message? Validate()
    {
        if (!Passed)
        {
            var player = Player;
            var ambassador = GetAmbassador(Game);

            if (ambassador == Ambassador.BeneGesserit)
            {
                if (!GetValidBeneGesseritAmbassadors(Game).Contains(BeneGesseritSelectedAmbassador)) return Message.Express("Invalid Ambassador selected");
                ambassador = BeneGesseritSelectedAmbassador;
            }

            switch (ambassador)
            {
                case Ambassador.Choam:
                    if (ChoamCards.Any(c => !GetValidChoamCards(player).Contains(c))) return Message.Express("Invalid card selected");
                    break;

                case Ambassador.Ecaz:
                    if (EcazOfferAlliance && !AllianceCanBeOffered(Game, player)) return Message.Express("You can't offer an alliance");
                    if (EcazTakeVidal && !VidalCanBeTaken(Game, Player)) return Message.Express("You can't take ", Game.Vidal);
                    if (EcazOfferAlliance && EcazGiveVidalToAlly && !VidalCanBeOfferedToNewAlly(Game, player)) return Message.Express("You can't offer ", Game.Vidal, " to your ally");
                    if (Game.Version >= 167 && EcazTakeVidal && EcazGiveVidalToAlly) return Message.Express("You don't have ", Game.Vidal, " yet");
                    if (Game.Version >= 176 && EcazTakeVidal && EcazOfferAlliance) return Message.Express("You can't both take ", Game.Vidal, " AND offer an alliance");
                    break;

                case Ambassador.Fremen:
                    if (FremenForceLocations.Any(kvp => Game.IsInStorm(kvp.Key))) return Message.Express("Can't move from storm");
                    if (FremenForceLocations.Any(kvp => player.ForcesIn(kvp.Key) < kvp.Value.AmountOfForces)) return Message.Express("Invalid amount of ", Player.Force);
                    if (FremenForceLocations.Any(kvp => player.SpecialForcesIn(kvp.Key) < kvp.Value.AmountOfSpecialForces)) return Message.Express("Invalid amount of ", Player.SpecialForce);
                    if (!ValidFremenTargets(Game, player).Contains(FremenOrGuildTo)) return Message.Express("Invalid target location");
                    break;

                case Ambassador.Ixian:
                    if (!GetValidIxianCards(player).Any()) return Message.Express("You don't have a card to discard");
                    if (IxianCard != null && !GetValidIxianCards(player).Contains(IxianCard)) return Message.Express("Invalid card selected");
                    break;

                case Ambassador.Richese:
                    if (player.Resources < 3) return Message.Express("You don't have enough ", Concept.Resource, " to buy a card");
                    if (!player.HasRoomForCards) return Message.Express("You don't have room for an additional card");
                    break;

                case Ambassador.Guild:
                    if (!ValidGuildTargets(Game, player).Contains(FremenOrGuildTo)) return Message.Express("Invalid target location");
                    if (GuildForceAmount > ValidGuildMaxForces(player)) return Message.Express("Invalid amount of forces");
                    break;

                case Ambassador.Tleilaxu:
                    if (TleilaxuAmountOfForces < 0) return Message.Express("You can't revive a negative amount of forces");
                    if (TleilaxuAmountOfForces > ValidTleilaxuMaxAmount(player)) return Message.Express("You can't revive that many");
                    if (TleilaxuAmountOfForces > 0 && TleilaxuHero != null) return Message.Express("You can't revive both forces and a leader");
                    if (TleilaxuHero != null && !ValidTleilaxuHeroes(Game, player).Contains(TleilaxuHero)) return Message.Express("Invalid leader");
                    if (TleilaxuAssignSkill && TleilaxuHero == null) return Message.Express("Select a leader to assign a skill to");
                    if (TleilaxuAssignSkill && !Revival.MayAssignSkill(Game, Player, TleilaxuHero)) return Message.Express("You can't assign a skill to this leader");
                    break;
            }
        }

        return null;
    }

    private static Territory? GetTerritory(Game g)
    {
        return g.LastAmbassadorTrigger?.Territory;
    }

    public static Faction GetVictim(Game g)
    {
        return g.LastAmbassadorTrigger?.Initiator ?? Faction.None;
    }

    public static Player? GetVictimPlayer(Game g)
    {
        var victim = GetVictim(g);
        return victim != Faction.None ? g.GetPlayer(victim) : null;
    }

    public static Ambassador GetAmbassador(Game g)
    {
        var territory = GetTerritory(g);
        
        return g.LastAmbassadorTrigger != null && territory != null
            ? g.AmbassadorIn(territory) 
            : Ambassador.None;
    }

    public static bool AllianceCanBeOffered(Game g, Player p)
    {
        var victimPlayer = GetVictimPlayer(g);
        return !p.HasAlly && victimPlayer is { HasAlly: false };
    }
    
    public static bool VidalCanBeOfferedToNewAlly(Game g, Player p)
    {
        return g.VidalIsAlive;
    }

    public static bool VidalCanBeTaken(Game g, Player p)
    {
        return (g.Version < 167 || 
                g.Vidal != null && !p.Has(g.Vidal)) && g is { VidalIsAlive: true, VidalIsCapturedOrGhola: false } && g.OccupierOf(World.Ecaz) == null;
    }

    public static IEnumerable<Ambassador> GetValidBeneGesseritAmbassadors(Game g)
    {
        return g.UnassignedAmbassadors!.Items;
    }

    public static IEnumerable<TreacheryCard> GetValidChoamCards(Player p)
    {
        return p.TreacheryCards;
    }

    public static IEnumerable<TreacheryCard> GetValidIxianCards(Player p)
    {
        return p.TreacheryCards;
    }

    public static IEnumerable<Territory> ValidFremenSources(Game g, Player p)
    {
        return PlacementEvent.ValidMovementSources(g, p);
    }

    public static IEnumerable<Location> ValidFremenTargets(Game g, Player p)
    {
        return g.Map.Locations(false).Where(l =>
            l.Visible &&
            l.Sector != g.SectorInStorm &&
            g.IsNotFull(p, l));
    }

    public static IEnumerable<Location> ValidGuildTargets(Game g, Player p)
    {
        return g.Map.Locations(g.Applicable(Rule.Homeworlds)).Where(l =>
            l.Sector != g.SectorInStorm &&
            (!ReferenceEquals(l, g.Map.HiddenMobileStronghold) || p.Is(Faction.Ixian)) &&
            !g.ContainsConflictingAlly(p, l) &&
            Shipment.IsEitherValidHomeworldOrNoHomeworld(g, p, l, false) &&
            Shipment.IsEitherValidDiscoveryOrNoDiscovery(l) &&
            g.IsNotFull(p, l));
    }

    public static int ValidGuildMaxForces(Player p)
    {
        return Math.Min(p.ForcesInReserve, 4);
    }

    public static int ValidTleilaxuMaxAmount(Player p)
    {
        return Math.Min(p.ForcesKilled, 4);
    }

    public static IEnumerable<IHero> ValidTleilaxuHeroes(Game game, Player player)
    {
        return game.KilledHeroes(player);
    }

    public static bool MayPass(Player p)
    {
        return p.Faction == Faction.Ecaz;
    }

    #endregion Validation

    #region Execution

    protected override void ExecuteConcreteEvent()
    {
        Game.CurrentAmbassadorActivated = this;
        var ambassadorFaction = GetAmbassador(Game);
        var territory = GetTerritory(Game);
        var victim = GetVictim(Game);
        Game.DequeueIntrusion(IntrusionType.Ambassador);

        if (!Passed && territory != null)
        {
            Game.AmbassadorsOnPlanet.Remove(territory);
            Game.Stone(Milestone.AmbassadorActivated);

            var ecaz = GetPlayer(Faction.Ecaz)!;

            if (ambassadorFaction == Ambassador.BeneGesserit)
            {
                ambassadorFaction = BeneGesseritSelectedAmbassador;
                Log("The ", Ambassador.BeneGesserit, " Ambassador is removed from the game");
            }
            else if (ambassadorFaction == Ambassador.Ecaz)
            {
                ecaz.Ambassadors.Add(ambassadorFaction);
                Log("The ", ambassadorFaction, " Ambassador returns to supply");
            }
            else
            {
                Game.AmbassadorsSetAside.Add(ambassadorFaction);
                Log("The ", ambassadorFaction, " Ambassador is set aside");
            }

            HandleAmbassador(ambassadorFaction, victim);

            if (ecaz.Ambassadors.Union(Game.AmbassadorsOnPlanet.Values).All(f => f == Ambassador.Ecaz))
            {
                Game.AssignRandomAmbassadors(ecaz);
                Log(Faction.Ecaz, " draw 5 random Ambassadors");
            }
        }
        else
        {
            Log(Initiator, " don't activate an Ambassador");
            Game.DetermineNextShipmentAndMoveSubPhase();
        }
    }

    private void HandleAmbassador(Ambassador ambassadorFaction, Faction victim)
    {
        var victimPlayer = Game.GetPlayer(victim);

        Log(Initiator, " activate the ", ambassadorFaction, " ambassador");

        switch (ambassadorFaction)
        {
            case Ambassador.Atreides:

                if (victimPlayer!.TreacheryCards.Any())
                {
                    Log(Initiator, " see all treachery cards owned by ", victim);
                    LogTo(Initiator, victim, " own: ", victimPlayer.TreacheryCards);
                }
                else
                {
                    Log(victim, " don't own any cards");
                }

                Game.DetermineNextShipmentAndMoveSubPhase();
                break;

            case Ambassador.Choam:

                var totalEarned = 0;
                foreach (var c in ChoamCards)
                {
                    totalEarned += 3;
                    Game.Discard(Player, c);
                }

                Log(Initiator, " get ", Payment.Of(totalEarned));
                Player.Resources += totalEarned;
                Game.DetermineNextShipmentAndMoveSubPhase();
                break;

            case Ambassador.Ecaz:

                if (EcazOfferAlliance)
                {
                    Game.AllianceByAmbassadorOfferedTo = victim;
                    Log(Initiator, " offer an alliance to ", victim, MessagePart.ExpressIf(EcazGiveVidalToAlly, " offering ", Game.Vidal, " if they accept"));
                    Game.PausedAmbassadorPhase = Game.CurrentPhase;
                    Game.Enter(Phase.AllianceByAmbassador);
                }
                else 
                {
                    if (Game.CurrentAmbassadorActivated!.EcazTakeVidal)
                    {
                        Game.TakeVidal(Player, Game.Version >= 167 ? VidalMoment.Never : VidalMoment.AfterUsedInBattle);
                    }

                    Game.DetermineNextShipmentAndMoveSubPhase();
                }
                break;

            case Ambassador.Emperor:

                Player.Resources += 5;
                Log(Initiator, " get ", Payment.Of(5));
                Game.DetermineNextShipmentAndMoveSubPhase();
                break;

            case Ambassador.Fremen:

                Game.PerformMoveFromLocations(Player, FremenForceLocations, this, false, false);
                Game.RecentMoves.Add(this);
                Game.CheckIntrusion(this);
                Game.DetermineNextShipmentAndMoveSubPhase();
                break;

            case Ambassador.Harkonnen:

                if (victim == Faction.Harkonnen)
                    Log(Initiator, " see one of the ", Faction.Harkonnen, " traitors");
                else if (victim == Faction.Tleilaxu)
                    Log(Initiator, " see one of the ", Faction.Tleilaxu, " unrevealed Face Dancers");
                else
                    Log(Initiator, " see the ", victim, " traitor");

                var toSelectFrom = victim == Faction.Tleilaxu 
                    ? victimPlayer!.FaceDancers.Where(t => !victimPlayer.RevealedFaceDancers.Contains(t)) 
                    : victimPlayer!.Traitors;
                
                var revealed = toSelectFrom.RandomOrDefault(Game.Random!);
                LogTo(Initiator, victim, " reveal ", revealed);
                LogTo(victim, Initiator, " get to see ", revealed);
                Game.DetermineNextShipmentAndMoveSubPhase();
                break;

            case Ambassador.Ixian:

                Game.Discard(Player, IxianCard);
                Log(Initiator, " draw a new card");
                var drawn = Game.DrawTreacheryCard();
                if (drawn != null)
                    Player.TreacheryCards.Add(drawn);
                Game.DetermineNextShipmentAndMoveSubPhase();
                break;

            case Ambassador.Richese:

                Log(Initiator, " buy a card for ", Payment.Of(3));
                Player.Resources -= 3;
                var drawnCard = Game.DrawTreacheryCard();
                if (drawnCard != null)
                    Player.TreacheryCards.Add(drawnCard);
                Game.DetermineNextShipmentAndMoveSubPhase();
                break;

            case Ambassador.Guild:

                Log(Initiator, " send ", GuildForceAmount, Player.Force, " to ", FremenOrGuildTo);
                if (FremenOrGuildTo != null)
                    Player.ShipForces(FremenOrGuildTo, GuildForceAmount);
                Game.LastShipmentOrMovement = this;
                Game.CheckIntrusion(this);
                Game.DetermineNextShipmentAndMoveSubPhase();
                break;

            case Ambassador.Tleilaxu:

                Game.DetermineNextShipmentAndMoveSubPhase();
                if (TleilaxuHero != null)
                {
                    if (!Game.IsFaceDownDead(TleilaxuHero))
                        Log(Initiator, " revive ", TleilaxuHero);
                    else
                        Log(Initiator, " revive a face down leader");

                    Game.Revive(Player, TleilaxuHero);

                    if (TleilaxuAssignSkill) Game.PrepareSkillAssignmentToRevivedLeader(Player, TleilaxuHero as Leader);
                }
                else
                {
                    Log(Initiator, " revive ", TleilaxuAmountOfForces, " ", Player.Force);
                    Player.ReviveForces(TleilaxuAmountOfForces);
                }

                break;
        }
    }

    public override Message GetMessage()
    {
        return Message.Express(Initiator, Passed ? " don't" : "", " activate an Ambassador");
    }

    #endregion Execution
}