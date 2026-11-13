using Treachery.Bots;
using Treachery.Oracle.Canon;

namespace Treachery.Oracle;

public sealed class FuzzSettings
{
    /// <summary>Probability per step of submitting a freshly generated random event.</summary>
    public double RandomEvent { get; init; } = 0.12;

    /// <summary>Probability per step of submitting a perturbed copy of the bot's event instead of the bot's event.</summary>
    public double MutateBotEvent { get; init; } = 0.10;

    /// <summary>Probability per step of calling Game.Undo.</summary>
    public double Undo { get; init; } = 0.004;

    /// <summary>Probability, right after an Undo, of calling RestoreRecentlyUndone or DismissRecentlyUndone next.</summary>
    public double RestoreOrDismissAfterUndo { get; init; } = 0.3;

    /// <summary>
    /// True (the default, --fuzz-profile client): every event the fuzzer submits is one the client UI could send. Ids
    /// are real, values come from the engine's Valid*/Max* helpers or the game state, and invalid candidates are only
    /// submitted to be rejected. False (--fuzz-profile wide): also raw values from broad domains, forged ids included.
    /// </summary>
    public bool ClientOnly { get; init; } = true;

    /// <summary>Probability per step, while a battle is in progress, of submitting a battle action for a player in the battle.</summary>
    public double BattleAction { get; init; } = 0.25;

    public void WriteJson(JsonOut o)
    {
        o.ObjStart();
        o.Key("randomEvent"); o.Double(RandomEvent);
        o.Comma(); o.Key("mutateBotEvent"); o.Double(MutateBotEvent);
        o.Comma(); o.Key("undo"); o.Double(Undo);
        o.Comma(); o.Key("restoreOrDismissAfterUndo"); o.Double(RestoreOrDismissAfterUndo);
        o.Comma(); o.Key("battleAction"); o.Double(BattleAction);
        o.Comma(); o.Key("profile"); o.WriteString(ClientOnly ? "client" : "wide");
        o.ObjEnd();
    }
}

public sealed record GameSummary(int Index, string File, string EndReason, string? Detail, int Steps, int HistoryCount, int Version, int Players, string HostStyle);

/// <summary>Plays one bot or fuzz game and records its trace.</summary>
public sealed class GameRunner
{
    private static readonly FieldInfo BotRandomField = typeof(ClassicBot).GetField("_random", BindingFlags.Instance | BindingFlags.NonPublic)!;

    private readonly string _mode;
    private readonly int _masterSeed;
    private readonly int _index;
    private readonly int _gameSeed;
    private readonly Random _rng;
    private readonly FuzzSettings? _fuzz;
    private readonly Fuzzer? _fuzzer;
    private readonly GameConfig _config;
    private readonly Dictionary<Faction, (ClassicBot Bot, int Seed)> _bots = new();
    private DateTimeOffset _clock = new(2024, 1, 1, 12, 0, 0, TimeSpan.Zero);
    private Game _game = null!;
    private Recorder _rec = null!;

    private readonly bool _checkBotPurity;

    // Hang detection. Some fuzzed states send the engine into an endless loop inside Execute (for example
    // PlayerSequence.NextPlayer when nobody may get a turn). .NET can't abort that thread, so a watchdog ends the
    // trace with reason "hung" and the run continues without this game.
    private readonly object _hangLock = new();
    private (long Since, string? EventJson, bool Validate, bool IsHost, string? UndoKind, int? Until)? _executing;
    private bool _hung;


    public sealed class GameHungException : Exception;

    /// <summary>If the current Execute has run longer than <paramref name="seconds"/>, ends the trace as hung and returns true.</summary>
    public bool TryMarkHung(int seconds, out string reason)
    {
        reason = "hung";
        lock (_hangLock)
        {
            if (_hung || _executing is not { } e) return false;
            if (System.Diagnostics.Stopwatch.GetElapsedTime(e.Since).TotalSeconds < seconds) return false;
            _hung = true;
            if (e.UndoKind != null) _rec.EndUndoHung(e.UndoKind, e.Until, seconds);
            else _rec.EndHung(e.EventJson, e.Validate, e.IsHost, seconds);
            reason = e.UndoKind != null ? "undoHung" : e.EventJson != null ? "hung" : "botHung";
            return true;
        }
    }

