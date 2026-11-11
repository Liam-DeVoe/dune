# The differ

`rust/differ` replays traces from the C# oracle through the Rust engine and reports the first place where the two engines disagree. A trace is a recorded game: every event the oracle submitted, plus what C# did with it (see [trace-format.md](trace-format.md)). The differ makes the same calls in the same order and compares, line by line:

- the result (`accepted`, `rejected` or `error`), the returned validation message and any exception;
- the event as serialized after it ran (`eventAfter`);
- `History.Count` and the number of random draws;
- the event types every player may submit next (`applicable`);
- the SHA-256 of the canonical state dump.

It stops a trace at the first difference.

All commands below run from `rust/`. Use a release build for anything bigger than a handful of traces.

## Running it

```sh
# One trace: a full report of its first difference
cargo run -p differ --release -- run traces/sample/fuzz-2026-00007.jsonl.gz

# A directory, in parallel: a summary grouped by reason
cargo run -p differ --release -- run traces/sample --jobs 8

# The first difference of one trace in detail
cargo run -p differ --release -- first traces/sample/bots-2026-00011.jsonl.gz --dump-dir /tmp/diff

# Generate and check games without keeping traces on disk
cargo run -p differ --release -- live --mode fuzz --seed 7 --games 500 --jobs 8
```

`live` needs the oracle built once:

```sh
export DOTNET_ROOT="$HOME/dotnet10" PATH="$HOME/dotnet10:$PATH"
dotnet build -c Release ../treachery-online/Oracle/Treachery.Oracle.csproj
```

`run` and `first` exit with 0 when every trace matches and 1 otherwise. Add `-v` to `run` or `live` for one line per trace.

`live` takes one of the machine-wide slots of `rust/tools/heavy.sh` itself and waits while both are taken (PORTING.md, "CPU"). Run inside `heavy.sh`, it uses that slot instead. Its `--jobs` defaults to at most 4.

`live --mode fuzz` records and compares `GameStatus` by default (see "Status queries" below). Add `--status none` to turn that off. Status traces are several times larger, but `live` keeps at most two batches of gzipped traces on disk and deletes each trace once it is checked.

`live` checks every batch, whatever the oracle's exit status. When a game hangs in C#, the oracle gives up on it after its `--hang-seconds` (120 s), writes a trace that ends in `hung`, `undoHung` or `botHung`, and exits with 4. The differ replays the finished games of that batch as usual, replays the hung game up to the hang, and lists it in the summary. A game the oracle wrote no trace for is reported as a bad trace. The run stops only if a batch produced no trace at all. When the oracle died for another reason (a crash, or exit status 5 because it went over its `--memory-mb`), the games it was still running have cut-off traces or none. `live` reruns each of those alone in a fresh oracle process and replays that trace instead.

Every replay has a memory budget, `--memory-mb` (default 2048, 0 for none), for `run`, `first` and `live`. The differ's allocator counts what each replay thread holds. A replay that would go over its budget is stopped inside the allocation and reported as `memory limit`. Its thread stays asleep, holding what it had, until the differ exits. An end line with reason `memoryLimit` (C# went over the oracle's limit on that event) counts as a match when Rust also goes over its budget or doesn't return.

The test suite runs the sample corpus too:

```sh
cargo test -p differ --test sample_corpus -- --nocapture   # prints the summary, passes unless a trace is unreadable
cargo test -p differ --test sample_corpus -- --ignored     # requires every trace to match
cargo test -p differ --test field_mapping                  # the dump has the same fields as C#
```

`sample_corpus_passes` is `#[ignore]` until the engine is ported. Delete the attribute to make it required.

## Checking bot decisions

`--bots check` (on `run`, `first` and `live`) also checks the bots, in traces of trace format 2 recorded in `bots` mode or in `fuzz` mode with the `client` profile (the header's `fuzz.profile`, the oracle's default). Fuzz traces with the `wide` profile submit forged values, which are outside PORTING.md's scope, so their `bots` entries are skipped. Without the flag, `bots` entries are always skipped, so engine replays don't stop at unported bot code.

```sh
cargo run -p differ --release -- live --mode bots --seed 7 --games 200 --jobs 4 --bots check
cargo run -p differ --release -- live --mode fuzz --seed 7 --games 200 --jobs 4 --bots check
```

In a fuzz game the oracle's bots deliberate before most steps, but the fuzzer often submits something else: a mutated copy of the bot's event, a random event, a battle action, or a host event. The `bots` entry records the deliberation either way, so the Rust bots run at the same points and make the same changes to their own state and to the game. Use `--bots check` for fuzz runs: a C# bot that changes the game while deliberating (the Rock Melter case in [known-issues.md](known-issues.md)) shows up as a false `hash` difference without it.

