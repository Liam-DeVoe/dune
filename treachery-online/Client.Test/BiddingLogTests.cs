using Bunit;
using Treachery.Bots;
using Treachery.Client.OtherComponents;
using Treachery.Shared;

namespace Treachery.Client.Test;

[TestClass]
[DoNotParallelize]
public sealed class BiddingLogTests
{
    [TestMethod]
    [Timeout(60_000, CooperativeCancellation = true)]
    public void EachSoldCardIsABlockWithWinnerAndPrice()
    {
        var driver = new BotDriver();
        driver.PlayUntil(game => game.CurrentReport.About == MainPhase.Bidding && game.CurrentReport.Messages.Count(m => m.Log is CardSold) >= 2);
        using var test = new ComponentTestContext(driver.Game);
        var component = test.RenderContext.Render<ReportComponent>();

        var sold = driver.Game.CurrentReport.Messages.Select(m => m.Log).OfType<CardSold>().ToList();
        var lots = component.FindAll(".bid-lot");
        Assert.HasCount(sold.Count, lots);
        for (var i = 0; i < sold.Count; i++)
        {
            Assert.AreEqual($"Card {sold[i].CardNumber}", lots[i].QuerySelector(".n")!.TextContent.Trim());
            Assert.AreEqual(DefaultSkin.Default.Describe(sold[i].Winner), lots[i].QuerySelector(".top > .chip.faction .name")!.TextContent.Trim());
            if (sold[i].KaramaCard == null)
            {
                Assert.AreEqual(sold[i].Price.ToString(), lots[i].QuerySelector(".price .spice-amt b")!.TextContent.Trim());
                Assert.AreEqual(sold[i].PaidTo != Faction.None, lots[i].QuerySelector(".price .payee") != null);
            }
        }

        Assert.Contains("cards were drawn for bidding", component.Find(".bid-note").TextContent);
    }

}