    /// <summary>Seconds the current watched call (Execute, bot deliberation, undo) has run, or null if none is running.</summary>
    public double? WatchedSeconds
    {
        get
        {
            lock (_hangLock)
                return _hung || _executing is not { } e ? null : System.Diagnostics.Stopwatch.GetElapsedTime(e.Since).TotalSeconds;
        }
    }

    /// <summary>Ends the trace because the process went over its memory limit during the current watched call.</summary>
    public bool TryMarkMemoryLimit(string detail, out string reason)
    {
        reason = "memoryLimit";
        lock (_hangLock)
        {
            if (_hung || _executing is not { } e) return false;
            _hung = true;
            if (e.UndoKind != null) _rec.EndUndoHung(e.UndoKind, e.Until, 0, detail);
            else _rec.EndHung(e.EventJson, e.Validate, e.IsHost, 0, detail);
            reason = e.UndoKind != null ? "undoMemoryLimit" : e.EventJson != null ? "memoryLimit" : "botMemoryLimit";
            return true;
        }
    }

    private void EnterWatched(string? eventJson, bool validate, bool isHost, string? undoKind = null, int? until = null)
    {
        lock (_hangLock)
        {
            if (_hung) throw new GameHungException();
            _executing = (System.Diagnostics.Stopwatch.GetTimestamp(), eventJson, validate, isHost, undoKind, until);
        }
    }

    private void LeaveWatched()
    {
        lock (_hangLock)
        {
            _executing = null;
            if (_hung) throw new GameHungException();
        }
    }

    public int Index => _index;

    public string FileName { get; private set; } = "";

    public GameRunner(string mode, int masterSeed, int index, FuzzSettings? fuzz, bool checkBotPurity = false, bool latestOnly = false)
    {
        _checkBotPurity = checkBotPurity;
        _mode = mode;
        _masterSeed = masterSeed;
        _index = index;
        _gameSeed = DeriveSeed(masterSeed, index, mode == "fuzz" ? 0x46555A5A : 0x424F5453);
        _rng = new Random(_gameSeed);
        _fuzz = fuzz;
        _fuzzer = fuzz != null ? new Fuzzer(new Random(DeriveSeed(_gameSeed, 1, 7))) { ClientOnly = fuzz.ClientOnly } : null;
        if (_fuzzer != null && checkBotPurity) _fuzzer.StateHasher = StateHash;
        _config = GameConfig.Randomize(_rng, fuzz != null, latestOnly);
    }

    public static int DeriveSeed(int a, int b, int c)
    {
        // SplitMix64 over the three inputs; any fixed mixing function works, this one is documented in trace-format.md.
        var x = (ulong)(uint)a * 0x9E3779B97F4A7C15UL ^ (ulong)(uint)b * 0xC2B2AE3D27D4EB4FUL ^ (ulong)(uint)c * 0x165667B19E3779F9UL;
        x += 0x9E3779B97F4A7C15UL;
        x = (x ^ (x >> 30)) * 0xBF58476D1CE4E5B9UL;
        x = (x ^ (x >> 27)) * 0x94D049BB133111EBUL;
        x ^= x >> 31;
        return (int)(x & 0x7FFFFFFF);
    }

