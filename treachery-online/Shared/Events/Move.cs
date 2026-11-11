/*
 * Copyright (C) 2020-2025 Ronald Ossendrijver (admin@treachery.online)
 * This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License
 * as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version. This
 * program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details. You should have
 * received a copy of the GNU General Public License along with this program. If not, see <http://www.gnu.org/licenses/>.
 */

namespace Treachery.Shared;

public class Move : PlacementEvent
{
    #region Construction

    public Move(Game game, Faction initiator) : base(game, initiator)
    {
    }

    public Move()
    {
    }

    #endregion Construction

    #region Properties

    public bool AsAdvisors { get; set; }

    #endregion Properties

    #region Validation

    public override Message? Validate()
    {
        if (!Passed && Game.InGuildCunningShipment) return Message.Express("You cannot move after Cunning shipment");

        return ValidateMove(AsAdvisors);
    }

    #endregion Validation

    #region Execution

    protected override void ExecuteConcreteEvent()
    {
        Game.StormLossesToTake.Clear();

        Game.CurrentPlayerMayPerformExtraMove = Game.CurrentFlightUsed != null && Game.CurrentFlightUsed.Initiator == Initiator && Game.CurrentFlightUsed.ExtraMove;

        if (!Game.CurrentPlayerMayPerformExtraMove && !Game.InGuildCunningShipment)
        {
            Game.HasActedOrPassed.Add(Initiator);

            if (Game.CurrentPhase == Phase.NonGuildMove)
            {
                Game.ShipmentAndMoveSequence.NextPlayer();

                if (Game.ShipmentAndMoveSequence.CurrentFaction == Faction.Guild && Game.GuildMayShipOutOfTurnOrder) Game.ShipmentAndMoveSequence.NextPlayer();
            }
        }

        if (Game.InGuildCunningShipment)
        {
            Game.CurrentGuildCunning = null;
            Game.InGuildCunningShipment = false;
        }

        if (!Passed)
        {
            Game.RecentMoves.Add(this);

            if (Game.ContainsConflictingAlly(Player, To)) Game.ChosenDestinationsWithAllies.Add(To.Territory);

            Game.PerformMoveFromLocations(Player, ForceLocations, this, Initiator != Faction.BeneGesserit || AsAdvisors, false);
            Game.CheckIntrusion(this);
        }
        else
        {
            Log(Message.Express(Initiator, " pass movement").WithLog(new MovePassed(Initiator)));
        }

        Game.DetermineNextShipmentAndMoveSubPhase();
        Game.CheckIfForcesShouldBeDestroyedByAllyPresence(Player);
        Game.FlipBeneGesseritAdvisorsWhenAlone();

        if (!Game.Applicable(Rule.FullPhaseKarma)) Game.Allow(FactionAdvantage.FremenExtraMove);
        if (!Game.Applicable(Rule.FullPhaseKarma)) Game.Allow(FactionAdvantage.IxianCyborgExtraMove);

        Game.CurrentFlightUsed = null;
        Game.CurrentFlightDiscoveryUsed = null;
        Game.CurrentPlanetology = null;
    }

    public override Message GetMessage()
    {
        if (Passed)
            return Message.Express(Initiator, " pass move");
        return Message.Express(Initiator, " move to ", To);
    }

    #endregion Execution
}