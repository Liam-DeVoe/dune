# The server

`rust/server` runs treachery.online: accounts, the lobby, games and their bots. Clients talk to it over one WebSocket each, in the JSON format of [protocol.md](protocol.md). It serves the web client's files too. Everything that must survive a restart is in one SQLite file.

## Threads

One thread, the game thread, owns all state: accounts, sessions, running games, connections. It handles one thing at a time to completion, whether that's a call from a client, a closed socket or a timer. So handlers take `&mut self`, need no locks, and everything players see happens in one order. The engine's `Game` isn't `Send`, which is the other reason for the single thread.

The HTTP server (axum, on a multi-threaded tokio runtime) accepts sockets. Each socket task forwards incoming text to the game thread as a `Command` and writes back whatever the game thread queues for it. See `src/http.rs` and `src/socket.rs`.

A call's notifications go out as the game thread produces them, then the game views it caused, then its reply.

## Modules

| Module | What it does |
|---|---|
| `http.rs` | Starts the game thread and the HTTP server; saves and stops on Ctrl-C or SIGTERM |
| `socket.rs` | One WebSocket: 4 MiB frames, a ping every 15 s, closed after 30 s of silence |
| `server/` | The `Server`: state and every call. `accounts`, `lobby`, `games` (seats, hosts, undo), `play` (events, mail), `bots`, `pauses`, `admin`, `storage` (loading and saving), `scene` (dev scenes) |
| `protocol.rs` | Decodes a frame into a `Request`, encodes replies and notifications |
| `wire.rs`, `participation.rs` | The objects in replies and notifications |
| `connections.rs` | Open sockets, the session each is logged in with, which connection follows which game, notifications |
| `views.rs` | Each participant's `GameView`, built on a replica of the game (`rust/view`), and the view calls |
| `pause_rules.rs` | What each pause between phases waits for |
| `db.rs` | The database and its migrations |
| `settings.rs` | Settings from environment variables |
| `passwords.rs` | Argon2id hashes of account passwords |
| `time.rs`, `timers.rs`, `mail.rs`, `ids.rs`, `render.rs`, `static_files.rs` | Clock, delayed work, mail, random tokens and ids and the digests tokens are stored as, text with the default skin, the web client's files |

Views never use the running game itself: building a view asks the engine questions that can change a game's state. Each followed game has a replica that replays the running game's events (`views.rs`).

## Accounts and sessions

The client sends the SHA-256 of a password, as the C# client did. The server stores an Argon2id hash of that value (`passwords.rs`: a random 16-byte salt, 19 MiB, 2 passes, 1 lane, OWASP's minimum), as a PHC string that names its own parameters. Hashing a password takes a few tens of milliseconds on the game thread, once per login, account or password change.

Each connection is logged in with at most one session (`Connections::log_in`). The calls that log in bind one; every other call takes its user from there (`Server::caller`), and none carries a token. Session tokens are 32 random bytes from the operating system. The server keeps only their SHA-256, in memory and in the database. A session ends 90 days after it started or 30 days after its last login on a connection, and a user keeps at most 20. Logout, a new password and deleting the account end sessions (`end_session`), which sends `SessionEnded` to their connections, logs them out and stops them following games. A call on a connection whose session expired since it logged in ends it the same way.

Async-play mails carry a login link instead of a session token: a token for `RequestLinkLogin` that works once, within 24 hours, and starts a new session. Like sessions, links are stored as digests and survive a restart.

## Who may do what

The server is the only judge of who acts in a game. Each call checks the caller's role (protocol.md, Methods): players for events, draft queries, pauses, nudging the bots and game chat; members (players and observers) for reports, the game state and reconnecting; hosts for undo, loading, settings, seats, hosts, kicks, the timer, the skin, bot speed and Continue. A submitted event is rewritten as the caller's: `Initiator` is the caller's faction, `FactionSelected` gets the caller's seat and name, and the server draws the seed of `EstablishPlayers`, which only hosts may submit. Chat messages get the caller as their source, and a message to one user reaches only that user and the sender.

## Persistence

The server keeps everything in memory and writes changes through to the database. Accounts, sessions and scheduled games are written at once. A changed game is saved 5 seconds after its first unsaved change, so a burst of events costs one write. Shutdown saves every unsaved game. At startup the server loads everything back. A game whose savegame no longer loads, for example because an event was renamed, is left out with a log line and stays in the database.

