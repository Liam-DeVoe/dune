# The observable-behavior harness

`rust/observe` checks that the working tree behaves like a frozen reference build of the engine, as far as players can tell. It is the safety net for making the Rust code native ([native-plan.md](native-plan.md)). It replaces state-dump comparison, because native code is free to change the shape of the game state.

The reference binary records bot and fuzz games at `Game::LATEST_VERSION`. Each trace lists every call the driver made on the game and what players could observe of it. The working tree makes the same calls and must observe the same things. It compares:

- whether each submitted event was accepted, and the text of its rejection message;
- the log: every new message with its target faction, rendered with the default skin;
- the view of every player, of a seatless host and of an observer (the `rust/view` `GameView` JSON, actions and their options included), after every call that changed the history;
- the event types each queried player may submit (the bots' and fuzzer's `GetApplicableEvents` calls);
- each bot round's decision: which player's bot acted, and the event JSON it built.

It doesn't compare state shape, random draw counts, bot priority levels or internal ids that players never see. An event that makes the engine throw counts as "not accepted", and the text of its "Game Error" message isn't compared, so turning a C#-style exception into a `Result` or a proper validation message is not a difference. Card, leader, location and territory ids are compared where they appear in views and event JSON, because clients see and send them.

## Running it

From `rust/`:

```sh
cargo build --release -p observe

# 200 games each, through heavy.sh like any batch run
tools/heavy.sh ./target/release/observe live --mode bots --seed 1 --games 200 --jobs 4
tools/heavy.sh ./target/release/observe live --mode fuzz --seed 1 --games 200 --jobs 4

# One game in detail: recorded again with full views, so a view difference shows its fields
./target/release/observe first --mode fuzz --seed 1 --game 17

# Trace files (from --keep-failures, or `observe record`)
./target/release/observe replay /tmp/failures/fuzz-1-00017.jsonl
```

`live` takes a heavy slot itself, so running it outside `heavy.sh` also waits for one, but only `heavy.sh` enforces the 3 GB memory cap. Use `--jobs 2` while trying something new and `--jobs 4` at most. Each job records a game with the reference binary into memory, then replays it, so a job uses one core at a time. Nothing goes to disk unless you pass `--keep-failures DIR`. Add `-v` for one line per game.

A failing game is recorded again with full views, so the report names the viewer and lists the view fields that differ:

```
fuzz-1-00017: first difference at trace line 304, event Bid (bot)
  what:      views
  viewer:    player 1 (Choam) (view 1)
  reference: 72a8902d27c1f8bc
  working:   0b5e...
  view fields that differ (path: reference -> working tree):
    /status/message/0/v: "Choam is bidding" -> "Choam bids"
```

