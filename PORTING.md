# Rust port

We are porting treachery-online from C# to Rust. The requirement is that the
Rust game logic behaves exactly like the C# game logic: same validation
results, same game state after every event, same random draws, same log
messages, same bot decisions. The C# code is the specification, bugs included.

This file is the shared plan. Every agent working on the port reads it first.

## Rules

1. **C# is the spec.** If the C# code has a bug, the Rust code has the same
   bug. Example: in `Shared/Events/Donated.cs` the Double Inflation check builds
   a message and never returns it, so it has no effect. Port it with no effect
   and add a `// C# parity:` comment explaining why.
2. **Never change C# behavior.** The C# code may only gain test hooks that do
   not change behavior when unused (for example, a way to inject a seed where it
   currently uses an unseeded `Random`). Every such hook is listed in
   `rust/docs/csharp-hooks.md`.
3. **Only `Game.LatestVersion` is supported** (since 2026-11-11, native-plan.md
   step 2). The engine throws for any other version, and the oracle runs with
   `--versions latest` for the differ.
4. **Use the compat crate for .NET semantics.** `rust/dotnet-compat` holds exact
   copies of `System.Random`, `Dictionary`, `HashSet`, `List.Sort` and numeric
   rules. Engine code must use these wherever the C# uses the .NET type.
   `std::collections::HashMap`/`HashSet` are banned in engine code because their
   iteration order differs. See `rust/docs/hazards.md` for the full list of
   ways C# and Rust differ.
5. **Mirror the C# structure.** Same names in snake_case, same field names,
   same method order. (Modules no longer follow the C# files since native-plan.md
   step 3; `rust/docs/engine.md` describes the layout.)
6. **Every escaped bug is a testing bug.** When a difference between C# and
   Rust is found by anything other than the automated differential tests (or is
   found late), fix it, then write an entry in `rust/docs/escapes.md`: what the
   bug was, why our tests did not catch it, and what test change now catches
   that whole class of bug. Make that test change before closing the entry.

## Scope: only differences that can happen in practice

Equivalence matters for games real players and bots can produce. A difference is in scope only when it can arise from:

- a game created with settings the client or server can produce,
- events the unmodified client UI or a bot can send (with ids of real objects) that pass `Validate` in C#, and the server re-executing such events (undo, savegame loading),
- server-side calls on the live game (`GameStatus`, applicable events).

Out of scope: differences that need a forged id (-1, unknown card) even if validation accepts it, a null the client can't send, a broken game state, or execution without validation of an event that was never valid. Don't investigate or fix these. If one turns up, write one line in `rust/docs/unreachable.md` and move on. Agreement on *whether* an invalid event is rejected is in scope (the server shows that result to the player); what the engine does if it ran the invalid event anyway is not.

When it's unclear whether a difference is reachable, spend a few minutes trying to reproduce it with validated events from normal settings. If that fails, treat it as out of scope.

## How equivalence is checked

