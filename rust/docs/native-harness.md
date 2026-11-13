# The observable-behavior harness

`rust/observe` checks that the working tree behaves like a frozen reference build of the engine, as far as players can tell. It is the safety net for making the Rust code native ([native-plan.md](native-plan.md)). It replaces state-dump comparison, because native code is free to change the shape of the game state.

The reference binary records bot and fuzz games at `Game::LATEST_VERSION`. Each trace lists every call the driver made on the game and what players could observe of it. The working tree makes the same calls and must observe the same things. It compares:

- whether each submitted event was accepted, and the text of its rejection message;
- the log: every new message with its target faction, rendered with the default skin;
- the view of every player, of a seatless host and of an observer (the `rust/view` `GameView` JSON, actions and their options included), after every call that changed the history;
- the event types each queried player may submit (the bots' and fuzzer's `GetApplicableEvents` calls);
- each bot round's decision: which player's bot acted, and the event JSON it built.

It doesn't compare state shape, random draw counts, bot priority levels or internal ids that players never see. An event that makes the engine throw counts as "not accepted", and the text of its "Game Error" message isn't compared, so turning a C#-style exception into a `Result` or a proper validation message is not a difference.

Since step 3b the engine no longer catches its own panics. The working tree's replay (`session.rs`, `guarded`) treats a call that panics as the server does: the call failed, with the panic message as its error, and the game goes on. So a former C# exception that is now a panic compares like the exception did. A view that fails to build is compared as failed: the reference records `error: <exception>` as the view's hash, and the error text isn't compared. Card, leader, location and territory ids are compared where they appear in views and event JSON, because clients see and send them.

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

# What 200 fuzz games exercise, recorded by the reference (see "Reach" below)
tools/heavy.sh ./target/release/observe reach --mode fuzz --seed 1 --games 200 --jobs 4 --recorder reference/observe
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

`live` ends with the `first` command for the first failing game. Other options: `--hang-seconds` (default 60; a call that runs longer counts as hung), `--memory-mb` (default 640 per game), and `--reference BIN`. A game whose call hangs or goes over the memory budget fails, as `runaway` when the reference ran away at the same call too, since no real game may end like that (on the server it freezes every game). The memory default was 2048 until fuzz seed 1 games 2 and 176 reached states where a `Bid` pass made the engine allocate without end (fixed since, escapes.md "Automatic bid passes that never stop"). With 4 jobs in one `live` process, one such game at 2 GB plus the other three went over heavy.sh's 3 GB cap and the whole run was killed. Normal fuzz games stay well under 640 MB. `first` is the exception: its recording keeps every full view, and a long game goes over the budget in the reference (fuzz seed 1 game 2 did at trace line 17,000 even with `--memory-mb 3000`). Check such a game with `live --start I --games 1` instead.

See "Timing" below for how long 200 games take.

## The reference binary

`rust/reference/observe` (not in git) is the `observe` binary built from a frozen copy of the engine, bots and view crates. A copy sits in `~/treachery-native-ref/observe`, which `observe` uses when `rust/reference/observe` doesn't exist, as in a git worktree. `$OBSERVE_REFERENCE` or `--reference BIN` override both. `observe live` and `first` run it as `observe record --mode M --seed S --game I` and read the trace from its stdout.

The current reference is built from `~/treachery-native-ref/rust`. That's a copy of `rust/` taken on 2026-11-11, before step 2 (dropping old versions) started editing the engine, with `rust/observe` copied in. It was last rebuilt on 2026-11-12 with rustc 1.100.0, after the steered fuzz generator was copied in (a rebuild with the runaway and card trade fixes waits for promotion, see "Changes to the recorder so far" below); `rust/reference/SOURCE` records the compiler. If `~/treachery-native-ref` is lost, unpack `rust/reference/source.tar.gz` into `~/treachery-native-ref` (it holds `rust/<crate>/Cargo.toml` and `src` for every crate the binary needs). To build the binary again:

```sh
rust/tools/build-native-ref.sh --source ~/treachery-native-ref/rust
```

The script builds in a temporary target dir (about 1.5 GB, deleted afterwards) and writes three files to `rust/reference/`: the binary, `source.tar.gz` with the exact sources, and `SOURCE`, which says where they came from. `--archive rust/reference/source.tar.gz` rebuilds from that archive. `--rev REV` builds from a git revision once the harness is committed. With no argument it builds from the working tree.

