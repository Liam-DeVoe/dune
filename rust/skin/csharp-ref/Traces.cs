// The trace corpus: replays oracle traces (trace-format.md section 6) and renders, with
// DefaultSkin.Default, every message the game produces along the way:
//
// - the message Execute returns (validation errors, "Game Error" messages),
// - GetMessage() and GetShortMessage() of every submitted event,
// - every message added to a report (including the pre-game report),
// - for status queries: both GameStatus descriptions and the FlashInfo messages,
// - at the end of the game: every faction's rules sheet (GetFactionInfo_HTML, GetFactionSheet)
//   and GetPopup(hero, game) for every leader.
//
// One JSON line per text: {"n": trace line number, "what": ..., "out": text}. The Rust test
// (tests/trace_corpus.rs) replays the same trace with the Rust engine and must produce the same
// lines in the same order. Stack traces are cut out of exception texts, since Rust has none.

using System.Text.Json.Nodes;
using System.Text.RegularExpressions;
using Treachery.Oracle;

public static class Traces
{
    private static readonly Skin S = DefaultSkin.Default;

    private static readonly FieldInfo PreGameReport =
        typeof(Game).GetField("_preGameReport", BindingFlags.Instance | BindingFlags.NonPublic)!;

    private static readonly PropertyInfo Awaited =
        typeof(GameStatus).GetProperty("DescriptionWhenAwaited", BindingFlags.Instance | BindingFlags.NonPublic)!;

    private static readonly PropertyInfo Waiting =
        typeof(GameStatus).GetProperty("DescriptionWhenWaiting", BindingFlags.Instance | BindingFlags.NonPublic)!;

    public static void Run(string input, string outDir)
    {
        var files = Directory.Exists(input)
            ? Directory.GetFiles(input).Where(f => f.EndsWith(".jsonl") || f.EndsWith(".jsonl.gz")).OrderBy(f => f, StringComparer.Ordinal).ToArray()
            : [input];
        Directory.CreateDirectory(outDir);
        foreach (var file in files)
        {
            var name = Path.GetFileName(file).Replace(".jsonl.gz", "").Replace(".jsonl", "");
            using var w = Output.Open(Path.Combine(outDir, name + ".skin.jsonl.gz"));
            var lines = Replay(file, w);
            Console.Error.WriteLine($"{name}: {lines} lines");
        }
    }

    private static string Clean(string s) => Regex.Replace(s, @"(\r?\n\s+at [^\r\n]*?\)( in [^\r\n]*?:line \d+)?|\r?\n\s+--- End of inner exception stack trace ---)+", "");