    public GameSummary Run(string outDir, DumpPolicy policy, bool gzip, bool discard = false)
    {
        var file = $"{_mode}-{_masterSeed}-{_index:D5}.jsonl" + (gzip ? ".gz" : "");
        FileName = file;
        using (_rec = new Recorder(discard ? Recorder.DiscardPath : Path.Combine(outDir, file), policy))
        {
            var participation = _config.CreateParticipation();
            _game = new Game(_config.Version, participation);
            _rec.Header(o =>
            {
                o.Comma(); o.Key("mode"); o.WriteString(_mode);
                o.Comma(); o.Key("masterSeed"); o.Int(_masterSeed);
                o.Comma(); o.Key("gameIndex"); o.Int(_index);
                o.Comma(); o.Key("gameSeed"); o.Int(_gameSeed);
                o.Comma(); o.Key("gameVersion"); o.Int(_config.Version);
                o.Comma(); o.Key("engineSeed"); o.Int(_config.EngineSeed);
                o.Comma(); o.Key("config"); _config.WriteJson(o);
                o.Comma(); o.Key("participation"); new StateDumper(o).DumpDetached(participation, null);
                if (_fuzz != null)
                {
                    o.Comma(); o.Key("fuzz"); _fuzz.WriteJson(o);
                }
            });

            var (reason, detail) = Play();
            _rec.End(reason, detail, _game);
            return new GameSummary(_index, file, reason, detail, _rec.StepCount, _game.History.Count, _config.Version, _config.NumberOfPlayers, _config.HostStyle.ToString());
        }
    }

