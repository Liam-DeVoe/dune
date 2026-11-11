using System.Globalization;
using System.Text.Json.Nodes;
using Microsoft.Data.Sqlite;
using Treachery.Oracle.Canon;

namespace Treachery.ServerOracle;

/// <summary>
/// Runs one scenario through the real <see cref="GameHub"/>, in process, and writes one transcript line per step.
/// The format of scenarios and transcripts is in rust/docs/server-plan.md, "Equivalence testing"; the Rust side is
/// rust/server-equiv. Everything that the hub reads from outside (clock, tokens, GUIDs, delays, mail) goes through
/// <see cref="TestHooks"/>.
/// </summary>
public sealed class Harness : IDisposable
{
    public const int TranscriptFormat = 2;

    /// <summary>Configuration every scenario starts with; a scenario header's <c>config</c> overrides keys.</summary>
    public static readonly IReadOnlyDictionary<string, string?> DefaultConfig = new Dictionary<string, string?>
    {
        ["GameEndEmailUsername"] = "harness",
        ["GameEndEmailPassword"] = "harness",
        ["GameEndEmailFrom"] = "player@treachery.online",
        ["GameEndEmailTo"] = "admin@treachery.online",
        ["GameAdminUsername"] = "admin",
    };

    /// <summary>Strings longer than this are written as their SHA-256 in compact transcripts.</summary>
    public const int CompactLimit = 200;

    private readonly string _dbPath;
    private readonly IConfiguration _config;
    private readonly DbContextOptions<TreacheryContext> _dbOptions;
    private readonly FakeSignalR _signalR = new();
    private readonly bool _full;

    private DateTimeOffset _now;
    private long _ids;
    private long _timerSeq;
    private readonly List<PendingTimer> _pending = [];
    private readonly List<string> _mails = [];
    private readonly List<string> _scheduled = [];
    private readonly List<string> _fired = [];
    private readonly List<string> _randoms = [];
    private int _seeds;
    private int _stepNr;

    private sealed record PendingTimer(DateTimeOffset Due, long Seq, string What, Func<Task> Action);

    public Harness(JsonObject header, bool full)
    {
        _full = full || header["full"]?.GetValue<bool>() == true;
        _now = DateTimeOffset.Parse(header["start"]?.GetValue<string>() ?? "2026-03-02T10:00:00+02:00", CultureInfo.InvariantCulture);
        if (_now.Offset != TimeZoneInfo.Local.GetUtcOffset(_now))
            throw new InvalidOperationException($"The scenario starts at {_now:O}, but this process's local offset is {TimeZoneInfo.Local.GetUtcOffset(_now)}. Run with TZ set to match (the run script uses TZ=Etc/GMT-2).");

        _dbPath = Path.Combine(Path.GetTempPath(), $"server-oracle-{Environment.ProcessId}-{Guid.NewGuid():N}.db");

        var config = new Dictionary<string, string?>(DefaultConfig) { ["ConnectionStrings:TreacheryDatabase"] = "Data Source=" + _dbPath };
        if (header["config"] is JsonObject overrides)
            foreach (var (k, v) in overrides)
                config[k] = v?.GetValue<string>();

        _config = new ConfigurationBuilder().AddInMemoryCollection(config).Build();
        _dbOptions = new DbContextOptionsBuilder<TreacheryContext>().Options;

        using (var db = new TreacheryContext(_dbOptions, _config))
            db.Database.Migrate();

        // "seed": SQL statements run on the new database, so a scenario can start from stored users, tokens and games.
        if (header["seed"] is JsonArray seed)
        {
            using var connection = new SqliteConnection($"Data Source={_dbPath};Pooling=False");
            connection.Open();
            foreach (var sql in seed)
            {
                using var cmd = connection.CreateCommand();
                cmd.CommandText = sql!.GetValue<string>();
                cmd.ExecuteNonQuery();
            }
        }

        ResetHubStatics();

        TestHooks.Clock = () => _now;
        TestHooks.NewToken = () => $"TOKEN{++_ids:D11}";
        TestHooks.NewGuid = () => Guid.Parse($"00000000-0000-0000-0000-{++_ids:D12}");
        TestHooks.Delay = (ms, what, action) =>
        {
            var due = _now.AddMilliseconds(ms);
            _pending.Add(new PendingTimer(due, _timerSeq++, what, action));
            _scheduled.Add(new JsonText().ObjStart().MemberSer("Due", due).Member("What", what).ObjEnd().ToString());
        };
        // The bots' randomness: seeds 1, 2, 3, ... in the order the hub asks. Rust's SequentialIds counts the same way.
        TestHooks.NewRandom = what =>
        {
            _randoms.Add(what);
            return new LoggedRandom(++_seeds);
        };
        TestHooks.MailSink = mail =>
        {
            _mails.Add(MailJson(mail));
            return Task.CompletedTask;
        };
    }

