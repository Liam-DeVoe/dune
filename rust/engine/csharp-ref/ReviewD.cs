// "review-d" mode: probes for the package D review (storm, spice blow, collection,
// charity). Each line is "name = value"; tests/review_pkg_d.rs hardcodes the values.
//
//   dotnet run -c Release -- review-d

using Treachery.Shared;
using Treachery.Shared.Model;

public static class ReviewD
{
    private static string Try(Func<object?> f)
    {
        try
        {
            var r = f();
            return r == null ? "null" : r.ToString()!;
        }
        catch (Exception e)
        {
            return "exception:" + e.GetType().Name + ":" + e.Message.Replace("\n", "\\n");
        }
    }

    private static void P(string name, Func<object?> f) => Console.WriteLine($"{name} = {Try(f)}");

    public static void Run()
    {
        // FremenRidesMonster.ValidTargets is a lazy Where, and Validate calls Contains on it.
        // A player whose Ally names a faction without a player has AlliedPlayer == null, so
        // the predicate throws, but only for locations that Contains actually reaches.
        var g = new Game();
        var fremen = new Player(g, Faction.Fremen);
        g.Players.Add(fremen);
        fremen.Ally = Faction.Atreides;
        typeof(Game).GetProperty("SectorInStorm")!.SetValue(g, 5);
        P("ridesValidTargets.firstLocationId", () => g.Map.Locations(false).First().Id);
        P("ridesValidTargets.containsPolarSink", () => FremenRidesMonster.ValidTargets(g, fremen).Contains(g.Map.PolarSink));
        P("ridesValidTargets.containsArrakeen", () => FremenRidesMonster.ValidTargets(g, fremen).Contains(g.Map.Arrakeen));

        P("ridesValidTargets.noAlly.count", () => { fremen.Ally = Faction.None; var n = FremenRidesMonster.ValidTargets(g, fremen).Count(); fremen.Ally = Faction.Atreides; return n; });



        // HarvesterPlayed: TryGetValue with a null key.
        P("tryGetValueNullKey", () => new Dictionary<Location, int>().TryGetValue(null!, out _));
    }
}
