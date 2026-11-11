// C# reference values for rust/bots/tests/b2_battle.rs (package B2: Bot_Battle).
//
// Usage: dotnet run -c Release -- b2 KNOWLEDGE.jsonl.gz OUT.jsonl.gz [every]
//
// Replays the events of a knowledge golden (its "header" and "step" lines). After every step in
// the battle phase it calls the battle decisions that fit the state (the battle plan for the two
// fighters, BattleInitiated for a player with battles to fight, Retreat for the loser, and so on),
// and after every `every`-th step it calls GetDialNeeded, DetermineDialShortageForBattle and a few
// static helpers for every seated player. Every call gets a fresh ClassicBot with a fixed seed.
// Each probe writes the result (an event as Utilities.Serialize<GameEvent> writes it, a number, a
// bool, null, or the exception) and the bot's draw count after the call. OUT holds only the
// probes; the test reads the events from the knowledge golden.
//
// DetermineRockWasMelted changes the game (it runs Battle.DetermineBattleOutcome on the live
// plans), so after it the line also records the number of messages in the current report.

using System.Globalization;
using System.IO.Compression;
using System.Text.Json.Nodes;
using Treachery.Bots;
using Treachery.Oracle;

static class B2
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

            ProbeAll(game, step, step % every == 0, output);
        }

        output.Flush();
    }

    private static void ProbeAll(Game game, int step, bool helpers, StreamWriter output)
    {
        var probeNr = 0;
        var strongholds = game.Map.Strongholds.Take(2).Select(s => s.Territory).ToList();

        for (var i = 0; i < game.Players.Count; i++)
        {
            var player = game.Players[i];
            if (player.Faction == Faction.None) continue;

            void Probe(string name, object?[] args, string[] encoded)
            {
                StepExecutor.InvalidateNeighbourCache(game);
                var seed = step * 997 + probeNr++ * 7 + i;
                var random = new LoggedRandom(seed);
                var bot = new ClassicBot(game, player, BotParameters.GetDefaultParameters(player.Faction), random);
                var m = typeof(ClassicBot).GetMethods(All).Single(x => x.Name == name && x.GetParameters().Length == args.Length);
                string r;
                try
                {
                    r = Fmt(m.Invoke(m.IsStatic ? null : bot, args));
                }
                catch (TargetInvocationException e)
                {
                    r = "EX:" + e.InnerException!.GetType().FullName + ": " + e.InnerException.Message;
                }

                var a = new JsonArray();
                foreach (var x in encoded) a.Add(x);
                var o = new JsonObject { ["n"] = step, ["seed"] = seed, ["p"] = i, ["m"] = name, ["a"] = a, ["r"] = r, ["d"] = random.GetCount() };
                if (name == "DetermineRockWasMelted") o["messages"] = game.CurrentReport.Messages.Count;
                output.WriteLine(o.ToJsonString());
            }

            if (game.CurrentMainPhase == MainPhase.Battle)
            {
                var battle = game.CurrentBattle;
                var fighter = battle != null && battle.IsAggressorOrDefender(player);

                if (battle == null && Battle.BattlesToBeFought(game, player).Any()) Probe("DetermineBattleInitiated", [], []);

                if (game.BattleAboutToStart != null && game.IsPlaying(Faction.Ecaz) &&
                    (player.Faction == Faction.Ecaz || player.Ally == Faction.Ecaz))
                    Probe("DetermineBattleClaimed", [], []);

                if (fighter)
                {
                    var myPlan = battle!.PlanOf(player);
                    var opponentPlan = battle.PlanOf(battle.OpponentOf(player));

                    if (myPlan == null)
                    {
                        Probe("DetermineBattle", [], []);
                        Probe("DetermineBattlePlan", [false], ["false"]);
                        Probe("DetermineBattlePlan", [true], ["true"]);
                        Probe("DetermineSwitchedSkilledLeader", [], []);
                        Probe("DetermineHmsAdvantageChosen", [], []);
                        Probe("DetermineThought", [], []);
                    }

                    Probe("DetermineTreacheryCalled", [], []);
                    if (game.CurrentThought != null) Probe("DetermineThoughtAnswered", [], []);
                    if (myPlan != null && opponentPlan != null) Probe("DeterminePortableAntidoteUsed", [], []);
                    if (myPlan != null) Probe("DetermineBattleConcluded", [], []);
                    if (game.CurrentPhase == Phase.Retreating && game.BattleLoser == player.Faction) Probe("DetermineRetreat", [], []);
                    if (game.CurrentPhase == Phase.BattleConclusion) Probe("DetermineLoserConcluded", [], []);
                    if (game.CurrentPhase == Phase.MeltingRock && RockWasMelted.CanBePlayed(game, player)) Probe("DetermineRockWasMelted", [], []);
                }
                else if (battle != null)
                {
                    Probe("DetermineTreacheryCalled", [], []);
                }
            }

            if (!helpers) continue;

            Probe("DetermineAudited", [], []);
            Probe("DetermineAuditCancelled", [], []);
            Probe("DetermineResidualPlayed", [], []);
            Probe("CanOnlyUseTraitorsOrFacedancers", [player], ["P" + i]);
            Probe("HeroesForBattle", [game, player, true], ["G", "P" + i, "true"]);
            Probe("HeroesForBattle", [game, player, false], ["G", "P" + i, "false"]);

            foreach (var o in game.Players)
            {
                if (o == player || o.Faction == Faction.None) continue;
                var op = "P" + game.Players.IndexOf(o);
                Probe("NrOfUnknownOpponentCards", [game, player, o], ["G", "P" + i, op]);

                var territories = strongholds.ToList();
                var occupied = o.ForcesOnPlanet.Keys.Select(l => l.Territory).FirstOrDefault();
                if (occupied != null && !territories.Contains(occupied)) territories.Add(occupied);

                foreach (var t in territories)
                {
                    Probe("GetDialNeeded", [t, o, false], ["T" + t.Id, op, "false"]);
                    Probe("GetDialNeeded", [t, o, true], ["T" + t.Id, op, "true"]);
                    if (t == territories[^1])
                        foreach (var dial in new[] { 2.5f, 13.5f })
                            Probe("DetermineDialShortageForBattle", [dial, o.Faction, t, 5, 3, 4], [F(dial), "F" + (int)o.Faction, "T" + t.Id, "i5", "i3", "i4"]);
                }
            }

            Probe("GetDialNeeded", [strongholds[0], null, true], ["T" + strongholds[0].Id, "N", "true"]);
        }
    }

    private static string F(float f) => f.ToString(CultureInfo.InvariantCulture);

    private static string Fmt(object? v) => v switch
    {
        null => "null",
        bool b => b ? "true" : "false",
        int i => i.ToString(CultureInfo.InvariantCulture),
        float f => F(f),
        GameEvent e => Utilities.Serialize<GameEvent>(e),
        IHero h => "H" + h.Id,
        IEnumerable<IHero> e => "[" + string.Join(",", e.Select(Fmt)) + "]",
        _ => throw new ArgumentException("no format for " + v.GetType())
    };
}