    private (string Reason, string? Detail) Play()
    {
        // Setup: the host establishes the game. In fuzz mode it is sometimes a perturbed copy first.
        var establish = _config.CreateEstablishPlayers(_game);
        if (_fuzzer != null && _rng.NextDouble() < 0.3)
        {
            var mutated = _fuzzer.Mutate(_game, establish, out var field);
            if (mutated != null) Submit(mutated, "mutate", null, true, null, null, field);
        }

        if (_game.CurrentPhase == Phase.AwaitingPlayers)
        {
            Submit(establish, "host", null, true, null, null, null);
            if (_game.CurrentPhase == Phase.AwaitingPlayers) return ("setupFailed", null);
        }

        // Fuzz: a few events of arbitrary types while the game is still being set up. Most are not applicable yet,
        // and the rejection message calls the event's GetMessage(), whose text is otherwise only seen by the UI.
        if (_fuzzer != null)
        {
            var burst = _rng.Next(0, 7);
            for (var i = 0; i < burst; i++)
            {
                var evt = _fuzzer.AnyTypeEvent(_game, out var actor, out var isHost);
                Submit(evt, "fuzz", actor, isHost, null, null, null);
            }
        }

        var cap = 400 + _config.MaximumTurns * Math.Max(2, _game.Players.Count) * 80;
        var consecutiveRejected = 0;

        while (_game.CurrentPhase != Phase.GameEnded)
        {
            // --status every: what the server and the clients compute between events, on the live game.
            if (_rec.Policy.StatusEvery) RecordStatus();

            if (_rec.StepCount >= cap) return ("cap", null);

            if (consecutiveRejected >= (_fuzzer == null ? 25 : 80)) return ("stalled", "too many consecutive rejected events");

            if (_fuzz != null)
            {
                var r = _rng.NextDouble();
                if (r < _fuzz.Undo && _game.History.Count > 3)
                {
                    DoUndo();
                    continue;
                }

                // Restore or dismiss without a preceding undo: the engine's "nothing to restore" paths.
                if (r > 1 - _fuzz.Undo / 4)
                {
                    DoRestoreOrDismiss();
                    continue;
                }

                if (r < _fuzz.Undo + _fuzz.RandomEvent)
                {
                    int? actor = null;
                    var isHost = false;
                    var evt = _fuzzer!.RandomEvent(_game, (pl, h) => _rec.Query(_game, pl, h), out actor, out isHost);
                    FlagIfFuzzerMutated();
                    if (evt != null) Submit(evt, "fuzz", actor, isHost, null, null, null);
                    continue;
                }
            }

            BotChoice? choice;
            EnterWatched(null, true, false);
            try
            {
                choice = DeliberateBots();
            }
            catch (Exception e)
            {
                LeaveWatched();
                return ("botException", e.GetType().FullName + ": " + e.Message);
            }

            LeaveWatched();

            // Version 188: change an element of a withdrawn plan that the opponent saw through prescience. Must be rejected.
            if (_fuzz != null && _lastQueried != null && _game.Version >= 188 && _game.PrescienceSeenPlans.Count > 0 && _rng.NextDouble() < 0.3)
            {
                var changed = GenerateWatched(() => _fuzzer!.ChangedSeenPlan(_game, _lastQueried, out var a) is { } e ? (e, a) : ((GameEvent, int)?)null);
                if (changed is { } cp)
                {
                    Submit(cp.Item1, "fuzz", cp.Item2, _lastQueriedAsHost, null, null, null);
                    continue;
                }
            }

            // While a battle is on, act in it: the battle actions (Voice, Juice, Residual, Karama dial, audit, ...) that
            // the bots rarely play, by a player in the battle, filled from the battle.
            if (_fuzz != null && _lastQueried != null && Fuzzer.BattleInProgress(_game) && _rng.NextDouble() < _fuzz.BattleAction)
            {
                var action = GenerateWatched(() => _fuzzer!.BattleAction(_game, _lastQueried, out var a) is { } e ? (e, a) : ((GameEvent, int)?)null);
                if (action is { } ba)
                {
                    Submit(ba.Item1, "fuzz", ba.Item2, _lastQueriedAsHost, null, null, null);
                    continue;
                }
            }

            // A rarely executed event type that is applicable right now gets a steered event, so short windows
            // (a faction's turn, a card in hand, a sub-phase) are used before the bots move the game on.
            if (_fuzz != null && _lastQueried != null && _rng.NextDouble() < OpportunisticProbability)
            {
                var rare = GenerateWatched(() => _fuzzer!.Opportunistic(_game, _lastQueried, _lastQueriedAsHost, out var a) is { } e ? (e, a) : ((GameEvent, int)?)null);
                if (rare is { } r)
                {
                    Submit(r.Item1, "fuzz", r.Item2, _lastQueriedAsHost, null, null, null);
                    continue;
                }
            }

            if (choice == null)
            {
                // Since v188 the alliance phase waits for every faction to pass. The bots playing seated humans
                // wait for humans to pass, which includes themselves, so stand in for a human who hasn't acted yet.
                if (HumanPass() is { } pass)
                {
                    var passed = Submit(pass.Item1, "human", pass.Item2, false, null, null, null);
                    consecutiveRejected = passed ? 0 : consecutiveRejected + 1;
                    continue;
                }

                var host = HostAction();
                if (host == null)
                {
                    if (_fuzzer == null) return ("stalled", "no bot or host event");

                    // In fuzz mode keep throwing random events at a stuck game; one may get it moving again.
                    int? actor = null;
                    var isHost = false;
                    var evt = _fuzzer.RandomEvent(_game, (pl, h) => _rec.Query(_game, pl, h), out actor, out isHost);
                    FlagIfFuzzerMutated();
                    var progressed = evt != null && Submit(evt, "fuzz", actor, isHost, null, null, null);
                    consecutiveRejected = progressed ? 0 : consecutiveRejected + 1;
                    continue;
                }

                var accepted = Submit(host, "host", null, true, null, null, null);
                consecutiveRejected = accepted ? 0 : consecutiveRejected + 1;
                continue;
            }

            var c = choice.Value;
            if (_fuzz != null && _rng.NextDouble() < _fuzz.MutateBotEvent)
            {
                string? field = null;
                var mutated = GenerateWatched(() => _fuzzer!.Mutate(_game, c.Event, out field));
                if (mutated != null)
                {
                    Submit(mutated, "mutate", c.Actor, c.IsHost, c.Bot, c.Order, field);
                    continue;
                }
            }

            var ok = Submit(c.Event, "bot", c.Actor, c.IsHost, c.Bot, c.Order, null);
            consecutiveRejected = ok ? 0 : consecutiveRejected + 1;
        }

        return ("gameEnded", null);
    }