    public void Dispose()
    {
        TestHooks.Clock = null;
        TestHooks.NewToken = null;
        TestHooks.NewGuid = null;
        TestHooks.Delay = null;
        TestHooks.NewRandom = null;
        TestHooks.MailSink = null;
        ResetHubStatics();
        SqliteConnection.ClearAllPools();
        foreach (var suffix in new[] { "", "-wal", "-shm" })
            File.Delete(_dbPath + suffix);
    }

    /// <summary>The virtual clock, for the bot-game generator.</summary>
    public DateTimeOffset Now => _now;

    /// <summary>When the earliest pending delayed call is due, or null if none is pending.</summary>
    public DateTimeOffset? NextTimerDue => _pending.Count == 0 ? null : _pending.Min(t => t.Due);

    /// <summary>The live game of a running game, for the trace generator.</summary>
    public static ManagedGame? RunningGame(string gameId) =>
        ((ConcurrentDictionary<string, ManagedGame>)Static("RunningGamesByGameId")).GetValueOrDefault(gameId);

    #region Steps

    /// <summary>Runs one scenario step and returns its transcript line, and the hub method's return value for calls.</summary>
    public (string Line, object? Result) Step(JsonObject step)
    {
        var op = step["op"]!.GetValue<string>();
        _signalR.Received.Clear();
        _mails.Clear();
        _scheduled.Clear();
        _fired.Clear();
        _randoms.Clear();

        var line = new JsonText().ObjStart().Key("step").Int(_stepNr++).Member("op", op);
        object? result = null;
        string? resultJson = null;

        switch (op)
        {
            case "connect":
                _signalR.Connect(Conn(step));
                break;

            case "disconnect":
                _signalR.Disconnect(Conn(step));
                break;

            case "advance":
                Advance(_now.AddMilliseconds(step["ms"]?.GetValue<double>() ?? 0).AddSeconds(step["seconds"]?.GetValue<double>() ?? 0)
                    .AddMinutes(step["minutes"]?.GetValue<double>() ?? 0).AddDays(step["days"]?.GetValue<double>() ?? 0));
                break;

            case "restart":
                // A new server process on the same database: in-memory state, connections and pending timers are gone.
                ResetHubStatics();
                _signalR.DropAll();
                _pending.Clear();
                break;

            case "call":
            {
                var method = step["method"]!.GetValue<string>();
                line.Member("method", method);
                if (step["rust_only"] is { } expected)
                {
                    // A Rust-only rule: C# does not run the step; Rust must return a result matching `expected` and
                    // change nothing. No scenario uses it since the bots were ported.
                    line.MemberRaw("rustOnly", expected.ToJsonString()).ObjEnd();
                    return (line.ToString(), null);
                }

                (resultJson, result) = Call(Conn(step), method, (JsonArray)Resolve(step["args"]!.DeepClone())!);
                if (step["as"] is { } name)
                    _vars[name.GetValue<string>()] = result?.GetType().GetProperty("Contents")?.GetValue(result) is { } contents
                        ? JsonNode.Parse(Utilities.Serialize(contents))
                        : null;
                break;
            }

            default:
                throw new InvalidOperationException("Unknown step op " + op);
        }

        line.Key("result").Raw(resultJson ?? "null");
        WriteCallbacks(line);
        line.Key("mails").ArrStart();
        foreach (var m in _mails) line.Raw(m);
        line.ArrEnd();
        line.Key("timers").ArrStart();
        foreach (var t in _scheduled) line.Raw(t);
        line.ArrEnd();
        line.Key("fired").ArrStart();
        foreach (var t in _fired) line.Str(t);
        line.ArrEnd();
        line.Key("randoms").ArrStart();
        foreach (var r in _randoms) line.Str(r);
        line.ArrEnd();
        Section(line, "tables", WriteTables);
        Section(line, "games", WriteGames);
        Section(line, "hub", WriteHub);
        line.ObjEnd();
        return (line.ToString(), result);
    }