At each `bots` query entry (trace-format.md section 3, "Bot deliberations") the differ:

1. creates the bots the entry lists in `created`, with the recorded seeds, and keeps them for the rest of the trace (one per faction, in creation order, across undo, as the oracle does);
2. runs the oracle's loop on the live replay game: levels `Highest` to `Low`, then `End` if `end` is true, players in `order`, `set_game_and_player` before each ask, and stops at the first event. Each bot gets the result of its player's `GetApplicableEvents` query just before the entry. The game isn't cloned: bots can change it, and Rust must make the same changes;
3. compares the outcome with the recorded one: same player, same level and the same event JSON, byte for byte (`json::serialize` against `choice.event`), or the same exception, or no event on both sides. A difference is key `bot`.

On every line it also compares `botRandom`, the draw count of each bot, with the Rust bots' counts (key `botRandom`). This often finds a divergence before any decision differs.

The replay then executes the event recorded in the trace, never the Rust bot's event. Like any other difference, a bot difference stops the trace.

The summary groups bot failures by what C# decided, which names the `Determine*` method:

```
bot deliberations: 5120 matched; 23 traces stopped at a bot difference
       12  not ported yet: not yet implemented: port: Bot_Battle.cs:417 (at bots/src/bot_battle.rs:88)
        7  bot decision differs: C# Battle at Middle
        4  bot random draws differ: C# Shipment at Low
```

- **bot decision differs: C# `<Event>` at `<level>`**: C# chose that event type at that level and Rust chose something else. `C# no event, Rust <Event> at <level>` means C# chose nothing; `C# threw <type>` that C# deliberation threw. `first` prints both sides in full.
- **bot random draws differ: `<deliberation>`**: the decision matched, but some bot drew a different number of random numbers. The expected and actual lines list each bot's count.
- **not ported yet** with a location in `bots/src`: a `todo!` of an unported bot method, or another Rust panic in bot code.

"matched" counts deliberations whose outcome matched, over all traces. A bot package is done when `differ live --mode bots --bots check` over a few hundred games has no bot group of its files.

## Reading the summary

```
17 traces: 2 passed, 15 failed (1834 lines replayed without a difference)

9 traces: not ported yet: not yet implemented: port: Game_Battle.cs:1102 (at engine/src/game_battle.rs:412)
    bots-2026-00008.jsonl.gz at line 57
    ...

4 traces: hash differs after Shipment, first at Game.Players[].Resources
    fuzz-2026-00013.jsonl.gz at line 88
    expected: 3be1...
    actual:   91c0...
    Game.Players[2].Resources: C# 7 / Rust 5
```

If some games never finished in C# (they hung or went over the oracle's memory limit), a line under the totals says how many, and whether each one matched up to that point:

```
3 never finished in C# (hung, or over the memory limit): 2 of them match up to that point, 1 differ
    fuzz-41-00017.jsonl.gz (hung)
    ...
```

A hung game that matches counts as passed, and `-v` prints it as `HUNG` instead of `PASS`. For `hung` and `undoHung` it means Rust didn't return from the same call either. For `botHung` there is nothing to run, so it means every line before the hang matched.

Each group is one reason, largest first, with up to three example traces. "Line" is the `n` of the trace line, which is the step number. The reasons are:

- **not ported yet**: Rust panicked with something other than a C# exception, such as a `todo!`, an `unwrap` or an index out of bounds. The location is where it panicked. Port that code and the traces in the group move on to their next failure.
- **`<key>` differs after `<Event>`**: a recorded value differs after an event of that type. For `hash`, the summary shows the first state fields that differ, if the trace line has a full dump. Indices are dropped from the path in the group name, so the same bug in different players lands in one group.
- **termination differs**: either Rust did not finish a line within `--step-timeout` seconds (default 120) when C# did, or Rust returned from an event that C# never returned from (see below).
- **uncaught C# exception during ...**: a C# exception escaped from somewhere the oracle never saw one, such as `new Game` or a query between events.
- **bad trace**: the file could not be read, or it has no end line.
- **bot decision differs**, **bot random draws differ**: only with `--bots check`, see "Checking bot decisions".

## Reproducing one failing step

1. Run `first` on the trace. It replays up to the first difference and prints the trace line (without its state), every value that differs on that line, and up to 40 state fields that differ, as path, C# value and Rust value.
2. If the report says the line has a hash but no full dump, regenerate the game with a dump on every line. The file name gives the mode, seed and game index:

   ```sh
   dotnet ../treachery-online/Oracle/bin/Release/net10.0/Treachery.Oracle.dll fuzz \
       --seed 2026 --start 13 --games 1 --dump every --gzip true --out /tmp/repro
   cargo run -p differ --release -- first /tmp/repro/fuzz-2026-00013.jsonl.gz
   ```

   A game depends only on mode, seed and index, so this gives the same trace. `live` prints this command for the first failing game. It also reruns any game that failed on a hash without a dump, so its report already has the state fields. With `--keep-failures DIR` it keeps those regenerated traces.
