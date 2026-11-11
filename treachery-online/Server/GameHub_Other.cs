namespace Treachery.Server;

public partial class GameHub
{
    public async Task<Result<ServerInfo>> Connect()
    {
        await RestoreGamesIfServerJustStarted();
        
        var result = new ServerInfo
        {
            AdminName = Configuration["GameAdminUsername"] ?? "admin",
            ScheduledMaintenance = MaintenanceDate,
        };

        return Success(result);
    }

    private async Task CleanupUserTokensIfNeeded()
    {
        if (LastCleanedUpUserTokens.AddHours(CleanupFrequencyHours) > TestHooks.Now())
            return;

        await CleanupUserTokens();
    }

    private static void Log(string message)
    {
        Console.WriteLine(message);
    }

    private async Task CleanupUserTokens()
    {
        var now = TestHooks.Now();
        if (LastCleanedUpUserTokens.AddHours(CleanupFrequencyHours) > now)
            return;
        
        LastCleanedUpUserTokens = now;
        
        Log($"{nameof(CleanupUserTokensIfNeeded)} {UsersByUserToken.Count} {ConnectionInfoByUserId.Count}");
        
        foreach (var tokenAndInfo in UsersByUserToken.ToArray())
        {
            var age = now.Subtract(tokenAndInfo.Value.LoggedInDateTime).TotalDays;
            if (age >= MaximumLoginTimeDays)
            {
                UsersByUserToken.Remove(tokenAndInfo.Key, out _);
            } 
        }

        await using (var db = GetDbContext())
        {
            var expired = db.UserTokens.AsEnumerable()
                .Where(t => now.Subtract(t.Created).TotalDays >= MaximumLoginTimeDays)
                .ToList();
            db.UserTokens.RemoveRange(expired);
            await db.SaveChangesAsync();
        }

        foreach (var userIdAndConnectionInfo in ConnectionInfoByUserId)
        {
            foreach (var gameId in userIdAndConnectionInfo.Value.GetGameIdsWithOldConnections(MaximumLoginTimeDays).ToArray())
            {
                await RemoveFromGroup(gameId, userIdAndConnectionInfo.Key);
            }
        }
    }

    private static void CleanupScheduledGamesIfNeeded()
    {
        if (LastCleanedUpScheduledGames.AddHours(CleanupFrequencyHours) > TestHooks.Now())
            return;
        
        Log($"{nameof(CleanupScheduledGamesIfNeeded)} {ScheduledGamesByGameId.Count}");
        
        var thresholdDateTime = TestHooks.Now().AddHours(-4);
        foreach (var gameIdAndGame in ScheduledGamesByGameId.Where(g => g.Value.DateTime < thresholdDateTime).ToArray())
        {
            ScheduledGamesByGameId.Remove(gameIdAndGame.Key, out _);
        }
        
        LastCleanedUpScheduledGames = TestHooks.Now();
    }

    public async Task<Result<string>> AdminUpdateMaintenance(string userToken, DateTimeOffset maintenanceDate)
    {
        if (!UsersByUserToken.TryGetValue(userToken, out var user) || user.Username != Configuration["GameAdminUsername"])
            return Error<string>(ErrorType.InvalidUserNameOrPassword);

        MaintenanceDate = maintenanceDate;
        return await Task.FromResult(Success("Maintenance window updated"));
    }

    public async Task<Result<string>> AdminPersistState(string userToken)
    {
        if (!UsersByUserToken.TryGetValue(userToken, out var user) || user.Username != Configuration["GameAdminUsername"])
            return Error<string>(ErrorType.InvalidUserNameOrPassword);
        
        var (amountOfNewGames, amountOfUpdatedGames, amountOfUnchanged, amountOfDeletedGames) = await PersistRunningGames();
        var amountOfScheduledGames = await PersistScheduledGames();
            
        return Success($"Games: {amountOfNewGames} new, {amountOfUpdatedGames} updated, {amountOfUnchanged} unchanged, {amountOfDeletedGames} deleted. Scheduled games: {amountOfScheduledGames}.");
    }
    
