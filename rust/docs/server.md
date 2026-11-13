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
| `connections.rs` | Open sockets, which connection follows which game, notifications |
| `views.rs` | Each participant's `GameView`, built on a replica of the game (`rust/view`), and the view calls |
| `pause_rules.rs` | What each pause between phases waits for |
| `db.rs` | The database and its migrations |
| `settings.rs` | Settings from environment variables |
| `time.rs`, `timers.rs`, `mail.rs`, `ids.rs`, `render.rs`, `static_files.rs` | Clock, delayed work, mail, random tokens and ids, text with the default skin, the web client's files |

Views never use the running game itself: building a view asks the engine questions that can change a game's state. Each followed game has a replica that replays the running game's events (`views.rs`).

## Persistence

The server keeps everything in memory and writes changes through to the database. Accounts, sessions and scheduled games are written at once. A changed game is saved 5 seconds after its first unsaved change, so a burst of events costs one write. Shutdown saves every unsaved game. At startup the server loads everything back. A game whose savegame no longer loads, for example because an event was renamed, is left out with a log line and stays in the database.

The tables are `users`, `sessions`, `games` and `scheduled_games`. Times are RFC 3339 text in UTC. SQLite's `user_version` holds the schema version, and `Db::open` migrates older databases in one transaction before the server starts. Version 1 is the first schema. A database of the old C# server (Entity Framework's tables, recognised by `__EFMigrationsHistory`) is converted to it: accounts, sessions, games and scheduled games are copied, and the old tables are dropped. `tests/persistence.rs` checks this on a database the C# server created.

## Timers

Bot turns, async-play mail, the countdown of a pause and saving a game are `Timer`s. In production a timer sleeps on the game thread and then queues itself as a command. Tests record them and run them when they move a manual clock forward.

Each game has at most one pending bot turn: scheduling a new one replaces the old. The delay depends on the last event and the phase, divided by the bot speed. At speed 0 the bots don't act.

## Settings

Environment variables, listed with their defaults in `src/settings.rs`. Production sets them in `deploy/treachery.service`. Mail goes out only when `TREACHERY_SMTP_USER` is set; otherwise the server logs each mail it would have sent.

## Tests

The tests in `tests/` drive the server through the protocol only: they send the JSON a client sends and read what it gets back. `tests/common/mod.rs` runs a server in process on a temporary database with a manual clock, recorded timers and recorded mail, and can restart it on the same database. The files are by area: `accounts`, `games` (lobby, seats, chat, the envelope), `play` (events, views, undo, pauses, bots, async mail), `admin`, `persistence` (restarts, the C# database) and `dev_scene`. `TREACHERY_DB=<file> cargo test -p treachery-server --test persistence` also opens a copy of a real database and checks every account is there.

End to end, `scripts/ui/e2e_server.py` starts the release binary with the web build and drives two browsers through Playwright.

## Behaviour that differs from the C# server

The server was first a line-by-line port of the C# `GameHub`. These differences were made on purpose when it was rewritten:

- A refused event's message no longer ends in `*`.
- An invalid `EstablishPlayers` is refused like any other event, and the seats stay as they were.
- Async-play mail goes out when the game starts. Before, the seats were assigned after the mail, so nobody got the first one.
- Each login starts its own session; logging in again no longer returns the token of an earlier session.
- `RequestReconnectGame` returns the game's name.
- Leaving a game or being kicked stops every connection of that user from following it, not only the last one.
- Changes are saved within seconds and on shutdown, not every five minutes.
- The admin's `AdminRestoreState` loads the saved games that aren't running and leaves running games alone. `AdminCloseGame` also deletes the game from the database.
- Deleting a user ends their sessions, and they leave every game they joined, as if they had left it themselves.
- A mail to an address that doesn't parse is skipped with a log line. The call that sends it still succeeds.
- The lobby is computed for each request instead of from a copy refreshed every 6 seconds.
- A bot speed change is sent to the game's members only, not to every connection on the server.
