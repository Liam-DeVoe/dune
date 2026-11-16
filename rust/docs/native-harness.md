# The observable-behavior harness

`rust/observe` checks that the working tree behaves like a frozen reference build of the engine, as far as players can tell. It is the safety net for making the Rust code native ([native-plan.md](native-plan.md)). It compares only what players can observe, because native code is free to change the shape of the game state.

The reference binary records bot and fuzz games at `Game::LATEST_VERSION`. Each trace lists every call the driver made on the game and what players could observe of it. The working tree makes the same calls and must observe the same things. It compares:

- whether each submitted event was accepted, and the text of its rejection message;
- the log: every new message with its target faction, rendered with the default skin;
- the view of every player, of a seatless host and of an observer (the `rust/view` `GameView` JSON, actions and their options included), after every call that changed the history;
- the event types each queried player may submit (the bots' and fuzzer's `GetApplicableEvents` calls);
- each bot round's decision: which player's bot acted, and the event it built.

It doesn't compare state shape, random draw counts, bot priority levels or internal ids that players never see. An event that makes the engine throw counts as "not accepted", and the text of its "Game Error" message isn't compared, so turning a C#-style exception into a `Result` or a proper validation message is not a difference.

Events are compared as decoded values, not as JSON text (`observe/src/events.rs`). The working tree decodes each event of the trace with its own reader before submitting it. A bot decision is compared after both sides' events are decoded and encoded again by the working tree, so member order, whitespace and members the reader ignores make no difference. An event the working tree can't decode counts as rejected, and its message isn't compared, the same as a Game Error. When the event format changes (wave 4b in [rewrite-plan.md](rewrite-plan.md)), the reference still records the old format, and `events::from_reference` is the one place that has to read it.

Since step 3b the engine no longer catches its own panics. The working tree's replay (`session.rs`, `guarded`) treats a call that panics as the server does: the call failed, with the panic message as its error, and the game goes on. So a former C# exception that is now a panic compares like the exception did. A view that fails to build is compared as failed: the reference records `error: <exception>` as the view's hash, and the error text isn't compared. Card, leader, location and territory ids are compared where they appear in views and event JSON, because clients see and send them.

## Running it

From `rust/`:

```sh
cargo build --release -p observe

# 200 games each, through heavy.sh like any batch run
tools/heavy.sh ./target/release/observe live --mode bots --seed 1 --games 200 --jobs 4
tools/heavy.sh ./target/release/observe live --mode fuzz --seed 1 --games 200 --jobs 4

# One game in detail: a failing game is recorded again with full views at its first difference
./target/release/observe first --mode fuzz --seed 1 --game 17

# Trace files (from --keep-failures, or `observe record`)
./target/release/observe replay /tmp/failures/fuzz-1-00017.jsonl

# Two reference binaries on the same games (see "Comparing two references" below)
tools/heavy.sh ./target/release/observe compare-refs OLD/observe NEW/observe --mode fuzz --seed 1 --games 200 --jobs 4

# What 200 fuzz games exercise, recorded by the reference (see "Reach" below)
tools/heavy.sh ./target/release/observe reach --mode fuzz --seed 1 --games 200 --jobs 4 --recorder reference/observe
```

`live` takes a heavy slot itself, so running it outside `heavy.sh` also waits for one, but only `heavy.sh` enforces the 3 GB memory cap. Use `--jobs 2` while trying something new and `--jobs 4` at most. Each job records a game with the reference binary into memory, then replays it, so a job uses one core at a time. Nothing goes to disk unless you pass `--keep-failures DIR`. Add `-v` for one line per game.

A failing game is recorded again with full views at the line of its first difference, so the report names the viewer and lists the view fields that differ:

```
fuzz-1-00017: first difference at trace line 304, event Bid (bot)
  what:      views
  viewer:    player 1 (Choam) (view 1)
  reference: 72a8902d27c1f8bc
  working:   0b5e...
  view fields that differ (path: reference -> working tree):
    /status/message/0/v: "Choam is bidding" -> "Choam bids"
```

`live` ends with the `first` command for the first failing game. Other options: `--hang-seconds` (default 60; a call that runs longer counts as hung), `--memory-mb` (default 640 per game), and `--reference BIN`. A game whose call hangs or goes over the memory budget fails, as `runaway` when the reference ran away at the same call too, since no real game may end like that (on the server it freezes every game). The memory default was 2048 until fuzz seed 1 games 2 and 176 reached states where a `Bid` pass made the engine allocate without end (fixed since, escapes.md "Automatic bid passes that never stop"). With 4 jobs in one `live` process, one such game at 2 GB plus the other three went over heavy.sh's 3 GB cap and the whole run was killed. Normal fuzz games stay well under 640 MB.

