using System.Globalization;
using System.IO.Compression;
using System.Text.Json.Nodes;

namespace Treachery.ServerOracle;

public static class Program
{
    private const string Usage = """
        Treachery.ServerOracle: runs server scenarios through the C# GameHub and writes transcripts for the Rust
        port to compare against (rust/server-equiv; format in rust/docs/server-plan.md, "Equivalence testing").

        Usage:
          run      --in SCENARIO.jsonl|DIR --out DIR [--full true]
                   Writes DIR/<name>.transcript.jsonl for every scenario.
          generate --in TRACE.jsonl[.gz]|DIR --out DIR [--max-steps N] [--advance-seconds S] [--async-play true]
                   [--restart-every K]
                   Turns oracle traces into hub scenarios (N seated users play the trace's events through
                   SubmitEvent) and writes DIR/<name>.jsonl with its transcript.
          botgames --out DIR [--seed S] [--games N] [--start I] [--max-steps N]
                   Hub scenarios in which the server's bots play whole games, driven by the virtual clock.
                   Writes DIR/botgame-S-I.jsonl with its transcript.

        Run with TZ=Etc/GMT-2 (scenarios start at +02:00). rust/server-equiv/run-oracle.sh does this.
        """;

    public static int Main(string[] args)
    {
        if (args.Length == 0 || args[0] is "-h" or "--help")
        {
            Console.Error.WriteLine(Usage);
            return args.Length == 0 ? 1 : 0;
        }

        var opts = new Dictionary<string, string>();
        for (var i = 1; i + 1 < args.Length; i += 2)
        {
            if (!args[i].StartsWith("--")) return Fail("Unexpected argument " + args[i]);
            opts[args[i][2..]] = args[i + 1];
        }

        // As the server process: Program.Main sets the describer. The harness pins the culture.
        CultureInfo.DefaultThreadCurrentCulture = CultureInfo.InvariantCulture;
        CultureInfo.DefaultThreadCurrentUICulture = CultureInfo.InvariantCulture;
        CultureInfo.CurrentCulture = CultureInfo.InvariantCulture;
        CultureInfo.CurrentUICulture = CultureInfo.InvariantCulture;
        Message.DefaultDescriber = DefaultSkin.Default;

        // The hub logs with Console.WriteLine; keep that out of the way.
        var stdout = Console.Out;
        Console.SetOut(TextWriter.Null);
        try
        {
            return args[0] switch
            {
                "run" => Run(Required(opts, "in"), Required(opts, "out"), opts.GetValueOrDefault("full") == "true"),
                "generate" => Generator.Run(Required(opts, "in"), Required(opts, "out"), new GeneratorOptions
                {
                    MaxSteps = int.Parse(opts.GetValueOrDefault("max-steps", "100000")),
                    AdvanceSeconds = double.Parse(opts.GetValueOrDefault("advance-seconds", "20"), CultureInfo.InvariantCulture),
                    AsyncPlay = opts.GetValueOrDefault("async-play") == "true",
                    RestartEvery = int.Parse(opts.GetValueOrDefault("restart-every", "0")),
                }),
                "botgames" => BotGames.Run(Required(opts, "out"), new BotGameOptions
                {
                    Seed = int.Parse(opts.GetValueOrDefault("seed", "1")),
                    Games = int.Parse(opts.GetValueOrDefault("games", "1")),
                    Start = int.Parse(opts.GetValueOrDefault("start", "0")),
                    MaxSteps = int.Parse(opts.GetValueOrDefault("max-steps", "4000")),
                }),
                _ => Fail("Unknown command " + args[0] + "\n\n" + Usage),
            };
        }
        finally
        {
            Console.SetOut(stdout);
        }
    }

    private static string Required(Dictionary<string, string> opts, string key) =>
        opts.TryGetValue(key, out var v) ? v : throw new ArgumentException($"--{key} is required");

    private static int Fail(string message)
    {
        Console.Error.WriteLine(message);
        return 2;
    }

    public static IEnumerable<string> InputFiles(string path, string pattern) =>
        Directory.Exists(path)
            ? Directory.GetFiles(path, pattern).Where(f => !f.EndsWith(".transcript.jsonl", StringComparison.Ordinal)).Order(StringComparer.Ordinal)
            : [path];

    public static string[] ReadLines(string file)
    {
        using var stream = File.OpenRead(file);
        using var reader = file.EndsWith(".gz", StringComparison.Ordinal)
            ? new StreamReader(new GZipStream(stream, CompressionMode.Decompress))
            : new StreamReader(stream);
        return reader.ReadToEnd().Split('\n', StringSplitOptions.RemoveEmptyEntries);
    }

    public static string ScenarioName(string file)
    {
        var name = Path.GetFileName(file);
        foreach (var ext in new[] { ".gz", ".jsonl" })
            if (name.EndsWith(ext, StringComparison.Ordinal)) name = name[..^ext.Length];
        return name;
    }

    private static int Run(string input, string outDir, bool full)
    {
        Directory.CreateDirectory(outDir);
        var failed = 0;
        foreach (var file in InputFiles(input, "*.jsonl"))
        {
            var name = ScenarioName(file);
            try
            {
                var bytes = File.ReadAllBytes(file);
                var lines = Encoding.UTF8.GetString(bytes).Split('\n', StringSplitOptions.RemoveEmptyEntries);
                var header = JsonNode.Parse(lines[0])!.AsObject();
                using var harness = new Harness(header, full);
                var transcript = new List<string> { TranscriptHeader(name, JsonText.Sha256(Encoding.UTF8.GetString(bytes)), full || header["full"]?.GetValue<bool>() == true) };
                foreach (var line in lines.Skip(1))
                    transcript.Add(harness.Step(JsonNode.Parse(line)!.AsObject()).Line);
                File.WriteAllText(Path.Combine(outDir, name + ".transcript.jsonl"), string.Join("\n", transcript) + "\n");
                Console.Error.WriteLine($"{name}: {lines.Length - 1} steps");
            }
            catch (Exception e)
            {
                failed++;
                Console.Error.WriteLine($"{name}: FAILED: {e}");
            }
        }

        return failed == 0 ? 0 : 1;
    }

    public static string TranscriptHeader(string name, string scenarioSha256, bool full) =>
        new JsonText().ObjStart()
            .MemberSer("transcript", Harness.TranscriptFormat)
            .Member("scenario", name)
            .Member("scenarioSha256", scenarioSha256)
            .MemberSer("full", full)
            .ObjEnd().ToString();
}
