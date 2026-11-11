using Treachery.Oracle.Canon;

namespace Treachery.Oracle;

/// <summary>When to write full state dumps and hashes. See rust/docs/trace-format.md, "Dump policy".</summary>
public sealed record DumpPolicy(string State, int Every, bool HashEveryStep)
{
    /// <summary>--status every: call GameStatus.DetermineStatus for every player after every line and record the results as queries.</summary>
    public bool StatusEvery { get; init; }

    public static DumpPolicy Parse(string? state, string? hash)
    {
        state ??= "final";
        var every = 0;
        if (state.StartsWith("every:", StringComparison.Ordinal))
        {
            every = int.Parse(state["every:".Length..]);
            if (every <= 0) throw new ArgumentException("--dump every:N needs N > 0");
            state = "everyN";
        }
        else if (state is not ("every" or "final" or "none"))
        {
            throw new ArgumentException("--dump must be every, every:N, final or none");
        }

        var hashEvery = (hash ?? "every") switch
        {
            "every" => true,
            "none" => false,
            _ => throw new ArgumentException("--hash must be every or none")
        };
        return new DumpPolicy(state, every, hashEvery);
    }

    public bool FullDumpAt(int stepIndex) => State switch
    {
        "every" => true,
        "everyN" => stepIndex % Every == 0,
        _ => false
    };

    public bool FinalDump => State != "none";

    public string Describe() => State == "everyN" ? "every:" + Every : State;
}

/// <summary>Information about one submitted event, gathered by a driver.</summary>
public sealed class StepInfo
{
    public required string Source { get; init; }
    public int? Actor { get; init; }
    public required bool PerformValidation { get; init; }
    public required bool IsHost { get; init; }
    public required string EventJson { get; init; }
    public string? MutatedField { get; init; }
    public BotInfo? Bot { get; init; }
    public int[]? BotOrder { get; init; }
}

public sealed class BotInfo
{
    public required Faction Faction { get; init; }
    public required int Seed { get; init; }
    public required string Priority { get; init; }
    public required int Player { get; init; }
}

/// <summary>Writes one trace file (JSON Lines).</summary>
public sealed class Recorder : IDisposable
{
    private readonly Stream _out;
    private readonly DumpPolicy _policy;
    private readonly JsonOut _line = new(1 << 20);
    private readonly JsonOut _state = new(1 << 20);
    private int _stepIndex;

    public Recorder(string path, DumpPolicy policy)
    {
        if (path == DiscardPath)
        {
            _out = Stream.Null;
            _policy = policy;
            return;
        }

        Stream file = new FileStream(path, FileMode.Create, FileAccess.Write, FileShare.Read, 1 << 16);
        if (path.EndsWith(".gz", StringComparison.Ordinal))
            file = new System.IO.Compression.GZipStream(file, System.IO.Compression.CompressionLevel.SmallestSize);
        _out = new BufferedStream(file, 1 << 20);
        _policy = policy;
    }

    public DumpPolicy Policy => _policy;

    /// <summary>Pass as the path to compute everything but write nothing (benchmarks, coverage runs).</summary>
    public const string DiscardPath = "\0discard";

    // GetApplicableEvents calls (StatusJson null) and GameStatus.DetermineStatus calls (StatusJson is the result) since the last line.
    // A "bots" entry (trace format 2) has Raw set: the whole entry, written verbatim.
    private readonly List<(int? Player, bool Flag, string? StatusJson, string? ExceptionJson, string? Raw)> _queries = [];
    private bool _botMutatedState;

    /// <summary>
    /// Calls Game.GetApplicableEvents on behalf of a driver and remembers the call, so the next step can list it under "queries".
    /// GetApplicableEvents can change state (it reads Game.NextPlayerToBattle), so a replayer must repeat these calls.
    /// If the call throws, the entry records the exception and the exception is rethrown to the driver.
    /// </summary>
    public List<Type> Query(Game game, Player? player, bool isHost)
    {
        int? index = player == null ? null : game.Players.IndexOf(player);
        try
        {
            var result = game.GetApplicableEvents(player, isHost);
            _queries.Add((index, isHost, null, null, null));
            return result;
        }
        catch (Exception e)
        {
            _queries.Add((index, isHost, null, ExceptionJson(e), null));
            throw;
        }
    }

