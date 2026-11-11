using System.Collections;
using System.Globalization;
using System.Net;
using System.Reflection;
using System.Text;
using System.Text.RegularExpressions;

// Every message of every report of every scene fixture, one JSON line each:
//
//   fixture, report, line   where the message is (index into Game.Reports and its Messages)
//   target                  Message.Target
//   text                    the parts ExpressionComponent.razor draws, in the view's Part
//                           format (rust/view/src/schema.rs), labels from DefaultSkin
//   heading                 Message.Heading
//   record, route,          Message.Log, Route, Plans and Fact, serialised by reflection:
//   plans, fact             camelCase property names, enums by name, game objects as parts
//   dom                     the text the component shows in the log (LogStyle, Size 20):
//                           the textContent of what it renders
//
// The part and text functions follow the cases of ExpressionComponent's switch in order.
// The game is the one the C# client renders with: the fixture replayed with
// Execute(false, false), as rust/view/src/replica.rs does.
public static class LogLines
{
    private static Skin Skin => DefaultSkin.Default;

    public static void Run(string fixturesDir, StreamWriter w)
    {
        foreach (var file in Directory.GetFiles(fixturesDir, "*.json").OrderBy(f => f, StringComparer.Ordinal))
        {
            var name = Path.GetFileNameWithoutExtension(file);
            var state = GameState.Load(File.ReadAllText(file));
            var game = new Game(state.Version, new Participation());
            var nr = 0;
            foreach (var e in state.Events)
            {
                e.Initialize(game);
                var result = e.Execute(false, false);
                if (result != null) throw new InvalidOperationException($"{name}: event {nr} failed: {result.ToString(Skin)}");
                nr++;
            }

            for (var r = 0; r < game.Reports.Count; r++)
            {
                var line = 0;
                foreach (var m in game.Reports[r].Messages)
                {
                    var o = new JsonObject { ["fixture"] = name, ["report"] = r, ["line"] = line++ };
                    foreach (var (k, v) in Line(m, game)) o[k] = v?.DeepClone();
                    w.WriteLine(o.ToJsonString());
                }
            }
        }
    }

    // The final game of each bot-game trace (accepted events replayed with Execute(false, false),
    // as the Rust tests' replica does), keeping the first two messages of each shape: the record
    // type, the kinds of route, plans and fact, and the kinds of parts. This reaches records and
    // parts the fixtures don't have, with a small golden file.
    public static void RunTraces(IEnumerable<string> traces, StreamWriter w)
    {
        var seen = new Dictionary<string, int>();
        foreach (var path in traces)
        {
            var name = Path.GetFileName(path);
            Stream s = File.OpenRead(path);
            if (path.EndsWith(".gz")) s = new System.IO.Compression.GZipStream(s, System.IO.Compression.CompressionMode.Decompress);
            using var reader = new StreamReader(s);
            Game? game = null;
            while (reader.ReadLine() is { } text)
            {
                if (text.Length == 0) continue;
                var line = JsonNode.Parse(text)!.AsObject();
                var kind = (string)line["kind"]!;
                if (kind == "header") game = new Game((int)line["gameVersion"]!, new Participation());
                else if (kind == "step" && (string?)line["result"] == "accepted")
                {
                    var evt = Utilities.Deserialize<GameEvent>(line["event"]!.ToJsonString())!;
                    evt.Initialize(game!);
                    var result = evt.Execute(false, false);
                    if (result != null) throw new InvalidOperationException($"{name}: {result.ToString(Skin)}");
                }
            }

            for (var r = 0; r < game!.Reports.Count; r++)
            {
                var index = 0;
                foreach (var m in game.Reports[r].Messages)
                {
                    var o = Line(m, game);
                    var shape = string.Join("|",
                        m.Log?.GetType().Name ?? "-", m.Route != null, m.Plans != null, m.Fact?.Kind.ToString() ?? "-",
                        string.Join(",", o["text"]!.AsArray().Select(p => (string)p!["t"]! + (p["ty"] is { } ty ? ":" + (string)ty! : "")).Distinct().Order(StringComparer.Ordinal)));
                    seen[shape] = seen.GetValueOrDefault(shape) + 1;
                    if (seen[shape] <= 2)
                    {
                        var head = new JsonObject { ["trace"] = name, ["report"] = r, ["line"] = index };
                        foreach (var (k, v) in o) head[k] = v?.DeepClone();
                        w.WriteLine(head.ToJsonString());
                    }
                    index++;
                }
            }
        }
    }

