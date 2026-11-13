# Development

treachery-online is an online version of the Dune board game, running at https://treachery.exe.xyz/. This file covers how to work on it. Agents working on the code read it first.

## Layout

- `rust/engine`: the game rules. Events, validation, the game log. [engine.md](rust/docs/engine.md) describes the modules and [engine-conventions.md](rust/docs/engine-conventions.md) the coding conventions.
- `rust/bots`: the bots.
- `rust/view`: builds each player's view of a game, including the actions they can take.
- `rust/server`: the server ([server.md](rust/docs/server.md)). It holds every game and sends views to clients over a WebSocket ([protocol.md](rust/docs/protocol.md)).
- `rust/skin`: text and image names for log messages ([skin.md](rust/docs/skin.md)).
- `rust/observe`: the behavior harness ([native-harness.md](rust/docs/native-harness.md)).
- `rust/dotnet-compat`: copies of the few .NET behaviors the game still depends on, mainly the random number generator ([compat.md](rust/docs/compat.md)).
- `web/`: the React client ([client.md](rust/docs/client.md)), with its art, fonts and stylesheets.
- `deploy/`: everything needed to rebuild and deploy the server ([deploy/README.md](deploy/README.md)).

The code started as a port of a C# version, which is gone now (it's in git history before commit f794887). [behavior-changes.md](rust/docs/behavior-changes.md) lists where behavior deliberately differs from it.

## Checking a change

Run `rust/tools/check.sh` before reporting anything done. It runs every test in the workspace in release mode and names each failure. Extra arguments go to `cargo test`, so `rust/tools/check.sh -p treachery-engine` checks one crate. Test bugs have gone unnoticed for a long time when people only ran the tests they were working on, so run the whole thing.

For anything that touches game behavior, also run the behavior harness. It plays bot games and random "fuzz" games through a frozen reference build and through your code, and compares everything a player can see: which events are accepted, the log, each player's view, and the bots' decisions. A refactor must pass with no differences. A deliberate change must show only the differences it intends. [native-harness.md](rust/docs/native-harness.md) explains how to run it, read its reports and allow intended differences.

Run 20 games at a time while working (`--games 20`), or one game with `observe first`, and 200 bot games plus 200 fuzz games once at the end.

## Rules

1. **Every bug found late is also a testing bug.** When a bug turns up outside the tests that should have caught it, fix it, then add an entry to [escapes.md](rust/docs/escapes.md): what the bug was, why the tests missed it, and what test now catches that kind of bug. Add that test before closing the entry.
2. **Behavior changes are deliberate.** Changing what players see or how games play out needs a reason in [behavior-changes.md](rust/docs/behavior-changes.md), a test, and a harness run showing that nothing else changed.
3. **Only the coordinator replaces the harness reference.** The reference binary in `~/treachery-native-ref/observe` and its source in `~/treachery-native-ref/rust` are shared by every checkout and must match main. They're replaced when a deliberate behavior change is merged. If your branch changes behavior, build your own reference into a separate path and pass it with `--reference`.
4. **Only the current game version is supported.** The engine refuses games and savegames of any other version.

## Shared machine limits

The machine has 8 cores and 16 GB of RAM, and several agents often share it.

- Run `observe live` and other long batches through `rust/tools/heavy.sh`. It allows two such runs at a time, and kills any process it started that grows past 3 GB. If a run is killed for memory, find out why before rerunning. Don't raise the cap.
- Use `--jobs 4` at most, and `--jobs 2` for anything new.
- Single-game repros and runs under a minute can skip the queue with `HEAVY_SLOTS=4 rust/tools/heavy.sh ...`, which uses two extra slots and still applies the memory cap.
- Queue at most one heavy run at a time per agent.
- Don't poll a run with a loop that matches its own command line: `pgrep -f "observe live"` inside a shell whose arguments contain "observe live" never ends.
- Build and test with `--release` only. `check.sh` does this.

## Working in parallel

Agents working on the same crates at the same time each use their own git worktree, with its own Cargo target dir, so one agent's half-finished edit never breaks another's build. The coordinator merges their branches into main. An agent working alone can use the main checkout and the shared `rust/target`, as long as every edit leaves the workspace compiling. Delete a worktree's target dir when you're done.

Keep each agent's context under 500k tokens. Near that, write progress and next steps to the relevant doc and stop with a summary, so a fresh agent can continue.

## Deploying

`just deploy` deploys the working tree to production, including anything uncommitted. Agents never deploy unless their task says so, and then only after `rust/tools/check.sh` passes on the whole workspace. Back up the production database first (deploy/README.md says how).
