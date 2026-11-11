using System.Threading;

namespace Treachery.Server;

/// <summary>
/// Continues the pauses between phases on its own once every player who may act there has acted or passed and
/// any countdown has run out (see PhaseControl). The host's Continue stays as an override. Pauses are not
/// persisted: after a server restart, an undo or a loaded game, the current pause waits for the host or for the
/// next event.
/// </summary>
public partial class GameHub
{
    private static ConcurrentDictionary<string, Pause> PausesByGameId { get; } = [];

    private static int _lastPauseId;

    private class Pause
    {
        public int Id { get; init; }
        public Phase Phase { get; init; }
        public int Turn { get; init; }

        /// <summary>Set after an undo or a loaded game, until the next event.</summary>
        public bool Suspended { get; set; }

        /// <summary>Seats of the players who clicked Done.</summary>
        public HashSet<int> Done { get; } = [];

        public HashSet<int> Holding { get; } = [];
        public bool WasHeld { get; set; }
        public DateTimeOffset? CountdownEnds { get; set; }
        public bool CountdownExpired { get; set; }

        /// <summary>Whether the bots had a turn in this pause and did nothing.</summary>
        public bool BotsPassed { get; set; }

        public PhaseControlInfo? LastSent { get; set; }
    }

    public async Task<VoidResult> RequestPhaseDone(string userToken, string gameId) =>
        await ChangePause(userToken, gameId, (pause, seat) =>
        {
            pause.Done.Add(seat);
            pause.Holding.Remove(seat);
        });

    public async Task<VoidResult> RequestPhaseHold(string userToken, string gameId) =>
        await ChangePause(userToken, gameId, (pause, seat) =>
        {
            if (pause.CountdownEnds == null || pause.CountdownExpired) return;
            pause.Holding.Add(seat);
            pause.WasHeld = true;
        });

    public async Task<VoidResult> RequestPhaseRelease(string userToken, string gameId) =>
        await ChangePause(userToken, gameId, (pause, seat) => pause.Holding.Remove(seat));

    /// <summary>
    /// The host's Continue. It only ends the pause the host was looking at: if the game has moved on from
    /// <paramref name="eventNumber"/> events, for example because the server continued the pause first, it does nothing.
    /// </summary>
    public async Task<VoidResult> RequestPhaseContinue(string userToken, string gameId, int eventNumber)
    {
        if (!AreValid(userToken, gameId, out var user, out var game, out var error))
            return error!;

        if (!game!.Game.IsHost(user!.Id))
            return Error(ErrorType.NoHost);

        var faction = game.Game.GetPlayerByUserId(user.Id)?.Faction ?? Faction.None;
        return await ValidateAndExecute(new EndPhase(game.Game, faction), game, true, eventNumber);
    }

    private async Task<VoidResult> ChangePause(string userToken, string gameId, Action<Pause, int> change)
    {
        if (!AreValid(userToken, gameId, out var user, out var game, out var error))
            return error!;

        var player = game!.Game.GetPlayerByUserId(user!.Id);
        if (player == null)
            return Error(ErrorType.InvalidGameEvent, "Only players can do this");

        await UpdatePause(game, change: pause => change(pause, player.Seat));
        return Success();
    }

    /// <summary>
    /// Brings the game's pause up to date, tells the players if what it waits for changed, and continues it when
    /// it waits for nothing.
    /// </summary>
    /// <param name="executed">The event just executed, if any.</param>
    /// <param name="change">A change to the current pause, made before it is evaluated.</param>
    /// <param name="suspend">Starts a new pause that waits for the host or the next event.</param>
    private async Task UpdatePause(ManagedGame game, GameEvent? executed = null, Action<Pause>? change = null, bool suspend = false)
    {
        var (continueAt, info, scheduleCountdown) = await game.ProcessEventAsync(() =>
            Task.FromResult(EvaluatePause(game, executed, change, suspend)));

        if (info != null)
            await Clients.Group(game.GameId).HandlePhaseControl(info);

        if (scheduleCountdown is { } id)
        {
            var gameId = game.GameId;
            async Task Expire()
            {
                if (RunningGamesByGameId.TryGetValue(gameId, out var running))
                    await UpdatePause(running, change: pause =>
                    {
                        if (pause.Id == id) pause.CountdownExpired = true;
                    });
            }

            if (TestHooks.Delay != null)
                TestHooks.Delay(PhaseControl.CountdownMilliseconds, "PhaseCountdown " + game.GameId, Expire);
            else
                _ = Task.Delay(PhaseControl.CountdownMilliseconds).ContinueWith(_ => Expire());
        }

        if (continueAt is { } eventCount)
            await ValidateAndExecute(new EndPhase(game.Game, Faction.None), game, true, eventCount);
    }

