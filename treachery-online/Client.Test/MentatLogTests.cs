using Bunit;
using Treachery.Client.OtherComponents.Log;
using Treachery.Shared;

namespace Treachery.Client.Test;

[TestClass]
public sealed class MentatLogTests
{
    [TestMethod]
    public void BribesAreRowsOfFactionAndAmount()
    {
        var html = Render(
            Message.Express(Faction.Atreides, " add ", Payment.Of(3), " from bribes to their reserves").WithLog(new BribesCollected(Faction.Atreides, 3)),
            Message.Express(Faction.Guild, " add ", Payment.Of(5), " from bribes to their reserves").WithLog(new BribesCollected(Faction.Guild, 5)));

        Assert.AreEqual("Bribes collected", html.Find(".log-label").TextContent.Trim());
        var cells = html.Find(".log-cols").Children;
        Assert.AreEqual(4, cells.Length);
        Assert.AreEqual("5", cells[3].QuerySelector(".spice-amt b")!.TextContent.Trim());
        Assert.IsEmpty(html.FindAll(".plain"));
    }

    [TestMethod]
    public void GameEndIsOneBlockWithWinnersAndReason()
    {
        var html = Render(
            Message.Express(Faction.Harkonnen, " have enough victory points to win the game").WithLog(new WinReason(WinMethod.Strongholds)),
            Message.Express("The game has ended.").WithLog(new GameEnded([Faction.Harkonnen])),
            Message.Express(Faction.Harkonnen, " win!").WithLog(new GameWinner(Faction.Harkonnen)));

        var block = html.Find(".mentat-result");
        Assert.AreEqual("The game has ended.", block.QuerySelector(".title")!.TextContent.Trim());
        Assert.HasCount(1, block.QuerySelectorAll(".winner"));
        Assert.Contains("have enough victory points to win the game", block.QuerySelector(".reason")!.TextContent);
        Assert.IsEmpty(html.FindAll(".plain"));
    }

    [TestMethod]
    public void PredictionIsTheReasonAndTheEarlierReasonStaysAPlainLine()
    {
        var html = Render(
            Message.Express(Faction.Emperor, " and ", Faction.Guild, " have enough victory points to win the game").WithLog(new WinReason(WinMethod.Strongholds)),
            Message.Express(Faction.BeneGesserit, " predicted ", Faction.Emperor, " victory in turn ", 6, "! They had everything planned...").WithLog(new WinReason(WinMethod.Prediction)),
            Message.Express("The game has ended.").WithLog(new GameEnded([Faction.BeneGesserit])),
            Message.Express(Faction.BeneGesserit, " win!").WithLog(new GameWinner(Faction.BeneGesserit)));

        Assert.Contains("predicted", html.Find(".mentat-result .reason").TextContent);
        var plain = html.FindAll(".plain");
        Assert.HasCount(1, plain);
        Assert.Contains("have enough victory points", plain[0].TextContent);
    }

    private static IRenderedComponent<MentatLog> Render(params Message[] messages)
    {
        var test = new ComponentTestContext();
        return test.RenderContext.Render<MentatLog>(p => p
            .Add(c => c.Messages, messages.ToList())
            .Add(c => c.Line, m => $"<div class=\"plain\">{m.ToString(DefaultSkin.Default)}</div>"));
    }
}