    private readonly Dictionary<string, string> _lastSection = [];

    /// <summary>Writes a state section, or <c>"="</c> when it is the same as in the previous line that had one.</summary>
    private void Section(JsonText line, string key, Action<JsonText> write)
    {
        var j = new JsonText();
        write(j);
        var text = j.ToString();
        line.Key(key).Raw(_lastSection.TryGetValue(key, out var last) && last == text ? "\"=\"" : text);
        _lastSection[key] = text;
    }

    private readonly Dictionary<string, JsonNode?> _vars = [];

    /// <summary>
    /// Replaces <c>${name.Member.0.Member}</c> in argument strings with a value from the <c>Contents</c> of the result
    /// of an earlier call that had <c>"as": "name"</c>. A string that is only a reference becomes that value (a number
    /// stays a number); a reference inside a longer string is replaced by the value's text.
    /// </summary>
    private JsonNode? Resolve(JsonNode? node)
    {
        switch (node)
        {
            case JsonArray a:
                for (var i = 0; i < a.Count; i++) a[i] = Resolve(a[i]?.DeepClone());
                return a;
            case JsonObject o:
                foreach (var key in o.Select(kv => kv.Key).ToList()) o[key] = Resolve(o[key]?.DeepClone());
                return o;
            case JsonValue v when v.TryGetValue<string>(out var s) && s.Contains("${"):
            {
                var whole = System.Text.RegularExpressions.Regex.Match(s, @"^\$\{([^}]*)\}$");
                if (whole.Success) return Lookup(whole.Groups[1].Value)?.DeepClone();
                return System.Text.RegularExpressions.Regex.Replace(s, @"\$\{([^}]*)\}", m =>
                    Lookup(m.Groups[1].Value) is JsonValue lv && lv.TryGetValue<string>(out var str) ? str : Lookup(m.Groups[1].Value)?.ToJsonString() ?? "null");
            }
            default:
                return node;
        }
    }

    private JsonNode? Lookup(string reference)
    {
        var parts = reference.Split('.');
        if (!_vars.TryGetValue(parts[0], out var node)) throw new InvalidOperationException($"Step {_stepNr - 1}: unknown variable {parts[0]}");
        foreach (var p in parts.Skip(1))
            node = int.TryParse(p, out var i) && node is JsonArray a ? a[i] : node?[p];
        return node;
    }

    private static int Conn(JsonObject step) => step["conn"]!.GetValue<int>();

    private void Advance(DateTimeOffset target)
    {
        while (true)
        {
            var next = _pending.Where(t => t.Due <= target).OrderBy(t => t.Due).ThenBy(t => t.Seq).FirstOrDefault();
            if (next == null) break;
            _pending.Remove(next);
            if (next.Due > _now) _now = next.Due;
            _fired.Add(next.What);
            try
            {
                next.Action().GetAwaiter().GetResult();
            }
            catch (Exception)
            {
                // An unobserved ContinueWith task in production: the exception goes nowhere.
            }
        }

        AgeConnections(target - _now);
        _now = target;
    }

    /// <summary>
    /// <c>ConnectionInfo.Created</c> (Shared/Model/UserConnections.cs) reads the real clock, and Shared has no hook.
    /// Moving each <c>Created</c> back by the time the virtual clock moves forward keeps its age equal to the
    /// virtual age, which is what the Rust port computes with the hub clock.
    /// </summary>
    private void AgeConnections(TimeSpan by)
    {
        if (by <= TimeSpan.Zero) return;
        var field = typeof(ConnectionInfo).GetField("<Created>k__BackingField", BindingFlags.Instance | BindingFlags.NonPublic)!;
        foreach (var info in ConnectionInfos().SelectMany(x => x.Value.Values))
            field.SetValue(info, info.Created - by);
    }

