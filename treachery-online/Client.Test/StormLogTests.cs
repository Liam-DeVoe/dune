using Bunit;
using Microsoft.AspNetCore.Components;
using Treachery.Bots;
using Treachery.Client.OtherComponents;
using Treachery.Shared;

namespace Treachery.Client.Test;

[TestClass]
[DoNotParallelize]
public sealed class StormLogTests
{
    [TestMethod]
    [Timeout(60_000, CooperativeCancellation = true)]
    public void FirstTurnLightsTheStartingSectorAndShowsTheDials()
    {
        var driver = new BotDriver();
        driver.PlayUntil(game => game.CurrentReport.About == MainPhase.Storm && game.CurrentReport.Messages.Any(m => m.Log is StormPlaced));
        using var test = new ComponentTestContext(driver.Game);
        var component = test.RenderContext.Render<ReportComponent>();

        var placed = driver.Game.CurrentReport.Messages.Select(m => m.Log).OfType<StormPlaced>().Single();
        var ring = component.Find("svg.storm-ring");
        Assert.HasCount(Map.NumberOfSectors, ring.QuerySelectorAll("path.w"));
        Assert.AreEqual(placed.Sector, IndexOf(ring, "path.w.end"));
        Assert.IsEmpty(ring.QuerySelectorAll("path.w.swept"));
        Assert.IsNull(ring.QuerySelector(".distance"));
        Assert.HasCount(2, component.Find(".storm-dials").QuerySelectorAll(".chip.faction"));
        Assert.DoesNotContain("Storm dial", component.Find(".card-body").TextContent);
    }

    [TestMethod]
    [Timeout(60_000, CooperativeCancellation = true)]
    public void LaterTurnsSweepTheRingAndBoxEachPlace()
    {
        var driver = new BotDriver();
        driver.PlayUntil(game => game.CurrentReport.About == MainPhase.Storm && game.CurrentReport.Messages.Any(m => m.Log is StormMoved));
        using var test = new ComponentTestContext(driver.Game);
        var component = test.RenderContext.Render<ReportComponent>();

        var data = driver.Game.CurrentReport.Messages.Select(m => m.Log).ToList();
        var moved = data.OfType<StormMoved>().Single();
        var ring = component.Find("svg.storm-ring");
        Assert.AreEqual((moved.From + moved.Distance) % Map.NumberOfSectors, IndexOf(ring, "path.w.end"));
        Assert.HasCount(Math.Max(0, moved.Distance - 1), ring.QuerySelectorAll("path.w.swept"));
        Assert.AreEqual(moved.Distance.ToString(), ring.QuerySelector(".distance")!.TextContent.Trim());

        var places = data.Select(d => d switch { StormKilled k => k.Location, StormDestroyedSpice s => s.Location, _ => null })
            .OfType<Location>().Select(l => l.Territory).Distinct().Count();
        Assert.HasCount(places, component.FindAll(".log-box.storm-place"));

        var text = component.Find(".card-body").TextContent;
        Assert.DoesNotContain("The storm moves", text);
        Assert.DoesNotContain("The storm will move", text);
    }

    [TestMethod]
    [Timeout(60_000, CooperativeCancellation = true)]
    public void AsidesAndBoxesFromTheLogData()
    {
        var driver = new BotDriver();
        driver.PlayUntil(game => game.CurrentReport.About == MainPhase.Storm && game.CurrentReport.Messages.Any(m => m.Log is StormMoved));
        var game = driver.Game;
        using var test = new ComponentTestContext(game);

        var flat = game.Map.Territories(false).First(t => !t.IsStronghold && t.Locations.Any(l => l.Sector >= 0));
        var location = flat.Locations.First(l => l.Sector >= 0);
        var sietch = game.Map.Territories(false).First(t => t.IsStronghold);
        Message M(object log) => Message.Express("plain").WithLog(log);
        var messages = new List<Message>
        {
            M(new BlueFlipped(sietch)),
            M(new FamilyAtomicsUsed(Faction.Black, null)),
            M(new FamilyAtomicsKilled(Faction.Yellow, 2, 1)),
            M(new StormForecast(3)),
            M(new StormMoved(location.Sector - 1 + Map.NumberOfSectors, 1)),
            M(new StormKilled(Faction.Yellow, location, 2, 1)),
            M(new StormDestroyedSpice(location, 6)),
            Message.Express("an ambassador line")
        };

        var component = test.RenderContext.Render<Treachery.Client.OtherComponents.Log.StormLog>(ps => ps
            .Add(c => c.Messages, messages)
            .Add(c => c.Line, (RenderFragment<Message>)(m => b => b.AddContent(0, "LINE"))));

        var asides = component.FindAll(".log-aside");
        Assert.HasCount(2, asides);
        StringAssert.Contains(asides[0].TextContent, "are alone and flip to");
        Assert.IsNotNull(asides[0].QuerySelector(".force-token"));
        StringAssert.Contains(asides[1].TextContent, "to destroy the");

        var boxes = component.FindAll(".log-box");
        Assert.HasCount(2, boxes);
        StringAssert.Contains(boxes[0].TextContent, "kills");
        Assert.HasCount(2, boxes[0].QuerySelectorAll(".storm-forces"));
        Assert.AreEqual(string.Join(",", flat.Locations.Where(l => l == location).Select(l => l.Id)), boxes[1].GetAttribute("data-ref-locations"));
        StringAssert.Contains(boxes[1].TextContent, "The storm kills");
        StringAssert.Contains(boxes[1].TextContent, "The storm destroys");
        Assert.AreEqual(1, component.Markup.Split("LINE").Length - 1);

        boxes[1].MouseEnter();
        var hot = component.FindAll("svg.storm-ring path.w.hot");
        Assert.HasCount(1, hot);
        Assert.Contains("end", hot[0].ClassList);
    }

    private static int IndexOf(AngleSharp.Dom.IElement ring, string selector) =>
        ring.QuerySelectorAll("path.w").ToList().IndexOf(ring.QuerySelector(selector)!);

}