    private async Task PersistScheduledGamesIfNeeded()
    {
        if (LastPersistedScheduledGames.AddMinutes(PersistFrequencyMinutes) > TestHooks.Now())
            return;

        await PersistScheduledGames();
    } 
    
    private async Task<(int amountOfNewGames, int amountOfUpdatedGames, int amountOfUnchanged, int amountOfDeletedGames)> PersistRunningGames()
    {
        if (LastRestored == default)
            return (0, 0, 0, 0);
        
        var now = TestHooks.Now();
        
        Log($"{nameof(PersistRunningGames)} started...");

        var amountOfNewGames = 0;
        var amountOfUpdatedGames = 0;
        var amountOfDeletedGames = 0;
        var amountOfUnchanged = 0;
        
        await using (var context = GetDbContext())
        {
            foreach (var (key, game) in RunningGamesByGameId.Where(x => x.Value.LastActivity > LastPersistedRunningGames))
            {
                var persistedGame = await context.PersistedGames.FirstOrDefaultAsync(g => g.GameId == game.GameId);
                if (persistedGame != null)
                {
                    if (persistedGame.LastAction == game.LastActivity)
                    {
                        amountOfUnchanged++;
                        continue;
                    }
                    
                    persistedGame.GameState = GameState.GetStateAsString(game.Game);
                    persistedGame.GameParticipation = Utilities.Serialize(game.Game.Participation);
                    persistedGame.StatisticsSent = game.StatisticsSent;
                    persistedGame.LastAsyncPlayMessageSent = game.LastAsyncPlayMessageSent;
                    persistedGame.LastAction = game.LastActivity;
                    context.PersistedGames.Update(persistedGame);
                    amountOfUpdatedGames++;
                }
                else
                {
                    context.PersistedGames.Add(new PersistedGame
                    {
                        CreationDate = game.CreationDate,
                        CreatorUserId = game.CreatorUserId,
                        GameId = key,
                        GameState = GameState.GetStateAsString(game.Game),
                        GameName = game.Name,
                        GameParticipation = Utilities.Serialize(game.Game.Participation),
                        HashedPassword = game.HashedPassword,
                        ObserversRequirePassword = game.ObserversRequirePassword,
                        StatisticsSent = game.StatisticsSent,
                        LastAsyncPlayMessageSent = game.LastAsyncPlayMessageSent,
                        LastAction = game.Game.History.LastOrDefault()?.Time ?? game.CreationDate
                    });
                    amountOfNewGames++;
                }

                await context.SaveChangesAsync();
                context.ChangeTracker.Clear();
            }

            foreach (var persistedGame in (await context.PersistedGames.ToListAsync())
                     .Where(persistedGame => persistedGame.GameId != null && !RunningGamesByGameId.ContainsKey(persistedGame.GameId)))
            {
                context.Remove(persistedGame);
                amountOfDeletedGames++;
            }

            LastPersistedRunningGames = now;
                
            await context.SaveChangesAsync();
            context.ChangeTracker.Clear();
        }
        
        Log($"{nameof(PersistRunningGames)}: {amountOfNewGames} new, {amountOfUpdatedGames} updated, {amountOfUnchanged} unchanged, {amountOfDeletedGames} deleted.");
        
        return (amountOfNewGames, amountOfUpdatedGames, amountOfUnchanged, amountOfDeletedGames);
    }
    
