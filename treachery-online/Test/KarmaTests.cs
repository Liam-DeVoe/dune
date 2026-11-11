/*
 * Copyright (C) 2020-2025 Ronald Ossendrijver (admin@treachery.online)
 * This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License
 * as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version. This
 * program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details. You should have
 * received a copy of the GNU General Public License along with this program. If not, see <http://www.gnu.org/licenses/>.
 */

using System.Collections.Generic;
using System.Linq;
using System.Reflection;
using Microsoft.VisualStudio.TestTools.UnitTesting;
using Treachery.Shared;

namespace Treachery.Test;

[TestClass]
public class KarmaTests
{
    [TestMethod]
    public void PlayingLastCardDuringIxianSwapContinuesBidding()
    {
        var game = CreateGameInIxianSwapPhase();
        var ixian = game.GetPlayer(Faction.Ixian)!;
        var karama = TreacheryCardManager.Get(23)!;
        ixian.TreacheryCards.Add(karama);

        new Karma(game, Faction.Ixian)
        {
            Card = karama,
            Prevented = FactionAdvantage.EmperorReceiveBid
        }.Execute(false, true);

        Assert.AreEqual(Phase.Bidding, game.CurrentPhase);
        Assert.IsFalse(ixian.TreacheryCards.Contains(karama));
        Assert.IsTrue(game.TreacheryDiscardPile!.Items.Contains(karama));
    }

    [TestMethod]
    public void PlayingKarmaDuringIxianSwapKeepsDecisionWhenAnotherCardRemains()
    {
        var game = CreateGameInIxianSwapPhase();
        var ixian = game.GetPlayer(Faction.Ixian)!;
        var karama = TreacheryCardManager.Get(23)!;
        ixian.TreacheryCards.Add(karama);
        ixian.TreacheryCards.Add(TreacheryCardManager.Get(1)!);

        new Karma(game, Faction.Ixian)
        {
            Card = karama,
            Prevented = FactionAdvantage.EmperorReceiveBid
        }.Execute(false, true);

        Assert.AreEqual(Phase.IxianSwappingCard, game.CurrentPhase);
    }

    [TestMethod]
    public void PlayingKarmaLogsOneAside()
    {
        var game = CreateGameInIxianSwapPhase();
        var ixian = game.GetPlayer(Faction.Ixian)!;
        var karama = TreacheryCardManager.Get(23)!;
        ixian.TreacheryCards.Add(karama);
        var before = game.CurrentReport.Messages.Count;

        new Karma(game, Faction.Ixian)
        {
            Card = karama,
            Prevented = FactionAdvantage.EmperorReceiveBid
        }.Execute(false, true);

        var logged = game.CurrentReport.Messages.Skip(before).ToList();
        Assert.AreEqual(1, logged.Count(m => m.Log is AsideLog));
        var aside = logged.Single(m => m.Log is AsideLog).Expression.Elements;
        Assert.AreEqual(Faction.Ixian, aside[0]);
        Assert.AreEqual(" play ", aside[1]);
        Assert.AreEqual(karama, aside[2]);
        Assert.IsFalse(logged.Any(m => m.Expression.Elements.Contains(" discard ")));
        Assert.IsFalse(logged.Any(m => m.Expression.Elements.Contains(" prevent ")));
    }

    private static Game CreateGameInIxianSwapPhase()
    {
        var game = new Game(Game.LatestVersion, new Participation());
        new EstablishPlayers(game, Faction.None)
        {
            Seed = 144,
            Settings = new GameSettings
            {
                NumberOfPlayers = 2,
                MaximumTurns = 10,
                InitialRules = [Rule.IxianSwappingCardOnBid],
                AllowedFactionsInPlay = [Faction.Ixian, Faction.Emperor]
            }
        }.Execute(false, true);

        SetProperty(game, nameof(Game.BidSequence), new PlayerSequence(game));
        SetProperty(game, "BiddingRoundWasStarted", true);
        SetProperty(game, nameof(Game.CurrentPhase), Phase.IxianSwappingCard);
        return game;
    }

    private static void SetProperty<T>(Game game, string propertyName, T value)
    {
        typeof(Game).GetProperty(propertyName, BindingFlags.Instance | BindingFlags.Public | BindingFlags.NonPublic)!
            .SetValue(game, value);
    }
}
