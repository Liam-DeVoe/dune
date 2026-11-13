using System.Diagnostics;
using System.Globalization;
using Treachery.Oracle.Canon;

namespace Treachery.Oracle;

public static class Program
{
    private const string Usage = """
        Treachery.Oracle: records engine traces for the Rust port. Format: rust/docs/trace-format.md.

        Usage:
          bots   --seed S --games N --out DIR [--start I] [--dump every|every:N|final|none] [--hash every|none] [--threads T] [--gzip true] [--check-bot-purity true] [--discard true] [--hang-seconds 120] [--status every] [--memory-mb 2048] [--versions all|latest]
          fuzz   --seed S --games N --out DIR [--start I] [--dump ...] [--hash ...] [--threads T] [--gzip true] [--check-bot-purity true] [--discard true] [--hang-seconds 120] [--status every] [--fuzz-profile client|wide] [--memory-mb 2048] [--versions all|latest]
          replay --in SAVEGAME.json --out DIR [--dump ...] [--hash ...]
          verify --in FILE|DIR   (replays traces from their contents alone and checks every recorded value)
          enums  --out FILE      (writes the enum name table used by the state dump)
          coverage --in COBERTURA.xml [--top K]
                 (summarizes a coverage report; Oracle/coverage.sh collects one over a bots + fuzz run)

        Defaults: --dump final --hash every --start 0 --threads <processor count>.
        Game i of a run is fully determined by (mode, S, i); --start lets a run continue where another stopped.
        """;

    public static int Main(string[] args)
    {
        if (args.Length == 0 || args[0] is "-h" or "--help")
        {
            Console.Error.WriteLine(Usage);
            return args.Length == 0 ? 1 : 0;
        }

        var opts = ParseOptions(args.Skip(1).ToArray());
        SelfCheck();

        // The engine formats and parses numbers with the current culture. Pin it (the project also sets InvariantGlobalization).
        CultureInfo.DefaultThreadCurrentCulture = CultureInfo.InvariantCulture;
        CultureInfo.DefaultThreadCurrentUICulture = CultureInfo.InvariantCulture;
        CultureInfo.CurrentCulture = CultureInfo.InvariantCulture;
        CultureInfo.CurrentUICulture = CultureInfo.InvariantCulture;

        // Message.DefaultDescriber changes ToString() of model objects, which reaches exception messages. It must stay null.
        if (Message.DefaultDescriber != null) throw new InvalidOperationException("Message.DefaultDescriber must be null in the oracle");
        var stdout = Console.Out;

        // Bots write diagnostics to Console.Out; keep them out of our output and off the shared console lock.
        if (args[0] is "bots" or "fuzz" or "replay" or "verify") Console.SetOut(TextWriter.Null);

        try
        {
            return args[0] switch
            {
                "bots" => RunGames("bots", opts, null),
                "fuzz" => RunGames("fuzz", opts, new FuzzSettings
                {
                    ClientOnly = opts.GetValueOrDefault("fuzz-profile", "client") switch
                    {
                        "client" => true,
                        "wide" => false,
                        _ => throw new ArgumentException("--fuzz-profile must be client or wide")
                    }
                }),
                "replay" => Replay(opts),
                "enums" => EnumTable.Write(Required(opts, "out")),
                "verify" => Verifier.Run(Required(opts, "in")),
                "coverage" => CoverageReport.Print(Required(opts, "in"), int.Parse(opts.GetValueOrDefault("top", "25"))),
                _ => Fail("Unknown command " + args[0] + "\n\n" + Usage)
            };
        }
        finally
        {
            Console.SetOut(stdout);
        }
    }

    /// <summary>The dump identifies locations and territories by id; make sure ids are unique and locations are indexed by id.</summary>
    private static void SelfCheck()
    {
        var map = new Game().Map;
        var locations = StateDumper.MapLocations(map);
        for (var i = 0; i < locations.Count; i++)
            if (locations[i].Id != i) throw new InvalidOperationException($"Location at index {i} has id {locations[i].Id}");
        var territories = locations.Select(l => l.Territory).Distinct().ToList();
        if (territories.Select(t => t.Id).Distinct().Count() != territories.Count) throw new InvalidOperationException("Territory ids are not unique");
        if (LeaderManager.Leaders.Select(l => l.Id).Append(LeaderManager.Messiah.Id).Distinct().Count() != LeaderManager.Leaders.Count + 1) throw new InvalidOperationException("Hero ids are not unique");
        var cards = TreacheryCardManager.GetCardsInAndOutsidePlay().ToList();
        if (cards.Select(c => c.Id).Distinct().Count() != cards.Count) throw new InvalidOperationException("Card ids are not unique");
    }

    private static int Fail(string msg)
    {
        Console.Error.WriteLine(msg);
        return 2;
    }

