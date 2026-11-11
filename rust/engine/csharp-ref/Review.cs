// "review" mode: probes C# behaviour that the static golden file does not
// cover (exception messages, draw counts, string formatting). Each line is
// "name = value"; tests/review_foundation.rs hardcodes the values it checks.
//
//   dotnet run -c Release -- review

using System.Globalization;
using System.Reflection;
using Treachery.Shared;

public static class Review
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

    // State of the private static ExtensionMethods.Random, to count draws.
    private static string StaticRandomState()
    {
        var rnd = (Random)typeof(ExtensionMethods).GetField("Random", BindingFlags.NonPublic | BindingFlags.Static)!.GetValue(null)!;
        var impl = typeof(Random).GetField("_impl", BindingFlags.NonPublic | BindingFlags.Instance)!.GetValue(rnd)!;
        return string.Join(",", impl.GetType().GetFields(BindingFlags.NonPublic | BindingFlags.Instance).Select(f => f.GetValue(impl)?.ToString()));
    }

    public static void Run()
    {
        P("culture", () => CultureInfo.CurrentCulture.Name);

        // EnumerationExtensions.ToNexus / ToFaction on undefined values
        P("to_nexus_13", () => ((Faction)13).ToNexus());
        P("to_faction_13", () => ((Nexus)13).ToFaction());

        // List indexer through LocationLookup
        P("location_lookup_200", () => new Map().LocationLookup.Find(200));

        // LoggedRandom.Next(-1): exception and count
        {
            var r = new LoggedRandom(1);
            P("logged_random_next_neg", () => r.Next(-1));
            P("logged_random_next_neg_count", () => r.GetCount());
        }

        // LINQ predicate overloads: messages
        P("single_or_default_pred_two", () => new[] { 1, 1 }.SingleOrDefault(x => x == 1));
        P("single_pred_none", () => new[] { 1 }.Single(x => x == 2));
        P("single_pred_two", () => new[] { 1, 1 }.Single(x => x == 1));

        // TakeRandomN: draws from the static Random
        {
            var src = new List<int> { 1, 2, 3, 4, 5 };
            var before = StaticRandomState();
            var q = src.TakeRandomN(0).ToList();
            P("take_random_n_0_draws", () => StaticRandomState() != before);
            before = StaticRandomState();
            var lazy = src.TakeRandomN(2);
            P("take_random_n_lazy_draws_before_enumeration", () => StaticRandomState() != before);
            // Count key computations of the same LINQ shape
            var n = 0;
            _ = src.OrderBy(_ => { n++; return 0; }).Take(0).ToList();
            P("orderby_take0_key_calls", () => n);
            n = 0;
            _ = src.OrderBy(_ => { n++; return 0; }).Take(-3).ToList();
            P("orderby_take_neg_key_calls", () => n);
            n = 0;
            _ = new List<int> { 7 }.OrderBy(_ => { n++; return 0; }).Take(1).ToList();
            P("orderby_take1_single_key_calls", () => n);
        }

        // Message.ToString of numbers without a describer
        P("msg_float_small", () => Message.Express(0.00001f).ToString());
        P("msg_float_big", () => Message.Express(1e20f).ToString());
        P("msg_double_small", () => Message.Express(1e-7).ToString());
        P("msg_double_big", () => Message.Express(1e16).ToString());
        P("msg_float_inf", () => Message.Express(float.PositiveInfinity).ToString());
        P("msg_float_third", () => Message.Express(1f / 3f).ToString());

        P("float_inf_invariant", () => float.PositiveInfinity.ToString(CultureInfo.InvariantCulture));
        P("float_small_invariant", () => 0.00001f.ToString(CultureInfo.InvariantCulture));

        // int.MinValue % -1 and / -1 on this machine
        {
            var a = int.MinValue;
            var b = -1;
            P("int_min_rem_minus1", () => a % b);
            P("int_min_div_minus1", () => a / b);
        }

        // CounteredBy with a null defense
        var laser = TreacheryCardManager.Get(0)!;
        var poison = TreacheryCardManager.Get(5)!;
        P("laser_countered_by_null", () => laser.CounteredBy(null!, null!));
        P("poison_countered_by_null", () => poison.CounteredBy(null!, null!));

        // Highest/Lowest with string keys: culture comparison
        {
            var r = new LoggedRandom(1);
            P("highest_string", () => new[] { "a", "B" }.HighestOrDefault(r, s => s));
            P("one_of_highest_1_string", () => new[] { "a", "B" }.OneOfHighestNOrDefault(r, s => s, 1));
        }

        // Highest with float keys where NaN is present
        {
            var r = new LoggedRandom(1);
            P("highest_float_nan", () => new[] { 1f, float.NaN, 2f }.HighestOrDefault(r, x => x));
            P("lowest_float_nan", () => new[] { 1f, float.NaN, 2f }.LowestOrDefault(r, x => x));
        }

        // Selector call counts
        {
            var r = new LoggedRandom(1);
            var calls = 0;
            _ = new[] { 5, 3, 9, 1, 7 }.OneOfHighestNOrDefault(r, x => { calls++; return x; }, 2);
            P("one_of_highest_selector_calls", () => calls);
            calls = 0;
            _ = new[] { 5, 3, 9, 1, 7 }.OneOfLowestNOrDefault(r, x => { calls++; return x; }, 2);
            P("one_of_lowest_selector_calls", () => calls);
            calls = 0;
            _ = new[] { 5, 3, 9, 1, 7 }.HighestOrDefault(r, x => { calls++; return x; });
            P("highest_selector_calls", () => calls);
        }

        // GetDiscoveryStronghold for an undefined token
        P("discovery_stronghold_99", () => new Map().GetDiscoveryStronghold((DiscoveryToken)99));

        // Deck.Draw on empty
        P("deck_draw_empty", () => new Deck<int>(new LoggedRandom(1)).Draw());

        // Exception text in a message element
        P("msg_exception", () => Message.Express(new KeyNotFoundException("k")).ToString());

        // Record equality with NaN
        P("plan_summary_nan_equal", () =>
            new PlanSummary(Faction.Atreides, null, false, float.NaN, null, 0, Faction.None, null, null) ==
            new PlanSummary(Faction.Atreides, null, false, float.NaN, null, 0, Faction.None, null, null));

        // Route equality compares the From list by reference
        P("route_equal_distinct_lists", () =>
            new Route(Faction.Atreides, new List<Location>(), null) == new Route(Faction.Atreides, new List<Location>(), null));

        // Message.ToString of a list element
        P("msg_list", () => Message.Express(new List<Faction> { Faction.Atreides }).ToString());
    }
}