    /// <summary>An exception from GetApplicableEvents as written in "applicable" and in a query entry (trace-format.md 7.4).</summary>
    public static string ExceptionJson(Exception e)
    {
        var o = new JsonOut();
        WriteTypes(o, e);
        return o.ToUtf8String();
    }

    /// <summary>
    /// Calls GameStatus.DetermineStatus as the client and server do, and remembers the call and its canonical result for
    /// the next line's "queries". DetermineStatus can change state (it reads Game.NextPlayerToBattle), so a replayer
    /// must repeat these calls in order. Returns the recorded result.
    /// </summary>
    public string Status(Game game, Player? player, bool isPlayer)
    {
        int? index = player == null ? null : game.Players.IndexOf(player);
        var o = new JsonOut();
        try
        {
            var status = GameStatus.DetermineStatus(game, player, isPlayer);
            new StateDumper(o).DumpDetached(status, game);
        }
        catch (Exception e)
        {
            o.Clear();
            new StateDumper(o).DumpDetached(e, game);
        }

        var json = o.ToUtf8String();
        _queries.Add((index, isPlayer, json, null, null));
        return json;
    }

    /// <summary>
    /// Records one bot deliberation as a <c>["bots", {...}]</c> query entry (trace format 2, trace-format.md section 3):
    /// the order in which players' bots were asked, whether the End level was asked, the bots created during it as
    /// (faction, player, seed), the event that was chosen (serialized before Submit sets its Time), and the exception
    /// if deliberation threw.
    /// </summary>
    public void Bots(int[] order, bool end, IReadOnlyList<(Faction Faction, int Player, int Seed)> created,
        (int Player, Faction Faction, string Priority, string EventJson)? choice, Exception? exception)
    {
        var o = new JsonOut();
        o.ArrStart();
        o.WriteString("bots");
        o.Comma();
        o.ObjStart();
        o.Key("order");
        o.ArrStart();
        for (var i = 0; i < order.Length; i++)
        {
            if (i > 0) o.Comma();
            o.Int(order[i]);
        }
        o.ArrEnd();
        o.Comma(); o.Key("end"); o.Bool(end);
        o.Comma(); o.Key("created");
        o.ArrStart();
        for (var i = 0; i < created.Count; i++)
        {
            if (i > 0) o.Comma();
            o.ArrStart();
            WriteEnum(o, created[i].Faction);
            o.Comma(); o.Int(created[i].Player);
            o.Comma(); o.Int(created[i].Seed);
            o.ArrEnd();
        }
        o.ArrEnd();
        o.Comma(); o.Key("choice");
        if (choice is { } c)
        {
            o.ObjStart();
            o.Key("player"); o.Int(c.Player);
            o.Comma(); o.Key("faction"); WriteEnum(o, c.Faction);
            o.Comma(); o.Key("priority"); o.WriteString(c.Priority);
            o.Comma(); o.Key("event"); o.RawJson(c.EventJson);
            o.ObjEnd();
        }
        else
        {
            o.Null();
        }

        if (exception != null)
        {
            o.Comma(); o.Key("exception"); WriteTypes(o, exception);
        }

        o.ObjEnd();
        o.ArrEnd();
        _queries.Add((null, false, null, null, o.ToUtf8String()));
    }

    /// <summary>Notes that bot deliberation changed the state hash; reported on the next step.</summary>
    public void FlagBotMutatedState() => _botMutatedState = true;

    // Bot asks that changed the state hash since the last line: player, faction, level, type of the returned event.
    private readonly List<(int Player, Faction Faction, string Level, string? EventType)> _botMutations = [];

    /// <summary>Notes which bot ask changed the state hash (--check-bot-purity true); reported on the next step as botMutatedBy.</summary>
    public void NoteBotMutation(int player, Faction faction, string level, string? eventType) => _botMutations.Add((player, faction, level, eventType));

