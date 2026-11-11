// random_helpers.json: draw accounting for every random helper in
// ExtensionMethods and Deck<T>, over a grid of seeds, source sizes, counts
// and key shapes. Each entry records the result, how often the selector ran,
// LoggedRandom.GetCount(), and the next value the random would produce, so a
// port that draws one number too many (or calls the selector a different
// number of times) fails even when the picked item happens to agree.
//
// The unseeded overloads use the private static ExtensionMethods.Random. This
// program reseeds it by swapping in the internal state of `new Random(seed)`
// through reflection; Shared itself is not changed.

using System.Reflection;
using System.Text.Json.Nodes;
using Treachery.Shared;

public static class RandomGrid
{
    private static readonly int[] Seeds = [1, 42];
    private static readonly int[] Sizes = [0, 1, 2, 5];
    private static readonly string[] Strings = ["b", "B", "a", "A", "a-b", "ab", "Ab", "_"];

    // Key shapes. Items are 0..size-1; each shape maps an item to its key.
    public static readonly (string Name, Func<int, IComparable> Key)[] Keys =
    [
        ("distinct", x => (x * 7 + 3) % 11),
        ("ties", x => x % 2),
        ("enum", x => x % 2 == 0 ? Faction.Atreides : Faction.Harkonnen),
        ("bool", x => x % 3 == 0),
        ("float", x => x == 1 ? float.NaN : x % 3 == 2 ? -0f : (float)(x % 3)),
        ("double", x => x == 3 ? double.NaN : (double)(x % 2) / 3),
        ("string", x => Strings[x % Strings.Length])
    ];

    private static Random StaticRandom =>
        (Random)typeof(ExtensionMethods).GetField("Random", BindingFlags.NonPublic | BindingFlags.Static)!.GetValue(null)!;

    public static void SeedStatic(int seed)
    {
        var impl = typeof(Random).GetField("_impl", BindingFlags.NonPublic | BindingFlags.Instance)!;
        impl.SetValue(StaticRandom, impl.GetValue(new Random(seed)));
    }

    private static string StaticState()
    {
        var implField = typeof(Random).GetField("_impl", BindingFlags.NonPublic | BindingFlags.Instance)!;
        var impl = implField.GetValue(StaticRandom)!;
        var prng = impl.GetType().GetField("_prng", BindingFlags.NonPublic | BindingFlags.Instance)!.GetValue(impl)!;
        return string.Join(",", prng.GetType().GetFields(BindingFlags.NonPublic | BindingFlags.Instance)
            .Select(f => f.GetValue(prng) is int[] a ? string.Join(":", a) : f.GetValue(prng)?.ToString()));
    }

    public static JsonObject Dump()
    {
        var seeded = new JsonArray();
        var unseeded = new JsonArray();
        var takeRandomN = new JsonArray();
        var deck = new JsonArray();

        foreach (var seed in Seeds)
        foreach (var size in Sizes)
        {
            var source = Enumerable.Range(0, size).ToArray();
            var ns = new[] { -1, 0, 1, size, size + 1 }.Distinct().ToArray();

            // RandomOrDefault has no selector.
            seeded.Add(Seeded(seed, size, "RandomOrDefault", "-", 0, (r, _) => source.RandomOrDefault(r), _ => 0));
            unseeded.Add(Unseeded(seed, size, "RandomOrDefault", "-", 0, _ => source.RandomOrDefault(), _ => 0));

            foreach (var (keyName, key) in Keys)
            {
                seeded.Add(Seeded(seed, size, "HighestOrDefault", keyName, 0, (r, sel) => source.HighestOrDefault(r, sel), key));
                seeded.Add(Seeded(seed, size, "LowestOrDefault", keyName, 0, (r, sel) => source.LowestOrDefault(r, sel), key));
                unseeded.Add(Unseeded(seed, size, "HighestOrDefault", keyName, 0, sel => source.HighestOrDefault(sel), key));
                unseeded.Add(Unseeded(seed, size, "LowestOrDefault", keyName, 0, sel => source.LowestOrDefault(sel), key));
                foreach (var n in ns)
                {
                    seeded.Add(Seeded(seed, size, "OneOfHighestNOrDefault", keyName, n, (r, sel) => source.OneOfHighestNOrDefault(r, sel, n), key));
                    seeded.Add(Seeded(seed, size, "OneOfLowestNOrDefault", keyName, n, (r, sel) => source.OneOfLowestNOrDefault(r, sel, n), key));
                    unseeded.Add(Unseeded(seed, size, "OneOfHighestNOrDefault", keyName, n, sel => source.OneOfHighestNOrDefault(sel, n), key));
                    unseeded.Add(Unseeded(seed, size, "OneOfLowestNOrDefault", keyName, n, sel => source.OneOfLowestNOrDefault(sel, n), key));
                }
            }

            foreach (var n in ns)
            {
                SeedStatic(seed);
                var before = StaticState();
                var query = source.TakeRandomN(n);
                var drewOnCall = StaticState() != before;
                var first = Enc.Try(() => query.ToList());
                var second = Enc.Try(() => query.ToList());
                takeRandomN.Add(new JsonObject
                {
                    ["seed"] = seed, ["size"] = size, ["n"] = n,
                    ["drew_on_call"] = drewOnCall,
                    ["first"] = first, ["second"] = second,
                    ["next"] = StaticRandom.Next(1000)
                });
            }

            // Deck: shuffle, then draw everything plus one more (throws), then Top.
            {
                var r = new LoggedRandom(seed);
                var d = new Deck<int>(source, r);
                d.Shuffle();
                var drawn = new JsonArray();
                for (var i = 0; i <= size; i++) drawn.Add(Enc.Try(() => d.Draw()));
                deck.Add(new JsonObject
                {
                    ["seed"] = seed, ["size"] = size,
                    ["drawn"] = drawn,
                    ["top"] = Enc.Try(() => d.Top),
                    ["count"] = r.GetCount(),
                    ["next"] = r.Next(1000)
                });
            }
        }

        return new JsonObject
        {
            ["strings"] = new JsonArray(Strings.Select(s => (JsonNode?)s).ToArray()),
            ["seeded"] = seeded,
            ["unseeded"] = unseeded,
            ["take_random_n"] = takeRandomN,
            ["deck"] = deck
        };
    }

    private static JsonObject Entry(int seed, int size, string helper, string key, int n, JsonNode? result, int calls) => new()
    {
        ["helper"] = helper, ["key"] = key, ["seed"] = seed, ["size"] = size, ["n"] = n,
        ["result"] = result, ["calls"] = calls
    };

    private static JsonObject Seeded(int seed, int size, string helper, string key, int n,
        Func<LoggedRandom, Func<int, IComparable>, object?> run, Func<int, IComparable> keyFn)
    {
        var r = new LoggedRandom(seed);
        var calls = 0;
        var result = Enc.Try(() => run(r, x => { calls++; return keyFn(x); }));
        var e = Entry(seed, size, helper, key, n, result, calls);
        e["count"] = r.GetCount();
        e["next"] = r.Next(1000);
        return e;
    }

    private static JsonObject Unseeded(int seed, int size, string helper, string key, int n,
        Func<Func<int, IComparable>, object?> run, Func<int, IComparable> keyFn)
    {
        SeedStatic(seed);
        var calls = 0;
        var result = Enc.Try(() => run(x => { calls++; return keyFn(x); }));
        var e = Entry(seed, size, helper, key, n, result, calls);
        e["next"] = StaticRandom.Next(1000);
        return e;
    }
}
