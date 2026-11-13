# Making the Rust code native

The port is finished and matches C#. C# development has stopped, and production moves to Rust. From now on the Rust code is the product, not a port. This plan turns code written to match C# byte for byte into code written as if Rust had been the original.

Decisions (2026-11-11, from the user):

- Production moves to Rust. The root path serves the Rust server and React client.
- Games in progress on production don't need to keep working. Accounts do.
- C# development stops. The C# tree stays in git history and is deleted from the working tree at the end.

PORTING.md's rule "C# is the spec" no longer applies to new work. Its process rules (heavy.sh, memory, disk, release builds, keeping the shared build green, context budget, escapes.md) still do.

## The safety net

The C# oracle compares full state dumps, which are shaped like C# objects. A native rewrite changes the state's shape, so state dumps can't be the reference any more. Instead, a frozen build of today's Rust engine is the reference, and runs compare what can be observed from outside the engine:

- whether each event is valid, and the error message if not,
- the game log,
- each player's and observer's view (the `rust/view` GameView JSON), including the actions on offer,
- the bots' decisions.

This harness has to exist before any cleanup starts (step 1).

## Steps

0. **Move production to Rust.** Back up the production database. Serve Rust and the React client at the root, copy the art and CSS the Rust site takes from the C# app, stop the C# service, redirect `/beta/` to the root. Old games that fail to load are closed cleanly, not a crash.
1. **Observable-behavior harness.** A reference binary built from today's engine records traces from bot and fuzz games at the latest game version: events plus the observables above. The working tree's engine replays them and must match. It plugs into `differ live` and `heavy.sh` the same way the C# oracle does.
2. **Drop old game versions.** Only `Game.LatestVersion` games are created from now on, so every version check becomes its latest-version branch. This keeps behavior the same for new games and removes about 260 checks before the larger cleanup.
3. **Cleanup that keeps behavior the same,** split by area across agents, each change checked by the harness:
   - module layout by concern instead of one file per C# file, no generated skeleton comments, no blanket `#![allow(...)]`;
   - `Result` instead of panics that imitate C# exceptions;
   - `Option` instead of null handles;
   - plain state snapshots for undo instead of object identity across games (`GameRef`, `other_games`);
   - ordinary iterators instead of imitations of C#'s lazy queries; `LiveList` removed;
   - std or `indexmap` collections where iteration order can't be observed;
   - a server designed for itself rather than imitating the C# hub and EF schema (keep the accounts data).
4. **Changes to behavior,** one at a time, each reviewed and re-recorded in the harness on purpose:
   - a Rust random number generator instead of .NET `Random`;
   - bots that never change the game while deciding;
   - the C# bugs we copied on purpose (`// C# parity:` comments): the ones the user chose are in behavior-changes.md.
5. **Delete the C# tree,** the oracle, `dotnet-compat` if nothing still needs it, and the C# sync check. Rewrite PORTING.md for the native code base. First move what the site still takes from the C# tree into `web/`: the art, fonts, scripts and PDFs in `treachery-online/Client/wwwroot` and the SCSS in `treachery-online/Client/Styles`. Then point `deploy/env.sh` (`ASSETS_SRC`, `STYLES_SRC`) and the server's default `TREACHERY_ASSETS_ROOT` (`rust/server/src/settings.rs`) at the new place.

## Status

