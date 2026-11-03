/*
 * Copyright (C) 2020-2025 Ronald Ossendrijver (admin@treachery.online)
 * This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License
 * as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version. This
 * program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details. You should have
 * received a copy of the GNU General Public License along with this program. If not, see <http://www.gnu.org/licenses/>.
 */


using System.Linq;
using Microsoft.VisualStudio.TestTools.UnitTesting;
using Treachery.Shared;
using Treachery.Shared.Model;

namespace Treachery.Test;

[TestClass]
public class DealTests
{
    private static Game GameWith(params Faction[] factions)
    {
        var game = new Game();
        foreach (var faction in factions) game.Players.Add(new Player(game, faction));
        return game;
    }

    private static DealOffered Offer(Game game, Faction from, params Faction[] to)
    {
        var offer = new DealOffered(game, from) { To = to, Type = DealType.None, Text = "no attacks", EndPhase = Phase.TurnConcluded };
        Assert.IsNull(offer.Execute(false, false));
        return offer;
    }

    [TestMethod]
    public void DecliningHidesTheOfferFromThatRecipientOnly()
    {
        var game = GameWith(Faction.Green, Faction.Black, Faction.Red);
        var offer = Offer(game, Faction.Green);

        var rejection = offer.Rejection(Faction.Black);
        Assert.IsNull(rejection.Validate());
        Assert.IsNull(rejection.Execute(false, false));

        Assert.IsFalse(game.DealResponse(offer, Faction.Black));
        Assert.IsNull(game.DealResponse(offer, Faction.Red));
        Assert.IsEmpty(DealAccepted.AcceptableDeals(game, game.GetPlayer(Faction.Black)).ToArray());
        Assert.HasCount(1, DealAccepted.AcceptableDeals(game, game.GetPlayer(Faction.Red)).ToArray());

        Assert.IsNotNull(offer.Rejection(Faction.Black).Validate());
        Assert.IsNotNull(offer.Acceptance(Faction.Black).Validate());
    }

    [TestMethod]
    public void AnOfferCanOnlyBeAcceptedOncePerRecipient()
    {
        var game = GameWith(Faction.Green, Faction.Black);
        var offer = Offer(game, Faction.Green);

        Assert.IsNull(offer.Acceptance(Faction.Black).Validate());
        Assert.IsNull(offer.Acceptance(Faction.Black).Execute(false, false));

        Assert.IsTrue(game.DealResponse(offer, Faction.Black));
        Assert.HasCount(1, game.Deals);
        Assert.IsNotNull(offer.Acceptance(Faction.Black).Validate());
        Assert.IsNotNull(offer.Rejection(Faction.Black).Validate());
    }

    [TestMethod]
    public void MakingTheOfferAgainAsksAgain()
    {
        var game = GameWith(Faction.Green, Faction.Black);
        var offer = Offer(game, Faction.Green);
        Assert.IsNull(offer.Rejection(Faction.Black).Execute(false, false));

        Assert.IsNull(offer.Cancellation().Execute(false, false));
        Assert.IsEmpty(game.DealResponses);

        Offer(game, Faction.Green);
        Assert.HasCount(1, DealAccepted.AcceptableDeals(game, game.GetPlayer(Faction.Black)).ToArray());
    }

    [TestMethod]
    public void RecipientsOfAnOpenOfferExcludeTheOfferersAlly()
    {
        var game = GameWith(Faction.Green, Faction.Black, Faction.Red);
        game.GetPlayer(Faction.Green)!.Ally = Faction.Red;
        game.GetPlayer(Faction.Red)!.Ally = Faction.Green;

        CollectionAssert.AreEqual(new[] { Faction.Black }, game.DealRecipients(Offer(game, Faction.Green)).ToArray());
        CollectionAssert.AreEqual(new[] { Faction.Red }, game.DealRecipients(Offer(game, Faction.Black, Faction.Red)).ToArray());
    }

    [TestMethod]
    public void OnlyRecipientsMayDecline()
    {
        var game = GameWith(Faction.Green, Faction.Black, Faction.Red);
        var offer = Offer(game, Faction.Green, Faction.Black);

        Assert.IsNotNull(offer.Rejection(Faction.Red).Validate());
    }
}
