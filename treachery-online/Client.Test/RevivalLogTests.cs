using Bunit;
using Treachery.Bots;
using Treachery.Client.OtherComponents;
using Treachery.Shared;

namespace Treachery.Client.Test;

[TestClass]
[DoNotParallelize]
public sealed class RevivalLogTests
{
    [TestMethod]
    [Timeout(60_000, CooperativeCancellation = true)]
    public void RevivalsAreTableRowsWithOneTokenPerForce()
    {
        var driver = new BotDriver();
        driver.PlayUntil(game =>
            game.CurrentReport.About == MainPhase.Resurrection &&
            game.CurrentReport.Messages.Count(m => m.Log is RevivalRow) >= 2);
        using var test = new ComponentTestContext(driver.Game);
        var component = test.RenderContext.Render<ReportComponent>();

        var rows = driver.Game.CurrentReport.Messages.Select(m => m.Log).OfType<RevivalRow>().ToList();
        var table = component.Find(".revival-table");
        var what = table.QuerySelectorAll(".what");
        Assert.AreEqual(rows.Count, what.Length);

        for (var i = 0; i < rows.Count; i++)
        {
            var tokens = what[i].QuerySelectorAll(".force-token");
            Assert.AreEqual(rows[i].Forces + rows[i].SpecialForces, tokens.Length);
        }

        // The revival sentences are drawn by the table, not repeated as lines.
        Assert.DoesNotContain(" for free", component.Find(".card-body").TextContent);
    }

}
