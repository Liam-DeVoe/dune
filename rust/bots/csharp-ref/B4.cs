// C# reference values for rust/bots/tests/b4_other.rs (package B4: Bot_Other).
//
// Usage: dotnet run -c Release -- b4 KNOWLEDGE.jsonl.gz OUT.jsonl.gz [every]
//
// Replays the events of a knowledge golden (its "header" and "step" lines) and, after every
// `every`-th step, calls the Bot_Other members for each seated player through reflection. Every
// call gets a fresh ClassicBot with its own seed (written to OUT), so one call can't change what
// the next one sees, and the neighbour cache is invalidated before each call. Some calls first
// set Player.Nexus or Game.LatestClairvoyance, to reach the Nexus and clairvoyance branches, and
// restore them afterwards. Each call writes its result (an event as Utilities.Serialize<GameEvent>
// writes it, a number, a bool, a hero/territory/location id, null, or the exception) and the
// bot's draw count after the call.

using System.Globalization;
using System.IO.Compression;
using System.Text.Json.Nodes;
using Treachery.Bots;
using Treachery.Oracle;

static class B4
{
    private const BindingFlags All = BindingFlags.Instance | BindingFlags.Static | BindingFlags.Public | BindingFlags.NonPublic;

    public static readonly string[] Decisions =
    [
        "DetermineResourcesTransferred", "DetermineDiscoveryEntered", "DetermineDiscoveryRevealed", "DetermineTestingStationUsed",
        "DetermineDivideResources", "DetermineCardGiven", "DetermineDivideResourcesAccepted", "DetermineDiscarded",
        "DetermineTraitorDiscarded", "DetermineWorstTraitor", "DetermineNexusPlayed", "DetermineTerritoryToRemoveEcazFrom",
        "DetermineCharityClaimed", "DetermineKarmaFreeRevival", "DetermineKarmaHandSwap", "DetermineKarmaShipmentPrevention",
        "DetermineKarmaHandSwapInitiated", "DetermineHarvesterPlayed", "DetermineMulliganPerformed", "DetermineDelay",
        "DetermineClairvoyance", "DetermineClairVoyanceAnswered", "DetermineRaiseDeadPlayed", "DetermineAmalPlayed",
        "DetermineRecruitsPlayed", "DetermineMetheorPlayed", "DetermineStormDialled", "DetermineTraitorsSelected",
        "DetermineFactionTradeOffered", "DetermineThumperPlayed", "DetermineStormSpellPlayed", "DetermineDistransUsed",
        "DetermineRevival", "DetermineDiscardedSearchedAnnounced", "DetermineDiscardedSearched", "DetermineDiscardedTaken",
        "DetermineJuicePlayed", "DetermineBureaucracy", "DetermineDiplomacy", "DetermineSkillAssigned", "DeterminePlanetology"
    ];

    // The factions with a case in the Nexus switches.
    public static readonly Faction[] NexusFactions =
    [
        Faction.Atreides, Faction.Harkonnen, Faction.Fremen, Faction.Emperor, Faction.Guild, Faction.BeneGesserit,
        Faction.Ixian, Faction.Tleilaxu, Faction.Choam, Faction.Richese, Faction.Ecaz, Faction.Moritani
    ];

    public static readonly string[] NexusMethods = ["DetermineNexusPlayed_Betrayal", "DetermineNexusPlayed_Cunning", "DetermineNexusPlayed_SecretAlly"];

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

    // Clairvoyance questions to answer: [question, QuestionParameter1, QuestionParameter2]. "LEADER" is
    // the first leader of the player, "TRAITOR" its first traitor.
    private static readonly string?[][] Questions =
    [
        ["CardTypeAsDefenseInBattle", "PoisonDefense", null], ["CardTypeAsDefenseInBattle", "ProjectileDefense", null],
        ["CardTypeAsWeaponInBattle", "Poison", null], ["CardTypeInBattle", "Projectile", null], ["CardTypeInBattle", "Mercenary", null],
        ["CardTypeInBattle", null, null], ["DialOfMoreThanXInBattle", "3.5", null], ["LeaderInBattle", "LEADER", null],
        ["HasCardTypeInHand", "Shield", null], ["HasCardTypeInHand", "Useless", null], ["HasCardTypeInHand", null, null],
        ["LeaderAsFacedancer", "LEADER", null], ["LeaderAsTraitor", "TRAITOR", null], ["LeaderAsTraitor", "LEADER", null],
        ["Prediction", "Atreides", "3"], ["Prediction", "PREDICTED", "PREDICTEDTURN"], ["Prediction", "Fremen", null],
        ["WillAttackX", "3", null]
    ];

