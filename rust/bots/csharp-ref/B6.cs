// C# reference values for rust/bots/tests/b6_decisions.rs (package B6: Bot_Bidding, Bot_Deals, Bot_Alliance).
//
// Usage: dotnet run -c Release -- b6 KNOWLEDGE.jsonl.gz OUT.jsonl.gz [every]
//
// Replays the events of a knowledge golden (its "header" and "step" lines) and, after every
// `every`-th step, gives each seated player a fresh ClassicBot with a fixed seed and calls the B6
// members on it through reflection, in a fixed order. Each call writes the result (the event as
// Utilities.Serialize<GameEvent> writes it, a number, a bool, null, or the exception) and the
// bot's draw count after the call. OUT holds only the probes; the test reads the events from
// the knowledge golden.

using System.Globalization;
using System.IO.Compression;
using System.Text.Json.Nodes;
using Treachery.Bots;
using Treachery.Oracle;

static class B6
{
    private const BindingFlags All = BindingFlags.Instance | BindingFlags.Static | BindingFlags.Public | BindingFlags.NonPublic;

    public static readonly string[] Decisions =
    [
        "DetermineBid", "DetermineBlackMarketBid", "DetermineDealAccepted", "DetermineDealCancelled", "DetermineDealOffered",
        "DetermineAllianceByTerror", "DetermineAllianceByAmbassador", "DetermineNexusVoted", "DetermineAllianceOffered",
        "DetermineAlliancePassed", "DetermineAllianceBroken", "DetermineNexusCardDrawn", "DetermineAlliancePermissions",
        "DetermineCardTraded", "DetermineAllowedResources"
    ];

    public static void Run(string inPath, string outPath, int every)
    {
        using var input = new StreamReader(new GZipStream(File.OpenRead(inPath), CompressionMode.Decompress));
        using var outFile = File.Create(outPath);
        using var gz = new GZipStream(outFile, CompressionLevel.SmallestSize);
        var output = new StreamWriter(gz) { NewLine = "\n" };
        Game game = null!;

        string? text;
        while ((text = input.ReadLine()) != null)
        {
            var line = JsonNode.Parse(text)!.AsObject();
            var kind = (string)line["k"]!;
            if (kind == "header")
            {
                var p = line["participation"]!.AsObject();
                var participation = new Participation { BotsSpeed = (int)p["BotsSpeed"]! };
                foreach (var kv in p["PlayerNames"]!.AsArray()) participation.PlayerNames.Add((int)kv![0]!, (string)kv[1]!);
                foreach (var kv in p["SeatedPlayers"]!.AsArray()) participation.SeatedPlayers.Add((int)kv![0]!, (int)kv[1]!);
                foreach (var v in p["Observers"]!.AsArray()) participation.Observers.Add((int)v!);
                foreach (var v in p["Hosts"]!.AsArray()) participation.Hosts.Add((int)v!);
                foreach (var v in p["AvailableSeats"]!.AsArray()) participation.AvailableSeats.Add((int)v!);
                foreach (var v in p["Kicked"]!.AsArray()) participation.Kicked.Add((int)v!);
                game = new Game((int)line["version"]!, participation);
                continue;
            }

            if (kind != "step") continue;

            var step = (int)line["n"]!;
            var evt = Utilities.Deserialize<GameEvent>((string)line["event"]!)!;
            evt.Initialize(game);
            StepExecutor.Run(game, evt, (bool)line["validate"]!, (bool)line["isHost"]!);

            if (step % every == 0) ProbeAll(game, step, output);
        }

        output.Flush();
    }

    private static void ProbeAll(Game game, int step, StreamWriter output)
    {
        for (var i = 0; i < game.Players.Count; i++)
        {
            var player = game.Players[i];
            if (player.Faction == Faction.None) continue;
            StepExecutor.InvalidateNeighbourCache(game);
            var seed = step * 31 + i + 6;
            var random = new LoggedRandom(seed);
            var bot = new ClassicBot(game, player, BotParameters.GetDefaultParameters(player.Faction), random);

            void Probe(string name, object?[] args, string[] encoded)
            {
                var m = typeof(ClassicBot).GetMethods(All).Single(x => x.Name == name && x.GetParameters().Length == args.Length);
                string r;
                try
                {
                    r = Fmt(m.Invoke(bot, args));
                }
                catch (TargetInvocationException e)
                {
                    r = "EX:" + e.InnerException!.GetType().FullName + ": " + e.InnerException.Message;
                }

                var a = new JsonArray();
                foreach (var x in encoded) a.Add(x);
                output.WriteLine(new JsonObject
                {
                    ["n"] = step, ["seed"] = seed, ["p"] = i, ["m"] = name, ["a"] = a, ["r"] = r, ["d"] = random.GetCount()
                }.ToJsonString());
            }

            foreach (var name in Decisions) Probe(name, [], []);
            foreach (var o in game.Players) Probe("PlayerStanding", [o], ["P" + game.Players.IndexOf(o)]);
            var top = game.CardsOnAuction?.Top;
            if (top != null) Probe("WannaHave", [top], ["C" + top.Id]);
            foreach (var c in player.TreacheryCards) Probe("WannaHave", [c], ["C" + c.Id]);
        }
    }

    // The unboxing casts the bots apply to ClairVoyancePlayed.Parameter1/2 (bots/src/support.rs),
    // on every kind of value Parameter1 can hold, plus a few others.
    public static void Unbox(string outPath)
    {
        var game = new Game();
        object?[] values =
        [
            null, TreacheryCardType.Laser, Faction.Atreides, 3, 2.5f, 2.5, "x", LeaderManager.Leaders[0],
            TreacheryCardManager.Lookup.Find(5), LeaderManager.Messiah, game.Map.TerritoryLookup.Find(3)
        ];
        (string, Func<object?, object?>)[] casts =
        [
            ("TreacheryCardType", o => (TreacheryCardType)o!), ("Faction", o => (Faction)o!), ("int", o => (int)o!),
            ("float", o => (float)o!), ("IHero", o => (IHero?)o)
        ];
        var result = new JsonArray();
        foreach (var v in values)
        foreach (var (name, cast) in casts)
        {
            string r;
            try
            {
                r = cast(v) switch
                {
                    null => "null",
                    float f => "float:" + f.ToString(CultureInfo.InvariantCulture),
                    int i => "int:" + i,
                    Enum e => "enum:" + Convert.ToInt32(e),
                    IHero h => "hero:" + h.Id,
                    var x => throw new ArgumentException(x.GetType().Name)
                };
            }
            catch (Exception e) when (e is NullReferenceException or InvalidCastException)
            {
                r = "EX:" + e.GetType().FullName + ": " + e.Message;
            }

            var kind = v switch
            {
                null => "null", Enum e => e.GetType().Name + ":" + Convert.ToInt32(e), int i => "int:" + i, float f => "float:" + f.ToString(CultureInfo.InvariantCulture),
                double d => "double:" + d.ToString(CultureInfo.InvariantCulture), string s => "string:" + s, Messiah => "messiah",
                TreacheryCard c => "card:" + c.Id, Leader l => "leader:" + l.Id, Territory t => "territory:" + t.Id, _ => "?"
            };
            result.Add(new JsonArray(kind, name, r));
        }

        File.WriteAllText(outPath, result.ToJsonString() + "\n");
    }

    private static string Fmt(object? v) => v switch
    {
        null => "null",
        bool b => b ? "true" : "false",
        int i => i.ToString(CultureInfo.InvariantCulture),
        GameEvent e => Utilities.Serialize<GameEvent>(e),
        _ => throw new ArgumentException("no format for " + v.GetType())
    };
}
