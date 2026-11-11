using System.IO.Compression;
using System.Text.Json.Nodes;

namespace Treachery.Oracle;

/// <summary>
/// Replays a trace with the C# engine, using only what is in the trace (trace-format.md section 6), and checks that every
/// recorded value comes out the same. This is the replay the Rust side implements; running it in C# shows that a trace
/// carries everything a replayer needs (no bots, no oracle state).
/// </summary>
public static class Verifier
{
    private static readonly string[] ComparedStepKeys =
        ["result", "validation", "exception", "eventAfter", "historyCount", "random", "typeLists", "applicable", "hash", "state"];

    public static int Run(string path)
    {
        var files = Directory.Exists(path)
            ? Directory.GetFiles(path).Where(f => f.EndsWith(".jsonl") || f.EndsWith(".jsonl.gz")).OrderBy(f => f, StringComparer.Ordinal).ToArray()
            : [path];

        var failed = 0;
        var lines = 0L;
        Parallel.ForEach(files, file =>
        {
            var (ok, n, message) = VerifyFile(file);
            Interlocked.Add(ref lines, n);
            if (!ok)
            {
                Interlocked.Increment(ref failed);
                Console.Error.WriteLine($"MISMATCH {Path.GetFileName(file)}: {message}");
            }
        });

        Console.Error.WriteLine($"verify: {files.Length} files, {lines} lines, {failed} mismatching");
        return failed == 0 ? 0 : 1;
    }

    private static (bool Ok, int Lines, string? Message) VerifyFile(string file)
    {
        using var stream = OpenRead(file);
        using var reader = new StreamReader(stream);
        var tmp = Path.GetTempFileName();
        try
        {
            using var rec = new Recorder(tmp, new DumpPolicy("every", 0, true));
            Game? game = null;
            var n = 0;
            while (reader.ReadLine() is { } text)
            {
                n++;
                var line = JsonNode.Parse(text)!.AsObject();
                var kind = (string)line["kind"]!;
                if (kind == "header")
                {
                    var participation = ParseParticipation(line["participation"]!.AsObject());
                    game = new Game((int)line["gameVersion"]!, participation);
                    continue;
                }

                if (game == null) return (false, n, "no header");

                foreach (var q in line["queries"]!.AsArray())
                {
                    // ["bots", {...}] (trace format 2): a bot deliberation. The verifier doesn't run bots.
                    if (q![0] is JsonValue bv && bv.TryGetValue<string>(out var btag) && btag == "bots") continue;

                    if (q![0] is JsonValue v && v.TryGetValue<string>(out var tag) && tag == "status")
                    {
                        // ["status", player, isPlayer, result]: GameStatus.DetermineStatus, compared on the spot.
                        var sp = q[1] is null ? null : game.Players[(int)q[1]!];
                        var statusResult = JsonNode.Parse(rec.Status(game, sp, (bool)q[2]!));
                        if (!Same(q[3], statusResult))
                            return (false, n, $"line {n} ({kind} {line["n"]}): status of player {q[1]?.ToJsonString() ?? "null"} differs: expected {Short(q[3])}, got {Short(statusResult)}");
                        continue;
                    }

                    // [player, isHost] or, when the call threw, [player, isHost, exception]: compared exactly.
                    var p = q[0] is null ? null : game.Players[(int)q[0]!];
                    var expectedThrow = q.AsArray().Count > 2 ? q[2] : null;
                    JsonNode? actualThrow = null;
                    try
                    {
                        rec.Query(game, p, (bool)q[1]!);
                    }
                    catch (Exception e)
                    {
                        actualThrow = JsonNode.Parse(Recorder.ExceptionJson(e));
                    }

                    if (!Same(expectedThrow, actualThrow))
                        return (false, n, $"line {n} ({kind} {line["n"]}): GetApplicableEvents({q[0]?.ToJsonString() ?? "null"}, {q[1]}) differs: expected {(expectedThrow == null ? "no exception" : Short(expectedThrow))}, got {(actualThrow == null ? "no exception" : Short(actualThrow))}");
                }

                JsonObject produced;
                switch (kind)
                {
                    case "step":
                    {
                        var json = line["event"]!.ToJsonString();
                        var evt = Utilities.Deserialize<GameEvent>(json)!;
                        evt.Initialize(game);
                        var outcome = StepExecutor.Run(game, evt, (bool)line["validate"]!, (bool)line["isHost"]!);
                        string? after;
                        try
                        {
                            after = Utilities.Serialize<GameEvent>(evt);
                        }
                        catch
                        {
                            after = null;
                        }

                        produced = Capture(rec, tmp, r => r.Step(new StepInfo
                        {
                            Source = "verify", PerformValidation = (bool)line["validate"]!, IsHost = (bool)line["isHost"]!,
                            EventJson = json
                        }, outcome, game, after == json ? null : after, null));
                        break;
                    }
                    case "undo":
                        game = game.Undo((int)line["untilEventNr"]!);
                        produced = Capture(rec, tmp, r => r.SpecialStep(kind, (int)line["untilEventNr"]!, game, null));
                        break;
                    case "restoreRecentlyUndone":
                        game = game.RestoreRecentlyUndone();
                        produced = Capture(rec, tmp, r => r.SpecialStep(kind, null, game, null));
                        break;
                    case "dismissRecentlyUndone":
                        game = game.DismissRecentlyUndone();
                        produced = Capture(rec, tmp, r => r.SpecialStep(kind, null, game, null));
                        break;
                    case "end" when (string)line["reason"]! is "hung" or "botHung" or "undoHung":
                        // The oracle gave up on an Execute (or bot) that never returned; re-running it would hang here too.
                        return (true, n, null);
                    case "end":
                        produced = Capture(rec, tmp, r => r.End((string)line["reason"]!, null, game));
                        if (!Same(line["hash"], produced["hash"])) return (false, n, "final hash differs");
                        return (true, n, null);
                    default:
                        return (false, n, "unknown kind " + kind);
                }

                foreach (var key in ComparedStepKeys)
                {
                    if (key == "eventAfter" && kind != "step") continue;
                    var expected = line[key];
                    var actual = produced[key];
                    if (key == "eventAfter")
                    {
                        // The verifier re-serializes the event from a parsed node, so compare parsed values.
                        expected ??= line["event"];
                        actual ??= produced["event"];
                    }

                    if (key is "hash" or "state" && expected == null) continue;
                    if (!Same(expected, actual))
                        return (false, n, $"line {n} ({kind} {line["n"]}): {key} differs: expected {Short(expected)}, got {Short(actual)}");
                }
            }

            return (false, n, "no end line");
        }
        finally
        {
            File.Delete(tmp);
        }
    }