    private static void ProbeAll(Game game, int step, StreamWriter output)
    {
        for (var i = 0; i < game.Players.Count; i++)
        {
            var player = game.Players[i];
            if (player.Faction == Faction.None) continue;
            var nr = 0;

            void Probe(string name, Func<ClassicBot, object?> call, string[] encoded)
            {
                StepExecutor.InvalidateNeighbourCache(game);
                var seed = step * 1000 + i * 100 + nr++;
                var random = new LoggedRandom(seed);
                var bot = new ClassicBot(game, player, BotParameters.GetDefaultParameters(player.Faction), random);
                string r;
                try
                {
                    r = Fmt(call(bot));
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

            object? Invoke(ClassicBot bot, string name, object?[] args) =>
                typeof(ClassicBot).GetMethods(All).Single(x => x.Name == name && x.GetParameters().Length == args.Length).Invoke(bot, args);

            foreach (var name in Decisions) Probe(name, b => Invoke(b, name, []), []);

            foreach (var f in game.Players.Select(p => p.Faction).Append(Faction.None))
                Probe("IsWinningOrIsOpponentInBattle", b => Invoke(b, "IsWinningOrIsOpponentInBattle", [f]), ["F" + (int)f]);

            foreach (var (t, w) in new[] { (TreacheryCardType.PoisonDefense, false), (TreacheryCardType.ProjectileDefense, false), (TreacheryCardType.Poison, true), (TreacheryCardType.Projectile, true) })
                Probe("OpponentMayNotUse", b => Invoke(b, "OpponentMayNotUse", [t, w]), ["Y" + (int)t, w ? "true" : "false"]);

            foreach (var l in player.Leaders) Probe("HeroRevivalPenalty", b => Invoke(b, "HeroRevivalPenalty", [l]), ["H" + l.Id]);

            Probe("BestRevivalLocation", b => Invoke(b, "BestRevivalLocation", [game.Map.Locations(false).ToList()]), ["ALL"]);

            foreach (var cunning in new[] { false, true })
                Probe("DetermineOptimalUseOfEmperorRevivals", _ =>
                {
                    object?[] args = [game, player, null, null, cunning];
                    typeof(ClassicBot).GetMethod("DetermineOptimalUseOfEmperorRevivals", All)!.Invoke(null, args);
                    return (int)args[2]! + "," + (int)args[3]!;
                }, [cunning ? "true" : "false"]);

            var savedNexus = player.Nexus;
            foreach (var f in NexusFactions)
            foreach (var m in NexusMethods)
                Probe(m, b =>
                {
                    player.Nexus = f;
                    try
                    {
                        return Invoke(b, m, [new NexusPlayed(game, player.Faction) { Faction = f }]);
                    }
                    finally
                    {
                        player.Nexus = savedNexus;
                    }
                }, ["F" + (int)f]);

            var savedClairvoyance = game.LatestClairvoyance;
            var initiator = game.Players.FirstOrDefault(p => p != player)?.Faction ?? Faction.None;
            foreach (var q in Questions)
            {
                var p1 = q[1] switch
                {
                    "LEADER" => player.Leaders.FirstOrDefault()?.Id.ToString(),
                    "TRAITOR" => player.Traitors.FirstOrDefault()?.Id.ToString(),
                    "PREDICTED" => player.PredictedFaction.ToString(),
                    var x => x
                };
                var p2 = q[2] == "PREDICTEDTURN" ? player.PredictedTurn.ToString() : q[2];
                Probe("ClairVoyanceAnswered", b =>
                {
                    SetLatestClairvoyance(game, new ClairVoyancePlayed(game, initiator)
                    {
                        Target = player.Faction, Question = Enum.Parse<ClairvoyanceQuestion>(q[0]!), QuestionParameter1 = p1, QuestionParameter2 = p2
                    });
                    try
                    {
                        return Invoke(b, "DetermineClairVoyanceAnswered", []);
                    }
                    finally
                    {
                        SetLatestClairvoyance(game, savedClairvoyance);
                    }
                }, ["F" + (int)initiator, q[0]!, p1 ?? "", p2 ?? "", p1 == null ? "null1" : "", p2 == null ? "null2" : ""]);
            }
        }
    }

    // The setter is internal to Shared.
    private static void SetLatestClairvoyance(Game game, ClairVoyancePlayed? value) =>
        typeof(Game).GetProperty("LatestClairvoyance")!.SetValue(game, value);

    private static string Fmt(object? v) => v switch
    {
        null => "null",
        bool b => b ? "true" : "false",
        int i => i.ToString(CultureInfo.InvariantCulture),
        string s => s,
        GameEvent e => Utilities.Serialize<GameEvent>(e),
        IHero h => "H" + h.Id,
        Territory t => "T" + t.Id,
        Location l => "L" + l.Id,
        _ => throw new ArgumentException("no format for " + v.GetType())
    };
}