    /// <summary>GameStatus.DetermineStatus for every player (as a player), then for an observer (no player, not a player).</summary>
    private void RecordStatus()
    {
        foreach (var p in _game.Players) _rec.Status(_game, p, true);
        _rec.Status(_game, null, false);
    }

    private void DoUndo()
    {
        // Mostly undo the last few events, as players do; sometimes jump far back.
        var count = _game.History.Count;
        var until = _rng.NextDouble() < 0.7 ? Math.Max(1, count - _rng.Next(1, 6)) : _rng.Next(1, count);
        // Undo replays the history without validation, which can loop forever where the original run didn't.
        EnterWatched(null, true, false, "undo", until);
        _game = _game.Undo(until);
        LeaveWatched();
        _rec.SpecialStep("undo", until, _game, BotRandomCounts());

        if (_rng.NextDouble() < _fuzz!.RestoreOrDismissAfterUndo) DoRestoreOrDismiss();
    }

    private void DoRestoreOrDismiss()
    {
        if (_rng.NextDouble() < 0.5)
        {
            EnterWatched(null, true, false, "restoreRecentlyUndone");
            _game = _game.RestoreRecentlyUndone();
            LeaveWatched();
            _rec.SpecialStep("restoreRecentlyUndone", null, _game, BotRandomCounts());
        }
        else
        {
            EnterWatched(null, true, false, "dismissRecentlyUndone");
            _game = _game.DismissRecentlyUndone();
            LeaveWatched();
            _rec.SpecialStep("dismissRecentlyUndone", null, _game, BotRandomCounts());
        }
    }

    /// <summary>Serializes the event, executes the deserialized copy, and records the step. Returns true if accepted.</summary>
    private bool Submit(GameEvent evt, string source, int? actor, bool isHost, BotInfo? bot, int[]? order, string? mutatedField)
    {
        _clock = NextTime();
        evt.Time = _clock;
        if (_fuzzer != null && source == "fuzz" && !_fuzz!.ClientOnly && _rng.NextDouble() < 0.03) evt.Time = default;

        const bool validate = true;

        var json = Utilities.Serialize<GameEvent>(evt);
        var copy = Utilities.Deserialize<GameEvent>(json)!;
        copy.Initialize(_game);

        var phaseBefore = _game.CurrentPhase;
        EnterWatched(json, validate, isHost);
        var outcome = StepExecutor.Run(_game, copy, validate, isHost);
        LeaveWatched();

        string? after;
        try
        {
            after = Utilities.Serialize<GameEvent>(copy);
        }
        catch (Exception e)
        {
            after = "{\"$serializationError\":" + System.Text.Json.JsonSerializer.Serialize(e.GetType().FullName) + "}";
        }

        _rec.Step(new StepInfo
        {
            Source = source,
            Actor = actor,
            PerformValidation = validate,
            IsHost = isHost,
            EventJson = json,
            MutatedField = mutatedField,
            Bot = bot,
            BotOrder = order
        }, outcome, _game, after, BotRandomCounts());

        if (outcome.Accepted) _fuzzer?.NoteAccepted(copy.GetType(), phaseBefore);
        return outcome.Accepted;
    }

    private DateTimeOffset NextTime()
    {
        var r = _rng.NextDouble();
        var seconds = r < 0.02 ? 7200 + _rng.Next(0, 3600) : _rng.Next(1, 120);
        return _clock.AddSeconds(seconds);
    }

    private IReadOnlyDictionary<Faction, int>? BotRandomCounts()
    {
        if (_bots.Count == 0) return null;
        var result = new Dictionary<Faction, int>();
        foreach (var (f, b) in _bots) result[f] = ((LoggedRandom)BotRandomField.GetValue(b.Bot)!).GetCount();
        return result;
    }

    #region Bots

    private readonly record struct BotChoice(GameEvent Event, int Actor, bool IsHost, BotInfo Bot, int[] Order);