    /// <summary>Runs a recorder call and returns the line it wrote.</summary>
    private static JsonObject Capture(Recorder rec, string tmp, Action<Recorder> write)
    {
        rec.CaptureNext();
        write(rec);
        return JsonNode.Parse(rec.TakeCaptured())!.AsObject();
    }

    private static bool Same(JsonNode? a, JsonNode? b) => JsonNode.DeepEquals(a, b);

    private static string Short(JsonNode? n)
    {
        var s = n?.ToJsonString() ?? "null";
        return s.Length > 300 ? s[..300] + "..." : s;
    }

    private static Stream OpenRead(string file)
    {
        Stream s = File.OpenRead(file);
        return file.EndsWith(".gz") ? new GZipStream(s, CompressionMode.Decompress) : s;
    }

    /// <summary>Builds a Participation from its canonical dump (header field "participation").</summary>
    private static Participation ParseParticipation(JsonObject o)
    {
        var p = new Participation
        {
            BotsSpeed = (int)o["BotsSpeed"]!
        };
        foreach (var kv in o["PlayerNames"]!.AsArray()) p.PlayerNames.Add((int)kv![0]!, (string)kv[1]!);
        foreach (var kv in o["SeatedPlayers"]!.AsArray()) p.SeatedPlayers.Add((int)kv![0]!, (int)kv[1]!);
        foreach (var v in o["Observers"]!.AsArray()) p.Observers.Add((int)v!);
        foreach (var v in o["Hosts"]!.AsArray()) p.Hosts.Add((int)v!);
        foreach (var v in o["AvailableSeats"]!.AsArray()) p.AvailableSeats.Add((int)v!);
        foreach (var v in o["Kicked"]!.AsArray()) p.Kicked.Add((int)v!);
        return p;
    }
}