The full views of every line of a long game take gigabytes, so `live`, `first` and `compare-refs` record a failing game again with full views only at the line of its first difference (`record --full-views-from N --full-views-to N`), or from that line to the end when the allow file has view rules, since those rules may accept the difference and then need full views for the lines after it. The replay builds full views only for lines that have them. A recorder from before 2026-11-16 doesn't know these arguments, ignores them and records full views everywhere, so until the shared reference is rebuilt from a later source, a difference late in a long game still runs it out of memory (fuzz seed 1 game 2 did at trace line 17,000 even with `--memory-mb 3000`), and `live --start I --games 1` is no help either. To check this, a reference built from the working tree with a view field changed from the 2000th event on was run with `first --mode fuzz --seed 1 --game 42`. It showed the changed field at trace line 38,374 (of about 41,000) in 49 s, with the `first` process at 50 MB. The same reference with every line's full views ran out of memory at trace line 2537.

See "Timing" below for how long 200 games take.

## The reference binary

`rust/reference/observe` (not in git) is the `observe` binary built from a frozen copy of the engine, bots and view crates. A copy sits in `~/treachery-native-ref/observe`, which `observe` uses when `rust/reference/observe` doesn't exist, as in a git worktree. `$OBSERVE_REFERENCE` or `--reference BIN` override both. A branch that changes behavior builds its own reference with `rust/tools/build-native-ref.sh --out DIR`, which leaves the shared one alone, and passes `--reference DIR/observe`. `observe live` and `first` run it as `observe record --mode M --seed S --game I` and read the trace from its stdout.

The current reference is built from `~/treachery-native-ref/rust`. That's a copy of `rust/` taken on 2026-11-11, before step 2 (dropping old versions) started editing the engine, with `rust/observe` copied in. It was last rebuilt on 2026-11-12 with rustc 1.100.0, after the steered fuzz generator was copied in (a rebuild with the runaway and card trade fixes waits for promotion, see "Changes to the recorder so far" below); `rust/reference/SOURCE` records the compiler. If `~/treachery-native-ref` is lost, unpack `rust/reference/source.tar.gz` into `~/treachery-native-ref` (it holds `rust/<crate>/Cargo.toml` and `src` for every crate the binary needs). To build the binary again:

```sh
rust/tools/build-native-ref.sh --source ~/treachery-native-ref/rust
```

The script builds in a temporary target dir (about 1.5 GB, deleted afterwards) and writes three files to `rust/reference/`: the binary, `source.tar.gz` with the exact sources, and `SOURCE`, which says where they came from. `--archive rust/reference/source.tar.gz` rebuilds from that archive. `--rev REV` builds from a git revision once the harness is committed. With no argument it builds from the working tree.

Rebuild the reference only on purpose. A reference built from a working tree with an unreviewed change makes that change invisible.

### Changing the harness itself

The trace format is defined by `observe/src/trace.rs`, and both sides must agree on it. When native refactors change engine APIs, update the working tree's `rust/observe` (mostly `session.rs`) so it compiles and makes the same calls. Don't change what it records. If the recorder itself has to change (a new observable, a fuzzer fix), change it in both places: copy the new `rust/observe/src` into `~/treachery-native-ref/rust/observe/src` and rebuild the reference with `--source`.

The fuzz generator reads the shadow game through the view (`actions::action_view`, `actions::default_draft`, `side::deals`) and a few engine fields and functions (`players`, `forces_in_locations`, `turn.current_main_phase`, `battle.current_battle`, and for intrusions and retreats `ambassador_in`, `terror_in`, `forces_in_territory`, `skilled_as` and `Revival`'s placement limits). A refactor that renames these must update `fuzz.rs` in the working tree too, without changing what it generates.

Changes to the recorder so far:

- 2026-11-12: steered fuzz generator and `observe reach` (see "How games are made" and "Reach"). The reference before this change is kept as `~/treachery-native-ref/observe-before-fuzz-reach` with its sources in `source-before-fuzz-reach.tar.gz`.
- Pending (2026-11-15, focus games and steering towards intrusions and retreats, see "How games are made" and "Phases the fuzzer doesn't reach", not frozen yet): every fuzz game changes, and bots games don't (their headers gain `focus: false`, which replay doesn't compare). The shared reference is the one built from main caabede, and it records the old games. Against a reference built from the branch (`build-native-ref.sh --rev HEAD --out DIR`, passed with `--reference`), `live` passes 200 of 200 bots games (197 ended, 3 stalled, as before; built from 823eaa9, one commit before the last, which only changes fuzz games) and 12 of 12 fuzz games (seed 1; 4 ended, 8 at the cap; built from 99872a5). Promote by rebuilding the reference from the merge commit; until then the shared reference keeps generating the old fuzz games, which pass as before.

