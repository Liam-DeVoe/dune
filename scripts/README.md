# Scripts

| Piece | What it does |
|---|---|
| `fixtures/` | Saved games in chosen situations. The server's dev scenes (`/scene/<fixture>/<faction>`, `TREACHERY_DEV_FIXTURES`) and many Rust tests load them |
| `ui/e2e_server.py` | End-to-end check of the web client against a running Rust server, in Chromium through Playwright. Its docstring says how to build and run it |
| `leader-tokens.py` | Draws each leader's strength on its token art in `web/assets/art` (`uv run scripts/leader-tokens.py`) |

## Fixtures

A fixture is a savegame: its event log, replayed on load, so it can only hold a state that legal events reach. Every fixture is at the latest game version, since the engine loads no other. The C# tool that played bots until a situation came up was deleted with the C# tree. A new fixture is a savegame whose events reach the situation, for example an existing fixture with its event list cut short.

A change to the random number generator makes every fixture replay differently: the first choice that names a dealt card or traitor is refused, and the rest of the log with it. `observe refit` carries a log over to such a change. It keeps the events that still pass validation, replaces the others with the same faction's event of the same type (as its bot or the client's form would build it), and lets the bots play on where needed, so the fixture reaches the same turn and phase with different cards. `rust/observe/src/refit.rs` explains how, and `observe --help` lists its options. The fixtures and recorded games were carried over to game version 189 this way ([native-harness.md](../rust/docs/native-harness.md#refitting-recorded-games)).
