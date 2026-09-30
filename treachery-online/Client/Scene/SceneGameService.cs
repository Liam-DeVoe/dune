using System.Linq;

namespace Treachery.Client.Scene;

// The game service behind the /scene page: one game held in memory, played
// as one faction, with no server. Everything the in-game components read is
// derived the same way Client does it. Game events are executed locally, as
// the server would, so confirm buttons advance the scene. Login, lobby, chat
// and admin calls do nothing and report success.
public class SceneGameService : IGameService
{
    // The user id given to the person at the keyboard. Any id works; it is
    // seated in the chosen faction's seat and made host.
    private const int SceneUserId = 1;

    public Game? Game { get; private set; }
    public GameStatus? Status { get; private set; }
    public List<Type> Actions { get; private set; } = [];

    public bool InGame => Game != null;
    public Player? Player => Game?.GetPlayerByUserId(UserId);
    public Faction? Faction => Player?.Faction;
    public Phase? CurrentPhase => Game?.CurrentPhase;

    // As in Client: components see the moment the host is choosing to undo
    // to, while the members above stay on the live game.
    public UndoSession? UndoSession { get; private set; }
    private Game? ShownGame => UndoSession?.Preview ?? Game;
    Game? IGameService.Game => ShownGame;
    Player? IGameService.Player => ShownGame?.GetPlayerByUserId(UserId);
    GameStatus? IGameService.Status => UndoSession?.PreviewStatus ?? Status;
    List<Type> IGameService.Actions => UndoSession?.Preview != null ? [] : Actions;
    Phase? IGameService.CurrentPhase => ShownGame?.CurrentPhase;

    public void OpenUndo()
    {
        if (Game == null || UndoSession != null) return;
        UndoSession = new UndoSession(Game, SceneUserId);
        Refresh(nameof(OpenUndo));
    }

    public void PreviewUndo(int gap)
    {
        if (UndoSession == null) return;
        UndoSession.MoveTo(gap);
        Refresh(nameof(PreviewUndo));
    }

    public void CloseUndo()
    {
        if (UndoSession == null) return;
        EndUndoSession();
        Refresh(nameof(CloseUndo));
    }

    private void EndUndoSession()
    {
        UndoSession?.RebindLiveHistory();
        UndoSession = null;
    }

    // Runs after the session ends, so the history isn't re-bound to the game being replaced.
    private Task<string> ReplaceGame(Func<Game, Game> next, string source)
    {
        if (Game == null) return Task.FromResult("Not in game");
        EndUndoSession();
        Game = next(Game);
        UpdateStatusAndActions();
        Refresh(source);
        return Task.FromResult(string.Empty);
    }

    public bool LoggedIn => true;
    public bool IsConnected => true;
    public bool IsHost => true;
    public bool IsObserver => false;
    public bool PlayersNeedSeating => false;
    public bool IsAdmin { get; set; }
    public bool InScheduledMaintenance => false;

    public int? UserId => SceneUserId;
    public string? UserName => "scene";
    public string? UserEmail => null;
    public string? PlayerName => "Scene";
    public UserStatus? UserStatus => Shared.UserStatus.None;
    public string? GameName => "Scene";
    public string? GameId => "scene";

    public ServerInfo ServerInfo { get; } = new();
    public AdminInfo AdminInfo { get; } = new();
    public GameInfo[] OwnGames => [];
    public GameInfo[] ActiveGames => [];
    public GameInfo[] ActiveGamesWithOpenSeats => [];
    public GameInfo[] ActiveGamesWithoutOpenSeats => [];
    public ScheduledGameInfo[] ScheduledGames => [];
    public Dictionary<int, LoggedInUserInfo> RecentlySeenUsers { get; } = [];

    public float CurrentEffectVolume { get; set; } = -1;
    public float CurrentChatVolume { get; set; } = -1;
    public int BidAutoPassThreshold { get; set; }
    public bool AutoPass { get; set; }
    public bool KeepAutoPassSetting { get; set; }
    public int Timer { get; set; } = -1;
    public bool MuteGlobalChat { get; set; }
    public ISet<string> RejectedDealOffers { get; } = new HashSet<string>();
    public LinkedList<ChatMessage> Messages { get; } = [];
    public Skin CurrentSkin { get; set; } = DefaultSkin.Default;
    public MapInteraction MapInteraction { get; } = new();