- Step 0: done 2026-11-11. https://treachery.exe.xyz/ runs the Rust server and React client on a copy of the C# production database (deployed with `just deploy`, see deploy/README.md). Six old games failed to load (event classes renamed since they were saved) and were dropped; backups of the C# database are in `~/backups` on the VM.
- Step 1: done 2026-11-12. `rust/observe` records games with a frozen reference build and replays them through the working tree ([native-harness.md](native-harness.md)). 200 bots and 200 fuzz games pass against the working tree after step 2, which confirms step 2 kept latest-version behavior. Reference binary: `rust/reference/observe`, copy and frozen source in `~/treachery-native-ref`.
- Step 2: done 2026-11-12. All 266 version checks in engine, bots, view and server now keep only their version-188 branch (engine/src: about 1,180 lines removed, 420 added). `Game::new_int32_participation` throws for any version other than `LATEST_VERSION`, so older savegames fail to load with "Game version N is not supported". The oracle has `--versions latest`, `differ live` always passes it, and the differ refuses traces of other versions. Confirmed with `--bots check`: 200/200 fuzz games (seed 9301; game 155 hung in C# and matched up to the hang) and 200/200 bot games (seed 9302). Tests were moved to the latest version: C# goldens were regenerated with csharp-ref, ViewRef, SkinRef and the oracle, the old-version traces in `traces/sample` and `view/tests/data` were re-recorded at 188, and tests that only covered old versions were deleted, including the seed 301 undo regression trace. `HMSAdvantageChosen` form coverage now comes from a built state (`hms_advantage_form_in_a_built_state` in view/tests/battle_forms.rs), because no latest-version game reached it. `check.sh` is green.
- Step 3 (server): done 2026-11-11. rust/server was rewritten as a Rust design (game thread, `Server` with handlers by area, its own SQLite schema with a migration that converts the C# database on startup, settings as `TREACHERY_*` environment variables). rust/server-equiv, the C# test hooks' Rust side and the .NET probes are gone. The WebSocket protocol is unchanged apart from a few fixes listed in [server.md](server.md#behaviour-that-differs-from-the-c-server). Tests drive the server through the protocol (`rust/server/tests`); `scripts/ui/e2e_server.py` passes. Not deployed: the next `just deploy` converts the production database and installs the new `deploy/treachery.service`.
- Step 3a (module layout): done 2026-11-12 on its own branch, waiting to be merged. Engine and bots modules are organized by concern ([engine.md](engine.md)), the skeleton headers, C# signature doc comments and blanket `#![allow]`s are gone, and the workspace builds without warnings. `csharp-gen --glue` is retired, because its output no longer matches the layout.
- Step 3 (undo and iterators): done 2026-11-12 on its own branch, waiting to be merged. Undo keeps the replaced game as a snapshot in `previous_game_state` ([engine-conventions.md](engine-conventions.md#undo)). `GameRef`, `GameId`, `other_games` and the map tags on locations and territories are gone, and `PlayerIndex` is a plain index. `compat/lazy.rs` is gone too. `LiveList` stays, because the log in the player views shows its current contents, as the C# client did. The bots still use the `cs_*_or_default_where` helpers. Harness: 200/200 bots (seed 1). Fuzz (seed 1): 4 games differ after an undo that was restored, which is the intended change ([engine-conventions.md](engine-conventions.md#undo)). Another 24 end with "hang differs at end" (the reference reaches its memory limit at the last call, the working tree returns); the build before this branch (8e3340f) gives the same result on all 24.
- Step 3b, view and web (2026-11-12, own branch): the dead `PlayerReplaced` and `HideSecrets` forms, `Simple.dismissLabel`, `FaceDanced.placeOnly`/`mayCall` and the story's `secret` column are gone from the view, the generated types and `web/`. The C# comparison tooling is gone (`rust/view/csharp-ref`, `csharp-inventory`, client-inventory.md, `scripts/ui/compare.py` and its comparisons and helpers); the log line goldens are now Rust snapshots with an update mode (`tests/log_lines.rs`). client-plan.md became [client.md](client.md), and protocol.md's Game view section describes the view as it is. View comments no longer point at C# files.
- Step 3c (null handles, .NET collections and helpers), 2026-11-13, own branch: handles have no null value (`Option`, `required`, `expect`), `compat/cs.rs` is gone (`Shared`/`RcKey` in `shared.rs`, enums in `enum_type.rs`), `wrapping_*` arithmetic and the LINQ/indexer helpers are std code, and maps and sets are `IndexMap`/`IndexSet` except 13 game fields whose .NET slot order players see ([engine-conventions.md](engine-conventions.md#collections)). dotnet-compat lost `List.Sort`, LINQ and the null-key check. The state dump stays, because engine tests that compare with C# goldens use it. Tests of the imitation itself (null handles, LINQ table, boundary harness, review_pkg_j) are deleted.
- Step 4 prep: a review page of the copied C# bugs is being written for the user.
- Step 5, assets: done 2026-11-13 on its own branch, not deployed. The site's art, fonts, scripts, PDFs and Bootstrap moved from `treachery-online/Client/wwwroot` to `web/assets`, and the SCSS from `treachery-online/Client/Styles` to `web/styles` (with `git mv`). `npm run build:css` compiles it with Dart Sass 1.103.0, pinned in `web/package.json`, and `npm run build` runs it first. `deploy/env.sh`, the server's default `TREACHERY_ASSETS_ROOT` and the Vite dev middleware point at `web/assets`. `web/public/skin/default.json` is now edited by hand, and its exporter `web/tools/export-skin.cs` is gone. The 73 art files nothing links (`person*`, the leader tokens from before `scripts/leader-tokens.py` redrew them as `leader*.png`, and a few unused sounds and images), `wwwroot/index.html` and the C# client's own scripts (`scene.js`, `listbox.js`, `follow-log.js`, `turn.js`) stay in the C# tree and go with it. The Blazor client no longer builds its CSS or shows its art.
- Step 5, C# tree: done 2026-11-13 on its own branch. `treachery-online/`, rust/differ, the csharp-ref and csharp-gen programs, the C# sync and Roslyn coverage tests, the Blazor screenshot tooling in `scripts/` and the port documents (hazards.md, differ.md, trace-format.md, coverage.md and the rest) are gone. Tests that compared with C#-written values are Rust snapshot tests now (`UPDATE_GOLDENS=1` rewrites them, [engine-conventions.md](engine-conventions.md#snapshot-tests)). The canonical state dump went with them (`canon.rs`, canon-derive). The sample games the view and skin tests replay are in `rust/testdata/games`, without the C# state dumps. dotnet-compat stays: the RNG, the 13 slot-ordered fields, float formatting and parsing still use it. PORTING.md isn't rewritten yet.

## Paused 2026-11-11 14:35 (all agents stopped by the user)

- Step 1: finished after the pause (see its status line).
- Step 2 (done since, see its status line above): all version checks removed; its 200-game fuzz confirmation (seed 9301) was cut off, and 4 view/skin tests that replay old-version corpora still fail (skin trace_corpus_matches_csharp; view battle_forms_in_rare_traces, bot_game_log_lines_match_csharp, options_equal_the_engine_helpers_over_the_corpus).
- Server rewrite done, not deployed. Before `just deploy`: back up the production DB (it is converted on first start) and get check.sh green.
- Then: engine/bots/view cleanup (step 3), behavior-changes.md fixes (step 4), step 5.

## How step 3 runs (2026-11-12)

Each step 3 agent works in its own git worktree and branch with its own target dir (PORTING.md, "Disk space"); the coordinator merges. To keep merges small, changes that touch every file run first and alone: the module layout (files by concern, no generated skeleton comments, no blanket `#![allow]`). After that merges, the rest splits by concern across agents: Result instead of panics, Option instead of null handles, snapshot undo, iterators and LiveList, collections, bots, view. Every branch must pass the native harness (docs/native-harness.md) and check.sh before merging.
- Step 5, PORTING.md: replaced by DEVELOPMENT.md (2026-11-13), which describes the code base as it is.
