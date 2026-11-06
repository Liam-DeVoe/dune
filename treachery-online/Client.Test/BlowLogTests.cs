using Bunit;
using Microsoft.AspNetCore.Components;
using Treachery.Bots;
using Treachery.Client.OtherComponents.Log;
using Treachery.Shared;

namespace Treachery.Client.Test;

[TestClass]
[DoNotParallelize]
public sealed class BlowLogTests
{
    [TestMethod]
    [Timeout(60_000, CooperativeCancellation = true)]
    public void EachDrawnCardIsShownAsItsCard()
    {
        var driver = new BotDriver();
        driver.PlayUntil(game => game.CurrentReport.About == MainPhase.Blow && game.CurrentReport.Messages.Count(m => m.Log is BlowCardLog) == 2);
        using var test = new ComponentTestContext(driver.Game);
        var component = Render(test, driver.Game.CurrentReport);

        CollectionAssert.AreEqual(new[] { "Deck A", "Deck B" }, component.FindAll(".blow-deck").Select(h => h.TextContent.Trim()).ToArray());

        var cards = driver.Game.CurrentReport.Messages.Select(m => m.Log).OfType<BlowCardLog>().ToList();
        var shown = component.FindAll(".blow-card");
        Assert.HasCount(cards.Count, shown);
        for (var i = 0; i < cards.Count; i++)
        {
            Assert.AreEqual(DefaultSkin.Default.GetImageUrl(cards[i].Card), shown[i].QuerySelector("img.card")!.GetAttribute("src"));
            Assert.AreEqual(cards[i].Lost, shown[i].QuerySelector("img.card")!.ClassList.Contains("lost"));
            Assert.AreEqual(cards[i].Amount.ToString(), shown[i].QuerySelector(".spice-amt b")!.TextContent.Trim());
            Assert.IsNotNull(shown[i].QuerySelector(".chip.place"));
        }
    }

    [TestMethod]
    [Timeout(120_000, CooperativeCancellation = true)]
    public void AWormIsAnAsideFollowedByTheNexus()
    {
        var driver = new BotDriver();
        driver.PlayUntil(game => game.CurrentMainPhase > MainPhase.Blow && game.Reports.Any(HasWorm));
        using var test = new ComponentTestContext(driver.Game);
        var report = driver.Game.Reports.Last(HasWorm);
        var component = Render(test, report);

        var worm = component.Find(".log-aside.blow-worm");
        Assert.IsNotNull(worm.QuerySelector("img[alt='Shai Hulud']"));
        Assert.DoesNotContain("Shai Hulud", worm.TextContent);
        Assert.DoesNotContain(" forces in ", worm.TextContent);
        Assert.IsNotEmpty(component.FindAll(".log-aside.blow-nexus .log-label"));
    }

    [TestMethod]
    public void AnAcceptedOfferIsFoldedIntoItsAlliance()
    {
        var driver = new BotDriver();
        using var test = new ComponentTestContext(driver.Game);
        Message Change(NexusAllianceChange change, Faction from, Faction to) =>
            Message.Express(from, " ", change, " ", to).WithLog(new NexusLog(Alliance: new NexusAlliance(change, from, to)));
        var messages = new List<Message>
        {
            Change(NexusAllianceChange.Offered, Faction.Blue, Faction.Black),
            Change(NexusAllianceChange.Offered, Faction.Yellow, Faction.Black),
            Change(NexusAllianceChange.Made, Faction.Black, Faction.Yellow),
            Change(NexusAllianceChange.Ended, Faction.Red, Faction.Orange),
        };
        var component = test.RenderContext.Render<BlowLog>(p => p.Add(x => x.Messages, messages).Add(x => x.Line, _ => _ => { }));

        var rows = component.FindAll(".blow-nexus .nexus-step")
            .Select(r => (r.QuerySelector("svg title")!.TextContent, string.Join(",", r.QuerySelectorAll(".chip").Select(c => c.GetAttribute("data-ref-faction")))))
            .ToArray();
        CollectionAssert.AreEqual(new[]
        {
            ("offer an alliance to", "Blue,Black"),
            ("allied with", "Yellow,Black"),
            ("ended their alliance with", "Red,Orange"),
        }, rows);
    }

    private static bool HasWorm(Report report) =>
        report.About == MainPhase.Blow && report.Messages.Any(m => m.Log is WormAppearedLog);

    private static IRenderedComponent<BlowLog> Render(ComponentTestContext test, Report report)
    {
        var messages = report.Messages.Where(m => m.Target == Faction.None).ToList();
        RenderFragment<Message> line = m => builder =>
        {
            builder.OpenElement(0, "div");
            builder.AddAttribute(1, "class", "plain");
            builder.AddContent(2, m.ToString(DefaultSkin.Default));
            builder.CloseElement();
        };
        return test.RenderContext.Render<BlowLog>(p => p.Add(x => x.Messages, messages).Add(x => x.Line, line));
    }

}
