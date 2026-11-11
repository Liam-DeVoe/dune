using System.Text.Json.Nodes;

namespace Treachery.ServerOracle;

public sealed class GeneratorOptions
{
    public int MaxSteps { get; init; }
    public double AdvanceSeconds { get; init; }
    public bool AsyncPlay { get; init; }
    public int RestartEvery { get; init; }
}

/// <summary>
/// Turns an oracle trace (rust/docs/trace-format.md) into a hub scenario: one connection and user per player,
/// the first user creates the game and the others join, and every event of the trace is submitted with
/// <c>SubmitEvent</c> by the user seated for its initiator (by the host when the trace submitted it as host).
/// Undo lines become <c>RequestUndo</c>, <c>RequestRestoreRecentlyUndone</c> and <c>RequestDismissRecentlyUndone</c>.
/// The scenario is built while it runs through the C# hub, so the generator can look up which user plays which
/// faction after <c>EstablishPlayers</c> reassigned the seats; the transcript of that run is written next to it.
/// </summary>
public static class Generator
{
    public static int Run(string input, string outDir, GeneratorOptions options)
    {
        Directory.CreateDirectory(outDir);
        var failed = 0;
        foreach (var file in Program.InputFiles(input, "*.jsonl*"))
        {
            if (!(file.EndsWith(".jsonl", StringComparison.Ordinal) || file.EndsWith(".jsonl.gz", StringComparison.Ordinal))) continue;
            var name = "gen-" + Program.ScenarioName(file);
            try
            {
                var message = Generate(file, name, outDir, options);
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

    internal sealed class Session(Harness harness)
    {
        public readonly List<string> Scenario = [];
        public readonly List<string> Transcript = [];

        public object? Emit(JsonObject step)
        {
            Scenario.Add(step.ToJsonString());
            var (line, result) = harness.Step(step);
            Transcript.Add(line);
            return result;
        }

        public object? Call(int connection, string method, params JsonNode?[] args) =>
            Emit(new JsonObject { ["op"] = "call", ["conn"] = connection, ["method"] = method, ["args"] = new JsonArray(args) });
    }

    private static string Generate(string traceFile, string name, string outDir, GeneratorOptions options)
    {
        var lines = Program.ReadLines(traceFile);
        var header = JsonNode.Parse(lines[0])!.AsObject();
        var version = header["gameVersion"]!.GetValue<int>();

        // Before version 170 the players come from the event (names and bot rules), not from the seated users, so a
        // trace of a bot game can't be replayed by humans.
        if (version < 170) return $"skipped (game version {version} < 170)";

        var steps = lines.Skip(1).Select(l => JsonNode.Parse(l)!.AsObject()).Where(l => l["kind"]!.GetValue<string>() != "end").ToList();
        if (steps.Count == 0 || steps[0]["event"]?["$type"]?.GetValue<string>() != "EstablishPlayers")
            return "skipped (does not start with EstablishPlayers)";

        // Every seat gets a user, so the game has no bots even when the trace's rules add them (FillWithBots).
        var establish = steps[0]["event"]!.AsObject();
        var settings = establish["Settings"]!.AsObject();
        if (options.AsyncPlay)
        {
            settings["AsyncPlay"] = true;
            settings["AsyncPlayMessageIntervalMinutes"] = 1;
        }

        var players = settings["NumberOfPlayers"]!.GetValue<int>();

        var scenarioHeader = new JsonObject
        {
            ["scenario"] = name,
            ["start"] = "2026-03-02T10:00:00+02:00",
            ["generatedFrom"] = Path.GetFileName(traceFile),
        };

        using var harness = new Harness(scenarioHeader, false);
        var s = new Session(harness);

        // Users: connection i is user i. The admin (for AdminPersistState before restarts) comes last.
        var users = players + (options.RestartEvery > 0 ? 1 : 0);
        var tokens = new string[users + 1];
        var userIds = new int[users + 1];
        for (var i = 1; i <= users; i++)
        {
            s.Emit(new JsonObject { ["op"] = "connect", ["conn"] = i });
            var isAdmin = i > players;
            var login = (Result<LoginInfo>)s.Call(i, "RequestCreateUser", isAdmin ? "admin" : $"player{i}", $"hash{i}", $"player{i}@example.com", isAdmin ? "Administrator" : $"Player {i}")!;
            tokens[i] = login.Contents!.Token;
            userIds[i] = login.Contents.UserId;
        }

        var stateData = version == Game.LatestVersion ? "" : Utilities.Serialize(new GameState { Version = version });
        var created = (Result<GameInitInfo>)s.Call(1, "RequestCreateGame", "Generated game", tokens[1], "", stateData, "")!;
        if (!created.Success) return $"skipped (RequestCreateGame failed: {created.Error} {created.ErrorDetails})";
        var gameId = created.Contents!.GameId;

        // The lobby default is 6 players: the host sets the game's settings before the others can take more seats.
        s.Call(1, "RequestUpdateSettings", tokens[1], gameId, settings.DeepClone());

        for (var i = 2; i <= players; i++)
            s.Call(i, "RequestJoinGame", tokens[i], gameId, "", -1);

        var submitted = 0;
        var rejected = 0;
        foreach (var step in steps.Take(options.MaxSteps))
        {
            if (options.AdvanceSeconds > 0)
                s.Emit(new JsonObject { ["op"] = "advance", ["seconds"] = options.AdvanceSeconds });

            if (options.RestartEvery > 0 && submitted > 0 && submitted % options.RestartEvery == 0)
                Restart(s, users, players, tokens, gameId);

            var kind = step["kind"]!.GetValue<string>();
            switch (kind)
            {
                case "step":
                {
                    var evt = step["event"]!.AsObject().DeepClone().AsObject();
                    var connection = step["isHost"]!.GetValue<bool>() ? 1 : Submitter(gameId, evt["Initiator"]?.GetValue<int>() ?? 0, userIds, players);
                    var result = (VoidResult?)s.Call(connection, "SubmitEvent", tokens[connection], gameId, evt);
                    submitted++;
                    if (result is not { Success: true }) rejected++;
                    break;
                }

                case "undo":
                    s.Call(1, "RequestUndo", tokens[1], gameId, step["untilEventNr"]!.GetValue<int>());
                    break;

                case "restoreRecentlyUndone":
                    s.Call(1, "RequestRestoreRecentlyUndone", tokens[1], gameId);
                    break;

                case "dismissRecentlyUndone":
                    s.Call(1, "RequestDismissRecentlyUndone", tokens[1], gameId);
                    break;
            }
        }

        var scenarioText = scenarioHeader.ToJsonString() + "\n" + string.Join("\n", s.Scenario) + "\n";
        File.WriteAllText(Path.Combine(outDir, name + ".jsonl"), scenarioText);
        File.WriteAllText(Path.Combine(outDir, name + ".transcript.jsonl"),
            Program.TranscriptHeader(name, JsonText.Sha256(scenarioText), false) + "\n" + string.Join("\n", s.Transcript) + "\n");

        return $"{s.Scenario.Count} steps, {submitted} events submitted, {rejected} rejected";
    }

    /// <summary>The connection (user) seated for the player with the given faction; another non-host user if none.</summary>
    private static int Submitter(string gameId, int initiator, int[] userIds, int players)
    {
        var game = Harness.RunningGame(gameId)?.Game;
        var player = game?.Players.FirstOrDefault(p => (int)p.Faction == initiator && initiator != 0);
        if (game != null && player != null)
        {
            foreach (var (userId, seat) in game.Participation.SeatedPlayers)
            {
                if (seat != player.Seat) continue;
                var i = Array.IndexOf(userIds, userId);
                if (i >= 1 && i <= players) return i;
            }
        }

        return players >= 2 ? 2 : 1;
    }

    /// <summary>Persists everything as the admin, restarts the server, and lets every user log in and reconnect.</summary>
    private static void Restart(Session s, int users, int players, string[] tokens, string gameId)
    {
        s.Call(users, "AdminPersistState", tokens[users]);
        s.Emit(new JsonObject { ["op"] = "restart" });
        for (var i = 1; i <= users; i++)
        {
            s.Emit(new JsonObject { ["op"] = "connect", ["conn"] = i });
            s.Call(i, "GetLoginInfo", tokens[i]);
            if (i <= players) s.Call(i, "RequestReconnectGame", tokens[i], gameId);
        }
    }
}