The C# **oracle** (`treachery-online/Oracle`) records traces. A trace is a
sequence of steps. Each step has the event (C# JSON format), whether it was
submitted as host, the validation result, any exception, a canonical dump of the
complete game state afterwards, and the applicable events for every player. The
Rust **replayer** (`rust/engine` tests and `rust/differ`) runs the same events
and must produce byte-identical output.

The canonical state dump is made by reflection over every field of `Game`,
public and private, recursively. This means a field the Rust port forgets shows
up as a difference immediately.

Trace sources, from cheapest to most thorough:

- **Focused tests**: hand-written scenarios for specific rules, ported from the
  C# tests plus new ones for each rules area.
- **Bot games**: bots play complete games with random rules, factions and
  player counts. Unlimited volume, but bots only do sensible things.
- **Fuzzing**: at every step, pick a random applicable event type and fill it
  with random field values, including invalid ones. Both engines must agree on
  validity and on the resulting state. This reaches edge cases that bots never
  try. Run fuzz checks with `--bots check`: C# bots sometimes change the game
  while deciding (rust/docs/known-issues.md, Rock Melter), and without the flag
  the replay reports that as a false difference.
- **Savegames**: real games, if we can get a corpus (the C# regression tests
  expect `savegame*.json` files that are not in this repo).
- **Coverage**: C# line coverage over the trace corpus tells us which rules
  code was never exercised. Uncovered code gets a focused test or a fuzzer
  improvement.

## Phases

0. Foundation: compat crate, oracle, hazard survey, Rust data types.
1. Engine (`Shared`): game logic, events, validation, reports.
2. Bots (`Bots`), checked by bot-vs-bot traces with seeded bot randomness.
3. Server (axum + websockets + sqlx, same SQLite schema).
4. Client (Rust/WASM UI) and ML.

## Layout

- `rust/dotnet-compat`: .NET semantics.
- `rust/engine`: port of `Shared`.
- `rust/docs/hazards.md`: C#/Rust differences that can break equivalence.
- `rust/docs/escapes.md`: postmortems for bugs our tests missed.
- `rust/docs/csharp-hooks.md`: test hooks added to the C# code.
- `treachery-online/Oracle`: C# trace recorder.

To use .NET 10: `export DOTNET_ROOT="$HOME/dotnet10" PATH="$HOME/dotnet10:$PATH"`.

## Before you report done

Run `rust/tools/check.sh`. It runs `cargo test --workspace --release` with `--no-fail-fast` (about a minute with a warm build), prints one line per test file and the name of every failing test, and exits non-zero on any failure or compile error. Report done only when it passes, or name each failure that is not yours and say whose area it is in. Tests marked `#[ignore]` are skipped. Extra arguments go to `cargo test`, so `rust/tools/check.sh -p treachery-engine` checks one crate.

Several test bugs went unnoticed for a long time because agents ran only the test files they were working on.

## Disk space

The machine now has about 280 GB free (2026-11-12), so disk is no longer the limit it was. Still delete large trace corpora when done, keep `rust/traces/sample/` under 50 MB, and check `df -h /` before writing more than 10 GB.

Agents working in parallel on the same crates (step 3 of native-plan.md) each work in their own git worktree with their own Cargo target dir, so one agent's half-finished edit never breaks another's build. The coordinator merges their branches. Agents working alone keep using the shared `rust/target`. Delete a worktree's target dir when the agent finishes.

## CPU

The machine has 8 cores, and agents running differ and oracle batches at the same time pushed the load average to 70–100, which made every run several times slower. Every agent must:

- Run differ `live`, oracle batches and coverage runs through `rust/tools/heavy.sh`, which allows two such runs at a time, with `--jobs 4` at most.
- Small runs (a single-game repro, five games at most) shouldn't queue behind long batches: run them as `HEAVY_SLOTS=4 rust/tools/heavy.sh ...`, which uses the extra slots 3 and 4 and still applies the memory cap.
- Size runs to answer the question at hand. Reproduce one failing game with `differ first` instead of rerunning a batch.
- Not poll a run with a loop that matches its own command line (`pgrep -f "differ live"` inside a shell whose arguments contain "differ live" never ends).

## Memory

The machine has 16 GB of RAM. A differ run once grew to 29 GB and ran the machine out of memory. Every agent must:

- Run differ, oracle and coverage commands through `rust/tools/heavy.sh`, even though `differ live` takes a CPU slot by itself. `heavy.sh` kills any process it started that uses more than 3 GB (`HEAVY_MEM_MB`). `ulimit -v` does nothing on macOS.
- If a run is killed for memory, find out why before rerunning. Don't raise the cap to get past it.
- Keep `--jobs` low (4 at most, 2 for anything new or untested).

## Decisions for phases 3 and 4 (2026-11-08)

- Bots (phase 2) are deferred. The Rust server offers no bot seats until they are ported.
- Server: axum + tokio + sqlx on the same SQLite schema as the C# server.
- The server is authoritative and the only holder of game state. It runs the Rust engine. After each event it sends each player (and observer) their own view: what they may see, plus the actions they can take now and the valid choices for each. Inputs that depend on other inputs ask the server over the socket. Clients never replay events and never see hidden information.
- Server and client talk JSON over a WebSocket (spec in `rust/docs/protocol.md`). The old C# client is not supported by the new server.
- Client: React + TypeScript, with no engine and no WASM. It looks and behaves like the C# client, but its code is a redesign, not a port.
- Client verification: Playwright drives both clients through the same game states and compares screenshots and DOM.
- The engine stays a port, and equivalence rules apply to it and to server logic. The per-player views and action options are new code built on the engine; they are checked against what the C# client shows, not against C# code.

## Agent context budget

Keep each agent's context under 500k tokens. Near that, write progress and next steps to the relevant plan doc and stop with a summary, so a fresh agent can continue.

## Keeping the shared build green

Many agents build the same workspace, so one agent's half-finished edit blocks every other agent's tests. Every agent must:

- Make each edit leave the workspace compiling. Add a new module or function with a stub body first, then fill it in. Never leave a caller and its callee out of step between two tool calls.
- After editing a shared file (`rust/view/src/actions.rs`, `schema.rs`, `server/src/runtime.rs`, `web/src/generated`), run `cargo check --workspace` at once and fix any break you caused before doing anything else.
- When the build is broken by someone else's file, wait a few minutes and retry. If it stays broken for 15 minutes, say so in your report rather than editing their file.

## Build profile and comparison runs (2026-11-10)

- Build and test with `--release` only (`check.sh` does). A debug build doubles `rust/target` and disk is tight.
- `HEAVY_SLOTS=4` (slots 3–4) is for single-game repros and runs of a few seconds to a minute only. Screenshot comparisons over more than one fixture go through the normal two slots.
- Screenshot runs over many fixtures (`just shots`) are heavy runs too: run them through `rust/tools/heavy.sh`, with `--workers 2` at most. Memory is the limit.

## Bot porting runs (2026-11-11)

Five bot packages share the same done check, the differ in bots mode. To keep the two heavy slots free:

- While iterating, run 30–50 bot games (`--games 50`), or a single game with `differ first` when chasing one mismatch.
- Run the full 200-game check once, at the end, with `--jobs 4` through `heavy.sh`.
- The oracle and the differ must come from the same commit: after B0 changes the trace format, rebuild the differ (`cargo build --release -p differ`) before any run.

## Deploying (2026-11-11)

Production (https://treachery.exe.xyz/) runs the Rust server, and `just deploy` ships the working tree, including other agents' unfinished edits. Agents never deploy unless their brief says so, and then only after `rust/tools/check.sh` passes on the whole workspace.