Deliberate behavior changes frozen into the reference (the frozen engine edited the same way as the working tree, then rebuilt):

- 2026-11-13: automatic bid passes stop with the bidding phase and happen at most once per player per bid, and `CardTraded` must name the ally. See escapes.md, "Automatic bid passes that never stop" and "A card trade offered to a faction that isn't the ally". The frozen files changed are `engine/src/events/bid.rs` and `engine/src/events/card_traded.rs`. Until the branch is merged and the reference promoted, the changed frozen copy is `~/treachery-native-ref/rust-runaway-fixes` and its binary `~/treachery-native-ref/observe-runaway-fixes` (pass it with `--reference`). The reference before this change is kept as `~/treachery-native-ref/observe-before-runaway-fixes`, with its sources in `source-before-runaway-fixes.tar.gz`.
- Pending (2026-11-13, bot fixes of step 4, commits 0e82687 and c0f4762, not frozen yet): bot decisions change on purpose (behavior-changes.md, Bots entries). Against the reference built from 70ba201, 88 of 200 bots games (seed 1) and 90 of 200 fuzz games (seed 1) still match. Every first difference is a bot decision. Each was attributed by replaying the trace through builds with one fix reverted: the fix whose revert moves the difference later explains it. Bots: Bene Gesserit prediction 92, ally dial estimate 13, Mirror Weapon pick 7. Fuzz: 90, 7 and 13. With the prediction reverted, the games it hid show the Mirror Weapon pick, the ally dial estimate and the poison voice, and nothing else. Some first differences are decisions such as `DealOffered` or `Bid`: an earlier decision of the same bot came out the same but used a different number of draws. After merging main's step 3c (its log text fixes compared with the coordinator's allow rules), 86 bots and 83 fuzz games match. The bot fixes explain the same games as before, minus 3 bots and 9 fuzz games whose first difference is now an earlier view difference: a fixed Nexus text showing in the view's log. Main's own build shows each of these at the same line. Promote by rebuilding the reference from the merge commit.
- Pending (2026-11-13, the four bugs at the end of behavior-changes.md, not frozen yet): against a reference built from 9ac66fa, 184 of 200 bots games and 77 of 200 fuzz games (seed 1) match. Attributed the same way. Bots: Richese cards in deck and cache 8, Guild shipping back to reserves 8. Fuzz: refused Karama bids 105 (93 after a `Bid` with `Passed` and `KarmaBid` that threw in the reference, 12 after a Karama bid without a card), Richese cards 18. The dropped leftover intrusions and the storm-loss estimate cause no first difference in these games. Recorded by the working tree, no game ends hung, out of memory, with a panic or with a bot exception. The same games stall as in the reference (bots 37, 48, 73, 121; fuzz 121), except bots game 163 (the double sale), which now ends. 69 fuzz games reach the 3000-event cap, against 81 in the reference.

- Pending (2026-11-13, the new random number generator, behavior-changes.md "Random number generator", game version 189, not frozen yet): every draw changes, so no game can match a reference built before it. Checked instead by recording 200 bots and 200 fuzz games (seed 1) with `observe record`, once with the build of the base commit (345bf6c) and once with the branch, and comparing how they end:

  | Ending | bots, before | bots, after | fuzz, before | fuzz, after |
  |---|---|---|---|---|
  | gameEnded | 196 | 197 | 130 | 126 |
  | cap (3000 events) | 0 | 0 | 69 | 74 |
  | stalled | 4 | 3 | 1 | 0 |

  No game panics, hangs, runs out of memory or ends with a bot exception, before or after. The games that stall are bots 37, 73 and 121 in both builds (48 only before; fuzz 121 only before). Against a reference built from the branch itself (a copy of its `observe` passed with `--reference`), `live` passes 20 of 20 bots and 20 of 20 fuzz games (seed 1), so the harness records and replays games with the new generator. Promote by rebuilding the reference from the merge commit.

