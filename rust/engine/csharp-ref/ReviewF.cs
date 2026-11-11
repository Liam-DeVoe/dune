// "review-f" mode: probes for the package F review (revival and leaders).
// Each line is "name = value"; tests/review_pkg_f.rs hardcodes the values.
//
//   dotnet run -c Release -- review-f

using Treachery.Shared;

public static class ReviewF
{
    private static void P(string name, object? v) => Console.WriteLine($"{name} = {v ?? "null"}");

    private static Game Run(params string[] events)
    {
        var p = new Participation();
        p.PlayerNames[100] = "Human100";
        p.SeatedPlayers[100] = 2;
        var g = new Game(187, p);
        foreach (var json in events)
        {
            var e = Utilities.Deserialize<GameEvent>(json)!;
            e.Initialize(g);
            var r = e.Execute(true, true);
            if (r != null) throw new Exception("rejected: " + json);
        }
        return g;
    }

    public static void Run()
    {
        // 1. SetIncreasedRevivalLimits with a null Factions array, executed without
        //    validation (Game.TryLoad with performValidation false, or any other
        //    Execute(false, ...)). ExecuteConcreteEvent stores the null before Log()
        //    throws, and GetMaxRevivals later throws on it.
        {
            var g = Run(ReviewA.Establish);
            var faction = g.Players[0].Faction;
            var other = g.Players[1];
            var e = Utilities.Deserialize<GameEvent>(
                $$"""{"$type":"SetIncreasedRevivalLimits","Factions":null,"Initiator":{{(int)faction}},"Time":"2024-01-01T12:02:40+00:00"}""")!;
            e.Initialize(g);
            var r = e.Execute(false, true);
            P("sirl_null.result_null", r == null);
            P("sirl_null.history", g.History.Count);
            P("sirl_null.limits_is_null", g.FactionsWithIncreasedRevivalLimits == null);
            try
            {
                P("sirl_null.max_revivals", g.GetMaxRevivals(g, other, false));
            }
            catch (Exception ex)
            {
                P("sirl_null.max_revivals.exception", ex.GetType().Name);
                P("sirl_null.max_revivals.message", ex.Message);
            }
        }
    }
}
