// The static corpus: every Describe/Format/GetImageUrl/GetPopup overload of DefaultSkin.Default
// on every enum value, map object, leader and card, plus string.Format and text edge cases.
// One JSON line per call: {"call":..., "a":[args...], "out":...}. rust/skin/tests/static_corpus.rs
// reads each line, makes the same call in Rust and compares "out".

using System.IO.Compression;
using System.Reflection;
using System.Text.Json;

public static class Static
{
    private static readonly Skin S = DefaultSkin.Default;
    private static StreamWriter _w = null!;

    public static void Run(string path)
    {
        using var w = Output.Open(path);
        _w = w;

        Enums();
        MapObjects();
        Heroes();
        Cards();
        Values();
        Spice();
        FormatCases();
    }

    // An argument token: ["Faction", 20], ["Location", 5], ["string", "x"], ...
    private static object Tok(object? o) => o switch
    {
        null => new object?[] { "null" },
        string s => new object[] { "string", s },
        bool b => new object[] { "bool", b },
        int i => new object[] { "int", i },
        long l => new object[] { "long", l },
        float f => new object[] { "float", BitConverter.SingleToInt32Bits(f) },
        double d => new object[] { "double", BitConverter.DoubleToInt64Bits(d) },
        Enum e => new object[] { e.GetType().Name, Convert.ToInt32(e) },
        Territory t => new object[] { "Territory", t.Id },
        Location l => new object[] { "Location", l.Id },
        Leader l => new object[] { "Leader", l.Id },
        Messiah => new object[] { "Messiah" },
        TreacheryCard c => new object[] { "TreacheryCard", c.Id },
        ResourceCard rc => new object[] { "ResourceCard", ResourceCards().IndexOf(rc) },
        Payment p => new object[] { "Payment", p.Amount },
        _ => throw new Exception("no token for " + o.GetType())
    };

    private static void Emit(string call, object?[] args, Func<string> f)
    {
        string result;
        try
        {
            result = f();
        }
        catch (Exception e)
        {
            result = "!" + e.GetType().Name + ": " + e.Message;
        }

        _w.WriteLine(JsonSerializer.Serialize(new Dictionary<string, object>
        {
            ["call"] = call,
            ["a"] = args.Select(Tok).ToArray(),
            ["out"] = result
        }));
    }

    private static void Emit(string call, object? arg, Func<string> f) => Emit(call, [arg], f);

    private static List<ResourceCard> ResourceCards()
    {
        var m = typeof(Map).GetMethod("GetResourceCardsInAndOutsidePlay", BindingFlags.NonPublic | BindingFlags.Static)!;
        _resourceCards ??= ((IEnumerable<ResourceCard>)m.Invoke(null, [Map])!).ToList();
        return _resourceCards;
    }

    private static List<ResourceCard>? _resourceCards;

    private static readonly Map Map = new();

    private static IEnumerable<T> Values<T>() where T : struct, Enum
        => Enum.GetValues<T>().Concat([(T)(object)(-1), (T)(object)12345]);

