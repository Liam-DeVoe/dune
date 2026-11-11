/*
 * Copyright (C) 2020-2025 Ronald Ossendrijver (admin@treachery.online)
 * This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License
 * as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version. This
 * program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details. You should have
 * received a copy of the GNU General Public License along with this program. If not, see <http://www.gnu.org/licenses/>.
 */


using System.Linq;
using System.Reflection;
using Microsoft.VisualStudio.TestTools.UnitTesting;
using Treachery.Shared;

namespace Treachery.Test;

[TestClass]
public class AlliancePhaseTests
{
    private static readonly Faction[] Factions = [Faction.Emperor, Faction.Atreides, Faction.Fremen, Faction.Harkonnen];

    [TestMethod]
    public void HostCannotEndThePhase()
    {
        var game = CreateGameInAlliancePhase();

        Assert.IsFalse(game.GetApplicableEvents(null, true).Contains(typeof(EndPhase)));
        foreach (var p in game.Players) Assert.IsTrue(game.GetApplicableEvents(p, false).Contains(typeof(AlliancePassed)));
    }

    [TestMethod]
    public void PhaseEndsWhenEveryoneHasPassedOrAllied()
    {
        var game = CreateGameInAlliancePhase();

        Do(game, new AllianceOffered(game, Faction.Emperor) { Target = Faction.Atreides });
        Do(game, new AllianceOffered(game, Faction.Atreides) { Target = Faction.Emperor });
        Assert.AreEqual(Faction.Atreides, game.GetPlayer(Faction.Emperor)!.Ally);
        Assert.AreEqual(Phase.AllianceB, game.CurrentPhase);
        Assert.AreEqual(0, game.GetApplicableEvents(game.GetPlayer(Faction.Emperor), false).Count(t => t == typeof(AlliancePassed)));

        Do(game, new AllianceOffered(game, Faction.Fremen) { Target = Faction.Harkonnen });
        Do(game, new AlliancePassed(game, Faction.Harkonnen));
        Assert.AreEqual(0, game.CurrentAllianceOffers.Count, "passing withdraws offers made to the passing faction");
        Assert.IsFalse(AllianceOffered.ValidTargets(game, game.GetPlayer(Faction.Fremen)!).Contains(Faction.Harkonnen));
        Assert.AreEqual(Phase.AllianceB, game.CurrentPhase);

        Do(game, new AlliancePassed(game, Faction.Fremen));
        Assert.AreNotEqual(Phase.AllianceB, game.CurrentPhase);
    }

    [TestMethod]
    public void BreakingAnAllianceLetsTheFormerAllyActAgain()
    {
        var game = CreateGameInAlliancePhase();
        Invoke(game, "MakeAlliance", Faction.Emperor, Faction.Atreides);

        Do(game, new AlliancePassed(game, Faction.Atreides));
        Assert.IsFalse(game.GetApplicableEvents(game.GetPlayer(Faction.Atreides), false).Contains(typeof(AlliancePassed)));

        Do(game, new AllianceBroken(game, Faction.Emperor));
        Assert.IsTrue(game.GetApplicableEvents(game.GetPlayer(Faction.Atreides), false).Contains(typeof(AllianceOffered)));

        foreach (var f in Factions) Do(game, new AlliancePassed(game, f));
        Assert.AreNotEqual(Phase.AllianceB, game.CurrentPhase);
    }

    private static void Do(Game game, GameEvent e)
    {
        Assert.IsNull(e.Validate(), e.GetType().Name);
        e.Execute(false, false);
    }

    private static Game CreateGameInAlliancePhase()
    {
        var game = new Game(Game.LatestVersion, new Participation());
        new EstablishPlayers(game, Faction.None)
        {
            Seed = 144,
            Settings = new GameSettings
            {
                NumberOfPlayers = 4,
                MaximumTurns = 10,
                AllowedFactionsInPlay = [..Factions]
            }
        }.Execute(false, true);

        Assert.IsTrue(Factions.All(f => game.IsPlaying(f)), "factions: " + string.Join(",", game.Players.Select(p => p.Faction)));
        typeof(Game).GetProperty(nameof(Game.CurrentPhase), BindingFlags.Instance | BindingFlags.Public | BindingFlags.NonPublic)!
            .SetValue(game, Phase.AllianceB);
        game.HasActedOrPassed.Clear();
        return game;
    }

    private static void Invoke(Game game, string method, params object[] args)
    {
        typeof(Game).GetMethod(method, BindingFlags.Instance | BindingFlags.Public | BindingFlags.NonPublic)!.Invoke(game, args);
    }
}