3. `--dump-dir DIR` writes the two state dumps of the failing line as `csharp.json` and `rust.json`, for a full diff (`jq -S . csharp.json > a; jq -S . rust.json > b; diff a b`).
4. `--step N` stops after line N. Use it to check that the lines before a failure still match after a fix.
5. Set `DIFFER_PRINT_PANICS=1` to see Rust panic messages as they happen. The differ hides them by default, because C# exceptions are panics too and most of them are expected. Add `CS_EXCEPTION_BACKTRACE=1` to also print the Rust call stack of every C# exception. That shows which engine function threw when the location is a shared accessor such as `TreacheryCard::type_`.

The C# side of the same replay is `Treachery.Oracle verify --in FILE` (`Oracle/Verifier.cs`). It replays a trace with the C# engine from the trace alone. If the differ and a trace disagree in a way that looks like a harness bug, check that `verify` passes on the trace.

## What the replay does

For each line it follows [trace-format.md](trace-format.md) section 6 and `Oracle/StepExecutor.cs`:

- It makes the line's `queries` (`GetApplicableEvents` calls) first, because they can change state. A query entry that records an exception must throw exactly that exception in Rust, and one that doesn't must not throw; a difference is reported with key `query`. Older traces didn't record query exceptions. For those, a throw from the last query of a `botException` end line still matches when its text equals `detail`.
- It marks the map's neighbour cache stale before every event.
- It appends a marker milestone before `Execute` to tell validation exceptions from execution exceptions, as the oracle does. For `EstablishPlayers`, which overrides `Execute` and has no try/catch, it captures a backtrace instead and looks for `validate` or `execute_concrete_event` frames.
- It renders validation messages and exceptions with the canonical dump and compares the bytes.

**Hung games.** A trace whose end line has reason `hung` records an event that C# never returned from. The differ runs that event on the trace's thread, and if it is still running after `--hang-seconds` (default 10), the trace counts as a match. A thread that never returns keeps running until the differ exits, as in the oracle. An end line with reason `undoHung` is handled the same way: the differ makes its queries, then the `Undo`, `RestoreRecentlyUndone` or `DismissRecentlyUndone` call, and counts it as a match if that call is still running after `--hang-seconds`.

**Status queries.** `differ live --status every` passes `--status every` to the oracle. Such traces have `["status", player, isPlayer, result]` entries in `queries` (trace-format.md section 3). The differ makes each `GameStatus::determine_status` call in order with the other queries and compares the canonical dump of its result byte for byte; a difference is reported with key `status`. What the calls change in the game is covered by the line's `hash`.

## The state dump

The dump is the Rust side of `Oracle/Canon/StateDumper.cs`. It lives in `engine/src/canon.rs`, and every state struct derives it with `#[derive(Canon)]`. The attributes are described in [engine-conventions.md](engine-conventions.md#state-dump-names).

`tests/field_mapping.rs` checks it without any game logic:

- Field names: `tests/golden/canon_schema.json` lists the fields the C# dumper writes for every type, taken by reflection with the dumper's rules. The test compares it with the Rust derives and fails on any C# field with no Rust counterpart, any Rust field with no C# counterpart, and any C# type in the dump that has no Rust type. Regenerate the file after a C# change:

  ```sh
  cd engine/csharp-ref && dotnet run -c Release -- --canon-schema ../../differ/tests/golden/canon_schema.json
  ```

- Value kinds: every value in the sample traces' dumps must be something the Rust field can write. For example, a reference must stay a reference and a list must stay a list.
- Reference forms: for every field whose C# type the dumper writes as a reference (`Game`, `Player`, `Location`, `Territory`), the Rust field must be able to write every form the dumper can, including the ones only another game produces (`OtherGame`, a full `Player`, `"foreign":true`). This needs no trace.
- The map: a fresh Rust `Map` dumps to exactly the map in a sample trace.
- The header's `Participation` dumps back to the same bytes.

### Games after undo

After `Undo`, `RestoreRecentlyUndone` or `DismissRecentlyUndone`, some C# events and players still belong to the other game. C# writes that game as `{"$ref":"OtherGame"}`, such a player as a full `Player` object, and the locations inside that player with `"foreign":true`. Rust writes the same, from `Game.other_games` ([engine-conventions.md](engine-conventions.md#game-identity-and-undo)).
