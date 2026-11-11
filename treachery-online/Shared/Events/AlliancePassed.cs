/*
 * Copyright (C) 2020-2025 Ronald Ossendrijver (admin@treachery.online)
 * This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License
 * as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version. This
 * program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details. You should have
 * received a copy of the GNU General Public License along with this program. If not, see <http://www.gnu.org/licenses/>.
 */

namespace Treachery.Shared;

/// <summary>
/// A faction is done with the alliance phase: it will not make an alliance (or keeps its current one).
/// The phase ends once every faction has passed or formed an alliance (since v188).
/// </summary>
public class AlliancePassed : GameEvent
{
    #region Construction

    public AlliancePassed(Game game, Faction initiator) : base(game, initiator)
    {
    }

    public AlliancePassed()
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
        Log();

        while (Game.CurrentAllianceOffers.FirstOrDefault(x => x.Initiator == Initiator || x.Target == Initiator) is { } withdrawnOffer)
            Game.CurrentAllianceOffers.Remove(withdrawnOffer);

        Game.HasActedOrPassed.Add(Initiator);
        Game.EndNexusIfEveryoneIsDone();
    }

    public override Message GetMessage()
    {
        return Player.HasAlly
            ? Message.Express(Initiator, " keep their alliance with ", Player.Ally)
            : Message.Express(Initiator, " will not make an alliance");
    }

    #endregion Execution
}
