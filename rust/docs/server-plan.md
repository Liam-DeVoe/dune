# Phase 3: the server

Plan for porting `treachery-online/Server` (the ASP.NET Core SignalR hub, about 2,000 lines of logic plus EF Core migrations) to `rust/server`. Read [PORTING.md](../../PORTING.md) first, including "Decisions for phases 3 and 4". The wire protocol is in [protocol.md](protocol.md).

Status (2026-11-08): plan written, skeleton crate compiles, the database layer opens and creates C#-compatible databases. Every hub method is a `todo!()` stub. See [Progress and next steps](#progress-and-next-steps).

## Goals and limits

- The server is authoritative. It runs the Rust engine and is the only holder of game state.
- Server logic must be equivalent to the C# server: validation and error results, game lifecycle, participation and seating, persistence, timers, e-mail content, the engine calls it makes and their order (`GameStatus.DetermineStatus` changes game state, so it is called exactly where C# calls it), undo. The transport is new and is not compared.
- Same SQLite database as the C# server, both ways: the Rust server opens a database the C# server wrote, and the C# server opens one the Rust server wrote.
- Bots play the seats without a user, as in C#. See [Bots](#bots).
- Out of scope here: what each player sees and the action options (the "Game view" section of protocol.md, owned by the client-architecture agent), and the React client.

## Architecture

```
rust/server/src
  main.rs            Program.cs + Startup.cs
  config.rs          IConfiguration: appsettings*.json + environment
  runtime.rs         hub thread, axum app, real timers
  transport/         wire format (mod.rs) and WebSocket task (ws.rs). Thin, replaceable
  hub/               GameHub*.cs, one module per partial-class file
    mod.rs           GameHub.cs: state (the C# statics), constants, AreValid, Error/Success, SendMail
    call.rs          HubCall (one variant per hub method) and Reply; GameHub::invoke
    authentication.rs, game_management.rs, game_events.rs, chat.rs, other.rs
  clients.rs         SignalR Clients/Groups: Callback (IGameClient), ConnectionSink, group membership
  db/                Server/Data + Migrations on sqlx
  model/             server types from Shared/Model (ManagedGame, GameInfo, Result, ...)
  logged_in_user.rs  LoggedInUser.cs
  json.rs            Utilities.Serialize for Participation and int-keyed dictionaries
  mail.rs            MailMessage, MailAddress, Mailer (recording and SMTP)
  render.rs          the DefaultSkin text the server needs, through rust/skin
  clock.rs, timers.rs  DateTimeOffset.Now and Task.Delay, injectable
```

The hub layer (`hub/`) is the port. It takes typed arguments ([`HubCall`](../server/src/hub/call.rs)), returns the C# result types, and emits [`Callback`](../server/src/clients.rs)s, the C# `IGameClient` calls, to groups of connections. It knows nothing about JSON envelopes or sockets. That is the layer the equivalence harness drives and compares. The transport decodes messages into `HubCall`s and encodes replies and callbacks; it can change without touching the hub.

### Concurrency

The engine's `Game` is not `Send` (`Rc`, `RefCell`, a thread-local message counter). All hub state therefore lives on one thread, a current-thread tokio runtime with a `LocalSet` (`runtime.rs`). Socket tasks on the multi-threaded axum runtime send raw text to it over a channel and get text back.

The hub thread runs each call **to completion** before it starts the next, timers included. The C# hub runs calls concurrently and interleaves them at `await` points (only `ProcessEventAsync` serializes events per game). Running them one at a time gives one of the orders C# could produce, makes behaviour deterministic for testing, and means hub methods take `&mut self` and need no locks. The per-game semaphore of `ManagedGame.ProcessEventAsync` is unnecessary and is left out.

Costs: a slow database write delays every user. SQLite on a local disk takes milliseconds, so this is acceptable. SMTP is the one slow dependency, so mails are built on the hub thread and *sent* on a separate task (`mail.rs`). In C# a send failure only prints a line, so nothing observable changes.

### Transport

[protocol.md](protocol.md) defines the format: a `call`/`reply`/`notify` envelope, the C# method names and parameter lists, and `SubmitEvent` in place of the 140 `Request<Event>` methods. `GameInitInfo.GameState` (the whole savegame) is computed by the hub, as in C#, and left out by the transport, because clients get per-player views instead.

Callbacks that carry game state (`HandleGameEvent`, `HandleUndo`, `HandleRestoreRecentlyUndone`, `HandleDismissRecentlyUndone`, `HandleLoadGame`) become `GameView` notifications built by the game-view layer. `Clients` records the user each group member joined as, so the transport knows whose view to send.

**Rule for the game-view layer:** it must never call engine methods on the authoritative game (`ManagedGame.game`). Building a view will need `GetApplicableEvents`, `GameStatus.DetermineStatus` and similar, and in C# those ran in each *client's* copy of the game. Several of them change state (`DetermineStatus`, `NextPlayerToBattle` via `GetApplicableEvents`, hazards.md 4.3). Run on the server's game, they would make it diverge from what the C# server holds. The view layer must work on a separate replica, for example a second `Game` per running game that is kept in step by replaying the same events, the way C# clients did. The authoritative game sees only the engine calls the C# server made.

## Inventory and Rust mapping