    private ClassicBot GetBot(Player p, out int seed)
    {
        if (_bots.TryGetValue(p.Faction, out var entry))
        {
            entry.Bot.SetGameAndPlayer(_game, p);
            seed = entry.Seed;
            return entry.Bot;
        }

        seed = DeriveSeed(_gameSeed, (int)p.Faction, 0x424F54);
        var bot = new ClassicBot(_game, p, BotParameters.GetDefaultParameters(p.Faction), new LoggedRandom(seed));
        _bots.Add(p.Faction, (bot, seed));
        _createdBots?.Add((p.Faction, _game.Players.IndexOf(p), seed));
        return bot;
    }

    private static readonly (string Name, Func<IBot, List<Type>, GameEvent?> Ask)[] Levels =
    [
        ("Highest", (b, e) => b.DetermineHighestPriorityInPhaseAction(e)),
        ("High", (b, e) => b.DetermineHighPriorityInPhaseAction(e)),
        ("Middle", (b, e) => b.DetermineMiddlePriorityInPhaseAction(e)),
        ("Low", (b, e) => b.DetermineLowPriorityInPhaseAction(e)),
        ("End", (b, e) => b.DetermineEndPhaseAction(e))
    ];

    /// <summary>Probability per bot step of looking for a rarely executed applicable event type (fuzz mode).</summary>
    private const double OpportunisticProbability = 0.3;

    // The applicable event types per player from the latest bot deliberation, and whether they were listed as host.
    private List<Type>[]? _lastQueried;
    private bool _lastQueriedAsHost;

    /// <summary>Runs fuzzer generation; with --check-bot-purity true, flags the next step if generation changed the state.</summary>
    private T GenerateWatched<T>(Func<T> generate)
    {
        var hashBefore = _checkBotPurity ? StateHash(_game) : null;
        var result = generate();
        if (_checkBotPurity && StateHash(_game) != hashBefore) _rec.FlagBotMutatedState();
        return result;
    }

    private void FlagIfFuzzerMutated()
    {
        if (_fuzzer is { StateChangedDuringGeneration: true })
        {
            _fuzzer.StateChangedDuringGeneration = false;
            _rec.FlagBotMutatedState();
        }
    }

    private BotChoice? DeliberateBots()
    {
        _lastQueried = null;
        var players = _game.Players;
        if (players.Count == 0) return null;

        var order = Enumerable.Range(0, players.Count).ToArray();
        if (_config.HostStyle != HostStyle.Tests) GameConfig.Shuffle(order, _rng);

        // Tests.cs asks in Players order, the server in its (shuffled) bot order.
        var listAsHost = _config.HostStyle is HostStyle.Tests or HostStyle.ServerBotsHost;
        var events = new List<Type>[players.Count];
        foreach (var i in order) events[i] = _rec.Query(_game, players[i], listAsHost);
        _lastQueried = events;
        _lastQueriedAsHost = listAsHost;

        var hashBefore = _checkBotPurity ? StateHash(_game) : null;
        var created = new List<(Faction, int, int)>();
        _createdBots = created;
        BotChoice? choice = null;
        Exception? error = null;
        try
        {
            choice = AskBots(players, order, events);
            return choice;
        }
        catch (Exception e)
        {
            error = e;
            throw;
        }
        finally
        {
            _createdBots = null;
            // Trace format 2: the deliberation itself, so a replayer can run its own bots at this point and compare.
            (int, Faction, string, string)? recorded = null;
            if (choice is { } c) recorded = (c.Actor, c.Bot.Faction, c.Bot.Priority, SerializeChoice(c.Event));
            _rec.Bots(order, _config.HostStyle != HostStyle.ServerHumanHost, created, recorded, error);
            if (_checkBotPurity && StateHash(_game) != hashBefore) _rec.FlagBotMutatedState();
        }
    }

    // Bots created during the current deliberation, for the "bots" query entry.
    private List<(Faction, int, int)>? _createdBots;

