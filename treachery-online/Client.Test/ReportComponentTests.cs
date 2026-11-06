using Bunit;
using Treachery.Bots;
using Treachery.Client.OtherComponents;
using Treachery.Shared;

namespace Treachery.Client.Test;

[TestClass]
[DoNotParallelize]
public sealed class ReportComponentTests
{
    [TestMethod]
    [Timeout(30_000, CooperativeCancellation = true)]
    public void StaysOnAnEarlierPhaseUntilSteppedBackToTheLatest()
    {
        var driver = new BotDriver();
        driver.PlayUntil(game => PhaseCount(game) >= 4);
        using var test = new ComponentTestContext(driver.Game);
        var component = test.RenderContext.Render<ReportComponent>();

        var latest = PhaseName(test, driver.Game.CurrentReport);
        Assert.AreEqual(latest, ShownPhase(component));
        Assert.IsTrue(Next(component).HasAttribute("disabled"));

        Previous(component).Click();
        var earlier = ShownPhase(component);
        Assert.AreNotEqual(latest, earlier);

        // A new phase starts. The panel stays where the player left it.
        var count = PhaseCount(driver.Game);
        driver.PlayUntil(game => PhaseCount(game) > count);
        component.Render();
        Assert.AreEqual(earlier, ShownPhase(component));

        // Stepping forward to the latest phase follows again.
        while (!Next(component).HasAttribute("disabled")) Next(component).Click();
        Assert.AreEqual(PhaseName(test, driver.Game.CurrentReport), ShownPhase(component));
        count = PhaseCount(driver.Game);
        driver.PlayUntil(game => PhaseCount(game) > count);
        component.Render();
        Assert.AreEqual(PhaseName(test, driver.Game.CurrentReport), ShownPhase(component));
    }

    [TestMethod]
    [Timeout(60_000, CooperativeCancellation = true)]
    public void BattlesHaveCaptionsAndResults()
    {
        var driver = new BotDriver();
        driver.PlayUntil(game => game.Reports.Any(r => r.IsBattle && r.BattleDecided));
        using var test = new ComponentTestContext(driver.Game);
        var component = test.RenderContext.Render<ReportComponent>();

        var battle = driver.Game.Reports.First(r => r.IsBattle);
        var block = component.FindAll(".battle").First();
        var heading = block.QuerySelector(".faceoff")!;
        var sides = heading.QuerySelectorAll(".side");
        Assert.AreEqual(DefaultSkin.Default.Describe(battle.Aggressor), sides[0].TextContent.Trim());
        Assert.AreEqual(DefaultSkin.Default.Describe(battle.Defender), sides[1].TextContent.Trim());
        Assert.AreEqual(DefaultSkin.Default.Describe(battle.Territory), heading.QuerySelector(".where")!.TextContent.Trim());
        var expected = battle.BattleWinner == Faction.None ? "No winner" : $"{DefaultSkin.Default.Describe(battle.BattleWinner)} win";
        Assert.AreEqual(expected, heading.GetAttribute("title"));
        Assert.AreEqual(battle.BattleWinner == battle.Aggressor, sides[0].ClassList.Contains("win"));
        Assert.AreEqual(battle.BattleWinner == battle.Defender, sides[1].ClassList.Contains("win"));
        Assert.IsFalse(block.TextContent.Contains("initiate battle against"));
        Assert.IsFalse(block.ClassList.Contains("live"));
    }

    [TestMethod]
    [Timeout(30_000, CooperativeCancellation = true)]
    public void AsidesShowCardsAndAdvantagesInTheSentence()
    {
        var driver = new BotDriver();
        driver.PlayUntil(game => PhaseCount(game) >= 2);
        var report = driver.Game.CurrentReport;
        var karama = TreacheryCardManager.GetCardsInAndOutsidePlay().First(c => c.Type == TreacheryCardType.Karma);
        report.Express(Message.Express(Faction.Black, " play ", karama, MessagePart.Express(": ", MessagePart.Express("prevent ", FactionAdvantage.RedReceiveBid))).WithLog(AsideLog.Aside));
        report.Express(Message.Express("By ", karama, ", ", Faction.Red, " ask ", Faction.Yellow, " a question").WithLog(AsideLog.Aside));
        report.Express(Message.Express(Faction.Yellow, " answer: No").WithLog(AsideLog.Answer));
        report.Express(Message.Express(FactionAdvantage.BlackFreeCard, " is no longer prevented").WithLog(AsideLog.After));
        using var test = new ComponentTestContext(driver.Game);
        var component = test.RenderContext.Render<ReportComponent>();

        var asides = component.FindAll(".log-aside").TakeLast(4).ToList();
        Assert.AreEqual(4, asides.Count);

        // Atreides (Black) play [Karama]: prevent [Emperor] receiving [spice] for a treachery card
        var karamaLine = asides[0];
        Assert.AreEqual(2, karamaLine.QuerySelectorAll(".chip.faction").Length, karamaLine.InnerHtml);
        Assert.IsNotNull(karamaLine.QuerySelector(".card-chip"), karamaLine.InnerHtml);
        Assert.IsNotNull(karamaLine.QuerySelector(".spice-amt"), karamaLine.InnerHtml);
        Assert.IsNull(karamaLine.QuerySelector(".fw-bold"), karamaLine.InnerHtml);
        StringAssert.Contains(karamaLine.TextContent, "for a treachery card");
        Assert.IsFalse(karamaLine.TextContent.Contains("spice"), karamaLine.TextContent);

        Assert.IsTrue(asides[2].ClassList.Contains("answer"));
        Assert.IsTrue(asides[3].ClassList.Contains("after"));
        Assert.IsNotNull(asides[3].QuerySelector(".chip.faction"), asides[3].InnerHtml);
    }

    [TestMethod]
    [Timeout(30_000, CooperativeCancellation = true)]
    public void SpiceBlowHasDeckHeadingsAndPlaceChips()
    {
        var driver = new BotDriver();
        driver.PlayUntil(game => game.CurrentReport.About == MainPhase.Blow && game.CurrentReport.Messages.Count(m => m.Heading != null) == 2);
        using var test = new ComponentTestContext(driver.Game);
        var component = test.RenderContext.Render<ReportComponent>();

        var headings = component.FindAll(".card-body .blow-deck");
        CollectionAssert.AreEqual(new[] { "Deck A", "Deck B" }, headings.Select(h => h.TextContent.Trim()).ToArray());

        var spiceLines = component.FindAll(".card-body > div")
            .Where(line => line.TextContent.Contains("detected in") || line.TextContent.Contains("lost in the storm"))
            .ToList();
        Assert.IsGreaterThanOrEqualTo(2, spiceLines.Count);
        foreach (var line in spiceLines)
            Assert.IsNotNull(line.QuerySelector(".chip.place"), line.TextContent);
    }

    private static int PhaseCount(Game game) =>
        game.Reports.Select(r => (r.Turn, r.About)).Distinct().Count();

    private static string PhaseName(ComponentTestContext test, Report report) =>
        test.Client.CurrentSkin.Describe(report.About);

    private static string ShownPhase(IRenderedComponent<ReportComponent> component) =>
        component.Find(".stepper .phase").TextContent;

    private static AngleSharp.Dom.IElement Previous(IRenderedComponent<ReportComponent> component) =>
        component.Find("button[title='Previous phase']");

    private static AngleSharp.Dom.IElement Next(IRenderedComponent<ReportComponent> component) =>
        component.Find("button[title='Next phase']");

}