    private static void Enums()
    {
        var types = typeof(Game).Assembly.GetTypes()
            .Where(t => t.IsEnum && t.IsPublic && t.Namespace == "Treachery.Shared")
            .OrderBy(t => t.Name, StringComparer.Ordinal);
        foreach (var t in types)
        {
            var values = Enum.GetValues(t).Cast<object>().Concat([Enum.ToObject(t, -1), Enum.ToObject(t, 12345)]);
            foreach (var v in values)
            {
                Emit("Describe", v, () => S.Describe(v));
                Emit("GetImageUrl", v, () => S.GetImageUrl(v));
            }
        }

        foreach (var v in Values<ErrorType>()) Emit("DescribeErrorType", v, () => S.Describe(v));
        foreach (var v in Values<AuctionType>()) Emit("DescribeDetailed", v, () => S.DescribeDetailed(v));
        foreach (var v in Values<Milestone>()) Emit("GetSound", v, () => S.GetSound(v));
        foreach (var v in Values<World>())
        {
            Emit("GetImageUrlWorld", v, () => S.GetImageUrl(v));
            Emit("GetHomeworldCardImageUrl", v, () => S.GetHomeworldCardImageUrl(v));
        }

        foreach (var v in Values<Faction>())
        {
            Emit("GetFactionColor", v, () => S.GetFactionColor(v));
            Emit("GetFactionColorTransparent", v, () => S.GetFactionColorTransparent(v, "80"));
            Emit("GetFactionTableImageUrl", v, () => S.GetFactionTableImageUrl(v));
            Emit("GetFactionFaceDownImageUrl", v, () => S.GetFactionFaceDownImageUrl(v));
            Emit("GetFactionForceImageUrl", v, () => S.GetFactionForceImageUrl(v));
            Emit("GetFactionSpecialForceImageUrl", v, () => S.GetFactionSpecialForceImageUrl(v));
            Emit("FactionChipHtml", [v, false], () => S.FactionChipHtml(v));
            Emit("FactionChipHtml", [v, true, 3, 17], () => S.FactionChipHtml(v, true, [3, 17]));
            foreach (var adv in Values<StrongholdAdvantage>())
                Emit("GetPopupStrongholdAdvantageFaction", [adv, v], () => S.GetPopup(adv, v));
        }

        foreach (var v in Values<DiscoveryTokenType>()) Emit("GetImageUrlDiscoveryTokenType", v, () => S.GetImageUrl(v));
        foreach (var v in Values<DiscoveryToken>())
        {
            Emit("GetImageUrlDiscoveryToken", v, () => S.GetImageUrl(v));
            Emit("GetDiscoveryTokenDescription", v, () => S.GetDiscoveryTokenDescription(v));
            Emit("GetPopupDiscoveryToken", v, () => S.GetPopup(v));
        }

        foreach (var v in Values<TerrorType>())
        {
            Emit("GetTerrorTypeDescription", v, () => S.GetTerrorTypeDescription(v));
            Emit("GetPopupTerrorType", v, () => S.GetPopup(v));
        }

        foreach (var v in Values<Ambassador>())
        {
            Emit("GetAmbassadorDescription", v, () => S.GetAmbassadorDescription(v));
            Emit("GetPopupAmbassador", v, () => S.GetPopup(v));
        }

        foreach (var v in Values<LeaderSkill>()) Emit("GetPopupLeaderSkill", v, () => S.GetPopup(v));
        foreach (var v in Values<TechToken>()) Emit("GetPopupTechToken", v, () => S.GetPopup(v));
        foreach (var v in Values<Nexus>()) Emit("GetNexusCardPopup", v, () => S.GetNexusCardPopup(v));
        foreach (var v in Values<StrongholdAdvantage>()) Emit("GetPopupStrongholdAdvantage", v, () => S.GetPopup(v));
    }

    private static string P(PointD p) => p.X.ToString("R") + "," + p.Y.ToString("R");

    private static void MapObjects()
    {
        foreach (var l in Map.Locations(true))
        {
            Emit("Describe", l, () => S.Describe(l));
            Emit("DescribeLocation", l, () => S.Describe(l));
            Emit("GetCenter", l, () => P(S.GetCenter(l, Map)));
            Emit("GetSpiceLocation", l, () => P(S.GetSpiceLocation(l)));
            if (l is Homeworld hw) Emit("GetPopupHomeworld", l, () => S.GetPopup(hw));
        }

        foreach (var t in Map.Territories(true))
        {
            Emit("Describe", t, () => S.Describe(t));
            Emit("GetTerritoryBorder", t, () => S.GetTerritoryBorder(t));
        }

        // Attached locations: GetCenter follows the location they point at.
        var game = new Game(Game.LatestVersion, new Participation());
        var map = game.Map;
        foreach (var (attached, target) in new (AttachedLocation, Location)[]
                 {
                     (map.HiddenMobileStronghold, map.Arrakeen),
                     (map.HiddenMobileStronghold, map.RockOutcroppings.Locations.First()),
                     (map.Shrine, map.SietchTabr),
                     (map.Cistern, map.TheGreatFlat)
                 })
        {
            attached.PointAt(game, target);
            Emit("GetCenterAttached", [attached, target], () => P(S.GetCenter(attached, map)));
        }
    }

