// "review-a-part2" mode: package A review probes (EndPhase, PlacementEvent,
// Utilities, GameState, ObjectCounter). tests/review_pkg_a_part2.rs hardcodes
// the values it checks.
//
//   dotnet run -c Release -- review-a-part2

using Treachery.Shared;

public static class ReviewA2
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

    private static string Parse(Game g, string s) =>
        string.Join(";", PlacementEvent.ParseForceLocations(g, Faction.Emperor, s).Select(kv => $"{kv.Key.Id}={kv.Value.AmountOfForces}/{kv.Value.AmountOfSpecialForces}"));

    public static void Run()
    {
        var g = new Game();
        var hw = g.Map.GetHomeWorld(World.Emperor)!.Id;
        var ds = g.Map.Shrine.Id;
        var hms = g.Map.HiddenMobileStronghold.Id;
        P("ids", () => $"{hw},{ds},{hms}");
        P("parse_dup_homeworld", () => Parse(g, $"{hw}:1|0,{hw}:2|0"));
        P("parse_dup_discovered", () => Parse(g, $"{ds}:1|0,{ds}:2|0"));
        P("parse_dup_hms", () => Parse(g, $"{hms}:1|0,{hms}:2|0"));
        P("parse_trailing_comma", () => Parse(g, "5:1|2,"));
        P("parse_plus", () => Parse(g, "+5:+1|-0"));
        P("parse_overflow", () => Parse(g, "5:2147483648|0"));
        P("parse_neg_location", () => Parse(g, "-2:1|0"));
        P("objcounter_int_empty_highest", () => new ObjectCounter<int>().Highest);
        P("objcounter_faction_empty_highest", () => new ObjectCounter<Faction>().Highest);
        P("load_null", () => GameState.Load("null").Version);
        P("load_empty_obj", () => GameState.Load("{}").Events.Count());
        P("load_events_null", () => GameState.Load("{\"_events\":null}").Events == null);
        P("load_event_no_type", () => GameState.Load("{\"_events\":[{}]}").Events.Count());
        P("load_event_unknown_type", () => GameState.Load("{\"_events\":[{\"$type\":\"Nope\"}]}").Events.Count());
        var pad = new string(' ', 130);
        P("load_legacy_unknown_event", () => GameState.Load("{\"$type\":\"Treachery.Shared.GameState, Treachery.Shared\",\"_events\":{\"$type\":\"x\",\"$values\":[{\"$type\":\"Treachery.Shared.Nope, Treachery.Shared\"}]}}" + pad).Events.Count());
        P("load_legacy_root_wrapper", () => GameState.Load("{\"$values\":[],\"x\":\"Treachery.Shared.GameState\"}" + pad).Events.Count());
        P("load_legacy_root_only_values", () => GameState.Load("{\"$type\":\"Treachery.Shared.GameState\",\"$values\":[]}" + pad).Events.Count());
        P("load_dollar_member", () => GameState.Load("{\"$values\":[],\"Version\":3}").Version);
        P("load_dollar_id", () => GameState.Load("{\"$id\":\"1\",\"Version\":3}").Version);
        P("ev_dollar_foo", () => Utilities.Deserialize<GameEvent>("{\"$type\":\"EndPhase\",\"$foo\":1,\"Initiator\":3}")!.Initiator);
        P("ev_dollar_id", () => Utilities.Deserialize<GameEvent>("{\"$type\":\"EndPhase\",\"$id\":\"1\",\"Initiator\":3}")!.Initiator);
        P("ev_nested_dollar", () => ((EstablishPlayers)Utilities.Deserialize<GameEvent>("{\"$type\":\"EstablishPlayers\",\"Settings\":{\"$foo\":1,\"MaximumTurns\":4}}")!).Settings.MaximumTurns);
        P("ev_nested_dollar_values", () => ((EstablishPlayers)Utilities.Deserialize<GameEvent>("{\"$type\":\"EstablishPlayers\",\"Settings\":{\"$values\":1,\"MaximumTurns\":4}}")!).Settings.MaximumTurns);
        P("state_dollar_foo", () => GameState.Load("{\"$foo\":1,\"Version\":3}").Version);
        P("state_dollar_ref", () => GameState.Load("{\"$ref\":\"1\",\"Version\":3}").Version);
        P("state_dollar_type", () => GameState.Load("{\"$type\":\"x\",\"Version\":3}").Version);
        P("px_nan", () => Utilities.Px(double.NaN));
        P("px_big", () => Utilities.Px(1e20));
        P("scale_font", () => Utilities.ScaleFont("Bold 1,000PX Arial 1.5px", 1.1f));
    }
}