    private (string Json, object? Result) Call(int connection, string method, JsonArray args)
    {
        MethodInfo m;
        object?[] argv;
        try
        {
            if (method == "SubmitEvent")
            {
                var evt = Utilities.Deserialize<GameEvent>(args[2]!.ToJsonString())!;
                // The hub method whose event parameter has the event's type (names differ in case for a few, such as
                // RequestClairvoyancePlayed for ClairVoyancePlayed).
                m = typeof(GameHub).GetMethods(BindingFlags.Public | BindingFlags.Instance | BindingFlags.DeclaredOnly)
                        .SingleOrDefault(x => x.Name.StartsWith("Request", StringComparison.Ordinal) && x.GetParameters() is [_, _, var p] && p.ParameterType == evt.GetType())
                    ?? throw new ArgumentException("No hub method for " + evt.GetType().Name);
                argv = [Arg<string>(args[0]), Arg<string>(args[1]), evt];
            }
            else
            {
                m = typeof(GameHub).GetMethod(method, BindingFlags.Public | BindingFlags.Instance | BindingFlags.DeclaredOnly)
                    ?? throw new ArgumentException("No hub method " + method);
                var ps = m.GetParameters();
                if (ps.Length != args.Count) throw new ArgumentException($"{method} takes {ps.Length} arguments, the step has {args.Count}");
                argv = ps.Select((p, i) => DeserializeArg(args[i], p.ParameterType)).ToArray();
            }
        }
        catch (Exception e)
        {
            throw new InvalidOperationException($"Step {_stepNr - 1}: bad arguments for {method}: {e.Message}", e);
        }

        var hub = new GameHub(_dbOptions, _config)
        {
            Context = new FakeContext(connection.ToString(CultureInfo.InvariantCulture)),
            Clients = new FakeCallerClients(_signalR, connection),
            Groups = new FakeGroupManager(_signalR),
        };

        try
        {
            var task = (Task)m.Invoke(hub, argv)!;
            task.GetAwaiter().GetResult();
            var result = task.GetType().GetProperty("Result")!.GetValue(task);
            return (new JsonText().ObjStart().MemberRaw("Result", ResultJson(result)).ObjEnd().ToString(), result);
        }
        catch (Exception e)
        {
            if (e is TargetInvocationException { InnerException: { } inner }) e = inner;
            return (new JsonText().ObjStart().Member("Exception", ExceptionName(e)).ObjEnd().ToString(), null);
        }
        finally
        {
            // SignalR disposes the hub after each call. Delayed calls keep using the disposed object, as in production.
            ((IDisposable)hub).Dispose();
        }
    }

    private static T? Arg<T>(JsonNode? node) => node == null ? default : Utilities.Deserialize<T>(node.ToJsonString());

    private static object? DeserializeArg(JsonNode? node, Type type)
    {
        if (node == null) return null;
        var deserialize = typeof(Utilities).GetMethod(nameof(Utilities.Deserialize))!.MakeGenericMethod(type);
        return deserialize.Invoke(null, [node.ToJsonString()]);
    }

    /// <summary>
    /// Exception type names as compared with Rust: the short name (<c>JsonException</c>), as the Rust engine's
    /// <c>CsException</c> names them. Database exceptions all count as one kind.
    /// </summary>
    public static string ExceptionName(Exception e)
    {
        var name = e.GetType().FullName!;
        return name.StartsWith("Microsoft.EntityFrameworkCore.", StringComparison.Ordinal) || name.StartsWith("Microsoft.Data.Sqlite.", StringComparison.Ordinal)
            ? "(database)"
            : e.GetType().Name;
    }

    /// <summary>
    /// <c>Utilities.Serialize(result)</c>, except that the four lists of a <see cref="ServerStatus"/> are sorted:
    /// C# fills them from <c>ConcurrentDictionary</c>s, whose order is not defined (server-plan.md, "Order").
    /// </summary>
    public static string ResultJson(object? result)
    {
        // An engine exception caught by GameEvent.Execute becomes a "Game Error" message that includes
        // Exception.ToString(), with stack frames and build paths. A port can't reproduce those, so the frames
        // ("\n   at ...") are left out on this side. The Rust side writes none.
        if (result?.GetType().GetProperty("ErrorDetails") is { } detailsProperty && detailsProperty.GetValue(result) is string details && details.Contains("\n   at "))
            detailsProperty.SetValue(result, System.Text.RegularExpressions.Regex.Replace(details, @"\n   at [^\n]*?(?=\n|\.\*$|$)", ""));

        if (result is Result<ServerStatus> { Contents: { } status } r)
        {
            return new JsonText().ObjStart()
                .MemberSer("Success", r.Success)
                .MemberSer("Error", r.Error)
                .MemberSer("ErrorDetails", r.ErrorDetails)
                .MemberRaw("Contents", ServerStatusJson(status))
                .ObjEnd().ToString();
        }

        return Utilities.Serialize(result);
    }