    private static JsonObject Line(Message m, Game game) => new()
    {
        ["target"] = m.Target.ToString(),
        ["text"] = Parts(m.Expression, game),
        ["heading"] = m.Heading?.ToString(),
        ["record"] = m.Log == null ? null : Record(m.Log, game, true),
        ["route"] = m.Route == null ? null : Record(m.Route, game, false),
        ["plans"] = m.Plans == null ? null : Record(m.Plans, game, false),
        ["fact"] = m.Fact == null ? null : Record(m.Fact, game, false),
        ["dom"] = Dom(m.Expression, game),
    };

    // ExpressionComponent.DetermineElements.
    private static List<Expression> DetermineElements(IEnumerable e)
    {
        var result = new List<Expression>();
        var useSeparators = false;
        var firstItem = true;
        foreach (var elt in e)
        {
            if (firstItem) useSeparators = ShowWithSeparator(elt);
            result.Add(useSeparators && !firstItem ? new Expression(", ", elt) : new Expression(elt));
            firstItem = false;
        }
        return result;
    }

    private static bool ShowWithSeparator(object e) =>
        !(e is Faction || e is MessagePart || e is FactionForce || e is FactionSpecialForce || e is Leader);

    // Client/Support.Round.
    private static string Round(double x) => Math.Round(x, 3).ToString(CultureInfo.InvariantCulture);

    // ---- Parts ----

    public static JsonArray Parts(Expression e, Game? game)
    {
        var a = new JsonArray();
        foreach (var el in e.Elements) Part(el, game, a);
        return a;
    }

    private static JsonObject P(string t, params (string, JsonNode?)[] members)
    {
        var o = new JsonObject { ["t"] = t };
        foreach (var (k, v) in members) o[k] = v;
        return o;
    }

    private static void Part(object? part, Game? game, JsonArray a)
    {
        switch (part)
        {
            case null: a.Add(P("none")); break;
            case string s: a.Add(P("text", ("v", s))); break;
            case IEnumerable e:
                foreach (var nested in DetermineElements(e))
                foreach (var el in nested.Elements)
                    Part(el, game, a);
                break;
            case Message m: foreach (var el in m.Expression.Elements) Part(el, game, a); break;
            case MessagePart mp: foreach (var el in mp.Expression.Elements) Part(el, game, a); break;
            case int i: a.Add(P("num", ("v", i))); break;
            case float f: a.Add(P("num", ("v", Math.Round((double)f, 3)))); break;
            case double d: a.Add(P("num", ("v", Math.Round(d, 3)))); break;
            case Payment p: a.Add(P("payment", ("amount", p.Amount), ("by", p.By != Faction.None ? p.By.ToString() : null))); break;
            case Faction f: a.Add(P("faction", ("v", f.ToString()), ("label", Skin.Describe(part)))); break;
            case TreacheryCard c: a.Add(P("treacheryCard", ("id", c.Id), ("label", Skin.Describe(part)))); break;
            case Leader l:
                {
                    var skill = game?.Skill(l) ?? LeaderSkill.None;
                    a.Add(P("leader", ("id", l.Id), ("label", Skin.Describe(part)), ("skill", skill != LeaderSkill.None ? skill.ToString() : null)));
                    break;
                }
            case IHero: a.Add(P("messiah", ("label", Skin.Describe(part)))); break;
            case ResourceCard rc: a.Add(P("resourceCard", ("id", rc.SkinId), ("label", Skin.Describe(part)))); break;
            case Location l: a.Add(P("location", ("id", l.Id), ("label", Skin.Describe(part)))); break;
            case Territory t: a.Add(P("territory", ("id", t.Id), ("label", Skin.Describe(part)))); break;
            case bool b: a.Add(P("bool", ("v", b), ("label", Skin.Describe(part)))); break;
            case Enum en: a.Add(P("enum", ("ty", en.GetType().Name), ("v", en.ToString()), ("label", Skin.Describe(part)))); break;
            default: a.Add(P("text", ("v", Skin.Describe(part)))); break;
        }
    }

