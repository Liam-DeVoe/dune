// Reference oracle for the dotnet-compat crate.
// Reads a JSON array of cases on stdin, writes a JSON array of results (one per
// case) on stdout. Doubles travel as their IEEE bit patterns (long), floats as
// int bit patterns, so no formatting is involved. See ../tests/ for the format.

using System.Globalization;
using System.Text;
using System.Text.Json;

CultureInfo.CurrentCulture = CultureInfo.InvariantCulture;
CultureInfo.CurrentUICulture = CultureInfo.InvariantCulture;

var input = Console.In.ReadToEnd();
using var doc = JsonDocument.Parse(input, new JsonDocumentOptions { MaxDepth = 256 });
var stdout = Console.OpenStandardOutput();
using var w = new Utf8JsonWriter(stdout);
w.WriteStartArray();
foreach (var c in doc.RootElement.EnumerateArray())
{
    var t = c.GetProperty("t").GetString();
    switch (t)
    {
        case "random": Cases.Random(c, w); break;
        case "dict": Cases.Dict(c, w); break;
        case "set": Cases.Set(c, w); break;
        case "sort": Cases.Sort(c, w); break;
        case "linq": Cases.Linq(c, w); break;
        case "math": Cases.MathCase(c, w); break;
        default: throw new Exception("unknown case " + t);
    }
}
w.WriteEndArray();
w.Flush();

record struct Item(int Key, int Id);

// Not [Flags]; Enum.Parse still ORs comma-separated names.
enum TestEnum { None = 0, Atreides = 1, Harkonnen = 2, Emperor = 4, Fremen = 8, Guild = 16, fremen = 32, Negative = -5 }

struct CItem : IComparable<CItem>
{
    public int Key; public int Id;
    public int CompareTo(CItem other) => Key.CompareTo(other.Key);
}

class KeyComparer : IComparer<Item>
{
    public int Compare(Item a, Item b) => a.Key.CompareTo(b.Key);
}

static class Cases
{
    static int[] Ints(JsonElement e) => e.EnumerateArray().Select(x => x.GetInt32()).ToArray();
    static long[] Longs(JsonElement e) => e.EnumerateArray().Select(x => x.GetInt64()).ToArray();
    static double D(long bits) => BitConverter.Int64BitsToDouble(bits);
    static long B(double d) => BitConverter.DoubleToInt64Bits(d);
    static float F(int bits) => BitConverter.Int32BitsToSingle(bits);
    static int BF(float f) => BitConverter.SingleToInt32Bits(f);

    public static void Random(JsonElement c, Utf8JsonWriter w)
    {
        var r = new Random(c.GetProperty("seed").GetInt32());
        w.WriteStartArray();
        foreach (var op in c.GetProperty("ops").EnumerateArray())
        {
            var o = Ints(op);
            try
            {
                switch (o[0])
                {
                    case 0: w.WriteNumberValue(r.Next()); break;
                    case 1: w.WriteNumberValue(r.Next(o[1])); break;
                    case 2: w.WriteNumberValue(r.Next(o[1], o[2])); break;
                    case 3: w.WriteNumberValue(B(r.NextDouble())); break;
                }
            }
            catch (ArgumentOutOfRangeException ex) { w.WriteStringValue(Msg(ex)); }
        }
        w.WriteEndArray();
    }

    static void Snap(Dictionary<int, int> d, Utf8JsonWriter w)
    {
        w.WriteStartArray();
        foreach (var kv in d) { w.WriteNumberValue(kv.Key); w.WriteNumberValue(kv.Value); }
        w.WriteEndArray();
    }

