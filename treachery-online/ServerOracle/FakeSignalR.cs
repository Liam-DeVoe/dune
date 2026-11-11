using System.Security.Claims;
using Microsoft.AspNetCore.Http.Features;

namespace Treachery.ServerOracle;

/// <summary>
/// What SignalR does for the hub: the open connections, the group of each game, and the callbacks
/// (<see cref="IGameClient"/> calls) each connection received during the current step. Connection ids are the
/// scenario's connection numbers as strings, matching the Rust <c>ConnectionId</c>.
/// </summary>
public sealed class FakeSignalR
{
    public SortedSet<int> Connections { get; } = [];

    /// <summary>Game id to the connections in its group. Groups without members are removed, as in SignalR.</summary>
    public SortedDictionary<string, SortedSet<int>> Groups { get; } = new(StringComparer.Ordinal);

    /// <summary>Callbacks received in the current step, each serialized as a JSON array.</summary>
    public SortedDictionary<int, List<string>> Received { get; } = [];

    public void Connect(int connection) => Connections.Add(connection);

    /// <summary>A socket closed: SignalR removes the connection from every group.</summary>
    public void Disconnect(int connection)
    {
        Connections.Remove(connection);
        foreach (var (gameId, members) in Groups.ToArray())
        {
            members.Remove(connection);
            if (members.Count == 0) Groups.Remove(gameId);
        }
    }

    public void DropAll()
    {
        Connections.Clear();
        Groups.Clear();
    }

    public void AddToGroup(int connection, string gameId)
    {
        if (!Groups.TryGetValue(gameId, out var members))
        {
            members = [];
            Groups[gameId] = members;
        }

        members.Add(connection);
    }

    public void RemoveFromGroup(int connection, string gameId)
    {
        if (!Groups.TryGetValue(gameId, out var members)) return;
        members.Remove(connection);
        if (members.Count == 0) Groups.Remove(gameId);
    }

    public void Record(IEnumerable<int> connections, string callback)
    {
        foreach (var c in connections)
        {
            if (!Received.TryGetValue(c, out var list))
            {
                list = [];
                Received[c] = list;
            }

            list.Add(callback);
        }
    }
}

/// <summary>An <see cref="IGameClient"/> proxy that records each call for a set of connections.</summary>
public sealed class RecordingClient(FakeSignalR signalR, Func<IEnumerable<int>> targets) : IGameClient
{
    private Task Record(string name, params string[] args)
    {
        var j = new JsonText().ArrStart().Str(name);
        foreach (var a in args) j.Raw(a);
        signalR.Record(targets().ToArray(), j.ArrEnd().ToString());
        return Task.CompletedTask;
    }

    private static string S<T>(T value) => Utilities.Serialize(value);

    public Task HandleGameEvent<TEvent>(TEvent evt, int newEventNumber) where TEvent : GameEvent =>
        Record("HandleGameEvent", Utilities.Serialize<GameEvent>(evt), S(newEventNumber));

    public Task HandleChatMessage(GameChatMessage gameChatMessage) => Record("HandleChatMessage", S(gameChatMessage));

    public Task HandleGlobalChatMessage(GlobalChatMessage message) => Record("HandleGlobalChatMessage", S(message));

    public Task HandleSetTimer(int value) => Record("HandleSetTimer", S(value));

    public Task HandlePhaseControl(PhaseControlInfo info) => Record("HandlePhaseControl", S(info));

    public Task HandleSetSkin(string skin) => Record("HandleSetSkin", S(skin));

    public Task HandleUndo(int untilEventNr) => Record("HandleUndo", S(untilEventNr));

    public Task HandleRestoreRecentlyUndone() => Record("HandleRestoreRecentlyUndone");

    public Task HandleDismissRecentlyUndone() => Record("HandleDismissRecentlyUndone");

    public Task HandleUpdateSettings(GameSettings settings) => Record("HandleUpdateSettings", S(settings));

    public Task HandleJoinGame(int userId, string userName, int seat) => Record("HandleJoinGame", S(userId), S(userName), S(seat));

    public Task HandleSetOrUnsetHost(int userId) => Record("HandleSetOrUnsetHost", S(userId));

    public Task HandleObserveGame(int userId, string userName) => Record("HandleObserveGame", S(userId), S(userName));

    public Task HandleOpenOrCloseSeats(int[] seats) => Record("HandleOpenOrCloseSeats", S(seats));

    public Task HandleRemoveUser(int userId, bool kick) => Record("HandleRemoveUser", S(userId), S(kick));

    public Task HandleBotSpeed(int speed) => Record("HandleBotSpeed", S(speed));

    public Task HandleLoadGame(GameInitInfo initInfo) => Record("HandleLoadGame", S(initInfo));

    public Task HandleAssignSeats(Dictionary<int, int> assignment) => Record("HandleAssignSeats", S(assignment));
}

/// <summary><c>Hub.Clients</c>. The hub only uses <c>All</c> and <c>Group</c>; the rest fail loudly.</summary>
public sealed class FakeCallerClients(FakeSignalR signalR, int caller) : IHubCallerClients<IGameClient>
{
    public IGameClient All => new RecordingClient(signalR, () => signalR.Connections);

    public IGameClient Group(string groupName) =>
        new RecordingClient(signalR, () => signalR.Groups.TryGetValue(groupName, out var m) ? m : []);

    public IGameClient Caller => new RecordingClient(signalR, () => [caller]);

    public IGameClient Others => throw Unexpected();
    public IGameClient OthersInGroup(string groupName) => throw Unexpected();
    public IGameClient AllExcept(IReadOnlyList<string> excludedConnectionIds) => throw Unexpected();
    public IGameClient Client(string connectionId) => throw Unexpected();
    public IGameClient Clients(IReadOnlyList<string> connectionIds) => throw Unexpected();
    public IGameClient GroupExcept(string groupName, IReadOnlyList<string> excludedConnectionIds) => throw Unexpected();
    public IGameClient Groups(IReadOnlyList<string> groupNames) => throw Unexpected();
    public IGameClient User(string userId) => throw Unexpected();
    public IGameClient Users(IReadOnlyList<string> userIds) => throw Unexpected();

    private static Exception Unexpected() => new InvalidOperationException("ServerOracle: the hub used a Clients member the harness does not model");
}

/// <summary><c>Hub.Groups</c>.</summary>
public sealed class FakeGroupManager(FakeSignalR signalR) : IGroupManager
{
    public Task AddToGroupAsync(string connectionId, string groupName, CancellationToken cancellationToken = default)
    {
        signalR.AddToGroup(int.Parse(connectionId), groupName);
        return Task.CompletedTask;
    }

    public Task RemoveFromGroupAsync(string connectionId, string groupName, CancellationToken cancellationToken = default)
    {
        signalR.RemoveFromGroup(int.Parse(connectionId), groupName);
        return Task.CompletedTask;
    }
}

/// <summary><c>Hub.Context</c>: only the connection id is used.</summary>
public sealed class FakeContext(string connectionId) : HubCallerContext
{
    public override string ConnectionId => connectionId;
    public override string? UserIdentifier => null;
    public override ClaimsPrincipal? User => null;
    public override IDictionary<object, object?> Items { get; } = new Dictionary<object, object?>();
    public override IFeatureCollection Features { get; } = new FeatureCollection();
    public override CancellationToken ConnectionAborted => CancellationToken.None;
    public override void Abort() { }
}
