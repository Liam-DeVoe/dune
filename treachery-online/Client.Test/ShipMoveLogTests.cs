using Bunit;
using Microsoft.AspNetCore.Components;
using Treachery.Bots;
using Treachery.Client.OtherComponents.Log;
using Treachery.Shared;

namespace Treachery.Client.Test;

[TestClass]
[DoNotParallelize]
public sealed class ShipMoveLogTests
{
    [TestMethod]
    [Timeout(60_000, CooperativeCancellation = true)]
    public void EveryShipmentIsABlockWithShipAndMoveRows()
    {
        var driver = new BotDriver();
        driver.PlayUntil(game => game.Reports.Any(r => r.About == MainPhase.ShipmentAndMove) && game.CurrentReport.About != MainPhase.ShipmentAndMove);
        using var test = new ComponentTestContext(driver.Game);

        var messages = driver.Game.Reports.First(r => r.About == MainPhase.ShipmentAndMove).Messages.Where(m => m.Target == Faction.None).ToList();
        var component = Render(test, messages);

        var ships = messages.Select(m => m.Log).OfType<ShipLogged>().ToList();
        var blocks = component.FindAll(".shipmove-turn");
        Assert.HasCount(ships.Count, blocks);
        for (var i = 0; i < ships.Count; i++)
        {
            Assert.AreEqual(DefaultSkin.Default.Describe(ships[i].Faction), blocks[i].QuerySelector(".head .chip.faction .name")!.TextContent.Trim());
            var labels = Labels(blocks[i]);
            Assert.IsTrue(labels[0] is "Ship" or "Rally", string.Join(",", labels));
            Assert.IsTrue(labels.Contains("Move"), string.Join(",", labels));
        }

        Assert.DoesNotContain(" pass shipment", component.Markup);
        Assert.DoesNotContain(" pass movement", component.Markup);
    }

