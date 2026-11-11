/*
 * Copyright (C) 2020-2025 Ronald Ossendrijver (admin@treachery.online)
 * This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License
 * as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version. This
 * program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details. You should have
 * received a copy of the GNU General Public License along with this program. If not, see <http://www.gnu.org/licenses/>.
 */

using System;

namespace Treachery.Shared;

public class RequestTleilaxuRevival : GameEvent
{
    #region Construction

    public RequestTleilaxuRevival(Game game, Faction initiator) : base(game, initiator)
    {
    }

    public RequestTleilaxuRevival()
    {
    }

    #endregion Construction

    #region Properties

    public int _heroId;

    [JsonIgnore]
    public IHero Hero
    {
        get => LeaderManager.HeroLookup.Find(_heroId);
        set => _heroId = LeaderManager.HeroLookup.GetId(value);
    }

    #endregion Properties

    #region Validation

    public override Message? Validate()
    {
        if (!ValidTargets(Game, Player).Contains(Hero)) return Message.Express(Hero, " can't be revived this way");

        return null;
    }

    public static IEnumerable<IHero> ValidTargets(Game g, Player p)
    {
        var tleilaxu = g.GetPlayer(Faction.Tleilaxu);
        var gholas = tleilaxu != null ? tleilaxu.Leaders.Where(l => l.Faction == p.Faction) : [];
        return g.KilledHeroes(p).Union(gholas);
    }

    #endregion Validation

    #region Execution

    protected override void ExecuteConcreteEvent()
    {
        var existingRequest = Game.CurrentRevivalRequests.FirstOrDefault(r => r.Hero == Hero);
        if (existingRequest != null) Game.CurrentRevivalRequests.Remove(existingRequest);

        Log();
        Game.CurrentRevivalRequests.Add(this);
    }

    public override Message GetMessage()
    {
        return Message.Express(Initiator, " request ", Faction.Tleilaxu, " revival of a leader");
    }

    #endregion Execution
}