    private static string SerializeChoice(GameEvent evt)
    {
        try
        {
            return Utilities.Serialize<GameEvent>(evt);
        }
        catch (Exception e)
        {
            return "{\"$serializationError\":" + System.Text.Json.JsonSerializer.Serialize(e.GetType().FullName) + "}";
        }
    }

    private static string StateHash(Game game)
    {
        var o = new JsonOut();
        new StateDumper(o).DumpGame(game);
        return StateDumper.Sha256Hex(o.WrittenSpan);
    }

    private BotChoice? AskBots(List<Player> players, int[] order, List<Type>[] events)
    {

        foreach (var (name, ask) in Levels)
        {
            var isHost = _config.HostStyle == HostStyle.Tests || name == "End";
            if (name == "End" && _config.HostStyle == HostStyle.ServerHumanHost) break;

            foreach (var i in order)
            {
                var p = players[i];
                var bot = GetBot(p, out var seed);
                var hashBefore = _checkBotPurity ? StateHash(_game) : null;
                var evt = ask(bot, events[i]);
                // With --check-bot-purity true, name the ask that changed the state (botMutatedBy).
                if (_checkBotPurity && StateHash(_game) != hashBefore) _rec.NoteBotMutation(i, p.Faction, name, evt?.GetType().Name);
                if (evt != null)
                    return new BotChoice(evt, i, isHost, new BotInfo { Faction = p.Faction, Seed = seed, Priority = name, Player = i }, order);
            }
        }

        return null;
    }

    #endregion

    #region Host

    /// <summary>What a human host would do when no bot acts: the host-only events the bots never produce.</summary>
    private (GameEvent, int)? HumanPass()
    {
        if (_game.CurrentPhase is not (Phase.AllianceA or Phase.AllianceB)) return null;

        for (var i = 0; i < _game.Players.Count; i++)
        {
            var p = _game.Players[i];
            if (_game.IsBot(p) || _game.HasActedOrPassed.Contains(p.Faction)) continue;

            List<Type> types;
            try
            {
                types = _rec.Query(_game, p, false);
            }
            catch
            {
                continue;
            }

            if (types.Contains(typeof(AlliancePassed))) return (new AlliancePassed(_game, p.Faction), i);
        }

        return null;
    }

    private GameEvent? HostAction()
    {
        List<Type> types;
        try
        {
            types = _rec.Query(_game, null, true);
        }
        catch
        {
            return null;
        }

        if (types.Contains(typeof(FactionSelected)))
        {
            var p = _game.Players.FirstOrDefault(x => x.Faction == Faction.None);
            var valid = FactionSelected.ValidFactions(_game).ToList();
            if (p != null && valid.Count > 0)
                return new FactionSelected { Initiator = Faction.None, Seat = p.Seat, Faction = valid[_rng.Next(valid.Count)], InitiatorPlayerName = p.Name };
        }

        if (types.Contains(typeof(CardsDetermined)))
        {
            var cards = _game.TreacheryDeck?.Items.ToList() ?? [];
            var richese = _game.IsPlaying(Faction.Richese) ? TreacheryCardManager.GetRicheseCards() : [];
            return new CardsDetermined(_game, Faction.None) { TreacheryCards = cards, RicheseCards = richese };
        }

        if (types.Contains(typeof(PerformSetup)))
            return new PerformSetup(_game, _game.NextFactionToPerformCustomSetup) { Resources = _rng.Next(0, 21), ForceLocations = new Dictionary<Location, Battalion>() };

        if (types.Contains(typeof(PerformHmsPlacement)) && _game.Players.Count > 0)
        {
            var p = _game.Players[0];
            var valid = PerformHmsPlacement.ValidLocations(_game, p).ToList();
            if (valid.Count > 0) return new PerformHmsPlacement(_game, p.Faction) { Target = valid[_rng.Next(valid.Count)] };
        }

        if (types.Contains(typeof(EndPhase))) return new EndPhase(_game, Faction.None);

        return null;
    }

    #endregion
}