- Pending (2026-11-15, wave 2a of rewrite-plan.md, behavior-changes.md "Order of spice, forces and other lists on the board", not frozen yet): every bots and fuzz game (seed 1, 200 each) differs from the shared reference, first in a view. `observe live` ran out of memory re-recording the failing games with full views, so each game was recorded with `REF record` and replayed with `observe replay` through the branch's build, a build with all thirteen fields put back on `NetDictionary`/`NetHashSet`, and one build per field put back. All 400 games pass with every field put back. Attributed by the one field whose revert moves the first difference later: forces 200 bots and 197 fuzz games, bids 2 fuzz, spice 1 fuzz. With the forces put back as well: bids 146 and 123, spice 40 and 59, stronghold owners 1 and 10, discoveries 3 and 1, ambassadors 0 and 1; 9 bots and 3 fuzz games then match, and bots game 76 shows spice and discoveries at the same event. Promote by rebuilding the reference from the merge commit.

- Pending (2026-11-15, behavior-changes.md "The card swap after an auction told everyone who holds a Nexus card" and "The Emperor's homeworld abilities counted Salusa Secundus as Kaitain", not frozen yet): games with Nexus cards and no Ixian gain an `EndPhase` after every sale, and Emperor games with only one homeworld at a threshold can differ at the end of Bidding and in revival limits. The recorded test games, the bot knowledge game 00008 and four fixtures were refitted (see "Refitting recorded games"). Promote by rebuilding the reference from the merge commit.

Many failing games make `observe live` unreliable with a reference from before 2026-11-16. It records each failing game again with full views, and a long game's full-view recording goes over the 640 MB budget, so the report says "hang differs at end" and hides the real first difference. Several such recordings at once also went over heavy.sh's 3 GB cap and killed the run. For a change that makes many games fail, record each game with `REF record --mode M --seed S --game I` and replay it with `observe replay FILE` instead.

## Comparing two references

`observe compare-refs A B` records the same games with two reference binaries and compares the two traces, without replaying anything through the working tree (`observe/src/compare_refs.rs`). It's for checking that a new recorder generates the same games as the old one, for example after the event format changes (wave 4b in rewrite-plan.md). It takes the same options as `live` (`--mode`, `--seed`, `--games`, `--start`, `--jobs`, `--allow`, `--keep-failures`, `-v`), takes a heavy slot the same way, and should run under `heavy.sh` too.

Both traces must make the same calls in the same order: the same queries, bot rounds, submissions (by source, player and host flag), undos and ending. For each call it compares what `live` compares, by the same rules: whether an event was accepted, the rejection message, the log, the view hashes, the applicable types, and which player's bot acted. The event JSON is never compared, in event lines or in bot decisions, so A and B may write events in different formats. The header must agree on mode, seed, game, game version and seated humans. A game whose call hangs or runs out of memory in both at the same call fails as `runaway`, as in `live`. A failing game is recorded again with full views at its first difference, so the report lists the view fields that differ. That recording is only used when it shows the same difference, because a long game can run out of memory with full views. The report calls the two sides `A` and `B`. `--keep-failures DIR` writes both traces of a failing game, as `NAME-A.jsonl` and `NAME-B.jsonl`.

Checked on 2026-11-15, seed 1:

- The shared reference against itself matches in 20 of 20 bots and 20 of 20 fuzz games.
- Against a recorder that rewrites every event (members after `$type` reversed, an unknown member added, spaces around separators), it matches in 10 of 10 fuzz games. `observe replay` of one such trace (fuzz game 3) passes too. The replayer before events were compared decoded fails it at the first bot decision.
- `observe-before-chacha` against the shared reference fails every game at line 1, because the older one records game version 188.
- `observe-before-runaway-fixes` against `observe-runaway-fixes` (same game version) fails fuzz games 2 and 4 of 0 to 4. In game 2 the first difference is at line 9049: a `CardTraded` that the older one accepts and the newer one rejects with "Invalid requested card".

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

   View rules don't scale to a change that shows up in every game. Recording a long game with full views from its first difference to the end takes more than the default 640 MB per game (2 GB wasn't enough for seed 1 bots game 3), so the reference ends with `memoryLimit` and the game fails. For a removed or renamed view field, build a temporary `observe` from the working tree with the old field put back, and run `live` with no allow file instead. The view cleanup of step 3b did that for `Simple.dismissLabel`, which every game's `EndPhase` form has.