    private static int Replay(string file, StreamWriter w)
    {
        Stream stream = File.OpenRead(file);
        if (file.EndsWith(".gz")) stream = new System.IO.Compression.GZipStream(stream, System.IO.Compression.CompressionMode.Decompress);
        using var reader = new StreamReader(stream);

        Game? game = null;
        var n = 0;
        var seen = new List<int>();

        void Emit(string what, Func<string> f)
        {
            string result;
            try
            {
                result = Clean(f());
            }
            catch (Exception e)
            {
                result = "!" + e.GetType().Name + ": " + e.Message;
            }

            w.WriteLine(JsonSerializer.Serialize(new Dictionary<string, object> { ["n"] = n, ["what"] = what, ["out"] = result }));
        }

        List<Report> AllReports() => [(Report)PreGameReport.GetValue(game)!, .. game!.Reports];

        void NewReportMessages()
        {
            var reports = AllReports();
            for (var i = 0; i < reports.Count; i++)
            {
                if (seen.Count <= i) seen.Add(0);
                var k = 0;
                foreach (var m in reports[i].Messages)
                {
                    if (k++ < seen[i]) continue;
                    Emit($"report {i}", () => m.ToString(S));
                }

                seen[i] = reports[i].Messages.Count;
            }

            seen.RemoveRange(reports.Count, seen.Count - reports.Count);
        }

        void ApplicableCalls()
        {
            foreach (var p in game!.Players.ToList())
            {
                Try(() => game.GetApplicableEvents(p, false));
                Try(() => game.GetApplicableEvents(p, true));
            }

            Try(() => game.GetApplicableEvents(null, false));
            Try(() => game.GetApplicableEvents(null, true));
        }

        while (reader.ReadLine() is { } text)
        {
            n++;
            if (text.Length == 0) continue;
            var line = JsonNode.Parse(text)!.AsObject();
            var kind = (string)line["kind"]!;
            if (kind == "header")
            {
                game = new Game((int)line["gameVersion"]!, ParseParticipation(line["participation"]!.AsObject()));
                continue;
            }

            if (game == null) break;

            foreach (var q in line["queries"]?.AsArray() ?? [])
            {
                // A bot deliberation (trace format 2): skins don't run bots.
                if (q![0] is JsonValue bv && bv.TryGetValue<string>(out var btag) && btag == "bots") continue;

                if (q![0] is JsonValue v && v.TryGetValue<string>(out var tag) && tag == "status")
                {
                    var sp = q[1] is null ? null : game.Players[(int)q[1]!];
                    GameStatus? status = null;
                    try
                    {
                        status = GameStatus.DetermineStatus(game, sp, (bool)q[2]!);
                    }
                    catch (Exception e)
                    {
                        Emit("status", () => "!" + e.GetType().Name + ": " + e.Message);
                    }

                    if (status != null)
                    {
                        Emit("status awaited", () => ((Message)Awaited.GetValue(status)!).ToString(S));
                        Emit("status waiting", () => ((Message)Waiting.GetValue(status)!).ToString(S));
                        foreach (var fi in status.FlashInfo) Emit("flash", () => fi.Message.ToString(S));
                    }

                    continue;
                }

                var p = q[0] is null ? null : game.Players[(int)q[0]!];
                Try(() => game.GetApplicableEvents(p, (bool)q[1]!));
            }

            switch (kind)
            {
                case "step":
                {
                    var evt = Utilities.Deserialize<GameEvent>(line["event"]!.ToJsonString())!;
                    evt.Initialize(game);
                    var outcome = StepExecutor.Run(game, evt, (bool)line["validate"]!, (bool)line["isHost"]!);
                    if (outcome.Result != null) Emit("result", () => outcome.Result.ToString(S));
                    if (outcome.Escaped) Emit("escaped", () => outcome.Error!.GetType().Name + ": " + outcome.Error.Message);
                    Emit("GetMessage", () => evt.GetMessage().ToString(S));
                    Emit("GetShortMessage", () => evt.GetShortMessage().ToString(S));
                    NewReportMessages();
                    break;
                }
                case "undo":
                    game = game.Undo((int)line["untilEventNr"]!);
                    NewReportMessages();
                    break;
                case "restoreRecentlyUndone":
                    game = game.RestoreRecentlyUndone();
                    NewReportMessages();
                    break;
                case "dismissRecentlyUndone":
                    game = game.DismissRecentlyUndone();
                    NewReportMessages();
                    break;
                case "end":
                    if ((string)line["reason"]! is "hung" or "botHung" or "undoHung" or "memoryLimit" or "botMemoryLimit" or "undoMemoryLimit")
                        return n;
                    End(game, Emit);
                    return n;
            }

            ApplicableCalls();
        }

        return n;
    }

    private static void End(Game game, Action<string, Func<string>> emit)
    {
        foreach (var f in Enum.GetValues<Faction>())
        {
            emit($"sheet {f}", () => S.GetFactionInfo_HTML(game, f));
            emit($"sheet {f} styled", () => S.GetFactionInfo_HTML(game, f, "x"));
            emit($"sheet parts {f}", () =>
            {
                var sheet = S.GetFactionSheet(game, f);
                if (sheet == null) return "null";
                return string.Join("|", sheet.AtStart, sheet.Resources, sheet.FreeRevival, sheet.Theme,
                    string.Join(";", sheet.Advantages), string.Join(";", sheet.AdvancedAdvantages),
                    string.Join(";", sheet.Parameters.Select(p => S.Describe(p))));
            });
        }

        foreach (var l in LeaderManager.Leaders) emit($"popup {l.Id}", () => S.GetPopup(l, game));
        emit("popup messiah", () => S.GetPopup(LeaderManager.Messiah, game));
    }

    private static void Try(Action a)
    {
        try
        {
            a();
        }
        catch
        {
            // The oracle's drivers catch these too.
        }
    }

    private static Participation ParseParticipation(JsonObject o)
    {
        var p = new Participation { BotsSpeed = (int)o["BotsSpeed"]! };
        foreach (var kv in o["PlayerNames"]!.AsArray()) p.PlayerNames.Add((int)kv![0]!, (string)kv[1]!);
        foreach (var kv in o["SeatedPlayers"]!.AsArray()) p.SeatedPlayers.Add((int)kv![0]!, (int)kv[1]!);
        foreach (var v in o["Observers"]!.AsArray()) p.Observers.Add((int)v!);
        foreach (var v in o["Hosts"]!.AsArray()) p.Hosts.Add((int)v!);
        foreach (var v in o["AvailableSeats"]!.AsArray()) p.AvailableSeats.Add((int)v!);
        foreach (var v in o["Kicked"]!.AsArray()) p.Kicked.Add((int)v!);
        return p;
    }
}