The tables are `users`, `sessions`, `login_links`, `games` and `scheduled_games`. Times are RFC 3339 text in UTC. SQLite's `user_version` holds the schema version, and `Db::open` migrates older databases in one transaction before the server starts:

- Version 1 is the first schema. A database of the old C# server (Entity Framework's tables, recognised by `__EFMigrationsHistory`) is converted to it: accounts, sessions, games and scheduled games are copied, and the old tables are dropped.
- Version 2 hashes each stored password, the client's SHA-256 as sent, with Argon2id once, and marks it `users.password_scheme` 1. The server refuses to load a user with another scheme. Pending reset tokens are kept as digests (`reset_token_digest`). Every session ends, because their tokens had been in URLs and mails; `sessions` now holds token digests and the last use. `login_links` is new.

`tests/persistence.rs` checks both steps on a database the C# server created, and `db.rs`'s unit tests check the version 1 to 2 step.

## Timers

Bot turns, async-play mail, the countdown of a pause and saving a game are `Timer`s. In production a timer sleeps on the game thread and then queues itself as a command. Tests record them and run them when they move a manual clock forward.

Each game has at most one pending bot turn: scheduling a new one replaces the old. The delay depends on the last event and the phase, divided by the bot speed. At speed 0 the bots don't act.

## Settings

Environment variables, listed with their defaults in `src/settings.rs`. Production sets them in `deploy/treachery.service`. Mail goes out only when `TREACHERY_SMTP_USER` is set; otherwise the server logs each mail it would have sent.

## Tests

The tests in `tests/` drive the server through the protocol only: they send the JSON a client sends and read what it gets back. `tests/common/mod.rs` runs a server in process on a temporary database with a manual clock, recorded timers and recorded mail, and can restart it on the same database. The files are by area: `accounts` (logins, sessions, logout, expiry), `authority` (outsiders, observers and other players refused; events rewritten as the caller's), `games` (lobby, seats, chat, the envelope), `play` (events, views, undo, pauses, bots, async mail and its login links), `admin`, `persistence` (restarts, the C# database) and `dev_scene`. The test server draws seed 12345 for every new game (`Services::seeds`), so games started in tests play out the same each time. `TREACHERY_DB=<file> cargo test -p treachery-server --test persistence` also opens a copy of a real database and checks every account is there.

End to end, `scripts/ui/e2e_server.py` starts the release binary with the web build and drives browsers through Playwright: two players, logging in through the form and out, and the dev scene.

## Behaviour that differs from the C# server

The server was first a line-by-line port of the C# `GameHub`. These differences were made on purpose when it was rewritten:

- A refused event's message no longer ends in `*`.
- An invalid `EstablishPlayers` is refused like any other event, and the seats stay as they were.
- Async-play mail goes out when the game starts. Before, the seats were assigned after the mail, so nobody got the first one.
- Each login starts its own session; logging in again no longer returns the token of an earlier session.
- Calls carry no token: a connection logs in once and acts as that user. Session tokens are random 32-byte values stored as digests, expire after 30 idle or 90 days, and end on logout, a new password or the account's deletion. Async-play mails link with a single-use login token, not a session token.
- Passwords are stored as Argon2id hashes of the client's SHA-256, not the SHA-256 itself.
- The server sets an event's `Initiator`, `FactionSelected`'s seat and name, and the game's seed; only hosts start a game. Before, any user who knew a game's id could act as any faction in it.
- Observers and users outside a game can't submit events, query drafts, chat in it, take part in pauses or nudge its bots; outsiders can't read its reports.
- Chat messages carry the caller as their source, and a message to one user goes only to that user and the sender.
- Settings are for hosts, not the creator alone. Hosts can only make players hosts, kick members of the game, open real seats and seat players on real seats, and set bot speeds of 0 to 4. A user joins a particular seat only once it is open, and a kicked user can't observe.
- `RequestReconnectGame` returns the game's name.
- Leaving a game or being kicked stops every connection of that user from following it, not only the last one.
- Changes are saved within seconds and on shutdown, not every five minutes.
- The admin's `AdminRestoreState` loads the saved games that aren't running and leaves running games alone. `AdminCloseGame` also deletes the game from the database.
- Deleting a user ends their sessions, and they leave every game they joined, as if they had left it themselves.
- A mail to an address that doesn't parse is skipped with a log line. The call that sends it still succeeds.
- The lobby is computed for each request instead of from a copy refreshed every 6 seconds.
- A bot speed change is sent to the game's members only, not to every connection on the server.