    private async Task<int> PersistScheduledGames()
    {
        if (LastRestored == default)
            return 0;

        LastPersistedScheduledGames = TestHooks.Now();
        
        Log($"{nameof(PersistScheduledGames)} started...");
        
        var amountOfScheduledGames = 0;
        
        await using (var context = GetDbContext())
        {
            await context.ScheduledGames.ExecuteDeleteAsync();
            
            foreach (var (key, game) in ScheduledGamesByGameId)
            {
                var persisted = new PersistedScheduledGame
                {
                    DateTime = game.DateTime,
                    CreatorUserId = game.CreatorUserId,
                    CreatorPlayerName = game.CreatorPlayerName,
                    GameId = key,
                    Ruleset = game.Ruleset,
                    MaximumTurns = game.MaximumTurns,
                    NumberOfPlayers = game.NumberOfPlayers,
                    AllowedFactionsInPlay = game.AllowedFactionsInPlay,
                    SubscribedUsers = Utilities.Serialize(game.SubscribedUsers),
                    AsyncPlay = game.AsyncPlay
                };

                context.ScheduledGames.Add(persisted);
                await context.SaveChangesAsync();
                amountOfScheduledGames++;
            }
        }
        
        Log($"{nameof(PersistScheduledGames)}: {amountOfScheduledGames}.");
        
        return amountOfScheduledGames;
    }
    
    private async Task EraseGame(ManagedGame game)
    {
        await using var context = GetDbContext();
        var persistedGame = await context.PersistedGames.FirstOrDefaultAsync(g => g.GameId == game.GameId);
        if (persistedGame != null)
            context.Remove(persistedGame);

        await context.SaveChangesAsync();
        context.ChangeTracker.Clear();
    }

    private async Task PersistGameIfNeeded(ManagedGame game)
    {
        if (game.LastPersisted.AddMinutes(GamePersistFrequencyMinutes) > TestHooks.Now())
            return;
        
        await PersistGame(game);
    }

    private async Task PersistGame(ManagedGame game)
    {
        game.LastPersisted = TestHooks.Now();
        
        await using var context = GetDbContext();
        
        var persistedGame = await context.PersistedGames.FirstOrDefaultAsync(g => g.GameId == game.GameId);
        if (persistedGame != null)
        {
            if (persistedGame.LastAction == game.LastActivity)
                return;

            persistedGame.GameState = GameState.GetStateAsString(game.Game);
            persistedGame.GameParticipation = Utilities.Serialize(game.Game.Participation);
            persistedGame.StatisticsSent = game.StatisticsSent;
            persistedGame.LastAsyncPlayMessageSent = game.LastAsyncPlayMessageSent;
            persistedGame.LastAction = game.LastActivity;
            context.PersistedGames.Update(persistedGame);
        }
        else
        {
            context.PersistedGames.Add(new PersistedGame
            {
                CreationDate = game.CreationDate,
                CreatorUserId = game.CreatorUserId,
                GameId = game.GameId,
                GameState = GameState.GetStateAsString(game.Game),
                GameName = game.Name,
                GameParticipation = Utilities.Serialize(game.Game.Participation),
                HashedPassword = game.HashedPassword,
                ObserversRequirePassword = game.ObserversRequirePassword,
                StatisticsSent = game.StatisticsSent,
                LastAsyncPlayMessageSent = game.LastAsyncPlayMessageSent,
                LastAction = game.Game.History.LastOrDefault()?.Time ?? game.CreationDate
            });
        }

        await context.SaveChangesAsync();
    }

    public async Task<Result<string>> AdminRestoreState(string userToken)
    {
        if (!UsersByUserToken.TryGetValue(userToken, out var user) || user.Username != Configuration["GameAdminUsername"])
            return Error<string>(ErrorType.InvalidUserNameOrPassword);
        
        var (amountRunning, amountScheduled) = await RestoreGamesAndUserCache();

        return Success($"Restored: {amountRunning} running games, {amountScheduled} scheduled games");
    }

    private static bool Restoring { get; set; }
    private static readonly System.Threading.SemaphoreSlim RestoreLock = new(1, 1);
    
    // Called before any token lookup, so the first request after a restart waits for the caches
    // instead of failing with UserNotFound and logging the user out.
    private async Task RestoreGamesIfServerJustStarted()
    {
        if (LastRestored != default) return;

        await RestoreLock.WaitAsync();
        try
        {
            if (LastRestored == default)
                await RestoreGamesAndUserCache();
        }
        finally
        {
            RestoreLock.Release();
        }
    } 
    
