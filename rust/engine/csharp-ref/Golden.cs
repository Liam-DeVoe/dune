// Shared state for the golden generators: the ported type list, sample
// instances used as arguments and receivers, and the writer.

using System.Text.Json;
using System.Text.Json.Nodes;
using Treachery.Shared;

public static class Golden
{
    public static readonly Map SampleMap = new();

    public static Type FindType(string name) =>
        typeof(Map).Assembly.GetType("Treachery.Shared." + name.Replace('.', '+'))
        ?? throw new Exception($"ported_types.txt names unknown type {name}");

    // tests/coverage/ported_types.txt: one C# type name per line ('#' starts a comment).
    public static List<string> ReadPortedTypes(string path) =>
        File.ReadAllLines(path)
            .Select(l => (l.Contains('#') ? l[..l.IndexOf('#')] : l).Trim())
            .Where(l => l.Length > 0)
            .ToList();

    public static void Write(string path, JsonNode node)
    {
        File.WriteAllText(path, node.ToJsonString(new JsonSerializerOptions { WriteIndented = false }) + "\n");
        Console.WriteLine($"wrote {path}");
    }

    private static List<Location> Locations => SampleMap.Locations(true).ToList();

    private static List<TreacheryCard> CardReps =>
        TreacheryCardManager.Items.GroupBy(c => c.Type).Select(g => g.First()).ToList();

    // Instances passed as arguments of type t, or null if t is unsupported.
    public static IEnumerable<object>? ArgSamples(Type t)
    {
        if (typeof(Location).IsAssignableFrom(t))
        {
            var m = SampleMap;
            var picks = Locations.GroupBy(l => l.GetType()).Select(g => g.First())
                .Concat([m.Arrakeen, m.PolarSink, m.HiddenMobileStronghold, m.Shrine, m.Homeworlds.Last(), Locations[42]]);
            return picks.Where(t.IsInstanceOfType).Distinct().ToList();
        }
        if (t == typeof(Territory))
            return SampleMap.Territories(true).Take(3).Concat([SampleMap.ShieldWall, SampleMap.Homeworlds.First().Territory]).Distinct().ToList();
        if (t == typeof(Leader))
            return [LeaderManager.Leaders[0], LeaderManager.Leaders[35], LeaderManager.Leaders[56], LeaderManager.Leaders.Last()];
        if (t == typeof(IHero))
            return [LeaderManager.Leaders[0], LeaderManager.Leaders[56], LeaderManager.Messiah, TreacheryCardManager.Items[0], TreacheryCardManager.Items[17]];
        if (t == typeof(TreacheryCard))
            return CardReps.Take(12).ToList();
        if (t == typeof(ResourceCard))
        {
            var rc = Enc.ResourceCards;
            return new[] { rc.First(), rc.First(c => c.IsShaiHulud), rc.First(c => c.IsDiscovery), rc.Last() }.Distinct().ToList();
        }
        if (t == typeof(PointD))
            return new object[] { new PointD(0, 0), new PointD(1.5, -2), new PointD(double.NaN, 0) };
        return null;
    }

    // Receivers for instance members declared on t: (label, factory). The
    // label is how the Rust side rebuilds the receiver; factories build a
    // fresh object per call where the type is mutable.
    public static List<(JsonNode? Label, Func<object?> Make)>? Receivers(Type t)
    {
        // Locations and territories belong to a map and have mutators
        // (Territory.AddLocation), so each call gets them from a fresh Map.
        if (typeof(Location).IsAssignableFrom(t))
            return Locations.Where(t.IsInstanceOfType)
                .Select(l => (Enc.Value(l), (Func<object?>)(() => new Map().Locations(true).First(x => x.Id == l.Id)))).ToList();
        if (t == typeof(Territory))
            return SampleMap.Territories(true)
                .Select(x => (Enc.Value(x), (Func<object?>)(() => new Map().Territories(true).First(y => y.Id == x.Id)))).ToList();

        IEnumerable<object>? all = null;
        if (t == typeof(Leader)) all = LeaderManager.Leaders;
        else if (t == typeof(Messiah)) all = [LeaderManager.Messiah];
        else if (t == typeof(TreacheryCard)) all = TreacheryCardManager.Items;
        else if (t == typeof(IHero) || t == typeof(IIdentifiable))
            all = LeaderManager.Leaders.Cast<object>().Append(LeaderManager.Messiah).Concat(TreacheryCardManager.Items);
        else if (t == typeof(ResourceCard)) all = Enc.ResourceCards;
        if (all != null) return all.Select(o => (Enc.Value(o), (Func<object?>)(() => o))).ToList();

        (string, Func<object?>)[]? made = t.Name switch
        {
            "Map" => [("new:Map", () => new Map())],
            "LocationFetcher" => [("new:Map", () => new Map().LocationLookup)],
            "TerritoryFetcher" => [("new:Map", () => new Map().TerritoryLookup)],
            "LeaderFetcher" => [("new:LeaderFetcher", () => new LeaderManager.LeaderFetcher())],
            "HeroFetcher" => [("new:HeroFetcher", () => new LeaderManager.HeroFetcher())],
            "Payment" => [("new:Payment()", () => new Payment()), ("new:Payment(3)", () => new Payment(3)), ("new:Payment(5,Atreides)", () => new Payment(5, Faction.Atreides))],
            "PointD" => [("pt:0,0", () => new PointD(0, 0)), ("pt:1.5,-2", () => new PointD(1.5, -2))],
            "LoggedRandom" => [("new:LoggedRandom(1)", () => new LoggedRandom(1)), ("new:LoggedRandom(42)", () => new LoggedRandom(42))],
            "Report" => [("new:Report(Bidding,3)", () => new Report(MainPhase.Bidding, 3))],
            "Message" => [("new:Message(x)", () => Message.Express("x"))],
            "MessagePart" => [("new:MessagePart(x)", () => MessagePart.Express("x"))],
            "Expression" => [("new:Expression(x)", () => new Expression("x"))],
            _ => null
        };
        return made?.Select(p => ((JsonNode?)JsonValue.Create(p.Item1), p.Item2)).ToList();
    }
}