    private static string ServerStatusJson(ServerStatus s) =>
        new JsonText().ObjStart()
            .MemberRaw("RunningGames", SortedArray(s.RunningGames))
            .MemberRaw("OwnGames", SortedArray(s.OwnGames))
            .MemberRaw("ScheduledGames", SortedArray(s.ScheduledGames))
            .MemberRaw("RecentlySeenUsers", SortedArray(s.RecentlySeenUsers))
            .ObjEnd().ToString();

    private static string SortedArray<T>(IEnumerable<T> items)
    {
        var j = new JsonText().ArrStart();
        foreach (var s in items.Select(i => Utilities.Serialize(i)).OrderBy(s => s, StringComparer.Ordinal)) j.Raw(s);
        return j.ArrEnd().ToString();
    }

    #endregion

    #region Transcript sections

    private string Compact(string? s) => s == null || _full || s.Length <= CompactLimit ? JsonText.Escape(s) : JsonText.Escape("sha256:" + JsonText.Sha256(s));

    private void WriteCallbacks(JsonText line)
    {
        line.Key("callbacks").ObjStart();
        foreach (var (connection, callbacks) in _signalR.Received)
        {
            line.Key(connection.ToString(CultureInfo.InvariantCulture)).ArrStart();
            foreach (var c in callbacks) line.Raw(c);
            line.ArrEnd();
        }

        line.ObjEnd();
    }

    private string MailJson(MailMessage mail)
    {
        var j = new JsonText().ObjStart()
            .Member("From", mail.From?.ToString())
            .Key("To").ArrStart();
        foreach (var to in mail.To) j.Str(to.ToString());
        j.ArrEnd()
            .Member("Subject", mail.Subject)
            .Member("Body", mail.Body)
            .MemberSer("IsBodyHtml", mail.IsBodyHtml)
            .Member("Priority", mail.Priority.ToString())
            .Key("Attachments").ArrStart();
        foreach (var a in mail.Attachments)
        {
            a.ContentStream.Position = 0;
            var content = new StreamReader(a.ContentStream).ReadToEnd();
            j.ObjStart().Member("Name", a.Name).MemberRaw("Content", Compact(content)).ObjEnd();
        }

        return j.ArrEnd().ObjEnd().ToString();
    }

    /// <summary>Tables, their sort keys, and the columns left out (row ids C# assigns in an undefined order).</summary>
    public static readonly (string Table, string OrderBy, string? Skip)[] Tables =
    [
        ("Users", "Id", null),
        ("UserTokens", "Token", null),
        ("PersistedGames", "GameId", "Id"),
        ("ScheduledGames", "GameId", "Id"),
        ("ArchivedGames", "Id", null),
    ];

    private void WriteTables(JsonText line)
    {
        line.ObjStart();
        using var connection = new SqliteConnection($"Data Source={_dbPath};Pooling=False");
        connection.Open();
        foreach (var (table, orderBy, skip) in Tables)
        {
            line.Key(table).ArrStart();
            using var cmd = connection.CreateCommand();
            cmd.CommandText = $"SELECT * FROM \"{table}\" ORDER BY \"{orderBy}\"";
            using var reader = cmd.ExecuteReader();
            while (reader.Read())
            {
                line.ObjStart();
                for (var i = 0; i < reader.FieldCount; i++)
                {
                    var name = reader.GetName(i);
                    if (name == skip) continue;
                    line.Key(name);
                    switch (reader.GetValue(i))
                    {
                        case DBNull: line.Null(); break;
                        case long l: line.Int(l); break;
                        case string s: line.Raw(Compact(s)); break;
                        case var other: throw new InvalidOperationException($"{table}.{name}: unexpected SQLite value {other}");
                    }
                }

                line.ObjEnd();
            }

            line.ArrEnd();
        }

        line.ObjEnd();
    }

