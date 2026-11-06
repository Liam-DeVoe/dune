using Bunit;
using Treachery.Bots;
using Treachery.Client.OtherComponents;
using Treachery.Shared;

namespace Treachery.Client.Test;

[TestClass]
[DoNotParallelize]
public sealed class CharityLogTests
{
    [TestMethod]
    [Timeout(60_000, CooperativeCancellation = true)]
    public void ClaimsAreRowsOfFactionAndAmount()
    {
        var driver = new BotDriver();
        driver.PlayUntil(game => game.CurrentReport.About == MainPhase.Charity && game.CurrentReport.Messages.Any(m => m.Log is CharityClaim));
        using var test = new ComponentTestContext(driver.Game);
        var component = test.RenderContext.Render<ReportComponent>();

        var claims = driver.Game.CurrentReport.Messages.Select(m => m.Log).OfType<CharityClaim>().ToList();
        var cells = component.Find(".log-cols").Children;
        Assert.AreEqual(claims.Count * 2, cells.Length);
        for (var i = 0; i < claims.Count; i++)
        {
            Assert.AreEqual(DefaultSkin.Default.Describe(claims[i].Faction), cells[2 * i].QuerySelector(".chip.faction .name")!.TextContent.Trim());
            Assert.AreEqual(claims[i].Amount.ToString(), cells[2 * i + 1].QuerySelector(".spice-amt b")!.TextContent.Trim());
        }

        Assert.DoesNotContain(" claim ", component.Find(".card-body").TextContent);
    }

}