    /// <returns>
    /// The number of events the game has when it should be continued, the info to send if it changed, and the id
    /// of a pause whose countdown just started.
    /// </returns>
    private static (int?, PhaseControlInfo?, int?) EvaluatePause(ManagedGame managed, GameEvent? executed, Action<Pause>? change, bool suspend)
    {
        var g = managed.Game;
        var pause = PausesByGameId.GetOrAdd(managed.GameId, _ => NewPause(g, null, true));

        if (suspend)
        {
            pause = NewPause(g, pause, true);
        }
        else if (executed != null)
        {
            if (executed is EndPhase || pause.Phase != g.CurrentPhase || pause.Turn != g.CurrentTurn)
            {
                pause = NewPause(g, pause, false);
            }
            else
            {
                pause.Suspended = false;
                pause.BotsPassed = false;
                if (executed.Player != null)
                {
                    pause.Holding.Remove(executed.Player.Seat);
                    if (PhaseControl.FinishesPause(g, executed)) pause.Done.Add(executed.Player.Seat);
                }
            }
        }

        PausesByGameId[managed.GameId] = pause;
        change?.Invoke(pause);

        var requirements = pause.Suspended ? null : PhaseControl.Determine(g);
        if (requirements == null)
            return (null, Send(pause, new PhaseControlInfo()), null);

        var botsDone = pause.BotsPassed || g.Participation.BotsSpeed == 0 || !g.Players.Any(p => p.IsBot);
        var waitingFor = requirements.WaitFor
            .Where(p => !pause.Done.Contains(p.Seat) && !(p.IsBot && botsDone))
            .Select(p => p.Seat)
            .ToArray();

        int? countdownStarted = null;
        if (requirements.Countdown && pause.CountdownEnds == null)
        {
            pause.CountdownEnds = TestHooks.Now().AddMilliseconds(PhaseControl.CountdownMilliseconds);
            countdownStarted = pause.Id;
        }

        var countdownDone = !requirements.Countdown || (pause.Holding.Count == 0 && (pause.CountdownExpired || pause.WasHeld));

        if (waitingFor.Length == 0 && countdownDone && botsDone)
            return (g.History.Count, null, countdownStarted);

        var running = requirements.Countdown && !pause.CountdownExpired && !pause.WasHeld;
        var info = new PhaseControlInfo
        {
            Active = true,
            WaitingFor = waitingFor,
            Countdown = requirements.Countdown,
            CountdownEnds = running ? pause.CountdownEnds : null,
            Holding = pause.Holding.Order().ToArray()
        };

        return (null, Send(pause, info), countdownStarted);
    }

    private static PhaseControlInfo CurrentPhaseControl(string gameId) =>
        PausesByGameId.TryGetValue(gameId, out var pause) ? pause.LastSent ?? new() : new();

    private static Pause NewPause(Game g, Pause? previous, bool suspended) => new()
    {
        Id = Interlocked.Increment(ref _lastPauseId),
        Phase = g.CurrentPhase,
        Turn = g.CurrentTurn,
        Suspended = suspended,
        LastSent = previous?.LastSent
    };

    /// <summary>The info if it differs from what the players last got, else null. Nothing is sent before a first active pause.</summary>
    private static PhaseControlInfo? Send(Pause pause, PhaseControlInfo info)
    {
        var last = pause.LastSent;
        if (last == null ? !info.Active : SameInfo(last, info))
            return null;

        pause.LastSent = info;
        return info;
    }

    private static bool SameInfo(PhaseControlInfo a, PhaseControlInfo b) =>
        a.Active == b.Active &&
        a.WaitingFor.SequenceEqual(b.WaitingFor) &&
        a.Countdown == b.Countdown &&
        a.CountdownEnds == b.CountdownEnds &&
        a.Holding.SequenceEqual(b.Holding);
}
