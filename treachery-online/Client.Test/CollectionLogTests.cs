using Bunit;
using Microsoft.AspNetCore.Components;
using Treachery.Bots;
using Treachery.Client.OtherComponents;
using Treachery.Client.OtherComponents.Log;
using Treachery.Shared;

namespace Treachery.Client.Test;

[TestClass]
[DoNotParallelize]
public sealed class CollectionLogTests
{
    [TestMethod]
    [Timeout(60_000, CooperativeCancellation = true)]
    public void CollectionsAreRowsOfPlaceAmountAndFaction()
    {
        var driver = new BotDriver();
        driver.PlayUntil(game => game.CurrentReport.About == MainPhase.Collection && game.CurrentReport.Messages.Any(m => m.Log is SpiceCollected));
        using var test = new ComponentTestContext(driver.Game);
        var component = test.RenderContext.Render<ReportComponent>();

        var collected = driver.Game.CurrentReport.Messages.Select(m => m.Log).OfType<SpiceCollected>().ToList();
        var places = component.FindAll(".collection-table > .cell.place");
        var amounts = component.FindAll(".collection-table > .cell.amount");
        var factions = component.FindAll(".collection-table > .cell.faction");
        Assert.HasCount(collected.Count, places);
        for (var i = 0; i < collected.Count; i++)
        {
            Assert.AreEqual(DefaultSkin.Default.Describe(collected[i].Territory), places[i].QuerySelector(".chip.place .name")!.TextContent.Trim());
            Assert.AreEqual(collected[i].Amount.ToString(), amounts[i].QuerySelector(".spice-amt b")!.TextContent.Trim());
            Assert.AreEqual(DefaultSkin.Default.Describe(collected[i].Faction), factions[i].QuerySelector(".chip.faction .name")!.TextContent.Trim());
        }

        Assert.DoesNotContain(" collect ", component.Find(".card-body").TextContent);
    }

    [TestMethod]
    public void TwoFieldsInOneTerritoryNameTheirSectorsAndIncomeSitsUnderARule()
    {
        var game = BotGeneratedComponentSmokeTests.CreateGame();
        using var test = new ComponentTestContext(game);
        var territory = game.Map.Territories(false).First(t => t.Locations.Count(l => l.Orientation != "") >= 2);
        var west = territory.Locations.First(l => l.Orientation != "");
        var east = territory.Locations.Last(l => l.Orientation != "");
        var arrakeen = game.Map.Arrakeen;

        var messages = new List<Message>
        {
            Message.Express(Faction.Fremen, " collect ", Payment.Of(4), " from ", territory).WithLog(new SpiceCollected(Faction.Fremen, territory, 4, west, false)),
            Message.Express("an aside"),
            Message.Express(Faction.Fremen, " collect ", Payment.Of(2), " from ", territory).WithLog(new SpiceCollected(Faction.Fremen, territory, 2, east, false)),
            Message.Express(Faction.Atreides, " collect ", Payment.Of(2), " from ", arrakeen.Territory).WithLog(new SpiceCollected(Faction.Atreides, arrakeen.Territory, 2, null, true)),
        };
        RenderFragment<Message> line = m => b => b.AddMarkupContent(0, $"<div class=\"plain\">{m}</div>");
        var component = test.RenderContext.Render<CollectionLog>(p => p
            .Add(x => x.Messages, messages)
            .Add(x => x.Line, line));

        var sectors = component.FindAll(".collection-table .sector").Select(s => s.TextContent.Trim()).ToArray();
        CollectionAssert.AreEqual(new[] { $"{west.Orientation} sector", $"{east.Orientation} sector" }, sectors);

        var children = component.Find(".collection-table").Children;
        Assert.HasCount(10, children);
        Assert.IsTrue(children[6].ClassList.Contains("income-rule"));
        Assert.IsTrue(children[7].ClassList.Contains("first"));

        Assert.HasCount(1, component.FindAll(".plain"));
    }

}
