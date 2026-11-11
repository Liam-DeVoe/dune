// messages.json: Message.ToString() (no describer, invariant culture) for
// generated values of every kind of expression element, and the element
// lists produced by Message.Express for each way of passing arguments to
// `params object?[]`. tests/message_corpus.rs rebuilds each element and
// compares ExpressionElement::to_cs_string.
//
// Input encoding (see Enc.cs for the shared tags):
//   "fbits:<uint>" / "dbits:<ulong>"   a float / double by bit pattern
//   {"list": "<C# type>", "items": [...]} a collection of that runtime type
//   {"exception": "<type>", "message": "<text>" | null}   new T(message) / new T()
//   {"message": [...]} / {"part": [...]}  a nested Message / MessagePart
//   "payment:<amount>"                    Payment.Of(amount)

using System.Globalization;
using System.Text.Json.Nodes;
using Treachery.Shared;

public static class MessageCorpus
{
    private static readonly List<JsonObject> Entries = [];

    private static void Add(JsonNode input, object? element)
    {
        Entries.Add(new JsonObject
        {
            ["input"] = input,
            ["expected"] = Enc.Try(() => Message.Express(new object?[] { element }).ToString())
        });
    }

    public static JsonObject Dump()
    {
        var saved = CultureInfo.CurrentCulture;
        CultureInfo.CurrentCulture = CultureInfo.InvariantCulture;
        try
        {
            Entries.Clear();
            Numbers();
            Enums();
            Objects();
            Collections();
            Exceptions();
            Nested();
            return new JsonObject
            {
                ["elements"] = new JsonArray(Entries.Select(e => (JsonNode?)e).ToArray()),
                ["params"] = Params()
            };
        }
        finally
        {
            CultureInfo.CurrentCulture = saved;
        }
    }

    private static void Numbers()
    {
        var floats = new List<float> { 0f, -0f, 1f, -1f, 0.1f, 1f / 3f, 2.5f, 1e-5f, 1e-4f, 1e7f, 1e8f, 1e9f, 1e10f, 123456789f, 1e20f, 1e38f,
            float.MaxValue, float.MinValue, float.Epsilon, float.NaN, float.PositiveInfinity, float.NegativeInfinity, 1.17549435e-38f, 16777216f };
        var doubles = new List<double> { 0d, -0d, 1d, -1d, 0.1d, 1d / 3d, 2.5d, 1e-5, 1e-4, 1e7, 1e15, 1e16, 1e17, 1e20, 1e300,
            double.MaxValue, double.MinValue, double.Epsilon, double.NaN, double.PositiveInfinity, double.NegativeInfinity, 9007199254740993d };
        var rnd = new Random(12345);
        var buffer = new byte[8];
        for (var i = 0; i < 2000; i++)
        {
            rnd.NextBytes(buffer);
            floats.Add(BitConverter.Int32BitsToSingle(BitConverter.ToInt32(buffer, 0)));
            doubles.Add(BitConverter.Int64BitsToDouble(BitConverter.ToInt64(buffer, 0)));
        }
        // Subnormals and small exponents
        for (var i = 1; i < 40; i++)
        {
            floats.Add(BitConverter.Int32BitsToSingle(i * 7919));
            doubles.Add(BitConverter.Int64BitsToDouble(i * 7919L));
        }
        foreach (var f in floats) Add("fbits:" + BitConverter.SingleToUInt32Bits(f), f);
        foreach (var d in doubles) Add("dbits:" + BitConverter.DoubleToUInt64Bits(d), d);

        foreach (var i in Boundary.IntSamples) Add(Enc.Value(i)!, i);
        foreach (var l in new[] { long.MinValue, -1L, 0L, 5000000000L, long.MaxValue }) Add(Enc.Value(l)!, l);
        Add(Enc.Value(true)!, true);
        Add(Enc.Value(false)!, false);
        foreach (var s in new[] { "", "abc", " pays ", "ünïcödé" }) Add(Enc.Value(s)!, s);
        Add(JsonValue.Create("null")!, null);
    }

    private static void Enums()
    {
        foreach (var t in typeof(Map).Assembly.GetTypes().Where(t => t.IsEnum && t.IsPublic).OrderBy(t => t.Name, StringComparer.Ordinal))
        {
            var defined = Enum.GetValues(t).Cast<object>().Select(Convert.ToInt64).Distinct().ToList();
            var values = defined.Concat(new long[] { -1, 0, 13, 999999, int.MinValue, int.MaxValue }).ToList();
            if (t.IsDefined(typeof(FlagsAttribute), false))
                foreach (var a in defined)
                foreach (var b in defined)
                    values.Add(a | b);
            foreach (var v in values.Distinct())
            {
                var e = Enum.ToObject(t, v);
                Add(Enc.Value(e)!, e);
            }
        }
    }

    private static void Objects()
    {
        var m = Golden.SampleMap;
        foreach (var l in m.Locations(true)) Add(Enc.Value(l)!, l);
        foreach (var t in m.Territories(true)) Add(Enc.Value(t)!, t);
        foreach (var l in LeaderManager.Leaders) Add(Enc.Value(l)!, l);
        Add(Enc.Value(LeaderManager.Messiah)!, LeaderManager.Messiah);
        foreach (var c in TreacheryCardManager.Items) Add(Enc.Value(c)!, c);
        foreach (var rc in Enc.ResourceCards) Add(Enc.Value(rc)!, rc);
        Add("payment:3", Payment.Of(3));
    }

    private static JsonObject ListInput(string type, IEnumerable<object?> items) =>
        new() { ["list"] = type, ["items"] = new JsonArray(items.Select(Enc.Value).ToArray()) };