    private void WriteGames(JsonText line)
    {
        line.ArrStart();
        foreach (var (id, g) in ((ConcurrentDictionary<string, ManagedGame>)Static("RunningGamesByGameId")).OrderBy(x => x.Key, StringComparer.Ordinal))
        {
            var dump = new JsonOut();
            new StateDumper(dump).DumpGame(g.Game);
            line.ObjStart()
                .Member("Key", id)
                .Member("GameId", g.GameId)
                .MemberSer("CreationDate", g.CreationDate)
                .MemberSer("LastActivity", g.LastActivity)
                .MemberSer("LastPersisted", g.LastPersisted)
                .MemberSer("CreatorUserId", g.CreatorUserId)
                .Member("Name", g.Name)
                .Member("HashedPassword", g.HashedPassword)
                .MemberSer("ObserversRequirePassword", g.ObserversRequirePassword)
                .MemberSer("StatisticsSent", g.StatisticsSent)
                .MemberSer("LastAsyncPlayMessageSent", g.LastAsyncPlayMessageSent)
                .MemberRaw("Participation", Utilities.Serialize(g.Game.Participation))
                .MemberRaw("State", Compact(GameState.GetStateAsString(g.Game)))
                .Member("Canon", StateDumper.Sha256Hex(dump.WrittenSpan))
                .Key("Bots").ArrStart();
            // ManagedGame.Bots in insertion order, with each bot's draw count (ClassicBot._random).
            foreach (var (faction, bot) in g.Bots)
                line.ObjStart().MemberSer("Faction", faction).MemberSer("Draws", BotRandom(bot).GetCount()).ObjEnd();
            line.ArrEnd().ObjEnd();
        }

        line.ArrEnd();
    }

    private static LoggedRandom BotRandom(object bot) =>
        (LoggedRandom)bot.GetType().GetField("_random", BindingFlags.Instance | BindingFlags.NonPublic)!.GetValue(bot)!;

    private static string UserJson(User u) =>
        new JsonText().ObjStart()
            .MemberSer("Id", u.Id)
            .Member("Name", u.Name)
            .Member("Email", u.Email)
            .Member("HashedPassword", u.HashedPassword)
            .Member("PasswordResetToken", u.PasswordResetToken)
            .MemberSer("PasswordResetTokenCreated", u.PasswordResetTokenCreated)
            .Member("PlayerName", u.PlayerName)
            .MemberSer("LastLogin", u.LastLogin)
            .ObjEnd().ToString();

