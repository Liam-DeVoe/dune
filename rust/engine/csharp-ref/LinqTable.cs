// linq.json: the .NET behaviour that rust/engine/src/cs.rs stands in for.
// Every C# operator with a cs.rs helper is run on small inputs (empty, one
// element, one match, two matches, no match), from a List<int> and from a
// lazy sequence, and the result or exception text is recorded together with
// how often the predicate ran. tests/linq_table.rs fails on any operator here
// that has no Rust counterpart.

using System.Text.Json.Nodes;
using Treachery.Shared;

public static class LinqTable
{
    private static readonly int[][] Inputs = [[], [1], [2], [1, 1], [1, 2], [2, 1, 3, 1], [3, 2, 3]];

    private static readonly (string Name, Func<IEnumerable<int>, Func<int, bool>, object?> Run)[] Operators =
    [
        ("First()", (s, _) => s.First()),
        ("First(pred)", (s, p) => s.First(p)),
        ("FirstOrDefault()", (s, _) => s.FirstOrDefault()),
        ("FirstOrDefault(pred)", (s, p) => s.FirstOrDefault(p)),
        ("Last()", (s, _) => s.Last()),
        ("Last(pred)", (s, p) => s.Last(p)),
        ("LastOrDefault()", (s, _) => s.LastOrDefault()),
        ("LastOrDefault(pred)", (s, p) => s.LastOrDefault(p)),
        ("Single()", (s, _) => s.Single()),
        ("Single(pred)", (s, p) => s.Single(p)),
        ("SingleOrDefault()", (s, _) => s.SingleOrDefault()),
        ("SingleOrDefault(pred)", (s, p) => s.SingleOrDefault(p)),
        ("Any(pred)", (s, p) => s.Any(p)),
        ("All(pred)", (s, p) => s.All(p)),
        ("Count(pred)", (s, p) => s.Count(p)),
        ("Max()", (s, _) => s.Max()),
        ("Min()", (s, _) => s.Min()),
        ("Distinct()", (s, _) => s.Distinct().ToList()),
        ("ElementAt(1)", (s, _) => s.ElementAt(1)),
        ("ElementAt(-1)", (s, _) => s.ElementAt(-1)),
        ("ElementAtOrDefault(1)", (s, _) => s.ElementAtOrDefault(1)),
    ];

    // A sequence with no IList or Iterator shortcuts. (`Where` is not enough:
    // .NET turns `Where` over an empty array into an empty array, which then
    // takes the IList paths.)
    private static IEnumerable<int> Lazy(int[] items)
    {
        foreach (var x in items) yield return x;
    }