    public static void Dict(JsonElement c, Utf8JsonWriter w)
    {
        var d = new Dictionary<int, int>();
        w.WriteStartArray();
        foreach (var op in c.GetProperty("ops").EnumerateArray())
        {
            var o = Ints(op);
            int k = o[1], v = o[2];
            switch (o[0])
            {
                case 0: d[k] = v; break;
                case 1:
                    try { d.Add(k, v); w.WriteNumberValue(1); }
                    catch (ArgumentException ex) { w.WriteStringValue(Msg(ex)); }
                    break;
                case 2: w.WriteNumberValue(d.TryAdd(k, v) ? 1 : 0); break;
                case 3: w.WriteNumberValue(d.Remove(k) ? 1 : 0); break;
                case 4: d.Clear(); break;
                case 5: Snap(d, w); break;
                case 6: d = new Dictionary<int, int>(d); break;
                case 7: d = d.ToDictionary(kv => kv.Key, kv => kv.Value); break;
                case 8: w.WriteNumberValue(d.ContainsValue(v) ? 1 : 0); break;
                case 9: w.WriteNumberValue(d.TryGetValue(k, out var x) ? x : -1); break;
                case 10: w.WriteNumberValue(d.GetValueOrDefault(k)); break;
                case 11:
                    try { w.WriteNumberValue(d[k]); }
                    catch (KeyNotFoundException ex) { w.WriteStringValue(Msg(ex)); }
                    break;
            }
        }
        Snap(d, w);
        w.WriteEndArray();
    }

    static void SnapSet(HashSet<int> s, Utf8JsonWriter w)
    {
        w.WriteStartArray();
        foreach (var x in s) w.WriteNumberValue(x);
        w.WriteEndArray();
    }

    public static void Set(JsonElement c, Utf8JsonWriter w)
    {
        var s = new HashSet<int>();
        w.WriteStartArray();
        foreach (var op in c.GetProperty("ops").EnumerateArray())
        {
            int code = op[0].GetInt32();
            int k = op[1].GetInt32();
            var xs = Ints(op[2]).ToList();
            switch (code)
            {
                case 0: w.WriteNumberValue(s.Add(k) ? 1 : 0); break;
                case 1: w.WriteNumberValue(s.Remove(k) ? 1 : 0); break;
                case 2: s.Clear(); break;
                case 3: SnapSet(s, w); break;
                case 4: s = new HashSet<int>(s); break;
                case 5: s = new HashSet<int>(xs); break;
                case 6: s = xs.Where(_ => true).ToHashSet(); break;
                case 7: s.UnionWith(xs); break;
                case 8: s.ExceptWith(xs); break;
                case 9: s.IntersectWith(xs); break;
                case 10: s.IntersectWith(xs.Where(_ => true)); break;
                case 11: s.IntersectWith(new HashSet<int>(xs)); break;
                case 12: s.SymmetricExceptWith(xs); break;
                case 13: s.SymmetricExceptWith(new HashSet<int>(xs)); break;
                case 14: w.WriteNumberValue(s.RemoveWhere(x => x % k == 0)); break;
                case 15: s.TrimExcess(); break;
                case 16: s = new HashSet<int>(k); break;
                case 17: w.WriteNumberValue(s.Contains(k) ? 1 : 0); break;
            }
        }
        SnapSet(s, w);
        w.WriteEndArray();
    }

    public static void Sort(JsonElement c, Utf8JsonWriter w)
    {
        var keys = Ints(c.GetProperty("keys"));
        int mode = c.GetProperty("mode").GetInt32();
        int[] ids;
        switch (mode)
        {
            case 0:
            {
                var l = keys.Select((k, i) => new Item(k, i)).ToList();
                l.Sort((a, b) => a.Key.CompareTo(b.Key));
                ids = l.Select(x => x.Id).ToArray();
                break;
            }
            case 1:
            {
                var l = keys.Select((k, i) => new Item(k, i)).ToList();
                l.Sort(new KeyComparer());
                ids = l.Select(x => x.Id).ToArray();
                break;
            }
            case 2:
            {
                var l = keys.Select((k, i) => new CItem { Key = k, Id = i }).ToList();
                l.Sort();
                ids = l.Select(x => x.Id).ToArray();
                break;
            }
            case 3:
            {
                var a = keys.Select((k, i) => new Item(k, i)).ToArray();
                Array.Sort(a, (x, y) => x.Key.CompareTo(y.Key));
                ids = a.Select(x => x.Id).ToArray();
                break;
            }
            case 5:
            case 6:
            {
                // a bogus comparison (always "less") or one that throws
                var l = keys.Select((k, i) => new Item(k, i)).ToList();
                try
                {
                    if (mode == 5) l.Sort((a, b) => -1);
                    else l.Sort((a, b) => a.Key == 7 || b.Key == 7 ? throw new InvalidOperationException("seven") : a.Key.CompareTo(b.Key));
                }
                catch (Exception ex)
                {
                    w.WriteStartArray();
                    w.WriteStringValue(Msg(ex));
                    foreach (var x in l) w.WriteNumberValue(x.Id);
                    w.WriteEndArray();
                    return;
                }
                ids = l.Select(x => x.Id).ToArray();
                break;
            }
            default:
            {
                // descending comparison
                var l = keys.Select((k, i) => new Item(k, i)).ToList();
                l.Sort((a, b) => b.Key.CompareTo(a.Key));
                ids = l.Select(x => x.Id).ToArray();
                break;
            }
        }
        w.WriteStartArray();
        foreach (var i in ids) w.WriteNumberValue(i);
        w.WriteEndArray();
    }