    private void WriteQueries()
    {
        _line.Comma(); _line.Key("queries");
        _line.ArrStart();
        for (var i = 0; i < _queries.Count; i++)
        {
            if (i > 0) _line.Comma();
            if (_queries[i].Raw is { } raw)
            {
                _line.RawJson(raw);
                continue;
            }

            _line.ArrStart();
            if (_queries[i].StatusJson != null)
            {
                _line.WriteString("status");
                _line.Comma();
            }

            if (_queries[i].Player is { } p) _line.Int(p); else _line.Null();
            _line.Comma();
            _line.Bool(_queries[i].Flag);
            if (_queries[i].ExceptionJson is { } ex)
            {
                _line.Comma();
                _line.RawJson(ex);
            }

            if (_queries[i].StatusJson is { } st)
            {
                _line.Comma();
                _line.RawJson(st);
            }

            _line.ArrEnd();
        }
        _line.ArrEnd();
        _queries.Clear();

        if (_botMutatedState)
        {
            _line.Comma(); _line.Key("botMutatedState"); _line.Bool(true);
            _botMutatedState = false;
        }

        if (_botMutations.Count > 0)
        {
            _line.Comma(); _line.Key("botMutatedBy");
            _line.ArrStart();
            for (var i = 0; i < _botMutations.Count; i++)
            {
                var m = _botMutations[i];
                if (i > 0) _line.Comma();
                _line.ArrStart();
                _line.Int(m.Player);
                _line.Comma(); WriteEnum(_line, m.Faction);
                _line.Comma(); _line.WriteString(m.Level);
                _line.Comma();
                if (m.EventType != null) _line.WriteString(m.EventType); else _line.Null();
                _line.ArrEnd();
            }
            _line.ArrEnd();
            _botMutations.Clear();
        }
    }

    public int StepCount => _stepIndex;

    public void Header(Action<JsonOut> fields)
    {
        _line.Clear();
        _line.ObjStart();
        _line.Key("kind"); _line.WriteString("header");
        _line.Comma(); _line.Key("traceFormat"); _line.Int(Trace.FormatVersion);
        _line.Comma(); _line.Key("dumpFormat"); _line.Int(StateDumper.FormatVersion);
        _line.Comma(); _line.Key("dumpPolicy");
        _line.ObjStart();
        _line.Key("state"); _line.WriteString(_policy.Describe());
        _line.Comma(); _line.Key("hash"); _line.WriteString(_policy.HashEveryStep ? "every" : "none");
        if (_policy.StatusEvery)
        {
            _line.Comma(); _line.Key("status"); _line.WriteString("every");
        }

        _line.ObjEnd();
        fields(_line);
        _line.ObjEnd();
        Flush();
    }

    /// <summary>Records an event step. Call after the event was executed.</summary>
    public void Step(StepInfo info, StepOutcome outcome, Game game, string? eventJsonAfter, IReadOnlyDictionary<Faction, int>? botRandomCounts)
    {
        var n = _stepIndex++;
        _line.Clear();
        _line.ObjStart();
        _line.Key("kind"); _line.WriteString("step");
        _line.Comma(); _line.Key("n"); _line.Int(n);
        _line.Comma(); _line.Key("source"); _line.WriteString(info.Source);
        _line.Comma(); _line.Key("actor");
        if (info.Actor is { } a) _line.Int(a); else _line.Null();
        _line.Comma(); _line.Key("validate"); _line.Bool(info.PerformValidation);
        _line.Comma(); _line.Key("isHost"); _line.Bool(info.IsHost);
        if (info.MutatedField != null)
        {
            _line.Comma(); _line.Key("mutatedField"); _line.WriteString(info.MutatedField);
        }

        if (info.Bot != null)
        {
            _line.Comma(); _line.Key("bot");
            _line.ObjStart();
            _line.Key("faction"); WriteEnum(_line, info.Bot.Faction);
            _line.Comma(); _line.Key("player"); _line.Int(info.Bot.Player);
            _line.Comma(); _line.Key("priority"); _line.WriteString(info.Bot.Priority);
            _line.Comma(); _line.Key("seed"); _line.Int(info.Bot.Seed);
            _line.ObjEnd();
        }

        if (info.BotOrder != null)
        {
            _line.Comma(); _line.Key("botOrder");
            _line.ArrStart();
            for (var i = 0; i < info.BotOrder.Length; i++)
            {
                if (i > 0) _line.Comma();
                _line.Int(info.BotOrder[i]);
            }
            _line.ArrEnd();
        }

        WriteQueries();
        _line.Comma(); _line.Key("event"); _line.RawJson(info.EventJson);
        if (eventJsonAfter != null && eventJsonAfter != info.EventJson)
        {
            _line.Comma(); _line.Key("eventAfter"); _line.RawJson(eventJsonAfter);
        }

        _line.Comma(); _line.Key("result");
        _line.WriteString(outcome.Accepted ? "accepted" : outcome.Error != null ? "error" : "rejected");

        _line.Comma(); _line.Key("validation");
        if (outcome.Result == null) _line.Null();
        else new StateDumper(_line).DumpDetached(outcome.Result, game);

        _line.Comma(); _line.Key("exception");
        WriteException(outcome);

        WriteAfterState(game, n, botRandomCounts);
        _line.ObjEnd();
        Flush();
    }