    public event Action? RefreshHandler;
    public event Action? RefreshPopoverHandler;

    public event EventHandler<Location>? OnLocationSelected;
    public event EventHandler<Location>? OnLocationSelectedWithCtrlOrAlt;
    public event EventHandler<Location>? OnLocationSelectedWithShift;
    public event EventHandler<Location>? OnLocationSelectedWithShiftAndWithCtrlOrAlt;
    public event EventHandler<Location>? OnLocationRightClicked;

    // Replays a savegame into a fresh game and seats the scene's user as the
    // given faction. Throws when the savegame cannot be replayed.
    public void Load(string stateData, Faction faction)
    {
        var state = GameState.Load(stateData);
        if (state.Version != Game.LatestVersion)
            throw new InvalidOperationException($"The fixture is game version {state.Version}; this client plays version {Game.LatestVersion}.");

        var game = new Game();
        var nr = 0;
        foreach (var gameEvent in state.Events)
        {
            gameEvent.Initialize(game);
            var validation = gameEvent.Execute(false, true);
            if (validation != null)
                throw new InvalidOperationException($"Could not replay event {nr} ({gameEvent.GetType().Name}): {validation.ToString(DefaultSkin.Default)}");
            nr++;
        }

        var player = game.GetPlayer(faction)
            ?? throw new InvalidOperationException($"{DefaultSkin.Default.Describe(faction)} is not in this game. Playing: {string.Join(", ", game.Players.Select(p => p.Faction))}.");

        // Seated directly rather than through Game.AddPlayer, which would
        // write "now controls" into the report shown on the page.
        game.Participation.SeatedPlayers[SceneUserId] = player.Seat;
        game.Participation.PlayerNames[SceneUserId] = PlayerName!;
        game.Participation.Hosts.Add(SceneUserId);

        EndUndoSession();
        Game = game;
        UpdateStatusAndActions();
    }

    private void UpdateStatusAndActions()
    {
        if (Game == null) return;
        Status = GameStatus.DetermineStatus(Game, Player, !IsObserver);
        Actions = Game.GetApplicableEvents(Player, IsHost);
    }

    public void Refresh(string? source = null) => RefreshHandler?.Invoke();

    public void LocationClick(LocationEventArgs e)
    {
        if (e.Location is null) return;

        if (e.RightButton)
        {
            OnLocationRightClicked?.Invoke(this, e.Location);
            return;
        }

        if (e.ShiftKey)
        {
            if (e.CtrlKey || e.AltKey)
                OnLocationSelectedWithShiftAndWithCtrlOrAlt?.Invoke(this, e.Location);
            else
                OnLocationSelectedWithShift?.Invoke(this, e.Location);
        }
        else
        {
            if (e.CtrlKey || e.AltKey)
                OnLocationSelectedWithCtrlOrAlt?.Invoke(this, e.Location);
            else
                OnLocationSelected?.Invoke(this, e.Location);
        }
    }

    // Runs the event on the in-memory game the way the server does, then
    // refreshes as the real client does when the server echoes an event back.
    public Task<string?> RequestGameEvent<T>(T e) where T : GameEvent
    {
        if (Game == null) return Task.FromResult<string?>("Not in game");

        e.Initialize(Game);
        var validation = e.Execute(true, IsHost);
        e.Time = DateTimeOffset.Now;
        if (validation != null)
            return Task.FromResult<string?>(validation.ToString(CurrentSkin));

        UpdateStatusAndActions();
        Refresh(nameof(RequestGameEvent));
        return Task.FromResult<string?>(null);
    }

    public Task<string> RequestUndo(int untilEventNr) => ReplaceGame(g => g.Undo(untilEventNr), nameof(RequestUndo));

    public Task<string> RequestLoadGame(string state, string? skin = null)
    {
        try
        {
            Load(state, Faction ?? Shared.Faction.None);
            Refresh(nameof(RequestLoadGame));
            RefreshPopoverHandler?.Invoke();
            return Task.FromResult(string.Empty);
        }
        catch (Exception e)
        {
            return Task.FromResult(e.Message);
        }
    }