    private static Dictionary<string, string> ParseOptions(string[] args)
    {
        var result = new Dictionary<string, string>();
        for (var i = 0; i < args.Length; i++)
        {
            if (!args[i].StartsWith("--")) throw new ArgumentException("Unexpected argument " + args[i]);
            var key = args[i][2..];
            if (i + 1 >= args.Length) throw new ArgumentException("Missing value for --" + key);
            result[key] = args[++i];
        }

        return result;
    }

    private static string Required(Dictionary<string, string> opts, string key) =>
        opts.TryGetValue(key, out var v) ? v : throw new ArgumentException("Missing --" + key);

    private static int RunGames(string mode, Dictionary<string, string> opts, FuzzSettings? fuzz)
    {
        var seed = int.Parse(Required(opts, "seed"));
        var games = int.Parse(Required(opts, "games"));
        var start = int.Parse(opts.GetValueOrDefault("start", "0"));
        var outDir = Required(opts, "out");
        var threads = int.Parse(opts.GetValueOrDefault("threads", Environment.ProcessorCount.ToString()));
        var policy = DumpPolicy.Parse(opts.GetValueOrDefault("dump"), opts.GetValueOrDefault("hash")) with
        {
            StatusEvery = opts.GetValueOrDefault("status", "none") switch
            {
                "every" => true,
                "none" => false,
                _ => throw new ArgumentException("--status must be every or none")
            }
        };
        var gzip = opts.GetValueOrDefault("gzip", "false") == "true";
        var checkPurity = opts.GetValueOrDefault("check-bot-purity", "false") == "true";
        var discard = opts.GetValueOrDefault("discard", "false") == "true";
        // --versions latest: every game is played at Game.LatestVersion (the Rust engine no longer supports older versions).
        var latestOnly = opts.GetValueOrDefault("versions", "all") switch
        {
            "all" => false,
            "latest" => true,
            _ => throw new ArgumentException("--versions must be all or latest")
        };
        Directory.CreateDirectory(outDir);

        var hangSeconds = int.Parse(opts.GetValueOrDefault("hang-seconds", "120"));
        var memoryMb = long.Parse(opts.GetValueOrDefault("memory-mb", "2048"));
        var summaries = new GameSummary[games];
        var running = new System.Collections.Concurrent.ConcurrentDictionary<int, GameRunner>();
        var sw = Stopwatch.StartNew();
        var done = 0;
        var hung = 0;
        var work = Task.Run(() => Parallel.ForEach(
            // One game at a time per worker, so a hung game never holds back games queued behind it.
            System.Collections.Concurrent.Partitioner.Create(Enumerable.Range(0, games), System.Collections.Concurrent.EnumerablePartitionerOptions.NoBuffering),
            new ParallelOptions { MaxDegreeOfParallelism = threads }, i =>
        {
            var index = start + i;
            GameSummary s;
            try
            {
                var runner = new GameRunner(mode, seed, index, fuzz, checkPurity, latestOnly);
                running[i] = runner;
                s = runner.Run(outDir, policy, gzip, discard);
            }
            catch (GameRunner.GameHungException)
            {
                return;
            }
            catch (Exception e)
            {
                s = new GameSummary(index, "", "oracleCrash", e.ToString(), 0, 0, 0, 0, "");
            }
            finally
            {
                running.TryRemove(i, out _);
            }

            summaries[i] = s;
            var n = Interlocked.Increment(ref done);
            if (n % 50 == 0 || n == games)
                Console.Error.WriteLine($"{mode}: {n}/{games} games, {sw.Elapsed.TotalSeconds:F0}s");
        }));

        // Watchdog: a game stuck inside Execute can't be stopped, so record it as hung and stop waiting for it.
        while (!work.Wait(1000))
        {
            foreach (var (i, runner) in running)
            {
                if (!runner.TryMarkHung(hangSeconds, out var reason)) continue;
                Interlocked.Increment(ref hung);
                summaries[i] = new GameSummary(runner.Index, runner.FileName, reason, $"did not return within {hangSeconds} s", 0, 0, 0, 0, "");
                Console.Error.WriteLine($"{mode}: game {runner.Index} hung inside Execute; continuing without it");
            }

            if (hung > 0 && Volatile.Read(ref done) + hung >= games) break;

            // Memory guard. A .NET thread can't be stopped and per-thread memory can't be measured, so: when the managed
            // heap goes over --memory-mb, blame the game whose current engine call has run longest, end its trace with
            // reason memoryLimit (or botMemoryLimit, undoMemoryLimit), and exit with 5. Games still running lose their
            // traces; differ live reruns them one at a time.
            // GetTotalMemory(false) includes garbage the server GC hasn't collected yet; confirm with a full collection.
            var heapMb = GC.GetTotalMemory(false) >> 20;
            if (memoryMb > 0 && heapMb > memoryMb) heapMb = GC.GetTotalMemory(true) >> 20;
            if (memoryMb > 0 && heapMb > memoryMb)
            {
                var culprit = running.Values.Where(r => r.WatchedSeconds != null).OrderByDescending(r => r.WatchedSeconds).FirstOrDefault();
                var detail = $"the oracle's heap reached {heapMb} MB (--memory-mb {memoryMb}) during this call";
                if (culprit != null && culprit.TryMarkMemoryLimit(detail, out var reason))
                {
                    Console.Error.WriteLine($"{mode}: game {culprit.Index} went over {memoryMb} MB; stopping the run");
                    summaries[culprit.Index - start] = new GameSummary(culprit.Index, culprit.FileName, reason, detail, 0, 0, 0, 0, "");
                }
                else
                {
                    Console.Error.WriteLine($"{mode}: the heap reached {heapMb} MB (--memory-mb {memoryMb}) outside any engine call; stopping the run");
                }

                for (var i = 0; i < summaries.Length; i++)
                    summaries[i] ??= new GameSummary(start + i, "", "notRun", "the run was stopped for memory", 0, 0, 0, 0, "");
                WriteSummary(Path.Combine(outDir, $"summary-{mode}-{seed}-{start}.json"), mode, seed, start, summaries);
                Console.Out.Flush();
                Console.Error.Flush();
                Environment.Exit(5);
            }
        }

        WriteSummary(Path.Combine(outDir, $"summary-{mode}-{seed}-{start}.json"), mode, seed, start, summaries);

        var elapsed = sw.Elapsed;
        var steps = summaries.Sum(s => (long)s.Steps);
        Console.Error.WriteLine($"{mode}: {games} games, {steps} steps in {elapsed.TotalSeconds:F1}s " +
                                $"({games / elapsed.TotalHours:F0} games/hour, {steps / elapsed.TotalSeconds:F0} steps/s) on {threads} threads");
        foreach (var g in summaries.GroupBy(s => s.EndReason).OrderBy(g => g.Key))
            Console.Error.WriteLine($"  {g.Key}: {g.Count()}");
        if (!StateDumper.UnsupportedTypes.IsEmpty)
            Console.Error.WriteLine("  WARNING unsupported types in dumps: " + string.Join(", ", StateDumper.UnsupportedTypes.Keys));

        if (hung > 0)
        {
            // The hung threads never finish, so leave without waiting for them.
            Console.Out.Flush();
            Console.Error.Flush();
            Environment.Exit(4);
        }

        return summaries.Any(s => s.EndReason == "oracleCrash") ? 3 : 0;
    }

