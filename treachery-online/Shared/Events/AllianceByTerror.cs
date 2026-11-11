/*
 * Copyright (C) 2020-2025 Ronald Ossendrijver (admin@treachery.online)
 * This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License
 * as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version. This
 * program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details. You should have
 * received a copy of the GNU General Public License along with this program. If not, see <http://www.gnu.org/licenses/>.
 */

namespace Treachery.Shared;

public class AllianceByTerror : PassableGameEvent
{
    #region Construction

    public AllianceByTerror(Game game, Faction initiator) : base(game, initiator)
    {
    }

    public AllianceByTerror()
    {
    }

    #endregion Construction

    #region Validation

    public override Message? Validate()
    {
        return null;
    }

    #endregion Validation

    #region Execution

    protected override void ExecuteConcreteEvent()
    {
        Game.Enter(Game.PausedTerrorPhase);

        if (!Passed)
        {
            if (Player.HasAlly)
            {
                Log(Initiator, " and ", Player.Ally, " end their alliance");
                Game.BreakAlliance(Initiator);
            }

            var moritani = Game.GetPlayer(Faction.Moritani);
            if (moritani is { HasAlly: true })
            {
                Log(Faction.Moritani, " and ", moritani.Ally, " end their alliance");
                Game.BreakAlliance(Faction.Moritani);
            }

            Game.MakeAlliance(Initiator, Faction.Moritani);

            if (Game.HasActedOrPassed.Contains(Initiator) && Game.HasActedOrPassed.Contains(Faction.Moritani)) Game.CheckIfForcesShouldBeDestroyedByAllyPresence(Player);

            var territory = Game.LastTerrorTrigger!.Territory;
            Log("Terror in ", territory, " is returned to supplies");
            foreach (var t in Game.TerrorIn(territory).ToList())
            {
                Game.TerrorOnPlanet.Remove(t);
                Game.UnplacedTerrorTokens.Add(t);
            }

            Game.AllianceByTerrorWasOffered = false;
            Game.DequeueIntrusion(IntrusionType.Terror);
            Game.DetermineNextShipmentAndMoveSubPhase();
        }
        else
        {
            Log(Initiator, " don't ally with ", Faction.Moritani);
        }

        Game.LetFactionsDiscardSurplusCards();
    }

    public override Message GetMessage()
    {
        return Message.Express(Initiator, !Passed ? "" : " don't", " agree to ally");
    }

    #endregion Execution
}