    /// <summary>Records an Undo / RestoreRecentlyUndone / DismissRecentlyUndone step. <paramref name="game"/> is the resulting game.</summary>
    public void SpecialStep(string kind, int? until, Game game, IReadOnlyDictionary<Faction, int>? botRandomCounts)
    {
        var n = _stepIndex++;
        _line.Clear();
        _line.ObjStart();
        _line.Key("kind"); _line.WriteString(kind);
        _line.Comma(); _line.Key("n"); _line.Int(n);
        if (until != null)
        {
            _line.Comma(); _line.Key("untilEventNr"); _line.Int(until.Value);
        }

        WriteQueries();

        WriteAfterState(game, n, botRandomCounts);
        _line.ObjEnd();
        Flush();
    }

    public void End(string reason, string? detail, Game? game, Action<JsonOut>? extra = null)
    {
        _line.Clear();
        _line.ObjStart();
        _line.Key("kind"); _line.WriteString("end");
        _line.Comma(); _line.Key("reason"); _line.WriteString(reason);
        _line.Comma(); _line.Key("detail"); _line.WriteString(detail);
        _line.Comma(); _line.Key("steps"); _line.Int(_stepIndex);
        WriteQueries();
        if (game != null)
        {
            _line.Comma(); _line.Key("historyCount"); _line.Int(game.History.Count);
            _line.Comma(); _line.Key("phase"); WriteEnum(_line, game.CurrentPhase);
            _line.Comma(); _line.Key("turn"); _line.Int(game.CurrentTurn);
            _line.Comma(); _line.Key("winners");
            _line.ArrStart();
            var first = true;
            foreach (var w in SafeWinners(game))
            {
                if (!first) _line.Comma();
                first = false;
                WriteEnum(_line, w);
            }
            _line.ArrEnd();

            _state.Clear();
            new StateDumper(_state).DumpGame(game);
            _line.Comma(); _line.Key("hash"); _line.WriteString(StateDumper.Sha256Hex(_state.WrittenSpan));
            if (_policy.FinalDump)
            {
                _line.Comma(); _line.Key("state"); _line.Raw(_state.WrittenSpan);
            }
        }

        extra?.Invoke(_line);
        _line.ObjEnd();
        Flush();
    }

    private static IEnumerable<Faction> SafeWinners(Game game)
    {
        try
        {
            return game.Winners.Select(p => p.Faction).ToList();
        }
        catch
        {
            return [];
        }
    }

