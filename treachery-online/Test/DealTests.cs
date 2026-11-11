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
        var game = GameWith(Faction.Atreides, Faction.Harkonnen, Faction.Emperor);
        var offer = Offer(game, Faction.Atreides);

        var rejection = offer.Rejection(Faction.Harkonnen);
        Assert.IsNull(rejection.Validate());
        Assert.IsNull(rejection.Execute(false, false));

        Assert.IsFalse(game.DealResponse(offer, Faction.Harkonnen));
        Assert.IsNull(game.DealResponse(offer, Faction.Emperor));
        Assert.IsEmpty(DealAccepted.AcceptableDeals(game, game.GetPlayer(Faction.Harkonnen)).ToArray());
        Assert.HasCount(1, DealAccepted.AcceptableDeals(game, game.GetPlayer(Faction.Emperor)).ToArray());

        Assert.IsNotNull(offer.Rejection(Faction.Harkonnen).Validate());
        Assert.IsNotNull(offer.Acceptance(Faction.Harkonnen).Validate());
    }

    [TestMethod]
    public void AnOfferCanOnlyBeAcceptedOncePerRecipient()
    {
        var game = GameWith(Faction.Atreides, Faction.Harkonnen);
        var offer = Offer(game, Faction.Atreides);

        Assert.IsNull(offer.Acceptance(Faction.Harkonnen).Validate());
        Assert.IsNull(offer.Acceptance(Faction.Harkonnen).Execute(false, false));

        Assert.IsTrue(game.DealResponse(offer, Faction.Harkonnen));
        Assert.HasCount(1, game.Deals);
        Assert.IsNotNull(offer.Acceptance(Faction.Harkonnen).Validate());
        Assert.IsNotNull(offer.Rejection(Faction.Harkonnen).Validate());
    }

    [TestMethod]
    public void MakingTheOfferAgainAsksAgain()
    {
        var game = GameWith(Faction.Atreides, Faction.Harkonnen);
        var offer = Offer(game, Faction.Atreides);
        Assert.IsNull(offer.Rejection(Faction.Harkonnen).Execute(false, false));

        Assert.IsNull(offer.Cancellation().Execute(false, false));
        Assert.IsEmpty(game.DealResponses);

        Offer(game, Faction.Atreides);
        Assert.HasCount(1, DealAccepted.AcceptableDeals(game, game.GetPlayer(Faction.Harkonnen)).ToArray());
    }

    [TestMethod]
    public void RecipientsOfAnOpenOfferExcludeTheOfferersAlly()
    {
        var game = GameWith(Faction.Atreides, Faction.Harkonnen, Faction.Emperor);
        game.GetPlayer(Faction.Atreides)!.Ally = Faction.Emperor;
        game.GetPlayer(Faction.Emperor)!.Ally = Faction.Atreides;

        CollectionAssert.AreEqual(new[] { Faction.Harkonnen }, game.DealRecipients(Offer(game, Faction.Atreides)).ToArray());
        CollectionAssert.AreEqual(new[] { Faction.Emperor }, game.DealRecipients(Offer(game, Faction.Harkonnen, Faction.Emperor)).ToArray());
    }

    [TestMethod]
    public void OnlyRecipientsMayDecline()
    {
        var game = GameWith(Faction.Atreides, Faction.Harkonnen, Faction.Emperor);
        var offer = Offer(game, Faction.Atreides, Faction.Harkonnen);

        Assert.IsNotNull(offer.Rejection(Faction.Emperor).Validate());
    }
}