    [TestMethod]
    public void ReactionsAndLossesFollowTheLineThatCausedThem()
    {
        var game = BotGeneratedComponentSmokeTests.CreateGame();
        using var test = new ComponentTestContext(game);
        var map = game.Map;
        var karama = TreacheryCardManager.GetCardsInAndOutsidePlay().First(c => c.Type == TreacheryCardType.Karma);

        var messages = new List<Message>
        {
            // Two delays in a row are one line.
            Message.Express(Faction.Orange, " delay their turn").WithLog(new GuildDelayed()),
            Message.Express(Faction.Orange, " delay their turn").WithLog(new GuildDelayed()),
            // Fremen storm losses are logged before the shipment.
            Message.Express(3, Faction.Yellow, " forces are killed by the storm").WithLog(new ShipMoveLoss(Faction.Yellow, true, false, 3, null)),
            Message.Express(Faction.Yellow, " rally ...").WithLog(new ShipLogged(Faction.Yellow, ShipKind.Normal, 3, 0, null, map.Arrakeen, 0, 0, Faction.None, false, null, false, null, false)),
            Message.Express(Faction.Yellow, " pass movement").WithLog(new MovePassed(Faction.Yellow)),
            // A Karama shipment: its discard line goes.
            Message.Express(Faction.Black, " discard ", karama).WithFact(new BattleFact(BattleFactKind.Discard, Faction.Black) { Card = karama }),
            Message.Express(Faction.Black, " ship ...").WithLog(new ShipLogged(Faction.Black, ShipKind.Normal, 8, 0, null, map.Carthag, 4, 0, Faction.None, false, karama, false, null, false)),
            Message.Express(Faction.Blue, " stay as advisors").WithLog(new BlueReaction(BlueReactionKind.StayAs, map.Carthag.Territory, true)),
            Message.Express(Faction.Blue, " accompany").WithLog(new BlueReaction(BlueReactionKind.Accompany, map.PolarSink, false)),
            // A move from two territories, a storm loss logged between the two lines.
            Message.Express(Faction.Black, " move a").WithLog(new MoveLogged(Faction.Black, 7, map.Carthag.Territory, 2, 0, map.Arrakeen, false, null)),
            Message.Express(1, Faction.Black, "forces are killed by the storm while travelling").WithLog(new ShipMoveLoss(Faction.Black, true, true, 1, null)),
            Message.Express(Faction.Black, " move b").WithLog(new MoveLogged(Faction.Black, 7, map.PolarSink.Territory, 1, 0, map.Arrakeen, false, null)),
            Message.Express(Faction.Blue, " become fighters").WithLog(new BlueReaction(BlueReactionKind.Become, map.Arrakeen.Territory, false)),
            Message.Express("All ", Faction.Black, " forces in ", map.Arrakeen.Territory, " were killed due to ally presence").WithLog(new ShipMoveLoss(Faction.Black, false, false, 0, map.Arrakeen.Territory)),
            Message.Express("between turns"),
            // The game skips the move before logging the shipment: the Move row still comes last.
            Message.Express(Faction.Orange, " pass movement").WithLog(new MovePassed(Faction.Orange)),
            Message.Express(Faction.Orange, " ship back").WithLog(new ShipLogged(Faction.Orange, ShipKind.BackToReserves, 2, 0, map.Arrakeen, null, 1, 0, Faction.None, false, null, false, null, false)),
            Message.Express(Faction.Green, " pass shipment").WithLog(new ShipLogged(Faction.Green, ShipKind.Passed, 0, 0, null, null, 0, 0, Faction.None, false, null, true, karama, false)),
            Message.Express("inside the turn"),
            Message.Express(Faction.Green, " move hajr").WithLog(new MoveLogged(Faction.Green, 8, map.Carthag.Territory, 2, 0, map.Arrakeen, true, null)),
            Message.Express(Faction.Green, " pass movement").WithLog(new MovePassed(Faction.Green)),
        };
        var component = Render(test, messages);

        var delays = component.FindAll(".shipmove-delay");
        Assert.HasCount(1, delays);
        StringAssert.Contains(delays[0].TextContent, "×2");

        var blocks = component.FindAll(".shipmove-turn");
        Assert.HasCount(4, blocks);
        CollectionAssert.AreEqual(new[] { "Rally", "+", "Move" }, Labels(blocks[0]));
        StringAssert.Contains(Rows(blocks[0])[1], "are killed by the storm");
        CollectionAssert.AreEqual(new[] { "Ship", "+", "+", "Move", "+", "+", "+" }, Labels(blocks[1]));
        var black = Rows(blocks[1]);
        StringAssert.Contains(black[1], "stay as");
        StringAssert.Contains(black[2], "accompany to");
        StringAssert.Contains(black[3], "+");
        StringAssert.Contains(black[4], "while travelling");
        StringAssert.Contains(black[5], "become");
        StringAssert.Contains(black[6], "ally presence");
        Assert.IsNotNull(blocks[1].QuerySelector(".card-chip"));
        CollectionAssert.AreEqual(new[] { "Ship", "Move" }, Labels(blocks[2]));
        StringAssert.Contains(Rows(blocks[2])[0], "reserves");
        CollectionAssert.AreEqual(new[] { "Ship", "Hajr", "Move" }, Labels(blocks[3]));
        StringAssert.Contains(Rows(blocks[3])[0], "blocked by");
        Assert.HasCount(1, blocks[3].QuerySelectorAll(".other .plain"));

        var plain = component.FindAll(".plain").Select(p => p.TextContent.TrimEnd('*')).ToList();
        CollectionAssert.AreEqual(new[] { "between turns", "inside the turn" }, plain, string.Join(" | ", plain));
    }

    private static IRenderedComponent<ShipMoveLog> Render(ComponentTestContext test, List<Message> messages)
    {
        RenderFragment<Message> line = m => b => b.AddMarkupContent(0, $"<div class=\"plain\">{m}</div>");
        return test.RenderContext.Render<ShipMoveLog>(p => p
            .Add(x => x.Messages, messages)
            .Add(x => x.Line, line));
    }

    private static string[] Labels(AngleSharp.Dom.IElement block) =>
        block.QuerySelectorAll(".rows > .log-label").Select(l => l.TextContent.Trim()).ToArray();

    private static string[] Rows(AngleSharp.Dom.IElement block) =>
        block.QuerySelectorAll(".rows > .route").Select(r => r.TextContent.Trim()).ToArray();

}