    private void WriteHub(JsonText line)
    {
        var usersById = (ConcurrentDictionary<int, User>)Static("UsersById");
        line.ObjStart();

        line.Key("UsersByUserToken").ArrStart();
        foreach (var (token, u) in ((ConcurrentDictionary<string, LoggedInUser>)Static("UsersByUserToken")).OrderBy(x => x.Key, StringComparer.Ordinal))
        {
            line.ObjStart()
                .Member("Token", token)
                .MemberSer("Status", u.Status)
                .MemberSer("LoggedInDateTime", u.LoggedInDateTime)
                .MemberSer("LastSeenDateTime", u.LastSeenDateTime)
                .MemberRaw("User", UserJson(u.User))
                .MemberSer("SameObjectAsUsersById", usersById.TryGetValue(u.Id, out var cached) && ReferenceEquals(cached, u.User))
                .ObjEnd();
        }

        line.ArrEnd();

        line.Key("UsersById").ArrStart();
        foreach (var (_, u) in usersById.OrderBy(x => x.Key)) line.Raw(UserJson(u));
        line.ArrEnd();

        line.Key("ConnectionInfoByUserId").ArrStart();
        foreach (var (userId, byGame) in ConnectionInfos().OrderBy(x => x.Key))
        {
            line.ObjStart().MemberSer("UserId", userId).Key("Games").ArrStart();
            foreach (var (gameId, info) in byGame)
                line.ObjStart().Member("GameId", gameId).Key("ConnectionId").Int(long.Parse(info.ConnectionId)).ObjEnd();
            line.ArrEnd().ObjEnd();
        }

        line.ArrEnd();

        line.Key("ScheduledGamesByGameId").ArrStart();
        foreach (var (id, g) in ((ConcurrentDictionary<string, ScheduledGame>)Static("ScheduledGamesByGameId")).OrderBy(x => x.Key, StringComparer.Ordinal))
        {
            line.ObjStart()
                .Member("Key", id)
                .Member("ScheduledGameId", g.ScheduledGameId)
                .MemberSer("DateTime", g.DateTime)
                .MemberSer("CreatorUserId", g.CreatorUserId)
                .Member("CreatorPlayerName", g.CreatorPlayerName)
                .MemberSer("NumberOfPlayers", g.NumberOfPlayers)
                .MemberSer("MaximumTurns", g.MaximumTurns)
                .MemberSer("Ruleset", g.Ruleset)
                .MemberSer("AsyncPlay", g.AsyncPlay)
                .MemberSer("AllowedFactionsInPlay", g.AllowedFactionsInPlay)
                .MemberSer("SubscribedUsers", g.SubscribedUsers)
                .ObjEnd();
        }

        line.ArrEnd();

        line.MemberRaw("RunningGames", SortedArray((GameInfo[])Static("RunningGames")));
        line.MemberRaw("ScheduledGames", SortedArray((ScheduledGameInfo[])Static("ScheduledGames")));
        line.MemberRaw("RecentlySeenUsers", SortedArray((LoggedInUserInfo[])Static("RecentlySeenUsers")));

        foreach (var name in new[] { "LastRestored", "LastPersistedRunningGames", "LastPersistedScheduledGames", "MaintenanceDate", "LastCleanedUpUserTokens", "LastCleanedUpScheduledGames", "LastUpdatedServerStatus" })
            line.MemberSer(name, (DateTimeOffset)Static(name));

        line.MemberSer("Restoring", (bool)Static("Restoring"));

        line.Key("Groups").ObjStart();
        foreach (var (gameId, members) in _signalR.Groups)
        {
            line.Key(gameId).ArrStart();
            foreach (var c in members) line.Int(c);
            line.ArrEnd();
        }

        line.ObjEnd();

        line.Key("PendingTimers").ArrStart();
        foreach (var t in _pending.OrderBy(t => t.Due).ThenBy(t => t.Seq))
            line.ObjStart().MemberSer("Due", t.Due).Member("What", t.What).ObjEnd();
        line.ArrEnd();

        line.ObjEnd();
    }

    private static IEnumerable<KeyValuePair<int, Dictionary<string, ConnectionInfo>>> ConnectionInfos()
    {
        var property = typeof(UserConnections).GetProperty("ConnectionInfoByGameId", BindingFlags.Instance | BindingFlags.NonPublic)!;
        return ((ConcurrentDictionary<int, UserConnections>)Static("ConnectionInfoByUserId"))
            .Select(x => KeyValuePair.Create(x.Key, (Dictionary<string, ConnectionInfo>)property.GetValue(x.Value)!));
    }

    #endregion

    #region Hub statics

    private static object Static(string name) =>
        typeof(GameHub).GetProperty(name, BindingFlags.Static | BindingFlags.NonPublic)!.GetValue(null)!;

    /// <summary>
    /// Brings the hub's static state back to that of a freshly started process. Fails on a static property of a
    /// type it doesn't know, so a new static in GameHub can't leak from one scenario into the next.
    /// </summary>
    private static void ResetHubStatics()
    {
        foreach (var p in typeof(GameHub).GetProperties(BindingFlags.Static | BindingFlags.NonPublic | BindingFlags.Public))
        {
            var t = p.PropertyType;
            if (t.IsGenericType && t.GetGenericTypeDefinition() == typeof(ConcurrentDictionary<,>))
                t.GetMethod("Clear")!.Invoke(p.GetValue(null), null);
            else if (t.IsArray)
                p.SetValue(null, Array.CreateInstance(t.GetElementType()!, 0));
            else if (t == typeof(DateTimeOffset))
                p.SetValue(null, default(DateTimeOffset));
            else if (t == typeof(bool))
                p.SetValue(null, false);
            else
                throw new InvalidOperationException($"ServerOracle: don't know how to reset GameHub.{p.Name} ({t})");
        }
    }

    #endregion
}
