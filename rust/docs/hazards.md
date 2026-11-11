# C# to Rust hazards

> Helper names used below may differ from the real compat API. See the "dotnet-compat helpers" section of engine-conventions.md for the mapping.

This is the list of places where a straightforward Rust translation of `Shared` or `Bots` would behave differently from the C#. Read the summary and the top-ten list once. Then, before porting a file, search this document for that file's name.

**Conventions**

- Paths are relative to `treachery-online/treachery-online/`. `Bots/X.cs` means `Bots/Treachery.Bots/Classic/X.cs`.
- Line numbers are from commit `7bc78bd`. If they drift, the quoted code or the grep command will find the site again.
- "Throws" means a .NET exception. `GameEvent.Execute` usually catches it (section 1).
- Runtime behaviour marked **(probed)** was checked on .NET 10 with a scratch program. The results are in the [appendix](#appendix-probed-net-10-behaviour).
- The oracle's state dumper (`Oracle/Canon/StateDumper.cs`) already omits `Message.Nr`, `LeaderState.TimeOfDeath`, `Map._neighbourCache*` and `LoggedRandom._supply`, and it dumps `History` as a count. Those fields still matter for behaviour even though they are not compared.

## Summary

| § | Category | Hazards | Worst outcome if missed |
|---|---|---|---|
| 1 | [Exceptions and partial execution](#1-exceptions-and-partial-execution) | 9 | Wrong state after a fuzzed event; a Rust panic where C# returns a message |
| 2 | [Randomness](#2-randomness) | 8 | Every later draw in the game diverges |
| 3 | [Dictionary and HashSet order](#3-dictionary-and-hashset-order) | 6 | Wrong terror token removed, wrong spice order, wrong fights list |
| 4 | [Evaluation order, laziness, side-effecting reads](#4-evaluation-order-laziness-and-side-effecting-reads) | 7 | Different state depending on whether validation ran |
| 5 | [Shared references and mutable events](#5-shared-references-and-mutable-event-objects) | 5 | Old-version faction decks differ; event contents change after execution |
| 6 | [Caches and static state](#6-caches-and-static-state) | 5 | Stale movement results reproduced in C# but not in Rust |
| 7 | [Equality and identity](#7-equality-and-identity) | 6 | `Contains`/`Remove` hit the wrong object; cache keys mismatch |
| 8 | [LINQ semantics](#8-linq-semantics) | 7 | Tie-breaks pick a different element; overflow not raised |
| 9 | [Numbers](#9-numbers) | 9 | Off-by-one costs, wrong battle totals, wrong bot thresholds |
| 10 | [Null and nullable values](#10-null-and-nullable-values) | 5 | `null` vs `Faction.None` confusion changes legality |
| 11 | [Strings, enums and messages](#11-strings-enums-and-messages) | 9 | Log and validation text differs |
| 12 | [JSON](#12-json) | 7 | Old savegames fail to load or load differently |
| 13 | [Time](#13-time) | 2 | Timer state differs |
| 14 | [Version gating](#14-version-gating) | 5 | Old games replay differently |
| 15 | [Polymorphism and type checks](#15-polymorphism-and-type-checks) | 5 | Wrong overload or dispatch |
| 16 | [Bots](#16-bots) | 9 | Bot decisions diverge |
| 17 | [Other language details](#17-other-language-details) | 5 | Precedence or ref-parameter mistakes |
| 18 | [C# bugs to preserve](#18-c-bugs-to-preserve) | ~65 sites | "Fixing" a bug breaks equivalence |

## The ten most dangerous hazards

1. **Exceptions are part of the behaviour.** `GameEvent.Execute` catches every exception and keeps whatever state changes happened before the throw (1.1). At least a dozen exception paths are reachable by a fuzzer (1.4). The Rust port must fail at the same statement, with the same exception type and message, and must never panic.
2. **`EstablishPlayers.Execute` has no try/catch.** Its exceptions escape to the caller, it skips the applicability check, and it does not clear `RecentMilestones` (1.2).
3. **.NET `Dictionary` reuses freed slots, last freed first.** After a `Remove`, iteration order is no longer insertion order. This decides which terror token Moritani's Nexus card removes, the order of spice collection, and the order of the fights list (3.1, 3.2).
4. **The movement neighbour cache in `Map` returns stale results** within an event and between validation and execution. C# behaviour depends on whether anyone queried earlier at the same `History.Count` (6.1).
5. **`Game.NextPlayerToBattle` is a getter that mutates `BattleSequence`,** and validation and UI code call it. State depends on how many times it was read (4.3).
6. **`Game.Settings`, `Game.FactionsInPlay` and `Game.AllRules` are the same list objects as the `EstablishPlayers` event's own data,** and later events mutate them. Before version 174 the shuffled faction deck is built from that shared list (5.1).
7. **Event objects are live game state.** `Game.AggressorPlan = this` and similar assignments keep a reference to the event, and `Battle` rewrites its own `Weapon`/`Defense` fields later (5.2).
8. **The bot `Do` chain stops a whole priority tier after one invalid decision,** and bot code relies on exceptions for control flow without rolling back random draws (16.1, 16.2).
9. **Bot tie-break selectors that draw random numbers are evaluated twice per element** by `LowestOrDefault` (16.3). Lazy LINQ chains in bots interleave random draws element by element (16.4).
10. **Float precision is mixed.** Battle strength is `float`, some costs are `double`, `Enumerable.Sum` over floats accumulates in `double`, and bots compare floats against double literals (`0.1f > 0.1` is true). Each site needs the exact C# type (9.1 to 9.4).

---

## 1. Exceptions and partial execution

### 1.1 `GameEvent.Execute` catches everything and keeps partial changes

`Shared/Events/GameEvent.cs:220-255`. The order inside `Execute` is:

1. `if (Game == null) throw` — this check is **outside** the try.
2. Inside the try:
   - If `performValidation`: `IsApplicable(isHost)`, which calls `Game.GetApplicableEvents(...)`, then `Validate()`.
   - `Game.RecentMilestones.Clear()`
   - `Game.PerformPreEventTasks(this)`: timers, clearing recent payments, clearing `RecentlyDiscarded`.
   - `ExecuteConcreteEvent()`
   - `Game.PerformPostEventTasks(...)`: `MainPhaseMiddle`, `History.Add`, `Moments.Add`.
3. `catch (Exception e)` returns `Message.Express("Game Error: ", e.Message, ". Technical description: ", e, ".")`.

So an exception at any point keeps every mutation made before it. `History` and `Moments` are not appended. An exception thrown while building the applicable-events list or inside `Validate` (for example, an id lookup out of range) turns into a "Game Error" result instead of a normal validation message.

The fourth message element is the `Exception` object. Rendering it calls `e.ToString()`, which includes the stack trace and source paths, and Rust cannot reproduce that.

**Porting guidance**

- Every operation that can throw in C# returns `Result<_, DotNetException { kind, message }>` in Rust. Propagate it with `?` up to the `execute` boundary, and keep the mutations made before the error.
- Do not validate up front or reorder statements to avoid the error. The partial mutation is the specified behaviour.
- The oracle should record the exception type and `e.Message` only. Rust compares those two.
- Add `catch_unwind` at the boundary as a backstop. A Rust panic must always count as a mismatch.

### 1.2 `EstablishPlayers` overrides `Execute`

`Shared/Events/EstablishPlayers.cs:162-180`. Compared with the base `Execute`, it:

- has no try/catch, so exceptions reach `Game.TryLoad` or the caller;
- never calls `IsApplicable`, so it can be submitted in any phase;
- does not clear `RecentMilestones`;
- always passes `justEnteredStartOfPhase = true`.

For `Version < 170`, `Validate()` returns null unconditionally (line 67). A fuzzer can therefore re-run setup mid-game. With `MaximumNumberOfPlayers = 0`, `PlayerSequence.cs:206` does `% g.MaximumPlayers` and throws `DivideByZeroException`. A huge value makes `FillSeats` (line 377) loop until out of memory.

**Guidance:** port the override separately, with no catch. The harness should cap player counts in fuzzed `EstablishPlayers` events instead of letting either engine run out of memory.

### 1.3 `Game.TryLoad` wraps messages differently

`Shared/Game.cs:754-782`.

- If `Execute` returns a message, it becomes `Message.Express(e.GetType().Name, "(", nr, "):", message)`.
- If an exception escapes (only possible from `EstablishPlayers` or the `Game` constructor), the result is `Message.Express(e.Message)`, with no type and no prefix.

### 1.4 Exception sites a fuzzer can reach

These were traced by reading the code. Each one leaves partial state behind.

| Site | Trigger | Exception |
|---|---|---|
| `Shared/Map.cs:1913-1918` `LocationFetcher.Find` | Any location id other than -1 that is out of range. 19 event fields use it (`grep -rn "LocationLookup.Find" Shared`). Runs inside `Validate`. | `ArgumentOutOfRangeException` |
| `Shared/Events/PlacementEvent.cs:53-68`, `Shipment.cs:157-175`, `AmbassadorActivated` `_fremenForceLocations`, `FaceDanced` `_targetForceLocations` | Force-location string with a missing `:` or `\|`; a non-numeric part; a repeated location; per-location sums that overflow | `IndexOutOfRangeException`, `FormatException`, `OverflowException`, `ArgumentException` (duplicate key) |
| `Shared/Events/AutomationConfigured.cs:76` | `Action = Delete` with an unknown `AutomationRuleId`. `Validate` returns null. | `InvalidOperationException` "Sequence contains no matching element" |
| `Shared/Events/LoserConcluded.cs:158` → `Game.cs:711/713` `.First` | Karma decision when the player's "Karma" is Clairvoyance-at-Shrine or a Bene Gesserit Useless card. Cards have already been discarded and the log written. | `InvalidOperationException` |
| `Shared/Events/StormDialled.cs:66` | One-player game: `HasBattleWheel` holds the same faction twice (`Game_Setup.cs:79-80`), so `Dials[1]` is out of range | `ArgumentOutOfRangeException` |
| `Shared/Events/CardsDetermined.cs:53` → `Game_Setup.cs:337` | All cards in `RicheseCards`; dealing later draws from an empty deck | `InvalidOperationException` "Cannot draw from an empty deck" |
| `Shared/Events/Shipment.cs:277`, `Donated.cs:120` | `Math.Abs(int.MinValue)` from a negative amount | `OverflowException` |
| `Shared/Events/PerformHmsPlacement.cs:51` | Host with no faction: `p.Faction` on null | `NullReferenceException` |
| `Shared/Events/CardTraded.cs:56` | No ally: `Player.AlliedPlayer!` | `NullReferenceException` |
| `Shared/Events/ClairVoyancePlayed.cs:57-92`, `Model/Deal.cs:41` | Free-text parameters parsed with `int.Parse`, `Enum.Parse`, `float.Parse` | `FormatException`, `ArgumentException`, `ArgumentNullException` |
| `Shared/Events/AcceptOrCancelTleilaxuRevival.cs:63`, `SetShipmentPermission.cs:65`, `AmbassadorPlaced.cs:85`, `Game.cs:738` (`RecentlyDiscarded.Add`) | Duplicate key in `Dictionary.Add` | `ArgumentException` "An item with the same key has already been added. Key: …" |
| `Shared/Events/Retreat.cs:66` | The battle plan has no hero: `plan.Hero.ValueInCombatAgainst` | `NullReferenceException` |
| `Shared/Game_Move.cs:874-875` | Intrusion queue empty or wrong type. Applicability is by phase, not queue contents | `ArgumentException` |
| `Shared/Model/PlayerSequence.cs:95` | No player may get a turn (all hands full) | `Exception` "Could not find a player after {Faction}" |

Find all `throw` statements with `grep -rnE '\bthrow\b' Shared --include=*.cs`. There are 41. The full table, with exact messages, came from the exceptions research. The ones not listed above guard invariants that only break if earlier state is already inconsistent.

### 1.5 Throwing collection operations

| Operation | Count | .NET behaviour |
|---|---|---|
| `.First(` | Shared 16, Bots 17 | "Sequence contains no elements" / "…no matching element" |
| `.Last(` | Shared 1 (`Game_Setup.cs:80`), Bots 1 | same |
| `.SingleOrDefault(` | Shared 8, Bots 1 | "Sequence contains more than one matching element" |
| LINQ `Max`/`Min` on an empty `int` sequence | 8 in Shared, all guarded | throws; on `int?` or a reference type it returns null **(probed)** |
| `Deck.Draw()` | Shared 45 | "Cannot draw from an empty deck" (`Model/Deck.cs:55`) |
| `List[i]` out of range | many | `ArgumentOutOfRangeException` "Index was out of range. Must be non-negative and less than the size of the collection. (Parameter 'index')" |
| `array[i]` out of range | `Split` results | `IndexOutOfRangeException` "Index was outside the bounds of the array." |
| `dict[key]` missing | `LeaderState[h]` and others | `KeyNotFoundException` "The given key '{key}' was not present in the dictionary." |
| `Dictionary.Add` duplicate | see 1.4 | `ArgumentException` "…Key: {key}" |
| `!.` null-forgiving dereference | Shared 183, Bots 13 | `NullReferenceException` "Object reference not set to an instance of an object." |

`Deck.Draw` sites that can plausibly hit an empty deck: the traitor deck (`Game_Battle.cs:329/333`, `Game_Contemplate.cs:81/98`, `NexusPlayed.cs:500/531/574/575`, `FaceDanced.cs:185`, `FaceDancerReplaced.cs:54`, `MulliganPerformed.cs:67`), the skill deck (`Game_Revival.cs:43-44`), custom treachery decks (`Game_Setup.cs:337/339`), ambassadors (`Game_Setup.cs:329`), faction assignment (`Game_Setup.cs:43`), and the spice deck when both discard piles are empty (`Game_Blow.cs:147`).

`LeaderState[h]` is risky because `HeroFetcher.Find` (`LeaderManager.cs:126-137`) returns *any* treachery card for ids below the leader range. An event's hero field can therefore hold a card that has no `LeaderState` entry.

Messages that embed a key's `ToString()` depend on `Message.DefaultDescriber`. With no describer, the key prints as its CLR type name (`Treachery.Shared.Location`) or, for enums, its member name. The oracle must keep `DefaultDescriber` null, and Rust must produce the same names.

**Guidance:** write one fallible helper per operation in `dotnet-compat` that returns the exact .NET exception kind and message. Use them for every site in the table. Never use `unwrap`, `expect` or slice indexing in engine code on values that can come from events.

### 1.6 Int arithmetic wraps, but some operations throw

C# `int` arithmetic is unchecked: no project sets `CheckForOverflowUnderflow`, and there are no `checked` blocks. These throw instead **(probed)**:

- LINQ `Sum` over ints: "Arithmetic operation resulted in an overflow." Shared has 26 `.Sum(` sites and Bots has 29. Client-controlled ones include `PlacementEvent.cs:75/78/90/94`, `Shipment.cs:242/248`, `PerformSetup.cs:38/41`, `PerformFremenSetup.cs:31/34`, `FaceDanced.cs:55/58/172`, `AmbassadorActivated.cs:91-97`, `EmperorBidSupport.cs:51` and `StormDialled.cs:68`.
- `Math.Abs(int.MinValue)`: "Negating the minimum value of a twos complement number is invalid."
- `x / 0` and `x % 0`: `DivideByZeroException` "Attempted to divide by zero."
- `int.MinValue / -1` and `int.MinValue % -1`: `OverflowException`.

Validation sometimes depends on wrapping. `Shipment.cs:224` checks `ForceAmount + SmuggledAmount > ForcesInReserve`, and with `int.MaxValue + 1` the sum wraps negative, so the check passes. `Shipment.cs:200`, `Bid.cs:46` (`TotalAmount`) and `ResourcesTransferred.cs:38` (upper bound only) behave the same way. `Battalion.TotalAmountOfForces` is a wrapping `+` whose result then feeds a checked `Sum`.

**Guidance:**
- Use `wrapping_add`/`wrapping_sub`/`wrapping_mul`, or a `CsInt` newtype, for every C# `int` operation.
- Port `Sum` as a checked sum that raises `OverflowException`.
- Port `Math.Abs`, `/` and `%` with checked variants that raise the .NET exceptions.
- Turn on `overflow-checks = true` in all profiles, so any operation that was not ported this way panics and the differ reports it.

### 1.7 Switch expressions without a default arm

A C# `switch` expression with no matching arm throws `SwitchExpressionException`, which `Execute` catches. A Rust `match` must have an explicit arm that raises the same error wherever the C# switch has no `_` arm. Check this per switch while porting.

### 1.8 `Player` can be null or stale inside an event

`GameEvent.Initialize` (`GameEvent.cs:203-213`) assigns `Player` only when `game.GetPlayer(initiator)` finds one. If it doesn't, `Player` keeps its previous value, which is null for a fresh event or the old game's player for an event re-initialised during `Undo`.

`GetPlayer(Faction.None)` returns the first player whose faction is still `None` (`Game_Participation.cs:22`). Before factions are assigned, an initiator of `None` therefore maps to an arbitrary seated player.

**Guidance:** model `player` as `Option<PlayerIndex>` and reproduce the "only assign if found" rule.

### 1.9 Loops whose bound comes from the client

- `EstablishPlayers.cs:377/388`: the player-count loop (1.2).
- `Revival.cs:187/194/418/424` and `Game_Revival.cs:149/155`: they count up to a bound one step at a time. Port them as loops, or prove a closed form gives the same result under wrapping arithmetic.
- `Game_Blow.cs:57`: draws until a spice-blow card appears. With none left, the deck runs out and throws.

---

## 2. Randomness

### 2.1 Every engine draw goes through one seeded `System.Random`

`Shared/LoggedRandom.cs`. `Game.Random` is created once, at `EstablishPlayers.cs:190`, as `new LoggedRandom(Seed)`, where `Seed` is a field of the event. Every `Deck` keeps a reference to the same `LoggedRandom` (`Deck.Random`).

- `new Random(seed)` uses the legacy seeded algorithm (`Net5CompatSeedImpl`, Knuth subtractive). `Next(max)` is `(int)(Sample() * max)`.
- `Next(0)` returns 0 **and consumes a sample** **(probed)**. `Next(-1)` throws `ArgumentOutOfRangeException` **(probed)**.
- `LoggedRandom.Next` does `_count++` *before* calling `_supply.Next`, so the count goes up even when `Next` throws.
- `_count` is a private field and is part of the dumped state. The dumper omits `_supply`.

**Guidance:** use `dotnet_compat::NetRandom` and a `LoggedRandom` wrapper with the same increment-then-call order. Store one random generator on the game, and pass it to deck operations. Don't give each deck its own.

### 2.2 Draw sites in Shared

Find them with:

```
grep -rnE 'Random|Shuffle\(' Shared --include=*.cs | grep -v '/obj/'
```

- **`Deck.Shuffle()`** (`Model/Deck.cs:61-71`) is Fisher–Yates from the end. It draws exactly `count - 1` numbers, `Next(n + 1)` for `n = count-1` down to `1`. Decks of 0 or 1 items draw nothing.
- **`RandomOrDefault(LoggedRandom)`** (`ExtensionMethods.cs:88-99`) draws **only when there are 2 or more items**.
- **`Game.DetermineLaterStormWithStormDeck`** (`Game_Storm.cs:90`) is `Next(6) + 1`.

Shuffle sites:

| Area | Sites |
|---|---|
| Setup | `EstablishPlayers.cs:233-240` (spice deck; treachery deck unless `CustomDecks`), `:296` (Nexus), `:361/367` (discovery tokens), `:386` (FillSeats, `Version <= 125`); `Game_Setup.cs:39` (factions), `:58` (seats), `:131` (traitors), `:158`, `:175` (skills), `:326` (ambassadors) |
| Card flow | `Game_Bidding.cs:395` (reshuffle), `Bid.cs:248` (**only when `Version >= 131`**), `IxianRemovedCardFromAuction.cs:63`, `IxianSelectedStartingCard.cs:56`, `CardsDetermined.cs:65`, `DiscardedSearched.cs:71`, `KarmaHandSwapInitiated.cs:58` |
| Spice | `Game_Blow.cs:174`, `:462` |
| Traitors and face dancers | `Game_Battle.cs:347`, `MulliganPerformed.cs:50`, `NexusPlayed.cs:347/534`, `FaceDanced.cs:183`, `BattleConcluded.cs:168`, `FaceDancerReplaced.cs:52` |
| Battle | `Game_Battle.cs:428` (audit), `:1069` (Harkonnen victim), `ResidualPlayed.cs:48` |
| Other | `Game_Revival.cs:41` (skills), `SkillAssigned.cs:80`, `NexusCardDrawn.cs:70`, `StormDialled.cs:128` (tech tokens) |

`RandomOrDefault(Game.Random!)` sites: `Game_Alliances.cs:61/63`, `Game_Setup.cs:91`, `AmbassadorActivated.cs:405`, `NexusPlayed.cs:423/472/646`, `TerrorRevealed.cs:175/195/198/231`.

### 2.3 What decides which item a draw picks

- **Deck contents and order before the shuffle.**
  - `Deck(IEnumerable)` copies its source in order when constructed.
  - `PutOnTop(IEnumerable)` inserts each item at index 0, which **reverses** the input (`Deck.cs:36`). `EstablishPlayers.cs:308` builds the spice deck this way.
  - `NexusPlayed.cs:533` adds `RevealedFaceDancers`, a `HashSet`, to the traitor deck before shuffling, so set iteration order matters.
- **Source order for `RandomOrDefault`.** `NexusPlayed.cs:472` picks from `Game.TerrorIn(territory)`, which iterates the `TerrorOnPlanet` dictionary. That dictionary has removals, so the pick depends on .NET slot order (3.1).
- **Whether the draw happens at all.** This depends on the item count (2.1), on version checks (`Bid.cs:248`), on rules (`CustomDecks`), and on `?.` short-circuits. For example, `GetPlayer(Faction.Harkonnen)?.Traitors.Add(TraitorDeck!.Draw())` (`Game_Contemplate.cs:81`) does not draw when Harkonnen is absent.

### 2.4 Unseeded randomness

| Source | Used by | Affects game state? |
|---|---|---|
| `ExtensionMethods.Random` (`ExtensionMethods.cs:70`, static `new Random()`) behind `RandomOrDefault()`, the selector-only `HighestOrDefault`/`LowestOrDefault`/`OneOf*NOrDefault`, and `TakeRandomN` | No caller in Shared or Bots. `TakeRandomN` is used by `Test/Tests.cs:497`. | No. Don't port these overloads into the engine. |
| `Deck<T>.Randomize` (`Deck.cs:74`, `new LoggedRandom()`) | `Server/GameHub_GameEvents.cs:329` orders bots | Indirectly: it changes which bot acts first on the server. Not engine state. |
| Bot `_random = random ?? new LoggedRandom()` (`Bots/Bot.cs:300`) | All bot decisions | Bot decisions only. No current caller injects a seed: `Server/GameHub_GameEvents.cs:388`, `Test/Tests.cs:516`, `Client.Test/BotDriver.cs:19`. Bot traces need an injected seed, which is a C# hook to record in `csharp-hooks.md`. |

There is no use of `DateTime` or `Guid` as a source of randomness in Shared.

### 2.5 Short-circuits and evaluation order around draws

Rust's `&&`, `||` and `?:`-as-`if` short-circuit the same way C# does, and both evaluate function arguments left to right. Draw order only changes if the port restructures code. The cases where restructuring is tempting:

- A `?.` call whose argument draws (2.3).
- `Enter(cond1, phase, cond2(), …)` evaluates every argument before the call, including side-effecting conditions (4.2).
- Object initialisers evaluate in source order, for example `{ AuctionType = D(1,2)…, Direction = D(1,2)… }` (`Bots/Bot_FactionSpecific.cs:1182/1206`).
- A nested `D(1, D(1,2) == 1 ? a : b)` draws the inner number first (`Bots/Bot_FactionSpecific.cs:349`).

### 2.6 No lazy query in Shared calls `Random`

`RandomOrDefault` converts its source to an array immediately, so the draw happens at the call. Bots are different (16.3, 16.4).

### 2.7 Draw-count parity for decks built from lazy sources

`Deck` constructors take snapshots at construction, so mutations after construction don't affect them. Port the constructor as `collect()` at the same point.

### 2.8 Deck reshuffle side effects

`Game_Bidding.cs:386-396` logs "Shuffled N cards…" with the count taken *before* the discard pile moves into the deck. Keep the log, the move and the shuffle in that order.

---

## 3. Dictionary and HashSet order

### 3.1 How .NET orders entries **(probed)**

- Entries live in an array. Iteration walks it from slot 0 and skips freed slots.
- The hash only selects a bucket. It never affects order.
- `Remove` pushes the freed slot onto a free list. The next `Add` reuses the **most recently freed** slot before appending at the end.
- `Clear` resets everything. Resizing copies the array without compacting, and only happens when there are no free slots.
- `HashSet<T>` uses the same layout.

Consequences:

- Remove-then-add of the *same* key keeps its old position. An `IndexMap` with `shift_remove` + `insert` would move it to the end.
- Removing several keys and adding them back reverses their positions. Probe: remove 1, then 2, from {1,2,3}, then add 9, 8, 7 → iteration order `8, 9, 3, 7`.
- Copying with `ToDictionary` or `new Dictionary(other)` compacts the holes.
- Overwriting an existing key through the indexer while iterating `.Keys` does not throw **(probed)**. Adding or removing does.

**Guidance:** use `NetDictionary`/`NetHashSet` for every dictionary or set reachable from `Game`, `Player` or an event. `std` `HashMap`/`HashSet` are banned in engine code.

### 3.2 Collections that see Remove followed by Add

These need the exact free-list behaviour:

| Field | Key → value | Declared | Removed at |
|---|---|---|---|
| `ResourcesOnPlanet` | Location → int | `Game.cs:65` | `Game.cs:552, 568` |
| `Bids` | Faction → IBid | `Game_Bidding.cs:21` | `Game_Bidding.cs:144`, `BlackMarketBid.cs:58`, each immediately before an Add of the same key |
| `RecentlyDiscarded` | TreacheryCard → Faction | `Game.cs:68` | `DiscardedTaken.cs:66`, and the freed slot is refilled in the same event |
| `TerrorOnPlanet` | TerrorType → Territory | `Game.cs:78` | `TerrorPlanted.cs:118/133`, `TerrorRevealed.cs:165`, `AllianceByTerror.cs:64`, `NexusPlayed.cs:473` |
| `DiscoveriesOnPlanet` | Location → Discovery | `Game.cs:84` | `DiscoveryRevealed.cs:73` |
| `AmbassadorsOnPlanet` | Territory → Ambassador | `Game.cs:85` | `Game_Storm.cs:194`, `Game_Move.cs:246`, `AmbassadorActivated.cs:284` |
| `HomeworldOccupation` | Homeworld → Faction | `Game.cs:88` | `Game_Battle.cs:1120` |
| `StrongholdOwnership` | Location → Faction | `Game_Contemplate.cs:17` | `Game_Contemplate.cs:185, 191` |
| `CapturedLeaders` | Leader → Faction | `Game_Battle.cs:1079` | `Game_Battle.cs:477`, `Game_Move.cs:221` |
| `EarlyRevivalsOffers` | IHero → int | `Game_Revival.cs:25` | `AcceptOrCancelTleilaxuRevival.cs:69`, `Revival.cs:555` |
| `PermittedUseOfAllySpice` / `PermittedUseOfAllyKarma` | Faction → … | `Game_Alliances.cs:27/29` | `Game_Alliances.cs:101-104` |
| `ShipmentPermissions` | Faction → ShipmentPermission | `Game_Move.cs:23` | `SetShipmentPermission.cs:62`: removes several factions, then adds them back in order |
| `DealResponses` | DealOffered → Dictionary | `Game_Deals.cs:20` | `DealOffered.cs:172` |
| `Player.ForcesInLocations` | Location → Battalion | `Player.cs:68` | `Player.cs:124, 131, 248, 314` |
| `Player.KnownCards` (HashSet) | TreacheryCard | `Player.cs:38` | `Game.cs:452` via `UnregisterKnown` (about 10 callers) |
| `Player.RevealedTraitors` (HashSet) | IHero | `Player.cs:42` | `Game_Contemplate.cs:97` |
| `Participation.*` | int keys | `Model/Participation.cs:8-33` | `Game_Participation.cs:107-149` |

Two traps:

- `Player.ChangeForces(loc, 0)` on a missing location does an Add followed by a Remove (`Player.cs:120-125`). That leaves a hole at the end.
- `Player.ForcesToReserves` (`Player.cs:240-248`) Adds to the homeworld first and Removes afterwards.

Collections that are only ever added to or cleared, such as `LeaderState`, the revival counters, `Timers` and `LegacyNames`, would be fine with an insertion-ordered map. Use `NetDictionary` anyway.

### 3.3 Iterations where the order changes an outcome

| Site | Collection | Effect |
|---|---|---|
| `NexusPlayed.cs:472` | `TerrorOnPlanet` via `TerrorIn` | Which terror token is removed (random index into dictionary order) |
| `Game_Collection.cs:128` | `ResourcesOnPlanet` | Collection order, log order, `CollectedResourcesToBeDivided` order, and for `Version < 159` which player the Processing Station thief steals from |
| `EndPhase.cs:411` | `DiscoveriesOnPlanet` | Order of `PendingDiscoveries` |
| `Battle.cs:604` | `OccupyingForcesOnPlanet`, keyed by Players × `ForcesInLocations` order | Fights list order. With the `Version >= 177` Ecaz rule (:609), it decides which fight is skipped and so whether `BattleInitiated` validates (`BattleInitiated.cs:204`) |
| `Game.cs:626` | `HomeworldOccupation` | Log order |
| `Bid.cs:296`, `BlackMarketBid.cs:117` | `Bids` | Silent-auction log order |
| `EmperorBidSupport.cs:52` | `Amounts` (the JSON order) | Log order |
| `NexusPlayed.cs:533` | `RevealedFaceDancers` | Shuffle input order |
| `Game_Other.cs:53` | `ForcesInLocations.FirstOrDefault(...)` | Richese's No-Field location |

Bots additionally iterate `ForcesOnPlanet`, `ResourcesOnPlanet`, `DiscoveriesOnPlanet` and `ShipmentPermissions.Keys` (16.6).

### 3.4 Order-preserving set operations

`Distinct` (Shared 12), `Union` (Shared 7, Bots 11) and `Intersect` (Shared 2) yield items in first-seen order and remove duplicates. `Intersect` follows the first sequence's order. There is no `Except`, `GroupBy`, `ToHashSet` or `ToLookup` in Shared.

Order matters at `Map.cs:115` (`Territories()`), `Game_Move.cs:81`, `DiscoveryEntered.cs:60` and `Game_Move.cs:702`. Use an order-preserving dedupe, never a collect into a set.

Find them all with:

```
grep -rnE --include=*.cs "ToDictionary|Distinct\(|\.Union\(|\.Intersect\(" Shared Bots
```

### 3.5 Clones compact holes

`Game.Clone` (`Game.cs:805-820`) and `Player.Clone` (`Player.cs:720-732`) rebuild some dictionaries with `ToDictionary`, which compacts them, and share others by reference. Only the client's `StoryComponent.razor:25` and `Test/Tests.cs:454` call them. Undo and replay construct a fresh `Game`. Don't port `Clone` unless a client snapshot needs it.

`Player.ForcesOnPlanet` (`Player.cs:70`) is a new `ToDictionary` on every access. Its relative order matches the source.

### 3.6 Dictionaries built from event data

`EmperorBidSupport.Amounts` is assigned directly to `Game.PermittedUseOfEmperorSpice` (`EmperorBidSupport.cs:45`), so its order is the JSON order. `ParseForceLocations` builds a `Dictionary<Location, Battalion>` in string order and throws on duplicates (12.5).

---

## 4. Evaluation order, laziness and side-effecting reads

### 4.1 `foreach` over a lazy `Where` re-checks the filter after each body

`Shared/Events/EndPhase.cs:270`:

```csharp
foreach (var p in Game.Players.Where(p => CharityClaimed.CanBePlayed(Game, p) && Game.IsAutomated(AutomationRuleType.CharityAutoClaim, p)))
    Game.ClaimCharity(p);
```

`ClaimCharity` changes CHOAM's spice. A player later in seat order is tested *after* earlier claims have been paid, so CHOAM can become eligible partway through the loop. A port that collects first would miss that.

**Guidance:** port `foreach (x in src.Where(pred)) body` as an index loop with the predicate checked inside, unless C# calls `ToList`/`ToArray`. Collect exactly where C# snapshots. The snapshot sites are `TraitorsSelected.cs:54`, `Game_Battle.cs:451/465`, `Game_Alliances.cs:68`, `Game_Deals.cs:60`, `Game_Collection.cs:128`, `Game_Contemplate.cs:93/116`, `Game_Move.cs:702`, `AllianceByTerror.cs:62`, `Retreat.cs:88/97` and `FactionTradeOffered.cs:89`.

No current lazy loop in Shared would throw "Collection was modified". Adding or removing during `foreach` over a `List` does throw **(probed)**, and the `Execute` catch would turn it into a Game Error.

### 4.2 Arguments are evaluated before the call, even when unused

`Shared/Game_Move.cs:53-55` and `:743-746`:

```csharp
Enter(JuiceForcesFirstPlayer && CurrentJuice.Initiator != Faction.Guild, Phase.NonGuildShip,
      CheckIfGuildActsOrAutoDelays(), Phase.GuildShip, Phase.NonGuildShip);
```

`CheckIfGuildActsOrAutoDelays()` logs "Guild delay their shipment" even when the first condition already decides the phase. A Rust `if a {…} else if check() {…}` would skip the log.

The `Action` arguments to `Enter` (such as `FinishBattle`) are delegates and stay deferred. Keep each argument's eager or deferred status exactly as in C#.

### 4.3 `NextPlayerToBattle` mutates state when read

`Shared/Game_Battle.cs:1196-1211` loops over `BattleSequence.NextPlayer()`, which appends to `_played`, may clear it, and bumps `_fullyCircled` (`Model/PlayerSequence.cs:128-139`). Both fields are dumped.

Callers:

- `Game_Battle.cs:1102`, `EndPhase.cs:147/367` (execution)
- `Game_Events.cs:346/445` (`GetApplicableEvents`, which `IsApplicable` calls only when `performValidation` is true)
- `Model/GameStatus.cs:482-483/604-605` (status display)

So C# state after an event depends on whether validation ran and whether status was computed.

**Guidance:** make it a `&mut self` method called at exactly the same points. The oracle and the Rust replayer must agree on `performValidation`, and on whether `GameStatus` or `GetApplicableEvents` runs between events. Record this in each trace step.

### 4.4 Lazy `IEnumerable` returns

Shared declares about 100 members that return a lazy query. Callers consume most of them with an operator that stops early (`Contains`, `Any`, `First`), so the predicate never runs on the elements after the first match. A port that returns a `Vec` runs it on all of them, and differs whenever a later element throws (a null card, a null `AlliedPlayer`, an unattached discovered location) or the predicate has side effects. They return `impl Iterator` in Rust. [lazy-queries.md](lazy-queries.md) lists every producer and site, found with Roslyn (`csharp-gen --lazy-queries`).

Some return a live collection directly, for example `Player.LocationsWithAnyForces => ForcesInLocations.Keys`, `CardsOwnedBy`, `TerrorPlanted.ValidTerrorTypes`, `LoserConcluded.CardsLoserMayKeep` and several `Valid*` helpers. No engine caller mutates the result.

`Map.GetResourceCardsInPlay` (`Map.cs:145`) creates new `ResourceCard` objects on each enumeration. It is enumerated once (`EstablishPlayers.cs:308`).

### 4.5 Messages keep live collections

`Message` stores raw objects. The oracle's state dump enumerates any collection inside a message at every step, and `Skin.Describe` (`Model/Skin.cs:303`) enumerates it when rendering. So a message shows the collection's contents at dump time, not at log time, and a lazy query runs again each time. Three sites log a game collection that changes later: `AmbassadorActivated.cs:333` (a hand), `AuditCancelled.cs:89` (`Game.AuditedCards`) and `DealAccepted.cs:152` (`DiscardedTraitors`). Four log a lazy query whose inputs don't change.

[live-lists.md](live-lists.md) lists all 60 sites where a collection can reach a message, found with Roslyn, with the reason the port matches at each. The Rust mechanism is in [engine-conventions.md](engine-conventions.md#collections-in-messages).

### 4.6 `GameEvent.IsValid` and bot validation call `Validate()` again

`Validate()` is called from `Execute`, from `IsValid`, and from `Bots/Bot.cs:236`. It must have no side effects other than the static `Message.Nr` counter. Anything in Validate that reads `NextPlayerToBattle` or the neighbour cache does have side effects (4.3, 6.1).

### 4.7 Log statements whose arguments run code

Bot `LogInfo` returns immediately (`LogBotInfo` is a constant `false`), but C# still evaluates its arguments. `action.GetMessage()` at `Bots/Bot.cs:239/243` runs outside the try. The arguments at `Bots/Bot_Other.cs:595/607-650` and `Bots/Bot_FactionSpecific.cs:380` can throw, and inside a bot `Do` a throw is swallowed and changes control flow. Drop such arguments only after checking they can't throw.

---

## 5. Shared references and mutable event objects

### 5.1 Game-start data shared with the event

`Shared/Events/EstablishPlayers.cs`:

- `:196` `Game.Settings = Settings` (`Version >= 170`) shares the event's own `GameSettings` object.
- `:203` (`Version < 170`) `Game.Settings.AllowedFactionsInPlay = FactionsInPlay`. Here the getter builds a fresh list.
- `:211` `Game.AllRules = Game.Settings.InitialRules` shares that list.
- `:268` `Game.FactionsInPlay = Game.Settings.AllowedFactionsInPlay` makes another reference to the same list.
- `:302` `RemoveClaimedFactions` removes from it.

Later mutations through `Game.FactionsInPlay`:

- `FactionSelected.cs:64` `Remove` (`Version < 172`)
- `FactionTradeOffered.cs:65` `Add` (`Version < 172`)

Each mutation changes the game's two fields *and* the event in `History`. `Game_Setup.cs:36-38` builds the shuffled faction deck from `FactionsInPlay` when `Version < 174`, so copying instead of sharing changes the deck contents and every later random draw.

**Guidance:** model the list as one shared object (`Rc<RefCell<Vec<Faction>>>`, or an arena index) referenced from all three places.

### 5.2 Events stored in game state, then mutated

About 40 sites assign the executing event into game state (`Game.X = this`). Examples: `AggressorPlan`/`DefenderPlan` (`Battle.cs:121`), `CurrentKarmaPrevention`, `CurrentRecruitsPlayed`, `LastShipmentOrMovement`, `CurrentNexusPrescience`, `BattleAboutToStart`, `AggressorTraitorAction`, `CurrentBid`. `Bids`, deal offers, alliance offers, `AutomationRules`, revival requests and Nexus votes also hold event objects.

Some of these events change after execution. `Battle.ActivateDynamicWeapons`/`DeactivateDynamicWeapons` (`Battle.cs:193-243`) rewrites `Weapon`, `Defense`, `OriginalWeapon` and `OriginalDefense` on the plan stored in `Game.AggressorPlan`. That is the same object as in `History`.

Find these sites with:

```
grep -rnE "Game\.[A-Za-z]+ = this;" Shared
```

**Guidance:** store events in an arena (for example, an index into history plus a pending slot) and point game fields at it by handle. Mutating "the plan" must mutate the stored event. `Undo` and replay call `Initialize` and `Execute` again on the same event objects, so mutations made during the first run are present on replay.

### 5.3 Other event-owned collections stored in the game

- `PermittedUseOfEmperorSpice = Amounts` (`EmperorBidSupport.cs:45`)
- `FactionsWithIncreasedRevivalLimits = Factions` (`SetIncreasedRevivalLimits.cs:52`)
- `StoredRecentlyPaid = RecentlyPaid` (`Game.cs:838`), a reference swap

### 5.4 Shared battalion objects

`Game.Forces()` returns a new dictionary whose values are the players' **live** `Battalion` objects. `Player.BattalionIn(location)` returns a **new, detached** battalion when the location is empty (`Player.cs:615-620`). Mutating the result therefore sometimes changes the player's forces and sometimes doesn't.

### 5.5 Nested event execution

`DistransUsed.cs:98` runs `new Bid(...).Execute(false, false)` inside its own execution. This repeats the pre- and post-event tasks and appends that `Bid` to `History` in the middle of the outer event. `Bid.cs:157`'s auto-pass bids skip the pre- and post-event tasks. Port both literally.

---

## 6. Caches and static state

### 6.1 The neighbour cache serves stale results

`Shared/Map.cs:1687-1750`. `FindNeighbours` caches by `(Start, Distance, Faction, IgnoreStorm)` and clears the cache only when `game.History.Count` changes.

- The key leaves out `SectorInStorm`, force obstacles and alliances, though the result depends on them.
- `History.Add` happens after the event executes (`Game.cs:146`). So validation of event N+1, its execution, and any bot, status or UI query in between all share one cache.
- During execution, `MustMoveThroughStorm` (`Game_Move.cs:168-172`) runs for each source location after earlier locations' forces have moved. It can read a result cached by validation.
- The cached `List<Location>` is returned by reference.
- Calls with `checkForceObstacles = false` (`MetheorPlayed.cs:50`) bypass the cache.
- An event that throws doesn't increment `History.Count`, so the next event reuses cache entries made against the partially mutated state.

**Guidance:** port the cache exactly: the same key, the `History.Count` timestamp, the same bypass rule, and returning the same list. Replay with the same validation and query pattern as the oracle (4.3). The dumper omits the cache fields, but their effects show up in state.

### 6.2 Process-wide counters

- `Message.Nr = Counter++` (`Model/Message.cs:16-18`) counts every message built in the process, including validation and bot messages.
- `LeaderState.TimeOfDeath = moment++` (`Model/LeaderState.cs:7, 34`) counts kills across every game in the process. Only the client reads it (`Tanks.razor:45/138`, to sort the tanks).

Both are omitted from the dump. Rust can implement them per game or not at all, but a client port that sorts by `TimeOfDeath` must not compare across games.

### 6.3 `Message.DefaultDescriber`

`Model/Message.cs:14` is a public static field. It changes the `ToString()` of Leader, Location, Territory, TreacheryCard, Messiah, ResourceCard and Homeworld, and of `Message` itself, by appending `"*"`. `ToString()` results reach exception messages (1.5) and `ClairVoyancePlayed.Parameter1` (`ClairVoyancePlayed.cs:76/102`). The server and tests set it. The oracle must leave it null.

Audit for the server (package E, 2026-11-08): no engine path the server reaches depends on it. Outside the `ToString` overrides, `Shared` turns these objects into text only in .NET exception messages (dictionary keys in `KeyNotFoundException`/`ArgumentException`, reachable only through engine bugs; they reach state through `GameEvent.Execute`'s "Game Error" message and `Game.TryLoad`'s catch) and in `ClairVoyancePlayed.Parameter1`/`Parameter2`'s setters, which only bots and the client call. Every other interpolation or concatenation in `Shared` formats ints, enums or strings. Messages keep their elements as objects, so the describer only matters where the server renders a `Message` (`ErrorDetails`, mails), which goes through the Skin port. When bots run on the server, `Parameter1`'s setter will write the describer's text plus `"*"` instead of the type name.

### 6.4 Static game data shared across games

`LeaderManager.Leaders`/`Messiah` and `TreacheryCardManager.Items` are static singletons shared by every game. `Leader` has public setters, but nothing in Shared calls them. Treat them as immutable static tables and refer to them by id.

`Map`, `Location` and `Territory` are per game, but `Location.Neighbours` is mutable. `AttachedLocation.PointAt` (`AttachedLocation.cs:24-30`) removes from the old location's list and appends to the new one, which changes the later depth-first search order in `FindNeighbours`. This happens when the Hidden Mobile Stronghold or a discovery attaches.

### 6.5 Derived values stored as state

- `Player.SalusaSecundusHomeworldIsOnLowThreshold` (`Player.cs:167-185`) is recomputed only in `ChangeSpecialForces`. For `Version <= 163` it depends on normal forces too, so it can go stale. Port it as a stored flag updated at the same points.
- The `PlayerSequence` `CurrentPlayer` backing field is used only when `Version <= 117`, but it is still in state.

---

## 7. Equality and identity

### 7.1 What each type's equality means

| Type | `Equals` | `GetHashCode` | `==` |
|---|---|---|---|
| `Location` and subclasses | by `Id` (`Location.cs:46-54`) | `Id` | reference |
| `Territory` | reference | default | reference |
| `Leader` | reference (override commented out, `Leader.cs:58-61`) | `Id` | reference |
| `TreacheryCard` | reference (commented out, `TreacheryCard.cs:46`) | `Id` | reference |
| `Messiah`, `Player`, `ResourceCard`, `Battalion`, events | reference | default | reference |
| `NeighbourCacheKey` (struct, `Map.cs:1687`) | `Start.Id`, `Distance`, `Faction`, `IgnoreStorm` | `HashCode.Combine(Start, Distance, Faction)`, randomised per process | n/a |
| `PointD`, `GameInfo` | value | value | `PointD` overloads it |
| 49 `record`s (Model/Log, `Message.cs:148-199`) | compiler-generated value equality | — | value |

Within one game, every Leader, TreacheryCard, Location and Territory is a single object per id. Events never deserialise object graphs; they store ids and resolve them through the lookups (12.4). So reference equality and id equality agree within a game.

The exception is `ResourceCard`. Each game creates its own instances (`Map.cs:122-143`), including six indistinguishable `ResourceCard(98)` objects that differ only by identity. Give each one an arena index.

### 7.2 Equality-based list operations

`List.Contains`, `Remove`, `IndexOf`, `Distinct` and dictionary lookups use `Equals`. That is reference equality for everything except `Location`. `List.Remove` removes the **first** match. The Leader and card cases work the same with Rust id comparison as long as ids are unique.

### 7.3 `IHero` mixes three types

`Leader`, `Messiah` and `TreacheryCard` (mercenaries) all implement `IHero`. `LeaderState` holds Leaders plus the Messiah. `Traitors`, `FaceDancers`, `RevealedTraitors` and `EarlyRevivalsOffers` can also hold a Cheap Hero card (`Game.cs:1097-1112`).

Ids don't overlap: cards 0–102, Messiah 1000, leaders 1001 and up. `HeroFetcher` relies on that (`LeaderManager.cs:126-137`). Model it as `enum HeroRef { Leader(id), Messiah, Card(id) }`.

`TreacheryCard.IsTraitor(h)` and `IsFaceDancer(h)` return `h is TreacheryCard` (`TreacheryCard.cs:32-38`). That is a type check, so any card matches any card. `Leader`'s versions use reference equality. Callers sometimes swap the receiver (`FaceDanced.cs:88/111`, `FaceDancerRevealed.cs:45`), so port each call with the same receiver.

### 7.4 Reference comparisons

`ReferenceEquals` is used at `Game_Other.cs:57`, `AmbassadorActivated.cs:243` and `BeneGesseritAccompanies.cs:120/147`. `==` on Location, Territory and IHero is a reference comparison everywhere, for example `t == Map.PolarSink.Territory`. `Location.Equals` is used at `Game.cs:910` and `Map.cs:1799/1836`.

`HiddenMobileStronghold`, `Homeworld` and `DiscoveredLocation` each have their own id and territory, so none equals the location it is attached to.

### 7.5 Identity tokens that cross calls

- `DealResponses` is keyed by the `DealOffered` event instance.
- The bot's `VoicePlan.Battle == Game.CurrentBattle` (`Bots/Bot_Battle.cs:165`) compares `BattleInitiated` instances.
- `LegacyNames` is keyed by `Player` object.

In Rust these need stable handles, such as a history index or a player index, not structural comparison.

### 7.6 Cloned territories break `TerrorIn`

`Game.Clone` clones `TerrorOnPlanet` values with `Territory.Clone()` (MemberwiseClone), so `TerrorIn(t)` (`kvp.Value == t`, `Game_Contemplate.cs:436`) never matches in a clone. This only matters if `Clone` is ported.

---

## 8. LINQ semantics

### 8.1 Stable sorting

Shared and Bots never call `List.Sort` or `Array.Sort`. All sorting is `OrderBy`/`OrderByDescending` (Shared 11, Bots 16), which is stable. There is no `ThenBy`. `OrderByDescending` is stable too: tied items keep their source order.

**Guidance:** use a stable `sort_by`. Write descending order as `b.cmp(a)` and never as sort-then-reverse, which would reverse ties. For float keys, use the `float.CompareTo` order (9.4).

### 8.2 The `HighestOrDefault` family

`Shared/ExtensionMethods.cs`. `HighestOrDefault(random, selector)` and `LowestOrDefault(random, selector)`:

1. `ToArray()` the source, running any lazy filters.
2. `Max`/`Min` of `selector(v)` as `IComparable`, through `Comparer<IComparable>.Default`. That calls the selector once per element.
3. `Where(v => selector(v).Equals(best))`, which calls the selector **again** for every element and keeps source order.
4. `RandomOrDefault(random)` draws only if 2 or more elements match.

Boxed `float.Equals` treats NaN as equal to NaN and -0 as equal to +0 **(probed)**.

`OneOfHighestNOrDefault`/`OneOfLowestNOrDefault(random, sel, n)` stable-sort, take `n`, then make one draw. A tie at the boundary is decided by source order, not at random.

**Guidance:** port the helper with its double selector call. This matters when the selector has side effects (16.3).

### 8.3 `Max`/`Min`/`First` on ties and empty sequences

- `Max`/`Min` return the value, not the element.
- `First`/`FirstOrDefault` take the first element in source order.
- `ObjectCounter.Highest` (`Model/ObjectCounter.cs:63-75`) takes the first maximum in insertion order.
- Empty-sequence behaviour is in 1.5.

### 8.4 `Sum`

`int` `Sum` is checked (1.6). `float` `Sum` accumulates in **double** and casts to float at the end **(probed)**: `{16777216f, 1f, 1f}.Sum()` is `16777218`, where a float accumulator gives `16777216`. It is used in bots (`Bots/Bot_Knowledge.cs:605`).

### 8.5 Laziness and multiple enumeration

See 4.1 and 4.4. A lazy query enumerated twice runs its predicates twice. [lazy-queries.md](lazy-queries.md) lists the sites (class `double`); `Game.TraitorsInPlay` re-runs `Players.Select(p => p.Faction)` for each leader, and the Rust port does the same.

### 8.6 `Take`/`Skip`

There are 21 sites. They are deterministic given the source order. `Take` on a lazy `OrderBy` sorts the whole source first.

### 8.7 `Stack.ToList()` order

`Map.FindPath` returns `Stack.ToList()`, which is top-first, so the order is reversed relative to push order (`Map.cs:1778-1895`).

---

## 9. Numbers

### 9.1 `float` versus `double`

Use exactly the C# type at each site. RyuJIT does not fuse multiply-adds and neither does Rust, so the same operations in the same order give the same bits.

- **`float` (`f32`), the `0.5f` sites (Shared 14):**
  - Battle dial and strength: `Battle.cs:246-260, 340-352, 447-507`; `BattleOutcome.AggTotal`/`DefTotal`.
  - Revival cost: `Revival.cs:130, 387-396`.
  - `BattleClaimed.cs:56/58`, `DivideResources.cs:63`, `TerrorRevealed.cs:219`, `Game_Collection.cs:83`, `Game_Battle.cs:710`, `Shipment.cs:295`.
  - `PlayerSequence.cs:206`: `(int)Math.Ceiling((float)SectorInStorm * MaximumPlayers / 18) % MaximumPlayers`.
- **`double` (`f64`):** `Shipment.cs:299-304`, `AmalPlayed.cs:46`, `Revival.cs:176/364/372/380`, `TakeLosses.cs:91`, `Territory.cs:63`, `Battalion.cs:59`.
- **Bots:** almost everything is `f32` (16.7).
- **UI only, skip:** `Model/Skin.cs`, `DefaultSkin.cs`, `Model/Point.cs`, `Model/Utilities.cs` (font scaling) and `GameStatistics.cs`.

`float * int` converts the int to `f32` first. `Math.Ceiling` and `Math.Floor` only take `double`, so a `float` argument is widened exactly first.

### 9.2 Rounding per rule

These formulas assume non-negative inputs. For negative fuzzed inputs, ceiling and `(int)` both round toward zero.

| Rule | Formula | Site |
|---|---|---|
| Ship back to reserves | ⌈n/2⌉ in f32 | `Shipment.cs:295` |
| Normal shipment | n×(1 or 2), halved for a discount, then ⌈·⌉ in f64. The int product can wrap. | `Shipment.cs:299-304` |
| Discounted force revival / Tleilaxu hero discount | ⌈x/2⌉ in f64 | `Revival.cs:176, 364, 372, 380` |
| Emperor-paid ally revival | ⌈f×p_f + s×p_s⌉ in f32 | `Revival.cs:130` |
| Amal | lose ⌈resources/2⌉ in f64 | `AmalPlayed.cs:46` |
| Terror robbery | ⌈resources/2⌉ in f32 | `TerrorRevealed.cs:219` |
| Take half losses | ⌈(f+s)/2⌉. The int add can wrap. | `TakeLosses.cs:91` |
| Battalion halving | ⌈f/2⌉ and ⌈s/2⌉ separately | `Battalion.cs:59` |
| Ecaz battle contribution | ⌊n/2⌋ before version 159, ⌈n/2⌉ from 159, in f32 | `BattleClaimed.cs:56/58`, `Battle.cs:348/352` |
| Collection lost at low threshold | ⌊received/2⌋ (f32, truncation) | `Game_Collection.cs:83` |
| CHOAM battle income | ⌊cost/2⌋ | `Game_Battle.cs:710` |
| Undivided resources | ⌊amount/2⌋ to the first player | `DivideResources.cs:63` |
| Sietch Tabr collection | ⌊opponent dial⌋ | `Game_Battle.cs:382` |
| Middle location | index ⌊count/2⌋ | `Territory.cs:63` |
| Storm movement | `Dials.Sum()` (checked), then `% 18` | `StormDialled.cs:68/79`, `Game_Storm.cs:132` |

No site uses the classic `Math.Ceiling(intExpr / 2)` integer-division bug. Every ceiling multiplies by a `0.5` literal or divides a value that is already floating point.

### 9.3 Casts from float to int

`(int)x` truncates toward zero. Since .NET 9, x64 saturates out-of-range values and maps NaN to 0 **(probed)**, which matches Rust's `as`.

### 9.4 Float comparisons and min/max

- `Math.Max`/`Math.Min` on floats propagate NaN. `Max(-0, +0)` is `+0` and `Min(+0, -0)` is `-0` **(probed)**. Rust's `f32::max` returns the non-NaN operand instead. Bot sites: `Bots/Bot_Shipment.cs:98, 182, 204, 324, 810`.
- `float.CompareTo` (used by `OrderBy` and `IComparable` `Max`) puts NaN first (`NaN.CompareTo(1) == -1`, **probed**) and treats -0 as equal to +0. Rust's `total_cmp` orders -0 before +0.
- A float compared with a double literal is widened to double. `0.1f > 0.1` is **true** **(probed)**. Sites: `Bots/Bot_Battle.cs:1019` (`< 0.8`), `:1021` (`> 0.1`), `Bots/Bot_FactionSpecific.cs:1085` (`0.33 * int`), `Bots/Bot_Deals.cs:185/204` (`Math.Floor(0.1 * R)`, `0.05 * R`).

**Guidance:** add `max_f32`, `min_f32`, `f32_compare` and `float_equals` to `dotnet-compat`. Port `x < 0.8` on a float as `(x as f64) < 0.8`.

### 9.5 Integer types

Shared and Bots use only `int`: no `long`, `short`, `byte`, `uint` or `nint`. The default context is unchecked (1.6).

### 9.6 Modulo

C# `%` takes the sign of the dividend, and so does Rust's `%`. Never use `rem_euclid`. `PlayerSequence.cs:224` has its own positive-modulo helper, `(x % m + m) % m`. `(firstPlayer.Seat + i) % MaximumPlayers` can be negative when `Seat` is -1.

Other `%` sites: `Game.cs:1034/1047`, `Game_Storm.cs:132`, `StormDialled.cs:79`, `LeaderState.cs:20/22`. Bots: `Bot_Other.cs:37/823`, `Bot_Knowledge.cs:252/615`, `Bot_Shipment.cs:504`.

### 9.7 `Math.` calls

Shared and Bots together have 161 `Math.` calls: 113 `Min`, 43 `Max`, 19 `Ceiling`, 4 `Floor`, 5 `Abs` and 1 `Round`. There is no `Pow`, `Sqrt`, `Sign`, `Truncate` or `MathF`. `Max`/`Min` on ints is trivially safe. The five `Abs` sites are `Shipment.cs:277` (twice) and `:299`, `Donated.cs:120` and `KarmaEcazDial.cs:47` (1.6). The only `Round` is `GameStatistics.cs:45` (`AwayFromZero`), which is not game state.

```
grep -rnE --include=*.cs "Math\.[A-Za-z]+" Shared Bots
```

### 9.8 Flags and bit operations

`ShipmentPermission` is the only `[Flags]` enum (`Enumerations.cs:905`). It is used with `&` at `Game_Move.cs:906` and `SetShipmentPermission.cs:73-75`, and with `|` at `Bots/Bot_FactionSpecific.cs:25`. `SetShipmentPermission.Permission` isn't validated, so any `i32` can be stored. Use `struct ShipmentPermission(i32)`.

Boolean `^` appears at `BeneGesseritFlip.cs:70/81` and `Bots/Bot_Battle.cs:39`.

### 9.9 Enum arithmetic and ordering

- `Fight.Id = (int)Faction * 1000 + Territory.Id` (`Model/Fight.cs:18`).
- `Phase` and `MainPhase` are compared with `<`/`>` at `Game.cs:588`, `Game_Events.cs:79/495/497/509/531/638-639` and `NexusPlayed.cs:221/239/407-409`.

Enum values are sparse (Faction 0, 10 … 120; Phase in the tens of thousands; `MainPhase.Collection = 95`). Rust enums need `#[repr(i32)]` with the exact discriminants, compared as integers.

---

## 10. Null and nullable values

### 10.1 `Faction?` null is not `Faction.None`

`BattleWinner`, `BattleLoser` and `CurrentEcazOrAllyFighter` (`Game_Battle.cs:26-30`) start as **null**. They become `Faction.None` only at `EndPhase.cs:390-399`. Until the first battle-phase `EndPhase`, `BattleWinner == Faction.None` is false.

Affected sites: `JuicePlayed.cs:43`, `Game_Battle.cs:410/436/1236`, `Audited.cs:44`, `AuditCancelled.cs:93`, `FaceDanced.cs:83` and `Battle.cs:256/441`. `OwnerOfFlightDiscovery` and `PhaseBeforeCardTrade` work the same way.

**Guidance:** use `Option<Faction>`, and keep `None` distinct from `Some(Faction::None)`.

### 10.2 Lifted operators

- `null > 0` is false. Example: `AlliedPlayer?.OccupyingForcesIn(l) > 0` (`Player.cs:463`).
- `null == false` is false. Example: `GetPlayer(Richese)?.HasRoomForCards == false` (`Bid.cs:287`, `BlackMarketBid.cs:113`).
- `null != Faction.None` is true.
- `Bots/Bot_Other.cs:462`: `p.Resources + p.AlliedPlayer?.Resources > 2` is false when there is no ally.

Port each as an explicit `match` on the `Option`.

### 10.3 `?.` skips evaluating the arguments

`x?.M(arg())` does not evaluate `arg()` when `x` is null. Shared has 128 `?.` and 58 `??`; Bots has 20 and 69. Sites where the skipped argument has side effects: `Game_Contemplate.cs:81` (a deck draw), `Game_Bidding.cs:114`, `BlackMarketBid.cs:101/138` and `Game_LeaderState.cs:30/53`.

### 10.4 Defaults

- Almost every enum has `None = 0`. These have a different member at 0: `ItemAction.Create`, `ShipmentType.Unknown`, `AutomationRuleType.Unknown`, `BattleFactKind.Strength`, `ShipKind.Passed`, `BeneGesseritReactionKind.Accompany`, `NexusAllianceChange.Offered` and `AsideKind.Aside`. `GameListScope` has no member at 0.
- A missing JSON `_xxxId` deserialises to 0, which is a real card, location or territory, not "none" (12.2).
- `string` fields without initialisers can be null after deserialisation, for example `FactionSelected.InitiatorPlayerName` and `BattleConcluded._cardIds`.

### 10.5 String concatenation with null

`"a" + null` gives `"a"`. `string.Join` and `Message.ToString` turn null elements into `""` **(probed)**.

---

## 11. Strings, enums and messages

### 11.1 Messages store objects

`Expression.Elements` is `object?[]` (`Model/Expression.cs:12`). `Report.Messages` is part of game state (`Game.ReportList`). Elements include strings, ints, floats, bools, null, enums, model singletons, `Payment` (whose `Reason` field points back to an event), nested `Message`/`MessagePart`, live or lazy collections (4.5), and `Exception` objects.

Rendering:

- **`Message.ToString()`** (no describer) is `string.Join("", Elements)`. That gives CLR type names such as `Treachery.Shared.Leader`.
- **`Message.ToString(IDescriber)`** goes through `Skin.Describe` (`Model/Skin.cs:260-305`):
  - `null` → `""`; `bool` → `"Yes"`/`"No"`; `Payment` → `"{amount} {Resource label}"`.
  - About 35 type cases look up skin labels. A missing label gives `""`.
  - Any `IEnumerable` except `string` → `", "`-joined, or `"-"` when empty.
  - **Everything else → `value.ToString()`.** That includes ints, floats, records and enums missing from the switch, such as `Nexus` (`Game_Other.cs:166/168`) and `DealType` (`DealAccepted.cs:181`).
- **`Skin.Format`** (`Skin.cs:199-209`) catches any exception from `string.Format` and returns the raw template.

**Guidance:** model elements as a closed Rust enum, port `Skin.Describe` and `Skin.Format` exactly, and have the oracle render with `DefaultSkin.Default` under invariant culture.

### 11.2 Culture

Only the Client project sets `InvariantGlobalization` (`Client/Treachery.Client.csproj:26`). Shared, Server, Bots and Test use the host culture for int and float `ToString()`, `string.Join`, `int.Parse` and `Convert.ToInt32`.

The oracle must pin invariant culture, for example `CultureInfo.DefaultThreadCurrentCulture = InvariantCulture`. Then Rust only has to match invariant output.

### 11.3 Float formatting

Floats appear in log text at `Battle.cs:172` (dial), `Game_Battle.cs:535-536/842` (strength) and `ClairVoyancePlayed.cs:163`. .NET uses shortest round-trip formatting **(probed)**:

| Value | .NET | Rust `{}` |
|---|---|---|
| `1e7f` | `10000000` | `10000000` |
| `1e-5f` | `1E-05` | `0.00001` |
| `1e21` (double) | `1E+21` | `1000000000000000000000` |
| `+∞` | `Infinity` | `inf` |
| NaN | `NaN` | `NaN` |
| `-0f` | `-0` | `-0` |

Battle values are multiples of 0.25, so normal play only needs plain decimals. Fuzzed ClairVoyance values reach the exponent forms. Add `dotnet_compat::fmt::f32_to_string`/`f64_to_string`.

### 11.4 Enum names

Rust needs a `csharp_name(i32)` for each enum. It returns the member name, the decimal number for an undefined value (`((Day)99).ToString()` is `"99"`, **probed**), and for `ShipmentPermission` a `"A, B"` list.

Enum names reach strings through the Skin fallback, exception messages (`Game_Move.cs:519/594/669/875`, `Map.cs:87`, `PlayerSequence.cs:95`), `ClairVoyancePlayed.QuestionParameter1` (written with `ToString()` by `Bots/Bot_Other.cs:590`, read with `Enum.Parse`), and the obsolete `EstablishPlayers._factionsInPlay`.

`GameEvent.GetMessage` uses `GetType().Name` (`GameEvent.cs:259`), and `TryLoad` uses `e.GetType().Name`. Event class names equal their JSON discriminators. Note `SwitchedSkilledLeader` lives in `SwitchSkilledLeader.cs`.

### 11.5 `Enum.GetValues` order

`Enum.GetValues` returns members sorted by value, not in declaration order. Two enums are declared out of value order:

- `Rule`: `BasicTreacheryCards = 10000` and `HasCharityPhase = 10001` come first, and `BotsCannotAlly = 1008` comes before `CustomInitialForcesAndResources = 100`.
- `Ruleset`: `Expansion3BasicGame = 300` comes before `AllExpansionsBasicGame = 180`.

Callers whose order matters:

- `Game_Rules.cs:307` `GetRulesInGroup` fills `Game.Rules`, which is state, and feeds the "House rules:" log.
- `Game_Setup.cs:174` builds the skill deck before a shuffle.
- `Enumerations.GetValuesExceptDefault` (`Enumerations.cs:21`).

**Guidance:** never iterate in declaration order (e.g. strum's `EnumIter`). Generate a `VALUES` array sorted by value.

### 11.6 Parsing strings from events **(probed)**

| Call | Behaviour Rust's `from_str` lacks |
|---|---|
| `Convert.ToInt32(s)` (`GameEvent.cs:276`, force-location strings) | null → 0; `" +12 "` → 12; `""` → `FormatException` "The input string '' was not in a correct format."; out of range → `OverflowException` "Value was either too large or too small for an Int32." |
| `int.Parse(s)` (`ClairVoyancePlayed.cs:57/61/92`, `Deal.cs:41`) | null → `ArgumentNullException` "Value cannot be null. (Parameter 's')"; otherwise like `Convert` |
| `Enum.Parse<T>(s)` (`ClairVoyancePlayed.cs:53/59`, `EstablishPlayers.cs:57`) | Case-sensitive; trims whitespace; `"12345"` gives an undefined value; `"Monday, Tuesday"` is OR'ed together even for non-flags enums; unknown name → `ArgumentException` "Requested value 'Foo' was not found." |
| `float.Parse(s, Invariant)` (`ClairVoyancePlayed.cs:63`) | `"1,5"` → 15 (thousands separator); `"infinity"` accepted; `"1e99"` → `Infinity`, not an error |
| `string.Split(';')` then `Convert` (`IdStringToObjects`) | `"1;;2"` throws on the empty piece. Unknown ids are skipped silently |

Put these in `dotnet_compat::parse`.

### 11.7 String comparison

No code in Shared or Bots sorts by string or uses culture-sensitive `Compare`, `StartsWith`, `EndsWith`, `ToLower` or `ToUpper`. The one `ToLower` is UI font code in `Utilities.cs:107`. String `==` is ordinal. `GameState.cs:41`'s `Replace` is ordinal.

### 11.8 Text quirks to reproduce byte for byte

- `Game_Blow.cs:473-488` `Natural(int)` gives "first" through "tenth", then `count + "th"`: "11th", "21th".
- Typos: "discovert" (`FlightDiscoveryUsed.cs:51`), "put select a" (`RicheseSpecifiesAuction.cs:111`), "A en B" (`Game_Blow.cs:155`), "Discovery were removed" (`:162`).
- Doubled spaces at `Karma.cs:294` and `FlightUsed.cs:61`. Missing spaces at `Shipment.cs:590` and `NexusPlayed.cs:385`.
- Leading spaces passed to `PlayNexusCard`: `Revival.cs:465`, `TakeLosses.cs:108`, `LoserConcluded.cs:139`, `Voice.cs:160`, `Game_Contemplate.cs:53/59/66`.
- Lowercase "cunning" at `FremenRidesMonster.cs:142`.
- `Log(params object?[])` unwraps a single `Message` argument, but `LogIf` does not (`Game.cs:786-792`).

### 11.9 User text with lone surrogates

A JSON string such as `"\ud800"` gives a .NET string with a lone surrogate, which a Rust `String` cannot hold. Either the fuzzer avoids these, or user-text fields use a WTF-16 type.

---

## 12. JSON

All rules below were checked against the fixture saves in `scripts/fixtures/` and with test programs on .NET 10.

### 12.1 Writing

- **Options:** `Utilities.cs:21` is `new JsonSerializerOptions { IncludeFields = true }`. There is no naming policy, no custom converter, no `JsonPropertyName` and no `JsonConstructor` in the repo.
- **Members written:** public properties with a public getter, and public instance fields. Private fields are excluded. The one exception is `Shipment._cunningNoFieldValue` (`[JsonInclude]`, `Shipment.cs:78-79`). Every `_xxxId` field is public.
- **Order:** `$type` first. Then the most-derived class's properties in declaration order, then its fields in declaration order. Then the same for each base class, ending with `GameEvent.Initiator`, `Time`. For example, `Shipment` writes `_toId` (declared second) after all of its properties.
- **Computed properties:** every getter-only property on an event is `[JsonIgnore]`, and `Test/Tests.cs:1035-1052` enforces this.
- **Values:** enums as integers; nulls and defaults always written; compact output.
- **`DateTimeOffset`:** `yyyy-MM-ddTHH:mm:ss`, then `.fffffff` with trailing zeros trimmed (omitted when zero), then `±HH:MM`. A zero offset is `+00:00`, never `Z`.
- **String escaping (default encoder):**
  - All non-ASCII becomes `\uXXXX` with uppercase hex, using UTF-16 surrogate pairs.
  - ASCII `"` `&` `'` `+` `<` `>` `` ` `` and DEL use the `\u00XX` form.
  - `\` → `\\`. `\b \t \n \f \r` use short forms; other control characters use `\u00XX`. `/` is not escaped.
- **Dictionary keys:** enum keys are written as member names; int keys as decimal strings.

### 12.2 Reading

- Property names are case-sensitive. Unknown properties are ignored and lost on re-save. If a key appears twice, the last value wins (serde's derive errors instead).
- The parameterless constructor and field initialisers run first. A missing member keeps its initialiser. Initialisers that matter:
  - `Capture = true` (BattleConcluded)
  - `-1` for `_traitorToReplaceId`, `_newTraitorId`, `_tleilaxuHeroId`, `_locationId`, `_cardId` (Distrans/Donated/RicheseGaveCard), `_karmaCardId` (Bid) and `AutomationRuleId`
  - `MaximumNumberOfPlayers = 6` and `MaximumTurns = 10` (EstablishPlayers)
  - GameSettings `NumberOfPlayers = 6`, `MaximumTurns = 10`, `AutoOpenEmptySeats = true`
  - Participation `BotsSpeed = 1`
- **A missing `_xxxId` is 0, not -1.** That is card, location or territory 0.
- Explicit `null` into `int`, `bool`, an enum or `DateTimeOffset` throws. Into a string, array, `List`, `Dictionary` or object, it stores null. `PlacementEvent.ParseForceLocations` then throws on `.Length` (`PlacementEvent.cs:56`). Shipment's own copy checks for null.
- These throw: `1.0`, `1e0`, `"1"`, a leading zero, or an out-of-range number into `int`; `1` into `bool`; a number into `string`; a string into an enum.
- An undefined enum integer such as `12345` is accepted. Rust enums need an `i32` newtype, or the harness must reject undefined values.
- Comments, trailing commas, single quotes, raw newlines in strings, lone-surrogate escapes, invalid UTF-8, trailing garbage and nesting deeper than 64 all throw.
- Enum dictionary keys are read leniently: case-insensitive names, trimmed, numeric strings accepted. Int keys accept `"+1"` but not `" 1"`.
- A `DateTimeOffset` without an offset is read as **machine-local time**. A date with no time is accepted. Fractional digits past 7 are truncated.

### 12.3 The `$type` discriminator

`GameEvent.cs:12-150` lists 139 `[JsonDerivedType(typeof(X), nameof(X))]` entries. On read:

- `$type` missing or not first → `NotSupportedException` "must specify a type discriminator".
- Unknown, wrong case, or not a string → `JsonException`. For an unknown string id the message is "Read unrecognized type discriminator id 'X'. Path: $._events[2] | LineNumber: 0 | BytePositionInLine: 548.", and Rust reproduces it exactly (`JsonError::unrecognized_discriminator`, `json::locate`), because savegames from before an event was renamed fail this way (`BluePrediction` after the Blue → BeneGesserit rename). .NET reads one more token before it resolves the id, so the position is past the next property name and its colon, or past the `}`; if that next property is `$id`, `$ref`, `$values` or a second `$type`, the metadata error comes first. `tests/golden/event_errors.jsonl` holds the C# messages.
- Duplicate `$type`, or any later `$`-prefixed key such as `$id` → `JsonException`.

serde's internally tagged enums accept the tag anywhere, so write a custom `Deserialize`.

### 12.4 How events store references

- **69 single-id `int` fields** such as `_cardId`, `_heroId`, `_toId` and `_territoryId`. Odd names: `FaceDancerReplaced.dancerId`, `KarmaChoamDiscard._cardToUse`, `TerrorRevealed._sneakAttackTo`. -1 means null.
  - Resolved through `TreacheryCardManager.Lookup` (`TreacheryCardManager.cs:149-156`), `LeaderManager.HeroLookup`/`LeaderLookup` (`LeaderManager.cs:110-140`), and `Map.LocationLookup`/`TerritoryLookup` (`Map.cs:1904-1918`).
  - **`LocationLookup` indexes a list and throws out of range;** the others return null for unknown ids.
- **7 `;`-joined id strings**, parsed by `IdStringToObjects` (`GameEvent.cs:271-289`): `AmbassadorActivated._choamCardIds`, `BattleConcluded._cardIds` (nullable, no initialiser), `CardsDetermined._treacheryCardIds`/`_richeseCardIds`, `KarmaChoamDiscard._cardIds`, `KarmaHandSwap._cardIds` and `LoserConcluded._forcedKeptOrDiscardedCardIds`.
- **Force-location strings** `"locId:forces|special,…"`: `_forceLocations` (PlacementEvent, plus Shipment's private copy at `Shipment.cs:152-175`), `AmbassadorActivated._fremenForceLocations` and `FaceDanced._targetForceLocations`. They are re-parsed on every property read, so each read gives fresh `Battalion` objects in string order.
- **Other encodings:** `EstablishPlayers._players` is a `>`-joined string. `_factionsInPlay` (obsolete) is a `,`-joined list of enum names.
- **Nested data:** `EstablishPlayers.Settings` (GameSettings) and `ApplicableRules` (obsolete `Rule[]`, still written); `EmperorBidSupport.Amounts` (`Dictionary<Faction,int>` with name keys, null when absent); the arrays `DealOffered.To`, `SetIncreasedRevivalLimits.Factions` and `SetShipmentPermission.Factions`.
- **Fields versus properties:** `KarmaHmsMovement.Passed` and the `Target`/`SpecialForce`/`Skill` members of ChoamRemoveForce, CardTraded and SkillAssigned are fields, which changes where they appear in the output.
- **Written by the engine during execution:** `AutomationConfigured.AutomationRuleId` (`AutomationConfigured.cs:66`) and `Initiator` (through `Initialize`).

List event members with:

```
grep -nE "^\s*(\[Json\w+\]\s*)?public [^(]*(;|\{ *get)" Shared/Events/*.cs
```

### 12.5 Loading legacy Newtonsoft saves

`Model/GameState.cs:27-42` and `Utilities.cs:25-90`:

1. **For every format:** an ordinal `Replace("Treachery.online", "Treachery")` over the whole text. This also changes user text such as deal text and player names.
2. The legacy path applies only if the text is at least 128 UTF-16 units long and the first 128 units contain `Treachery.Shared.GameState`.
3. `JsonNode.Parse`. Unlike the normal path, a duplicate key throws here.
4. For each object, recursively:
   - If it has `$type`, the value must be a string (null gives `NullReferenceException`, a number gives `InvalidOperationException`). `Type.GetType(name)` resolves `Treachery.Shared.X` with or without `, Treachery.Shared`. It is case-sensitive and does not trim. If the type is a concrete `GameEvent` subclass, the value is replaced in place with `Type.Name`, keeping its key position. Otherwise the key is removed. Removed or renamed event classes then fail later with "must specify a type discriminator".
   - If the object has exactly one key, `$values`, holding an array: build a new array from deep clones of the non-null items, process each recursively, and replace the object in its parent. A wrapper directly inside another wrapper is not replaced (the clone has no parent), and a wrapper at the root stays. A wrapper directly inside an array throws "Collection was modified".
   - Otherwise, recurse into each non-null child.
5. Arrays: recurse into each non-null element.
6. Write the tree compactly and deserialise normally.

The old writer used no reference preservation, so `$id`/`$ref` never appear. Replicate these steps on an order-preserving JSON tree.

### 12.6 What is persisted

- **Savegame:** `GameState` = `{"Version":int,"_events":[...]}`. GameSettings is inside `EstablishPlayers`.
- **Server-only:** `Participation` (dictionaries and `HashSet<int>`s with removals, so byte-identical output needs .NET slot order) and `Dictionary<int, SubscriptionType>`.
- **SignalR** uses its own options (`Server/Startup.cs:30`, `Client/Client.cs:133`) with concrete event types and no `$type`. It is a separate format from saves.

### 12.7 serde guidance

- Rename every field explicitly. Don't use `rename_all`; names mix `PascalCase`, `_camelId` and `dancerId`.
- Declare struct fields in output order. Emit base-class fields inline through a macro rather than `#[serde(flatten)]`, which buffers through a map and loses error behaviour.
- Custom `Serialize`/`Deserialize` for the event enum, enforcing the `$type` rules in 12.3.
- Model every C# reference-typed member as `Option<T>`. Never skip nulls on write. Use a `Default` that matches the C# initialisers.
- Don't use `deny_unknown_fields`. Handle duplicate keys with a custom visitor (last wins).
- Enums as integers through an `i32` newtype. Reject strings.
- A custom `serde_json` formatter that applies the escape table in 12.1, to keys as well as values.
- A custom `DateTimeOffset` type: i64 ticks plus offset minutes.
- `Convert.ToInt32` semantics when parsing id strings and force-location strings (11.6).

---

## 13. Time

### 13.1 Event timestamps feed game state

`GameEvent.Time` is set by the server (`Server/GameHub_GameEvents.cs:201`, `DateTimeOffset.Now`) and stored in the event JSON. `Game.UpdateTimers` (`Game.cs:227-252`) runs before execution. It acts only if `e.Time != default`, `History` is not empty, and the phase is a timed one (Bidding, GuildShip/Move, NonGuildShip/Move, BattlePhase).

It finds the previous event by exact type:

- Battle → the last `BattleInitiated`
- Move → the last Shipment or EndPhase
- Shipment → the last Move or EndPhase
- anything else → `History[^1]`

It adds the elapsed time to `Timers[e.Initiator][CurrentMainPhase]`. The check is `if (!(elapsed.TotalHours < 1)) return;`, so **negative elapsed times are counted**. `TimeSpan` has 100 ns resolution; use i64 ticks. `DateTimeOffset.Subtract` compares UTC instants.

`Timers` is a private dictionary, `Dictionary<Faction, Timer<MainPhase>>`, and it is dumped.

### 13.2 No clock reads in game logic

There is no `DateTime.Now`, `Stopwatch`, `Environment.TickCount` or `Guid` in Shared engine code. `DateTimeOffset.Now` appears only in `Model/UserConnections.cs:17/43` (server). `Duration`, `Started`, `FirstAction` and `LastAction` (`Game.cs:254-263, 1116-1118`) are read-only. `DateTimeOffset.DateTime` drops the offset.

---

## 14. Version gating

### 14.1 Counts and patterns

Shared has 253 numeric `Version` comparisons on 244 lines. Bots has 1.

- `<`: about 107. The most common thresholds are `< 103` (12), `< 113` (11), `< 159` (7), `< 179` (6), and `< 122`, `< 164`, `< 170` (5 each).
- `>=`: about 112. The most common are `>= 164` (14), `>= 103` (5), and `>= 107`, `>= 132`, `>= 155`, `>= 167` (4 each).
- `<=`: 31, including `<= 150` (14) and `<= 117` (4).
- `>`: 15. These are `> 150` (10), `> 125` (3), `> 117` (1) and `> 154` (1).
- No `==` or `!=`. No reversed operand order.
- Pattern forms: `Version is >= 176 and <= 179` (`Game_Revival.cs:82`), `Game is { Version: >= 151, … }` (`Bid.cs:292`), `{ Version: >= 174, … }` (`NexusPlayed.cs:443`), and a `Version switch` with `when` arms (`Game_Collection.cs:73-80`).
- Thresholds range from 82 to 187. `Game.LatestVersion = 187`, `LowestSupportedVersion = 100`.

```
grep -rnoE --include=*.cs '\bVersion\s*(<=|>=|<|>)\s*[0-9]+' Shared | wc -l
grep -rhoE --include=*.cs '\bVersion\s*(<=|>=|<|>)\s*[0-9]+' Shared | sort | uniq -c | sort -rn
grep -rnE --include=*.cs 'Version:|Version is|Version switch' Shared
```

**Guidance:** port every check literally, with the same operator and number. Don't simplify `>` to `>=` or merge adjacent ranges.

### 14.2 Odd checks to keep as written

- **Dead:** `Version < 100` (`Game_Events.cs:529`) and `Version <= 82` (`Game_Events.cs:192/254`, inside `< 103`) are always false. Keep them so the code stays line-for-line comparable.
- **Overlap:** `Game_Revival.cs:82` (`>= 176 and <= 179`) and `:88` (`>= 179`) both apply at v179, so Recruits doubles free revivals twice there. Verified.
- **The effective threshold differs from the written one:** `AttachedLocation.Sector` checks `Game?.Version >= 159` (`AttachedLocation.cs:21`), but `Game` is assigned only when `Version >= 163` (`:30`). For 159–162 it is null, so the real threshold is 163.
- **Redundant:** `FaceDanced.cs:109` `Version > 150 || (Version <= 150 && FaceDancerCalled)`.
- **Validation and execution gated differently:** `KarmaChoamDiscard.cs:53` validates `CardToUse` for v≥171, but Execute (`:68`) plays `ValidKarmaCards().FirstOrDefault()` regardless. `Revival.cs:184` has a `>= 122` gate that `:173` lacks.

### 14.3 Version-gated random draws

`Bid.cs:248` (`>= 131` shuffles the auction deck), `EstablishPlayers.cs:375-386` (`<= 125` shuffles factions for bots), and `Game_Setup.cs:36` (`< 174` decides the deck source, 5.1). Getting one of these wrong shifts every later draw.

### 14.4 Other gates

- `Applicable(Rule.X)`: 198 sites in Shared and 25 in Bots, over 72 rules. It is `Rules.Contains || RulesForBots.Contains` (`Game_Rules.cs:316`). The rule lists are changed during setup (`EstablishPlayers.cs:214-228`).
- `TreacheryCardManager.GetCardsInPlay` has a `Version <= 104` Amal special case (`TreacheryCardManager.cs:121-124`).
- `isHost` changes which events are applicable (`Game_Events.cs:23/190/234/647`), so traces must record it.
- `Settings.*`, `Prevented(FactionAdvantage)`, `IsPlaying` and `AutomationRules` also gate behaviour.

### 14.5 Version in model code

`Player.cs:463/579` and `PlayerSequence.cs:106` read the version through a back-reference to the game. `AttachedLocation` uses its `Game` field (14.2). No static code checks a version.

---

## 15. Polymorphism and type checks

### 15.1 Event hierarchy

Every concrete event derives from `GameEvent`, `PassableGameEvent` or `PlacementEvent`. **No concrete event subclasses another concrete event**, so `e is Shipment` and `e.GetType() == typeof(Shipment)` always agree. That holds for `LatestEvent(Type)` (`Game.cs:274`), `FindMostRecentEvent` (`Game.cs:280-288`, which returns `History[^1]` when given no types), `Game.cs:844` and `IsApplicable` (`GameEvent.cs:197`).

- `PlacementEvent` subclasses: Caravan, DiscoveryEntered, FaceDanced, Move, PerformSetup, PerformFremenSetup, FremenRidesMonster.
- `PassableGameEvent` subclasses: 26 events, including Bid, BlackMarketBid, Shipment, TerrorRevealed and AmbassadorActivated.

### 15.2 Virtual members

- Abstract: `Validate`, `ExecuteConcreteEvent`.
- Virtual: `Execute` (overridden only by `EstablishPlayers`, 1.2), `GetMessage`, `GetShortMessage`.
- `PlacementEvent.TotalAmountOfForcesAddedToLocation` is virtual (`PlacementEvent.cs:72`). `FremenRidesMonster.cs:33` overrides it to include reserve forces. The non-virtual `ForcesAddedToLocation`/`SpecialForcesAddedToLocation` do not include them.
- `Location.Sector` is virtual. `Homeworld` returns -1, though its setter still writes the backing field. `AttachedLocation` has the version-dependent override.
- `AttachedLocation.Neighbours` builds a new list on each read, so adding to the result does nothing.
- There are no `new`-hiding members.

### 15.3 Location subclass checks

`Homeworld : Location`. `AttachedLocation` (abstract) has the subclasses `DiscoveredLocation` and `HiddenMobileStronghold`.

Type checks on them: `Map.cs:86/108/111`, `Player.cs:70/694/708`, `PlacementEvent.cs:172`, `Shipment.cs:56/203/269/356/367/619/632`, `BeneGesseritAccompanies.cs:121`, `DiscoveryEntered.cs:52`, `FremenRidesMonster.cs:117/124` and `Game_Battle.cs:1113`. At `FremenRidesMonster.cs:117` the check reads `al.AttachedToLocation.Sector`, not `al.Sector`.

**Guidance:** one Rust `Location` struct with a `kind` enum, and helpers `is_homeworld()` / `as_attached()`.

### 15.4 `IHero` type checks

`is Leader` is used at `Game_LeaderState.cs:39/49`, `Game_Battle.cs:458/546/823/859/912/1143`, `Game_Move.cs:885`, `Battle.cs:415`, `LoserConcluded.cs:78-80`, `Revival.cs:337/535`, `RaiseDeadPlayed.cs:133-138`, `FaceDanced.cs:119`, `KarmaFreeRevival.cs:96`, `AmbassadorActivated.cs:453` and `NexusPlayed.cs:605`. `is TreacheryCard` is used at `Game_Battle.cs:257` and `LoserConcluded.cs:123`. See also 7.3.

### 15.5 `ClairVoyancePlayed.Parameter1/2` return boxed objects

These getters return a boxed enum, a float, an `IHero` or a `Territory`, depending on the question (`ClairVoyancePlayed.cs:42-92`). Callers cast or pattern-match: `ClairVoyanceAnswered.cs:87/99` checks `is IHero`, and `Bots/Bot_Other.cs:599-679` unboxes with `(float)` and `(int)`, which throws on a mismatch. Model the value as a Rust enum, and make each cast fail where C# throws `InvalidCastException` or `NullReferenceException`.

---

## 16. Bots

### 16.1 The `Do` chain stops after an invalid decision

`Bots/Bot.cs:219-250`. Each priority tier is an `||` chain of `Do<T>(DetermineX, ref action, events)`. `Do` runs only if `action == null` and `events.Contains(typeof(T))`, where `T` is the method's declared return type.

When `Validate` fails, `action` is **not** reset to null. Every later `Do` in that tier skips, and the tier returns null. The `Determine*` methods after it never run and never draw. Verified.

The bot never iterates the applicable-events list; it only checks membership. Server drivers run each tier for every bot before moving on, in an unseeded random bot order. Tests use `game.Players` order.

### 16.2 Exceptions used for control flow

- `Do` catches every exception from `method()` (`Bot.cs:225-232`) and logs it to the console. Random draws and bot-state changes made before the throw stay.
- `DetermineClairVoyanceAnswered` (`Bots/Bot_Other.cs:599-679`) swallows exceptions and still returns an answer.

Plausible throws:

- **`D(n, sides)` with negative `sides`:** `Random.Next` throws `ArgumentOutOfRangeException`. This happens with short games at `Bots/Bot_FactionSpecific.cs:354/358/362/363` (`MaximumTurns - 2` and `- 3`), and at `Bots/Bot_Other.cs:784` if `max < min - 1`. `_count` has already gone up.
- **`D(n, 0)` returns 0 without a draw.** `D(n, 1)` does draw.
- **`RandomItemFrom` with an empty array** indexes -1 and throws (`Bots/Bot_Other.cs:1094-1098`). With one or more items it always draws.
- **`First` and `Last`:**
  - `OrderBy(...).First()`: `Bots/Bot_Battle.cs:50`, `Bots/Bot_Other.cs:98`, `Bots/Bot_FactionSpecific.cs:42`.
  - `First(predicate)`: `Bots/Bot_Battle.cs:291/296`, `Bots/Bot_Move.cs:235/241/248/255`, `Bots/Bot_FactionSpecific.cs:165/941`.
  - Plain `First()`: `Bots/Bot_FactionSpecific.cs:77/162`, `Bots/Bot_Shipment.cs:371-372` (`First`/`Last`), `Bots/Bot_Other.cs:32/1053/1089`.
- **`SingleOrDefault` with 2 or more matches:** `Bots/Bot_Other.cs:796`.
- **`ToDictionary` with duplicate keys:** `Bots/Bot_Other.cs:26`, `Bots/Bot_FactionSpecific.cs:210`.
- **`ValidCunningNoFieldValues(...).Max()` on an empty sequence:** `Bots/Bot_Shipment.cs:586`.
- **Null `CurrentBattle`/`PlanOf` in Nexus decisions,** which run in the highest tier in any phase: `Bots/Bot_Other.cs:150/174/213/258/280/294/316/396-398`.
- **Other null dereferences:** `Bots/Bot_Knowledge.cs:307` (`LatestSpiceCardA.Location`) and `Bots/Bot_Bidding.cs:26-28` (`CardsOnAuction.Top.Type`).

**Guidance:** each `determine_*` returns `Result<Option<Event>, BotError>`. An `Err` becomes `None`, without rolling back random draws or bot state.

### 16.3 A random-drawing selector evaluated twice

`Bots/Bot_FactionSpecific.cs:287`:

```csharp
ValidTerritories(...).Where(IWantToAnnounceBattleIn).LowestOrDefault(_random, t => GetDialNeeded(t, GetOpponentThatOccupies(t), false))
```

`GetDialNeeded` draws random numbers through `ChanceOfEnemyLeaderDying`, `DetermineBestDefense`, `SelectHeroForBattle` and `BestVoice`. `LowestOrDefault` calls the selector once per element in `Min` and again in the `Equals(best)` filter (8.2). The second pass can produce different values, so zero elements may match and the result is null.

The draw order is:

1. `ToArray`, which runs `IWantToAnnounceBattleIn` (itself calling `GetDialNeeded`) per element.
2. The `Min` pass.
3. The filter pass.
4. A tie-break draw if 2 or more elements match.

Every other `Highest`/`Lowest` selector in Bots was checked and is pure.

### 16.4 Lazy chains that interleave draws

These evaluate element by element, with each element going through every stage before the next element starts. Port them as lazy iterator chains or a single loop, and never `collect()` between stages:

- `Bots/Bot_Knowledge.cs:535-553`: `Where → Select(OccupyingOpponentIn [draws]) → Where → Select(GetDialNeeded [draws]) → Where → OrderBy`.
- `Bots/Bot_Shipment.cs:249-257`, `:450-457`, `:477-492` (`ConstructAttack`).
- `Bots/Bot_FactionSpecific.cs:1139-1145`.
- `Bots/Bot_Knowledge.cs:390-425`: a `HighestOrDefault` tie draw inside a filter.
- `Bots/Bot_Knowledge.cs:429`.

### 16.5 Draws that depend on control flow

Short-circuits and early returns that gate draws. Rust's `&&` and `||` behave the same, but don't restructure:

- `Bots/Bot_Bidding.cs:35-36` (always draws), `:61`
- `Bots/Bot_Shipment.cs:35/38`, the strategy chain at `:44-60`, `:254/283/290/298-299`
- `Bots/Bot_FactionSpecific.cs:342-344` (a coin flip whose only effect is the draw count), `:349` (nested), `:353/362`, `:631`, `:918`, `:1021-1028`, `:1038-1056`, `:1182/1206` (initialiser order)
- `Bots/Bot_Battle.cs:34-35`, `:146` (`A || (B && C)`), `:795/814` (a MirrorWeapon redraw), `:870-871`
- `Bots/Bot_Knowledge.cs:447-451`
- `Bots/Bot_Alliance.cs:40-45`
- `Bots/Bot_Other.cs:105/111-116/573-574/724/789/885`
- `Bots/Bot_Deals.cs:60-65` (`D(3,3)` is 3 draws)
- `Bots/Bot_Move.cs:71-215/298-360`

The Nexus decisions (`Bots/Bot_Other.cs:121-418`) call whole other decision procedures, including `DetermineShipment`, which mutates bot state.

Draw-site counts per file: Alliance 4, Battle 11, Bidding 4, Deals 3, Knowledge 13, Other 26, FactionSpecific 48 (some lines draw twice), Shipment 11, Move 0.

```
cd Bots/Treachery.Bots/Classic && grep -nE '\bD\(|_random|Shuffle|RandomItemFrom' *.cs
```

### 16.6 Collection order in bots

Bots iterate:

- `Player.ForcesOnPlanet`: `Bots/Bot_Knowledge.cs:337-425/439`, `Bots/Bot_Move.cs:46/266`, `Bots/Bot_FactionSpecific.cs:1190`.
- `Game.ResourcesOnPlanet`: `Bots/Bot_Knowledge.cs:429`, `Bots/Bot_Shipment.cs:86/172/756`, `Bots/Bot_FactionSpecific.cs:219/1104`.
- `DiscoveriesOnPlanet.FirstOrDefault`: `Bots/Bot_Knowledge.cs:434`, `Bots/Bot_Shipment.cs:107/194`.
- `ShipmentPermissions.Keys`: `Bots/Bot_FactionSpecific.cs:21-22`.
- `Game.KnownCards(p)`: `Game.cs:414-426`, which includes `KnownCards` with its free-list holes.

All of these need engine collections with .NET order (3.1).

### 16.7 Bot float math

- **Float literals:**
  - `Bots/Bot_Alliance.cs:19/26` `> 0.75f * int`
  - `Bots/Bot_Deals.cs:35` `Price < 1.4f * Count()`. `1.4f * 10` rounds to `14.0f`.
  - `Bots/Bot_Deals.cs:185` `(int)((… ? 1 : 1.5f) * n)`
  - `Bots/Bot_FactionSpecific.cs:1005` `> 1.5f * int`
  - `BotParameters.PenaltyForAttackingBots = 6.0f`
- **Double literals:** 9.4.
- **`CumulativeChance`** (`Bots/Bot_Battle.cs:692-697`) runs in **double** and is cast to float at `:676/687`. Later steps use `1f - saving` (`:810`) and `1 - dying` (`:920`), and are compared against `f32` parameters such as `0.55f`.
- **Spice math** (`Bots/Bot_Shipment.cs:98/183/769`): `(float)x / rate`, then `Math.Min(float, 4)`, plus `TotalMaxDial`, then `Math.Ceiling` (widened to double), then `(int)`. Same pattern at `Bots/Bot_Shipment.cs:501` and `Bots/Bot_Battle.cs:1000`.
- **Integer halving:** `Bots/Bot_Move.cs:191` `TotalAmountOfForces / 2`.
- **Float sort keys:** `Bots/Bot_Battle.cs:735` and `Bots/Bot_Knowledge.cs:553` (9.4).

### 16.8 Bot state persists across calls

These fields make results depend on call history:

- `DecidedShipmentAction`, `DecidedShipment` and `FinalDestination` (`Bots/Bot_Shipment.cs:14-16`).
  - Reset by `DetermineShipment` (`:22-24`), and by `DetermineMovedBattalion` only on its no-move path (`Bots/Bot_Move.cs:213`). `DetermineCaravan` sets them whenever it found a move, even an invalid one (`Bots/Bot_Move.cs:60`).
  - Read by the move logic (`Bots/Bot_Move.cs:76-86`), by battle plans (`Bots/Bot_Battle.cs:157`) and by Nexus (`Bots/Bot_Other.cs:285`). Nexus can call `DetermineShipment` outside the shipment phase and overwrite them.
- `VoicePlan` (`Bots/Bot_FactionSpecific.cs:369`), matched by reference identity.
- `PriceSetEarlier` (`Bots/Bot_FactionSpecific.cs:781`), never cleared.
- `TurnWhenRevivalWasRequested` (`:782`).
- `_random`.

The server keeps one bot per faction per game. **Guidance:** a persistent bot struct per faction, with stable handles for any identity comparison.

`BotParameters` (`Bots/BotParameters.cs`) are `static readonly` instances with public setters that nothing mutates. `GetDefaultParameters` returns a `MemberwiseClone`, which is a deep copy because every field is a value type. Port as a `Copy` struct.

### 16.9 Bot reads of engine state with side effects

Bots call engine queries that can have side effects: the neighbour cache (6.1), `NextPlayerToBattle` (4.3) and `GetApplicableEvents`. A bot-vs-bot trace must call these in the same order in both engines.

Bots write only to their own `AllyPermission` event, through reflection (`Bots/Bot_Alliance.cs:195-207`), using 12 property names that exist on both `AllyPermission` and `Game`.

---

## 17. Other language details

### 17.1 Operator precedence

Mixed `&&`/`||` expressions all read as `(a && b) || c` in both languages. Sites: `Game.cs:595`, `Game_Revival.cs:79`, `Game_Participation.cs:60`, `Game_Battle.cs:753`, `Player.cs:463`, `Shipment.cs:105/109/281/290/301`, `Battle.cs:610`.

Ternaries need explicit parentheses in Rust:

- `a || b || c ? null : x` (`Game_Blow.cs:82-84`) means `(a || b || c) ? null : x`.
- `killed || (v < 145 && melt) ? 0 : …` (`Battle.cs:319-320`).

### 17.2 `ref` and `out` parameters

Sites: `Game_Bidding.cs:259` (`ref MessagePart`), `Battle.cs:374` (three `out`), `:388` (two `ref`), `:824/829/930/935` (`ref LeaderSkill`), `Shipment.cs:563`, `Game.cs:755`, `Player.cs:610`, plus 39 `out var` sites. Port them as `&mut`, and keep assignment order: a `ref` parameter may be written and then overwritten.

### 17.3 Structs and records

- **Structs:** `Moment`, `PointD`, `FlashInfo` and `NeighbourCacheKey`. None is mutated through a copy.
- **`Battalion` is a class** (5.4).
- **Records:** there are no `with` expressions, and record equality and `ToString` are never used in Shared.

### 17.4 Closures

No `for` loop variable is captured by a lambda. `foreach` variables get a fresh binding per iteration in C# 5 and later.

### 17.5 `params object?[]`

A null inside `params` stays as a null element. A single `object[]` argument is passed through as the array itself (`Game_Other.cs:166`, `Game.cs:703`).

---

## 18. C# bugs to preserve

Port all of these as-is, each with a `// C# parity:` comment. Most were found by reading the code. The ones marked ✓ were re-checked against the source while writing this document.

### 18.1 Validation messages built and never returned

- ✓ `Shared/Events/Donated.cs:56`: "No bribes can be made during Double Inflation".
- ✓ `Shared/Events/CardTraded.cs:56`: "Your ally does not have cards to trade". The same line also dereferences `AlliedPlayer!`.
- ~~`Shared/Events/Revival.cs:82`: "Select forces or a leader to revive".~~ Fixed in both C# and Rust; empty revivals are now rejected.
- `Shared/Events/Revival.cs:103`: "You can't revive that many" (the `Version < 124` branch).

### 18.2 Bugs that change state

| Site | Bug |
|---|---|
| ✓ `Model/Player.cs:277-286` `ForcesToReserves` | Uses `toRemoveInTotal` where `toRemoveInThisLocation` was meant. The whole amount goes to reserves at every location, and the location removal is skipped when it exceeds the forces there, which duplicates forces. |
| `Model/Battalion.cs:32-40` | `ChangeForces`/`ChangeSpecialForces` do nothing when the result would go negative (no clamp). |
| `Player.MoveForces`, `Game_Battle.cs:224-229` | When removing too many is a no-op, the matching add still happens. |
| ✓ `Game_Revival.cs:82/88` | Recruits doubles free revivals twice at v179. |
| `KarmaChoamDiscard.cs:53` vs `:68` | Validates `CardToUse` but plays `FirstOrDefault()`. The log at `:78` reads "XUsing". |
| `NexusPlayed.cs:129` | Tleilaxu secret-ally cost is validated but never deducted. |
| `NexusPlayed.cs:143-148` vs `623-629` | The CHOAM card is validated only for Cunning; SecretAlly uses an unvalidated card and still gains 2. |
| `NexusPlayed.cs:451-453` | v≥174 Richese betrayal refunds Emperor's contribution to Richese's ally. |
| `Game_Collection.cs:87-91` | Logs "0 … goes to occupier" whenever there is an occupier. |
| `Game_Collection.cs:191-193` | Before v162 the occupier is paid but Fremen is not charged. |
| `Game_Bidding.cs:285` | Richese's homeworld threshold and occupier are applied to Emperor's income. |
| `Game_Bidding.cs:293` | Records a recent payment of `receiverProfit` while it is still 0. |
| `Game_Move.cs:86-91` | Sandmaster is applied per source location. |
| `Game_Blow.cs:226-236` | Sand Trout doubling isn't reset when the doubled blow is lost to the storm. |
| `Game_Storm.cs:213/224` | `MainPhaseEnd()` is called twice. |
| `Game_Storm.cs:217` | Resumes in BlowA and skips the storm report. |
| `Game_Revival.cs:174-180` | Tleilaxu earns 1 when it revives its own forces for free. |
| `Model/Player.cs:240-246, 258-262, 279-281` | Richese special forces sent to reserves vanish. |
| `Model/Player.cs:139-140` | If no single homeworld has enough forces, the shipment silently does nothing. |
| `Model/TreacheryCard.cs:43-44` | Mirror Weapon against Mirror Weapon recurses without end (stack overflow, which `Execute` cannot catch). |
| `Game_Battle.cs:280-283` | v146–178: the melted-rock path discards a Portable Antidote. |
| `Game_Battle.cs:941-978` | `TwoTraitorsCalled` never sets the winner or loser. |
| `Battle.cs:224-227` | Portable Antidote overwrites `OriginalDefense` after the Diplomacy swap. |
| `Battle.cs:615` | `returnOnlyOneBattle` only breaks the inner loop. |
| `BattleInitiated.cs:87-96` | The Juice GoFirst and GoLast predicates are identical. |
| `BeneGesseritFlip.cs:54` vs `43/69/76` | Flips one territory but logs another. |
| `SkillAssigned.cs:69-82` | Execute ignores `Passed`. |
| `Shipment.cs:505` | Back-to-reserves moves only `ForceAmount` but charges for special forces too. |
| `Revival.cs:484` | Ignores `ExtraSpecialForcesPaidByEmperor`. |
| `FactionTradeOffered.cs:64-65` | Before v172, `Target` is never removed from `FactionsInPlay`. |
| `AutomationConfigured.cs:76-86, 93` | `Update` does nothing; `Delete` uses `First` (1.4) and doesn't check ownership; `GetMessage` was copied from AllyPermission. |
| `Game_Setup.cs:75` | An `Enter` whose result is overwritten at `:96`, but whose side effects run. |
| `Game_Move.cs:548-565` vs `623-640` | Inconsistent Terror and Ambassador phase mapping for BeneGesseritAccompanies. |
| `Game_Battle.cs:1116-1129` | A different occupation rule from `DetermineOccupationAtStartOrEndOfTurn`, which also logs. |
| `Game_Bidding.cs:417-418` | v151–158: enters ReplacingCardJustWon whenever anyone holds a Nexus card. |
| `DistransUsed.cs:98`, `Bid.cs:157` | Nested `Execute` and skipped pre/post tasks (5.5). |
| `AttachedLocation.cs:21/30` | The version check is effectively 163, not 159 (14.2). |
| `TraitorDiscarded.cs:61/70` | Logs "shuffle into the deck" but doesn't shuffle. |
| `TerrorRevealed.cs:200` | Atomics records `Milestone.MetheorUsed`. |
| ✓ `AllyPermission.cs:151-152` | With no ally, writes `PermittedUseOfAllySpice[Faction.None]` (an indexer set, so no throw). |
| `LoserConcluded.cs:64-65` | A stray `KeptCard` argument is appended to messages. |

### 18.3 Dead or constant code

`Karma.cs:267`, `TerrorRevealed.cs:167`, `BeneGesseritAccompanies.cs:102`, `Game_Blow.cs:305/384`, `Game_Setup.cs:186`, `Player.cs:462-463`, and the uncalled `FremenRidesMonster.IsEitherValidDiscoveryOrNoDiscovery`. Port them anyway, for line-by-line review.

### 18.4 Weak validation the port must accept

- **Negative amounts accepted:** Battle, Bid, BlackMarketBid, PlacementEvent (per location), ResourcesTransferred, DivideResources, TakeLosses, AmbassadorActivated, FaceDanced, KarmaFreeRevival, Revival, Retreat, FremenRidesMonster, TerrorRevealed, Shipment, NexusPlayed, PerformSetup, Donated.
- **Card ownership not checked:** Donated, KarmaChoamDiscard, KarmaHandSwap, the Ixian events, RicheseGaveCard, Karma, KarmaBid, BattleConcluded, LoserConcluded, TerrorRevealed, AmbassadorActivated, Diplomacy.
- **A `Valid*` helper exists but `Validate` doesn't use it:** BeneGesseritPrediction, BeneGesseritBattleAnnouncement, KarmaMonster, HMSAdvantageChosen, JuicePlayed, Prescience, TerrorPlanted, DiscoveryEntered, PerformFremenSetup, NexusPlayed, AmbassadorPlaced, CharityClaimed, BattleConcluded, StormDialled, NexusVoted, DealOffered (anyone can cancel an identical offer), DealAccepted.
- **Rejected only by throwing during execution:** see 1.4.

### 18.5 Bot bugs

- `Bots/Bot_Other.cs:42-44`: `b.Faction == Faction && b.Faction == Ally` is always false.
- `Bots/Bot_Other.cs:39-40`: the storm-sector ±1 arithmetic never wraps.
- `Bots/Bot_FactionSpecific.cs:482`: `d.IsProjectileDefense && !d.IsProjectileDefense`.
- `Bots/Bot_Knowledge.cs:570`: the ternary condition is always true.
- `Bots/Bot_Knowledge.cs:641`: `Karma` is checked twice.
- `Bots/Bot_Shipment.cs:632`: the loop increments the wrong force count.
- Duplicate conditions: `Bots/Bot_Shipment.cs:50`, `Bots/Bot_Move.cs:199-201`.
- `Bots/Bot_Battle.cs:904`: `GetDialNeeded(p, …)` always computes for `Player`.
- `Bots/Bot_Knowledge.cs:545`: uses the first opponent rather than `s.Opponent`.
- `Bots/Bot_Battle.cs:795/814`: MirrorWeapon redraw.
- `Bots/Bot_FactionSpecific.cs:342`: meaningless coin flip.
- Plus 16.1 (a blocked tier) and 10.2 (`Bots/Bot_Other.cs:462`).

### 18.6 Ordering details

- `Shipment.cs:525` and `AmbassadorActivated.cs:443` move to the next sub-phase before paying and logging.
- Moving Vidal or a ghola uses `Leaders.Remove` + `Add`, which puts them at the end of the new owner's list.
- A Karama card used in a bid goes back to the end of the hand (`Game_Bidding.cs:137/178`).
- `Deck.PutOnTop(IEnumerable)` reverses its input (2.3).

---

## Appendix: probed .NET 10 behaviour

Run with `dotnet run probe.cs` on .NET 10 under `CultureInfo.InvariantCulture`. The probe was a scratch file outside the repo.

| Expression | Result |
|---|---|
| `(1e7f).ToString()` / `(1e-5f)` / `(1e21).ToString()` | `10000000` / `1E-05` / `1E+21` |
| `float.PositiveInfinity` / `NaN` / `-0f` | `Infinity` / `NaN` / `-0` |
| `0.1f > 0.1` | `True` |
| `Math.Max(NaN, 1f)`; `1/Math.Max(-0f, 0f)`; `1/Math.Min(0f, -0f)` | `NaN`; `+∞`; `-∞` |
| `float.Parse("1,5")` / `("infinity")` / `("1e99")` | `15` / `∞` / `∞` |
| `Enum.Parse<Day>("Monday, Tuesday")` / `("12345")` / `(" Monday ")` / `("monday")` | `Wednesday` / `12345` / `Monday` / `ArgumentException` |
| `((Day)99).ToString()` | `99` |
| `Convert.ToInt32(" +12 ")` / `("")` / `(null)` / `("99999999999")` | `12` / `FormatException` / `0` / `OverflowException` |
| `int.MinValue / -1`, `% -1` | `OverflowException` |
| `Math.Abs(int.MinValue)` | `OverflowException` "Negating the minimum value of a twos complement number is invalid." |
| `(int)NaN` / `(int)1e20f` / `(int)-1e20` | `0` / `2147483647` / `-2147483648` |
| `new[]{int.MaxValue, 1}.Sum()` | `OverflowException` |
| `new[]{16777216f, 1f, 1f}.Sum()` | `16777218` (double accumulator) |
| `int.MaxValue + 1` (unchecked) | `-2147483648` |
| `new int[0].Max()` / `new int?[0].Max()` | `InvalidOperationException` / `null` |
| `new Random(1).Next(-1)` | `ArgumentOutOfRangeException` |
| `r.Next(0)` then `r.Next(100)` vs a fresh `r.Next(100)` (seed 5) | `28` vs `33`: `Next(0)` consumes a sample |
| `new Random(42)`, five `Next(6)` | `4,0,0,3,1` (a known-answer test for `NetRandom`) |
| `((IComparable)float.NaN).Equals(NaN)`; `(-0f).Equals(0f)`; `NaN.CompareTo(1f)` | `True`; `True`; `-1` |
| `string.Join("", {"a", null, 1, Day.Tuesday})` | `a1Tuesday` |
| Dictionary {1,2,3}: remove 1, remove 2, add 9, 8, 7 | iteration `8,9,3,7` |
| HashSet {1,2,3}: remove 1, add 9 | iteration `9,2,3` |
| `List.Remove` during `foreach` | `InvalidOperationException` "Collection was modified; enumeration operation may not execute." |
| Dictionary indexer overwrite during `foreach` over `.Keys` | no exception |
| Missing dictionary key; duplicate `Add` (enum key) | "The given key 'Monday' was not present in the dictionary."; "An item with the same key has already been added. Key: Monday" |