    public static JsonObject Dump()
    {
        var ops = new JsonArray();
        foreach (var (name, run) in Operators)
        foreach (var shape in new[] { "list", "lazy" })
        foreach (var input in Inputs)
        {
            var calls = 0;
            Func<int, bool> pred = x => { calls++; return x == 1; };
            IEnumerable<int> source = shape == "list" ? input.ToList() : Lazy(input);
            var result = Enc.Try(() => run(source, pred));
            ops.Add(new JsonObject
            {
                ["op"] = name, ["shape"] = shape, ["input"] = new JsonArray(input.Select(i => (JsonNode?)i).ToArray()),
                ["result"] = result, ["calls"] = calls
            });
        }

        // Indexers. List<T> and arrays throw different exceptions.
        var indexers = new JsonArray();
        foreach (var input in new[] { new int[0], new[] { 5 }, new[] { 5, 6 } })
        foreach (var i in new[] { int.MinValue, -1, 0, 1, 2, int.MaxValue })
        {
            var list = input.ToList();
            indexers.Add(new JsonObject
            {
                ["input"] = new JsonArray(input.Select(x => (JsonNode?)x).ToArray()), ["index"] = i,
                ["list"] = Enc.Try(() => list[i]),
                ["array"] = Enc.Try(() => input[i]),
                ["list_set"] = Enc.Try(() => { list[i] = 9; return list; }),
                ["array_set"] = Enc.Try(() => { var a = input.ToArray(); a[i] = 9; return a; })
            });
        }

        // int arithmetic
        var arithmetic = new JsonArray();
        foreach (var a in new[] { 7, -7, 0, int.MinValue, int.MaxValue })
        foreach (var b in new[] { 2, -2, 0, 1, -1 })
        {
            arithmetic.Add(new JsonObject
            {
                ["a"] = a, ["b"] = b,
                ["div"] = Enc.Try(() => a / b),
                ["rem"] = Enc.Try(() => a % b)
            });
        }

        // Dictionary indexer misses: the message embeds key.ToString().
        var dictionary = new JsonArray
        {
            new JsonObject { ["key"] = "int 5", ["result"] = Enc.Try(() => new Dictionary<int, int>()[5]) },
            new JsonObject { ["key"] = "int -1", ["result"] = Enc.Try(() => new Dictionary<int, int>()[-1]) },
            new JsonObject { ["key"] = "Faction.Atreides", ["result"] = Enc.Try(() => new Dictionary<Faction, int>()[Faction.Atreides]) },
            new JsonObject { ["key"] = "Faction 13", ["result"] = Enc.Try(() => new Dictionary<Faction, int>()[(Faction)13]) },
            new JsonObject { ["key"] = "string abc", ["result"] = Enc.Try(() => new Dictionary<string, int>()["abc"]) }
        };

        // Exception messages: parameterless constructors give a default text.
        var exceptions = new JsonArray();
        void Ex(string what, Func<Exception> make) => exceptions.Add(new JsonObject { ["what"] = what, ["result"] = JsonValue.Create(Enc.Exception(make())) });
        Ex("Exception()", () => new Exception());
        Ex("InvalidOperationException()", () => new InvalidOperationException());
        Ex("ArgumentException()", () => new ArgumentException());
        Ex("ArgumentOutOfRangeException()", () => new ArgumentOutOfRangeException());
        Ex("ArgumentNullException()", () => new ArgumentNullException());
        Ex("NullReferenceException()", () => new NullReferenceException());
        Ex("KeyNotFoundException()", () => new KeyNotFoundException());
        Ex("IndexOutOfRangeException()", () => new IndexOutOfRangeException());
        Ex("DivideByZeroException()", () => new DivideByZeroException());
        Ex("OverflowException()", () => new OverflowException());
        Ex("FormatException()", () => new FormatException());
        Ex("NotImplementedException()", () => new NotImplementedException());
        Ex("NotSupportedException()", () => new NotSupportedException());
        Ex("ArgumentOutOfRangeException(param)", () => new ArgumentOutOfRangeException("faction"));
        Ex("ArgumentOutOfRangeException(param, Faction 13, null)", () => new ArgumentOutOfRangeException("faction", (Faction)13, null));
        Ex("ArgumentOutOfRangeException(param, Faction.Atreides, null)", () => new ArgumentOutOfRangeException("faction", Faction.Atreides, null));
        Ex("ArgumentOutOfRangeException(param, int -5, msg)", () => new ArgumentOutOfRangeException("n", -5, "must be positive"));
        Ex("ArgumentOutOfRangeException(param, msg)", () => new ArgumentOutOfRangeException("n", "must be positive"));
        Ex("ArgumentNullException(param)", () => new ArgumentNullException("source"));
        Ex("ArgumentException(msg, param)", () => new ArgumentException("bad", "x"));
        Ex("ArgumentException(msg)", () => new ArgumentException("bad"));
        Ex("Exception(\"\")", () => new Exception(""));

        return new JsonObject
        {
            ["ops"] = ops,
            ["indexers"] = indexers,
            ["arithmetic"] = arithmetic,
            ["dictionary"] = dictionary,
            ["exceptions"] = exceptions
        };
    }
}
