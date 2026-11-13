# Client/server protocol

The Rust server and the React client talk JSON over one WebSocket. The server is authoritative: it holds the only copy of each game, runs the engine, and sends each player and observer their own view of it. Clients never see hidden information and never replay events.

Everything except the [Game view](#game-view) section (connection, envelope, accounts, lobby, seating, chat, admin, notifications) is the server's, described further in [server.md](server.md). Those payloads use PascalCase member names and enums as numbers, the savegame format. The game view is built by `rust/view` ([client.md](client.md)) and uses camelCase and enum names.

## Connection

- Endpoint: `GET /ws`, upgraded to a WebSocket, on the origin that serves the client.
- Each message is one UTF-8 text frame holding one JSON object. A binary frame closes the connection.
- Incoming messages are limited to 4 MiB (4194304 bytes). A larger frame closes the connection.
- Keepalive: the server sends a WebSocket ping every 15 s and closes a connection it has heard nothing from for 30 s. Browsers answer pings on their own. A client may also send `{"type":"ping"}`, which the server ignores.
- A new connection is nobody. `RequestLogin`, `RequestCreateUser`, `RequestSetPassword`, `RequestLinkLogin` and `RequestResumeSession` log it in: they bind a session to the connection, and every later call on it is made as that session's user. No call names its user or carries a token. A call that needs a user, on a connection that isn't logged in or whose session has ended, gets `UserTokenNotFound`. See [Sessions](#sessions).
- Creating, joining, observing or reconnecting to a game makes the *current connection* follow that game (the game's group), as its user. After a new WebSocket is opened, the client logs it in with `RequestResumeSession` and calls `RequestReconnectGame` again.
- When a WebSocket closes, the connection stops following every game. Nothing else changes: the session goes on and the user stays seated.
- A frame that is not a JSON object with a known `type`, or a call without a non-negative integer `id`, closes the connection. A call with an unknown method or arguments that don't decode gets an `error` reply.
- The same origin serves the web client's files (`TREACHERY_WEB_ROOT`, by default `web/dist`) and the shared assets (`TREACHERY_ASSETS_ROOT`, for `/art`, `/css`, `/fonts`, `/js`, `/docs`), and `index.html` for any other path without a file extension. Development only, with `TREACHERY_DEV_SCENE=true`: `POST /dev/scene {"fixture","faction"}` (the web client's `/scene` route, [client.md](client.md#the-web-app)) answers `{"userToken","gameId","userId"}`, where `userToken` is a session of the scene user for `RequestResumeSession`.

## Envelope

Client to server, a call:

```json
{"type":"call","id":7,"method":"RequestJoinGame","args":["<gameId>","<hashedPassword>",-1]}
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

For `InvalidGameEvent`, `ErrorDetails` is the validation message rendered as text. `UserTokenNotFound` means the connection isn't logged in, or its session ended: the client logs in again. `UserNotInGame` means the caller doesn't take part in the game, or only observes it and the call is for players.

### Payload objects

Members in the order the server writes them (`rust/server/src/wire.rs`).

| Object | Members |
|---|---|
| `LoginInfo` | `UserId`, `Token` (the token of the session the call started; null from calls that start none), `PlayerName`, `UserName`, `Email` |
| `LinkLoginInfo` | `LoginInfo`, `GameId` (the game the link opens; null when the user is no longer in it) |
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

"Void" means a result without contents. Every call but `Connect` and the ones that log in is made as the connection's user ([Connection](#connection)). Who may make a call:

- **players**: users seated in the game. Observers and users outside the game get `UserNotInGame`.
- **members**: players and observers. Others get `UserNotInGame`.
- **hosts**: the game's hosts. Others, players and observers included, get `NoHost`.
- **creator**: the user who created the game. Others get `NoCreator`.

A game the server doesn't know gives `GameNotFound` before any of these.

### Server and accounts

| Method | Args | Result |
|---|---|---|
| `Connect` | (none) | `ServerInfo`. Call once after the socket opens |
| `RequestCreateUser` | userName, hashedPassword, email `string?`, playerName | `LoginInfo`. Logs the connection in |
| `RequestLogin` | version (the engine's `Game::LATEST_VERSION`), userName, hashedPassword | `LoginInfo`. Logs the connection in |
| `RequestResumeSession` | token (a `LoginInfo.Token` from earlier) | `LoginInfo` with a null `Token`. Logs the connection in with that session |
| `RequestLinkLogin` | linkToken (from an async-play mail) | `LinkLoginInfo`. Logs the connection in with a new session; the link works once |
| `RequestLogout` | (none) | void. Ends the connection's session, which logs out every connection that uses it |
| `RequestPasswordReset` | usernameOrEmail | void. Mails a reset token |
| `RequestSetPassword` | userName, passwordResetToken, newHashedPassword | `LoginInfo`. Ends the user's other sessions and logs the connection in |
| `RequestUpdateUserInfo` | hashedPassword `string?` (empty keeps the password), playerName, email `string?` | `LoginInfo` with a null `Token`. A new password ends the user's other sessions |
| `RequestSetUserStatus` | status | `ServerStatus` (scope Active) |
| `RequestHeartbeat` | scope (GameListScope: None 1, Active 2, All 3) | `ServerStatus` |

Passwords are hashed by the client: SHA-256 of the password's UTF-8 bytes, as 64 lowercase hex digits (in the browser, `crypto.subtle.digest("SHA-256", new TextEncoder().encode(password))`), the hash the C# client used, so existing accounts keep their passwords. The server hashes that value again with Argon2id and a random salt before storing it ([server.md](server.md#accounts-and-sessions)). `RequestUpdateUserInfo` takes the same client hash, or an empty string to keep the password. The client should send `RequestHeartbeat` every few seconds: the user list only shows users seen in the last minute.

#### Sessions

- A session token is 32 random bytes as 64 hex digits. The server returns it once, in the `LoginInfo` of the call that started the session, and keeps only its SHA-256.
- The client keeps the token out of URLs. The web client keeps it in `sessionStorage`, and in `localStorage` when the user asks to stay logged in, and logs each new WebSocket in with `RequestResumeSession`.
- A session ends 90 days after it started, or 30 days after a connection last logged in with it, whichever comes first. A user keeps at most 20 sessions; a new one ends the least recently used.
- `RequestLogout`, a new password (`RequestSetPassword`, or `RequestUpdateUserInfo` with one, which keeps the caller's own session) and deleting the account end sessions. Every connection logged in with an ended session gets `SessionEnded`, stops following its games, and is logged out.
- The "Jump to game" link of an async-play mail is `<site>/link/<linkToken>`. The link token is not a session: it works once, within 24 hours, for `RequestLinkLogin`, which starts a new session.

### Lobby and game management

| Method | Args | Result |
|---|---|---|
| `RequestCreateGame` | name, hashedPassword, stateData (a savegame, or empty for a new game), skinData (or empty) | `GameInitInfo` |
| `RequestLoadGame` | gameId, state (savegame), skin `string?` | void. Hosts |
| `RequestUpdateSettings` | gameId, settings | void. Hosts. Any rules are taken |
| `RequestCloseGame` | gameId | `ServerStatus`. Creator |
| `RequestGameState` | gameId | `GameInitInfo`, and resends the caller's game view. Members |
| `RequestScheduleGame` | dateTime, ruleset (nullable), numberOfPlayers, maximumTurns, allowedFactionsInPlay [Faction], asyncPlay | `ServerStatus` |
| `RequestCancelGame` | scheduledGameId | `ServerStatus`. The scheduled game's creator |
| `RequestSubscribeGame` | scheduledGameId `string?`, subscription (SubscriptionType) | `ServerStatus` |

### Seating and participation

| Method | Args | Result |
|---|---|---|
| `RequestJoinGame` | gameId, hashedPassword, seat (-1: no particular seat) | `GameInitInfo`. A particular seat must be one the host opened (`SeatNotAvailableNotOpen`); kicked users can't come back (`SeatNotAvailableKicked`) |
| `RequestObserveGame` | gameId, hashedPassword | `GameInitInfo`. Not for kicked users |
| `RequestReconnectGame` | gameId | `GameInitInfo`. Members |
| `RequestLeaveGame` | gameId | `ServerStatus`. Members |
| `RequestSetOrUnsetHost` | gameId, userId | void. Hosts; the user must be a player of the game |
| `RequestOpenOrCloseSeat` | gameId, seat | void. Hosts; the seat must be a player's seat in the game |
| `RequestKick` | gameId, userId | void. Hosts; the user must be another member of the game |
| `RequestAssignSeats` | gameId, assignment [[userId, seat]] | void. Hosts; each user a player of the game, each seat a player's seat or -1, no user or seat twice |
| `RequestSetBotSpeed` | gameId, speed | void. Hosts; 0 to 4. See [Bots](#bots) |
| `RequestNudgeBots` | gameId | void. Players; runs the bots' turn at once. See [Bots](#bots) |

Creating, joining, observing and reconnecting to a game, and `RequestGameState`, send the caller a game view before the reply.

### Playing

| Method | Args | Result |
|---|---|---|
| `SubmitEvent` | gameId, event | void. Players |
| `RequestUndo` | gameId, untilEventNr | void. Hosts |
| `RequestRestoreRecentlyUndone` | gameId | void. Hosts |
| `RequestDismissRecentlyUndone` | gameId | void. Hosts |
| `SetTimer` | gameId, value | void. Hosts |
| `RequestSetSkin` | gameId, skin | void. Hosts |
| `RequestPhaseDone` | gameId | void. Players. The caller has acted or passes in the current pause, and stops holding its countdown |
| `RequestPhaseHold` | gameId | void. Players. Holds the running countdown; does nothing when none is running |
| `RequestPhaseRelease` | gameId | void. Players. Stops holding |
| `RequestPhaseContinue` | gameId, eventNumber | void. Hosts. Ends the current pause, but only while the game has `eventNumber` events, so a click that arrives after the server already continued does nothing |

The event is a `GameEvent` in the savegame format, with `$type` set to the event class. The event is always the caller's own. The server sets `Initiator` to the caller's faction (none before factions are dealt, or for a host without a seat), whatever the client wrote, so the engine accepts it only as an action of that faction, or a host action when the caller is a host. It sets `Seat` and `InitiatorPlayerName` of `FactionSelected` to the caller's. `EstablishPlayers` is for hosts (`NoHost` otherwise), and the server draws its `Seed`, which decides every deck. The server sets `Time`, validates and executes the event, and on success sends every participant a new view. A failed validation, or an event that doesn't decode, is `InvalidGameEvent` with the message in `ErrorDetails`, rendered with the default skin.

Pauses between phases continue on their own. After each event the server works out who the current pause waits for and whether it runs a countdown that any player can hold (`rust/server/src/pause_rules.rs`). Once nobody is left to wait for and the countdown has run out or every holder has released it, the server submits `EndPhase` itself. A player waited for is done once they call `RequestPhaseDone`, or, in the revival step, once they revive. After an undo or a loaded game the current pause waits for the host's `RequestPhaseContinue` or the next event. Games older than version 188 keep host-only control.

A valid `EstablishPlayers` event also seats the users on the players' seats, in the order they joined (`AssignSeats` notification). Seats that no user takes become bots (see [Bots](#bots)).

### Chat

| Method | Args | Result |
|---|---|---|
| `SendChatMessage` | gameId, message (GameChatMessage) | void. Players |
| `SendGlobalChatMessage` | message (GlobalChatMessage) | void |

The server sets `SourceUserId` to the caller. A game message with a negative `TargetUserId` goes to the game's group. One with a user's id goes only to that user's and the sender's connections in the group; the user must be a member of the game (`UserNotInGame` otherwise).

### Admin

The caller's user name must be the configured admin (`TREACHERY_ADMIN`), except for `DeleteUser`, which a user may also call on their own account (others get `NotAdmin`). Others get `InvalidUserNameOrPassword`.

| Method | Args | Result |
|---|---|---|
| `AdminUpdateMaintenance` | maintenanceDate | string |
| `AdminPersistState` | (none) | string (a summary). Saves every game with unsaved changes now |
| `AdminRestoreState` | (none) | string (a summary). Loads the saved games that aren't running; running games stay as they are |
| `AdminCloseGame` | gameId | string. Removes everyone and deletes the game |
| `AdminDownloadGame` | gameId | string, the savegame |
| `AdminCancelGame` | scheduledGameId | string |
| `DeleteUser` | userId | string. Deletes the account, ends its sessions (`SessionEnded`), and closes the games it created |
| `GetAdminInfo` | (none) | `AdminInfo` |

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
| `BotSpeed` | speed | game group |
| `SetTimer` | value | game group |
| `PhaseControl` | PhaseControlInfo | game group, when what the current pause waits for changes |
| `SetSkin` | skin | game group |
| `ChatMessage` | GameChatMessage | game group, or the target's and the sender's connections in it |
| `GlobalChatMessage` | GlobalChatMessage | every connection |
| `SessionEnded` | (none) | the connections logged in with a session that just ended. They are logged out and follow no game |

The game group is every connection that created, joined, observed or reconnected to the game, until it closes or its user leaves or is kicked, the caller's included. Participation changes (`JoinGame`, `RemoveUser`, `AssignSeats`, ...) can change what each member may see, so the game-view section decides whether they are followed by a `GameView`.

## Bots

The server plays every seat without a user (`rust/server/src/server/bots.rs`; the bots are the `treachery-bots` crate).

- Which seats are bots: since engine version 170, `EstablishPlayers` fills the seats that no user takes with bots (`Settings.NumberOfPlayers` players in all). A seat also becomes a bot when its user leaves or is kicked, when the host reassigns the seats without it, or when a loaded savegame has it empty. A user who takes the seat (an open seat, `RequestJoinGame`) replaces the bot. `GameView.players[].isBot` and `GameInfo.NrOfBots` show them.
- When bots act: after every event, and after a loaded game, undo, restore or dismiss, a seat assignment, a leave or kick, a new observer, the first seated join and a bot speed change, the server schedules a bot turn, replacing one scheduled earlier, after a delay that depends on the last event and the phase, divided by the bot speed (800 ms after an `AllyPermission`, `DealOffered` or `SetShipmentPermission`, 1600 ms after a `Bid`, 2400 ms in revival and charity, 9600 ms in shipment and battle, 4800 ms otherwise). A turn does nothing while the game awaits players or the speed is 0. Otherwise each bot, in a random order, is asked for its most urgent action, then the next level, and the first action found is executed as an event of that bot. The bots end phases themselves (`EndPhase` and other host actions) only when the game has no host and at least one observer. Otherwise the phase control continues the pauses, and a turn in which no bot acts tells it the bots have passed.
- `BotsSpeed` (`Participation.BotsSpeed`, default 1) is the divisor. 0 pauses the bots. A host may set 0 to 4; the client offers 0 (pause), 1, 2 and 4. The game's group gets the `BotSpeed` notification.
- `RequestNudgeBots` runs a bot turn at once (unless the bots are paused), for a game whose bots stopped (a refused bot event schedules no next turn). The game's players may call it.
- A bot is created when its faction is first asked, and keeps its state for the rest of the game, across undo. A server restart creates new bots.

## Game view

Built by `rust/view` ([client.md](client.md)). Wire types: `rust/view/src/schema.rs` and `schema/`, which document every member, generated to TypeScript in `web/src/generated/`.

The server sends each participant a `GameView` notification: what that user may see of the game, plus the actions they can take now and the valid choices for each. The client renders only views. While a player fills in an action, the client asks the server about the inputs chosen so far (a draft query), and submits the finished event with `SubmitEvent`.

### Encoding

- Member names in camelCase.
- Enum values by member name (`"Atreides"`, `"ShipmentAndMove"`), the keys the skin JSON uses. The TypeScript unions are in `web/src/generated/enums.ts`, with `PhaseValue`, `MainPhaseValue` and a few more holding the numbers, for ordered comparisons and for events.
- Static game objects by id: locations, territories, leaders, treachery cards, and resource cards (by skin id). Names, images and map positions come from the skin and the static data (`web/public/static/data.json`), never from the view.
- Text the game writes (log lines, prompts, validation messages, form headings) arrives rendered to [rich text](#rich-text).

Events stay in the savegame format of [Encoding](#encoding) (PascalCase, enums as numbers), both in draft queries and in `SubmitEvent`, because the engine reads and validates that format.

### When views are sent

| After | Who gets a new view |
|---|---|
| an event (`SubmitEvent`), undo, restore, dismiss, `LoadGame` | every member of the game group |
| `JoinGame`, `RemoveUser`, `AssignSeats`, `SetOrUnsetHost`, `OpenOrCloseSeats` | every member: seats decide `me`, host status decides actions, seat occupancy decides `isBot` |
| a successful `RequestCreateGame`, `RequestObserveGame`, `RequestReconnectGame`, `RequestJoinGame`, `RequestGameState` | the caller (and, for a join, every member, through `JoinGame`) |
| `SetSkin` | every member, because labels are rendered with the game's skin |
| `SetTimer` | every member: the timer is `timing.timer` |
| `UpdateSettings` | nobody: settings only matter before the game starts |

Each view is complete; there are no diffs. A view of the alliance phase of a 6-player game is about 22 KB of JSON, most of it the log.

```json
{"type":"notify","name":"GameView","args":[{"eventNumber":612,"version":188,"turn":6, "...": "..."}]}
```

### Calls

| Method | Args | Result |
|---|---|---|
| `QueryDraft` | gameId, eventNumber, draft (a `GameEvent` in savegame format) | players: `DraftResult`, or `InvalidGameEvent` when the draft is not one of the caller's actions in their latest view (`ErrorDetails` says why) |
| `RequestReport` | gameId, index (an entry of `log.reports`) | members: `ReportLines` of that report, filtered for the caller as in the view; `InvalidGameEvent` for an index that has no report |
| `RequestUndoList` | gameId | hosts: `[{eventNumber, text: RichText, listed, turn, phase}]`, the short message of every event, whether the undo panel lists it (ally permissions, battle revisions and automation settings are left out), and the turn and main phase it happened in. `eventNumber` is the event's index in the history, the `untilEventNr` of `RequestUndo` |
| `RequestUndoPreview` | gameId, gap | hosts: the caller's `GameView` at gap `gap`, the moment just before event `gap` (the game after `RequestUndo(gap)`), with no actions and no `playableWith`. The gap is clamped to 1..number of events. Sends no notification; the live game and its views are untouched |
| `RequestSaveGame` | gameId | hosts and the admin (`NoHost` otherwise): the server's game as a savegame string. Always the live game, also while the host previews an undo |

These use the same `Result` envelope as the other calls, with `Contents` in the view's camelCase encoding. A connection that isn't logged in, or a game the server doesn't know, gives the same errors as the other calls (`UserTokenNotFound`, `GameNotFound`).

The server sets a draft's `Initiator` to the caller's faction, so a client can't ask about another faction. A draft made from an older view is answered against the current game; the client discards answers for views it has replaced.

### GameView

| Field | Type | What |
|---|---|---|
| `eventNumber` | int | the number of events so far; draft queries and submissions name the view they were made from with it |
| `version`, `turn`, `maximumTurns`, `maximumPlayers` | int | the game version, current and last turn, and the seats around the map |
| `mainPhase`, `phase` | enum names | |
| `viewer` | `{faction, isHost, isObserver}` | the viewer, resolved from the game's participation |
| `players` | `PublicPlayer[]`, seat order | what everyone sees of each player |
| `me` | `PrivatePlayer` or null | the viewer's own faction panel |
| `board` | `BoardView` | the map |
| `status` | `StatusView` | the prompt: who is awaited and why |
| `actions` | `ActionView[]` | the event types the viewer may submit now, in the engine's order |
| `log` | `LogView` | report headers and the latest phase's lines |
| `ruleset`, `rules`, `rulesForBots` | enum names | a rule applies when it is in either list |
| `economicsStatus` | enum name | CHOAM's charity status |
| `piles` | | treachery deck count and discard pile (the top card; with notekeeping every card, `null` where unknown), resource deck count and, under resource prescience during ship and move, its top card, the latest spice cards A and B and, with notekeeping, their discard piles, Sand Trout, Richese's cache (a count; the cards for Richese) |
| `tanks` | | per faction: forces and special forces killed, dead leaders (face-down ones as `null` unless the viewer may identify them), the Messiah |
| `bidding` | | in the bidding main phase: auction type, the bids as the map shows them (`pass`, `bid` with amount, or `ready` in silent auctions), `auctionWon`, and in the bidding phases the lot (`panel`: black market or not, the card on auction when visible, "card n of m" in normal auctions, silent, the current bid outside silent auctions). Observers get the lot of a player without bidding prescience and no bid action |
| `battle` | | while there is a current battle: territory, aggressor and defender, and each plan as far as the viewer may see it: whole for its owner, for everyone once both are in, and for Atreides and their ally under Atreides Karama; the aspect prescience revealed, for the prescience user and their ally |
| `deals` | | seated viewers only: offers the viewer may accept (with their validation and the accept and decline events in savegame JSON), the viewer's own offers (each recipient's answer and the withdraw event), and every deal in force. An offer's `id` is its event number |
| `botsSpeed` | int | 0 paused, 1, 2 or 4 |
| `notes` | | with assisted notekeeping: per other player, spice texts, reserves, cards owned (a card, or `null` for one the viewer doesn't know), revealed traitors and face dancers, victory points, Messiah state |
| `tracker` | | Atreides from traitor selection on: cards in play, traitors in play, per-player card, traitor and face dancer counts, resource discard piles, remaining resource cards. The client keys its local notes by `timing.started` |
| `mentat` | | the same for everyone: the treachery cards in play (with Richese's when Richese plays) and the resource cards in play |
| `timing` | | game start, last activity, duration, time spent per player in bidding, ship and move, and battle (seconds), and the host's bidding timer |
| `ending` | | at game end: `winners` (`[{faction, name}]`) and `story`, null when the rules disable the end-of-game report. The story is a list of blocks (`setup`, `prediction`, `purchase`, `turn`, `situation` with a table of players, `battleFor`, `milestone`, `battle`, `strongholds`, `winner`, `ended`), built by replaying the history in a fresh game (`ending.rs`). The same for every viewer |
| `factionSheets` | | the same for everyone: per player, the faction's rules sheet from the skin (`atStart` as parts, with places as location or territory ids for place chips; `freeRevival`, `theme`, `advantages`, `advancedAdvantages`) and its HTML |
| `ruleLabels` | | the same for everyone: the ruleset and the rule groups in play, described by the skin, for the turn popover |
| `recentMilestones` | `Milestone[]` | the same for everyone: the milestones of the latest event, whose sounds the client plays |

`PublicPlayer`: `faction`, `seat`, `name`, `isBot`, `ally`, `forcesInReserve`, `specialForcesInReserve`, `forcesKilled`, `specialForcesKilled`, `cardCount`, `bribes`, `resourcesAfterBidding`, `extortion`, `bankedResources`, `techTokens`, `ambassadors` (Ecaz's unplaced ones), `hasSpecialForces`, `hasRoomForCards`, `cardsOwned` (the hand plus a Karma card set aside for a bid), `homeworlds` (`[{world, location, occupier, highThreshold, lowThreshold}]`), `skilledLeaders` in front of the shield (`[{id, skill}]`), `loyalLeader` (Ecaz), `latestRevealedNoField` (Richese). Whether a player is highlighted on the ring is `status.waitingFor`.

`PrivatePlayer`: `faction`, `resources`, `hand` (`[{id, playableWith}]`: the applicable event types that play the card from the faction panel; those event types are left out of the action column), `cardSetAside`, `leaders` (`[{id, alive, skilled, skill}]`), `traitors` (empty while the player is still picking), `faceDancers` (`[{hero, revealed, skill}]`), `predictedFaction`/`predictedTurn` (Bene Gesserit, after predicting), `nexus`, `nextStormMoves` (under storm prescience, after the dial), `messiah` (Atreides: `{forcesKilledInBattle, available}`), `unplacedTerror` (Moritani), `cardJustWon` (when the viewer knows it).

`BoardView`: `sectorInStorm`, `shieldWallDestroyed`, `forces` (`[{location, faction, forces, specialForces, threatened}]`, homeworlds included), `spice` (`[{location, amount}]`), `highlightedTerritories`, `strongholds` (owners), `terror` (one entry per token: the token for Moritani, `null` for others), `ambassadors`, `discoveries` (the type for everyone, the token for the faction of its type), `attached` (the Hidden Mobile Stronghold and discovered strongholds, with the location each is attached to), `leaders` (heroes standing in a territory), `monsters`, `numberOfMonsters`, `specialStrongholds`, `atomicsAftermath`, `protectedFromStorm`, `noFieldValue` (Richese; others draw `?`), `stormPrescienceMoves`, `recentMoves` (arrows during ship and move). The sectors where spice blew this turn follow from `piles.latestSpiceCardA`/`B` and the static data.

`StatusView`: `message` (observers get the waiting text), `waitingForMe`, `waitingForHost`, `waitingFor` (factions), `waitingInSequence` (`[{faction, hasTurn}]`), `timedEventTime`, `flash` (`[{toShow, message, bordered}]`, the cards and leaders to flash; the client flashes them when the game page opens and whenever a view with a higher `eventNumber` brings some), `toBid` (the players still to bid). The prompt's "Continue" is the `EndPhase` action.

### Rich text

`RichText` is an array of parts, the server's rendering of a message (`rust/view/src/expr.rs`). Nested messages and lists are flattened, with ", " separators. Collections a message holds are read from the game when the view is built. Every part that names something carries `label`, the skin's description of it, rendered with the game's skin. The client draws parts with `web/src/components/RichText.tsx`.

| `t` | Members | Drawn as |
|---|---|---|
| `text` | `v` | text |
| `none` | | bold "none" |
| `num` | `v` | a number token. Fractions are already rounded to three decimals |
| `bool` | `v`, `label` | text ("Yes", "No") |
| `payment` | `amount`, `by` (faction or null) | the spice icon, then " by " and a faction chip |
| `faction` | `v`, `label` | a faction chip |
| `enum` | `ty` (the enum type), `v`, `label` | by type: a name with a popover for `LeaderSkill`, `TechToken`, `DiscoveryToken`, `TerrorType`; the worm icon for `Concept.Monster`; a token image for `FactionForce`, `FactionSpecialForce` (tooltip) and `Ambassador` (popover); bold with spice icons for `AuctionType`, `BrownEconomicsStatus`, `TreacheryCardType`, `StrongholdAdvantage`, `FactionAdvantage`, `Ruleset`, `Rule`; plain text otherwise |
| `location`, `territory` | `id`, `label` | a place chip; `label` is its title |
| `leader` | `id`, `label`, `skill` (or null) | in the log, the leader disc; elsewhere the name; with a popover that shows the skill card when there is a skill |
| `messiah` | `label` | the name, with the Messiah's image as popover |
| `treacheryCard` | `id`, `label` | in the log, a card chip with its art; elsewhere the name; with the card popover |
| `resourceCard` | `id` (skin id), `label` | the name, with the spice card popover |

Popovers are HTML built by `web/src/popups.ts` from the skin JSON.

A log line (`LogLine`) is `{text, private, heading, record, route, plans, fact}`: `private` when the message was sent to the viewer alone (drawn in italics), `heading` the concept whose icon heads a section of the log, and the other four the data the phase logs draw from: the message's log record (tagged by `type`), the route of a shipment or move, both revealed battle plans, and one fact of a battle's outcome. Game objects in them are parts, so the client can draw them as rich text. The types are in `rust/view/src/log.rs`. `rust/view/tests/log_lines.rs` keeps snapshots of the lines, and `npm run check:rich-text` checks the text React shows for each.

### Actions

`ActionView` is `{eventType, options, validation}`:

- `eventType`: the event's type name, also the `$type` to submit.
- `options`: the form's choices, `ActionOptions`, tagged by `form`.
- `validation`: the validation message of the event the form would send with its default inputs, as `RichText`, or null. The confirm button shows it and is disabled while it is set. A validation that fails inside the engine gives "Validation failed".

The default inputs are the form's starting values after its widgets have adjusted them (a number picker clamps its value into its range, a required choice falls back to the first value). Forms whose payload holds a `draft` send that draft back adjusted; the validation is the adjusted draft's.

`ActionOptions` has one variant per form, named after the event type, except `Simple` (a confirm button with `title`, `confirmLabel`, `alert` and `allowAutoConfirm`, used by about twenty event types and by `EndPhase`, the prompt's "Continue") and `YesNo` (two buttons). The payloads are in `schema.rs` and `schema/{battle,ship_move,revival,other,setup}.rs`, which say what each member holds.

#### Dependent inputs

Some choices depend on another input of the same form: the weapons for the chosen leader and defense, the maximum forces from the chosen source, the questions for the chosen target. For these the client sends the draft with `QueryDraft` and gets a `DraftResult`, `{eventType, options, validation}`, with options computed from the draft's inputs. The client sends a query after each input change, drops answers older than its latest query, and keeps the previous options on screen until the answer arrives. A form with no dependent inputs still queries, for the validation next to its confirm button.

Map clicks are inputs too. Valid locations and territories are part of the options, and the map highlights them.

### Visibility

The view holds what the game rules let that user see, and nothing more.

| Data | Who sees it |
|---|---|
| Hand, traitors, face dancers (revealed ones marked), leaders' state | the owner |
| Card count | everyone |
| Spice behind the shield | the owner; others only through notekeeping (`notes`) and the bot ally's `AllyPermission` form |
| Spice after bidding, bribes, extortion, banked spice | everyone |
| Bene Gesserit prediction | Bene Gesserit, after predicting |
| Nexus card held | the owner |
| Card identities in other hands | with notekeeping, the cards the viewer has seen (not those only their ally has seen) |
| Treachery discard pile | the top card to everyone; with notekeeping every card, face up when the viewer or their ally knows it |
| Resource deck top | under resource prescience, during ship and move |
| Card on auction | under bidding prescience, or for everyone in Richese's auctions |
| Silent-auction bids | "ready" only, not whether a bid passed |
| Richese's cache contents | Richese |
| Battle plans | the submitter until both are in; both once revealed; the full opposing plan to Atreides and their ally under Atreides Karama; the aspect prescience revealed to the prescience user and their ally |
| Face-down dead leaders | the owner and Tleilaxu |
| Placed terror tokens | identity to Moritani, a count to others |
| Discovery tokens | identity to Fremen (Fremen type) or Guild (Guild type), type to others |
| No-Field value | Richese; `?` to others |
| Storm prescience (`nextStormMoves`) | the player with storm prescience |
| The card just won (`me.cardJustWon`) | the viewer, when they know it |
| Log lines | public lines, and those sent to the viewer's faction |
| Deals | offers to the viewer, the viewer's own offers, deals in force to everyone |
| Undo list | hosts |
| The story | everyone, after the game ends, unless the rules disable the report |

Ecaz's unplaced ambassadors and spice after bidding, bribes, extortion and banked spice are shown to everyone; they may be public under the rules.

The view never contains the game's seed, the order of any deck, or another player's known cards.

### Log and history

`log.reports` lists every report (`{index, turn, about, isBattle, aggressor, defender, territory, battleDecided, battleWinner}`), and `log.latest` holds the lines of the reports of the latest phase, which the report panel shows by default. When the player steps back to an earlier phase, the client calls `RequestReport` for each report of that phase and caches the answers: a report's lines never change once its phase is over, except the latest one.
