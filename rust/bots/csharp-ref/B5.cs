// C# reference values for rust/bots/tests/b5_movement.rs (package B5: Bot_Shipment, Bot_Move).
//
// Usage: dotnet run -c Release -- b5 KNOWLEDGE.jsonl.gz OUT.jsonl.gz [every]
//
// Replays the events of a knowledge golden and, after every `every`-th step, gives each seated
// player a fresh ClassicBot with a fixed seed and calls the B5 members on it through reflection,
// in a fixed order, whatever the phase. The shipment sub-strategies run directly (with the
// decided-shipment state cleared first), so branches that bot games rarely reach (Guild back to
// reserves, empty homeworlds, cunning No-Field, flights) are covered too. Each call writes its
// result, the bot's shipment state where it changes it, and the bot's draw count after the call.

using System.Globalization;
using System.IO.Compression;
using System.Text.Json.Nodes;
using Treachery.Bots;
using Treachery.Oracle;

static class B5
{
    private const BindingFlags All = BindingFlags.Instance | BindingFlags.Static | BindingFlags.Public | BindingFlags.NonPublic;

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

    private static string L(Location? l) => l == null ? "null" : "L" + l.Id;
    private static string F(float f) => f.ToString(CultureInfo.InvariantCulture);

    private static void ProbeAll(Game game, int step, StreamWriter output)
    {
        for (var i = 0; i < game.Players.Count; i++)
        {
            var player = game.Players[i];
            if (player.Faction == Faction.None) continue;
            StepExecutor.InvalidateNeighbourCache(game);
            var seed = step * 37 + i + 5;
            var random = new LoggedRandom(seed);
            var bot = new ClassicBot(game, player, BotParameters.GetDefaultParameters(player.Faction), random);

            var t = typeof(ClassicBot);
            var action = t.GetProperty("DecidedShipmentAction", All)!;
            var decided = t.GetProperty("DecidedShipment", All)!;
            var finalDestination = t.GetProperty("FinalDestination", All)!;

            string State()
            {
                var s = (Shipment?)decided.GetValue(bot);
                return "A" + (int)action.GetValue(bot)! + " " + (s == null ? "null" : Utilities.Serialize<GameEvent>(s)) + " " + L((Location?)finalDestination.GetValue(bot));
            }

            void Reset()
            {
                action.SetValue(bot, Enum.ToObject(action.PropertyType, 0));
                decided.SetValue(bot, null);
                finalDestination.SetValue(bot, null);
            }

            void Probe(string name, object?[] args, string[] encoded, Func<object?, object?[], string> fmt, bool reset = false)
            {
                if (reset) Reset();
                var m = t.GetMethods(All).Single(x => x.Name == name && x.GetParameters().Length == args.Length);
                string r;
                try
                {
                    r = fmt(m.Invoke(bot, args), args);
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

            string Ev(object? v, object?[] _) => (v == null ? "null" : Utilities.Serialize<GameEvent>((GameEvent)v)) + " " + State();
            string St(object? v, object?[] _) => State();

            Probe("DetermineShipment", [], [], Ev);
            Probe("DetermineMove", [], [], Ev);
            Probe("DetermineCaravan", [], [], Ev);
            Probe("DetermineFlightUsed", [], [], Ev);
            Probe("DetermineFlightDiscoveryUsed", [], [], Ev);

            Probe("DetermineShipment_PreventNormalWin", [4, 1, 6f, 1, 5], ["4", "1", "6", "1", "5"], St, true);
            Probe("DetermineShipment_PreventNormalWin", [20, 2, 2f, 0, 20], ["20", "2", "2", "0", "20"], St, true);
            Probe("DetermineShipment_ShipDirectlyToSpiceOrDiscoveryAsFremen", [], [], St, true);
            Probe("DetermineShipment_PreventFremenWin", [], [], St, true);
            Probe("DetermineShipment_StrengthenWeakestStronghold", [false, 1, 6f, true], ["false", "1", "6", "true"], St, true);
            Probe("DetermineShipment_StrengthenWeakestStronghold", [true, 2, 2f, false], ["true", "2", "2", "false"], St, true);
            Probe("DetermineShipment_ShipToStrongholdNearSpice", [], [], St, true);
            Probe("DetermineShipment_TakeVacantStronghold", [3, 1, 5], ["3", "1", "5"], St, true);
            Probe("DetermineShipment_AttackEmptyHomeworld", [1, 5], ["1", "5"], St, true);
            Probe("DetermineShipment_AttackWeakHMS", [1, 6f, 0, 5], ["1", "6", "0", "5"], St, true);
            Probe("DetermineShipment_AttackWeakStronghold", [1, 1, 20], ["1", "1", "20"], St, true);
            Probe("DetermineShipment_AttackWeakStronghold", [2, 0, 0], ["2", "0", "0"], St, true);
            Probe("DetermineShipment_ShipDirectlyToSpiceOrDiscoveryAsGuildOrGuildAlly", [], [], St, true);
            Probe("DetermineShipment_UnlockMoveBonus", [1], ["1"], St, true);
            Probe("DetermineShipment_BackToReserves", [], [], St, true);
            Probe("DetermineShipment_DummyAttack", [1], ["1"], St, true);
            Probe("DetermineShipment_PolarSinkAsFremen", [], [], St, true);

            var targets = game.Map.Locations(false).Where(l => l.IsStronghold).Append(game.Map.HiddenMobileStronghold).ToList();
            foreach (var l in targets)
            foreach (var prefer in new[] { true, false })
            {
                object?[] args = [5.5f, true, l, null, player.ForcesInReserve, player.SpecialForcesInReserve, 0, 0, 0, 0, 1, 5, prefer];
                Probe("DetermineShortageForShipment", args, [L(l), prefer ? "true" : "false"],
                    (v, a) => F((float)v!) + " " + a[6] + " " + a[7] + " " + a[8] + " " + a[9]);
            }

            foreach (var l in targets)
                Probe("MaxPotentialForceShortage", [true, l], ["true", L(l)], (v, _) => F((float)v!));

            foreach (var (l, b) in player.ForcesOnPlanet)
                Probe("DetermineMostSuitableNearbyLocation", [l, b, true, true], [L(l), "B" + b.AmountOfForces + "," + b.AmountOfSpecialForces],
                    (v, _) => L((Location?)v));
        }
    }
}
