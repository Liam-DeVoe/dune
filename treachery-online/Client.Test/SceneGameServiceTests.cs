using Treachery.Client.Scene;
using Treachery.Shared;

namespace Treachery.Client.Test;

[TestClass]
public sealed class SceneGameServiceTests
{
    private static string Fixture(string name)
    {
        var directory = new DirectoryInfo(AppContext.BaseDirectory);
        while (directory != null && !File.Exists(Path.Combine(directory.FullName, "treachery.online.sln")))
            directory = directory.Parent;
        var repo = directory?.Parent ?? throw new DirectoryNotFoundException("Could not find the repository root.");
        return File.ReadAllText(Path.Combine(repo.FullName, "scripts", "fixtures", name + ".json"));
    }

    [TestMethod]
    public void LoadSeatsTheFactionAsHostWithItsActions()
    {
        var scene = new SceneGameService();
        scene.Load(Fixture("ship"), Faction.Emperor);

        Assert.IsTrue(scene.InGame);
        Assert.IsTrue(scene.IsHost);
        Assert.AreEqual(Faction.Emperor, scene.Faction);
        Assert.IsNotNull(scene.Status);
        Assert.Contains(typeof(Shipment), scene.Actions);
        Assert.IsFalse(scene.Game!.CurrentReport.Messages.Any(m => m.ToString(DefaultSkin.Default).Contains("now controls")));
    }

    [TestMethod]
    public async Task RequestGameEventExecutesLocallyAndRefreshes()
    {
        var scene = new SceneGameService();
        scene.Load(Fixture("worm-ride"), Faction.Fremen);
        var refreshed = 0;
        scene.RefreshHandler += () => refreshed++;
        var eventsBefore = scene.Game!.EventCount;

        var error = await scene.RequestGameEvent(new FremenRidesMonster(scene.Game, Faction.Fremen) { Passed = true });

        Assert.IsNull(error);
        Assert.AreEqual(eventsBefore + 1, scene.Game.EventCount);
        Assert.AreEqual(1, refreshed);
        Assert.DoesNotContain(typeof(FremenRidesMonster), scene.Actions);
    }

    [TestMethod]
    public async Task RequestGameEventReturnsTheValidationMessageForAnInvalidEvent()
    {
        var scene = new SceneGameService();
        scene.Load(Fixture("ship"), Faction.Emperor);

        var error = await scene.RequestGameEvent(new FremenRidesMonster(scene.Game!, Faction.Emperor) { Passed = true });

        Assert.IsNotNull(error);
    }

    [TestMethod]
    public void StepsAndFactionsParse()
    {
        var steps = SceneStep.ParseAll("click:Carthag|shift:Polar Sink|button:Ride!");
        Assert.AreEqual(3, steps.Count);
        Assert.AreEqual("Carthag", steps[0].Target);
        Assert.IsTrue(steps[1].Shift);
        Assert.IsTrue(steps[2].IsButton);
        Assert.ThrowsExactly<ArgumentException>(() => SceneStep.ParseAll("hover:Carthag"));

        Assert.AreEqual(Faction.Emperor, SceneScript.ParseFaction("emperor"));
        Assert.AreEqual(Faction.Emperor, SceneScript.ParseFaction("Emperor"));
        Assert.AreEqual(Faction.Emperor, SceneScript.ParseFaction("4"));
        Assert.IsNull(SceneScript.ParseFaction("teal"));
    }

    [TestMethod]
    public void LocationsAreNamedAsTheDefaultSkinNamesThem()
    {
        var scene = new SceneGameService();
        scene.Load(Fixture("ship"), Faction.Emperor);
        var named = SceneScript.NamedLocations(scene.Game!);

        Assert.AreEqual(4, named["Carthag"].Id);
        Assert.AreEqual(24, named["False Wall South (East Sector)"].Id);
        Assert.AreEqual(0, named["Polar Sink"].Id);
        Assert.IsNotNull(SceneScript.FindLocation(scene.Game!, "polar sink"));
        Assert.IsNotNull(SceneScript.FindLocation(scene.Game!, "24"));
    }
}
