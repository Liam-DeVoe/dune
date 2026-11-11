// C# reference values for rust/bots/tests/b3_faction_specific.rs (package B3: Bot_FactionSpecific).
//
// Usage: dotnet run -c Release -- b3 KNOWLEDGE.jsonl.gz OUT.jsonl.gz [every]
//
// Replays the events of a knowledge golden (its "header" and "step" lines) and, after every
// `every`-th step, gives each seated player a fresh ClassicBot with a fixed seed and calls the
// Bot_FactionSpecific members on it through reflection, in a fixed order: every Determine method
// and PassAmbassadorActivated, IWantToBeFightersIn and IWantToAnnounceBattleIn for each
// stronghold, BestVoice against every other player, and BestPrescience against every other player
// for each earlier aspect. The calls share the bot, so its state (VoicePlan, PriceSetEarlier,
// TurnWhenRevivalWasRequested, DecidedShipment) and draws carry over, the same on both sides.
// Each call writes the result (an event as Utilities.Serialize<GameEvent> writes it, a bool, an
// enum as its number, a VoicePlan, null, or the exception) and the bot's draw count after it.
// OUT holds only the probes; the test reads the events from the knowledge golden.

using System.Globalization;
using System.IO.Compression;
using System.Text.Json.Nodes;
using Treachery.Bots;
using Treachery.Oracle;

static class B3
{
    private const BindingFlags All = BindingFlags.Instance | BindingFlags.Static | BindingFlags.Public | BindingFlags.NonPublic;

    public static readonly string[] Decisions =
    [
        "DetermineSetShipmentPermission", "DetermineEmperorDiscarded", "DetermineReplacedCardWon",
        "DetermineIxianRemovedCardFromAuction", "DetermineIxianSwappedCardOnBid", "DetermineIxianSelectedStartingCard",
        "DeterminePerformHmsMovement", "DeterminePerformHmsPlacement", "DetermineTakeLosses", "DeterminePerformFremenSetup",
        "DetermineFremenSentMonster", "DetermineFremenRidesMonster", "DetermineBeneGesseritPlacement", "DetermineBeneGesseritFlip",
        "DetermineBeneGesseritBattleAnnouncement", "DetermineBeneGesseritAccompanies", "DetermineBeneGesseritPrediction",
        "DetermineVoice", "DeterminePrescience", "DetermineSetIncreasedRevivalLimits", "DetermineFaceDancerRevealed",
        "DetermineFaceDanced", "DetermineFaceDancerReplaced", "DetermineRequestTleilaxuRevival",
        "DetermineAcceptOrCancelTleilaxuRevival", "DetermineRequestTleilaxuRevival", "DetermineAcceptOrCancelTleilaxuRevival",
        "DetermineResourcesAudited", "DetermineChoamEconomics", "DetermineChoamRemoveForce", "DetermineExtortionPrevented",
        "DeterminePerformMoritaniSetup", "DetermineTerrorPlanted", "DetermineTerrorRevealed", "DetermineAmbassadorPlaced",
        "DetermineAmbassadorActivated", "PassAmbassadorActivated", "DetermineRicheseAnnouncesBlackMarket",
        "DetermineRicheseRevealedNoField", "DetermineRicheseAnnouncesAuction", "DetermineRicheseSpecifiesAuction",
        "DetermineRicheseKeepsUnsoldCard", "DetermineCaptured"
    ];

    private static readonly PrescienceAspect[] Aspects =
        [PrescienceAspect.None, PrescienceAspect.Weapon, PrescienceAspect.Defense, PrescienceAspect.Dial, PrescienceAspect.Leader];

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
        var strongholds = game.Map.Strongholds.Select(s => s.Territory).Distinct().ToList();

        for (var i = 0; i < game.Players.Count; i++)
        {
            var player = game.Players[i];
            if (player.Faction == Faction.None) continue;
            StepExecutor.InvalidateNeighbourCache(game);
            var seed = step * 31 + i + 7;
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

            foreach (var t in strongholds)
            {
                Probe("IWantToBeFightersIn", [t], ["T" + t.Id]);
                Probe("IWantToAnnounceBattleIn", [t], ["T" + t.Id]);
            }

            var territory = game.Map.Carthag.Territory;
            foreach (var o in game.Players)
            {
                if (o == player) continue;
                var oi = "P" + game.Players.IndexOf(o);
                Probe("BestVoice", [null, player, o], ["null", "P" + i, oi]);
                foreach (var aspect in Aspects)
                    Probe("BestPrescience", [player, o, 3f, aspect, territory], ["P" + i, oi, "F3", "A" + (int)aspect, "T" + territory.Id]);
                Probe("BestPrescience", [player, o, 2f, PrescienceAspect.None, territory], ["P" + i, oi, "F2", "A0", "T" + territory.Id]);
            }
        }
    }

    private static string Fmt(object? v) => v switch
    {
        null => "null",
        bool b => b ? "true" : "false",
        PrescienceAspect a => ((int)a).ToString(CultureInfo.InvariantCulture),
        GameEvent e => Utilities.Serialize<GameEvent>(e),
        VoicePlan p => "battle:" + (p.Battle == null ? "null" : "set") + " voice:" + Utilities.Serialize<GameEvent>(p.Voice) +
                       " weapon:" + (p.WeaponToUse?.Id ?? -1) + " defense:" + (p.DefenseToUse?.Id ?? -1) +
                       " survive:" + p.PlayerHeroWillCertainlySurvive + " zero:" + p.OpponentHeroWillCertainlyBeZero,
        _ => throw new ArgumentException("no format for " + v.GetType())
    };
}