    private async Task<(int amountRunning, int amountScheduled)> RestoreGamesAndUserCache()
    {
        Restoring = true;
        
        Log($"{nameof(RestoreGamesAndUserCache)} started...");
        
        var amountRunning = 0;
        var amountScheduled = 0;
        
        await using (var context = GetDbContext())
        {
            UsersById.Clear();
            foreach (var user in context.Users.AsNoTracking())
            {
                UsersById.TryAdd(user.Id, user);
            }

            RestoreUserTokens(context);

            RunningGamesByGameId.Clear();

            foreach (var persistedGame in context.PersistedGames.AsNoTracking())
            {

                var id = persistedGame.GameId;

                if (id == null) continue;
                try
                {
                    var gameState = GameState.Load(persistedGame.GameState);
                    var gameName = persistedGame.GameName;
                    var participation = Utilities.Deserialize<Participation>(persistedGame.GameParticipation);
                    var loadMessage = Game.TryLoad(gameState, participation, false, true, out var game);
                    if (loadMessage == null)
                    {
                        var managedGame = new ManagedGame
                        {
                            CreationDate = persistedGame.CreationDate,
                            CreatorUserId = persistedGame.CreatorUserId,
                            GameId = persistedGame.GameId,
                            Game = game,
                            Name = gameName,
                            HashedPassword = persistedGame.HashedPassword,
                            ObserversRequirePassword = persistedGame.ObserversRequirePassword,
                            StatisticsSent = persistedGame.StatisticsSent,
                            LastActivity = persistedGame.LastAction,
                            LastAsyncPlayMessageSent = persistedGame.LastAsyncPlayMessageSent,
                        };

                        RunningGamesByGameId.TryAdd(id, managedGame);
                        amountRunning++;
                    }
                }
                catch (Exception ex)
                {
                    Log("Unable to restore game " + id + ": " + ex);
                }
            }

            ScheduledGamesByGameId.Clear();
            
            foreach (var scheduledGame in context.ScheduledGames.AsNoTracking())
            {
                var id = scheduledGame.GameId;
                
                if (id == null) continue;
                
                var subscriptions = Utilities.Deserialize<Dictionary<int,SubscriptionType>>(scheduledGame.SubscribedUsers);
                var game = new ScheduledGame
                {
                    ScheduledGameId = scheduledGame.GameId,
                    DateTime = scheduledGame.DateTime,
                    CreatorUserId = scheduledGame.CreatorUserId,
                    CreatorPlayerName = scheduledGame.CreatorPlayerName,
                    Ruleset = scheduledGame.Ruleset,
                    MaximumTurns = scheduledGame.MaximumTurns,
                    NumberOfPlayers = scheduledGame.NumberOfPlayers,
                    AllowedFactionsInPlay = scheduledGame.AllowedFactionsInPlay,
                    SubscribedUsers = subscriptions,
                    AsyncPlay = scheduledGame.AsyncPlay
                };
                
                ScheduledGamesByGameId.TryAdd(id, game);
                amountScheduled++;
            }
        }
        
        LastRestored = TestHooks.Now();
        LastPersistedRunningGames = LastRestored;
        
        Log($"{nameof(RestoreGamesAndUserCache)} {amountRunning} {amountScheduled}");

        Restoring = false;
        
        return (amountRunning, amountScheduled);
    }
    
    // Tokens already in memory are kept, so an admin-triggered restore does not reset anyone's status.
    private static void RestoreUserTokens(TreacheryContext context)
    {
        var now = TestHooks.Now();
        foreach (var token in context.UserTokens.AsNoTracking())
        {
            if (now.Subtract(token.Created).TotalDays >= MaximumLoginTimeDays) continue;
            if (!UsersById.TryGetValue(token.UserId, out var user)) continue;
            UsersByUserToken.TryAdd(token.Token, new LoggedInUser(user, token.Created));
        }
    }

