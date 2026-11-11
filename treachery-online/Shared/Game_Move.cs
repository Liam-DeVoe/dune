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
    #region State

    public PlayerSequence? ShipmentAndMoveSequence { get; internal set; }
    public ILocationEvent? LastShipmentOrMovement { get; internal set; }
    private Queue<Intrusion> Intrusions { get; } = new();
    public bool ShipsTechTokenIncome { get; internal set; }
    public List<IPlacement> RecentMoves { get; } = [];
    public int CurrentNoFieldValue { get; internal set; } = -1;
    public int LatestRevealedNoFieldValue { get; internal set; } = -1;
    public Dictionary<Faction, ShipmentPermission> ShipmentPermissions { get; } = new();
    internal List<Faction> FactionsWithOrnithoptersAtStartOfMovement { get; set; } = [];
    internal bool BeginningOfShipmentAndMovePhase { get; set; }
    internal KarmaShipmentPrevention? CurrentKarmaShipmentPrevention { get; set; }
    internal List<Territory> ChosenDestinationsWithAllies { get; } = [];
    internal bool CurrentPlayerMayPerformExtraMove { get; set; }
    internal bool BeneGesseritMayAccompany { get; set; }
    internal Phase PhaseBeforeCaravanCausedIntrusion { get; set; }
    internal Phase PhaseBeforeRevivalCausedIntrusion { get; set; }
    internal FlightUsed? CurrentFlightUsed { get; set; }
    internal FlightDiscoveryUsed? CurrentFlightDiscoveryUsed { get; set; }
    internal AmbassadorActivated? CurrentAmbassadorActivated { get; set; }
    public Faction AllianceByAmbassadorOfferedTo { get; internal set; }
    internal Phase PausedAmbassadorPhase { get; set; }
    private Player? PlayerToSetAsideVidal { get; set; }
    private VidalMoment WhenToSetAsideVidal { get; set; }
    internal Phase PausedTerrorPhase { get; set; }
    public bool AllianceByTerrorWasOffered { get; internal set; }
    public Faction AllianceByTerrorOfferedTo { get; internal set; }
    public bool InGuildCunningShipment { get; internal set; }
    public List<Territory> CurrentBlockedTerritories { get; } = [];
    internal bool ChoamHasExtraMove { get; set; }

    #endregion State

    internal void StartShipAndMoveSequence()
    {
        if (ShipmentAndMoveSequence?.CurrentFaction == Faction.Guild && GuildMayShipOutOfTurnOrder) 
            ShipmentAndMoveSequence.NextPlayer();
        
        Enter(JuiceForcesFirstPlayer && CurrentJuice.Initiator != Faction.Guild, 
            Phase.NonGuildShip, 
            CheckIfGuildActsOrAutoDelays(), Phase.GuildShip, Phase.NonGuildShip);
    }

    private bool CheckIfGuildActsOrAutoDelays()
    {
        var guild = GetPlayer(Faction.Guild);
        var guildMayDelay = GuildMayDelay;
        var guildHasToAct = IsPlaying(Faction.Guild) && HasActedOrPassed.All(p => p != Faction.Guild) && GuildMayShipOutOfTurnOrder;
        var guildIsOnAutoDelay = guild != null && IsAutomated(AutomationRuleType.ShipmentGuildAutoDelay, guild);
        if (guildHasToAct && guildIsOnAutoDelay && guildMayDelay)
        {
            Log(Message.Express(Faction.Guild, " delay their shipment").WithLog(new GuildDelayed()));
        }

        return guildHasToAct && !(guildIsOnAutoDelay && guildMayDelay);
    }

    internal void PerformMoveFromLocations(Player initiator, Dictionary<Location, Battalion> forceLocations, ILocationEvent evt, bool asAdvisors, bool byCaravan)
    {
        if (evt.To == null) return;
        
        LastShipmentOrMovement = evt;
        var wasOccupiedBeforeMove = IsOccupied(evt.To.Territory);
        var dist = DetermineMaximumMoveDistance(initiator, forceLocations.Values);
        int? moveNumber = evt is Move or Caravan ? ++MovesLogged : null;

        foreach (var fromTerritory in forceLocations.Keys.Select(l => l.Territory).Distinct())
        {
            var totalNumberOfForces = 0;
            var totalNumberOfSpecialForces = 0;

            foreach (var fl in forceLocations.Where(fl => fl.Key.Territory == fromTerritory))
                if (Version <= 150 || fl.Value.TotalAmountOfForces > 0)
                {
                    PerformMoveFromLocation(initiator, fl.Key, fl.Value, evt.To, ref totalNumberOfForces, ref totalNumberOfSpecialForces, false);
                    CheckSandmaster(initiator, evt.To, dist, fl);
                }

            if (initiator.Is(Faction.BeneGesserit)) initiator.FlipForces(evt.To, wasOccupiedBeforeMove && asAdvisors);

            if (totalNumberOfForces > 0 || totalNumberOfSpecialForces > 0)
            {
                var fromLocations = forceLocations.Where(fl => fl.Key.Territory == fromTerritory && fl.Value.TotalAmountOfForces > 0).Select(fl => fl.Key);
                LogMove(initiator, fromTerritory, fromLocations, evt.To, totalNumberOfForces, totalNumberOfSpecialForces, wasOccupiedBeforeMove && asAdvisors, byCaravan, moveNumber);
            }
        }

        Stone(Milestone.Move);
        FlipBeneGesseritAdvisorsWhenAlone();
    }

    private void CheckSandmaster(Player initiator, Location to, int dist, KeyValuePair<Location, Battalion> fl)
    {
        if (SkilledAs(initiator, LeaderSkill.Sandmaster))
        {
            var paths = Map.FindPaths(fl.Key, to, dist, initiator.Faction == Faction.Fremen && Applicable(Rule.FremenMayMoveIntoStorm), initiator.Faction, this);
            var mostSpice = 0;
            List<Location>? pathWithMostSpice = null;
            foreach (var p in paths)
            {
                var amountOfSpiceLocations = p.Where(l => ResourcesOnPlanet.ContainsKey(l)).Distinct().Count();
                if (amountOfSpiceLocations > mostSpice)
                {
                    mostSpice = amountOfSpiceLocations;
                    pathWithMostSpice = p;
                }
            }

            if (pathWithMostSpice != null && mostSpice > 0)
            {
                foreach (var loc in pathWithMostSpice)
                    if (ResourcesOnPlanet.ContainsKey(loc)) ChangeResourcesOnPlanet(loc, -1);

                initiator.Resources += mostSpice;
                Log(initiator.Faction, " ", LeaderSkill.Sandmaster, " collects ", Payment.Of(mostSpice), " along the way");
            }
        }
    }

    internal void PerformMoveFromLocation(Player initiator, Location from, Battalion battalion, 
        Location to, ref int totalNumberOfForces, ref int totalNumberOfSpecialForces, bool ignoreStorm)
    {
        var mustMoveThroughStorm = MustMoveThroughStorm(initiator, from, to, battalion);
        if (!ignoreStorm && (IsInStorm(to) || mustMoveThroughStorm))
        {
            int killCount;
            if (initiator.Is(Faction.Fremen) && !Prevented(FactionAdvantage.FremenProtectedFromStorm) && Applicable(Rule.FremenStormLosses))
            {
                killCount = 0;
                initiator.MoveForces(from, to, battalion.AmountOfForces);
                initiator.MoveSpecialForces(from, to, battalion.AmountOfSpecialForces);
                StormLossesToTake.Add(new LossToTake { Location = to, Amount = TakeLosses.HalfOf(battalion.AmountOfForces, battalion.AmountOfSpecialForces), Faction = battalion.Faction });
            }
            else
            {
                killCount = battalion.AmountOfForces + battalion.AmountOfSpecialForces;
                initiator.KillForces(from, battalion.AmountOfForces, battalion.AmountOfSpecialForces, false);
            }

            if (killCount > 0) Log(Message.Express(killCount, initiator.Faction, " forces are killed by the storm while travelling")
                .WithLog(new ShipMoveLoss(initiator.Faction, true, true, killCount, null)));
        }
        else
        {
            initiator.MoveForces(from, to, battalion.AmountOfForces);
            initiator.MoveSpecialForces(from, to, battalion.AmountOfSpecialForces);
        }

        totalNumberOfForces += battalion.AmountOfForces;
        totalNumberOfSpecialForces += battalion.AmountOfSpecialForces;
    }

    private bool MustMoveThroughStorm(Player initiator, Location from, Location to, Battalion moved)
    {
        var max = DetermineMaximumMoveDistance(initiator, [moved]);
        var targetsAvoidingStorm = Map.FindNeighbours(from, max, false, initiator.Faction, this);
        var targetsIgnoringStorm = Map.FindNeighbours(from, max, true, initiator.Faction, this);
        return !targetsAvoidingStorm.Contains(to) && targetsIgnoringStorm.Contains(to);
    }

    // Counts logged moves, so the phase log can join the lines of a move from
    // several territories.
    private int MovesLogged { get; set; }

    // moveNumber is set for a faction's own move (or Hajr move) in the
    // Ship & Move phase; those lines carry a record for the phase log.
    internal void LogMove(Player initiator, Territory from, IEnumerable<Location> fromLocations, Location to, int forceAmount, int specialForceAmount, bool asAdvisors, bool byCaravan, int? moveNumber = null)
    {
        var beneGesseritAs = initiator.Is(Faction.BeneGesserit) && Applicable(Rule.BeneGesseritAdvisors);
        var message = Message.Express(
            CaravanMessage(byCaravan),
            initiator.Faction,
            " move ",
            MessagePart.ExpressIf(forceAmount > 0, forceAmount, initiator.Force),
            MessagePart.ExpressIf(specialForceAmount > 0, specialForceAmount, initiator.SpecialForce),
            " from ",
            from,
            " to ",
            to,
            MessagePart.ExpressIf(beneGesseritAs, " as ", asAdvisors ? initiator.SpecialForce : initiator.Force))
            .WithRoute(initiator.Faction, fromLocations, to);

        if (moveNumber is { } nr)
            message.WithLog(new MoveLogged(initiator.Faction, nr, from, forceAmount, specialForceAmount, to, byCaravan, beneGesseritAs ? asAdvisors : null));

        Log(message);
    }

    private static MessagePart CaravanMessage(bool byCaravan)
    {
        return MessagePart.ExpressIf(byCaravan, "By ", TreacheryCardType.Caravan, ", ");
    }

    internal void TakeVidal(Player p, VidalMoment whenToSetAside)
    {
        var vidal = Vidal;

        if (vidal is null) return;

        var currentOwner = OwnerOf(vidal);
        if (currentOwner != null)
        {
            currentOwner.Leaders.Remove(vidal);

            if (IsAlive(vidal)) Log(currentOwner.Faction, " lose ", vidal);

            CapturedLeaders.Remove(vidal);
        }

        p.Leaders.Add(vidal);
        PlayerToSetAsideVidal = p;
        WhenToSetAsideVidal = whenToSetAside;

        Log(p.Faction, " take ", vidal);

        SetInFrontOfShield(vidal, false);
    }

    internal void LetPlayerDiscardTreacheryCardOfChoice(Faction f)
    {
        PhaseBeforeDiscarding = CurrentPhase;
        FactionsThatMustDiscard.Add(f);
        Enter(Phase.Discarding);
    }

    internal void KillAmbassadorIn(Territory territory)
    {
        var ambassador = AmbassadorIn(territory);
        if (ambassador != Ambassador.None)
        {
            var ecaz = GetPlayer(Faction.Ecaz);
            AmbassadorsOnPlanet.Remove(territory);
            ecaz!.Ambassadors.Add(ambassador);
            Log("The ambassador in ", territory, " returns to ", Faction.Ecaz);
        }
    }

    internal void DetermineNextShipmentAndMoveSubPhase()
    {
        CleanupObsoleteIntrusions();

        if (BeneGesseritMayAccompany)
        {
            if (Prevented(FactionAdvantage.BeneGesseritAccompanies))
            {
                LogPreventionByKarma(FactionAdvantage.BeneGesseritAccompanies);
                BeneGesseritMayAccompany = false;
                if (!Applicable(Rule.FullPhaseKarma)) Allow(FactionAdvantage.BeneGesseritAccompanies);
            }
            else if (HasLowThreshold(Faction.BeneGesserit))
            {
                BeneGesseritMayAccompany = false;
            }
        }

        if (StormLossesToTake.Count > 0)
        {
            PhaseBeforeStormLoss = CurrentPhase;
            Enter(Phase.StormLosses);
        }
        else if (Intrusions.Any())
        {
            switch (Intrusions.Peek().Type)
            {
                case IntrusionType.BeneGesseritIntrusion:

                    DetermineNextShipmentAndMoveSubPhaseOnBeneGesseritIntruded();
                    break;

                case IntrusionType.Terror:

                    DetermineNextShipmentAndMoveSubPhaseOnTerrorTriggered();
                    break;

                case IntrusionType.Ambassador:

                    DetermineNextShipmentAndMoveSubPhaseOnAmbassadorTriggered();
                    break;
            }
        }
        else
        {
            DetermineNextShipmentAndMoveSubPhaseOnNoIntrusion();
        }
        
        SkipMoveIfNothingToMove();
    }

    private void SkipMoveIfNothingToMove()
    {
        if (Version < 179 || CurrentPhase is not Phase.GuildMove and not Phase.NonGuildMove)
            return;
        
        var player = CurrentPhase is Phase.GuildMove 
            ? GetPlayer(Faction.Guild) 
            : ShipmentAndMoveSequence!.CurrentPlayer;

        if (player == null)
            return;

        if (PlacementEvent.ValidMovementSources(this, player).Any())
            return;

        StormLossesToTake.Clear();
        HasActedOrPassed.Add(player.Faction);

        if (CurrentPhase is Phase.NonGuildMove)
        {
            ShipmentAndMoveSequence!.NextPlayer();
            if (ShipmentAndMoveSequence.CurrentFaction == Faction.Guild && GuildMayShipOutOfTurnOrder)
                ShipmentAndMoveSequence.NextPlayer();
        }

        if (InGuildCunningShipment)
        {
            CurrentGuildCunning = null;
            InGuildCunningShipment = false;
        }

        Log(Message.Express(player.Faction, " pass movement").WithLog(new MovePassed(player.Faction)));

        DetermineNextShipmentAndMoveSubPhase();
        CheckIfForcesShouldBeDestroyedByAllyPresence(player);
        FlipBeneGesseritAdvisorsWhenAlone();

        if (!Applicable(Rule.FullPhaseKarma)) Allow(FactionAdvantage.FremenExtraMove);
        if (!Applicable(Rule.FullPhaseKarma)) Allow(FactionAdvantage.IxianCyborgExtraMove);

        CurrentFlightUsed = null;
        CurrentFlightDiscoveryUsed = null;
        CurrentPlanetology = null;
    }

    private void CleanupObsoleteIntrusions()
    {
        if (Intrusions.Any())
        {

            var i = 0;
            var search = true;
            while (search)
            {
                i++;
                if (i > 100) throw new Exception("Stuck");

                if (LastBeneGesseritIntrusion != null && GetPlayer(Faction.BeneGesserit)!.ForcesIn(LastBeneGesseritIntrusion.Territory) == 0)
                    DequeueIntrusion(IntrusionType.BeneGesseritIntrusion);
                else if (LastTerrorTrigger != null && !TerrorIn(LastTerrorTrigger.Territory).Any())
                    DequeueIntrusion(IntrusionType.Terror);
                else if (LastAmbassadorTrigger != null && AmbassadorIn(LastAmbassadorTrigger.Territory) == Ambassador.None)
                    DequeueIntrusion(IntrusionType.Ambassador);
                else
                    search = false;
            }
        }
    }

    private void DetermineNextShipmentAndMoveSubPhaseOnNoIntrusion()
    {
        switch (CurrentPhase)
        {
            case Phase.FremenRidingMonsterA:
            case Phase.BeneGesseritIntrudedByFremenRidingMonsterA:
            case Phase.TerrorTriggeredByFremenRidingMonsterA:
            case Phase.AmbassadorTriggeredByFremenRidingMonsterA:
                EndWormRideDuringPhase(Phase.FremenRidingMonsterA);
                break;

            case Phase.FremenRidingMonsterB:
            case Phase.BeneGesseritIntrudedByFremenRidingMonsterB:
            case Phase.TerrorTriggeredByFremenRidingMonsterB:
            case Phase.AmbassadorTriggeredByFremenRidingMonsterB:
                EndWormRideDuringPhase(Phase.FremenRidingMonsterB);
                break;

            case Phase.GuildShip:
            case Phase.BeneGesseritIntrudedByGuildShip:
            case Phase.TerrorTriggeredByGuildShip:
            case Phase.AmbassadorTriggeredByGuildShip:
                Enter(IsPlaying(Faction.BeneGesserit) && BeneGesseritMayAccompany, Phase.BeneGesseritAccompaniesGuildShip, !InGuildCunningShipment, Phase.GuildMove, Phase.NonGuildMove);
                break;

            case Phase.BeneGesseritAccompaniesGuildShip:
            case Phase.TerrorTriggeredByBeneGesseritAccompaniesGuildShip:
            case Phase.AmbassadorTriggeredByBeneGesseritAccompaniesGuildShip:
                Enter(!InGuildCunningShipment, Phase.GuildMove, Phase.NonGuildMove);
                break;

            case Phase.NonGuildShip:
            case Phase.BeneGesseritIntrudedByNonGuildShip:
            case Phase.TerrorTriggeredByNonGuildShip:
            case Phase.AmbassadorTriggeredByNonGuildShip:
                Enter(IsPlaying(Faction.BeneGesserit) && BeneGesseritMayAccompany, Phase.BeneGesseritAccompaniesNonGuildShip, Phase.NonGuildMove);
                break;

            case Phase.BeneGesseritAccompaniesNonGuildShip:
            case Phase.TerrorTriggeredByBeneGesseritAccompaniesNonGuildShip:
            case Phase.AmbassadorTriggeredByBeneGesseritAccompaniesNonGuildShip:
                Enter(Phase.NonGuildMove);
                break;

            case Phase.GuildMove:
            case Phase.BeneGesseritIntrudedByGuildMove:
            case Phase.TerrorTriggeredByGuildMove:
            case Phase.AmbassadorTriggeredByGuildMove:
                DetermineNextSubPhaseAfterGuildShipAndMove();
                break;

            case Phase.NonGuildMove:
            case Phase.BeneGesseritIntrudedByNonGuildMove:
            case Phase.TerrorTriggeredByNonGuildMove:
            case Phase.AmbassadorTriggeredByNonGuildMove:
                DetermineNextSubPhaseAfterNonGuildMove();
                break;

            case Phase.BeneGesseritIntrudedByCaravan:
            case Phase.TerrorTriggeredByCaravan:
            case Phase.AmbassadorTriggeredByCaravan:
                Enter(PhaseBeforeCaravanCausedIntrusion);
                break;

            case Phase.BeneGesseritIntrudedByRevival:
            case Phase.TerrorTriggeredByRevival:
            case Phase.AmbassadorTriggeredByRevival:
            {
                var nextPhase = PhaseBeforeRevivalCausedIntrusion;
                if (PhaseBeforeRevivalCausedIntrusion is Phase.AssigningSkill && !SkillAssigned.PlayersMustChooseLeaderSkills(this))
                {
                    nextPhase = PhaseBeforeSkillAssignment;
                    if (nextPhase == CurrentPhase)
                    {
                        nextPhase = Phase.Resurrection;
                    }
                }

                Enter(nextPhase);
                break;
            }
        }
    }

    private void DetermineNextShipmentAndMoveSubPhaseOnBeneGesseritIntruded()
    {
        switch (CurrentPhase)
        {
            case Phase.FremenRidingMonsterA:
            case Phase.BeneGesseritIntrudedByFremenRidingMonsterA:
            case Phase.TerrorTriggeredByFremenRidingMonsterA:
            case Phase.AmbassadorTriggeredByFremenRidingMonsterA:
                Enter(Phase.BeneGesseritIntrudedByFremenRidingMonsterA);
                break;

            case Phase.FremenRidingMonsterB:
            case Phase.BeneGesseritIntrudedByFremenRidingMonsterB:
            case Phase.TerrorTriggeredByFremenRidingMonsterB:
            case Phase.AmbassadorTriggeredByFremenRidingMonsterB:
                Enter(Phase.BeneGesseritIntrudedByFremenRidingMonsterB);
                break;

            case Phase.GuildShip:
            case Phase.BeneGesseritIntrudedByGuildShip:
            case Phase.TerrorTriggeredByGuildShip:
            case Phase.AmbassadorTriggeredByGuildShip:
            case Phase.TerrorTriggeredByBeneGesseritAccompaniesGuildShip:
            case Phase.AmbassadorTriggeredByBeneGesseritAccompaniesGuildShip:
                Enter(Phase.BeneGesseritIntrudedByGuildShip);
                break;

            case Phase.NonGuildShip:
            case Phase.BeneGesseritIntrudedByNonGuildShip:
            case Phase.TerrorTriggeredByNonGuildShip:
            case Phase.AmbassadorTriggeredByNonGuildShip:
            case Phase.TerrorTriggeredByBeneGesseritAccompaniesNonGuildShip:
            case Phase.AmbassadorTriggeredByBeneGesseritAccompaniesNonGuildShip:
                Enter(Phase.BeneGesseritIntrudedByNonGuildShip);
                break;

            case Phase.GuildMove:
            case Phase.BeneGesseritIntrudedByGuildMove:
            case Phase.TerrorTriggeredByGuildMove:
            case Phase.AmbassadorTriggeredByGuildMove:
                Enter(Phase.BeneGesseritIntrudedByGuildMove);
                break;

            case Phase.NonGuildMove:
            case Phase.BeneGesseritIntrudedByNonGuildMove:
            case Phase.TerrorTriggeredByNonGuildMove:
            case Phase.AmbassadorTriggeredByNonGuildMove:
                Enter(Phase.BeneGesseritIntrudedByNonGuildMove);
                break;

            case Phase.BeneGesseritIntrudedByCaravan:
            case Phase.TerrorTriggeredByCaravan:
            case Phase.AmbassadorTriggeredByCaravan:
                Enter(Phase.BeneGesseritIntrudedByCaravan);
                break;

            case Phase.BeneGesseritIntrudedByRevival:
            case Phase.TerrorTriggeredByRevival:
            case Phase.AmbassadorTriggeredByRevival:
                Enter(Phase.BeneGesseritIntrudedByRevival);
                break;

            default:
                throw new Exception($"BeneGesserit intrusion triggered during undefined phase: {CurrentPhase}");
        }
    }

    private void DetermineNextShipmentAndMoveSubPhaseOnTerrorTriggered()
    {
        switch (CurrentPhase)
        {
            case Phase.FremenRidingMonsterA:
            case Phase.BeneGesseritIntrudedByFremenRidingMonsterA:
            case Phase.TerrorTriggeredByFremenRidingMonsterA:
            case Phase.AmbassadorTriggeredByFremenRidingMonsterA:
                Enter(Phase.TerrorTriggeredByFremenRidingMonsterA);
                break;

            case Phase.FremenRidingMonsterB:
            case Phase.BeneGesseritIntrudedByFremenRidingMonsterB:
            case Phase.TerrorTriggeredByFremenRidingMonsterB:
            case Phase.AmbassadorTriggeredByFremenRidingMonsterB:
                Enter(Phase.TerrorTriggeredByFremenRidingMonsterB);
                break;

            case Phase.GuildShip:
            case Phase.BeneGesseritIntrudedByGuildShip:
            case Phase.TerrorTriggeredByGuildShip:
            case Phase.AmbassadorTriggeredByGuildShip:
                Enter(Phase.TerrorTriggeredByGuildShip);
                break;

            case Phase.BeneGesseritAccompaniesGuildShip:
            case Phase.TerrorTriggeredByBeneGesseritAccompaniesGuildShip:
            case Phase.AmbassadorTriggeredByBeneGesseritAccompaniesGuildShip:
                Enter(Phase.TerrorTriggeredByGuildShip);
                break;

            case Phase.NonGuildShip:
            case Phase.BeneGesseritIntrudedByNonGuildShip:
            case Phase.TerrorTriggeredByNonGuildShip:
            case Phase.AmbassadorTriggeredByNonGuildShip:
                Enter(Phase.TerrorTriggeredByNonGuildShip);
                break;

            case Phase.BeneGesseritAccompaniesNonGuildShip:
            case Phase.TerrorTriggeredByBeneGesseritAccompaniesNonGuildShip:
            case Phase.AmbassadorTriggeredByBeneGesseritAccompaniesNonGuildShip:
                Enter(Phase.TerrorTriggeredByNonGuildShip);
                break;

            case Phase.GuildMove:
            case Phase.BeneGesseritIntrudedByGuildMove:
            case Phase.TerrorTriggeredByGuildMove:
            case Phase.AmbassadorTriggeredByGuildMove:
                Enter(Phase.TerrorTriggeredByGuildMove);
                break;

            case Phase.NonGuildMove:
            case Phase.BeneGesseritIntrudedByNonGuildMove:
            case Phase.TerrorTriggeredByNonGuildMove:
            case Phase.AmbassadorTriggeredByNonGuildMove:
                Enter(Phase.TerrorTriggeredByNonGuildMove);
                break;

            case Phase.BeneGesseritIntrudedByCaravan:
            case Phase.TerrorTriggeredByCaravan:
            case Phase.AmbassadorTriggeredByCaravan:
                Enter(Phase.TerrorTriggeredByCaravan);
                break;

            case Phase.BeneGesseritIntrudedByRevival:
            case Phase.TerrorTriggeredByRevival:
            case Phase.AmbassadorTriggeredByRevival:
                Enter(Phase.TerrorTriggeredByRevival);
                break;

            default:
                throw new Exception($"Terror triggered during undefined phase: {CurrentPhase}");
        }
    }

    private void DetermineNextShipmentAndMoveSubPhaseOnAmbassadorTriggered()
    {
        switch (CurrentPhase)
        {
            case Phase.FremenRidingMonsterA:
            case Phase.BeneGesseritIntrudedByFremenRidingMonsterA:
            case Phase.TerrorTriggeredByFremenRidingMonsterA:
            case Phase.AmbassadorTriggeredByFremenRidingMonsterA:
                Enter(Phase.AmbassadorTriggeredByFremenRidingMonsterA);
                break;

            case Phase.FremenRidingMonsterB:
            case Phase.BeneGesseritIntrudedByFremenRidingMonsterB:
            case Phase.TerrorTriggeredByFremenRidingMonsterB:
            case Phase.AmbassadorTriggeredByFremenRidingMonsterB:
                Enter(Phase.AmbassadorTriggeredByFremenRidingMonsterB);
                break;

            case Phase.GuildShip:
            case Phase.BeneGesseritIntrudedByGuildShip:
            case Phase.TerrorTriggeredByGuildShip:
            case Phase.AmbassadorTriggeredByGuildShip:
                Enter(Phase.AmbassadorTriggeredByGuildShip);
                break;

            case Phase.BeneGesseritAccompaniesGuildShip:
            case Phase.TerrorTriggeredByBeneGesseritAccompaniesGuildShip:
            case Phase.AmbassadorTriggeredByBeneGesseritAccompaniesGuildShip:
                Enter(Phase.AmbassadorTriggeredByBeneGesseritAccompaniesGuildShip);
                break;

            case Phase.NonGuildShip:
            case Phase.BeneGesseritIntrudedByNonGuildShip:
            case Phase.TerrorTriggeredByNonGuildShip:
            case Phase.AmbassadorTriggeredByNonGuildShip:
                Enter(Phase.AmbassadorTriggeredByNonGuildShip);
                break;

            case Phase.BeneGesseritAccompaniesNonGuildShip:
            case Phase.TerrorTriggeredByBeneGesseritAccompaniesNonGuildShip:
            case Phase.AmbassadorTriggeredByBeneGesseritAccompaniesNonGuildShip:
                Enter(Phase.AmbassadorTriggeredByBeneGesseritAccompaniesNonGuildShip);
                break;

            case Phase.GuildMove:
            case Phase.BeneGesseritIntrudedByGuildMove:
            case Phase.TerrorTriggeredByGuildMove:
            case Phase.AmbassadorTriggeredByGuildMove:
                Enter(Phase.AmbassadorTriggeredByGuildMove);
                break;

            case Phase.NonGuildMove:
            case Phase.BeneGesseritIntrudedByNonGuildMove:
            case Phase.TerrorTriggeredByNonGuildMove:
            case Phase.AmbassadorTriggeredByNonGuildMove:
                Enter(Phase.AmbassadorTriggeredByNonGuildMove);
                break;

            case Phase.BeneGesseritIntrudedByCaravan:
            case Phase.TerrorTriggeredByCaravan:
            case Phase.AmbassadorTriggeredByCaravan:
                Enter(Phase.AmbassadorTriggeredByCaravan);
                break;

            case Phase.BeneGesseritIntrudedByRevival:
            case Phase.TerrorTriggeredByRevival:
            case Phase.AmbassadorTriggeredByRevival:
                Enter(Phase.AmbassadorTriggeredByRevival);
                break;

            default:
                throw new Exception($"Ambassador triggered during undefined phase: {CurrentPhase}");
        }
    }

    //private bool IsFirst(Faction a, Faction b) => !PlayerSequence.IsAfter(this, GetPlayer(a), GetPlayer(b));

    internal void CheckIfForcesShouldBeDestroyedByAllyPresence(Player p)
    {
        if (p.Ally != Faction.None && p.Faction != Faction.Ecaz && p.Ally != Faction.Ecaz)
        {
            var specialForcesAreAllowed = Version >= 177 && p.Is(Faction.BeneGesserit) && Applicable(Rule.AdvisorsDontConflictWithAlly);
            
            //Forces that must be destroyed because moves ended where allies are
            foreach (var t in ChosenDestinationsWithAllies)
                if (!(specialForcesAreAllowed && p.SpecialForcesIn(t) > 0) && p.AnyForcesIn(t) > 0)
                {
                    Log(Message.Express("All ", p.Faction, " forces in ", t, " were killed due to ally presence")
                        .WithLog(new ShipMoveLoss(p.Faction, false, false, 0, t)));
                    RevealCurrentNoField(p, t);
                    p.KillAllForces(t, false);
                }

            if (HasActedOrPassed.Contains(p.Ally))
            {
                //Forces that must be destroyed if both the player and his ally have moved

                var playerTerritories = Applicable(Rule.AdvisorsDontConflictWithAlly) 
                    ? p.OccupiedTerritories 
                    : p.TerritoriesWithForces;
                var allyTerritories = (Applicable(Rule.AdvisorsDontConflictWithAlly) 
                    ? GetPlayer(p.Ally)?.OccupiedTerritories 
                    : GetPlayer(p.Ally)?.TerritoriesWithForces) ?? [];

                foreach (var t in playerTerritories.Intersect(allyTerritories).Where(x => x != Map.PolarSink.Territory).ToList())
                {
                    Log(Message.Express("All ", p.Faction, " forces in ", t, " were killed due to ally presence")
                        .WithLog(new ShipMoveLoss(p.Faction, false, false, 0, t)));
                    RevealCurrentNoField(p, t);
                    p.KillAllForces(t, false);
                }
            }
        }
    }

    public bool ThreatenedByAllyPresence(Player p, Territory t)
    {
        if (t == Map.PolarSink.Territory) return false;

        var ally = GetPlayer(p.Ally);
        if (ally == null) return false;

        if (p.Faction == Faction.Ecaz || p.Ally == Faction.Ecaz) return false;

        if (Applicable(Rule.AdvisorsDontConflictWithAlly))
        {
            if (p.Is(Faction.BeneGesserit) && !p.Occupies(t))
                return false;
            
            if (p.Ally == Faction.BeneGesserit && !ally.Occupies(t))
                return false;
        }

        return ally.AnyForcesIn(t) > 0;
    }

    private void DetermineNextSubPhaseAfterNonGuildMove()
    {
        if (CurrentPlayerMayPerformExtraMove)
        {
            CurrentPlayerMayPerformExtraMove = false;
            Enter(Phase.NonGuildMove);
        }
        else
        {
            Enter(
                EveryoneActedOrPassed, ConcludeShipmentAndMove,
                CheckIfGuildActsOrAutoDelays(), Phase.GuildShip,
                Phase.NonGuildShip);
        }
    }

    private void DetermineNextSubPhaseAfterGuildShipAndMove()
    {
        if (CurrentPlayerMayPerformExtraMove)
        {
            CurrentPlayerMayPerformExtraMove = false;
            Enter(Phase.GuildMove);
        }
        else if (!InGuildCunningShipment && CurrentGuildCunning != null && CurrentGuildCunning.By(Faction.Guild))
        {
            InGuildCunningShipment = true;
            Enter(Phase.GuildShip);
        }
        else
        {
            Enter(!EveryoneActedOrPassed, Phase.NonGuildShip, ConcludeShipmentAndMove);
        }
    }

    private void ConcludeShipmentAndMove()
    {
        CheckIfMoritaniGainsVidal();
        CurrentKarmaShipmentPrevention = null;
        MainPhaseEnd();
        Enter(Phase.ShipmentAndMoveConcluded);
        ReceiveShipsTechIncome();
        ChoamHasExtraMove = false;
        CurrentBeneGesseritCunning = null;
    }

    private void CheckIfMoritaniGainsVidal()
    {
        var moritani = GetPlayer(Faction.Moritani);
        if (moritani == null || Prevented(FactionAdvantage.MoritaniGainingVidal)) 
            return;

        if (!VidalIsAlive) 
            return;
        
        var ecaz = GetPlayer(Faction.Ecaz);
        var nrOfBattlesInStrongholds = Battle.BattlesToBeFought(this, moritani).Select(b => b.Territory)
            .Where(t => (ecaz == null || ecaz.AnyForcesIn(t) == 0) && (t.IsStronghold || IsSpecialStronghold(t))).Distinct().Count();

        if (nrOfBattlesInStrongholds >= 2 && !VidalIsCapturedOrGhola && OccupierOf(World.Ecaz) == null) TakeVidal(moritani, VidalMoment.EndOfTurn);
    }

    private void ReceiveShipsTechIncome()
    {
        if (ShipsTechTokenIncome)
        {
            var techTokenOwner = Players.FirstOrDefault(p => p.TechTokens.Contains(TechToken.Ships));
            if (techTokenOwner != null)
            {
                var amount = techTokenOwner.TechTokens.Count;
                techTokenOwner.Resources += amount;
                Log(techTokenOwner.Faction, " receive ", Payment.Of(amount), " from ", TechToken.Ships);
            }
        }
    }

    #region Intrusions

    internal bool CheckIntrusion(ILocationEvent e)
    {
        CheckBeneGesseritIntrusion(e, e.Initiator, e.To!.Territory);
        CheckAmbassadorTriggered(e);
        CheckTerrorTriggered(e);
        return Intrusions.Count > 0;
    }

    private void CheckBeneGesseritIntrusion(ILocationEvent e, Faction initiator, Territory territory)
    {
        var bgPlayer = GetPlayer(Faction.BeneGesserit);
        if (bgPlayer != null &&
            territory != Map.PolarSink.Territory &&
            !territory.IsHomeworld &&
            Applicable(Rule.BeneGesseritAdvisors) &&
            initiator != bgPlayer.Ally &&
            initiator != Faction.BeneGesserit &&
            bgPlayer.ForcesIn(territory) > 0)
        {
            if (Prevented(FactionAdvantage.BeneGesseritIntrusion))
            {
                LogPreventionByKarma(FactionAdvantage.BeneGesseritIntrusion);
                if (!Applicable(Rule.FullPhaseKarma)) Allow(FactionAdvantage.BeneGesseritIntrusion);
            }
            else
            {
                QueueIntrusion(e, IntrusionType.BeneGesseritIntrusion);
            }
        }
    }

    private void CheckAmbassadorTriggered(ILocationEvent e)
    {
        var ecazPlayer = GetPlayer(Faction.Ecaz);
        if (ecazPlayer != null &&
            (Version < 160 || e.Initiator != Faction.BeneGesserit || e.ForcesAddedToLocation > 0) &&
            e.Initiator != Faction.Ecaz &&
            e.Initiator != ecazPlayer.Ally &&
            AmbassadorIn(e.To!.Territory) != Ambassador.None &&
            AmbassadorIn(e.To.Territory) != AmbassadorOf(e.Initiator))
            QueueIntrusion(e, IntrusionType.Ambassador);
    }

    private void CheckTerrorTriggered(ILocationEvent e)
    {
        var moritaniPlayer = GetPlayer(Faction.Moritani);
        if (moritaniPlayer != null &&
            (e.TotalAmountOfForcesAddedToLocation >= 3 || !moritaniPlayer.HasLowThreshold()) &&
            e.Initiator != Faction.Moritani &&
            e.Initiator != moritaniPlayer.Ally &&
            !IsOccupiedByFactionOrTheirAlly(World.Moritani, e.Initiator) &&
            TerrorIn(e.To!.Territory).Any())
            QueueIntrusion(e, IntrusionType.Terror);
    }

    private void QueueIntrusion(ILocationEvent e, IntrusionType type)
    {
        Intrusions.Enqueue(new Intrusion(e, type));
    }

    internal void DequeueIntrusion(IntrusionType type)
    {
        if (Intrusions.Count == 0)
            throw new ArgumentException("No intrusions to dequeue");
        if (Intrusions.Peek().Type != type) throw new ArgumentException($"Wrong intrusion type: actual:{type} vs expected:{Intrusions.Peek().Type}");

        Intrusions.Dequeue();
    }

    #endregion Intrusions


    #region Information

    public Leader? Vidal => LeaderState.Keys.FirstOrDefault(h => h.HeroType == HeroType.Vidal) as Leader;
    public bool VidalIsAlive => Vidal != null && IsAlive(Vidal);

    public bool VidalIsCapturedOrGhola
    {
        get
        {
            var playerWithVidal = Players.FirstOrDefault(p => p.Leaders.Any(l => l.HeroType == HeroType.Vidal));
            return playerWithVidal != null && (CapturedLeaders.Keys.Any(h => h.HeroType == HeroType.Vidal) || playerWithVidal.Is(Faction.Tleilaxu));
        }
    }


    public bool PreventedFromShipping(Faction f)
    {
        return CurrentKarmaShipmentPrevention != null && CurrentKarmaShipmentPrevention.Target == f;
    }

    public bool HasShipmentPermission(Player p, ShipmentPermission permission)
    {
        return ShipmentPermissions.TryGetValue(p.Faction, out var permissions) &&
               (permissions & permission) == permission;
    }

    public Intrusion? LastBeneGesseritIntrusion => Intrusions.Count > 0 && Intrusions.Peek().Type == IntrusionType.BeneGesseritIntrusion ? Intrusions.Peek() : null;

    public Intrusion? LastTerrorTrigger => Intrusions.Count > 0 && Intrusions.Peek().Type == IntrusionType.Terror ? Intrusions.Peek() : null;

    public Intrusion? LastAmbassadorTrigger => Intrusions.Count > 0 && Intrusions.Peek().Type == IntrusionType.Ambassador ? Intrusions.Peek() : null;

    internal bool OccupiesArrakeenOrCarthag(Player p)
    {
        return p.Occupies(Map.Arrakeen) || p.Occupies(Map.Carthag);
    }

    internal static Ambassador AmbassadorOf(Faction faction)
    {
        return (Ambassador)(int)faction;
    }

    public bool GuildMayDelay => GuildMayShipOutOfTurnOrder && Players.Count - HasActedOrPassed.Count > (JuiceForcesLastPlayer && !HasActedOrPassed.Contains(CurrentJuice.Initiator) ? 2 : 1);

    internal bool GuildMayShipOutOfTurnOrder => Applicable(Rule.GuildDetermineShipment) && (Version < 113 || !Prevented(FactionAdvantage.GuildDetermineMoveMoment));

    #endregion Information
}