    private static void WriteSummary(string path, string mode, int seed, int start, GameSummary[] summaries)
    {
        var o = new JsonOut();
        o.ObjStart();
        o.Key("mode"); o.WriteString(mode);
        o.Comma(); o.Key("masterSeed"); o.Int(seed);
        o.Comma(); o.Key("start"); o.Int(start);
        o.Comma(); o.Key("endReasons");
        o.ObjStart();
        var first = true;
        foreach (var g in summaries.GroupBy(s => s.EndReason).OrderBy(g => g.Key, StringComparer.Ordinal))
        {
            if (!first) o.Comma();
            first = false;
            o.Key(g.Key);
            o.Int(g.Count());
        }
        o.ObjEnd();
        o.Comma(); o.Key("games");
        o.ArrStart();
        for (var i = 0; i < summaries.Length; i++)
        {
            var s = summaries[i];
            if (i > 0) o.Comma();
            o.ObjStart();
            o.Key("index"); o.Int(s.Index);
            o.Comma(); o.Key("file"); o.WriteString(s.File);
            o.Comma(); o.Key("endReason"); o.WriteString(s.EndReason);
            o.Comma(); o.Key("detail"); o.WriteString(s.Detail);
            o.Comma(); o.Key("steps"); o.Int(s.Steps);
            o.Comma(); o.Key("historyCount"); o.Int(s.HistoryCount);
            o.Comma(); o.Key("version"); o.Int(s.Version);
            o.Comma(); o.Key("players"); o.Int(s.Players);
            o.Comma(); o.Key("hostStyle"); o.WriteString(s.HostStyle);
            o.ObjEnd();
        }
        o.ArrEnd();
        o.ObjEnd();
        o.Raw((byte)'\n');
        File.WriteAllBytes(path, o.WrittenSpan.ToArray());
    }

    private static int Replay(Dictionary<string, string> opts)
    {
        var input = Required(opts, "in");
        var outDir = Required(opts, "out");
        var policy = DumpPolicy.Parse(opts.GetValueOrDefault("dump"), opts.GetValueOrDefault("hash"));
        Directory.CreateDirectory(outDir);
        var name = Path.GetFileNameWithoutExtension(input);
        var reason = SavegameReplayer.Run(input, Path.Combine(outDir, "replay-" + name + ".jsonl"), policy);
        Console.Error.WriteLine($"replay {input}: {reason}");
        return 0;
    }
}