    public async Task<Result<string>> AdminCloseGame(string userToken, string gameId)
    {
        if (!UsersByUserToken.TryGetValue(userToken, out var user) || user.Username != Configuration["GameAdminUsername"])
            return Error<string>(ErrorType.InvalidUserNameOrPassword);

        if (RunningGamesByGameId.TryGetValue(gameId, out var game))
        {
            RunningGamesByGameId.Remove(gameId, out _);
            PausesByGameId.Remove(gameId, out _);

            foreach (var userId in game.Game.Participation.PlayerNames.Keys)
            {
                game.Game.RemoveUser(userId, true);
                await Clients.Group(gameId).HandleRemoveUser(userId, true);
                await RemoveFromGroup(gameId, userId);
            }
        }
        
        return Success("Game removed");
    }

    public async Task<Result<string>> AdminDownloadGame(string userToken, string gameId)
    {
        if (!UsersByUserToken.TryGetValue(userToken, out var user) || user.Username != Configuration["GameAdminUsername"])
            return Error<string>(ErrorType.InvalidUserNameOrPassword);

        if (!RunningGamesByGameId.TryGetValue(gameId, out var game))
            return Error<string>(ErrorType.GameNotFound);

        await Task.CompletedTask;
        return Success(GameState.GetStateAsString(game.Game));
    }
    
    public async Task<Result<string>> AdminCancelGame(string userToken, string scheduledGameId)
    {
        if (!UsersByUserToken.TryGetValue(userToken, out var user) || user.Username != Configuration["GameAdminUsername"])
            return Error<string>(ErrorType.InvalidUserNameOrPassword);

        ScheduledGamesByGameId.Remove(scheduledGameId, out _);
        return await Task.FromResult(Success("Scheduled game cancelled"));
    }
    
    public async Task<Result<string>> DeleteUser(string userToken, int userId)
    {
        if (!UsersByUserToken.TryGetValue(userToken, out var user))
            return Error<string>(ErrorType.InvalidUserNameOrPassword);

        if (userId != user.Id && user.Username != Configuration["GameAdminUsername"])
            return Error<string>(ErrorType.NotAdmin);

        foreach (var game in RunningGamesByGameId.Values.Where(x => x.CreatorUserId == userId).ToArray())
        {
            foreach (var participatingUserId in game.Game.Participation.PlayerNames.Keys)
            {
                game.Game.RemoveUser(participatingUserId, true);
                await Clients.Group(game.GameId).HandleRemoveUser(participatingUserId, true);
                await RemoveFromGroup(game.GameId, participatingUserId);
            }
            
            RunningGamesByGameId.Remove(game.GameId, out _);
            PausesByGameId.Remove(game.GameId, out _);
            await EraseGame(game);
        }

        await using var db = GetDbContext();
        await db.Users.Where(u => u.Id == userId).ExecuteDeleteAsync();
        
        return Success("User deleted");
    }
    
    public async Task<Result<AdminInfo>> GetAdminInfo(string userToken)
    {
        if (!UsersByUserToken.TryGetValue(userToken, out var user) || user.Username != Configuration["GameAdminUsername"])
            return Error<AdminInfo>(ErrorType.InvalidUserNameOrPassword);

        var result = new AdminInfo
        {
            Users = GetDbContext().Users.Select(u => new UserInfo { Id = u.Id, Username = u.Name, PlayerName = u.PlayerName, Email = u.Email, LastLogin = u.LastLogin }).ToList(),
            UsersByUserTokenCount = UsersByUserToken.Count,
            ConnectionInfoByUserIdCount = ConnectionInfoByUserId.Count,
            GamesByGameIdCount = RunningGamesByGameId.Count,
        };

        await Task.CompletedTask;
        return Success(result);
    }
    
    public async Task<VoidResult> RequestNudgeBots(string userToken, string gameId)
    {
        if (!AreValid(userToken, gameId, out _, out var game, out var error))
            return error!;

        ScheduleBotEvent(game!, true);

        await Task.CompletedTask;
        return Success();
    }
}