3. Once the change is accepted, freeze the new behavior: update the frozen copy (`~/treachery-native-ref/rust`, or a git tag) to include it, rebuild the reference, and run the full 200 + 200 games to confirm a clean pass. Keep the old `source.tar.gz` if you might need to go back. An allow file is only for the review, so drop it after the rebuild.

## Refitting recorded games

`observe refit IN OUT` carries a savegame, a recorded trace or a bot knowledge game over to an engine whose random draws differ (`observe/src/refit.rs`, `refit_files.rs`). It replays the log and keeps every event that still passes validation. An event that doesn't is replaced by an event of the same type from the same faction: first what that faction's bot decides, then one built from the choices the client's form offers (the fuzzer's `build`). Before each event the bots play on until the game reaches the turn and main phase the old log executed it in, and an event whose moment has passed is dropped. After the last event the bots play on to the phase the old log ended in. Bot rounds that stop changing the phase are taken back.

`--old FILE` says what the old engine saw: per event whether it was valid and the turn and main phase it ran in, and the final phase, turn, battle and factions per seat. It may also list `offers`, `[faction, "EventType"]` pairs the end should offer, since a design fixture is made for the actions it ends on. With `--seeds N` it tries N seeds for `EstablishPlayers`, only ones that seat the same factions in the same seats, and keeps the result that ends closest to the old log: first the phase (and seats and allies), then the fewest missing offers, then the turn, then the fewest changed events. `--probes FILE` renumbers the steps of a bot probe file to match a refitted knowledge game.

For game version 189 the `--old` files came from a throwaway test run in a checkout of the base commit, and every fixture in `scripts/fixtures`, every trace in `rust/testdata/games` and `rust/view/tests/data`, and the four knowledge games of `rust/bots/tests/golden` (with their probe files) were refitted. The 30 fixtures whose end lost an action the old one offered were refitted again with `offers` (every action of every player at the old end) and 40 seeds. 51 of the 53 fixtures end in the same phase and turn as before; `mc-handswap` ends one phase later in the same turn, and `retreat` ends in the battle report instead of the retreat (known-issues.md). A few end without an action they offered before: `karma-revival` (Emperor's `KarmaFreeRevival`), `nexus-prescience-base` (Emperor's `NexusPlayed`), `mc-discard` (Moritani's `Discarded`; Guild still discards), `ship-fremen` (Fremen's `Caravan`) and some incidental `RaiseDeadPlayed`. The snapshot tests were then rewritten with their update modes, and the bot probes were moved with `RELOCATE_PROBES` (bots/tests/common/mod.rs).

## Trace format

One game is one JSON Lines stream (`observe/src/trace.rs`). Every line has an `op`:

- `header`: format version, mode, seed, game index, game version, seated humans, the configuration (for information only, since the engine sees everything through events) and `viewHash`, how views are hashed.
- `query`: `GetApplicableEvents(p, host)` and the type names it returned, or that it threw.
- `bots`: one round of asking the bots, as the server's `perform_bot_event` does it: levels Highest to Low (and End when bots act as host), players in `order`, first event wins. It lists the bots created in this round with their seeds, and the choice.
- `event`: an event submitted with validation, with its source (`host`, `bot`, `human`, `fuzz`, `mutate`), its JSON, the result, the rendered message, and the observations.
- `undo`, `restore`, `dismiss`: `Undo(until)`, `RestoreRecentlyUndone`, `DismissRecentlyUndone`, with observations.
- `end`: why the game stopped (`gameEnded`, `cap`, `stalled`, `botException`, `panic`, `setupFailed`, `recorderPanic`, `hung`, `memoryLimit`). `recorderPanic` means the recorder's own reads of the game threw, for example a fuzzed state with a null card in a hand; the replay simply stops there. For `hung` and `memoryLimit`, `pending` holds the call that never returned.