`live` ends with the `first` command for the first failing game. Other options: `--hang-seconds` (default 60; a call that runs longer counts as hung, and if the reference hung at the same call, that's a match), `--memory-mb` (default 2048 per game), and `--reference BIN`.

See "Timing" below for how long 200 games take.

## The reference binary

`rust/reference/observe` (not in git) is the `observe` binary built from a frozen copy of the engine, bots and view crates. A copy sits in `~/treachery-native-ref/observe`, which `observe` uses when `rust/reference/observe` doesn't exist, as in a git worktree. `$OBSERVE_REFERENCE` or `--reference BIN` override both. `observe live` and `first` run it as `observe record --mode M --seed S --game I` and read the trace from its stdout.

The current reference is built from `~/treachery-native-ref/rust`. That's a copy of `rust/` taken on 2026-11-11, before step 2 (dropping old versions) started editing the engine, with `rust/observe` copied in. It was last rebuilt on 2026-11-12 with rustc 1.100.0, after the toolchain was reinstalled; `rust/reference/SOURCE` records the compiler. If `~/treachery-native-ref` is lost, unpack `rust/reference/source.tar.gz` into `~/treachery-native-ref` (it holds `rust/<crate>/Cargo.toml` and `src` for every crate the binary needs). To build the binary again:

```sh
rust/tools/build-native-ref.sh --source ~/treachery-native-ref/rust
```

The script builds in a temporary target dir (about 1.5 GB, deleted afterwards) and writes three files to `rust/reference/`: the binary, `source.tar.gz` with the exact sources, and `SOURCE`, which says where they came from. `--archive rust/reference/source.tar.gz` rebuilds from that archive. `--rev REV` builds from a git revision once the harness is committed. With no argument it builds from the working tree.

Rebuild the reference only on purpose. A reference built from a working tree with an unreviewed change makes that change invisible.

### Changing the harness itself

The trace format is defined by `observe/src/trace.rs`, and both sides must agree on it. When native refactors change engine APIs, update the working tree's `rust/observe` (mostly `session.rs`) so it compiles and makes the same calls. Don't change what it records. If the recorder itself has to change (a new observable, a fuzzer fix), change it in both places: copy the new `rust/observe/src` into `~/treachery-native-ref/rust/observe/src` and rebuild the reference with `--source`.

## Deliberate behavior changes

Step 4 of the plan changes behavior on purpose, for example a new random number generator. Most such changes alter the course of every game, so the old reference can't be compared any more. The workflow is:

1. Make the change in the working tree. Run `observe live` for a few dozen games and read the differences with `observe first`. Check that each one is the intended change and nothing else.
2. For changes that don't alter the course of a game, such as rewording a message or adding a view field, an allow file lets the run pass while the rest is still compared. Pass it with `--allow FILE` to `live`, `first` and `replay`:

   ```text
   # comment
   log <pattern>       leave log messages matching the pattern out, on both sides
   message <pattern>   rejection messages matching it (on either side) compare equal
   view <path>         don't compare this view path or anything below it
   bots                don't compare bot decisions
   ```

   Patterns match the whole text with `*` as a wildcard. To accept a reworded message, list the old and the new text. View paths look like `/status/message` or `/actions/*/validation`, where `*` matches one segment. View rules apply once the game is recorded again with full views, which `live` does by itself for every failing game.
3. Once the change is accepted, freeze the new behavior: update the frozen copy (`~/treachery-native-ref/rust`, or a git tag) to include it, rebuild the reference, and run the full 200 + 200 games to confirm a clean pass. Keep the old `source.tar.gz` if you might need to go back. An allow file is only for the review, so drop it after the rebuild.

## Trace format

One game is one JSON Lines stream (`observe/src/trace.rs`). Every line has an `op`:

- `header`: format version, mode, seed, game index, game version, seated humans, and the configuration (for information only, since the engine sees everything through events).
- `query`: `GetApplicableEvents(p, host)` and the type names it returned, or that it threw.
- `bots`: one round of asking the bots, as the server's `perform_bot_event` does it: levels Highest to Low (and End when bots act as host), players in `order`, first event wins. It lists the bots created in this round with their seeds, and the choice.
- `event`: an event submitted with validation, with its source (`host`, `bot`, `human`, `fuzz`, `mutate`), its JSON, the result, the rendered message, and the observations.
- `undo`, `restore`, `dismiss`: `Undo(until)`, `RestoreRecentlyUndone`, `DismissRecentlyUndone`, with observations.
- `end`: why the game stopped (`gameEnded`, `cap`, `stalled`, `botException`, `panic`, `setupFailed`, `recorderPanic`, `hung`, `memoryLimit`). `recorderPanic` means the recorder's own reads of the game threw, for example a fuzzed state with a null card in a hand; the replay simply stops there. For `hung` and `memoryLimit`, `pending` holds the call that never returned.

Observations are `log` (new messages as `[target, text]`), `logCut` (the log shrank, after undo), `views` (16-hex-digit SHA-256 prefixes of each view's JSON: players in order, then the host, then the observer) and, with `--full-views`, `full` (the views themselves). Views are built, as on the server, from a separate replica that executes accepted events without validation, so building them never changes the game.

## How games are made

`observe/src/record.rs` follows the C# oracle's `GameRunner.Play`, at the latest version only. The configuration (`config.rs`) draws a ruleset, rule toggles, factions, player count, turns, a host style and sometimes seated humans. The host establishes the game. Then each round queries every player, asks the bots, and submits the first bot event. If no bot acts, a seated human passes in the alliance phase, or the host acts (faction selection, decks, custom setup, HMS placement, `EndPhase`).

Fuzz games (`fuzz.rs`) add, per round: random events of a type applicable to a random player (12%), changed copies of the bot's event (10%), events of types rarely executed in the current phase (30% of bot rounds), and undo, restore and dismiss. Events start from the draft the client's form would send (`treachery_view::actions::default_draft`). Then up to three fields get real values of the game: the player's cards, leaders and force locations, other locations and territories, factions in play, and small amounts. Candidates that pass validation are preferred, but invalid ones are sent now and then and must be rejected the same way. The generator reads a separate shadow copy of the game, so it can't change state that the replay wouldn't change. It is coarser than the C# oracle's steered fuzzer. It doesn't build battle-specific actions such as Voice or Prescience on purpose, and it doesn't vary enum-valued fields.

Randomness comes from SplitMix64 seeded by mode, seed and game index (`rng.rs`). The engine's seed is in the `EstablishPlayers` event and each bot's seed is in the `bots` line, so a game depends only on its mode, seed and index.

## Timing

Measured on 2026-11-12, `--jobs 4` through `heavy.sh`, with another agent's runs on the machine (load average 7 to 25):

| Run | Wall time | CPU time |
|---|---|---|
| 200 bots games (seed 1) | 11 min | 37 min |
| 200 fuzz games (seed 1) | 15 min | 47 min |

The C# oracle took about 23 minutes per 200 fuzz games. The harness is faster per game, but its games are longer: every game is at the latest version, and fuzz games run up to 3000 events (the recorder's cap; before the cap, 200 fuzz games took 28 minutes). Each game is computed twice, once by the reference and once by the working tree, and most of the time goes into building 3 to 13 views after every accepted event. For a quick check while working, run 30 to 50 games.

## Tests

`cargo test --release -p observe` (part of `check.sh`, about 5 s) checks that clean recordings replay without a difference, that recording is deterministic, and that a trace with any observable changed fails: a log message, a view hash, a rejection message, an event's validity, a bot decision, an applicable-events list, or a cut-off trace. It also checks that full views name the differing field, and that allow rules accept what they cover.