    private static void Heroes()
    {
        IEnumerable<IHero> heroes = LeaderManager.Leaders.Cast<IHero>().Append(LeaderManager.Messiah);
        foreach (var h in heroes)
        {
            Emit("Describe", h, () => S.Describe(h));
            Emit("DescribeHero", h, () => S.Describe(h));
            Emit("GetImageUrlHero", h, () => S.GetImageUrl(h));
            Emit("GetImageUrl", h, () => S.GetImageUrl((object)h));
            Emit("GetPopupHero", h, () => S.GetPopup(h));
            foreach (var skill in new[] { LeaderSkill.None, LeaderSkill.Bureaucrat, LeaderSkill.Warmaster })
                Emit("GetPopupLeaderSkillPair", [h, skill], () => S.GetPopup(h as Leader, skill));
        }
    }

    private static void Cards()
    {
        foreach (var c in TreacheryCardManager.GetCardsInAndOutsidePlay())
        {
            Emit("Describe", c, () => S.Describe(c));
            Emit("DescribeHero", c, () => S.Describe((IHero)c));
            Emit("GetImageUrl", c, () => S.GetImageUrl((object)c));
            Emit("GetImageUrlTreacheryCard", c, () => S.GetImageUrl(c));
            Emit("GetPopupTreacheryCard", c, () => S.GetPopup(c));
            Emit("GetPopupHero", c, () => S.GetPopup((IHero)c));
        }

        foreach (var rc in ResourceCards())
        {
            Emit("Describe", rc, () => S.Describe(rc));
            Emit("GetImageUrl", rc, () => S.GetImageUrl((object)rc));
        }
    }

    private static void Values()
    {
        object?[] values =
        [
            null, "", "text", true, false, 0, -7, int.MaxValue, int.MinValue, 12L, -9876543210L,
            0f, -0f, 0.25f, 1.5f, 1e7f, 1e-5f, 123456.78f, float.NaN, float.PositiveInfinity, float.NegativeInfinity,
            0d, 0.1d, 1e21d, -2.5d, double.NaN,
            new Payment(3), new Payment(-1), new Payment(0)
        ];
        foreach (var v in values) Emit("Describe", v, () => S.Describe(v));
    }

    private static void Spice()
    {
        string[] texts =
        [
            "", "3 spice", "Gain 3 Spice.", "gain 10 spice cards", "the 2 spice blows", "1 spice deck now",
            "1 spice decks", "4 spice deckhand", "x3 spice", "3 spicey", "3  spice", "_3 spice", "3 spice_",
            "pay 2 SPICE and 3 spice", "<b>2 spice</b> & 'quotes' \"dq\" café ½ 𝄞", "12 spice card", "12 spice, cards",
            "007 spice", "5 spice card-like", "3 spice blowsy", "a3 spice", "3 Spice Spice"
        ];
        foreach (var t in texts)
        {
            Emit("WithSpiceIcons", [t, 18, true], () => S.WithSpiceIcons(t));
            Emit("WithSpiceIcons", [t, 20, false], () => S.WithSpiceIcons(t, 20, false));
        }

        Emit("SpiceHtml", ["7", 12], () => S.SpiceHtml("7", 12));
    }

    private static void FormatCases()
    {
        // Templates built from the characters string.Format cares about.
        var r = new Random(42);
        const string alphabet = "{}{}{}0123,-: x";
        string[][] argSets = [[], ["a"], ["a", "bc"], ["a", "bc", "déf"], ["", "𝄞x"]];
        for (var i = 0; i < 4000; i++)
        {
            var len = r.Next(0, 14);
            var chars = new char[len];
            for (var j = 0; j < len; j++) chars[j] = alphabet[r.Next(alphabet.Length)];
            var template = new string(chars);
            var args = argSets[r.Next(argSets.Length)];
            Emit("Format", [template, .. args], () => S.Format(template, args.Cast<object>().ToArray()));
        }

        string[] fixedTemplates =
        [
            "{0}", "{0,5}", "{0,-5}|", "{ 0}", "{0 }", "{0 , 3 }", "{0:N2}", "{0:{}", "{0:}}", "{{0}}", "}}{{", "{0}}",
            "{1000000}", "{999999}", "{0,1000000}", "{0,999999", "{", "}", "x{0", "{0,-}", "{0,}", "{-1}", "{0:x:y}"
        ];
        foreach (var t in fixedTemplates)
            Emit("Format", [t, "ab", "c"], () => S.Format(t, "ab", "c"));
    }
}
