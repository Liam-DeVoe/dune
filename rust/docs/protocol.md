# Client/server protocol

The Rust server and the React client talk JSON over one WebSocket. The server is authoritative: it holds the only copy of each game, runs the engine, and sends each player and observer their own view of it. Clients never see hidden information and never replay events.

Every message is a type in the `treachery-protocol` crate (`rust/protocol`): `Call` for what clients send, `ServerMessage` and `Notification` for what the server sends, and the lobby objects in `src/lobby.rs`. Its doc comments say what each call and member is for, and `rust/protocol/tests/ts_bindings.rs` generates their TypeScript types into `web/src/protocol/`. This page describes the format and the rules around it. The [game view](#game-view) is built by `rust/view` ([client.md](client.md)). The server's side is described in [server.md](server.md).

## Connection

- Endpoint: `GET /ws`, upgraded to a WebSocket, on the origin that serves the client.
- Each message is one UTF-8 text frame holding one JSON object. A binary frame closes the connection.
- Incoming messages are limited to 4 MiB (4194304 bytes). A larger frame closes the connection.
- Keepalive: the server sends a WebSocket ping every 15 s and closes a connection it has heard nothing from for 30 s. Browsers answer pings on their own.
- A new connection is nobody. `authenticate` (with the token of a session started earlier), `login`, `createUser`, `setPassword` and `linkLogin` bind a session to it, and every later call on it is made as that session's user. No other call names its user or carries a token. A call that needs a user, on a connection that isn't authenticated or whose session has ended, gets `notLoggedIn`. See [Sessions](#sessions).
- Creating, joining, observing or reconnecting to a game makes the *current connection* follow that game (the game's group), as its user. After a new WebSocket is opened, the client authenticates it and calls `reconnectGame` again.
- When a WebSocket closes, the connection stops following every game. Nothing else changes: the session goes on and the user stays seated.
- The same origin serves the web client's files (`TREACHERY_WEB_ROOT`, by default `web/dist`) and the shared assets (`TREACHERY_ASSETS_ROOT`, for `/art`, `/css`, `/fonts`, `/js`, `/docs`), and `index.html` for any other path without a file extension. Development only, with `TREACHERY_DEV_SCENE=true`: `POST /dev/scene {"fixture","faction"}` (the web client's `/scene` route, [client.md](client.md#the-web-app)) answers `{"userToken","gameId","userId"}`, where `userToken` is a session of the scene user for `authenticate`.

## Envelope

A call, from the client:

```json
{"id":7,"call":"joinGame","args":{"game":"615e1ef8-ed4b-4d82-98e4-5af7d88a088b","passwordHash":null,"seat":3}}
```

- `id` is a non-negative integer the client chooses. The server echoes it in the reply. Use a fresh id per call.
- `call` is the call's name, matched exactly.
- `args` is an object of the call's arguments. Calls without arguments (`serverInfo`, `logout`, `adminInfo`, ...) leave it out. An optional argument (`string | null` in TypeScript) may be null or missing. Members the call doesn't have are ignored.

The reply, from the server:

```json
{"id":7,"ok":{"gameId":"615e1ef8-...","gameName":"Alice's Game","participation":{...},"phaseControl":{...}}}
{"id":7,"err":{"code":"seatNotAvailableNotOpen","message":"Seat is not available"}}
```

- `ok` is the call's reply, of the type `CallReplies` gives for it (`replies` in Rust). A call that returns nothing has `"ok":null`.
- `err` means the call failed. `code` is an `ErrorCode` for the client to act on; `message` is a text to show the user. See [Errors](#errors).

A notification, from the server:

```json
{"notify":"joinGame","data":{"user":12,"name":"Alice","seat":-1}}
```

`data` is left out for notifications without data (`sessionEnded`).

A frame that isn't a JSON object with a non-negative integer `id` closes the connection.

Ordering:

- The server handles calls one at a time, in the order they arrive (see [server.md](server.md#threads)).
- Notifications caused by a call reach the caller before that call's reply.
- Notifications can also arrive without a call of yours: other users' actions, timers.

## Encoding

- Member names are camelCase.
- The engine's enums are sent by member name (`"Atreides"`, `"AwaitingPlayers"`), the names the game view and the skin use. In TypeScript they are the unions of `web/src/generated/enums.ts`. The lobby's own enums (`UserStatus`, `SubscriptionType`, `GameListScope`) are by name too.
- Times are RFC 3339 strings. The server writes UTC with microseconds, `"2026-09-18T04:14:35.622405Z"`, and reads any offset; a time without one is UTC. The time 0001-01-01 means none (`serverInfo.scheduledMaintenance`).
- Maps keyed by user id or seat are arrays of `[key, value]` pairs in the server's order: `"seatedPlayers":[[12,3],[5,-1]]`. An object would lose that order, because `JSON.parse` sorts integer keys. A seat of -1 is a user without a seat yet.
- A seat is a number from 0 to 255. -1 is allowed only where a user may have no seat (`seatedPlayers`, `assignSeats`, the `joinGame` notification); a call with any other number that is no seat is a `badCall`. User ids are integers, the ids of the server's database.
- Events are the engine's `Event`, as savegames hold them ([engine-conventions.md](engine-conventions.md#json)): tagged with `type`, camelCase fields, enums as names, and the fields of an event that can be passed in `play`, which is `null` for a pass: `{"type":"Bid","play":{"amount":3,..}}`. The client sends only the event, never a submitter or a time. Game settings are the engine's `GameSettings`. Both are typed in `web/src/generated/Event.ts`. They appear in `submitEvent`, `queryDraft`, `updateSettings` and the `updateSettings` notification.

## Errors

| Code | Means |
|---|---|
| `badCall` | The call isn't one the server knows, or its arguments don't decode. `message` says what's wrong |
| `serverError` | The server failed while handling the call: a database error or a bug. The call may have done part of its work |
| `notLoggedIn` | The connection isn't authenticated, or its session ended. The client logs in again |
| `invalidGameEvent` | The game refuses an event or draft, or an argument doesn't fit the game: a seat it doesn't have, a report it doesn't have, a savegame that doesn't load. `message` is the reason, rendered with the default skin |
| `gameNotFound`, `scheduledGameNotFound` | No game with that id |
| `userNotInGame` | The caller isn't in the game, or only observes it and the call is for players. Also a target user who isn't in the game |
| `noHost`, `noCreator`, `notAdmin` | The call is for hosts, the game's creator, or the admin |

The account calls have codes of their own (`invalidUserNameOrPassword`, `userNameExists`, `resetTokenExpired`, ...), and so do joining and seating (`incorrectGamePassword`, `seatNotAvailableNotOpen`, `seatNotAvailableKicked`, ...). `ErrorCode` in `rust/protocol/src/error.rs` lists them all with their default texts. The `message` is the server's text for that case where it has one, and the code's default text otherwise.

## Calls

`Call` in `rust/protocol/src/lib.rs` declares every call with its arguments and reply type. Every call but `serverInfo` and the ones that log in (below) is made as the connection's user. Who may make a call:

- **players**: users seated in the game. Observers and users outside the game get `userNotInGame`.
- **members**: players and observers. Others get `userNotInGame`.
- **hosts**: the game's hosts. Others, players and observers included, get `noHost`.
- **creator**: the user who created the game. Others get `noCreator`.

A game the server doesn't know gives `gameNotFound` before any of these.

### Server and accounts

| Call | Reply | Notes |
|---|---|---|
| `serverInfo` | `ServerInfo` | Needs no login. Call once after the socket opens |
| `createUser` | `NewSession` | Authenticates the connection with a new session |
| `login` | `NewSession` | `version` must be the server's `Game::LATEST_VERSION` (191, `latestVersion` in the lobby data), or the call fails with `invalidGameVersion`. Authenticates the connection with a new session |
| `authenticate` | `Account` | Authenticates the connection with the session of `token` |
| `linkLogin` | `LinkLoginInfo` | The link token of an async-play mail. Authenticates the connection with a new session; the link works once |
| `logout` | null | Ends the connection's session, which logs out every connection that uses it |
| `requestPasswordReset` | null | Mails a reset token |
| `setPassword` | `NewSession` | With the mailed token. Ends the user's other sessions and authenticates the connection with a new one |
| `updateUserInfo` | `Account` | A null or empty `passwordHash` keeps the password. A new one ends the user's other sessions |
| `setUserStatus` | `Lobby` | |
| `heartbeat` | `Lobby` | `scope`: `None` lists no games, `Active` games with activity in the last two hours and those the caller is seated in, `All` every game |

Passwords are hashed by the client: SHA-256 of the password's UTF-8 bytes, as 64 lowercase hex digits (in the browser, `crypto.subtle.digest("SHA-256", new TextEncoder().encode(password))`). That is what the previous client sent, so accounts made before the Rust server keep their passwords. The server hashes that value again with Argon2id and a random salt before storing it ([server.md](server.md#accounts-and-sessions)). The client should send `heartbeat` every few seconds: the user list only shows users seen in the last minute.

#### Sessions

- A session token is 32 random bytes as 64 hex digits. The server returns it once, in the `NewSession` of the call that started the session, and keeps only its SHA-256.
- The client keeps the token out of URLs. The web client keeps it in `sessionStorage`, and in `localStorage` when the user asks to stay logged in, and authenticates each new WebSocket with `authenticate` before anything else.
- A session ends 90 days after it started, or 30 days after a connection last authenticated with it, whichever comes first. A user keeps at most 20 sessions; a new one ends the least recently used.
- `logout`, a new password (`setPassword`, or `updateUserInfo` with one, which keeps the caller's own session) and deleting the account end sessions. Every connection authenticated with an ended session gets `sessionEnded`, stops following its games, and is nobody again.
- The "Jump to game" link of an async-play mail is `<site>/link/<linkToken>`. The link token is not a session: it works once, within 24 hours, for `linkLogin`, which starts a new session.

### Lobby and game management

| Call | Reply | Notes |
|---|---|---|
| `createGame` | `GameInitInfo` | From a savegame, or a new game when `savegame` is null or empty. An empty name names it after the creator. At most 10 running games per creator |
| `loadGame` | null | Hosts. Replaces the game with a savegame |
| `updateSettings` | null | Hosts. The settings for the game's start (`GameSettings`) |
| `closeGame` | `Lobby` | Creator. Removes everyone and deletes the game |
| `gameState` | `GameInitInfo` | Members. Also resends the caller's game view |
| `scheduleGame` | `Lobby` | At most 3 scheduled games per user. An unknown faction or ruleset is a `badCall` |
| `cancelScheduledGame` | `Lobby` | The scheduled game's creator |
| `subscribe` | `Lobby` | |

### Seating and participation

| Call | Reply | Notes |
|---|---|---|
| `joinGame` | `GameInitInfo` | A null `seat` joins without a particular seat, which works only while the game awaits players and has room. A particular seat must be one a host opened (`seatNotAvailableNotOpen`); kicked users can't come back (`seatNotAvailableKicked`). A player of the game only follows it again |
| `observeGame` | `GameInitInfo` | Not for players or kicked users |
| `reconnectGame` | `GameInitInfo` | Members |
| `leaveGame` | `Lobby` | Members |
| `setOrUnsetHost` | null | Hosts; the user must be a player of the game |
| `openOrCloseSeat` | null | Hosts; the seat must be a player's seat in the game |
| `kick` | null | Hosts; the user must be another member of the game |
| `assignSeats` | null | Hosts; `[[userId, seat]]`, each user a player of the game, each seat a player's seat or -1, no user or seat twice |
| `setBotSpeed` | null | Hosts; 0 to 4. See [Bots](#bots) |
| `nudgeBots` | null | Players; runs the bots' turn at once. See [Bots](#bots) |

Creating, joining, observing and reconnecting to a game, and `gameState`, send the caller a game view before the reply. `GameInitInfo.phaseControl` is set in the replies to join, observe, reconnect and game state.

### Playing

| Call | Reply | Notes |
|---|---|---|
| `submitEvent` | null | Players |
| `undo` | null | Hosts. Back to the moment before event `until` |
| `restoreUndone`, `dismissUndone` | null | Hosts |
| `setTimer` | null | Hosts. The bidding time limit in seconds, -1 for none |
| `setSkin` | null | Hosts |
| `phaseDone` | null | Players. The caller has acted or passes in the current pause, and stops holding its countdown |
| `phaseHold` | null | Players. Holds the running countdown; does nothing when none is running |
| `phaseRelease` | null | Players. Stops holding |
| `phaseContinue` | null | Hosts. Ends the current pause, but only while the game has `eventNumber` events, so a click that arrives after the server already continued does nothing |

The event is an `Event` (see [Encoding](#encoding)). It is always the caller's own. The client sends no submitter, and the server makes the caller's faction (none before factions are dealt, or for a host without a seat) the submitter, so the engine accepts it only as an action of that faction, or a host action when the caller is a host. It sets `seat` and `initiatorPlayerName` of `FactionSelected` to the caller's. `EstablishPlayers` is for hosts (`noHost` otherwise), and the server draws its `seed`, which decides every deck. The server sets the time, validates and executes the event, and on success sends every participant a new view. A failed validation, or an event that doesn't decode, is `invalidGameEvent` with the reason in `message`, rendered with the default skin.

Pauses between phases continue on their own. After each event the server works out who the current pause waits for and whether it runs a countdown that any player can hold (`rust/server/src/pause_rules.rs`). Once nobody is left to wait for and the countdown has run out or every holder has released it, the server submits `EndPhase` itself. A player waited for is done once they call `phaseDone`, or, in the revival step, once they revive. `PhaseControlInfo.reasons` says what each waited seat may do (`PauseReason`). Every reason is public; some need a secret card or spice, and the pause waits regardless, so the client words them as possibilities to everyone but the player concerned (`web/src/components/PauseReasons.tsx`). After an undo or a loaded game the current pause waits for the host's `phaseContinue` or the next event.

A valid `EstablishPlayers` event also seats the users on the players' seats, in the order they joined (`assignSeats` notification). Seats that no user takes become bots (see [Bots](#bots)).

### Chat

| Call | Reply | Notes |
|---|---|---|
| `chat` | null | Players. To everyone in the game, or with `to` to one member and the sender (`userNotInGame` for a user outside the game) |
| `globalChat` | null | To every connection |

The message the others get (`ChatMessage`, `GlobalChatMessage`) names the caller as `from`.

### Admin

The caller's user name must be the configured admin (`TREACHERY_ADMIN`), except for `deleteUser`, which a user may also call on their own account (others get `notAdmin`). Others get `invalidUserNameOrPassword`. Each call but `adminInfo` replies with a text for the admin page.

| Call | Reply |
|---|---|
| `adminSetMaintenance` | The time of the maintenance window, which `serverInfo` shows every client |
| `adminSaveAll` | Saves every game with unsaved changes now |
| `adminReloadAll` | Loads the saved games that aren't running; running games stay as they are |
| `adminCloseGame` | Removes everyone and deletes the game |
| `adminDownloadGame` | The savegame |
| `adminCancelScheduledGame` | |
| `deleteUser` | Deletes the account, ends its sessions (`sessionEnded`), and closes the games it created |
| `adminInfo` | `AdminInfo` |

## Notifications

Each notification names what happened. Changes to the game itself arrive as game views.

| Notification | Data | Sent to |
|---|---|---|
| `gameView` | `GameView`, see [Game view](#game-view) | each member of the game group, their own view: after events, undo, restore, dismiss, and anything else that changes the game |
| `loadGame` | `GameInitInfo` | game group, followed by a `gameView` |
| `updateSettings` | `{settings}` (`GameSettings`) | game group |
| `joinGame` | `{user, name, seat}` | game group |
| `observeGame` | `{user, name}` | game group |
| `setOrUnsetHost` | `{user}` | game group |
| `openOrCloseSeats` | `{seats}` | game group |
| `removeUser` | `{user, kick}` | game group |
| `assignSeats` | `{seats}`: `[[userId, seat]]` | game group |
| `botSpeed` | `{speed}` | game group |
| `setTimer` | `{value}` | game group |
| `phaseControl` | `PhaseControlInfo` | game group, when what the current pause waits for changes |
| `setSkin` | `{skin}` | game group |
| `chatMessage` | `ChatMessage` | game group, or the target's and the sender's connections in it |
| `globalChatMessage` | `GlobalChatMessage` | every connection |
| `sessionEnded` | | the connections authenticated with a session that just ended. They are nobody again and follow no game |

The game group is every connection that created, joined, observed or reconnected to the game, until it closes or its user leaves or is kicked, the caller's included. Participation changes (`joinGame`, `removeUser`, `assignSeats`, ...) can change what each member may see, so the game-view section decides whether they are followed by a `gameView`.

## Bots

The server plays every seat without a user (`rust/server/src/server/bots.rs`; the bots are the `treachery-bots` crate).

- Which seats are bots: `EstablishPlayers` fills the seats that no user takes with bots (`Settings.NumberOfPlayers` players in all). A seat also becomes a bot when its user leaves or is kicked, when the host reassigns the seats without it, or when a loaded savegame has it empty. A user who takes the seat (an open seat, `joinGame`) replaces the bot. `GameView.players[].isBot` and `GameInfo.bots` show them.
- When bots act: after every event, and after a loaded game, undo, restore or dismiss, a seat assignment, a leave or kick, a new observer, the first seated join and a bot speed change, the server schedules a bot turn, replacing one scheduled earlier, after a delay that depends on the last event and the phase, divided by the bot speed (800 ms after an `AllyPermission`, `DealOffered` or `SetShipmentPermission`, 1600 ms after a `Bid`, 2400 ms in revival and charity, 9600 ms in shipment and battle, 4800 ms otherwise). A turn does nothing while the game awaits players or the speed is 0. Otherwise each bot, in a random order, is asked for its most urgent action, then the next level, and the first action found is executed as an event of that bot. The bots end phases themselves (`EndPhase` and other host actions) only when the game has no host and at least one observer. Otherwise the phase control continues the pauses, and a turn in which no bot acts tells it the bots have passed.
- `botsSpeed` (`ParticipationInfo.botsSpeed`, default 1) is the divisor. 0 pauses the bots. A host may set 0 to 4; the client offers 0 (pause), 1, 2 and 4. The game's group gets the `botSpeed` notification.
- `nudgeBots` runs a bot turn at once (unless the bots are paused), for a game whose bots stopped (a refused bot event schedules no next turn). The game's players may call it.
- A bot is created when its faction is first asked, and keeps its state for the rest of the game, across undo. A server restart creates new bots.

## Game view

Built by `rust/view` ([client.md](client.md)). Wire types: `rust/view/src/schema.rs` and `schema/`, which document every member, generated to TypeScript in `web/src/generated/`.

The server sends each participant a `gameView` notification: what that user may see of the game, plus the actions they can take now and the valid choices for each. The client renders only views. While a player fills in an action, the client asks the server about the inputs chosen so far (a draft query), and submits the finished event with `submitEvent`.

### Encoding

- Member names in camelCase.
- Enum values by member name (`"Atreides"`, `"ShipmentAndMove"`), the keys the skin JSON uses. The TypeScript unions are in `web/src/generated/enums.ts`, with `PhaseValue`, `MainPhaseValue` and a few more holding the numbers, for ordered comparisons.
- Static game objects by id: locations, territories, leaders, treachery cards, and resource cards (by skin id). Names, images and map positions come from the skin and the static data (`web/public/static/data.json`), never from the view.
- Text the game writes (log lines, prompts, validation messages, form headings) arrives rendered to [rich text](#rich-text).

Events are in the format of [Encoding](#encoding), both in draft queries and in `submitEvent`. A field the event doesn't have makes it fail to decode.

### When views are sent

| After | Who gets a new view |
|---|---|
| an event (`submitEvent`), undo, restore, dismiss, `loadGame` | every member of the game group |
| `joinGame`, `removeUser`, `assignSeats`, `setOrUnsetHost`, `openOrCloseSeats`, `botSpeed` | every member: seats decide `me`, host status decides actions, seat occupancy decides `isBot` |
| a successful `createGame`, `observeGame`, `reconnectGame`, `joinGame`, `gameState` | the caller (and, for a join, every member, through the `joinGame` notification) |
| `setSkin` | every member, because labels are rendered with the game's skin |
| `setTimer` | every member: the timer is `timing.timer` |
| `updateSettings` | nobody: settings only matter before the game starts |

Each view is complete; there are no diffs. A view of the alliance phase of a 6-player game is about 22 KB of JSON, most of it the log.

```json
{"notify":"gameView","data":{"eventNumber":612,"version":191,"turn":6, "...": "..."}}
```

### Calls

| Call | Reply | Notes |
|---|---|---|
| `queryDraft` | `DraftResult` | Players. `draft` is an `Event`; `invalidGameEvent` when it is not one of the caller's actions in their latest view (`message` says why) |
| `report` | `ReportLines` | Members. The lines of the report at `index` in `log.reports`, filtered for the caller as in the view; `invalidGameEvent` for an index that has no report |
| `undoList` | `UndoEntry[]` | Hosts. `{eventNumber, text: RichText, listed, turn, phase}` for every event: its short message, whether the undo panel lists it (ally permissions, battle revisions and automation settings are left out), and the turn and main phase it happened in. `eventNumber` is the event's index in the history, the `until` of `undo` |
| `undoPreview` | `GameView` | Hosts. The caller's view at gap `gap`, the moment just before event `gap` (the game after `undo` to `gap`), with no actions and no `playableWith`. The gap is clamped to 1..number of events. Sends no notification; the live game and its views are untouched |
| `saveGame` | string | Hosts and the admin (`noHost` otherwise): the server's game as a savegame string. Always the live game, also while the host previews an undo |

A connection that isn't authenticated, or a game the server doesn't know, gives the same errors as the other calls (`notLoggedIn`, `gameNotFound`).

A draft is answered for the caller's faction, so a client can't ask about another faction. A draft made from an older view is answered against the current game; the client discards answers for views it has replaced.

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
| `waitingActions` | `WaitingActionView[]` | actions the viewer will get but can't take yet, each with a header and the reason; only Prescience while it waits for Voice |
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

- `eventType`: the event's type name, also the `type` to submit.
- `options`: the form's choices, `ActionOptions`, tagged by `form`.
- `validation`: the validation message of the event the form would send with its default inputs, as `RichText`, or null. The confirm button shows it and is disabled while it is set. A validation that fails inside the engine gives "Validation failed".

The default inputs are the form's starting values after its widgets have adjusted them (a number picker clamps its value into its range, a required choice falls back to the first value). Forms whose payload holds a `draft` send that draft back adjusted; the validation is the adjusted draft's.

`ActionOptions` has one variant per form, named after the event type, except `Simple` (a confirm button with `title`, `confirmLabel`, `alert` and `allowAutoConfirm`, used by about twenty event types and by `EndPhase`, the prompt's "Continue") and `YesNo` (two buttons). The payloads are in `schema.rs` and `schema/{battle,ship_move,revival,other,setup}.rs`, which say what each member holds.

#### Dependent inputs

Some choices depend on another input of the same form: the weapons for the chosen leader and defense, the maximum forces from the chosen source, the questions for the chosen target. For these the client sends the draft with `queryDraft` and gets a `DraftResult`, `{eventType, options, validation}`, with options computed from the draft's inputs. The client sends a query after each input change, drops answers older than its latest query, and keeps the previous options on screen until the answer arrives. A form with no dependent inputs still queries, for the validation next to its confirm button.

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

`log.reports` lists every report (`{index, turn, about, isBattle, aggressor, defender, territory, battleDecided, battleWinner}`), and `log.latest` holds the lines of the reports of the latest phase, which the report panel shows by default. When the player steps back to an earlier phase, the client calls `report` for each report of that phase and caches the answers: a report's lines never change once its phase is over, except the latest one.

## Changing the protocol

- A call is one line in the `calls!` list of `rust/protocol/src/lib.rs`: its wire name, its variant, its arguments and its reply type. The server handles it in `Server::handle` (`rust/server/src/server/mod.rs`), and the compiler points at every match that needs the new call.
- A notification is a variant of `Notification` in the same file. The lobby objects are in `src/lobby.rs`, and the error codes in `src/error.rs`.
- Then regenerate the TypeScript types with `UPDATE_TS_BINDINGS=1 cargo test --release -p treachery-protocol --test ts_bindings`, and fix what `npm run build` in `web/` reports. Doc comments on the Rust types carry over to TypeScript.
- The server's tests (`rust/server/tests`) drive it through these messages without sockets. `scripts/ui/e2e_server.py` drives it through the web client ([client.md](client.md#checks)).