    private static void Collections()
    {
        var m = Golden.SampleMap;
        Add(ListInput("List<Faction>", [Faction.Atreides]), new List<Faction> { Faction.Atreides });
        Add(ListInput("List<Faction>", []), new List<Faction>());
        Add(ListInput("List<int>", [1, 2]), new List<int> { 1, 2 });
        Add(ListInput("List<string>", ["a"]), new List<string> { "a" });
        Add(ListInput("List<Location>", [m.Arrakeen]), new List<Location> { m.Arrakeen });
        Add(ListInput("List<Territory>", [m.ShieldWall]), new List<Territory> { m.ShieldWall });
        Add(ListInput("List<Leader>", [LeaderManager.Leaders[0]]), new List<Leader> { LeaderManager.Leaders[0] });
        Add(ListInput("List<IHero>", [LeaderManager.Messiah]), new List<IHero> { LeaderManager.Messiah });
        Add(ListInput("List<TreacheryCard>", [TreacheryCardManager.Items[0]]), new List<TreacheryCard> { TreacheryCardManager.Items[0] });
        Add(ListInput("List<object>", ["a", 1]), new List<object> { "a", 1 });
        Add(ListInput("Faction[]", [Faction.Atreides]), new[] { Faction.Atreides });
        Add(ListInput("int[]", [1]), new[] { 1 });
        Add(ListInput("Location[]", [m.Arrakeen]), new[] { m.Arrakeen });
        Add(ListInput("TreacheryCard[]", [TreacheryCardManager.Items[0]]), new[] { TreacheryCardManager.Items[0] });
    }

    private static readonly (string Name, Func<string?, Exception> Make)[] ExceptionTypes =
    [
        ("Exception", s => s == null ? new Exception() : new Exception(s)),
        ("InvalidOperationException", s => s == null ? new InvalidOperationException() : new InvalidOperationException(s)),
        ("ArgumentException", s => s == null ? new ArgumentException() : new ArgumentException(s)),
        ("ArgumentOutOfRangeException", s => s == null ? new ArgumentOutOfRangeException() : new ArgumentOutOfRangeException(null, s)),
        ("NullReferenceException", s => s == null ? new NullReferenceException() : new NullReferenceException(s)),
        ("KeyNotFoundException", s => s == null ? new KeyNotFoundException() : new KeyNotFoundException(s)),
        ("IndexOutOfRangeException", s => s == null ? new IndexOutOfRangeException() : new IndexOutOfRangeException(s)),
        ("DivideByZeroException", s => s == null ? new DivideByZeroException() : new DivideByZeroException(s)),
        ("OverflowException", s => s == null ? new OverflowException() : new OverflowException(s)),
        ("FormatException", s => s == null ? new FormatException() : new FormatException(s)),
        ("ArgumentNullException", s => s == null ? new ArgumentNullException() : new ArgumentNullException(null, s)),
    ];

    private static void Exceptions()
    {
        foreach (var (name, make) in ExceptionTypes)
        foreach (var message in new[] { null, "k", "" })
            Add(new JsonObject { ["exception"] = name, ["message"] = message == null ? null : JsonValue.Create(message) }, make(message));
    }

    private static void Nested()
    {
        Add(new JsonObject { ["message"] = new JsonArray("s:a", 1, "e:Faction:20", null, "f:2.5") }, Message.Express("a", 1, Faction.Atreides, null, 2.5f));
        Add(new JsonObject { ["part"] = new JsonArray("s: x ", "o:Location#2") }, MessagePart.Express(" x ", Golden.SampleMap.Locations(true).ElementAt(2)));
        Add(new JsonObject { ["part"] = new JsonArray() }, MessagePart.ExpressIf(false, "never"));
    }

    // How `params object?[]` treats each kind of single argument: the
    // number of elements and their encodings. An argument that converts to
    // object?[] (object[], string[], any reference-type array) IS the
    // parameter array; anything else becomes one element.
    private static JsonArray Params()
    {
        var m = Golden.SampleMap;
        object?[] objects = ["a", 1];
        string[] strings = ["a", "b"];
        Location[] locations = [m.Arrakeen, m.Carthag];
        Faction[] factions = [Faction.Atreides, Faction.Harkonnen];
        int[] ints = [1, 2];
        var list = new List<object> { "a", 1 };
        var cases = new (string Name, Func<Message> Make)[]
        {
            ("object[]", () => Message.Express(objects)),
            ("string[]", () => Message.Express(strings)),
            ("Location[]", () => Message.Express(locations)),
            ("Faction[]", () => Message.Express(factions)),
            ("int[]", () => Message.Express(ints)),
            ("List<object>", () => Message.Express(list)),
            ("object[] and more", () => Message.Express(objects, "x")),
            ("(object)object[]", () => Message.Express((object)objects)),
            ("no arguments", () => Message.Express()),
            ("null object[]", () => Message.Express((object?[])null!)),
            ("null literal", () => Message.Express(null)),
            ("ExpressTo object[]", () => Message.ExpressTo(Faction.Atreides, objects)),
            ("MessagePart object[]", () => Message.Express(MessagePart.Express(objects))),
        };
        var result = new JsonArray();
        foreach (var (name, make) in cases)
        {
            var o = new JsonObject { ["case"] = name };
            try
            {
                var msg = make();
                o["elements"] = msg.Expression.Elements == null ? null : new JsonArray(msg.Expression.Elements.Select(e => e is Array a ? ListInput(e.GetType().Name, a.Cast<object?>()) : Enc.Value(e)).ToArray());
                o["to_string"] = Enc.Try(() => msg.ToString());
            }
            catch (Exception e)
            {
                o["exception"] = Enc.Exception(e);
            }
            result.Add(o);
        }
        return result;
    }
}