    private void WriteAfterState(Game game, int n, IReadOnlyDictionary<Faction, int>? botRandomCounts)
    {
        _line.Comma(); _line.Key("historyCount"); _line.Int(game.History.Count);
        _line.Comma(); _line.Key("random");
        var rnd = StateDumper.GameRandom(game);
        if (rnd == null) _line.Null(); else _line.Int(rnd.GetCount());

        if (botRandomCounts != null)
        {
            _line.Comma(); _line.Key("botRandom");
            _line.ArrStart();
            var first = true;
            foreach (var kv in botRandomCounts)
            {
                if (!first) _line.Comma();
                first = false;
                _line.ArrStart();
                WriteEnum(_line, kv.Key);
                _line.Comma();
                _line.Int(kv.Value);
                _line.ArrEnd();
            }
            _line.ArrEnd();
        }

        WriteApplicable(game);

        var full = _policy.FullDumpAt(n);
        if (full || _policy.HashEveryStep)
        {
            _state.Clear();
            new StateDumper(_state).DumpGame(game);
            _line.Comma(); _line.Key("hash"); _line.WriteString(StateDumper.Sha256Hex(_state.WrittenSpan));
            if (full)
            {
                _line.Comma(); _line.Key("state"); _line.Raw(_state.WrittenSpan);
            }
        }
    }

    private void WriteException(StepOutcome outcome)
    {
        if (outcome.Error == null)
        {
            _line.Null();
            return;
        }

        _line.ObjStart();
        _line.Key("phase"); _line.WriteString(outcome.ErrorPhase);
        _line.Comma(); _line.Key("escaped"); _line.Bool(outcome.Escaped);
        _line.Comma(); _line.Key("type"); _line.WriteString(outcome.Error.GetType().FullName);
        _line.Comma(); _line.Key("message"); _line.WriteString(outcome.Error.Message);
        _line.ObjEnd();
    }

    private readonly Dictionary<string, int> _typeLists = new(StringComparer.Ordinal);
    private readonly JsonOut _scratch = new(1 << 12);

    /// <summary>
    /// Writes "typeLists" (definitions of lists not seen before in this file) followed by "applicable":
    /// one [nonHost, host] pair of list indices per player in Game.Players order, then one pair for player null.
    /// </summary>
    private void WriteApplicable(Game game)
    {
        var count = game.Players.Count + 1;
        var indices = new int[count * 2];
        var fresh = new List<(int Index, byte[] Json)>();
        for (var i = 0; i < count; i++)
        {
            var p = i < game.Players.Count ? game.Players[i] : null;
            indices[2 * i] = Intern(Applicable(game, p, false), fresh);
            indices[2 * i + 1] = Intern(Applicable(game, p, true), fresh);
        }

        if (fresh.Count > 0)
        {
            _line.Comma(); _line.Key("typeLists");
            _line.ArrStart();
            for (var i = 0; i < fresh.Count; i++)
            {
                if (i > 0) _line.Comma();
                _line.ArrStart();
                _line.Int(fresh[i].Index);
                _line.Comma();
                _line.Raw(fresh[i].Json);
                _line.ArrEnd();
            }
            _line.ArrEnd();
        }

        _line.Comma(); _line.Key("applicable");
        _line.ArrStart();
        for (var i = 0; i < count; i++)
        {
            if (i > 0) _line.Comma();
            _line.ArrStart();
            _line.Int(indices[2 * i]);
            _line.Comma();
            _line.Int(indices[2 * i + 1]);
            _line.ArrEnd();
        }
        _line.ArrEnd();
    }

    private int Intern(object typesOrError, List<(int Index, byte[] Json)> fresh)
    {
        _scratch.Clear();
        WriteTypes(_scratch, typesOrError);
        var key = _scratch.ToUtf8String();
        if (_typeLists.TryGetValue(key, out var index)) return index;
        index = _typeLists.Count;
        _typeLists.Add(key, index);
        fresh.Add((index, _scratch.WrittenSpan.ToArray()));
        return index;
    }

    private static object Applicable(Game game, Player? p, bool isHost)
    {
        try
        {
            return game.GetApplicableEvents(p, isHost);
        }
        catch (Exception e)
        {
            return e;
        }
    }

    private static void WriteTypes(JsonOut o, object typesOrError)
    {
        if (typesOrError is Exception e)
        {
            o.ObjStart();
            o.Key("$type"); o.WriteString("$Exception");
            o.Comma(); o.Key("message"); o.WriteString(e.Message);
            o.Comma(); o.Key("type"); o.WriteString(e.GetType().FullName);
            o.ObjEnd();
            return;
        }

        var types = (List<Type>)typesOrError;
        o.ArrStart();
        for (var i = 0; i < types.Count; i++)
        {
            if (i > 0) o.Comma();
            o.WriteString(types[i].Name);
        }
        o.ArrEnd();
    }