Rebuild the reference only on purpose. A reference built from a working tree with an unreviewed change makes that change invisible.

### Changing the harness itself

The trace format is defined by `observe/src/trace.rs`, and both sides must agree on it. When native refactors change engine APIs, update the working tree's `rust/observe` (mostly `session.rs`) so it compiles and makes the same calls. Don't change what it records. If the recorder itself has to change (a new observable, a fuzzer fix), change it in both places: copy the new `rust/observe/src` into `~/treachery-native-ref/rust/observe/src` and rebuild the reference with `--source`.

The fuzz generator reads the shadow game through the view (`actions::action_view`, `actions::default_draft`, `side::deals`) and a few engine fields (`players`, `forces_in_locations`, `current_main_phase`, `current_battle`). A refactor that renames these must update `fuzz.rs` in the working tree too, without changing what it generates.

Changes to the recorder so far:

- 2026-11-12: steered fuzz generator and `observe reach` (see "How games are made" and "Reach"). The reference before this change is kept as `~/treachery-native-ref/observe-before-fuzz-reach` with its sources in `source-before-fuzz-reach.tar.gz`.

Deliberate behavior changes frozen into the reference (the frozen engine edited the same way as the working tree, then rebuilt):

- 2026-11-13: automatic bid passes stop with the bidding phase and happen at most once per player per bid, and `CardTraded` must name the ally. See escapes.md, "Automatic bid passes that never stop" and "A card trade offered to a faction that isn't the ally". The frozen files changed are `engine/src/events/bid.rs` and `engine/src/events/card_traded.rs`. Until the branch is merged and the reference promoted, the changed frozen copy is `~/treachery-native-ref/rust-runaway-fixes` and its binary `~/treachery-native-ref/observe-runaway-fixes` (pass it with `--reference`). The reference before this change is kept as `~/treachery-native-ref/observe-before-runaway-fixes`, with its sources in `source-before-runaway-fixes.tar.gz`.

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

   View rules don't scale to a change that shows up in every game. Recording a whole long game with full views takes more than the default 640 MB per game (2 GB wasn't enough for seed 1 bots game 3), so the reference ends with `memoryLimit` and the game fails. For a removed or renamed view field, build a temporary `observe` from the working tree with the old field put back, and run `live` with no allow file instead. The view cleanup of step 3b did that for `Simple.dismissLabel`, which every game's `EndPhase` form has.
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

Fuzz games (`fuzz.rs`) follow the C# oracle's `client` fuzz profile (`Oracle/Fuzzer.cs`, `FuzzerSteering.cs`). Per round they add:

- a random event (12%). Usually it is steered: every player (and nobody) is queried, and a (player, type) pair is drawn with weight 1 / (1 + times accepted in this phase)^1.5, as in C#. The weight is also lowered for types picked often in this phase and for types accepted often in the whole game, so always-applicable types (deals, donations, permissions) don't take every step. Otherwise it is a random player's applicable type, or any type now and then.
- during a battle, a battle action for a player (25% of bot rounds): an applicable `Voice`, `Prescience`, `JuicePlayed`, `BattleRevision`, `TreacheryCalled`, `FaceDanced` and the like, weighted towards rare types. Now and then `KarmaEcazDial`, `JuicePlayed`, `ResidualPlayed` or `AuditCancelled` even when not applicable. `JuicePlayed` usually gets `Aggressor` when the form offers it.
- a battle plan submitted again with its leader, a card or its dial changed (30% of bot rounds, when a faction that already planned this battle may plan again, mostly after `BattleRevision`). Parts the opponent saw with prescience must stay, so the engine rejects some of these.
- an event of a type rarely executed in this phase (30% of bot rounds), changed copies of the bot's event (10%), and undo, restore and dismiss.
- when the same bot sends the same accepted event again in the same phase (bots resend `AllyPermission` when their permissions can't settle), the fuzzer acts instead 80% of the time from the third repeat on, so the loop doesn't use up the game's events.

Values come from what the client's form offers. For an event type and a player, `Pools::of` reads the view's action options (`treachery_view::actions::action_view`). These hold the draft the form sends untouched (the default draft after the form adjusted it), the choice lists (heroes, weapons, targets, card types, enum members such as Voice types or Prescience aspects) and the maximums of number pickers. Each event field gets a choice from the list named like it (`heroes` for `_heroId`, `weapons` for `_weaponCardId`), or from a list holding its current value (`choices` for `JuicePlayed.Type`). Multi-select fields (`_cardIds`, arrays) get a subset of their list. Amounts get a value up to the matching maximum. Battle plans take a force mix from the form's `mixes`. Force placements use the player's own battalions or a few forces at locations the form offers. Deal answers are the accept, decline and withdraw events of the deals panel (`treachery_view::side::deals`). Fields the form offers no choice for keep the draft's value, or get the player's own cards, leaders and force locations or a small amount. Each event tries up to 8 candidates and sends a valid one 90% of the time. Invalid candidates are still sent now and then and must be rejected the same way. The generator reads a separate shadow copy of the game, so it can't change state that the replay wouldn't change.

Randomness comes from SplitMix64 seeded by mode, seed and game index (`rng.rs`). The engine's seed is in the `EstablishPlayers` event and each bot's seed is in the `bots` line, so a game depends only on its mode, seed and index.

## Timing

Measured on 2026-11-12, `--jobs 4` through `heavy.sh`, with another agent's runs on the machine (load average 7 to 25):

| Run | Wall time | CPU time |
|---|---|---|
| 200 bots games (seed 1) | 11 min | 37 min |
| 200 fuzz games (seed 1) | 15 min | 47 min |
| 200 fuzz games (seed 1), steered generator, `--memory-mb 640` | 15 min | 51 min |

The C# oracle took about 23 minutes per 200 fuzz games. The harness is faster per game, but its games are longer: every game is at the latest version, and fuzz games run up to 3000 events (the recorder's cap; before the cap, 200 fuzz games took 28 minutes). Each game is computed twice, once by the reference and once by the working tree, and most of the time goes into building 3 to 13 views after every accepted event. For a quick check while working, run 30 to 50 games.

## Reach

`observe reach` records games and counts what they exercise: event types accepted, (type, phase) pairs accepted, phases reached, and types offered to a player but never accepted. It runs `RECORDER record --reach` per game, which prints one line per submitted event and per offered type instead of the trace. Pass `--recorder reference/observe` to measure the generator the harness actually uses.

Fuzz reach over 200 games (seed 1) on 2026-11-12, against the C# oracle's `fuzz --seed 1 --games 200 --versions latest` (client profile). The C# traces have no phase per step, so only event types compare across both:

| | C# oracle | Rust before | Rust now |
|---|---|---|---|
| events submitted | 594,186 | 339,492 | 303,003 |
| event types accepted | 124 | 129 | 130 |
| (type, phase) pairs accepted | | 759 | 914 |
| phases reached | | 103 | 107 |

"Before" is the generator up to this change (form drafts with up to three fields set to game values). Compared with it, the steered generator executes `BattleRevision` 3.4 times as often (in 145 games instead of 81), `Caravan`, `AllianceBroken`, `CardTraded` and `FlightUsed` in about three times as many games, and `DealRejected`, which never passed validation before, in 194 games. The C# fuzzer still executes `BattleRevision` more often (4,716 times), mostly because its games are longer.

What is left:

- C# executed `ChoamRemoveForce` once in 200 games; the Rust generator offered it in 11 games but never got it accepted. `Diplomacy` and `KarmaMonster` were offered (in 2 and 1 games) and never accepted.
- Deal events take about 7% of all events (20,555 `DealAccepted`/`DealRejected`, mostly fuzz), against 2,072 in C#. The fuzzer offers a deal and then answers it.
- Rust executes 7 types C# didn't (`EmperorBidSupport`, `Discarded`, `KarmaFreeRevival`, `KarmaHmsMovement`, `KarmaRevivalPrevention`, `FlightDiscoveryUsed`, `TestingStationUsed`).
- Fuzz games stop at 3000 events, and 70 of 200 hit that cap. Two ended in a bot exception (`NullReferenceException`; games 31 and 58), and two ran away inside a `Bid` (games 2 and 176). All four were engine bugs, fixed on 2026-11-13 (escapes.md, "Automatic bid passes that never stop" and "A card trade offered to a faction that isn't the ally"). Those games now run on and take longer.

## Tests

`cargo test --release -p observe` (part of `check.sh`, about 5 s) checks that clean recordings replay without a difference, that recording is deterministic, and that a trace with any observable changed fails: a log message, a view hash, a rejection message, an event's validity, a bot decision, an applicable-events list, or a cut-off trace. It also checks that full views name the differing field, and that allow rules accept what they cover.