    static void WriteInts(Utf8JsonWriter w, IEnumerable<int> xs)
    {
        w.WriteStartArray();
        foreach (var x in xs) w.WriteNumberValue(x);
        w.WriteEndArray();
    }

    // A sequence that is not an IList (Where over an empty array is one).
    static IEnumerable<int> Lazy(int[] xs) { foreach (var x in xs) yield return x; }

    public static string Msg(Exception ex) => ex.GetType().Name + ": " + ex.Message;

    static void Try(Utf8JsonWriter w, Action a)
    {
        try { a(); }
        catch (Exception ex) { w.WriteStringValue(Msg(ex)); }
    }

    public static void Linq(JsonElement c, Utf8JsonWriter w)
    {
        var op = c.GetProperty("op").GetString();
        var xs = Ints(c.GetProperty("xs"));
        var ys = Ints(c.GetProperty("ys"));
        int k = c.GetProperty("k").GetInt32();
        var idx = Enumerable.Range(0, xs.Length);
        switch (op)
        {
            case "orderby": WriteInts(w, idx.OrderBy(i => xs[i])); break;
            case "orderbydesc": WriteInts(w, idx.OrderByDescending(i => xs[i])); break;
            case "orderbytake": WriteInts(w, idx.OrderBy(i => xs[i]).Take(k)); break;
            case "orderbydesctake": WriteInts(w, idx.OrderByDescending(i => xs[i]).Take(k)); break;
            case "orderbythenbydesc": WriteInts(w, idx.OrderBy(i => xs[i] % 3).ThenByDescending(i => xs[i])); break;
            case "orderbyfirstpred": w.WriteNumberValue(idx.OrderBy(i => xs[i]).FirstOrDefault(i => i % 3 == 0, -1)); break;
            case "orderbydescfirstpred": w.WriteNumberValue(idx.OrderByDescending(i => xs[i]).FirstOrDefault(i => i % 3 == 0, -1)); break;
            case "orderbylast": Try(w, () => w.WriteNumberValue(idx.OrderBy(i => xs[i]).Last())); break;
            case "orderbyrandom":
            {
                var r = new Random(k);
                WriteInts(w, idx.OrderBy(_ => r.Next(5)).Take(xs.Length / 2));
                break;
            }
            case "maxby": Try(w, () => w.WriteNumberValue(idx.MaxBy(i => xs[i]))); break;
            case "minby": Try(w, () => w.WriteNumberValue(idx.MinBy(i => xs[i]))); break;
            case "max": Try(w, () => w.WriteNumberValue(xs.Max())); break;
            case "min": Try(w, () => w.WriteNumberValue(xs.Min())); break;
            case "maxsel": Try(w, () => w.WriteNumberValue(idx.Max(i => xs[i]))); break;
            case "maxnullable":
            {
                var v = xs.Select(x => x < 0 ? (int?)null : x).Max();
                if (v.HasValue) w.WriteNumberValue(v.Value); else w.WriteNullValue();
                break;
            }
            case "minnullable":
            {
                var v = xs.Select(x => x < 0 ? (int?)null : x).Min();
                if (v.HasValue) w.WriteNumberValue(v.Value); else w.WriteNullValue();
                break;
            }
            case "maxicomparable":
            {
                // the engine's HighestOrDefault pattern: Max over a Func<T, IComparable> selector
                var v = idx.Max(i => (IComparable)xs[i]);
                if (v == null) w.WriteNullValue(); else w.WriteNumberValue((int)v);
                break;
            }
            case "sum": Try(w, () => w.WriteNumberValue(xs.Sum())); break;
            case "sumsel": Try(w, () => w.WriteNumberValue(idx.Sum(i => xs[i]))); break;
            case "average": Try(w, () => w.WriteNumberValue(B(xs.Average()))); break;
            case "sumf": w.WriteNumberValue(BF(xs.Select(F).Sum())); break;
            case "averagef": Try(w, () => w.WriteNumberValue(BF(xs.Select(F).Average()))); break;
            case "maxf": Try(w, () => w.WriteNumberValue(BF(xs.Select(F).Max()))); break;
            case "minf": Try(w, () => w.WriteNumberValue(BF(xs.Select(F).Min()))); break;
            case "maxd": Try(w, () => w.WriteNumberValue(B(xs.Select(x => (double)F(x)).Max()))); break;
            case "mind": Try(w, () => w.WriteNumberValue(B(xs.Select(x => (double)F(x)).Min()))); break;
            case "orderbyf": WriteInts(w, idx.OrderBy(i => F(xs[i]))); break;
            case "orderbydescf": WriteInts(w, idx.OrderByDescending(i => F(xs[i]))); break;
            case "distinct": WriteInts(w, xs.Distinct()); break;
            case "union": WriteInts(w, xs.Union(ys)); break;
            case "intersect": WriteInts(w, xs.Intersect(ys)); break;
            case "except": WriteInts(w, xs.Except(ys)); break;
            case "groupby":
                w.WriteStartArray();
                foreach (var g in idx.GroupBy(i => xs[i]))
                {
                    w.WriteStartArray();
                    w.WriteNumberValue(g.Key);
                    foreach (var i in g) w.WriteNumberValue(i);
                    w.WriteEndArray();
                }
                w.WriteEndArray();
                break;
            case "single": Try(w, () => w.WriteNumberValue(xs.Single())); break;
            case "singleordefault": Try(w, () => w.WriteNumberValue(xs.Select(x => (int?)x).SingleOrDefault() ?? -1)); break;
            case "singlepred": Try(w, () => w.WriteNumberValue(xs.Select(x => (int?)x).SingleOrDefault(x => x % 2 == 0) ?? -1)); break;
            case "reverse": WriteInts(w, xs.Reverse()); break;
            case "first": Try(w, () => w.WriteNumberValue(xs.Where(_ => true).First())); break;
            case "firstpred": Try(w, () => w.WriteNumberValue(xs.First(x => x % 2 == 0))); break;
            case "lastpred": Try(w, () => w.WriteNumberValue(xs.Last(x => x % 2 == 0))); break;
            case "singlepred2": Try(w, () => w.WriteNumberValue(xs.Single(x => x % 2 == 0))); break;
            case "elementat": Try(w, () => w.WriteNumberValue(Lazy(xs).ElementAt(k))); break;
            case "elementatlist": Try(w, () => w.WriteNumberValue(xs.ToList().ElementAt(k))); break;
            case "todict": Try(w, () => WriteInts(w, xs.ToDictionary(x => x, x => x).Keys)); break;
            case "aggregate": Try(w, () => w.WriteNumberValue(xs.Aggregate((a, b) => a * 31 + b))); break;
            default: throw new Exception("unknown linq op " + op);
        }
    }