Lines refer to `treachery-online/Server` unless a path says otherwise. "Pkg" is the package in [Packages](#packages).

### Process and configuration

| C# | Rust | Pkg | Notes |
|---|---|---|---|
| `Program.Main` | `main.rs` | T | `Message.DefaultDescriber = DefaultSkin.Default` has no global in Rust; the server renders through `render.rs` instead. See [Hazards](#hazards) for what else the describer changes |
| `Startup.ConfigureServices` | `runtime.rs` | T | SignalR options: `MaximumReceiveMessageSize = 4194304`, `EnableDetailedErrors` |
| `Startup.Configure` | `runtime.rs`, `db::Db::open` | T, D | Static files with `index.html` fallback, HTTPS redirect and HSTS outside development, `/gameHub` becomes `/ws`, `Database.Migrate()` if migrations are pending |
| `appsettings.json` keys | `config.rs` (done) | | `ConnectionStrings:TreacheryDatabase`, `GameEndEmailUsername`, `GameEndEmailPassword`, `GameEndEmailFrom`, `GameEndEmailTo`, `GameAdminUsername`. Same files and environment variables as ASP.NET Core |
| `GlobalUsings.cs` | none | | |

### `GameHub.cs`

| Member | Rust | Pkg |
|---|---|---|
| constants `CleanupFrequencyHours` 7, `ServerStatusFrequencyMs` 6000, `PersistFrequencyMinutes` 30, `MaximumLoginTimeDays` 90, `ActiveGameThresholdMinutes` 120, `GamePersistFrequencyMinutes` 5, `MaximumNumberOfGamesPerPlayer` 10 | `hub/mod.rs` consts (done) | |
| statics `UsersByUserToken`, `UsersById`, `ConnectionInfoByUserId`, `RunningGamesByGameId`, `ScheduledGamesByGameId`, `RunningGames`, `ScheduledGames`, `RecentlySeenUsers`, `LastRestored`, `LastPersistedRunningGames`, `LastPersistedScheduledGames`, `MaintenanceDate`, `LastCleanedUpUserTokens`, `LastCleanedUpScheduledGames`, `LastUpdatedServerStatus` | `GameHub` fields (done) | |
| `GetDbContext` | `services.db` | |
| `GenerateToken` | `IdSource::generate_token` | T (random), H1 |
| `Error`, `Error<T>`, `Success`, `Success<T>` | `GameHub::error`, `success`, `success_with` (done) | |
| `AreValid` (both overloads) | `GameHub::are_valid` | H4 |
| `SendMail` | `GameHub::send_mail` + `Mailer` | H4, T |
| `GenerateStreamFromString` | `mail::Attachment` holds text | |

### `GameHub_Authentication.cs` (package H1)

`RequestCreateUser` :5, `LoginAndCreateToken` :76, `RequestLogin` :92, `RequestPasswordReset` :113, `RequestSetPassword` :165, `GetLoginInfo` :191, `RequestUpdateUserInfo` :201, `RequestSetUserStatus` :221. All in `hub/authentication.rs`. Two mails: welcome and password reset. H1 also owns `LoggedInUser.cs` (`logged_in_user.rs`, done) and `MailAddress::parse`.

### `GameHub_GameManagement.cs` (package H2)

`RequestCreateGame` :7, `RequestScheduleGame` :61, `RequestCancelGame` :93, `RequestLoadGame` :112, `RequestAssignSeats` :147, `RequestJoinGame` :164, `RequestSubscribeGame` :216, `RequestOpenOrCloseSeat` :240, `RequestSetOrUnsetHost` :256, `RequestLeaveGame` :275, `RequestKick` :300, `RequestUpdateSettings` :318, `RequestCloseGame` :335, `AddToGroup` :358, `RemoveFromGroup` :370, `RequestObserveGame` :380, `RequestReconnectGame` :413, `RequestGameState` :433, `RequestSetSkin` :450, `RequestUndo` :463, `RequestRestoreRecentlyUndone` :480, `RequestDismissRecentlyUndone` :497, `RequestSetBotSpeed` :514, `RequestHeartbeat` :531, `FilteredServerStatus` :545, `RunTasksIfNeeded` :590, `UpdateServerStatusIfNeeded` :602, `ExtractScheduledGameInfo` :627, `SendEndOfGameMail` :643, `ExtractGameInfo` :693. All in `hub/game_management.rs`. H2 also owns `Shared/Model/UserConnections.cs` (`model/user_connections.rs`).

Engine calls made here, all ported already: `Game::new`, `GameState::load`, `Game::try_load`, `GameState::get_state_as_string`, the `Game_Participation` members (`add_player`, `set_or_unset_host`, `is_host`, `is_player`, `is_open`, `was_kicked`, `user_id_in_seat`, `remove_user`, `number_of_seated_players`, `number_of_hosts`, `open_or_close_seat`, `add_observer`, `is_observer`, `is_participant`, `reset_seats`, `seat_is_available`, `number_of_bots`), `undo`, `restore_recently_undone`, `dismiss_recently_undone`, `Game::determine_approximate_ruleset`.

### `GameHub_GameEvents.cs` (package H3)

| Member | Rust |
|---|---|
| `RequestEstablishPlayers` :8 | `request_establish_players` |
| 138 one-line `Request<Event>` methods :35-173 | `submit_event`, which dispatches on the event type |
| `SetTimer` :175 | `set_timer` |
| `ProcessGameEvent` :187 | `process_game_event` |
| `ValidateAndExecute` :195 | `validate_and_execute` |
| `SendAsyncPlayMessagesIfApplicable` :226 | `send_async_play_messages_if_applicable`; its self-rescheduling `Task.Delay` is `TimerAction::SendAsyncPlayMessages` |
| `ScheduleBotEvent` :325 | `schedule_bot_event`; its delayed call is `TimerAction::PerformBotEvent` |
| `PerformBotEvent` :341, `GetOrInitializeBot` :402, `DetermineBotDelay` :415 | `perform_bot_event`, `get_or_initialize_bot`, `determine_bot_delay` |

Also `GameHub_Chat.cs`: `SendChatMessage` :5 and `SendGlobalChatMessage` :14, in `hub/chat.rs`.

### `GameHub_Other.cs` (package H4)

`Connect` :5, `CleanupUserTokensIfNeeded` :18, `Log` :26, `CleanupUserTokens` :31, `CleanupScheduledGamesIfNeeded` :68, `AdminUpdateMaintenance` :84, `AdminPersistState` :93, `PersistScheduledGamesIfNeeded` :104, `PersistRunningGames` :112, `PersistScheduledGames` :188, `EraseGame` :230, `PersistGameIfNeeded` :241, `PersistGame` :249, `AdminRestoreState` :289, `Restoring` and `RestoreLock` :299, `RestoreGamesIfServerJustStarted` :304, `RestoreGamesAndUserCache` :320, `RestoreUserTokens` :418, `AdminCloseGame` :429, `AdminDownloadGame` :449, `AdminCancelGame` :461, `DeleteUser` :470, `GetAdminInfo` :497, `RequestNudgeBots` :514. All in `hub/other.rs`. `RestoreLock` is not needed (calls never overlap).

### `Data/` and `Migrations/` (package D)

| C# | Rust |
|---|---|
| `TreacheryContext` | `db::Db` (sqlx pool, `open`, `ensure_schema`: done) |
| `User`, `PersistedGame`, `ArchivedGame`, `PersistedScheduledGame`, `PersistedUserToken` | `db/user.rs`, `persisted_game.rs`, `archived_game.rs`, `persisted_scheduled_game.rs`, `persisted_user_token.rs`: row structs and reads done, writes stubbed |
| 13 migrations and the model snapshot | `db/schema.rs`: the final schema, the migration history (done) |

### Types from `Shared` that only the server uses

| C# (`Shared/`) | Rust | Pkg |
|---|---|---|
| `Model/ManagedGame.cs` | `model/managed_game.rs` (fields done) | |
| `Model/ScheduledGame.cs` | `model/scheduled_game.rs` (done) | |
| `Model/UserConnections.cs` | `model/user_connections.rs` | H2 |
| `Model/Result.cs`, `LoginInfo`, `GameInitInfo`, `ServerStatus`, `ServerInfo`, `AdminInfo`, `UserInfo`, `LoggedInUserInfo`, `ScheduledGameInfo`, `AvailableSeatInfo`, `Events/GameInfo.cs`, `GameChatMessage`, `GlobalChatMessage` | `model/*.rs` with their System.Text.Json encoding (done, to be checked against .NET by package X) | |
| `Model/IGameClient.cs` | `clients::Callback` (done) | |
| `Model/IGameHub.cs` | `hub::call::HubCall` (done) | |
| `Model/Participation.cs` JSON | `json.rs` (done; byte-identical on the 24 games of a real database) | |
| `Model/Skin.cs`, `DefaultSkin.cs` | crate `rust/skin` ([skin.md](skin.md)), `render.rs` facade (done) | S |

## Database

The schema is EF Core's model after the last migration, `20260921083521_UserTokensAdded`. I got it by running the built C# server (`Server/bin/Debug/net10.0`) on an empty file, which runs `Database.Migrate()` in `Startup.Configure`. Its `sqlite_master` is in `server/tests/golden/ef_schema.json`, and the database itself is `server/tests/fixtures/csharp_fresh.db`.

| Table | Columns (SQLite type, C# type) |
|---|---|
| `Users` | `Id` INTEGER PK AUTOINCREMENT; `Email`, `HashedPassword`, `Name`, `PlayerName` TEXT NOT NULL; `PasswordResetToken` TEXT NULL; `LastLogin`, `PasswordResetTokenCreated` TEXT (DateTimeOffset) |
| `PersistedGames` | `Id` PK; `CreationDate`, `LastAction`, `LastAsyncPlayMessageSent` TEXT (DateTimeOffset); `CreatorUserId` INTEGER; `GameId`, `GameName`, `HashedPassword` TEXT NULL; `GameParticipation`, `GameState` TEXT; `ObserversRequirePassword`, `StatisticsSent` INTEGER (bool). Index `IX_PersistedGames_GameId` |
| `ScheduledGames` | `Id` PK; `AllowedFactionsInPlay` TEXT (JSON array, EF primitive collection); `AsyncPlay` INTEGER; `CreatorPlayerName`, `GameId`, `SubscribedUsers` TEXT; `CreatorUserId`, `MaximumTurns`, `NumberOfPlayers` INTEGER; `DateTime` TEXT; `Ruleset` INTEGER NULL |
| `ArchivedGames` | `Id` PK; `CreatorUserId`; `GameName`, `GameParticipation`, `GameState` TEXT. Never used by the hub |
| `UserTokens` | `Token` TEXT PK; `UserId` INTEGER; `Created` TEXT |
| `__EFMigrationsHistory`, `__EFMigrationsLock` | EF bookkeeping. History rows have `ProductVersion` `10.0.12` |

Value formats (`db/mod.rs`): `DateTimeOffset` is written as EF writes it, `yyyy-MM-dd HH:mm:ss.FFFFFFFzzz` (`db/date_time.rs`). It is read as EF reads it, so rows from sqlite3 or scripts work too. Text without an offset is UTC, because Microsoft.Data.Sqlite parses with `DateTimeStyles.AssumeUniversal`, so the machine's time zone doesn't matter. ISO shapes with `T`, `Z`, `+hh`, `+hhmm` and fractions of any length are accepted with .NET's values (`tests/db_dates.rs` checks this against EF under three time zones; [escapes.md](escapes.md) "Dates without an offset"). Booleans 0/1, enums as their number. `GameParticipation` and `SubscribedUsers` are `Utilities.Serialize` JSON, `GameState` is `GameState.GetStateAsString`.

Migration policy (`db/schema.rs`): an empty file gets the final schema and the full history, so the C# server sees nothing to migrate. A file with the full history opens as is. Any other history is refused with a message to run the C# server once. Porting EF's table-rebuild migrations for old databases is not worth it.

Already verified:

- `cargo test -p treachery-server --test schema` checks that a database created by `Db::open` has exactly the C# `sqlite_master` entries (byte for byte, including EF's CRLF line ends in one statement) and history, that the C# fixture opens, and that an unknown history is refused.
- I started the C# server on a database created by `Db::open`. It applied no migration and left the file byte-identical.
- With `TREACHERY_DB=treachery-online/Server/treachery.db`, the same test file reads every row of a real database (18 users, 24 games): every date parses, every participation re-serializes to the stored text byte for byte, and every savegame loads. That file is local and git-ignored.

Queries: each `db/*.rs` lists the C# LINQ it ports, with its line. Port them to the SQL EF generates (log it with `LogTo` in the harness when unsure). In particular, keep the SQLite `trim`/`lower` that EF emits on the column side and the .NET `Trim().ToLower()` that EF evaluates on the parameter side. EF's `Update(entity)` writes every column; `SaveChanges` on a tracked entity writes only the changed ones.

## Bots

Ported (2026-11-11). `hub/game_events.rs` has `ScheduleBotEvent`, `PerformBotEvent`, `GetOrInitializeBot` and `DetermineBotDelay`; the bots are the `treachery-bots` crate (bots-plan.md). The Rust-only refusals of bot rules and empty seats (in `RequestUpdateSettings`, the web lobby and the `EstablishPlayers` backstop) are gone, so the server behaves as the C# server again.

- **Live game.** The bots decide on the authoritative game, as in C#: `GetApplicableEvents` for each bot first, then the levels in order, and some decisions change the game (bots-plan.md, "Bots are not pure"). The event a bot returns is executed as built, not as a JSON copy.
- **One bot per faction**, in `ManagedGame::bots` (C# `Dictionary<Faction, IBot> Bots`, insertion order), created when that faction's bot is first asked and rebound with `SetGameAndPlayer` on every later ask. It lives as long as the `ManagedGame`: across events, undo, restore and dismiss, but not across a restart or `AdminRestoreState`, which create new `ManagedGame`s. Before factions are chosen every bot has `Faction.None` and they share one bot, as in C#.
- **Delayed turns hold the game object.** C#'s `Task.Delay(...).ContinueWith(_ => PerformBotEvent(managedGame))` captures the `ManagedGame`, so a turn scheduled before a restore runs on the old object (and can persist it and send callbacks to its group). `TimerAction::PerformBotEvent` carries an id; the hub keeps the `ManagedGameRef` in `bot_timer_games` until the timer fires, because timer actions must be `Send`.
- **`ScheduleBotEvent(immediate: true)`** (`RequestNudgeBots`) runs the turn inside the call; its exceptions are swallowed, as the unawaited C# task swallows them. A Rust panic that isn't a C# exception is re-raised.
- **Delays.** `Task.Delay` throws for a delay below -1, which a negative bot speed gives; the port throws `ArgumentOutOfRangeException` there too (the client only offers 0, 1, 2 and 4).
- **Randomness.** C# orders the bots with `Deck<Player>.Randomize` and gives each bot `new LoggedRandom()`, both unseeded. The server takes both from `IdSource::new_random` (`RandomIds`: a seed from the OS). The harness seeds them through the C# hook `TestHooks.NewRandom` and `SequentialIds` (seeds 1, 2, 3, ... in the order asked; [csharp-hooks.md](csharp-hooks.md#hooks-added-to-server)).
- **`LogError`.** The server sets `Message.DefaultDescriber`, so a bot's `LogError` never formats or throws. The bots crate's `log_error` now does the same (it used to throw `FormatException` on a stray brace, which matched neither the server nor the oracle).

## Hazards
## Hazards

C# behaviour that is easy to port wrongly. Each needs a harness scenario.

**Identity and aliasing**

- `User` objects are shared by reference between `UsersById` and `LoggedInUser.User`, and `RequestUpdateUserInfo` writes the cached object back with `db.Users.Update(user)`, all columns. If the database row changed since the object was cached (a later login updated `LastLogin` on a different object), the stale values are written back. `logged_in_user::UserRef` is `Rc<RefCell<User>>` to keep this identity. `RequestLogin` loads a fresh `User` and does not update the caches; `LoginAndCreateToken` returns an existing token whose `LoggedInUser` still holds the old object.
- `ExtractGameInfo` copies every field except `SeatedPlayers`, which references the live `Participation.SeatedPlayers` dictionary. The cached `RunningGames` (refreshed at most every 6 s unless forced) therefore shows current seats but stale counts, until `RequestAssignSeats` replaces the dictionary object. `ExtractScheduledGameInfo` similarly shares `SubscribedUsers` and `AllowedFactionsInPlay`.
- `GameInitInfo.Participation` is the game's own object, serialized when the reply is sent.

**Exceptions that escape a hub method** (the call fails, and the state changes before the throw stay)

- `new MailAddress(email)` throws `FormatException` for an invalid address. It runs outside `SendMail`'s try/catch: in `RequestCreateUser` after the user and token are saved, in `RequestPasswordReset` after the reset token is saved, and in `SendAsyncPlayMessagesIfApplicable`, which runs inside `ValidateAndExecute` after the event executed and `HandleGameEvent` was sent, so `PersistGameIfNeeded` and `ScheduleBotEvent` are skipped. `MailAddress::parse` must accept and reject what .NET does; fuzz it against .NET.
- `GameState.Load` throws on bad JSON (`RequestCreateGame`, `RequestLoadGame`). `RestoreGamesAndUserCache` catches per game.
- `userName.Trim()` and similar on a null argument throw `NullReferenceException`. The Rust `HubCall` uses `Option` only where C# tests for null or the client may send null; elsewhere the transport rejects a null, which also fails the call.
- An engine exception inside `Execute` is caught by the engine (the "Game Error" message), but one in `Undo`, `TryLoad` or `GameStatus` escapes. `runtime.rs` catches panics around each call.

**Text**

- `ErrorDetails` for an invalid event is `validationResult.ToString()` with `Message.DefaultDescriber = DefaultSkin.Default`, which is `ToString(DefaultSkin) + "*"`. The same for `TryLoad` errors. Needs the Skin port (package S).
- The server's describer also changes the `ToString()` of `Location`, `Territory`, `TreacheryCard`, `Leader`, `ResourceCard` and `Message` inside the engine (hazards.md 6.3). The oracle runs with no describer, so the verified engine behaves as C# without one. Package E must check whether any reachable engine path depends on it. Known users: exception messages (only reachable through engine bugs) and `ClairVoyancePlayed.Parameter1`'s setter (called by bots and the client, not the server).
- User names: `.Length` counts UTF-16 units; `Trim()` and `ToLower()` use .NET's Unicode tables and the server's culture; SQLite's `trim` removes only spaces and `lower` only folds ASCII. Rust's `str::to_lowercase` maps final sigma and some characters differently. Write .NET-compatible helpers and fuzz with non-ASCII names.
- Mail bodies: `SendEndOfGameMail` builds HTML with `StringBuilder.AppendLine` (`Environment.NewLine`, `\n` on Linux and macOS), the attachment is named `savegame{yyyyMMdd.HHmm}.json`, and the priority is `Low` when at least half the players are bots. The async-play mail uses `Environment.NewLine` twice, `GetShortMessage().ToString(DefaultSkin)`, `DefaultSkin.Format` and `GameStatus.GetMessage`.

**Time**

- All times are `DateTimeOffset.Now`, local time with the machine's offset. They end up in the database, in event `Time`s inside savegames, and in `GameInfo`. `clock.rs` reproduces the local offset. The harness pins the time zone.
- Comparisons and conversions: `(int)TimeSpan.TotalMinutes` truncates, `AddHours(7) > now` compares UTC instants, `DateTime.Now - DateTimeOffset` converts `DateTime.Now` with the local offset. `LastAction == LastActivity` compares instants after a database round trip, which is exact because EF stores 7 fraction digits.
- `ValidateAndExecute` sets `e.Time` after `Execute`, when the event is already in `History`. The engine's `GameEventBase.time` is a plain field, so it must become a `Cell` (package E).

**Order**

- `ConcurrentDictionary` enumeration order is not defined, and .NET randomizes string hashes per process. So the order of `RunningGames`, `ScheduledGames` and `RecentlySeenUsers`, which token `LoginAndCreateToken` finds when a user has several, and the order in which `PersistRunningGames` and `PersistScheduledGames` insert rows (and so their `Id`s) are not deterministic in C#. The Rust maps are `BTreeMap`s. The harness compares these lists as sets and ignores `Id` in those two tables.
- `Participation`'s dictionaries are .NET `Dictionary`s with deterministic order. `RequestLeaveGame` iterates `SeatedPlayers.Keys` to send `HandleSetOrUnsetHost`, and `RequestCloseGame`, `AdminCloseGame` and `DeleteUser` iterate `PlayerNames.Keys` while `RemoveUser` removes the current key. That is legal since .NET Core 3.0 (`Remove` no longer invalidates enumerators) and visits every key once; iterate over a copy of the keys in Rust. `RemoveUser` also writes to the game log, so it is an engine call whose order matters.

**Other**

- `RequestJoinGame` adds the caller to the group even when they are already a player, and only checks the password and seat for new players.
- `SendGlobalChatMessage` and `RequestSetBotSpeed` send to every connection, not a group.
- `GetLoginInfo`, `Connect` and `RequestHeartbeat` trigger the lazy restore of the caches from the database on the first call after a start.
- `AdminRestoreState` replaces every running game with the database copy, discarding unsaved changes, while group memberships stay.

## Equivalence testing

Server logic is compared by running the same scenario through the C# hub and the Rust hub, in process, on the same virtual clock, and comparing what each step did. This is the server's counterpart of the oracle and the differ (package X).

| Part | Where |
|---|---|
| C# side | `treachery-online/ServerOracle` (`run` and `generate` commands) |
| C# hooks | `Server/TestHooks.cs`, listed in [csharp-hooks.md](csharp-hooks.md#hooks-added-to-server) |
| Rust side | `rust/server-equiv`: the runner, the comparison and the report |
| Hand-written scenarios | `rust/server-equiv/scenarios/*.jsonl` |
| Their C# transcripts | `rust/server-equiv/golden/*.transcript.jsonl`, checked in |
| Scenarios that must pass | `rust/server-equiv/expected_pass.txt` |

### Running it

```sh
# All hand-written scenarios against the checked-in goldens, with the per-method report (no .NET needed).
cargo test --release -p server-equiv --test scenarios -- --nocapture
./target/release/server-equiv run --scenarios server-equiv/scenarios --golden server-equiv/golden -v --out /tmp/rust-lines

# After changing a scenario or the C# server: rewrite the goldens (needs .NET).
server-equiv/run-oracle.sh                                # all of them
server-equiv/run-oracle.sh --in scenarios/lobby.jsonl     # one

# Generated game scenarios from oracle traces: generate, compare, delete. Runs through heavy.sh.
server-equiv/gen-check.sh traces/sample --async-play true --restart-every 40
```

`tests/scenarios.rs` fails when a golden is missing or was written for an older version of its scenario, when a hub method has no scenario, or when a scenario listed in `expected_pass.txt` doesn't pass. Other failing scenarios are reported without failing the test. `-v` (and the test's output) shows the first difference of each failing scenario: the section, the JSON path, and both values.

### Scenarios

A scenario is a JSON Lines file. The first line is a header:

```json
{"scenario":"admin","start":"2026-03-02T10:00:00+02:00","config":{"GameAdminUsername":"admin"},"seed":["INSERT INTO \"Users\" ..."]}
```

- `start`: the virtual clock's start. Keep the `+02:00` offset: the C# server converts `DateTime.Now` with the machine's zone, and the scripts run .NET with `TZ=Etc/GMT-2`.
- `config`: overrides of the default configuration. The defaults set `GameEndEmailUsername` (so mails reach the sink), `GameEndEmailFrom`, `GameEndEmailTo` and `GameAdminUsername` = `admin`.
- `seed`: SQL statements run on the new database before the first step, to start from stored users, tokens and games. A first `Connect` then restores them.
- `full: true`: write long strings in full instead of as their SHA-256 (see below).

Each later line is a step:

| `op` | Fields | Does |
|---|---|---|
| `connect`, `disconnect` | `conn` | Opens or closes connection `conn`, a number. Closing removes it from every group, as SignalR does |
| `call` | `conn`, `method`, `args`, optional `as`, `rust_only`, `note` | Calls a hub method from `conn`. `args` are the C# parameters in `Utilities.Serialize` form. `SubmitEvent` takes `[token, gameId, event]` and stands for the 139 `Request<Event>` methods |
| `advance` | `ms`, `seconds`, `minutes`, `days` | Moves the clock. Delayed calls that come due run in due-time order, each at its due time |
| `restart` | | A new server process on the same database: in-memory state, connections and pending delayed calls are gone |

`"as": "alice"` saves the `Contents` of the call's result, and later arguments refer to it as `"${alice.Token}"` or `"${s1.ScheduledGames.0.ScheduledGameId}"`. A string that is only a reference becomes the referenced value, so `"${alice.UserId}"` is a number. Don't index into lists of a `ServerStatus`; their order is not defined in C#.

`"rust_only": {"Success": false, "Error": 23}` marks a call that only the Rust server refuses. C# skips it. Rust must return a result with those members, send nothing, and leave the state as C# had it after the previous step. It was used for the bot refusals; no scenario uses it since bots were ported.

Game ids, tokens and user ids are deterministic. Tokens are `TOKEN00000000001`, GUIDs `00000000-0000-0000-0000-000000000002`, from one counter shared by both; user ids come from SQLite. Prefer `as` and references anyway.

### What is compared

After every step both sides write one line with these sections, compared byte for byte in this order:

| Section | Contents |
|---|---|
| `result` | `{"Result": <the result, Utilities.Serialize>}`, or `{"Exception": "<short type name>"}`. EF Core and SQLite exceptions are all `(database)`. The four lists of a `ServerStatus` are sorted |
| `callbacks` | Per connection, the `IGameClient` calls it received: `["HandleJoinGame", 2, "Bobby", -1]` |
| `mails` | From, To, Subject, Body, IsBodyHtml, Priority, attachments |
| `timers`, `fired` | Delayed calls scheduled in this step (with due time), and those that ran, bot turns (`PerformBotEvent <gameId>`) included |
| `randoms` | The bots' randomness asked for in this step (`TestHooks.NewRandom`): `BotOrder <gameId>` for each bot turn past the speed and phase check, `Bot <gameId> <faction>` for each new bot |
| `tables` | Every row of `Users`, `UserTokens`, `PersistedGames`, `ScheduledGames`, `ArchivedGames` as stored, sorted by key. `Id` is left out for `PersistedGames` and `ScheduledGames` |
| `games` | Per running game: the `ManagedGame` fields, `Participation`, `GameState.GetStateAsString`, the SHA-256 of the oracle's canonical state dump, and `Bots`: each bot's faction and draw count, in creation order |
| `hub` | `UsersByUserToken` (with the cached `User` and whether it is the same object as in `UsersById`), `UsersById`, `ConnectionInfoByUserId`, `ScheduledGamesByGameId`, the cached `RunningGames`, `ScheduledGames` and `RecentlySeenUsers` (sorted), the `Last...` times, `MaintenanceDate`, `Restoring`, group members, pending delayed calls |

Strings over 200 UTF-16 units in tables, games and mail attachments are written as `sha256:<hex>`, unless the scenario or `ServerOracle run --full true` asks for full text. The C# transcript writes `"="` for a state section (`tables`, `games`, `hub`) that is unchanged since the previous line. In `ErrorDetails`, C# leaves out the stack frames (`\n   at ...` lines) that a "Game Error" message gets from `Exception.ToString()`; a port can't produce them. A Rust line also carries `rustDetail`, the Rust exception message, which is shown in reports and not compared.

A scenario stops at its first failing step. Later steps would mostly show that step's consequences. The report counts, per hub method, the calls that passed, failed, reached a `todo!()`, or were not reached.

### Generated scenarios

`ServerOracle generate --in TRACES --out DIR` turns oracle traces into hub scenarios. It creates one user and connection per player. The first user creates the game and sets its settings (the lobby default is 6 players), and the others join. Every trace event is then submitted with `SubmitEvent` by the user seated for its initiator, or by the host when the trace submitted it as host. Undo lines become the three undo calls. Every seat gets a user, so these games have no bots even when the trace's rules include `FillWithBots`. It skips traces of versions before 170, whose players come from the event and not from the seated users. Options:

- `--advance-seconds S`: the clock moves S seconds before each event (default 20), so games are persisted.
- `--async-play true`: turns on async play with a one-minute interval, so mails and delayed calls happen.
- `--restart-every K`: every K events, an admin persists everything, the server restarts, and every user logs in and reconnects.
- `--max-steps N`.

The generator runs the scenario through the C# hub while it writes it, to know which user plays which faction after `EstablishPlayers`. The transcript of that run is written next to the scenario. `gen-check.sh` then runs the Rust side on the directory and deletes it.

### Generated bot games

`ServerOracle botgames --out DIR [--seed S] [--games N] [--start I] [--max-steps N]` writes scenarios in which the server's bots play whole games through the hub. Each game takes its settings from the oracle's `GameConfig.Randomize` (rules, factions, 2 to 11 players, turns) at the latest version, and a bot speed of 1, 2 or 4. A host and an observer connect, the host starts the game, and then either leaves (the bots act as hosts, because there is an observer and no host) or gives up the seat with `RequestAssignSeats` and an empty assignment (the phase control continues the pauses; the host now and then undoes one to five events and restores, dismisses or keeps them, and changes the bot speed, 0 included). From then on each step moves the virtual clock to the next delayed call. When nothing is pending, the observer calls `RequestNudgeBots` (or the host sets the speed back to 1), twice at most, and then the game counts as stalled. A game stops when it ends, stalls, or reaches the step limit (default 4000).

```sh
server-equiv/bot-check.sh --seed 1 --games 50     # generate, compare, delete; through heavy.sh
```

### Not done yet

- Database round trips: run a scenario prefix on C#, `restart` the Rust hub on the C# database file, and continue; and the reverse. `seed` covers reading hand-written rows, not files written by EF.
- Random lobby scenarios (users, games, joins, leaves, kicks, seats, hosts, schedules, restarts and clock jumps with often invalid arguments).
- `SendEndOfGameMail` is not exercised: no hand-written scenario finishes a game, and none of the sample traces' generated games ended (2026-11-10).

## Packages

Each package owns the files listed and fills in their `todo!()`s. Signatures in the skeleton may change, together with their callers, as engine-conventions.md allows.

| Pkg | Owns | Depends on | Notes |
|---|---|---|---|
| E | engine: `GameEventBase.time` as a `Cell` (and the generated `json_members.rs`, via `csharp-gen --glue`); audit of describer-dependent engine paths | none | Small. Coordinate with whoever owns the engine |
| S | Port of `Model/Skin.cs` and `DefaultSkin.cs` (about 3,200 lines), then `server/src/render.rs` | none | The client-architecture agent may need the same port for log and status text; share one crate |
| D | `server/src/db/*` queries | none | Done: schema, open, reads, date format |
| H1 | `hub/authentication.rs`, `logged_in_user.rs`, `mail.rs` (`MailAddress::parse`) | D, S (mail text has no skin; none needed) | |
| H2 | `hub/game_management.rs`, `model/user_connections.rs` | D, S (`SendEndOfGameMail`) | |
| H3 | `hub/game_events.rs`, `hub/chat.rs` | D, E, S | Bot check, async-play mails, timers |
| H4 | `hub/other.rs`, `hub/mod.rs` helpers (`are_valid`, `send_mail`) | D | Persistence, restore, cleanup, admin |
| T | `transport/*`, `runtime.rs`, `main.rs`, `config.rs`, `SmtpMailer` (lettre, STARTTLS to `smtp.strato.com:25`), `RandomIds`, static files | protocol.md, the game-view layer for `GameView` | Keep it thin |
| X | `treachery-online/ServerOracle`, the C# hooks, the Rust scenario runner, scenarios | D for meaningful runs; grows with H1-H4 | Start early: the hand-written scenarios define done for H1-H4 |

H1 to H4 touch disjoint files and can run in parallel once D's query signatures are fixed (they are, as stubs). `hub/mod.rs` and `hub/call.rs` hold the shared state and dispatch; changes there go through H4.

Suggested order: D, E and X start at once; then H4 (helpers that the others call) and H1; then H2 and H3; T last, once the game-view section exists. S runs alongside and only blocks error texts and mails.

## Open questions

1. **Skin port ownership.** The server needs `DefaultSkin` rendering for `ErrorDetails` and mails, and the client architecture probably needs it for log and status text. Who ports it, and as which crate?
2. **Old databases.** The Rust server refuses a database whose migration history is not the latest (run the C# server once to migrate). Is that acceptable, or must it apply EF's migrations itself?
3. **Bot check.** Rejecting `EstablishPlayers` when seats would become bots is a Rust-only rule, so the C# server would accept those games. Is rejecting at that point (rather than in `RequestUpdateSettings` or the lobby) what you want?
4. **Game-view replica.** The view layer must not run state-changing engine queries on the authoritative game (see [Transport](#transport)). The client-architecture agent should confirm it will keep a separate replica.
5. **Hooks in `Server`.** The harness needs the four `TestHooks` above. They change the C# server's source, though not its behaviour. OK to add?
6. **Null user tokens.** C# returns `UserTokenNotFound` for a null token; the Rust transport rejects a null argument as a failed call instead. Only a broken client sends one. Acceptable?
7. **E-mail sending.** Mails go out on a separate task, so a slow SMTP server no longer delays the call. The SMTP host is still hard-coded as in C# (`GameHub.cs:141`, with a TODO to move it to configuration). Keep it hard-coded for parity?
8. **HTTPS.** The C# app redirects to HTTPS and sends HSTS outside development. Should the Rust server terminate TLS itself, or sit behind a proxy?

## Coordinator answers (2026-11-08)

1. Skin: package S ports it as a shared crate, `rust/skin`, used by the server and the view layer.
2. Refusing old migration histories is fine.
3. Reject bot rules where settings are made (`RequestUpdateSettings` and the lobby), and keep the `EstablishPlayers` check as a backstop. Superseded on 2026-11-11: the bots are ported and all three refusals are gone (see [Bots](#bots)).
4. The view layer runs state-changing queries on a disposable copy, never on the authoritative game. The client-architecture agent owns this.
5. Yes, add the four test hooks. List them in `rust/docs/csharp-hooks.md`.
6. Null token as a failed call: fine.
7. Make the SMTP host configurable, defaulting to the C# value.
8. Sit behind a reverse proxy; no TLS in the Rust server.

## Progress and next steps

Done:

- This plan and the non-view sections of [protocol.md](protocol.md).
- `rust/server` in the workspace, compiling: config (complete), clock, timers, mail types, model types with their JSON encoding, `Participation` JSON, the database layer (schema creation and check, EF date format, row reads, user and token inserts), `GameHub` state and helpers, `HubCall`/`Reply` dispatch, `Clients` groups, the hub thread and WebSocket plumbing. Every hub method and every remaining query is a `todo!("port: File.cs:line (package X)")`.
- Tests: `db::date_time` unit tests; `tests/schema.rs` (schema equality with the C# database, opening the C# fixture, refusing unknown histories, row round trips, optional real-database read).

- Package E (2026-11-08): `GameEventBase.time` is a `Cell` (set it with `e.base().time.set(now)`); the generator seeds `GameEvent.Time` as a cell member because the write is in `Server`, outside its analysis. `json_members.rs` regenerated with `--glue`, unchanged. Describer audit in hazards.md 6.3: no reachable engine path depends on it. Bots and fuzz differ runs (seed 81, 50 games each) pass.
- Package H4 (2026-11-08): `hub/other.rs`, `GameHub::are_valid` (returns the `LoggedInUser`'s `UserRef` and the game), `is_admin_token`, `send_mail` (`SmtpHost` setting, default `smtp.strato.com`, port 25), `UserConnections::get_game_ids_with_old_connections`, the persistence queries in `db/` that H4 uses, and `net_time.rs` (`AddHours`, `AddMinutes`, `Subtract`, `TotalDays`, `TotalMinutes` with .NET rounding and range checks, for every hub package). Tests: `tests/hub_other.rs`. Not yet run against C#: waits on package X's scenario runner, and `AdminCloseGame`/`DeleteUser`/`CleanupUserTokens` on games with players call H2's `remove_from_group`.
- Package H3 (2026-11-09): `hub/game_events.rs` and `hub/chat.rs` complete; `Db::users_find` (`FindAsync`). `submit_event` sends `EstablishPlayers` to `request_establish_players` and every other type (all 140 have a C# hub method, `AutomationConfigured` included) to `process_game_event`. The bot backstop runs after `AreValid` and only while the game awaits players (in other phases C# rejects the event itself): it rejects bot rules and fewer seated users than players (version 170+: `Settings.NumberOfPlayers`; older: the names in `Players`). Tests: `tests/hub_game_events.rs`. Against C#: the hand-written `async_play`, `chat`, `events`, `undo` and the new `async_play_bad_address` (invalid address stored by `RequestUpdateUserInfo`; the event is executed and broadcast, then `FormatException`; the timer path swallows it) pass and are in `expected_pass.txt`. Generated trace scenarios (`ServerOracle generate`, with `--async-play true` for the bot traces): bots 12, 16, 20, 40 and replay-battle pass with about 750 async-play mails and 280 fired timers; fuzz 20, 38 pass, fuzz 45 passes 3276 of 9177 steps. Remaining differences, none in H3: (1) an engine "Game Error" `ErrorDetails` carries the .NET stack trace (forged ids only, unreachable.md); I drop trace steps that recorded an exception before generating. (2) Traces with 7 or more players (fuzz 13, bots 42): the generator joins users before the settings allow 7 seats, so the 7th join fails, C# fills that seat with a bot and Rust's backstop rejects the event, as intended. Package X: call `RequestUpdateSettings` with the trace's settings before the joins, and strip "Game Error" stack traces in the comparison (or have `gen-check.sh` drop exception steps).

- Package H1 (2026-11-09): `hub/authentication.rs` (all eight members), `MailAddress::parse` (`mail.rs`, a port of .NET 10's `MailAddressParser` on UTF-16 units, NFC included), `dotnet_string.rs` (`Trim`, `ToLower` in the invariant culture with ICU, `Length`, `IsNullOrWhiteSpace`), and the `db/user.rs` queries of GameHub_Authentication.cs with EF's exact SQL (`users_update_columns` writes only changed columns, comparing `DateTimeOffset` exactly as EF does, and returns `false` where EF throws `DbUpdateConcurrencyException`). Checked against .NET by the probe in `server/dotnet-probe` (modes `mail`, `chars`, `ef`): `tests/mail_address.rs` (golden of 2,090 addresses; the ignored `against_dotnet` passed 1,000,090 generated addresses, 472,272 accepted), `tests/auth_dotnet.rs` (`ToLower` of every code point and `IsWhiteSpace` of every UTF-16 unit, EF's SQL text, and which rows the queries match on non-ASCII data), `tests/hub_authentication.rs`. Scenarios `server-equiv/scenarios/authentication.jsonl` (43 steps) and `authentication_status.jsonl` with C# goldens. `authentication` passes in full once the runner names exceptions as the C# harness does (see below); `authentication_status` waits on H2 (`UpdateServerStatusIfNeeded`, `FilteredServerStatus`). Open for X: `server-equiv/src/runner.rs::exception_name` strips the namespace from Rust names, while `Harness.ExceptionName` keeps C#'s full name (`System.FormatException`) and maps EF exceptions to `(database)`; H1 reports `FormatException`, `ArgumentException` and `DbUpdateConcurrencyException`. Open for T: `RandomIds::generate_token` must draw each of the 16 characters uniformly from `[A-Za-z0-9]` with a CSPRNG (`RandomNumberGenerator.GetInt32(62)`). The client must send `Support.GetHash` passwords (protocol.md).

- Package X (2026-11-09): the equivalence harness ([Equivalence testing](#equivalence-testing)). `treachery-online/ServerOracle` (C#, `run` and `generate`), the four hooks in `Server/TestHooks.cs` ([csharp-hooks.md](csharp-hooks.md#hooks-added-to-server)), and `rust/server-equiv` (runner, comparison, per-method report, `tests/scenarios.rs`). 20 hand-written scenarios call every hub method, error paths included; all pass and are listed in `expected_pass.txt`. The exception-name question H1 raised is settled: both sides now write the short name (`FormatException`), and EF/SQLite exceptions are `(database)` on both.

- Package H2 (2026-11-09): `hub/game_management.rs` (all 30 members) and `model/user_connections.rs`. The live seat list of cached lobby entries is `model::game_info::SeatedPlayersRef`: one shared reference per `Participation.SeatedPlayers` dictionary object, kept in the new `ManagedGame::seated_players_ref`, reading the participation while that dictionary is current and keeping its contents once `RequestAssignSeats` replaces it. `RequestUpdateSettings` refuses bot rules (`game_management::BOT_RULES`, `rules_add_bots`, `BOTS_NOT_AVAILABLE`, after the C# checks; H3 can share them for the `EstablishPlayers` backstop). `net_time.rs` gained `add_milliseconds`. Tests: `tests/hub_game_management.rs`; scenario `server-equiv/scenarios/lobby_live_seats.jsonl` (live seats with a stale count, the detached dictionary after `RequestAssignSeats` and after `AdminRestoreState`, `YouAreIn` on an inactive game, `RemoveFromGroup` with two connections of one user, a heartbeat whose token cleanup removes it) with its C# golden. `SendEndOfGameMail` uses `render.rs`; its body, subject, attachment and priority are unit-tested, and generated scenarios whose game ends compare it with C#. C# parity notes worth knowing: lobby changes (joins, seats, hosts, settings) reach the database only when `LastActivity` changed since the last write, so a restore can bring back an older participation; a user's first heartbeat does not list them in `RecentlySeenUsers`.

- Package S: `rust/skin` ports `Skin.cs` and `DefaultSkin.cs` completely, checked against C# on every static object and every message of the sample traces ([skin.md](skin.md)); `render.rs` is filled in.

- Package T and client package 2 (2026-11-10): the transport, the runtime and the server side of the game view.
  - `transport/mod.rs`: the envelope (`call`/`reply`/`notify`, `ping`, case-insensitive method names), replies with `GameInitInfo` minus `GameState`, and every callback as a notification except the four game-state ones. `transport/decode.rs` binds hub-call arguments; it moved here from `server-equiv`, whose `decode.rs` now re-exports it, so scenarios and sockets decode the same way. `transport/ws.rs`: 4 MiB limit, a ping every 15 s, close after 30 s of silence. `transport/static_files.rs`: `WebRoot` (default `web/dist`) and `SharedAssetsRoot` (the C# `wwwroot`, for `/art`, `/css`, `/fonts`, `/js`, `/docs`), with `index.html` for paths without an extension. No HTTPS redirect or HSTS: the server sits behind a proxy.
  - `runtime.rs`: `RandomIds` (the OS CSPRNG through `getrandom`; tokens draw each of the 16 characters with .NET's `GetInt32` rejection sampling, GUIDs are version 4 in the `D` format), `receive` (one frame: the call, its notifications, then its views, then the reply), `connect`, `flush_views`, `POST /dev/scene`, every `urls` entry bound (`*` and `+` mean all interfaces).
  - `mail.rs` `SmtpMailer`: lettre over STARTTLS (rustls) with the configured credentials, `SmtpHost` defaulting to `smtp.strato.com`, sent on its own task; .NET's three priority headers for `Low`; text attachments as `application/octet-stream`. A failure prints a line, as `SendMail`'s catch does. Not tried against a real SMTP server.
  - `views.rs`: one `ViewReplica` per game that has connected members. The transport's sink notes which callbacks need views (`PendingViews`); after each call or timer `GameViews::flush` brings the replica up to date by executing the server's history events it lacks (serialized, as the C# client received them), and sends each member their `GameView`. It rebuilds the replica after undo, restore, dismiss and load, and whenever the `ManagedGame`, the `Participation` object or the version changed, or the replica is ahead. Views go to every member after events and participation, skin and timer changes, and to the caller after a successful create, join, observe, reconnect or game-state call. `QueryDraft`, `RequestReport` and `RequestUndoList` are answered from the replica. The replica shares the server game's `Participation`; nothing in the view layer writes it.
  - `dev.rs`: `POST /dev/scene` (`DevScene` = `true`; fixtures from `DevSceneFixtures`, default `scripts/fixtures`) calls `Connect` first (on a fresh server that restores the running games, which would otherwise replace the scene game with its unseated saved copy at the web client's first `Connect`), logs in or creates the user "Scene", closes that user's earlier scene games, creates the game from the fixture, and assigns the user the faction's seat, all through the hub methods. It uses a connection id with no socket for `AddToGroup` and leaves the group again.
  - Tests: unit tests in `transport`, `runtime`, `static_files`; `tests/transport_views.rs` drives two users through `runtime::receive` on the alliance fixture (create, seats, join, draft queries, two offers making an alliance, a refused event, undo list and undo, reports, reconnect, bad and malformed messages). End to end in a browser: `scripts/ui/e2e_server.py` (below in client-plan.md).
  - Known differences from a C# client's own game copy: the C# client ran `AddPlayer` and `RemoveUser` on its copy, which writes "X now controls Y" lines to its log; the replica only executes events, so its log lacks them (as a C# client that loaded the game later did). `HandleUpdateSettings` changes the replica's nothing; views carry no settings.

- Bots on the server (2026-11-11): see [Bots](#bots) for the port. C#: the hook `TestHooks.NewRandom` (csharp-hooks.md); the ServerOracle compares bot turns now (transcript format 2: `PerformBotEvent` timers, the `randoms` section, each game's `Bots` with draw counts), no longer strips bot rules in `generate`, and has the `botgames` command ([Generated bot games](#generated-bot-games), `bot-check.sh`). Rust: `IdSource::new_random`, `ManagedGame::bots`, `TimerAction::PerformBotEvent` with `GameHub::bot_timer_games`, `schedule_bot_event` returning a result (callers use `?`), and `GameHub::scene_games`, which keeps bots out of `POST /dev/scene` games because the C# scene page has none. Removed: the bot refusals in `RequestUpdateSettings` and `RequestEstablishPlayers`, and the `rust_only` steps of `settings.jsonl`. New hand-written scenarios: `bots` (FillWithBots with two humans, bot speed changes and a pause, nudges, undo with restore and dismiss, a kick that hands a seat to a bot, the host leaving so the bots act as hosts, a persist and restart with new bots) and `bots_seatless_host` (every seat a bot under a host, through the phase control). Results: all 21 hand-written scenarios pass with regenerated goldens; `bot-check.sh --seed 1 --games 50` passes 50 of 50 (24,668 events; 34 games ended, 15 stalled at a step the bots never take, such as host-only setup, as in C#, and one hit the 4,000-step limit); `gen-check.sh traces/sample --async-play true --restart-every 40` passes 10 of 10. The bots crate's `log_error` no longer throws (see Bots).

Next:

1. Bots: more `bot-check.sh` seeds (one seed of 50 games so far). The generated bot games have no human seats; a mix of humans and bots is only in the hand-written `bots` scenario.
2. Package X: the items under [Not done yet](#not-done-yet): database round trips, random lobby scenarios, a hand-written game that ends.
3. Package T is done. Open: an SMTP send against a real server; the lobby screens of the web client (client-plan.md package 8) so the end-to-end test can log in and seat by clicking.