    public static void WriteEnum(JsonOut o, Enum value)
    {
        var raw = Convert.ToInt64(value, System.Globalization.CultureInfo.InvariantCulture);
        o.WriteString(value + "(" + raw.ToString(System.Globalization.CultureInfo.InvariantCulture) + ")");
    }

    private bool _capture;
    private string? _captured;

    /// <summary>Makes the next line go to <see cref="TakeCaptured"/> instead of the file. Used by the verifier.</summary>
    public void CaptureNext() => _capture = true;

    public string TakeCaptured()
    {
        var result = _captured ?? throw new InvalidOperationException("Nothing captured");
        _captured = null;
        return result;
    }

    private void Flush()
    {
        if (_capture)
        {
            _captured = _line.ToUtf8String();
            _capture = false;
            return;
        }

        if (_closed) return;
        _line.Raw((byte)'\n');
        _out.Write(_line.WrittenSpan);
    }

    private bool _closed;

    /// <summary>
    /// Ends a trace whose current event never returned from Execute. Called from the watchdog thread while the game's
    /// own thread is still inside Execute; the recorder isn't otherwise in use at that moment.
    /// </summary>
    public void EndHung(string? eventJson, bool performValidation, bool isHost, int seconds, string? memoryDetail = null)
    {
        _line.Clear();
        _line.ObjStart();
        _line.Key("kind"); _line.WriteString("end");
        // With memoryDetail the call ran the process over --memory-mb instead: same line, other reason.
        var reason = eventJson != null ? "hung" : "botHung";
        if (memoryDetail != null) reason = eventJson != null ? "memoryLimit" : "botMemoryLimit";
        _line.Comma(); _line.Key("reason"); _line.WriteString(reason);
        _line.Comma(); _line.Key("detail");
        _line.WriteString(memoryDetail ?? (eventJson != null ? $"Execute did not return within {seconds} s" : $"Bot deliberation did not return within {seconds} s"));
        _line.Comma(); _line.Key("steps"); _line.Int(_stepIndex);
        if (eventJson != null)
        {
            // The queries were made before this event; a replayer makes them, then executes the event and should loop.
            WriteQueries();
            _line.Comma(); _line.Key("validate"); _line.Bool(performValidation);
            _line.Comma(); _line.Key("isHost"); _line.Bool(isHost);
            _line.Comma(); _line.Key("event"); _line.RawJson(eventJson);
        }

        _line.ObjEnd();
        Flush();
        Dispose();
    }

    /// <summary>Ends the trace when Game.Undo, RestoreRecentlyUndone or DismissRecentlyUndone did not return.</summary>
    public void EndUndoHung(string undoKind, int? untilEventNr, int seconds, string? memoryDetail = null)
    {
        _line.Clear();
        _line.ObjStart();
        _line.Key("kind"); _line.WriteString("end");
        _line.Comma(); _line.Key("reason"); _line.WriteString(memoryDetail != null ? "undoMemoryLimit" : "undoHung");
        _line.Comma(); _line.Key("detail"); _line.WriteString(memoryDetail ?? $"{undoKind} did not return within {seconds} s");
        _line.Comma(); _line.Key("steps"); _line.Int(_stepIndex);
        // As for an undo line: the queries were made before the call, then the call is made and should loop.
        WriteQueries();
        _line.Comma(); _line.Key("undoKind"); _line.WriteString(undoKind);
        if (untilEventNr is { } u)
        {
            _line.Comma(); _line.Key("untilEventNr"); _line.Int(u);
        }

        _line.ObjEnd();
        Flush();
        Dispose();
    }

    public void Dispose()
    {
        if (_closed) return;
        _closed = true;
        _out.Flush();
        _out.Dispose();
    }
}

public static class Trace
{
    public const int FormatVersion = 2;
}
