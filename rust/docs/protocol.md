# Client/server protocol

The Rust server and the React client talk JSON over one WebSocket. The server is authoritative: it holds the only copy of each game, runs the engine, and sends each player and observer their own view of it. Clients never see hidden information and never replay events (see "Decisions for phases 3 and 4" in [PORTING.md](../../PORTING.md)).

This document has two owners:

- **Everything except "Game view"** (connection, envelope, accounts, lobby, seating, chat, admin, notifications) belongs to the server, [server.md](server.md). The method and member names still follow the C# hub the protocol started from, so the web client didn't have to change when the server was rewritten.
- **[Game view](#game-view)** (what each participant sees, the actions they can take and their valid choices, and the queries the client makes while filling in an action) is owned by the client-architecture agent.

## Connection

- Endpoint: `GET /ws`, upgraded to a WebSocket, on the origin that serves the client.
- Each message is one UTF-8 text frame holding one JSON object. A binary frame closes the connection.
- Incoming messages are limited to 4 MiB (4194304 bytes). A larger frame closes the connection.
- Keepalive: the server sends a WebSocket ping every 15 s and closes a connection it has heard nothing from for 30 s. Browsers answer pings on their own. A client may also send `{"type":"ping"}`, which the server ignores.
- Creating, joining, observing or reconnecting to a game makes the *current connection* follow that game (the game's group), as the user who called. After a new WebSocket is opened, the client calls `RequestReconnectGame` again.
- When a WebSocket closes, the connection stops following every game. Nothing else changes: the user stays logged in and stays seated.
- A frame that is not a JSON object with a known `type`, or a call without a non-negative integer `id`, closes the connection. A call with an unknown method or arguments that don't decode gets an `error` reply.
- The same origin serves the web client's files (`TREACHERY_WEB_ROOT`, by default `web/dist`) and the shared assets (`TREACHERY_ASSETS_ROOT`, for `/art`, `/css`, `/fonts`, `/js`, `/docs`), and `index.html` for any other path without a file extension. Development only, with `TREACHERY_DEV_SCENE=true`: `POST /dev/scene {"fixture","faction"}` (client-plan.md, Comparing with the C# client) answers `{"userToken","gameId","userId"}`.

## Envelope

Client to server, a call:

```json
{"type":"call","id":7,"method":"RequestJoinGame","args":["<userToken>","<gameId>","<hashedPassword>",-1]}
```

- `id` is an integer the client chooses. The server echoes it in the reply. Use a fresh id per call.
- `method` is one of the methods below. Names are matched ASCII case-insensitively.
- `args` is a JSON array of the arguments in the order listed.

Server to client, the reply to one call:

```json
{"type":"reply","id":7,"result":{"Success":true,"Error":0,"ErrorDetails":"","Contents":{...}}}
{"type":"reply","id":7,"error":"Unknown method 'RequestFoo'"}
```

- `result` is the method's result, a `Result` object (below). Application errors such as a wrong password or an invalid event are a `result` with `"Success":false`.
- `error`, a string in place of `result`, means the call itself failed: unknown method, wrong arguments, or an exception inside the server. The text is for logs.

Server to client, a notification:

```json
{"type":"notify","name":"JoinGame","args":[12,"Alice",-1]}
```

Ordering:

- The server handles calls one at a time, in the order they arrive (see [server.md](server.md#threads)).
- Notifications caused by a call reach the caller before that call's reply.
- Notifications can also arrive without a call of yours: other users' actions, timers.

## Encoding

Payloads have member names in PascalCase and enums as numbers. Events and game settings are in the savegame format the engine reads and writes (`engine/src/json/mod.rs`); events submitted with `SubmitEvent` must be in this format.

| Type | JSON |
|---|---|
| `string` | string, or `null` where marked `string?` |
| `int`, `bool` | number, `true`/`false` |
| enum (`Faction`, `Ruleset`, `ErrorType`, `UserStatus`, ...) | number. Values are in `engine/src/enumerations.rs` |
| time | RFC 3339 string. The server writes UTC with microseconds, `"2026-09-18T04:14:35.622405Z"`, and reads any offset; a time without one is UTC |
| `List<T>`, `T[]`, `HashSet<T>` | array |
| `Dictionary<int, V>` | array of `[key, value]` pairs in the server's order: `[[12,3],[5,0]]`. An object would lose that order, because `JSON.parse` sorts integer keys. Arguments may also use the object form |
| `GameEvent` | `{"$type":"Bid", ...members}`, the savegame format |
| `GameSettings` | the savegame format |

On input, member order doesn't matter (except that `$type` comes first in an event), unknown members are ignored, and missing members keep their defaults.

### Result

Every call answers with a `Result`:

```json
{"Success":false,"Error":1,"ErrorDetails":"","Contents":null}
```

A void result has `"Contents":{}` on success. `ErrorType` values: `None` 0, `UserNotFound` 1, `GameNotFound` 2, `GameIdNotFound` 3, `ScheduledGameNotFound` 4, `UserNameTooShort` 5, `UserNameTooLong` 6, `UserNameExists` 7, `UserTokenNotFound` 8, `EmailExists` 9, `PlayerNameTooShort` 10, `PlayerNameTooLong` 11, `UserCreationFailed` 12, `InvalidGameVersion` 13, `InvalidUserNameOrPassword` 14, `NotAdmin` 15, `UnknownUsernameOrEmailAddress` 16, `NoEmailAddressForUser` 17, `ResetRequestTooSoon` 18, `UnknownUserName` 19, `InvalidResetToken` 20, `ResetTokenExpired` 21, `NoHost` 22, `InvalidGameEvent` 23, `TooManyGames` 24, `TooManyScheduledGames` 25, `IncorrectGamePassword` 26, `SeatNotAvailableNotOpen` 27, `SeatNotAvailableKicked` 28, `AlreadyObserver` 29, `CannotRemoveLastHost` 30, `NoCreator` 31, `AlreadyPlayer` 32, `UserNotInGame` 33.

For `InvalidGameEvent`, `ErrorDetails` is the validation message rendered as text.

### Payload objects

Members in the order the server writes them (`rust/server/src/wire.rs`).

| Object | Members |
|---|---|
| `LoginInfo` | `UserId`, `Token`, `PlayerName`, `UserName`, `Email` |
| `GameInfo`, one game in the lobby | `CreatorId`, `CreatorUsername`, `CreationDate`, `GameId`, `HasPassword`, `Name`, `MaxPlayers`, `MaxTurns`, `MainPhase`, `Phase`, `Turn`, `NrOfBots`, `NrOfPlayers`, `FactionsInPlay` [Faction], `Ruleset`, `LastActivity` (nullable), `SeatedPlayers` [[userId, seat]], `AvailableSeats` [`{Seat, Faction, IsBot}`] |
| `ScheduledGameInfo` | `ScheduledGameId`, `DateTime`, `CreatorUserId`, `CreatorName`, `NumberOfPlayers` (nullable), `MaximumTurns` (nullable), `Ruleset` (nullable), `AsyncPlay`, `AllowedFactionsInPlay` [Faction], `Subscribers` [[userId, SubscriptionType]], `SubscriberNames` [[userId, name]] |
| `LoggedInUserInfo` | `Id`, `Name`, `Status` (UserStatus), `LastSeen` (nullable) |
| `ServerStatus`, the lobby | `RunningGames` [GameInfo], `OwnGames` [GameInfo], `ScheduledGames` [ScheduledGameInfo], `RecentlySeenUsers` [LoggedInUserInfo] |
| `Participation` | `PlayerNames` [[userId, name]], `SeatedPlayers` [[userId, seat]], `Observers` [userId], `Hosts` [userId], `AvailableSeats` [seat], `Kicked` [userId], `BotsSpeed` |
| `PhaseControlInfo` | `Active` (false: the host alone continues this phase), `WaitingFor` [seat], `Countdown`, `CountdownEnds` (nullable; set while the countdown runs and nobody holds it), `Holding` [seat] |
| `GameInitInfo` | `GameId`, `GameName`, `Participation`, `PhaseControl` (nullable; set in the replies to join, observe, reconnect and game state). The client gets a [game view](#game-view) for the game itself |
| `ServerInfo` | `ScheduledMaintenance`, `AdminName` |
| `AdminInfo` | `Users` [`{Id, Username, Email, LastLogin, PlayerName}`], `UsersByUserTokenCount` (sessions), `ConnectionInfoByUserIdCount` (users with a connection that follows a game), `GamesByGameIdCount` (running games) |
| `GameChatMessage` | `TargetUserId` (negative: everyone), `SourceUserId`, `Body` |
| `GlobalChatMessage` | `SourceUserId`, `Body` |

`GameInfo.CanBeJoined` (`Phase == AwaitingPlayers && SeatedPlayers.Count < MaxPlayers || AvailableSeats.Length > 0`) is for the client to compute.

## Methods

`userToken` is `LoginInfo.Token`. "Void" means a result without contents.

### Server and accounts

| Method | Args | Result |
|---|---|---|
| `Connect` | (none) | `ServerInfo`. Call once after the socket opens |
| `RequestCreateUser` | userName, hashedPassword, email `string?`, playerName | `LoginInfo` |
| `RequestLogin` | version (the engine's `Game::LATEST_VERSION`), userName, hashedPassword | `LoginInfo` |
| `GetLoginInfo` | userToken | `LoginInfo`. Resumes a saved session |
| `RequestPasswordReset` | usernameOrEmail | void. Mails a reset token |
| `RequestSetPassword` | userName, passwordResetToken, newHashedPassword | `LoginInfo` |
| `RequestUpdateUserInfo` | userToken, hashedPassword `string?` (empty keeps the password), playerName, email `string?` | `LoginInfo` |
| `RequestSetUserStatus` | userToken, status | `ServerStatus` (scope Active) |
| `RequestHeartbeat` | userToken, scope (GameListScope: None 1, Active 2, All 3) | `ServerStatus` |

Passwords are hashed by the client, and the server stores the hash as sent. Existing accounts need the hash the C# client used: SHA-256 of the password's UTF-8 bytes, as 64 lowercase hex digits (in the browser, `crypto.subtle.digest("SHA-256", new TextEncoder().encode(password))`). `RequestUpdateUserInfo` takes the same hash, or an empty string to keep the password. Each login starts a session; a session lasts 90 days. The client should send `RequestHeartbeat` every few seconds: the user list only shows users seen in the last minute.

### Lobby and game management

| Method | Args | Result |
|---|---|---|
| `RequestCreateGame` | name, userToken, hashedPassword, stateData (a savegame, or empty for a new game), skinData (or empty) | `GameInitInfo` |
| `RequestLoadGame` | userToken, gameId, state (savegame), skin `string?` | void. Host only |
| `RequestUpdateSettings` | userToken, gameId, settings | void. Creator only. Any rules are taken |
| `RequestCloseGame` | userToken, gameId | `ServerStatus`. Creator only |
| `RequestGameState` | userToken, gameId | `GameInitInfo`, and resends the caller's game view |
| `RequestScheduleGame` | userToken, dateTime, ruleset (nullable), numberOfPlayers, maximumTurns, allowedFactionsInPlay [Faction], asyncPlay | `ServerStatus` |
| `RequestCancelGame` | userToken, scheduledGameId | `ServerStatus` |
| `RequestSubscribeGame` | userToken, scheduledGameId `string?`, subscription (SubscriptionType) | `ServerStatus` |

### Seating and participation

| Method | Args | Result |
|---|---|---|
| `RequestJoinGame` | userToken, gameId, hashedPassword, seat (-1: no particular seat) | `GameInitInfo` |
| `RequestObserveGame` | userToken, gameId, hashedPassword | `GameInitInfo` |
| `RequestReconnectGame` | userToken, gameId | `GameInitInfo` |
| `RequestLeaveGame` | userToken, gameId | `ServerStatus` |
| `RequestSetOrUnsetHost` | userToken, gameId, userId | void. Host only |
| `RequestOpenOrCloseSeat` | userToken, gameId, seat | void. Host only |
| `RequestKick` | userToken, gameId, userId | void. Host only |
| `RequestAssignSeats` | userToken, gameId, assignment [[userId, seat]] | void. Host only |
| `RequestSetBotSpeed` | userToken, gameId, speed | void. Host only. See [Bots](#bots) |
| `RequestNudgeBots` | userToken, gameId | void. Any user with a token; runs the bots' turn at once. See [Bots](#bots) |

Creating, joining, observing and reconnecting to a game, and `RequestGameState`, send the caller a game view before the reply.

### Playing

| Method | Args | Result |
|---|---|---|
| `SubmitEvent` | userToken, gameId, event | void |
| `RequestUndo` | userToken, gameId, untilEventNr | void. Host only |
| `RequestRestoreRecentlyUndone` | userToken, gameId | void. Host only |
| `RequestDismissRecentlyUndone` | userToken, gameId | void. Host only |
| `SetTimer` | userToken, gameId, value | void. Host only |
| `RequestSetSkin` | userToken, gameId, skin | void. Host only |
| `RequestPhaseDone` | userToken, gameId | void. Players only. The caller has acted or passes in the current pause, and stops holding its countdown |
| `RequestPhaseHold` | userToken, gameId | void. Players only. Holds the running countdown; does nothing when none is running |
| `RequestPhaseRelease` | userToken, gameId | void. Players only. Stops holding |
| `RequestPhaseContinue` | userToken, gameId, eventNumber | void. Host only. Ends the current pause, but only while the game has `eventNumber` events, so a click that arrives after the server already continued does nothing |

The event is a `GameEvent` in the savegame format, with `$type` set to the event class and `Initiator` set to the submitter's faction. The server sets `Time`, validates and executes it, and on success sends every participant a new view. A failed validation is `InvalidGameEvent` with the message in `ErrorDetails`, rendered with the default skin.

Pauses between phases continue on their own. After each event the server works out who the current pause waits for and whether it runs a countdown that any player can hold (`rust/server/src/pause_rules.rs`). Once nobody is left to wait for and the countdown has run out or every holder has released it, the server submits `EndPhase` itself. A player waited for is done once they call `RequestPhaseDone`, or, in the revival step, once they revive. After an undo or a loaded game the current pause waits for the host's `RequestPhaseContinue` or the next event. Games older than version 188 keep host-only control.

A valid `EstablishPlayers` event also seats the users on the players' seats, in the order they joined (`AssignSeats` notification). Seats that no user takes become bots (see [Bots](#bots)).

### Chat

| Method | Args | Result |
|---|---|---|
| `SendChatMessage` | userToken, gameId, message (GameChatMessage) | void |
| `SendGlobalChatMessage` | userToken, message (GlobalChatMessage) | void |

The server relays chat as it is: it does not check `SourceUserId` against the token, and a game message goes to everyone in the game's group whatever its `TargetUserId`. Showing only messages meant for you is the client's job.

### Admin

The token's user name must be the configured admin (`TREACHERY_ADMIN`), except for `DeleteUser`, which a user may also call on their own account. Others get `InvalidUserNameOrPassword`.

| Method | Args | Result |
|---|---|---|
| `AdminUpdateMaintenance` | userToken, maintenanceDate | string |
| `AdminPersistState` | userToken | string (a summary). Saves every game with unsaved changes now |
| `AdminRestoreState` | userToken | string (a summary). Loads the saved games that aren't running; running games stay as they are |
| `AdminCloseGame` | userToken, gameId | string. Removes everyone and deletes the game |
| `AdminDownloadGame` | userToken, gameId | string, the savegame |
| `AdminCancelGame` | userToken, scheduledGameId | string |
| `DeleteUser` | userToken, userId | string. Deletes the account and its sessions, and closes the games it created |
| `GetAdminInfo` | userToken | `AdminInfo` |

## Notifications

Each notification names what happened. Changes to the game itself arrive as game views.

| Notification | Args | Sent to |
|---|---|---|
| `GameView` | see [Game view](#game-view) | each member of the game group, their own view: after events, undo, restore, dismiss, and anything else that changes the game |
| `LoadGame` | GameInitInfo | game group, followed by a `GameView` |
| `UpdateSettings` | settings | game group |
| `JoinGame` | userId, userName, seat | game group |
| `ObserveGame` | userId, userName | game group |
| `SetOrUnsetHost` | userId | game group |
| `OpenOrCloseSeats` | [seat] | game group |
| `RemoveUser` | userId, kick | game group |
| `AssignSeats` | [[userId, seat]] | game group |
| `BotSpeed` | speed | every connection |
| `SetTimer` | value | game group |
| `PhaseControl` | PhaseControlInfo | game group, when what the current pause waits for changes |
| `SetSkin` | skin | game group |
| `ChatMessage` | GameChatMessage | game group |
| `GlobalChatMessage` | GlobalChatMessage | every connection |

The game group is every connection that created, joined, observed or reconnected to the game, until it closes or its user leaves or is kicked, the caller's included. Participation changes (`JoinGame`, `RemoveUser`, `AssignSeats`, ...) can change what each member may see, so the game-view section decides whether they are followed by a `GameView`.

## Bots

The server plays every seat without a user (`rust/server/src/server/bots.rs`; the bots are the `treachery-bots` crate).

- Which seats are bots: since engine version 170, `EstablishPlayers` fills the seats that no user takes with bots (`Settings.NumberOfPlayers` players in all). A seat also becomes a bot when its user leaves or is kicked, when the host reassigns the seats without it, or when a loaded savegame has it empty. A user who takes the seat (an open seat, `RequestJoinGame`) replaces the bot. `GameView.players[].isBot` and `GameInfo.NrOfBots` show them.
- When bots act: after every event, and after a loaded game, undo, restore or dismiss, a seat assignment, a leave or kick, a new observer, the first seated join and a bot speed change, the server schedules a bot turn, replacing one scheduled earlier, after a delay that depends on the last event and the phase, divided by the bot speed (800 ms after an `AllyPermission`, `DealOffered` or `SetShipmentPermission`, 1600 ms after a `Bid`, 2400 ms in revival and charity, 9600 ms in shipment and battle, 4800 ms otherwise). A turn does nothing while the game awaits players or the speed is 0. Otherwise each bot, in a random order, is asked for its most urgent action, then the next level, and the first action found is executed as an event of that bot. The bots end phases themselves (`EndPhase` and other host actions) only when the game has no host and at least one observer. Otherwise the phase control continues the pauses, and a turn in which no bot acts tells it the bots have passed.
- `BotsSpeed` (`Participation.BotsSpeed`, default 1) is the divisor. 0 or less pauses the bots. The client offers 0 (pause), 1, 2 and 4. Every connection gets the `BotSpeed` notification.
- `RequestNudgeBots` runs a bot turn at once (unless the bots are paused), for a game whose bots stopped (a refused bot event schedules no next turn).
- A bot is created when its faction is first asked, and keeps its state for the rest of the game, across undo. A server restart creates new bots.

## Game view

*Owned by the client-architecture agent. Design and reasons: [client-plan.md](client-plan.md). Wire types: `rust/view/src/schema.rs`, generated to TypeScript in `web/src/generated/`. What the C# client reads, per screen area: [client-inventory.md](client-inventory.md).*

The server sends each participant a `GameView` notification: what that user may see of the game, plus the actions they can take now and the valid choices for each. The client renders only views. It never replays events and holds no engine. While a player fills in an action, the client asks the server about the inputs chosen so far (a draft query), and submits the finished event with `SubmitEvent`.

### Encoding

The view is new code, not a mirror of a C# type, so it uses its own JSON conventions, chosen for the TypeScript client:

- Member names in camelCase.
- Enum values as the C# member name (`"Atreides"`, `"ShipmentAndMove"`), the keys the skin JSON uses. The TypeScript unions are in `web/src/generated/enums.ts`.
- Static game objects by their C# `Id`: locations, territories, leaders, treachery cards, and resource cards (by `SkinId`). Names, images and map positions come from the skin and from the static data file (`rust/view/examples/static_data.rs`), never from the view.
- Text that the game writes (log lines, prompts, validation messages, form headings) arrives rendered to [parts](#rich-text).

Events stay in the savegame format of [Encoding](#encoding) (PascalCase, enums as numbers), both in draft queries and in `SubmitEvent`, because the engine reads and validates exactly that format.

### When views are sent

| After | Who gets a new view |
|---|---|
| an event (`SubmitEvent`), undo, restore, dismiss, `LoadGame` | every member of the game group |
| `JoinGame`, `RemoveUser`, `AssignSeats`, `SetOrUnsetHost`, `OpenOrCloseSeats` | every member: seats decide `me`, host status decides actions, seat occupancy decides `isBot` |
| a successful `RequestCreateGame`, `RequestObserveGame`, `RequestReconnectGame`, `RequestJoinGame`, `RequestGameState` | the caller (and, for a join, every member, through `JoinGame`) |
| `SetSkin` | every member, because labels are rendered with the game's skin |
| `SetTimer` | every member: the timer is `timing.timer` |
| `UpdateSettings` | nobody: settings only matter before the game starts, and the lobby sections cover that |

Each view is complete; there are no diffs. A view of the alliance phase of a 6-player game is about 22 KB of JSON (`web/public/views/`). If size becomes a problem, the log moves out first (it is the largest part); see [client-plan.md](client-plan.md#open-questions).

### Notification

```json
{"type":"notify","name":"GameView","args":[{"eventNumber":612,"version":188,"turn":6, "...": "..."}]}
```

### Calls

| Method | Args | Result |
|---|---|---|
| `QueryDraft` | userToken, gameId, eventNumber, draft (a `GameEvent` in savegame format) | `DraftResult`, or `InvalidGameEvent` when the draft is not one of the caller's actions in view `eventNumber` (`ErrorDetails` says why) |
| `RequestReport` | userToken, gameId, index (an entry of `log.reports`) | `ReportLines` of that report, filtered for the caller as in the view; `InvalidGameEvent` for an index that has no report |
| `RequestUndoList` | userToken, gameId | host only (`NoHost` otherwise): `[{eventNumber, text: RichText, listed, turn, phase}]`, the short message of every event (`GameEvent.GetShortMessage`), whether the undo panel lists it (`UndoSession.IsListed`), and the turn and main phase it happened in (`UndoSession.MomentOf`), for the undo panel and timeline. `eventNumber` is the event's index in the history, the `untilEventNr` of `RequestUndo` |
| `RequestUndoPreview` | userToken, gameId, gap | host only (`NoHost` otherwise): the caller's `GameView` at gap `gap`, the moment just before event `gap` (the game after `RequestUndo(gap)`), with no actions and no `playableWith`, as the C# client showed `UndoSession.Preview` (`IGameService.Actions` was empty while previewing). The gap is clamped to 1..`History.Count`. Sends no notification; the live game and its views are untouched |
| `RequestSaveGame` | userToken, gameId | hosts and the admin (`CanSave` of `MenuComponent`; `NoHost` otherwise): the server's game as a savegame string (`GameState.GetStateAsString`), which the C# client built from its own copy. Always the live game, also while the host previews an undo |

These five use the same `Result` envelope as the other calls, with `Contents` in the view's camelCase encoding. A token or game the server doesn't know gives the same errors as the other calls (`UserNotFound`, `GameNotFound`).

`QueryDraft` replaces the C# components' habit of calling `Validate()` and the `Valid*`/`Max*` helpers on every render. The server sets the draft's `Initiator` to the caller's faction, so a client can't ask about another faction. A draft for an older view (`eventNumber` is not the current `History.Count`) is answered against the current game; the client discards answers for views it has replaced.

`SubmitEvent` (in [Playing](#playing)) is unchanged: the same event, with all inputs filled in.

### GameView

Fields marked *planned* are specified here and added to `schema.rs` by the packages in [client-plan.md](client-plan.md#packages). The rest exist and are tested (`rust/view/tests/slice.rs`, `view_core.rs`, `corpus.rs`). `schema.rs` documents every member.

| Field | Type | Source (C#) |
|---|---|---|
| `eventNumber` | int | `Game.History.Count` |
| `version`, `turn`, `maximumTurns` | int | `Game.Version`, `CurrentTurn`, `MaximumTurns` |
| `mainPhase`, `phase` | enum names | `CurrentMainPhase`, `CurrentPhase` |
| `viewer` | `{faction, isHost, isObserver}` | `Client.Faction`, `IsHost`, `IsObserver`, resolved from `Participation` |
| `players` | `PublicPlayer[]`, seat order | see [Visibility](#visibility) for what each holds |
| `me` | `PrivatePlayer` or null | the viewer's own shield (`PlayerPanel.razor`) |
| `board` | `BoardView` | the map (`MapComponents/*`) |
| `status` | `StatusView` | `GameStatus.DetermineStatus(Game, Player, !IsObserver)` |
| `actions` | `ActionView[]` | `Game.GetApplicableEvents(Player, IsHost)`, in that order |
| `log` | `LogView` | `Game.Reports` |
| `ruleset`, `rules`, `rulesForBots` | enum names | `Game.Ruleset`, `Rules`, `RulesForBots`. `Game.Applicable(r)` is true for a rule in either list |
| `economicsStatus` | enum name | `Game.EconomicsStatus` (`Phases.razor`) |
| `piles` | treachery deck count, discard pile (top card; with notekeeping every card, `null` where unknown), resource deck count and the top card under resource prescience during ship and move, latest spice cards A/B and, with notekeeping, their discard piles, Sand Trout, Richese's cache (count; the cards for Richese) | `CardPiles.razor`, `PlayerPanel.razor:155` |
| `tanks` | per faction: forces and special forces killed, dead leaders (face-down ones as `null` unless the viewer may identify them), Messiah | `Tanks.razor` |
| `bidding` | during the bidding main phase: auction type, the bids as the map shows them (`pass`, `bid` with amount, or `ready` in silent auctions), `auctionWon` (`RecentMilestones` has `AuctionWon`; the client turns auto-pass off), and in `Bidding`/`BlackMarketBidding` the bidding panel's lot for players (`panel`: black market or not, the card on auction when visible, "card n of m" in normal auctions, silent, current bid outside silent auctions). Observers get the `panel` of a player without prescience (deviation 5) | `BiddingPanel`, `NormalBiddingPanel`, `BlackMarketBiddingPanel`, `Bids.razor` |
| `battle` | while there is a current battle: territory, aggressor and defender, and each plan as `BattlePlan.razor` draws it: whole for its owner, for everyone once both are in, and under Atreides Karama; the aspect prescience revealed for the prescience user and their ally. *Planned* (7b): voice, battle outcome, what the battle forms need | `BattlePlans.razor`, `BattlePlan.razor`, `BattleComponent` |
| `deals` | seated viewers only: offers the viewer may accept (`AcceptableDeals`, with `Acceptance(...).Validate()` and the Accept and Decline events in savegame JSON), the viewer's own offers (`CancellableDeals`, with each recipient's answer and the Withdraw event), every deal in force (`CurrentDeals`). An offer's `id` is its event number | `DealComponent`, `DealToasts`, `MenuComponent` (the count on the Deals icon) |
| `botsSpeed` | `Participation.BotsSpeed` | `MenuComponent` (host's bot controls) |
| `notes` | with notekeeping (`AssistedNoteKeepingEnabled(Player)`): per other player, spice texts, reserves, cards owned (a card, or `null` for one the viewer doesn't know), revealed traitors and face dancers, victory points, Messiah state | `NoteComponent.razor` |
| `tracker` | Atreides from `SelectingTraitors` on: cards in play (with Richese's when Richese plays), `TraitorsInPlay`, per-player card, traitor and face dancer counts, resource discard piles, remaining resource cards as a sorted set. The client keys its local notes by `timing.started` instead of `Game.Seed` | `TrackerComponent`, `AtreidesIntelligence.cs` |
| `mentat` | the same for everyone: `TreacheryCardManager.GetCardsInPlay`, `GetRicheseCards` when Richese plays, `Map.GetResourceCardsInPlay` (by `SkinId`). The list's other sections follow from `rules` and the players' factions | `MentatComponent` (the menu's "List of Cards") |
| `timing` | game start, last activity, duration, time spent per player in bidding, ship and move, and battle (seconds), and the bidding timer (`Client.Timer`, which the server sets on the replica) | `Phases.razor`, `TurnBanner`, `GameStatusComponent` |
| `ending` | in `Phase.GameEnded` only: `winners` (`[{faction, name}]`) and `story`, null under `Rule.DisableEndOfGameReport`. The story is a list of blocks in the order `StoryComponent` draws them (`setup`, `prediction`, `purchase`, `turn`, `situation` with a `StoryPlayerInfoComponent` table, `battleFor`, `milestone`, `battle`, `strongholds`, `winner`, `ended`), built by replaying the history in a fresh game (`ending.rs`). The same for every viewer; secrets of factions in `SecretsRemainHidden` are left out (deviation 2) | `WinnerPanel`, `Story*` |
| `factionSheets` | the same for everyone: per player with a rules sheet, in player order, `Skin.GetFactionSheet` (`atStart` as parts: text, or a `location`/`territory` id for a place chip, with the sheet's placeholders filled as `FactionPopupHtml` does; `freeRevival`, `theme`, `advantages`, `advancedAdvantages`) and `html` = `GetFactionInfo_HTML(game, faction, "")`. Public: the sheets read only the game's rules and which factions play (`skin_texts.rs`) | `Support.FactionPopupHtml` (the faction popover on a seat) |
| `ruleLabels` | the same for everyone: `ruleset` = `Describe(Game.Ruleset)`, and `groups` (`[{group, rules}]`, described): the groups of `Game.Rules` other than CoreBasic, Bots and None, in order of their first rule | `Phases.razor` `TurnPopover` |
| `recentMilestones` | the same for everyone: `Game.RecentMilestones`, the milestones of the latest event (`Milestone` names). They mark public outcomes; C# played their sounds for every client | `Client.PlaySoundsForMilestones` |

`PublicPlayer`: `faction`, `seat`, `name`, `isBot`, `ally`, `forcesInReserve`, `specialForcesInReserve`, `forcesKilled`, `specialForcesKilled`, `cardCount`, `bribes`, `resourcesAfterBidding`, `extortion`, `bankedResources`, `techTokens`, `ambassadors`, `hasSpecialForces`, `hasRoomForCards`, `cardsOwned` (`CardsOwnedBy`, which counts a Karma card set aside for a bid), `homeworlds` (`[{world, location, occupier, highThreshold, lowThreshold}]`), `skilledLeaders` in front of the shield (`[{id, skill}]`), `loyalLeader` (Ecaz), `latestRevealedNoField` (Richese). Whether a player is highlighted on the ring (`PlayerShouldBeHighlighted`) is `status.waitingFor`.

`PrivatePlayer`: `faction`, `resources`, `hand` (`[{id, playableWith}]`; `playableWith` lists the applicable event types that play the card from the faction panel, `CardPlays.PlaysCard`; an event type listed for some card is left out of the action column, `CardPlays.PlayedFromHand`), `cardSetAside`, `leaders` (`[{id, alive, skilled, skill}]`), `traitors` (empty while the player is still picking, `PlayerPanel.StillPickingTraitor`) and `faceDancers` (`[{hero, revealed, skill}]`; `skill` is `Game.Skill`, for the popover), `predictedFaction`/`predictedTurn` (Bene Gesserit, after predicting), `nexus`, `nextStormMoves` (under storm prescience, after the dial), `messiah` (Atreides with `AtreidesMessiah`: `{forcesKilledInBattle, available}`), `unplacedTerror` (Moritani), `cardJustWon` (when the viewer knows it). Ecaz's unplaced ambassadors are `PublicPlayer.ambassadors`; the resource deck's top is `piles.resourceDeckTop`.

`BoardView`: `sectorInStorm`, `shieldWallDestroyed`, `forces` (`[{location, faction, forces, specialForces, threatened}]`, `ForcesOnPlanetExcludingEmptyLocations(true)`, homeworlds included), `spice` (`[{location, amount}]`), `highlightedTerritories`, `strongholds` (owners; the advantage is static), `terror` (one entry per token, the token for Moritani, `null` for others), `ambassadors`, `discoveries` (type for everyone, the token for the faction of its type), `attached` (the Hidden Mobile Stronghold and discovered strongholds on the map, with the location each is attached to), `leaders` (heroes standing in a territory), `monsters`, `numberOfMonsters`, `specialStrongholds`, `atomicsAftermath`, `protectedFromStorm`, `noFieldValue` (Richese; others draw `?`), `stormPrescienceMoves` (`NextStormMoves` under storm prescience, with no phase gate, as `Planet.razor`), `recentMoves` (arrows during ship and move). The sectors where spice blew this turn (`RecentBlows.razor`) follow from `piles.latestSpiceCardA`/`B` and the static data.

`StatusView`: `message` (`GameStatus.GetMessage(Player, IsHost)`; observers get the waiting text), `waitingForMe`, `waitingForHost`, `waitingFor` (factions), `waitingInSequence` (`[{faction, hasTurn}]`), `timedEventTime`, `flash` (`[{toShow, message}]`: `FlashInfo`, `toShow` as a rich text part), `toBid` (`Bid.PlayersToBid` or `BlackMarketBid.PlayersToBid` in the bidding main phase). The prompt's "Continue" is the `EndPhase` action.

The static data the client needs beside the skin (locations with territory, sector and kind; territory flags; leaders' factions; card types; resource cards; homeworlds; each faction's force types; rule groups; the places `Map` names) is `web/public/static/data.json`, written by `rust/view/examples/static_data.rs` from `src/static_data.rs` and typed by the generated `StaticData`. `enums.ts` also has `PhaseValue` and `MainPhaseValue`, for the C# client's phase comparisons (`CurrentPhase >= Phase.TradingFactions`).

### Rich text

`RichText` is an array of parts, the server's rendering of a C# `Message` or `Expression` (`rust/view/src/expr.rs`, which follows the cases of `ExpressionComponent`'s `switch` in order). Nested messages and lists are flattened, with the ", " separators `ExpressionComponent.DetermineElements` adds. Collections the game still holds (hazards 4.5) are read from the game when the view is built, as the C# client enumerated them when it rendered. Every part that names something carries `label`, `Skin.Describe` of it rendered on the server with the game's skin (`rust/skin`). The client draws parts with `web/src/components/RichText.tsx`, with `size` and `logStyle` as the C# component's `Size` and `LogStyle`.

| `t` | Members | Drawn as (`ExpressionComponent.razor`) |
|---|---|---|
| `text` | `v` | text. Also a `long`, an exception and a `Player`, which C# draws through `Describe` |
| `none` | | bold "none" |
| `num` | `v` | a number token (`SimpleNumberComponent`). An `int`, or a `float`/`double` already rounded like the client's `Support.Round` |
| `bool` | `v`, `label` | text ("Yes", "No") |
| `payment` | `amount`, `by` (faction or null) | the spice icon, then " by " and a faction chip |
| `faction` | `v`, `label` | a faction chip |
| `enum` | `ty` (the C# enum type), `v`, `label` | by type: a name with a popover for `LeaderSkill`, `TechToken`, `DiscoveryToken`, `TerrorType`; the worm icon for `Concept.Monster`; a token image for `FactionForce`, `FactionSpecialForce` (tooltip) and `Ambassador` (popover); bold with spice icons for `AuctionType`, `BrownEconomicsStatus`, `TreacheryCardType`, `StrongholdAdvantage`, `FactionAdvantage`, `Ruleset`, `Rule`; plain text otherwise |
| `location`, `territory` | `id`, `label` | a place chip (`Support.PlaceChipHtml`); `label` is its title |
| `leader` | `id`, `label`, `skill` (`Game.Skill`, or null) | in the log, the leader disc; elsewhere the name; with a popover that shows the skill card when there is a skill |
| `messiah` | `label` | the name, with the Messiah's image as popover |
| `treacheryCard` | `id`, `label` | in the log, a card chip with its art; elsewhere the name; with the card popover |
| `resourceCard` | `id` (`SkinId`), `label` | the name, with the spice card popover |

Popovers are the HTML that `Skin.GetPopup` and the client's `Support.GetPopup(ResourceCard)` build, ported to `web/src/popups.ts` and shown by the C# client's Bootstrap bundle (`web/src/popover.ts`).

A log line (`LogLine`) is `{text, private, heading, record, route, plans, fact}`: `private` when `Message.Target` is the viewer (drawn in italics), `heading` the `Message.Heading` concept, and the other four the data the report's designs draw from: `Message.Log` (`record`, one of the records in `Shared/Model/Log/`, tagged by `type` with its C# name), `Message.Route`, `Message.Plans` and `Message.Fact`. Each is JSON in the shape of the C# record: property names in camelCase, enum values by name, and game objects (locations, territories, heroes, cards) as parts, so the client can draw them as rich text. C# `object` fields (`BeneGesseritReaction.Place`, `BattleFact.Cause`) are `RichText`. The types are in `rust/view/src/log.rs`.

How this is checked: `rust/view/csharp-ref` renders every log message of the scene fixtures, plus a sample of bot-game messages, with the C# engine and `DefaultSkin`, as `ExpressionComponent` draws it and with the records serialised by reflection. `rust/view/tests/log_lines.rs` compares the view's `LogLine`s with that, and `npm run check:rich-text` in `web/` compares the text the React component shows with the text the C# component shows.

### Actions

`ActionView` is `{eventType, options, validation}`:

- `eventType`: the C# event class, also the `$type` to submit.
- `options`: the form's choices, `ActionOptions`, tagged by `form`. `null` while the event's option builder isn't written; the client then shows nothing for it.
- `validation`: `Validate()` of the event the form would send with its default inputs, as `RichText`, or null. The confirm button shows it and is disabled while it is set, as `ButtonRowComponent` does. A `Validate()` that throws gives "Validation failed", as in `GameEventComponent.Validation`.

The default inputs are the C# component's field initializers after its widgets have adjusted them (a number picker clamps its initial value into its range).

`ActionOptions` has one variant per form. The ones that exist:

| `form` | Members | C# component |
|---|---|---|
| `Simple` | `title` (RichText), `confirmLabel`, `alert` | `SimpleActionComponent` with the arguments `ActionPanel.razor` gives it (`AllianceBroken`, `AlliancePassed`, ...). `YesNoActionComponent` will get its own variant with a second button |
| `AllianceOffered` | `targets` (factions) | `AllianceOfferedComponent`: `AllianceOffered.ValidTargets(Game, Player)` |
| `StormDialled` | `min`, `max` | `StormDialledComponent`: `StormDialled.ValidMinAmount(Game)`, `ValidMaxAmount(Game)` |
| `Revival`, `RaiseDeadPlayed`, `KarmaFreeRevival`, `KarmaRevivalPrevention`, `ChoamFreeRevivalPrevention`, `RequestTleilaxuRevival`, `AcceptOrCancelTleilaxuRevival`, `SetIncreasedRevivalLimits` | the helpers each component calls (`schema/revival.rs` documents every member), plus `draft`: the event with the inputs as the component shows them after its widgets have adjusted them (clamped amounts, hidden pickers at 0, a required choice falling back to the first value, the map pick dropped while no forces go to the planet). The validation is that draft's | the eight revival components (package 7d) |
| `Bid` | `maxAmount` (depends on the draft's Karma card), `maxAllyAmount`, `maxEmperorAmount`, `karmaCards`, `mayUseEmperorSecretAlly`, `defaultAmount`, `defaultAllyAmount` | `NormalBiddingPanel`: `Bid.ValidMaxAmount(Player, KarmaCard != null)`, `ValidMaxAllyAmount`, `ValidKarmaCards` when `CanKarma`, `MayUseEmperorSecretAlly`, `Game.SpiceForBidsEmperorCanPay`; the defaults are `BiddingPanel.OnParametersSetAsync` after the pickers clamp them |
| `BlackMarketBid` | `maxAmount`, `maxAllyAmount`, `maxEmperorAmount`, `defaultAmount`, `defaultAllyAmount` | `BlackMarketBiddingPanel`: `BlackMarketBid.ValidMaxAmount(Player)`, `ValidMaxAllyAmount(Game, Player)` |
| Battle forms (package 7b) | one variant per event class of the inventory's "Action panel: battle", payloads in `rust/view/src/schema/battle.rs`; each carries `draft`, the event with the form's inputs as the component shows them | `AuditCancelled` … `Voice`. `Battle` adds `mixes` (every affordable `[fullSpecial, halfSpecial, fullNormal, halfNormal, Battle.Dial, Battle.Cost]`, the C# component's `Mixes`), the weapons and defenses for the draft's leader and cards, and the leader's strength for the draft |

Every other event type gets a variant named after its component's event class, holding the results of the helpers that component calls. The "Action forms" table of [client-inventory.md](client-inventory.md#action-forms) lists those helpers per component; it is the checklist.

#### Dependent inputs

A helper that takes more than the game and the acting player depends on another input of the same form. The inventory marks these **dep**. Examples: `Battle.ValidWeapons(g, p, selectedDefense, selectedHero, territory)`, `Battle.MaxForces(g, p, specialForces)`, `Shipment.ValidMaxShipmentSiteToSiteForces(p, special, source)`, `ChoamRemoveForce.ValidFactions(g, p, location)`, `ClairVoyancePlayed.ValidQuestions(g, target)`. For these the client sends the draft with `QueryDraft` and gets a `DraftResult`, `{eventType, options, validation}`, with options computed from the draft's inputs: the valid weapons for the chosen defense and leader, the maximum forces from the chosen source. The client sends a query after each input change, drops answers older than its latest query, and keeps the previous options on screen until the answer arrives.

Map clicks are inputs too. Valid locations and territories are part of the options (for example the destinations of a move for the forces selected so far), and the map highlights them.

Choices that depend only on the game and the player are in the view's options already, so a form with no dependent inputs needs no query except for validation. The client may skip validation queries for such forms and rely on the server's answer to `SubmitEvent`, but then its confirm button would no longer show the validation message; the C# client always showed it, so the client queries.

### Visibility

The view holds what the C# client shows that user, and nothing the game rules hide from them. Every rule below is the C# client's own gate, found by reading the component (audit in [client-plan.md](client-plan.md#visibility-audit)).

| Data | Who sees it | C# gate |
|---|---|---|
| Hand, traitors, face dancers (revealed ones marked), leaders' state | the owner | `PlayerPanel` renders only `Client.Player` |
| Card count | everyone | `TreacheryCards.Count` |
| Spice behind the shield (`Resources`) | the owner; others only through notekeeping (`notes`) and the bot ally's `AllyPermission` form | `PlayerPanel`, `NoteComponent`, `AllyPermissionComponent` |
| `ResourcesAfterBidding`, bribes, extortion, banked spice | everyone | `Players.razor:195-234`, no gate |
| Bene Gesserit prediction | Bene Gesserit, after predicting | `PlayerPanel:101` |
| Nexus card held | the owner | `PlayerPanel:186` |
| Card identities in other hands | with notekeeping, the cards in the viewer's own `Player.KnownCards` (not the ally's) | `NoteComponent:196` |
| Treachery discard pile | the top card to everyone; with notekeeping every card, face up when in `Game.KnownCards(Player)` (which includes the ally's) | `CardPiles:97-155` |
| Resource deck top | under resource prescience, during ship and move | `HasResourceDeckPrescience(Player)` |
| Card on auction | under bidding prescience, or for everyone in Richese's auctions | `BiddingPanel:12` |
| Silent-auction bids | "ready" only | `Bids.razor` |
| Richese's cache contents | Richese | `CardPiles:171` |
| Battle plans | the submitter until both are in; both once revealed; the full opposing plan to Atreides and Atreides's ally under Atreides Karama | `BattlePlans.razor:13-17` |
| Face-down dead leaders | the owner and Tleilaxu | `Tanks.razor`, `MayIdentify` |
| Placed terror tokens | identity to Moritani, a count to others | `ForcesOnPlanet:27`, `MapComponent:225` |
| Discovery tokens | identity to Fremen (Fremen type) or Guild (Guild type), type to others | `ForcesOnPlanet:101` |
| No-Field value | Richese; `?` to others | `MapComponent:219` |
| Storm prescience (`nextStormMoves`) | `HasStormPrescience(Player)` | `PlayerPanel:96` |
| The card just won (`me.cardJustWon`) | the viewer, when it is in `Game.KnownCards(Player)` | `CardPickerComponent:41` (Atreides's tracker) |
| Victory points, other players' spice and cards (`notes`) | with notekeeping; cards by id only when in the viewer's own `Player.KnownCards` | `NoteComponent` |
| Log lines | `Target == None` or the viewer's faction | `ReportComponent:190` |
| Deals | offers to the viewer, the viewer's own offers, accepted deals to everyone | `DealComponent` |
| Undo list | hosts | `UndoComponent` |
| Everything at game end (story) | everyone, after `GameEnded`, unless the rules disable the report | `WinnerPanel:20` |

**Where the C# client shows more than the rules allow**, the view follows the rules, not the C# client, until the coordinator decides otherwise ([client-plan.md](client-plan.md#open-questions)). The Playwright comparison lists these as expected differences:

1. Partial prescience: `BattlePlans.razor:19-25` draws the element prescience revealed of the opponent's plan for every viewer, observers included. The view gives it to the prescience user and their side only.
2. Story at game end: `StoryComponent` asks `YieldsSecrets` of the game replayed up to each point, and a faction hides its secrets after the game, so the C# story shows everything: the cards held and bought, traitors and face dancers, spice, Nexus card and prediction of factions that chose `HideSecrets`. The view asks the finished game and leaves all of those out for such factions (their cards and traitors read "?", as C# draws them when `YieldsSecrets` is false; their spice reads "?" too). For everyone else, the whole story is shown, as in C#.
3. Silent auctions: `Bids.razor:25` colours a bid by `Passed`, which reveals a pass. The view sends "ready" only.
4. Not a leak, but compared the same way: an observer in `Bidding` or `BlackMarketBidding`. The C# bidding panel reads `Player`, which is null for an observer, so it throws and Blazor shows "An unhandled error has occurred" over the bottom of the page. The web client shows no error (see 5 for what it shows instead).

Deliberate changes from the C# client, approved by the user (2026-11-11). The comparison masks them too:

5. Observers (and a host without a seat, for whom C# throws the same way) in `Bidding` and `BlackMarketBidding` get the bidding panel a seated player without bidding prescience sees (`bidding.panel`): the card back, or the card in Richese's auctions where it is public; "Card n of m" in normal auctions; the current bid outside silent auctions. They get no bid inputs (no `Bid` or `BlackMarketBid` action) and no auto-pass. Mask `observer-bidding-error` hides the web panel and the C# error bar.
6. Card flash after every event. `ImageFlashComponent` sets the flash's time only on its first render, so C# flashes only when the game page opens with a flash current, though its `afterEventId` shows the intent to flash after each event. The web client flashes on opening as C# does, and also whenever a view with a higher `eventNumber` than the last one has a non-empty `status.flash`, for 5000 + 1000 per item ms. A click hides it; a newer flash replaces it. A view with the same or a lower `eventNumber` (a reconnect, a seating change, an undo) starts no flash, and neither does leaving an undo preview. Mask `flash-after-event` hides such a flash (marked `data-flash="event"`).

Two more are shown to everyone by C# and kept for now, because they may be public under the rules: Ecaz's unplaced ambassadors on the map ring, and `ResourcesAfterBidding`/bribes/extortion/banked spice.

The view never contains `Game.Seed`, the order of any deck, or another player's `KnownCards`.

### Log and history

`log.reports` lists every report (`{index, turn, about, isBattle, aggressor, defender, territory, battleDecided, battleWinner}`), and `log.latest` holds the lines of the reports of the latest phase, which the report panel shows by default. When the player steps back to an earlier phase, the client calls `RequestReport` for each report of that phase, and caches the answers: a report's lines never change once its phase is over, except the latest one.
