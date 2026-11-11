using System.Text.Json.Nodes;
using Treachery.Oracle;

namespace Treachery.ServerOracle;

public sealed class BotGameOptions
{
    public int Seed { get; init; }
    public int Games { get; init; }
    public int Start { get; init; }
    public int MaxSteps { get; init; }
}

/// <summary>
/// Hub scenarios in which the server's bots play a whole game (rust/docs/server-plan.md, "Generated bot games").
/// A host creates the game with random settings (<see cref="GameConfig.Randomize"/>, at the latest version), a
/// second user observes, and the host starts it. Every seat is then a bot: either the host leaves, so the bots act as
/// hosts for the observer, or the host stays without a seat (<c>RequestAssignSeats</c> with no seats), so the phase
/// control continues the pauses and the host now and then undoes or changes the bot speed. The generator then only
/// moves the virtual clock to the next delayed call, one step per call, and nudges the bots when nothing is pending.
/// Like <see cref="Generator"/>, it runs the scenario through the C# hub while it writes it, and writes the transcript
/// next to it.
/// </summary>
public static class BotGames
{
    public static int Run(string outDir, BotGameOptions options)
    {
        Directory.CreateDirectory(outDir);
        var failed = 0;
        for (var i = options.Start; i < options.Start + options.Games; i++)
        {
            var name = $"botgame-{options.Seed}-{i:D4}";
            try
            {
                var message = Generate(new Random(unchecked(options.Seed * 1_000_003 + i)), name, outDir, options.MaxSteps);
                Console.Error.WriteLine($"{name}: {message}");
            }
            catch (Exception e)
            {
                failed++;
                Console.Error.WriteLine($"{name}: FAILED: {e}");
            }
        }

        return failed == 0 ? 0 : 1;
    }

    private static JsonArray Ints(IEnumerable<int> values) => new(values.Select(v => (JsonNode)v).ToArray());

    private static string Generate(Random rng, string name, string outDir, int maxSteps)
    {
        var config = GameConfig.Randomize(rng);
        var hostStays = rng.NextDouble() < 0.5;
        int[] speeds = [1, 2, 4];
        var speed = speeds[rng.Next(speeds.Length)];

        var scenarioHeader = new JsonObject
        {
            ["scenario"] = name,
            ["start"] = "2026-03-02T10:00:00+02:00",
            ["generatedFrom"] = $"ServerOracle botgames, {(hostStays ? "seatless host" : "bots act as hosts")}, speed {speed}",
        };

        using var harness = new Harness(scenarioHeader, false);
        var s = new Generator.Session(harness);

        // Connection 1 is the host, connection 2 the observer.
        var tokens = new string[3];
        string[] names = ["", "host", "watcher"];
        for (var i = 1; i <= 2; i++)
        {
            s.Emit(new JsonObject { ["op"] = "connect", ["conn"] = i });
            var login = (Result<LoginInfo>)s.Call(i, "RequestCreateUser", names[i], $"hash{i}", $"{names[i]}@example.com", names[i] == "host" ? "Host" : "Watcher")!;
            tokens[i] = login.Contents!.Token;
        }

        var created = (Result<GameInitInfo>)s.Call(1, "RequestCreateGame", "Bot game", tokens[1], "", "", "")!;
        var gameId = created.Contents!.GameId;

        // GameConfig picks the rules, factions, players and turns as the oracle's bot games do.
        var settings = new JsonObject
        {
            ["NumberOfPlayers"] = config.NumberOfPlayers,
            ["MaximumTurns"] = config.MaximumTurns,
            ["InitialRules"] = Ints(config.Rules.Select(r => (int)r)),
            ["AllowedFactionsInPlay"] = Ints(config.AllowedFactions.Select(f => (int)f)),
            ["AutoOpenEmptySeats"] = true,
            ["AsyncPlay"] = false,
            ["AsyncPlayMessageIntervalMinutes"] = 0,
        };

        s.Call(1, "RequestUpdateSettings", tokens[1], gameId, settings.DeepClone());
        s.Call(2, "RequestObserveGame", tokens[2], gameId, "");
        if (speed != 1) s.Call(1, "RequestSetBotSpeed", tokens[1], gameId, speed);

        var establish = new JsonObject
        {
            ["$type"] = "EstablishPlayers",
            ["Settings"] = settings.DeepClone(),
            ["Seed"] = config.EngineSeed,
            ["Initiator"] = 0,
        };
        var started = (VoidResult?)s.Call(1, "SubmitEvent", tokens[1], gameId, establish);
        if (started is not { Success: true } || Harness.RunningGame(gameId)!.Game.CurrentPhase <= Phase.AwaitingPlayers)
            return "skipped (EstablishPlayers failed)";

        if (hostStays)
            s.Call(1, "RequestAssignSeats", tokens[1], gameId, new JsonObject());
        else
            s.Call(1, "RequestLeaveGame", tokens[1], gameId);

        var nudges = 0;
        var undos = 0;
        var outcome = "step limit";
        while (s.Scenario.Count < maxSteps)
        {
            var managed = Harness.RunningGame(gameId);
            if (managed == null)
            {
                outcome = "game gone";
                break;
            }

            var game = managed.Game;
            if (game.CurrentMainPhase == MainPhase.Ended)
            {
                outcome = "ended";
                break;
            }

            var due = harness.NextTimerDue;
            if (due == null)
            {
                if (nudges >= 2)
                {
                    outcome = $"stalled in {game.CurrentPhase}";
                    break;
                }

                nudges++;
                if (hostStays && game.Participation.BotsSpeed == 0)
                    s.Call(1, "RequestSetBotSpeed", tokens[1], gameId, 1);
                else
                    s.Call(2, "RequestNudgeBots", tokens[2], gameId);
                continue;
            }

            var before = game.History.Count;
            var ms = Math.Max(0, (due.Value - harness.Now).TotalMilliseconds);
            s.Emit(new JsonObject { ["op"] = "advance", ["ms"] = ms });
            if (Harness.RunningGame(gameId)?.Game.History.Count != before) nudges = 0;

            if (!hostStays) continue;

            game = Harness.RunningGame(gameId)?.Game;
            if (game == null) continue;

            // Now and then the host undoes a few events, and restores, dismisses or keeps the undone ones.
            if (game.History.Count > 12 && rng.NextDouble() < 0.004)
            {
                undos++;
                s.Call(1, "RequestUndo", tokens[1], gameId, game.History.Count - rng.Next(1, 6));
                var r = rng.NextDouble();
                if (r < 0.3) s.Call(1, "RequestRestoreRecentlyUndone", tokens[1], gameId);
                else if (r < 0.6) s.Call(1, "RequestDismissRecentlyUndone", tokens[1], gameId);
            }

            // Or changes the bot speed, pausing them (0) included.
            if (rng.NextDouble() < 0.003)
                s.Call(1, "RequestSetBotSpeed", tokens[1], gameId, new[] { 0, 1, 2, 4 }[rng.Next(4)]);
        }

        var events = Harness.RunningGame(gameId)?.Game.History.Count ?? 0;
        var scenarioText = scenarioHeader.ToJsonString() + "\n" + string.Join("\n", s.Scenario) + "\n";
        File.WriteAllText(Path.Combine(outDir, name + ".jsonl"), scenarioText);
        File.WriteAllText(Path.Combine(outDir, name + ".transcript.jsonl"),
            Program.TranscriptHeader(name, JsonText.Sha256(scenarioText), false) + "\n" + string.Join("\n", s.Transcript) + "\n");

        return $"{s.Scenario.Count} steps, {events} events, {undos} undos, {outcome}";
    }
}