    public Task<string> RequestSetSkin(string skin)
    {
        CurrentSkin = Skin.Load(skin, DefaultSkin.Default);
        Message.DefaultDescriber = CurrentSkin;
        Refresh(nameof(RequestSetSkin));
        RefreshPopoverHandler?.Invoke();
        return Task.FromResult(string.Empty);
    }

    public Task<string> SetTimer(int value)
    {
        Timer = value;
        return Task.FromResult(string.Empty);
    }

    public Task<string> RequestSetBotSpeed(int speed)
    {
        if (Game != null) Game.Participation.BotsSpeed = speed;
        Refresh(nameof(RequestSetBotSpeed));
        return Task.FromResult(string.Empty);
    }

    public Task ExitGame() => Task.CompletedTask;
    public Task Start(string? userToken = null, string? gameId = null) => Task.CompletedTask;
    public Task RememberLogin(bool keep) => Task.CompletedTask;
    public void Logout() { }
    public void RequestReseat() { }

    private static Task<string> Ok() => Task.FromResult(string.Empty);

    public Task<Result<LoginInfo>> RequestCreateUser(string userName, string hashedPassword, string email, string playerName) => Task.FromResult(new Result<LoginInfo>());
    public Task<Result<LoginInfo>> RequestLogin(string userName, string hashedPassword) => Task.FromResult(new Result<LoginInfo>());
    public Task<VoidResult> RequestPasswordReset(string usernameOrEmail) => Task.FromResult(new VoidResult());
    public Task<Result<LoginInfo>> RequestSetPassword(string userName, string passwordResetToken, string newHashedPassword) => Task.FromResult(new Result<LoginInfo>());
    public Task<string> RequestUpdateUserInfo(string hashedPassword, string email, string playerName) => Ok();
    public Task<string> RequestSetUserStatus(UserStatus status) => Ok();
    public Task<string> RequestCreateGame(string name, string password, string? stateData = null, string? skinData = null) => Ok();
    public Task<string> RequestCloseGame(string gameId) => Ok();
    public Task<string> RequestUpdateSettings(string gameId, GameSettings settings) => Ok();
    public Task<string> RequestJoinGame(string gameId, string password, int seat) => Ok();
    public Task<string> RequestObserveGame(string gameId, string password) => Ok();
    public Task<string> RequestSetOrUnsetHost(int userId) => Ok();
    public Task<string> RequestOpenOrCloseSeat(int seat) => Ok();
    public Task<string> RequestLeaveGame() => Ok();
    public Task<string> RequestKick(int userId) => Ok();
    public Task<string> RequestScheduleGame(DateTimeOffset dateTime, Ruleset? ruleset, int numberOfPlayers, int maximumTurns, List<Faction> allowedFactionsInPlay, bool asyncPlay) => Ok();
    public Task<string> RequestCancelGame(string scheduledGameId) => Ok();
    public Task<string> RequestSubscribeGame(string scheduledGameId, SubscriptionType subscription) => Ok();
    public Task<string> RequestAssignSeats(Dictionary<int, int> seatedPlayers) => Ok();
    public Task<string> RequestRestoreRecentlyUndone() => ReplaceGame(g => g.RestoreRecentlyUndone(), nameof(RequestRestoreRecentlyUndone));
    public Task<string> RequestDismissRecentlyUndone() => ReplaceGame(g => g.DismissRecentlyUndone(), nameof(RequestDismissRecentlyUndone));
    public Task SendChatMessage(GameChatMessage e) => Task.CompletedTask;
    public Task SendGlobalChatMessage(GlobalChatMessage message) => Task.CompletedTask;
    public Task<string> AdminUpdateMaintenance(DateTimeOffset maintenanceDate) => Ok();
    public Task<string> AdminPersistState() => Ok();
    public Task<string> AdminRestoreState() => Ok();
    public Task<string> AdminCloseGame(string gameId) => Ok();
    public Task<string> AdminDownloadGame(string gameId) => Ok();
    public Task<string> AdminCancelGame(string scheduledGameId) => Ok();
    public Task<string> AdminDeleteUser(int userId) => Ok();
    public Task<string> GetAdminInfo() => Ok();
}
