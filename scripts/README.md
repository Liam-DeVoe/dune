# Scripts

| Piece | What it does |
|---|---|
| `fixtures/` | Saved games in chosen situations. The server's dev scenes (`/scene/<fixture>/<faction>`, `TREACHERY_DEV_FIXTURES`) and many Rust tests load them |
| `ui/e2e_server.py` | End-to-end check of the web client against a running Rust server, in Chromium through Playwright. Its docstring says how to build and run it |
| `leader-tokens.py` | Draws each leader's strength on its token art in `web/assets/art` (`uv run scripts/leader-tokens.py`) |

## Fixtures

A fixture is a savegame: its event log, replayed on load, so it can only hold a state that legal events reach. Every fixture is at the latest game version, since the engine loads no other. The C# tool that played bots until a situation came up was deleted with the C# tree. A new fixture is a savegame whose events reach the situation, for example an existing fixture with its event list cut short.