    // ---- Text the component shows (LogStyle, Size 20) ----

    public static string Dom(Expression e, Game game)
    {
        var sb = new StringBuilder();
        foreach (var el in e.Elements) Dom(el, game, sb);
        return sb.ToString();
    }

    private static string TextContent(string html) => WebUtility.HtmlDecode(Regex.Replace(html, "<[^>]*>", ""));

    private static void Dom(object? part, Game game, StringBuilder sb)
    {
        switch (part)
        {
            case null: sb.Append("none"); break;
            case string s: sb.Append(s); break;
            case IEnumerable e:
                foreach (var nested in DetermineElements(e))
                foreach (var el in nested.Elements)
                    Dom(el, game, sb);
                break;
            case Message m: foreach (var el in m.Expression.Elements) Dom(el, game, sb); break;
            case MessagePart mp: foreach (var el in mp.Expression.Elements) Dom(el, game, sb); break;
            // SimpleNumberComponent: the number in an SVG <text>.
            case int i: sb.Append(i.ToString(CultureInfo.InvariantCulture)); break;
            case float f: sb.Append(Round(f)); break;
            case double d: sb.Append(Round(d)); break;
            // SpiceComponent's <b>, then " by " and the faction chip's name.
            case Payment payment:
                sb.Append(payment.Amount.ToString(CultureInfo.InvariantCulture));
                if (payment.By != Faction.None) sb.Append(" by ").Append(Skin.Describe(payment.By));
                break;
            case Faction faction: sb.Append(Skin.Describe(faction)); break;
            // LogStyle: the card chip's name; the leader disc is an image.
            case TreacheryCard card: sb.Append(Skin.Describe(card)); break;
            case Leader: break;
            case ResourceCard or LeaderSkill or TechToken or DiscoveryToken or IHero or TerrorType: sb.Append(Skin.Describe(part)); break;
            case Concept monster when monster == Concept.Monster: break;
            case FactionForce or FactionSpecialForce or Ambassador: break;
            // Support.PlaceChipHtml: the territory's name, then a location's compass word.
            case Location or Territory:
                {
                    var territory = part is Location l ? l.Territory : (Territory)part;
                    sb.Append(Skin.Describe(territory));
                    if (part is Location { Orientation: not "" } oriented) sb.Append(oriented.Orientation);
                    break;
                }
            case AuctionType or ChoamEconomicsStatus or TreacheryCardType or StrongholdAdvantage or FactionAdvantage or Ruleset or Rule:
                sb.Append(TextContent(Skin.WithSpiceIcons(Skin.Describe(part), 20)));
                break;
            default: sb.Append(Skin.Describe(part)); break;
        }
    }

    // ---- Records ----

    private static string Camel(string name) => char.ToLowerInvariant(name[0]) + name[1..];

    private static JsonNode? Value(object? v, Type declared, Game game)
    {
        if (v == null) return null;
        if (declared == typeof(object)) return Parts(new Expression([v]), game);
        switch (v)
        {
            case bool b: return b;
            case int i: return i;
            case float f: return f;
            case string s: return s;
            case Enum e: return e.ToString();
            case Location or Territory or IHero or ResourceCard:
                {
                    var a = new JsonArray();
                    Part(v, game, a);
                    return a[0]!.DeepClone();
                }
            case IEnumerable list:
                {
                    var a = new JsonArray();
                    foreach (var item in list) a.Add(Value(item, item?.GetType() ?? typeof(object), game));
                    return a;
                }
            default: return Record(v, game, false);
        }
    }

    private static JsonObject Record(object r, Game game, bool tagged)
    {
        var o = new JsonObject();
        if (tagged) o["type"] = r.GetType().Name;
        foreach (var p in r.GetType().GetProperties(BindingFlags.Public | BindingFlags.Instance))
            o[Camel(p.Name)] = Value(p.GetValue(r), p.PropertyType, game);
        return o;
    }
}