Observations are `log` (new messages as `[target, text]`), `logCut` (the log shrank, after undo), `views` (16-hex-digit hashes of each view's JSON: players in order, then the host, then the observer) and, with `--full-views`, `full` (the views themselves, on every line or on the lines `--full-views-from` and `--full-views-to` give). Views are built, as on the server, from a separate replica that executes accepted events without validation, so building them never changes the game.

New traces hash views with `fold64` (`observe/src/view_hash.rs`), a 64-bit multiply-and-fold hash defined in that file, so it never changes with a crate or compiler version. Traces recorded before 2026-11-16 have no `viewHash` and hold the first 8 bytes of each view's SHA-256. The replay hashes views the way the trace's header says, so those traces and the references that record them still work. `record --view-hash sha256` records the old way. `compare-refs` uses it when one recorder writes `fold64` and the other SHA-256: it records the newer one's game again with `--view-hash sha256`. The view JSON goes straight into the hasher, without a buffer. On aarch64, SHA-256 uses the CPU's SHA-2 instructions (`sha256_arm.rs`), so replaying the older traces costs only a little more than `fold64`.

## How games are made

`observe/src/record.rs` plays games at the latest version only. The configuration (`config.rs`) draws a ruleset, rule toggles, factions, player count, turns, a host style and sometimes seated humans. The host establishes the game. Then each round queries every player, asks the bots, and submits the first bot event. If no bot acts, a seated human passes in the alliance phase, or the host acts (faction selection, decks, custom setup, HMS placement, `EndPhase`).

30% of fuzz games are focus games (`config::FOCUS`). They seat Bene Gesserit, Ecaz and Moritani, whose fighters, ambassadors and terror tokens interrupt a shipment or move, together with the Guild, Fremen and Tleilaxu, whose moves, worm rides and revivals onto the planet can run into them, plus up to three other factions. They play all expansions' advanced rules, with Bene Gesserit accompanying to the shipment's location, and keep the house rules drawn for the game. Random games rarely seat all of these together, and before focus games the phases of those intrusions were never reached. The header's `config.focus` says whether a game is one.

Fuzz games (`fuzz.rs`) add, per round:

- a random event (12%). Usually it is steered: every player (and nobody) is queried, and a (player, type) pair is drawn with weight 1 / (1 + times accepted in this phase)^1.5. The weight is also lowered for types picked often in this phase and for types accepted often in the whole game, so always-applicable types (deals, donations, permissions) don't take every step. Otherwise it is a random player's applicable type, or any type now and then.
- during a battle, a battle action for a player (25% of bot rounds): an applicable `Voice`, `Prescience`, `JuicePlayed`, `BattleRevision`, `TreacheryCalled`, `FaceDanced` and the like, weighted towards rare types. Now and then `KarmaEcazDial`, `JuicePlayed`, `ResidualPlayed` or `AuditCancelled` even when not applicable. `JuicePlayed` usually gets `Aggressor` when the form offers it.
- a battle plan submitted again with its leader, a card or its dial changed (30% of bot rounds, when a faction that already planned this battle may plan again, mostly after `BattleRevision`). Parts the opponent saw with prescience must stay, so the engine rejects some of these.
- during skill assignment and battles, a step towards a retreat (50% of bot rounds, when possible): a player offered the Diplomat skill chooses it, or a player with a Diplomat leader plans the battle. Only a battle lost by a Diplomat who survives leads to `Retreating`.
- an intrusion (10% of bot rounds, 60% when the event type was rarely executed in this phase): a player who may ship, move, ride a worm, revive onto the planet or accompany a shipment does so into a territory where Bene Gesserit fighters, an Ecaz ambassador or a Moritani terror token would interrupt it (`fuzz::intrusion_locations`).
- an event of a type rarely executed in this phase (30% of bot rounds), changed copies of the bot's event (10%), and undo, restore and dismiss.
- when the same bot sends the same accepted event again in the same phase (bots resend `AllyPermission` when their permissions can't settle), the fuzzer acts instead 80% of the time from the third repeat on, so the loop doesn't use up the game's events.

Values come from what the client's form offers. For an event type and a player, `Pools::of` reads the view's action options (`treachery_view::actions::action_view`). These hold the draft the form sends untouched (the default draft after the form adjusted it), the choice lists (heroes, weapons, targets, card types, enum members such as Voice types or Prescience aspects) and the maximums of number pickers. Each event field gets a choice from the list named like it (`heroes` for `_heroId`, `weapons` for `_weaponCardId`), or from a list holding its current value (`choices` for `JuicePlayed.Type`). Multi-select fields (`_cardIds`, arrays) get a subset of their list. Amounts get a value up to the matching maximum. Battle plans take a force mix from the form's `mixes`. Force placements use the player's own battalions or a few forces at locations the form offers. Deal answers are the accept, decline and withdraw events of the deals panel (`treachery_view::side::deals`). Some choices are preferred. The destination of forces (`_toId`, `_locationId`, `_targetId`) is a location where they intrude half the time, when there is one, and such forces mostly move all together and don't pass, since a terror token needs at least three. A revival with a location places some of its forces there. The Diplomat skill and Diplomat leaders are picked 90% of the time when offered. Fields the form offers no choice for keep the draft's value, or get the player's own cards, leaders and force locations or a small amount. Each event tries up to 8 candidates and sends a valid one 90% of the time. Invalid candidates are still sent now and then and must be rejected the same way. The generator reads a separate shadow copy of the game, so it can't change state that the replay wouldn't change.

Randomness comes from SplitMix64 seeded by mode, seed and game index (`rng.rs`). The engine's seed is in the `EstablishPlayers` event and each bot's seed is in the `bots` line, so a game depends only on its mode, seed and index.

## Timing

Measured on 2026-11-12, `--jobs 4` through `heavy.sh`, with another agent's runs on the machine (load average 7 to 25):

| Run | Wall time | CPU time |
|---|---|---|
| 200 bots games (seed 1) | 11 min | 37 min |
| 200 fuzz games (seed 1) | 15 min | 47 min |
| 200 fuzz games (seed 1), steered generator, `--memory-mb 640` | 15 min | 51 min |

On 2026-11-16 the replay got faster (load average 3 to 8, CPU time of `observe replay`, one trace at a time). The view replica builds the faction sheets, about half of each view's JSON, once instead of for every viewer after every event, and views are hashed as described in "Trace format":

| Replay of the reference's traces (seed 1) | before | after |
|---|---|---|
| bots game 7 | 48 s | 12 s |
| fuzz game 42 | 23 s | 13 s |
| bots and fuzz games 0 to 11 | 238 s | 106 s |

Traces of the same 24 games recorded by the new recorder (`fold64`) took 115 s to replay, the same as SHA-256 within the noise of a shared machine. The reference's own recording in `live` keeps its old cost until it is rebuilt from a source with these changes.

The C# oracle, which the harness replaced, took about 23 minutes per 200 fuzz games. The harness is faster per game, but its games are longer: every game is at the latest version, and fuzz games run up to 3000 events (the recorder's cap; before the cap, 200 fuzz games took 28 minutes). Each game is computed twice, once by the reference and once by the working tree, and most of the time goes into building 3 to 13 views after every accepted event. For a quick check while working, run 30 to 50 games.

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

### Phases the fuzzer doesn't reach

Measured on 2026-11-15 with `observe reach --mode fuzz --seed 1 --games 200` and the recorder of main 77ba78e (before) and of the focus games, intrusion and retreat steering (after). A phase counts as reached when an event is submitted or a type is offered in it, or, for `GameEnded`, when a game ends. Before, `reach` didn't count `GameEnded`; the before column counts it here.

| | before | after |
|---|---|---|
| events submitted | 322,718 | 381,745 |
| event types accepted | 130 | 130 |
| (type, phase) pairs accepted | 903 | 959 |
| phases reached (of 123) | 105 | 115 |
| endings | 125 ended, 75 at the cap | 100 ended, 100 at the cap |
| wall time (`--jobs 4`) | 399 s | 547 s |

No game hung, ran out of memory, stalled or panicked, before or after. Focus games are larger (6 to 9 players, every expansion), so more of them stop at the 3000-event cap, and fuzz runs take about a third longer. The ten phases gained are `Retreating` (6 games) and these intrusions: Bene Gesserit by revival (9), Guild move (4) and worm ride A (1); terror by worm ride B (4) and revival (2); ambassador by Guild move (3), worm ride A (1), worm ride B (1) and revival (1). Intrusions that were reached before are reached in more games: those by caravan in 10 to 23 games instead of 2 or 3, those by the Guild's shipment in 12 to 24 instead of 4 or 5, and those by other players' moves in two to three times as many.

Lost in exchange: `ChoamMovePrevention`, `Diplomacy`, `KarmaPrescience` and `KarmaRevivalPrevention` were each accepted in a few games before and in none after. Each was rare before (accepted in 6, 1, 2 and 1 games), so this may be chance as much as the smaller share of other faction mixes.

The eight phases still not reached, and the unit tests that cover them instead, in `rust/engine/tests/phases_unreached.rs`:

- `TerrorTriggeredByBeneGesseritAccompaniesGuildShip` and `TerrorTriggeredByBeneGesseritAccompaniesNonGuildShip`: unreachable. `determine_next_shipment_and_move_sub_phase_on_terror_triggered` (engine/src/game/movement.rs) sends a terror token triggered while Bene Gesserit accompany a shipment to `TerrorTriggeredByGuildShip` or `TerrorTriggeredByNonGuildShip`, as C# did, and nothing else enters these two. Accompanying brings at most two forces, and `check_terror_triggered` lets fewer than three trigger a token only while Moritani is *not* at its low threshold. Bene Gesserit can only accompany into the shipment's territory, so the shipment itself must not trigger the token: Moritani or its ally ship. Tests: `terror_triggered_by_bene_gesserit_accompanying_a_non_guild_shipment` and `..._the_guilds_shipment` (the game enters the shipment's own terror phase, then goes on to the move without asking Bene Gesserit to accompany again), and `bene_gesserit_accompanying_dont_trigger_terror_when_moritani_is_at_its_low_threshold`. Wave 5 can drop the two phases.
- `AmbassadorTriggeredByBeneGesseritAccompaniesGuildShip` and `AmbassadorTriggeredByBeneGesseritAccompaniesNonGuildShip`: unreachable, and possibly a rules bug. `check_ambassador_triggered` lets Bene Gesserit trigger an ambassador only when the event adds forces (not advisors), but `BeneGesseritAccompanies::forces_added_to_location` is always 0. It counts its forces as advisors even when `execute_concrete_event` ships them as fighters (into an empty territory, one Bene Gesserit occupy, or any territory when the advisors rule is off). Whether fighters that accompany a shipment should trigger an ambassador is a question for the owner. Tests: `bene_gesserit_fighters_accompanying_a_non_guild_shipment_dont_trigger_an_ambassador` and `..._the_guilds_shipment_...` pin today's behavior (no trigger, the game goes on to the move).
- `BlowA` and `BlowB`: never a resting phase. `enter_blow_a` and `enter_blow_b` (engine/src/game/blow.rs) draw spice cards until a blow and leave the phase within the same event, so no player ever acts in it, but every game passes through it. `StormLosses` also uses `BlowA` as a marker: `phase_before_storm_loss == BlowA` means the losses came from the storm phase. Wave 5's interrupt stack should replace that marker. Tests: `the_spice_blow_passes_through_blow_a_and_blow_b_in_one_event`, `storm_losses_taken_in_the_storm_phase_end_in_the_storm_report` and `storm_losses_taken_during_a_shipment_go_back_to_the_shipment`.
- `BeneGesseritIntrudedByFremenRidingMonsterB` and `TerrorTriggeredByGuildMove`: too rare. Each is reached in 1 to 4 of 200 games, and this run happened to miss them, while a build one commit earlier reached them in 3 and 2 games. Other phases reached in 3 games or fewer are `AmbassadorTriggeredByFremenRidingMonsterA`/`B`, `AmbassadorTriggeredByRevival`, `BeneGesseritIntrudedByFremenRidingMonsterA`, `TerrorTriggeredByRevival`, `TerrorTriggeredByFremenRidingMonsterA`, `Auditing` and `Discarding`. Wave 5 should still run a second fuzz seed for the ship and move agent.

`rust/engine/tests/intrusion_phases.rs` covers every intrusion phase reached today, the rare ones included: for each kind of event (the Guild's shipment, another faction's shipment, the Guild's move, another faction's move, Caravan, a revival to the planet, worm rides A and B) and each obstacle (Bene Gesserit fighters, a terror token, an ambassador), `enters` checks the phase and who may react, and `resumes` reacts and checks where the game goes on. The module is named after the phase, for example `terror_by_guild_move::enters`. `phases_unreached.rs` also has `discarding_after_robbery_returns_to_the_phase_after_the_terror` and `auditing_goes_on_to_the_battle_conclusion`. Both files build their games with `rust/engine/tests/intrusion_table/mod.rs`.

`None` is no phase.

## Tests

`cargo test --release -p observe` (part of `check.sh`, about 5 s) checks that clean recordings replay without a difference, that recording is deterministic, and that a trace with any observable changed fails: a log message, a view hash, a rejection message, an event's validity, a bot decision, an applicable-events list, or a cut-off trace. It also checks that full views, on every line or on one, name the differing field, that traces hashed with SHA-256 or `fold64` replay, and that allow rules accept what they cover. Further tests check that events written differently but decoding to the same value replay without a difference, that an event that doesn't decode compares like a rejection, and that `compare-refs` matches the same games with every event in another format but catches any changed observation (`tests/compare_refs.rs`).
