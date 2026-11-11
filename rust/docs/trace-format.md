# Trace format

The C# oracle (`treachery-online/Oracle`) writes traces. The Rust replayer reads them, runs the same events through the Rust engine, and must reproduce every recorded value. This document is the contract between the two. If the oracle and this document disagree, the oracle has a bug.

Format versions: trace format 2, dump format 1 (the `traceFormat` and `dumpFormat` header fields). Format 2 adds the `bots` query entry (section 3, "Bot deliberations"); everything else is as in format 1, so a replayer that skips `bots` entries reads both.

Contents:

1. [Files](#1-files)
2. [Header line](#2-header-line)
3. [Step lines](#3-step-lines)
4. [Undo lines](#4-undo-lines)
5. [End line](#5-end-line)
6. [How to replay a trace](#6-how-to-replay-a-trace)
7. [Canonical state dump](#7-canonical-state-dump)
8. [What the dump leaves out, and why](#8-what-the-dump-leaves-out-and-why)
9. [How runs are seeded](#9-how-runs-are-seeded)
10. [Summary files](#10-summary-files)

## 1. Files

One game is one file, in [JSON Lines](https://jsonlines.org/) format: UTF-8, one JSON object per line, each line ending in `\n`. Files named `*.jsonl.gz` are the same content compressed with gzip.

- Bot and fuzz games: `<mode>-<masterSeed>-<gameIndex as 5 digits>.jsonl`, for example `bots-1-00042.jsonl`.
- Savegame replays: `replay-<savegame file name without extension>.jsonl`.

The first line is the header, then any number of step and undo lines, then one end line. Every line has a `kind` field: `header`, `step`, `undo`, `restoreRecentlyUndone`, `dismissRecentlyUndone` or `end`. A file without an end line was cut short (the oracle crashed or was killed).

Lines are written with no whitespace. The order of keys within a line is fixed and is the order listed below, but readers should look keys up by name. Keys marked "optional" are left out entirely when they don't apply.

### Value conventions used in trace lines

- **Enum values** are strings `"<name>(<number>)"`, for example `"Atreides(20)"`. Section 7.3 defines this exactly.
- **Player indices** are positions in `Game.Players` at the time the line was written.
- **Messages and exceptions** use the canonical dump rendering of section 7.

## 2. Header line

```json
{"kind":"header","traceFormat":2,"dumpFormat":1,"dumpPolicy":{"state":"final","hash":"every"},"mode":"bots",...}
```

| Key | Type | Meaning |
|---|---|---|
| `traceFormat` | int | Version of this document's line format. |
| `dumpFormat` | int | Version of the canonical dump (section 7). |
| `dumpPolicy.state` | string | When full state dumps were written: `every`, `every:N` (steps whose `n` is a multiple of N), `final` (only on the end line), or `none`. |
| `dumpPolicy.hash` | string | `every`: every step and undo line has a `hash`. `none`: only lines with a full dump, and the end line, have one. |
| `dumpPolicy.status` | string | Optional. `every` when the trace was recorded with `--status every`: `queries` then also contain `GameStatus` calls (section 3, "Status queries"). Absent otherwise. |
| `mode` | string | `bots`, `fuzz` or `replay`. |
| `masterSeed` | int | bots/fuzz only. The `--seed` of the run. |
| `gameIndex` | int | bots/fuzz only. The game's index within the run. |
| `gameSeed` | int | bots/fuzz only. Seed of the oracle's own choices for this game (section 9). |
| `gameVersion` | int | The `version` argument of `new Game(version, participation)`. |
| `engineSeed` | int | bots/fuzz only. The `Seed` of the `EstablishPlayers` event the oracle built. The authoritative value is the one inside the event that actually set up the game. |
| `config` | object | bots/fuzz only. Informational: `baseRuleset`, `rules`, `allowedFactions`, `numberOfPlayers`, `maximumTurns`, `hostStyle`. Everything the engine sees is in the events. |
| `participation` | object | The `Participation` object passed to `new Game`, in canonical dump form (section 7). The replayer must build an equal object: same dictionary and set contents in the same order. |
| `fuzz` | object | fuzz only. The probabilities used (`randomEvent`, `mutateBotEvent`, `undo`, `restoreOrDismissAfterUndo`, `battleAction`) and the `profile` (`client` or `wide`, see the Oracle README). Informational. |
| `source` | string | replay only. The savegame file name. |

### Host styles

`config.hostStyle` says how the oracle drove the bots. It doesn't change how a trace is replayed, but it explains the `isHost` values and the `queries`.

- `Tests`: like `Test/Tests.cs`. Bots are asked with `GetApplicableEvents(p, true)` in `Game.Players` order, and every bot event is submitted with `isHost = true`.
- `ServerBotsHost`: like `Server/GameHub_GameEvents.cs` `PerformBotEvent` when bots act as hosts. The bot order is shuffled every step. Events from the four in-phase priority tiers are submitted with `isHost = false`, and end-phase events with `isHost = true`.
- `ServerHumanHost`: the same, but bots never act as host. When no bot acts, the oracle plays a human host.

In all styles, when no bot acts, the oracle acts as the host (`source: "host"`). It submits, in this order of preference: `FactionSelected` for the first player without a faction, `CardsDetermined` with the whole current treachery deck, `PerformSetup` with no forces, `PerformHmsPlacement` to a random valid location, or `EndPhase`.

## 3. Step lines

A step is one event submitted to the engine.

| Key | Type | Meaning |
|---|---|---|
| `kind` | `"step"` | |
| `n` | int | Step number, starting at 0. Undo lines also use step numbers. |
| `source` | string | Who made the event: `bot`, `host` (the oracle acting as host), `fuzz` (random event), `mutate` (a bot or host event with one member changed), `human` (the oracle passing in the v188 alliance phase for a seated human, whose bot waits for humans and so never passes), or `replay` (from a savegame). |
| `actor` | int or null | Index of the player the event was made for. null for host events and some fuzz events. Informational. |
| `validate` | bool | The `performValidation` argument of `Execute`. Always true in traces written so far. Replayers pass it on as recorded. |
| `isHost` | bool | The `isHost` argument of `Execute`. |
| `mutatedField` | string | Optional. For `source: "mutate"`, the JSON member that was changed, or `Initiator`. Informational. |
| `bot` | object | Optional. For bot events: `faction` (the faction the bot was created for), `player` (index), `priority` (`Highest`, `High`, `Middle`, `Low` or `End`: which `IBot.Determine…Action` returned the event), and `seed` (the bot's random seed). For porting the bots. |
| `botOrder` | int[] | Optional. The order in which players' bots were asked. For porting the bots. |
| `queries` | array | Engine calls the oracle made after the previous line was written and before this event was executed, in call order. An entry `[playerIndex or null, isHost]` is a call to `Game.GetApplicableEvents` that returned. If the call threw, the entry is `[playerIndex or null, isHost, exception]`, with the exception object of 7.4 (the same form as in `applicable`). The drivers that make these calls (bots, fuzzer, host) catch the exception and carry on. An entry `["status", playerIndex or null, isPlayer, result]` is a call to `GameStatus.DetermineStatus` (only with `--status every`, see below). An entry `["bots", {...}]` is one bot deliberation (format 2, see "Bot deliberations" below). **The replayer must make the same calls** (section 6). |
| `botMutatedState` | bool | Optional, only written in runs with `--check-bot-purity true`. Present (and true) if the state hash changed while bots deliberated before this step, or, in fuzz runs, while the fuzzer built this step's event (after its queries). Bots can change engine state while they deliberate (bots-plan.md, "Bots are not pure"), and then the trace can't be replayed from events alone: only a replayer that runs the bots reproduces the change. |
| `botMutatedBy` | array | Optional, only with `--check-bot-purity true`. One `[player, faction, level, eventType or null]` per bot ask (one `IBot.Determine…Action` call) that changed the state hash: the player index, the faction the bot plays, the level (`Highest` … `End`) and the type name of the event the ask returned. Absent when only the fuzzer changed the state. |
| `event` | object | The event exactly as `Utilities.Serialize<GameEvent>(evt)` wrote it, inserted verbatim, including the `"$type"` discriminator. Read it as raw JSON (for example `serde_json::value::RawValue`) if you need the exact bytes. |
| `eventAfter` | object | Optional. The same serialization of the event object after `Execute` returned, when it differs from `event`. Some events rewrite their own fields during execution. |
| `result` | string | `accepted`: `Game.History` grew, so the event executed to the end. `rejected`: `Execute` returned a message and no exception was involved. `error`: an exception was thrown (see `exception`). |
| `validation` | object or null | The `Message` that `Execute` returned, in canonical dump form (section 7). null when `Execute` returned null or the exception escaped. For exceptions caught by `GameEvent.Execute`, this is the "Game Error: …" message, whose fourth element is the exception rendered as in 7.4. |
| `exception` | object or null | See below. |
| `historyCount` | int | `Game.History.Count` after the event. |
| `random` | int or null | `Game.Random.GetCount()` (the number of draws from the game's `LoggedRandom`) after the event. null before `EstablishPlayers` created it. |
| `botRandom` | array | Optional. `[[faction, count], …]`: draws so far from each bot's own `LoggedRandom`, in bot creation order. For porting the bots. |
| `typeLists` | array | Optional. New entries of this file's event-type-list table (below). |
| `applicable` | array | Event types each player may submit after the event (below). |
| `hash` | string | Optional (see `dumpPolicy`). Lowercase hex SHA-256 of the UTF-8 bytes of the canonical state dump. |
| `state` | object | Optional (see `dumpPolicy`). The canonical state dump itself. Its bytes in the file are exactly the hashed bytes. |

### The `exception` object

```json
{"phase":"execute","escaped":false,"type":"System.NullReferenceException","message":"Object reference not set to an instance of an object."}
```

- `type` is the .NET full type name, `message` is `Exception.Message`. Stack traces are never recorded.
- `phase` is `validate` if the exception happened while checking applicability or in `Validate()`, before the event changed anything. It is `execute` if it happened after `Execute` entered its execution branch. Partial state changes made before the throw are kept, and they show in the state dump.
- `escaped` is true when the exception was not caught by the engine and reached the oracle. Only event types that override `Execute` can do this. Today that is only `EstablishPlayers`, which has no try/catch. For these, `validation` is null.

How the oracle finds the phase: for event types that use `GameEvent.Execute`, it appends a marker value to `Game.RecentMilestones` before calling `Execute`. `GameEvent.Execute` clears that list exactly when it enters its execution branch, so if the marker is still there the exception came from validation. The oracle then removes the marker. For `EstablishPlayers` it looks for a `Validate` frame in the exception's stack trace.

### Status queries

With `--status every`, the oracle calls `GameStatus.DetermineStatus` the way the server and the client do, at the start of every round of its main loop: after the previous line was written, before bots deliberate or the fuzzer acts. It calls `DetermineStatus(game, players[i], true)` for each player in `Game.Players` order, then `DetermineStatus(game, null, false)` (an observer). Each call becomes a `queries` entry of the next line:

```json
["status", 0, true, {"$type":"GameStatus","DescriptionWhenAwaited":{...},...}]
```

- The second and third elements are the arguments: the player index (or null) and `isPlayer`.
- The fourth element is the result: the returned `GameStatus` in canonical dump form (section 7), rendered as a detached value against the current game, like the `validation` message of a step. All its fields are included, private ones too (`DescriptionWhenAwaited`, `DescriptionWhenWaiting`, `FlashInfo`, `HighlightedTerritories`, `TimedEvent`, `WaitingForHost`, `WaitingForPlayers`, `WaitingInSequence`). If the call throws, the result is the exception object of 7.4 instead.

`DetermineStatus` can change the game: in the `BattleReport` and `BattlePhase` phases it reads `Game.NextPlayerToBattle`, which advances `BattleSequence` (hazards.md 4.3). The server does this to the live game when it sends async-play e-mails. So a replayer makes these calls in order along with the `GetApplicableEvents` calls, compares each result with the recorded one, and the line's `hash` covers what the calls changed. The oracle makes no status calls once the game has reached `GameEnded`, so the end line of a finished game has none; the end line of a game stopped for another reason (`cap`, `stalled`, `hung`, ...) can have them.

Without `--status every`, traces contain no status entries and are byte-identical to traces from before this option existed.

### Bot deliberations

Trace format 2. Each time the oracle asks its bots for an event (`GameRunner.DeliberateBots`), it records one entry, after the deliberation's `GetApplicableEvents` calls and after the bots returned or threw:

```json
["bots",{"order":[2,0,1],"end":true,"created":[["Atreides(20)",0,1420323754]],"choice":{"player":0,"faction":"Atreides(20)","priority":"Low","event":{"$type":"EndPhase","Initiator":20,"Time":"0001-01-01T00:00:00+00:00"}}}]
```

| Key | Meaning |
|---|---|
| `order` | Player indices in the order their bots were asked. The oracle asks the levels `Highest`, `High`, `Middle`, `Low`, `End` in turn; within a level it asks every player's bot in this order and stops at the first event. |
| `end` | Whether the `End` level is asked. False for host style `ServerHumanHost`. |
| `created` | Bots created during this deliberation, in creation order, as `[faction, player, seed]`. A bot is `new ClassicBot(game, player, BotParameters.GetDefaultParameters(faction), new LoggedRandom(seed))`, created when its player is first asked, keyed by the player's faction at that moment (so a bot for `None(0)` can exist from before factions are chosen). Bots live for the whole game, across undo, and are rebound with `SetGameAndPlayer(game, player)` before every ask. |
| `choice` | The event the bots returned, or null: `player`, `faction`, `priority` (the level), and `event`, the event as `Utilities.Serialize<GameEvent>` wrote it before the oracle set its `Time`. It is recorded even when the oracle then submits something else (a mutation, a fuzz event, a host event). |
| `exception` | Only when deliberation threw: the exception object of 7.4. The line is then a `botException` end line. |

The event lists each bot gets are the results of this line's `[player, isHost]` queries just before the entry, one per player. They are not repeated. The draw counts of the bots after the deliberation are the next line's `botRandom` (bots draw only while deliberating).

A deliberation that never returned (`botHung`, `botMemoryLimit`) has no entry. Traces of format 1 have no entries; they replay the same way without them. The oracle records entries in both `bots` and `fuzz` mode. The differ checks them, with `--bots check`, in `bots` traces and in `fuzz` traces of the `client` profile ([differ.md](differ.md#checking-bot-decisions)).

### `typeLists` and `applicable`

After every step, the oracle calls `GetApplicableEvents` in this exact order:

1. For each player `i` in `Game.Players` order: `GetApplicableEvents(players[i], false)`, then `GetApplicableEvents(players[i], true)`.
2. `GetApplicableEvents(null, false)`, then `GetApplicableEvents(null, true)`.

These calls happen before the state is dumped or hashed, and `GetApplicableEvents` can change state (it reads `Game.NextPlayerToBattle`, see hazards.md 4.3).

Each result is a list of event type names (the C# class name, which is also the `$type` discriminator) in the order returned. If the call throws, the result is an exception object as in 7.4 instead of a list.

To keep files small, each file keeps a table of distinct results. Entry `k` is defined the first time it appears, in the `typeLists` of that line, as `[k, list]`. Indices start at 0 and increase by one. `applicable` then holds one `[nonHostIndex, hostIndex]` pair per player, in player order, followed by one pair for player null.

Example: `"typeLists":[[3,["Bid","AutomationConfigured"]]],"applicable":[[3,4],[0,4],[1,2]]` means player 0 may submit `Bid` and `AutomationConfigured` without host rights.

## 4. Undo lines

Fuzz runs sometimes call the engine's undo API instead of submitting an event:

| `kind` | Engine call |
|---|---|
| `undo` | `game = game.Undo(untilEventNr)` |
| `restoreRecentlyUndone` | `game = game.RestoreRecentlyUndone()` |
| `dismissRecentlyUndone` | `game = game.DismissRecentlyUndone()` |

From then on `game` is the returned object. `RestoreRecentlyUndone` can return the earlier game object itself, with whatever happened to it since (for example, its history events were re-initialized to point at the undo game).

Keys: `kind`, `n`, `untilEventNr` (undo only), `queries`, then the same after-state keys as a step: `historyCount`, `random`, `botRandom`, `typeLists`, `applicable`, `hash`, `state`. The queries are made on the game as it was before the call. The after-state keys describe the returned game.

## 5. End line

| Key | Meaning |
|---|---|
| `reason` | `gameEnded` (phase `GameEnded` reached), `cap` (step limit reached: 400 + 80 × maximum turns × max(2, players)), `stalled` (no bot or host event, or too many rejected events in a row: 25 in bots mode, 80 in fuzz mode), `botException` (a bot threw while deliberating), `setupFailed` (still awaiting players after `EstablishPlayers`), `hung` (an `Execute` call did not return within `--hang-seconds`, default 120), `botHung` (bot deliberation did not return in that time), `undoHung` (`Undo`, `RestoreRecentlyUndone` or `DismissRecentlyUndone` did not return in that time), `memoryLimit`, `botMemoryLimit`, `undoMemoryLimit` (the oracle's heap went over `--memory-mb` during that call; the line has the same keys as the `hung`, `botHung` and `undoHung` line), `replayed` / `replayedWithRejections` (replay mode). |
| `detail` | string or null. For `botException`, the exception type and message. |
| `steps` | Number of step and undo lines. |
| `queries` | As in step lines. Calls made after the last line, before the final dump. |
| `historyCount`, `phase`, `turn`, `winners` | Informational. |
| `hash` | Always present. Hash of the final state. |
| `state` | The final state dump, unless `dumpPolicy.state` is `none`. |

A `hung` end line is different: it has no `historyCount`, `phase`, `turn`, `winners`, `hash` or `state`. Instead it carries the event that never returned, as `queries`, `validate`, `isHost` and `event` with the same meaning as in a step line. A replayer makes the queries, executes the event, and should find that it doesn't terminate either (give it a step budget). Fuzzing does reach real endless loops in the engine, for example in the bidding player sequence when no player may bid. .NET can't stop a running thread, so the oracle leaves the hung thread behind, finishes the other games, writes the summary and exits with code 4. A `botHung` end line has neither `queries` nor `event`.

An `undoHung` end line also lacks the after-state keys. It carries `queries` and `undoKind` (`undo`, `restoreRecentlyUndone` or `dismissRecentlyUndone`), plus `untilEventNr` when `undoKind` is `undo`. These mean the same as on an undo line (section 4): make the queries, then make the call, which should not terminate. Undo re-executes the history with `Execute(false, false)`. Fuzzing has found an undo that never returned, looping in `Bid` via `PlayerSequence.DetermineCurrentPlayer`, although the original game had moved past that event. The cause is not diagnosed yet. One candidate is that the re-execution skips the `GetApplicableEvents` calls of the original run (hazards.md 4.3).

Whether a game hangs is deterministic, but the cut-off is wall-clock time. An `Execute` that is merely very slow (a loop counting to `int.MaxValue`, say) could in principle be recorded as `hung` on a slow machine and finish on a fast one. Use a generous `--hang-seconds` when that matters.

## 6. How to replay a trace

```
game = new Game(header.gameVersion, participation equal to header.participation)
for each line after the header:
    for q in line.queries:
        if q is ["status", p, isPlayer, result]:
            status = GameStatus.DetermineStatus(game, p == null ? null : game.Players[p], isPlayer)   // may change state
            compare the canonical dump of status (or of the exception it threw) with result
        else if q is ["bots", entry]:
            optional: run the bots as the oracle did (see "Bot deliberations") and compare with entry.choice
            or entry.exception. Bots can change the game, so a replayer that skips this can see a hash difference.
        else:   // q is [p, isHost]
            game.GetApplicableEvents(p == null ? null : game.Players[p], isHost)   // result unused; may change state
            compare: it must throw exactly q's exception (third element) if q has one, and must not throw otherwise
    if line.kind == "step":
        evt = deserialize line.event as a GameEvent
        evt.Initialize(game)                      // sets Game, Initiator and Player
        game.Map.neighbour cache: mark stale      // see below
        call evt.Execute(line.validate, line.isHost), catching an escaping exception
        compare result, validation, exception, historyCount, random
    else if line.kind is an undo kind:
        game = the corresponding call (section 4)
    else if line.kind == "end" and line.reason is "hung" or "undoHung":
        execute the event (hung) or make the undo call (undoHung); it must not return within a time budget
    else if line.kind == "end":
        compare hash (and state, if present); stop
    make the GetApplicableEvents calls of section 3, compare with applicable
    if line.hash: dump the state, compare hashes; on mismatch, compare with line.state if present
```

The oracle always executes the **deserialized** event, never the object a bot built. So everything the engine sees is in `event`.

`Treachery.Oracle verify --in FILE|DIR` is this algorithm in C# (`Oracle/Verifier.cs`). It replays traces from their contents alone and compares `result`, `validation`, `exception`, `eventAfter`, `historyCount`, `random`, `typeLists`, `applicable`, `hash` and `state` on every line. The sample corpus passes it, which shows a trace carries everything a replayer needs. Use it as a reference when the Rust replayer and a trace disagree.

**Neighbour cache.** `Map.FindNeighbours` caches results until `History.Count` changes (hazards.md 6.1). Bots and the oracle's own queries fill that cache between events, which a replayer can't reproduce. So before every `Execute` the oracle sets `Map._neighbourCacheTimestamp` to -1, which makes the next `FindNeighbours` call clear the cache. The replayer must do the same. Within one `Execute` the cache behaves exactly as in C#, stale results included.

## 7. Canonical state dump

The dump is a JSON rendering of the `Game` object graph, made by reflection over every instance field. It has no whitespace. Two engines in the same state produce byte-identical dumps.

### 7.1 Objects

An object is written as:

```json
{"$type":"<type name>","<field>":<value>,"<field>":<value>,...}
```

- **Which fields:** every instance field of the object's runtime type and of each base class, up to but not including `System.Object` / `System.ValueType`. Public and non-public, including compiler-generated backing fields. Static fields are never included.
- **Field names:** the CLR field name, with two normalizations. `<X>k__BackingField` (auto-property backing field, also used by the `field` keyword) becomes `X`. `<x>P` (a captured primary-constructor parameter) becomes `x`. Other names are kept as is, for example `_played` or `Messages`.
- **Order:** `"$type"` first, then the fields sorted by name with ordinal (UTF-16 code unit) comparison. All names are ASCII. So `"$territories"` sorts before `"Arrakeen"`, which sorts before `"_locations"`, which sorts before `"field"`.
- **Type names:** `Type.Name` without namespace or declaring type (`LocationFetcher`, not `Map.LocationFetcher`). Generic types drop the arity suffix and list their arguments: `Deck<TreacheryCard>`, `Timer<MainPhase>`, `Dictionary<Faction,Int32>`. Arrays append `[]`.
- **Structs** (value types) are objects too.
- **Aliasing is not represented.** An object reachable through two fields is written out in full at both places, except for the reference rules in 7.2. If one engine shares an object that the other copies, the difference shows up only once a later mutation goes through one of the two paths.
- **Cycles:** if an object is reached again while it is still being written, `{"$cycle":"<type name>"}` is written instead. None occurs in practice.

### 7.2 Things written as references

| Value | Written as | Where it is expanded |
|---|---|---|
| The dumped `Game` | `{"$ref":"Game"}` | It is the root. |
| Any other `Game` | `{"$ref":"OtherGame"}` | Nowhere. Happens after undo, when events still point at the other game. |
| The dumped game's `Map` | `{"$ref":"Map"}` | At `Game.Map`. |
| Any other `Map` | `{"$ref":"OtherMap"}` | Nowhere. |
| A `Player` in `Game.Players` | `{"$ref":"Player","index":i}` | Inside the `Players` list of the root game. A `Player` that is not in `Game.Players` is expanded where it is found. |
| `Leader` | `{"$ref":"Leader","id":1001}` | Nowhere: static data in `LeaderManager.Leaders`. |
| `Messiah` | `{"$ref":"Messiah","id":1000}` | Nowhere: `LeaderManager.Messiah`. |
| `TreacheryCard` | `{"$ref":"TreacheryCard","id":17}` | Nowhere: static data in `TreacheryCardManager`. |
| `Location` and subclasses (`Homeworld`, `DiscoveredLocation`, `HiddenMobileStronghold`) | `{"$ref":"Location","id":42}` | In `Map._locations`. |
| `Territory` | `{"$ref":"Territory","id":7}` | In `Map.$territories`. |
| `Game.Random`'s `LoggedRandom` | `{"$ref":"GameRandom"}` | At `Game.Random`, as `{"$type":"LoggedRandom","count":n}`. |
| Any other `LoggedRandom` | `{"$type":"LoggedRandom","count":n}` | Where found. |

If a `Leader`, `Messiah` or `TreacheryCard` is not the static instance with that id, or a `Location` or `Territory` is not the one in the dumped game's map, the reference gets an extra `"foreign":true`. This has not been seen. It would mean an engine created a copy where C# shares the object.

Ids are unique per kind. The oracle checks at startup that `Map._locations[i].Id == i`, that territory ids are unique, and that hero and card ids are unique.

**The map.** `Game.Map` is per game and partly mutable (attached locations change neighbour lists). It is expanded once, with two special rules:

- `_locations` is the list of all locations, each expanded as an object, in list order. Inside them, other locations and territories are references.
- A synthetic field `$territories` lists every distinct `Territory` object, expanded, in the order they first appear as `_locations[i].Territory`.

### 7.3 Primitive values

| C# type | JSON |
|---|---|
| `null` | `null` |
| `bool` | `true` / `false` |
| `int`, `long`, `short`, `sbyte`, `uint`, `ulong`, `ushort`, `byte` | decimal number |
| `float`, `double` | number, formatted by .NET `ToString("R", CultureInfo.InvariantCulture)`: the shortest text that round-trips, for example `2.5`, `3`, `1E-05`, `1E+21`, `-0`. NaN and infinities are strings: `"NaN"`, `"Infinity"`, `"-Infinity"`. |
| `decimal` | string, `ToString(CultureInfo.InvariantCulture)` |
| `char` | one-character string |
| `string` | string (escaping below) |
| enum | string `"<name>(<number>)"`. `<name>` is .NET `Enum.ToString()`. `<number>` is the underlying integer in decimal. For an undefined value the name is the number itself (`"99(99)"`). For combinations of a `[Flags]` enum it is the .NET list form (`"Cross, ToHomeworld(3)"`). Where several names share a value, .NET picks one. `rust/traces/sample/enums.json` (written by `Treachery.Oracle enums`) maps each enum's full type name to `{"flags": bool, "values": [[number, name used, [all names with that number]], …]}`, in `Enum.GetValues` order. |
| `DateTimeOffset` | string, `ToString("o")`, for example `"2024-01-01T12:00:50.0000000+00:00"` |
| `DateTime` | string, `ToString("o")` |
| `TimeSpan` | number of ticks (100 ns units) |
| `System.Type` | `{"$clrType":"<type name>"}` |

**String escaping.** `"` becomes `\"` and `\` becomes `\\`. Backspace, form feed, newline, carriage return and tab become `\b`, `\f`, `\n`, `\r`, `\t`. Other code units below U+0020, and unpaired surrogates, become `\u` and four lowercase hex digits. Everything else, including `/`, U+007F and all non-ASCII characters, is written as raw UTF-8.

### 7.4 Exceptions

An `Exception` object anywhere in the state or in a message (the "Game Error" message keeps one) is written as:

```json
{"$type":"$Exception","message":"<Exception.Message>","type":"<full .NET type name>"}
```

### 7.5 Collections

- Arrays and the `System.Collections*` types that aren't dictionaries (`List<T>`, `HashSet<T>`, `LinkedList<T>`, `Queue<T>`, `Stack<T>`, …) are JSON arrays in enumeration order.
- Dictionaries are arrays of `[key, value]` pairs in enumeration order.
- Enumeration order is meaningful and is never sorted. For `Dictionary` and `HashSet` it is .NET's slot order, which after a removal is not insertion order (hazards.md 3.1).
- Internal fields of these collections (capacity, version, buckets) are not written.
- Other `IEnumerable` types from `System.*`, in practice lazy LINQ queries kept inside log messages, are enumerated when the dump is made and written as arrays. If enumeration throws, the array ends with `{"$enumerationError":{"message":…,"type":…}}`.
- A `System.*` type that is none of the above and not a `KeyValuePair` or tuple would be written as `{"$unsupported":"<type name>"}`, and the oracle prints a warning. None occurs.

## 8. What the dump leaves out, and why

| Field | What is written | Why |
|---|---|---|
| `Game.History` | `{"$count":n}` | The events are already in the trace. |
| `Game.PreviousGameState` | `{"$present":true/false}` | It is a whole other game (the one before an undo). Its state was dumped on earlier lines. |
| `Message.Nr` | omitted | Comes from a process-wide static counter (`Message.Counter`), so it depends on everything else the process did, including other games on other threads. Nothing in Shared or Bots reads it. |
| `LeaderState.TimeOfDeath` | omitted | Comes from a process-wide static counter (`LeaderState.moment`). Nothing in Shared or Bots reads it; only the client sorts by it. |
| `Map._neighbourCache`, `Map._neighbourCacheTimestamp` | omitted | A cache whose contents depend on who queried it between events (bots, the UI). The oracle marks it stale before every event (section 6), and its stale-result effects within an event show up in the rest of the state. |
| `LoggedRandom._supply` (the `System.Random`) | omitted | Fully determined by the seed and the draw count: every call is `Next(int)`, which takes exactly one sample from the seeded `Net5CompatSeedImpl`. The seed is in the `EstablishPlayers` event and the count is written. |
| Internals of BCL collections | contents instead | See 7.5. |
| Exception stack traces | omitted | They contain file paths and line numbers that Rust can't reproduce. |
| Static fields | omitted | Static data (`LeaderManager`, `TreacheryCardManager`) is referenced by id. The two mutable static counters are covered above. |

Everything else is included: reports and their messages (`ReportList`, `_preGameReport`), the payment and milestone lists, timers, deals, event objects held in game state (with their private fields), the participation object, and so on.

## 9. How runs are seeded

`bots` and `fuzz` games are independent of each other and of thread scheduling. Game `i` of a run with `--seed S` is fully determined by `(mode, S, i)`, so `--start` and `--games` can split a run into pieces and the traces are byte-identical.

- `gameSeed = DeriveSeed(S, i, c)` with `c = 0x424F5453` for bots and `0x46555A5A` for fuzz.
- The oracle's choices (configuration, host style, bot order shuffles, event times, fuzz decisions) come from `new System.Random(gameSeed)`. The fuzzer's value generator uses `new System.Random(DeriveSeed(gameSeed, 1, 7))`.
- The engine seed is drawn from the game's `System.Random` and stored in the `EstablishPlayers` event.
- The bot for faction `f` is created on first use, with `new ClassicBot(game, player, BotParameters.GetDefaultParameters(f), new LoggedRandom(DeriveSeed(gameSeed, (int)f, 0x424F54)))`. One bot per faction, kept for the whole game and rebound to the current player with `SetGameAndPlayer`, like the server does. The step's `bot.seed` repeats the seed.

`DeriveSeed(a, b, c)` (in `GameRunner.cs`) is SplitMix64 over the three inputs, returning a non-negative int. Its exact definition matters only to reproduce the oracle's own choices, not to replay a trace.

Event times start at 2024-01-01T12:00:00+00:00 and advance 1 to 119 seconds per step, with a 2% chance of a 2 to 3 hour gap, so the engine's phase timers (`Game.Timers`) see both short and long gaps. Fuzz events sometimes have the default time, which the engine treats as "no time".

## 10. Summary files

Each bots or fuzz run also writes `summary-<mode>-<seed>-<start>.json`: the count of each end reason, and per game the file name, end reason, detail, step count, history count, game version, player count and host style. Stalled games are listed there. It contains no timings, so it is deterministic too.