    public static void MathCase(JsonElement c, Utf8JsonWriter w)
    {
        var op = c.GetProperty("op").GetString();
        var arr = c.GetProperty("xs");
        w.WriteStartArray();
        switch (op)
        {
            case "round":
                foreach (var bits in Longs(arr))
                {
                    var x = D(bits);
                    w.WriteStartArray();
                    w.WriteNumberValue(B(Math.Round(x)));
                    foreach (var m in new[] { MidpointRounding.ToEven, MidpointRounding.AwayFromZero, MidpointRounding.ToZero, MidpointRounding.ToNegativeInfinity, MidpointRounding.ToPositiveInfinity })
                    {
                        w.WriteNumberValue(B(Math.Round(x, m)));
                        foreach (var digits in new[] { 0, 1, 2, 5, 15 }) w.WriteNumberValue(B(Math.Round(x, digits, m)));
                    }
                    w.WriteNumberValue(B(Math.Round(x, 1)));
                    w.WriteNumberValue(BF(MathF.Round((float)x)));
                    w.WriteNumberValue(B(Math.Ceiling(x)));
                    w.WriteNumberValue(B(Math.Floor(x)));
                    w.WriteNumberValue((int)x);
                    w.WriteNumberValue((int)(float)x);
                    w.WriteNumberValue((long)x);
                    w.WriteEndArray();
                }
                break;
            case "fmtd":
                foreach (var bits in Longs(arr))
                {
                    var x = D(bits);
                    w.WriteStartArray();
                    w.WriteStringValue(x.ToString(CultureInfo.InvariantCulture));
                    w.WriteStringValue($"{x}");
                    w.WriteEndArray();
                }
                break;
            case "fmtf":
                foreach (var bits in Longs(arr))
                {
                    var x = F((int)bits);
                    w.WriteStartArray();
                    w.WriteStringValue(x.ToString(CultureInfo.InvariantCulture));
                    w.WriteStringValue($"{x}");
                    w.WriteEndArray();
                }
                break;
            case "parse":
                foreach (var e in arr.EnumerateArray())
                {
                    var s = e.ValueKind == JsonValueKind.Null ? null : e.GetString();
                    w.WriteStartArray();
                    try { w.WriteNumberValue(int.Parse(s)); }
                    catch (Exception ex) { w.WriteStringValue(ex.GetType().Name + ": " + ex.Message); }
                    try { w.WriteNumberValue(Convert.ToInt32(s)); }
                    catch (Exception ex) { w.WriteStringValue(ex.GetType().Name + ": " + ex.Message); }
                    try { w.WriteNumberValue(BF(float.Parse(s, CultureInfo.InvariantCulture))); }
                    catch (Exception ex) { w.WriteStringValue(ex.GetType().Name + ": " + ex.Message); }
                    try { w.WriteNumberValue(B(double.Parse(s, CultureInfo.InvariantCulture))); }
                    catch (Exception ex) { w.WriteStringValue(ex.GetType().Name + ": " + ex.Message); }
                    try { w.WriteNumberValue((int)Enum.Parse<TestEnum>(s)); }
                    catch (Exception ex) { w.WriteStringValue(ex.GetType().Name + ": " + ex.Message); }
                    w.WriteEndArray();
                }
                break;
            case "cmpd":
            {
                var xs = Longs(arr);
                for (int i = 0; i + 1 < xs.Length; i += 2)
                {
                    double a = D(xs[i]), b = D(xs[i + 1]);
                    w.WriteStartArray();
                    w.WriteNumberValue(a.CompareTo(b));
                    w.WriteBooleanValue(a.Equals(b));
                    w.WriteNumberValue(B(Math.Max(a, b)));
                    w.WriteNumberValue(B(Math.Min(a, b)));
                    w.WriteNumberValue(BF(Math.Max((float)a, (float)b)));
                    w.WriteNumberValue(BF(Math.Min((float)a, (float)b)));
                    w.WriteEndArray();
                }
                break;
            }
            case "abs":
                foreach (var x in Ints(arr))
                {
                    try { w.WriteNumberValue(Math.Abs(x)); }
                    catch (OverflowException ex) { w.WriteStringValue(Msg(ex)); }
                }
                break;
            case "rounddigits":
                foreach (var d in Ints(arr))
                {
                    try { w.WriteNumberValue(B(Math.Round(1.25, d))); }
                    catch (Exception ex) { w.WriteStringValue(Msg(ex)); }
                    try { w.WriteNumberValue(B(Math.Round(1.25, d, MidpointRounding.AwayFromZero))); }
                    catch (Exception ex) { w.WriteStringValue(Msg(ex)); }
                }
                break;
            default: throw new Exception("unknown math op " + op);
        }
        w.WriteEndArray();
    }
}
