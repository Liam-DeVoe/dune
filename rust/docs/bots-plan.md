# Phase 2: porting the bots

Goal: Rust bots that choose exactly the event the C# bots choose, given the same game and the same bot seeds. The check is the differ: at every point where the oracle's bots deliberated, the Rust bots deliberate on the Rust game and must produce the same event, byte for byte, with the same number of random draws.

Read [PORTING.md](../../PORTING.md) and [hazards.md](hazards.md) section 16 first. This file adds what those don't cover.

## What is being ported

`treachery-online/Bots` is one class, `ClassicBot`, split over 10 partial files (6,430 lines), plus `BotParameters` (270 lines). There is no ML at runtime. `ML/Treachery.ML` is a standalone ML.NET trainer that reads a `Shipment.csv`. No project references it, and Bots references only Shared (`Bots/Treachery.Bots/Treachery.Bots.csproj`). Phase 2 has no ML work.

The skeleton is in `rust/bots` (crate `treachery-bots`, in the workspace). Every C# member has a `todo!("port: File.cs:line")` body, the same as the engine skeleton had. Regenerate it with `cd rust/engine/csharp-gen && dotnet run -- --bots`. Like `--glue`, it never overwrites an existing file unless you pass `--force`, and `--force` must not be used once porting starts.

How the skeleton maps C# (the rules are in the header of `engine/csharp-gen/BotsGen.cs`):

- One module per C# file. The `ClassicBot` struct, with the fields of every partial file, is in `bot.rs`. Each file has its own `impl ClassicBot`, the way the `Game` partials do.
- The bot doesn't store the game. Every instance method takes `game`, and `set_game_and_player` only rebinds `player`.
- A method takes `&mut self` only if it writes bot state, directly or through a method it calls (38 methods: the shipment decisions, `DetermineVoice`, the Tleilaxu revival pair, the dispatch). The other methods take `&self`. `_random` is a `SharedRandom`, so a draw needs no `&mut`.
- `game: &mut Game` appears only on `DetermineRockWasMelted` and the dispatch (`Do` and the five `Determine*Action` methods). See "Bots are not pure" below.
- `Do<T>` came out as `&dyn Fn() -> T`, which is a placeholder. Package B1 gives it its real shape (see below).
- Overloads follow the engine rule (the parameter types are appended), so a few names are long. Rename them freely.
- `IBot` (Shared, left out of the engine) is the `IBot` trait in `interface.rs`.

## Randomness today

- Each bot owns a `LoggedRandom _random` (`Bot.cs:301`). Its only draws are `Next(int)`, through `D(n, sides)`, the `*OrDefault(_random, ...)` helpers, `RandomItemFrom`, and one `Deck` built on `_random` (`Bot_FactionSpecific.cs:1204`, which copies `Game.RicheseCache`).
- Bots have no unseeded randomness. Every `RandomOrDefault`/`HighestOrDefault`/`LowestOrDefault`/`OneOf…` call in Bots passes `_random` (`grep -nE "(RandomOrDefault|HighestOrDefault|LowestOrDefault|TakeRandomN)\((\)|[a-z]+ =>)" Bots -r` finds nothing). The `*_unseeded` engine helpers that [engine-conventions.md](engine-conventions.md#randomness) mentions are not needed.
- The oracle seeds each bot with `new LoggedRandom(DeriveSeed(gameSeed, (int)faction, 0x424F54))` when the bot is first asked (`Oracle/GameRunner.cs:441-454`). It keeps one bot per faction for the whole game, across undo, and calls `SetGameAndPlayer` before every ask. The bot is keyed by the player's faction at that moment, so a bot for `Faction.None` can exist from before factions are chosen.
- What a trace records today: `bot.seed` on steps where a bot's event was submitted, `botOrder` on those steps, and `botRandom` (draw counts of every bot, in creation order) on every line once a bot exists. It doesn't record the seed of a bot that never acted, a deliberation whose result was thrown away (fuzz mutation, a fuzz battle action, an opportunistic event), or the order of a deliberation that ended with a host event.
- The server constructs bots without a seed (`Server/GameHub_GameEvents.cs:389`) and orders them with the unseeded `Deck.Randomize`. That matters only in phase 3.

## How the oracle drives bots

Once per main-loop round, `DeliberateBots` (`GameRunner.cs:490-547`):

1. calls `GetApplicableEvents` for each player in bot order (these are recorded in `queries`);
2. for each level in Highest, High, Middle, Low, End, asks every player's bot in bot order and stops at the first non-null event. `ServerHumanHost` skips End.

So every bot runs every level up to the one that produced the event, and each run can draw. Inside a level, `Do` (`Bot.cs:219-250`) tries the `Determine*` methods in order. It swallows any exception, keeps an invalid event (which blocks the rest of the level, hazards 16.1) and validates on the bot's own event object. The oracle then sets the event's `Time`, serializes it, and executes a deserialized copy.

## Trace changes (oracle, trace format 2)

Add one entry kind to `queries`, written by `DeliberateBots` right after its `GetApplicableEvents` calls and after `AskBots` returns or throws:

```
["bots", {"order":[2,0,1], "end":true,
          "created":[[faction, player, seed], ...],
          "choice":{"player":0, "faction":"Atreides(20)", "priority":"Low", "event":{...}} | null,
          "exception":{...}}]
```

- `end`: whether the End level was asked (false for `ServerHumanHost`).
- `created`: bots created during this deliberation, in creation order. This makes the trace self-contained, so the differ never needs `DeriveSeed`.
- `choice.event`: the event as `Utilities.Serialize<GameEvent>` writes it right after `AskBots`, before `Submit` sets `Time`. It is recorded even when the oracle then submits something else.
- `exception`: only when deliberation threw (the line is then a `botException` end line), in the 7.4 form.

The applicable-event lists aren't repeated. They are the results of the `[player, isHost]` entries just before the `bots` entry.

`botRandom` on the following line already gives the draw counts after every deliberation since the previous line, so the entry doesn't repeat them. Bump `traceFormat` to 2, document the entry in [trace-format.md](trace-format.md) section 3, and regenerate `traces/sample/`. Older traces have no `bots` entries and replay as before.

The change is about 40 lines in `GameRunner.cs` and `Recorder.cs`. `Shared` and `Bots` stay untouched.

## Differ changes

- Keep a bot registry per trace, `Vec<(Faction, ClassicBot)>` in creation order, alive across undo lines, as the oracle's `_bots` is.
- Remember the Rust result of each `GetApplicableEvents` query on the current line by `(player, isHost)`.
- At a `bots` entry: create the bots listed in `created` with `ClassicBot::new(game, player, BotParameters::get_default_parameters(faction), Some(SharedRandom::from_seed(seed)))`. Then run the oracle's loop on the live replay game: levels in order (End only if `end`), players in `order`, `set_game_and_player` first, stop at the first `Some`. Don't clone the game. The bots must see the same neighbour cache and must make their state changes to the real game (see "Bots are not pure").
- Compare:
  - `choice`: same player and level, and `json::serialize` of the Rust event equals `choice.event` byte for byte. Key `bot`.
  - `exception`: a C# exception escaping deliberation must escape in Rust too, with the same text. Key `bot`.
  - `botRandom` of the next line against the registry's counts. Key `botRandom`. It catches a divergence at its first draw, often before any choice differs.
- Report group: "bot decision differs: C# `<EventType>` at `<level>`", so each group points at one `Determine*` method. Print both event JSONs in `first`.
- Then continue as today: execute the event recorded in the trace, not the Rust bot's event. One bad decision doesn't derail the rest of the trace, and the first difference stays the first.
- A Rust panic that isn't a C# exception (`todo!`, index out of bounds) propagates and lands in "not ported yet", as in phase 1. That is how partial ports make progress visible.
- `botHung` end lines: run the deliberation with `--hang-seconds` and count it as a match if it doesn't return. Low priority.

Scope: check bot decisions in `bots` mode and in fuzz traces with the `client` profile, never for `wide` (forged values, outside the Scope rule). Fuzz checking is on since 2026-11-11 (see "Fuzz traces are bot-checked" below).

The phase is done when `differ live --mode bots --status every` over a few hundred games (several seeds) matches on every line, including `bot` and `botRandom`, and `sample_corpus` passes with the regenerated corpus.

## Engine dependencies

Everything bots call exists in the engine, and nothing is a `todo!`. I checked this two ways:

- All 138 static event-helper calls in Bots (`Battle.ValidWeapons`, `Shipment.ValidSources`, `FremenRidesMonster.ValidTargets`, ...) resolve to `pub fn`s in `engine/src/events`, none with a `todo!` body.
- Every other member name Bots uses resolves to a `pub` fn or field by name. That check is name-level, not type-checked, so expect a few to need a different overload or a `pub` added.

Points that need attention:

- **Not tested yet.** The static `Valid*`/`Max*`/`IsApplicable` helpers, `GetDescription` and the obsolete getters are called only by bots and the UI, so no trace has exercised them ([reviews/pkg-c.md](reviews/pkg-c.md), "Members only bots and the UI call"). Expect some bot-decision differences to be engine bugs. Route those to the engine fixers.
- **`&mut Game`.** The only engine function bots reach that takes `&mut Game` is `Battle::determine_battle_outcome` (from `DetermineRockWasMelted`).
- **`DealOffered.Acceptance`** is `DealOffered::acceptance(&self, game, by) -> Option<Rc<DealAccepted>>`. The bot returns it as its event (`Bot_Deals.cs:23-44`).
- **`ClairVoyancePlayed.Parameter1`** is `parameter1(&self, game) -> ExpressionElement`, the boxed C# `object`. Bots unbox it with casts like `(TreacheryCardType)`, `(IHero)`, `(Faction)`, `(float)` and `(int)Parameter2` (13 sites). Bots need a small unboxing helper that throws where C# would (a null, or the wrong boxed type). Twelve sites are inside the `try` of `DetermineClairVoyanceAnswered` (`Bot_Other.cs:599-679`), so only "throws or not" matters there. `Bot_Battle.cs:825` is inside `Do`. Put the helper in the bots crate.
- **`ObjectCounter.Highest`** returns `None` where C# returns `default`. The bots' only caller uses `TreacheryCard`, where the two agree ([reviews/pkg-a.md](reviews/pkg-a.md)).
- **`GameStatus`**: bots don't use it.
- **`Message.DefaultDescriber`** is null in the oracle, so the describer branches of `LogInfo`/`LogError` are dead.

Helpers that only bots need go in `rust/bots/src` (a `support.rs`), not in the engine. Engine fix agents own `engine/src`.

## Bot hazards

[hazards.md](hazards.md) section 16 (and 18.5 for bot bugs to preserve) is the main list: `Do` blocking after an invalid decision, exceptions as control flow, the selector that draws twice, lazy chains with draws, draws behind short-circuits, collection order, float math, persistent bot state. These are new:

- **Bots are not pure.** `DetermineRockWasMelted` (`Bot_Battle.cs:1065-1070`) calls `Battle.DetermineBattleOutcome` on `Game.AggressorPlan`/`DefenderPlan`. That runs `ActivateDynamicWeapons`, which swaps a Mirror Weapon on the stored plan, writes a log entry, sets `CardUsedByDiplomat` and changes the Diplomat defense. `DetermineBattleOutcome` swaps the cards back at the end, so what lasts is the log entry and `CardUsedByDiplomat`. So a bot can change the game, and calling it twice changes it again. Fuzz seed 1 game 61 reaches it: with `--check-bot-purity true`, step 241 (`RockWasMelted`) is flagged `botMutatedState`, and C# has a "card 49 becomes a 55" log entry that the replay can't produce. The differ must run Rust bots on the live game for this reason. A phase-1 replay (no bots) of such a game will show a hash difference that isn't an engine bug.
- **`LogInfo` and `LogError` arguments are evaluated.** `LogInfo` returns early, but C# evaluates its arguments first. Examples: `action.GetMessage()` for every valid decision (`Bot.cs:244`), `result.Validate()` (`Bot_Move.cs:64`), `outcome.GetMessage()`, dictionary indexing (`Bot_Other.cs:836`) and `Count(...)` calls (`Bot_Other.cs:483/492`). Any of these can throw, and the throw counts. Port every argument expression and discard the result. Don't delete log calls. In `Do`, an exception from `GetMessage()` or from the `error` message's `ToString()` on the invalid or valid branch is outside the `try`, so it escapes deliberation (`botException`).
- **`Do` catches C# exceptions only.** Use `cs::catch`, which re-raises any panic that isn't a `CsException`. A `todo!` or a Rust indexing bug must never turn into "the bot chose nothing".
- **The logging target.** With `Message.DefaultDescriber` null, `LogError` calls `Console.WriteLine(msg, pars)`, which runs `string.Format` on any writer except `TextWriter.Null`. But the server sets `DefaultDescriber` (`Server/Program.cs:20`), so production calls `Console.WriteLine(DefaultSkin.Default.Format(msg, pars))`. With no `pars`, which is true of both call sites, `Format` returns `msg` unchanged, so production never formats and never throws. The oracle (`TextWriter.Null`) doesn't throw either, so the oracle already matches production and keeps `TextWriter.Null`. Details are in [csharp-hooks.md](csharp-hooks.md#bot-logging). The port's `log_error` must never throw. The arguments are still evaluated, and those can throw.
- **`OrderByDescending` is stable.** Ties keep source order. Sort with the comparison reversed. Never sort ascending and `.rev()`, which reverses ties. There are 5 sites in `Bot_Battle.cs` and 1 in `Bot_Other.cs`.
- **Bot-built events are validated in memory.** `Do` validates the bot's object, not a JSON copy. Build Rust events with the same field values, including dictionary insertion order (`ToDictionary` at `Bot_Other.cs:26` and `Bot_FactionSpecific.cs:210` feed `ForceLocations`, which serializes in order).
- **Reflection in `Bot_Alliance.cs:208-212`** reads `Game` and sets `AllyPermission` properties by name (12 names). Port it as a `match` on the name.
- **Neighbour cache.** Bots call `FindNeighbours` (`Bot_Knowledge.cs:296/302`, `Bot_FactionSpecific.cs:124`). The cache is shared with the queries before the deliberation and with the validation of the next event (hazards 6.1). Running the bots at the recorded point, on the live game, keeps it identical.
- **Bot state outlives undo.** `DecidedShipment` and `VoicePlan` can hold events of a game that was undone. Keep them as `Rc`s. Compare `VoicePlan` by reference, as C# does (hazards 16.8).

## Packages

Sizes are C# lines. Members means `todo!` bodies in the skeleton. Each package owns its `.rs` files, so they don't conflict. Cross-file calls go through the skeleton's signatures, and a call into an unported method panics with its location, as in phase 1.

| Pkg | Files | C# lines | Members | Notes |
|---|---|---|---|---|
| B0 harness | `Oracle/GameRunner.cs`, `Oracle/Recorder.cs`, `trace-format.md`, `rust/differ` | ~40 C#, ~300 Rust | | Trace format 2 and the differ changes above. Regenerate `traces/sample`. No bot code. |
| B1 core | `Bot.cs`, `BotParameters.cs`, `Bot_Knowledge.cs` | 312 + 270 + 732 = 1314 | 17 + 13 + 104 | First: `new`, `Do`, `D`, `LogInfo`/`LogError`, the five levels, `BotParameters`. Then Knowledge, the shared helpers that every other file calls (Shipment 76 calls, FactionSpecific 68, Battle 44, Other 39, Move 20). |
| B2 battle | `Bot_Battle.cs` | 1152 | 43 | Float math (hazards 16.7), `DetermineRockWasMelted`. |
| B3 factions | `Bot_FactionSpecific.cs` | 1233 | 45 | 53 draw sites, the most of any file. Voice and prescience plans, Tleilaxu revival state. |
| B4 other | `Bot_Other.cs` | 1103 | 54 | Nexus decisions (call into Shipment and Battle), clairvoyance unboxing, `SingleOrDefault` at :796. |
| B5 movement | `Bot_Shipment.cs`, `Bot_Move.cs` | 915 + 360 = 1275 | 41 + 10 | Shared state `DecidedShipment*`/`FinalDestination` (hazards 16.8), lazy chains with draws (16.4). |
| B6 small | `Bot_Bidding.cs`, `Bot_Deals.cs`, `Bot_Alliance.cs` | 169 + 209 + 242 = 620 | 5 + 12 + 14 | Reflection in Alliance, `DealOffered.Acceptance`. Smallest package, so it also takes the unboxing helper and `support.rs`. |

Order:

1. Start B0 and B1 together. B1 lands `bot.rs` and `parameters.rs` first. The dispatch is what the differ calls.
2. Start B2 to B6 once `bot.rs` is in. Only B1 edits `bot/knowledge.rs`. Others tell B1 which Knowledge helpers block them, and B1 ports in that order.
3. Each package works through its "bot decision differs" groups in `differ live --mode bots`. An engine bug found this way goes to the engine fixers with a `differ first` reproduction.

Every package follows PORTING.md's disk, CPU and memory rules. Bot-decision runs go through `rust/tools/heavy.sh` like any other `differ live` run.

## Decisions (2026-11-11, final)

1. **Trace format 2** with the `bots` query entry. Done (B0), see "B0 harness ready" below.
2. **Logging: production behaviour wins.** Production never formats in `LogError` (see "The logging target" above), so the oracle keeps `TextWriter.Null` and the Rust `log_error` must never throw.
3. **Bots that change the game while deciding are real** (the server runs bots on the live game), so the differ reproduces them by running the Rust bots on the live replay game. How often it happens: "B0 harness ready" below.
4. **Engine edits.** Bot porters may make trivial visibility changes in `engine/src` (adding `pub`), nothing else. Real engine fixes go to "Engine gaps" below, for the engine fixers.
5. **`rust/bots` stays in the workspace.**
6. **Bot decisions are checked in bots mode and in `client`-profile fuzz traces.** Until 2026-11-11 the differ skipped the `bots` entries of fuzz traces. See "Fuzz traces are bot-checked" below.

## Engine gaps

Engine bugs or missing engine functions that bot porters found. One line each: what, where, which bot method needs it, and a `differ first` reproduction if there is one. Engine fixers take them from here.

- (none yet)

## B1 core ready (2026-11-11)

`bot.rs`, `parameters.rs` and `bot/knowledge.rs` are ported. No `todo!` is left in them. B2 to B6 can start.

### Dispatch and `Do` (bot.rs)

- The five `determine_*_priority_in_phase_action(&mut self, game: &mut Game, events)` methods and `determine_end_phase_action` are the C# `||` chains, in C# order.
- `do_<T: EventKind>(&mut self, game: &mut Game, method: impl FnOnce(&mut ClassicBot, &mut Game) -> Option<Rc<T>>, action, allowed_actions) -> bool`. `T` is the C# declared return type, and `EventKind::TYPE` is `typeof(T)`. The `event_kind!` list in `bot.rs` implements the trait for every event type the dispatch uses.
  - A `Determine*` method that C# declares non-nullable is wrapped as `|b, g| Some(b.determine_bid(g))`.
  - `method` gets `&mut Game` (for `determine_rock_was_melted`). Every other method takes `&Game`.
  - Only `CsException` panics are caught, through `cs::catch`. A `todo!` or any other Rust panic propagates.
  - `GetMessage()` runs on both validation branches, outside the catch, as in C#.
- **If you change the signature of a `determine_*` method that the dispatch calls, change its line in `bot.rs` too.** That's the only cross-file edit to `bot.rs` I expect.

### Logging and randomness

- `ClassicBot::log_info(msg, pars)` and `log_info_message(msg)` do nothing. **Evaluate every argument before the call, as C# does.** An argument can throw.
- `ClassicBot::log_error(msg, pars)` follows production, as the coordinator decided. It runs .NET's `string.Format` parse (`bot::string_format_check`) and throws `FormatException` with the exact .NET 10 text. Only `Do` calls it.
- `self.d(game, amount, sides)` is `D(n, sides)`: 0 without a draw when either is 0, and it throws after counting the draw when `sides < 0`.
- `self.random` is the bot's `SharedRandom`. `RandomExt` (`random_or_default`, `highest_or_default`, `lowest_or_default`, `one_of_highest_n_or_default`, `one_of_lowest_n_or_default`) is in the bots prelude. Pass `&self.random`.

### Accessors (bot.rs)

`self.faction(game)`, `self.ally(game)`, `self.allied_player(game) -> Option<PlayerIndex>`, `self.resources(game)`, `self.player` (field), `self.param` (field, `Copy`).

### Knowledge helpers (bot/knowledge.rs)

All of them keep the skeleton's names and signatures. The overloads are:

| C# | Rust |
|---|---|
| `IsWinning(Player)` / `IsWinning(Faction)` | `is_winning` / `is_winning_faction` |
| `NotOccupiedByOthers(Location)` / `(Territory)` | `not_occupied_by_others` / `not_occupied_by_others_territory` |
| `NotOccupied(Territory)` / `(Location)` | `not_occupied` / `not_occupied_location` |
| `Vacant(Location)` / `(Territory)` | `vacant` / `vacant_territory` |
| `OccupiedByOpponent(Territory)` / `(Location)` | `occupied_by_opponent` / `occupied_by_opponent_location` |
| `GetOpponentThatOccupies(Location)` / `(Territory)` | `get_opponent_that_occupies` / `get_opponent_that_occupies_territory` |
| `AllyDoesntBlock(Territory)` / `(Location)` | `ally_doesnt_block` / `ally_doesnt_block_location` |
| `OccupyingOpponentsIn(.., Territory)` / `(.., Location)` | `occupying_opponents_in` / `occupying_opponents_in_game_player_location` |
| `IsWinningOpponent(Player)` / `(Faction)` | `is_winning_opponent` / `is_winning_opponent_faction` |
| `MaxDial(game, player, t, opponent, ignoreSpiceDialing = false)` | `ClassicBot::max_dial(game, p, t, opponent, false)` (pass the default) |
| `MaxDial(game, resources, battalion, opponent)` | `ClassicBot::max_dial_game_int32_battalion_faction` |
| `MaxDial(game, resources, forces, specialForces, p, opponent)` | `ClassicBot::max_dial_game_int32_int32_int32_player_faction` |

- Static C# members are associated functions: `ClassicBot::cards_unknown_to_player(game, p)`, `ClassicBot::can_ship(game, p)`, `ClassicBot::weapons(...)`, `ClassicBot::defenses(...)`, `ClassicBot::known_opponent_weapons(...)`, `ClassicBot::max_reinforced_dial_to(...)`, `ClassicBot::count_different_weapon_types(&cards)`, the clairvoyance helpers, and so on.
- Pass C# default parameter values yourself: `best_safe_and_nearby_resources(game, l, &b, false)`.
- The `Biggest*` properties return `Option<BattalionInLocation>`. `BattalionInLocation` is `Clone`, with public `location` and `battalion` fields.
- Knowledge calls these methods from other files: `get_dial_needed_territory_player_bool` (from `IsSafeAndNearby` and `WinnableNearbyStronghold`) and `determine_dial_shortage_for_battle` (Bot_Battle, B2). Until B2 ports them, the Knowledge helpers that reach them panic with B2's `todo!` location.

### B1 tests

- `bots/tests/core.rs` (golden `core.json`): `LogError` on a formatting console (52 strings, the exact .NET 10 `FormatException` texts and UTF-16 offsets), `D` over a grid including `sides` 0 and negative (results, exceptions, draw counts), `GetDefaultParameters` for every faction and some undefined values, and seven `Do<T>` scenarios run through reflection in C#: valid, not allowed, already decided, null, throws, throws with a brace in the message, invalid. There are also Rust-only checks that an invalid decision blocks the rest of the tier, that a `todo!` panic escapes `Do`, and that draws made before an exception are kept.
- `bots/tests/knowledge.rs` (goldens `knowledge-*.jsonl.gz`, 2.4 MB): replays four bot games and, every 20 to 35 steps, gives each player a freshly seeded bot and calls 89 Knowledge members with real arguments (~374k calls). It compares results, exceptions and the draw count after each call. A deliberate off-by-one in one helper made it fail, so it does catch differences. The golden files contain the events, so they don't depend on `traces/sample`.
- To regenerate, see the header comments of the two tests (`bots/csharp-ref`, `dotnet run -c Release -- core|knowledge ...`).
- Not covered yet, because no recorded state reached them: `WinnableNearbyStronghold` and `IsSafeAndNearby(mayFight: true)` (both need B2's `GetDialNeeded`), `BattalionThatShouldBeMovedDueToAllyPresence`, `BiggestMovableStackOfAdvisorsInStrongholdNearVacantStronghold`, `ResourcesFromEmperor` and the two clairvoyance-in-battle helpers. Those are only ever null or 0 in the goldens. The differ in bots mode will exercise them once their callers are ported.
- `differ live --mode bots --bots check` (seed 5, 4 games) runs the dispatch. Every game stops at the first unported decision (`Bot_Other.cs:794`, B4), reported as "not ported yet".

### Logging: the oracle doesn't match production yet

`log_error` formats, as production does. The oracle sets `Console.Out = TextWriter.Null` (`Oracle/Program.cs:47`), and `TextWriter.Null` doesn't format, so in the oracle `LogError` never throws. When a caught exception's message, a decision's `GetMessage()` or a validation message contains a brace, Rust throws `FormatException` out of `Do` and the oracle doesn't, so the differ reports a `bot` difference.. This needs to be fixed on the oracle side (B0): give the oracle a discard writer that does format, for example a `TextWriter` subclass that overrides only `Write(char)` with a no-op. The base `WriteLine(string, object[])` then runs `string.Format`. That is a test-harness change and doesn't change behaviour.

### Engine gaps

None found. Everything B1 needs was already `pub` in the engine, and B1 made no engine edits.

### Shared files B1 changed

- `bots/src/prelude.rs` re-exports `treachery_engine::extension_methods::RandomExt`.
- `bots/Cargo.toml` adds the dev-dependencies `serde_json` and `flate2` (already in the workspace lock).

## B6 ready (2026-11-11)

`bot/bidding.rs`, `bot/deals.rs` and `bot/alliance.rs` are ported, with no `todo!` left. B6 made no engine edits and found no engine gaps.

### What to know about these files

- `special_permissions_need_update` and `bool_permissions_need_update` take `&mut AllyPermission`, because C# writes to the permission they're given. `get_permission_properties` returns `Vec<&'static str>`.
- The reflection in `BoolPermissionsNeedUpdate` is two `match`es on the property name: one sets the `AllyPermission` field, the other reads the `Game` field. All 12 names exist as `bool` properties on both C# types, so `GetProperty` never returns null there.
- `DetermineCardTraded`'s `OrderBy(CardQuality).FirstOrDefault()` is a single pass that keeps the first card with the lowest quality, which is what a stable sort followed by `First` gives.

### `support.rs`: the unboxing casts

`support.rs` has the casts the bots apply to `ClairVoyancePlayed.Parameter1`/`Parameter2`: `unbox_enum::<E>`, `unbox_int`, `unbox_float` and `cast_hero`. They follow the CLR rules, which I probed on .NET 10:

- An unboxing cast of null throws NullReferenceException. `(IHero)null` is null.
- A boxed enum or `int` unboxes to any `int`-based enum and to `int`. So `(Faction)(object)TreacheryCardType.Laser` is `Faction.Fremen`, with no exception.
- `(float)o` accepts only a boxed `float`.
- Anything else throws InvalidCastException "Unable to cast object of type 'X' to type 'Y'.".

B2 (`bot/battle.rs`, `unbox_treachery_card_type`) wrote its own helper before `support.rs` existed (B4's `mod unbox` is gone; `bot/other.rs` uses `support.rs` now). Theirs throw InvalidCastException where .NET converts between enum types or between an enum and `int`, and their messages differ. I don't think any bot reaches those cases, because each cast matches the type that its clairvoyance question stores. Switching to `support.rs` would remove the doubt.

### B6 tests

- `bots/tests/b6_decisions.rs` (goldens `b6-*.jsonl.gz`, 1.7 MB). It replays the four games of the knowledge goldens. After every step (every second step in the two longer games), it gives each player a freshly seeded bot and calls the 14 B6 decisions, `DetermineAllowedResources`, `PlayerStanding` for every player, and `WannaHave` for the card on auction and each card in hand. That is about 375k calls. It compares the event JSON (or the exception) and the draw count after each call. The calls run in every phase, so they also check the exceptions these methods throw outside their own phase, such as `DetermineBid` with no card on auction. Changing `D(3, 3)` to `D(3, 2)` in `DetermineDealOffered_TellDiscardedTraitors` makes three of the four tests fail.
- `bots/tests/b6_unbox.rs` (golden `b6-unbox.json`) checks the four casts of `support.rs` on 11 kinds of boxed value against .NET.
- Both goldens come from `bots/csharp-ref` (`B6.cs`, modes `b6` and `b6unbox`). The regeneration commands are in each test's header comment.
- These games never produce `AllianceBroken` or `CardTraded` decisions, and `DealAccepted` only twice, so the differ is the only check of those.

## B2 ready (2026-11-11)

`bot/battle.rs` is ported, with no `todo!` left. B2 made no engine edits and found no engine gaps.

### What to know about this file

- `determine_best_defense` takes `most_effective_defense: &mut Option<TreacheryCard>`. C# declares the `out` non-nullable but assigns `ObjectCounter.Highest`, which can be null.
- `IsAllowedWithClairvoyance` uses `support::unbox_enum::<TreacheryCardType>` (B6's helper).
- `GetDialNeeded(p, ...)` computes for `Player`, not `p` (hazards 18.5), and `ChanceOfEnemyLeaderDying` draws again after picking a Mirror Weapon. Both are kept, with `// C# parity:` comments.
- Every `OrderBy`/`OrderByDescending` is a stable sort on keys computed once (`order_by`, `order_by_descending`, `order_by_f32` at the top of the file); descending sorts compare reversed, never sort-then-reverse.
- The float sites follow hazards 16.7: `CumulativeChance` in `f64`, `(count as f32 - n) as f64` for its first argument, `< 0.8` and `> 0.1` compared in `f64`, `Math.Ceiling(MaxReinforcedDialTo)` widened to `f64`. The `GetDialNeededForBattle` sum adds int terms to the float one at a time, left to right, as C# does.
- `determine_rock_was_melted` runs `Battle::determine_battle_outcome` on the live plans and evaluates `outcome.get_message()`, as C# does.

### B2 tests

- `bots/tests/b2_battle.rs` (goldens `b2-*.jsonl.gz`, 545 KB, from `csharp-ref/B2.cs`, mode `b2`). It replays the four knowledge-golden games. After every battle-phase step it gives each fighter a fresh seeded bot per call and calls the decisions that fit the state (plan, switch, HMS advantage, Thought, TreacheryCalled, PortableAntidote, BattleConcluded, LoserConcluded, BattleInitiated, Retreat, RockWasMelted, BattleClaimed). Every 20 steps it calls `GetDialNeeded` (both reinforcement flags, two strongholds plus a territory the opponent occupies), `DetermineDialShortageForBattle`, `NrOfUnknownOpponentCards`, `HeroesForBattle` and `CanOnlyUseTraitorsOrFacedancers` for every player. That is about 68k calls, compared on event JSON (or exception) and draw count. Changing the defender's 0.5 bonus to 0.4 makes about a quarter of the probes fail. A probe that reaches a `todo!` of another package is counted as blocked and printed, not failed (none are left).
- These games have no Ecaz, no Rock Melter and no retreat, so `BattleClaimed`, `RockWasMelted` and `Retreat` are covered only by the differ.
- `GetDialNeeded` is now ported, so B1 can add `WinnableNearbyStronghold` and `IsSafeAndNearby(mayFight: true)` to the knowledge probes.

### Differ

`differ live --mode bots --seed 31 --games 200 --jobs 4 --bots check`: 200 of 200 traces pass, 142,203 deliberations match, and no traces stopped at a bot difference. Seed 21 (50 games) passes too. It has 5 `RockWasMelted` decisions and 12 `BattleClaimed` decisions, all matching, with the state hash checked after every line.

The Rock Melter mutation itself isn't confirmed yet. I reran seed 21 with `--check-bot-purity true`, and none of the five `RockWasMelted` deliberations changed the game: no Mirror Weapon and no Diplomat. I regenerated fuzz seed 1 game 61, the case in known-issues.md, with the current fuzzer and `--check-bot-purity true`. It has no `botMutatedState` line any more, and `differ first` replays all 4881 lines with no difference, so the fuzzer no longer reaches the case there. Also, the differ never runs bots on fuzz traces (`bots_active` needs a bots-mode trace), so the fuzz cases can't be checked until it does. I left the known-issues entry in place. To confirm the mutation, find a bots-mode game flagged `botMutatedState` at `RockWasMelted` and replay it with `--bots check`.

## B0 harness ready (2026-11-11)

Trace format 2 and the differ's bot checking are in. How to use them is in [differ.md](differ.md#checking-bot-decisions), and the entry format is in [trace-format.md](trace-format.md#bot-deliberations).

### What changed

- **Oracle.** `DeliberateBots` records a `["bots", {...}]` query entry after `AskBots` returns or throws (`GameRunner.cs`, `Recorder.Bots`). `Trace.FormatVersion` is 2. `verify` and `SkinRef` skip the entry. With `--check-bot-purity true`, a flagged line also gets `botMutatedBy`: one `[player, faction, level, eventType]` for each bot ask that changed the state hash. This names the `Determine*` method that changed the game.
- **Differ.** `--bots check` (on `run`, `first` and `live`; the default is `skip`) runs the Rust bots at every `bots` entry of a bots-mode trace, on the live game, with a registry that survives undo. It compares the choice (player, level, event JSON byte for byte) or the exception, and each line's `botRandom`. The summary starts with `bot deliberations: N matched; M traces stopped at a bot difference` and lists the bot groups. `Outcome::bot_decisions_ok` and `Failure::is_bot` are public for scripts. Phase-1 replays are unchanged: without the flag, `bots` entries are skipped.
- **Tests.** `differ/tests/bots_harness.rs`: the entries parse, they change nothing without the flag, they never cause a bad trace, and a tampered choice or `botRandom` count fails with key `bot` or `botRandom`.
- **Sample corpus.** All 17 traces were regenerated with the format-2 oracle. Once the `bots` entries and `traceFormat` are dropped, every line is identical to the old corpus, so the skin and server-equiv goldens built on it still hold. `verify` passes. The corpus is 8.4 MB. With `--bots check`, 17 of 17 pass and 903 deliberations match.

### Bots that change the game: the purity run (decision 3)

`bots --seed 7 --games 100 --check-bot-purity true` (98 games ended, 2 stalled; 69,904 steps; 69,805 deliberations, each bot ask hashed before and after): **no deliberation changed the game, in any method.** `DetermineRockWasMelted` chose `RockWasMelted` 11 times in 10 games, and none of those changed the state. B2's seed 21 run found the same: 5 decisions, no change. So in bots mode, bots changing the game is rare, and so far it has only been seen in fuzz games, which have more varied battles.

The fuzz case from known-issues.md still reproduces with the current fuzzer: fuzz seed 102 game 193, steps 1669 and 1670, `botMutatedBy` `[6,"Tleilaxu(80)","Low","RockWasMelted"]`. That is the Tleilaxu bot's `DetermineRockWasMelted`, and it is the only method seen changing the game. The differ doesn't bot-check fuzz traces (decision 6), so as a one-off I relabeled a copy of that trace as bots mode. Without bots, `differ first --step 1680` reports `hash differs after DealAccepted` at line 1669 (the known issue). With `--bots check`, all 1681 lines match, including the bot decisions and draw counts. So the Rust bots reproduce the C# change to the game. To reproduce: `fuzz --seed 102 --start 193 --games 1 --dump final`, change `"mode":"fuzz"` to `"mode":"bots"` in the header, and run `differ first FILE --step 1680 --bots check`.

### Reply to B1: logging (decision 2)

The oracle stays on `TextWriter.Null`, because that is what matches production. The server sets `Message.DefaultDescriber = DefaultSkin.Default` (`Server/Program.cs:20`), so production's `LogError` takes the describer branch: `Console.WriteLine(DefaultSkin.Default.Format(msg, pars))`. Both call sites pass no `pars`, `Skin.Format` returns `msg` unchanged when `pars` is empty, and `Console.WriteLine(string)` doesn't format. So production never throws `FormatException` here, and neither does the oracle. A formatting discard writer would make the oracle throw where the server can't. **B1: `log_error` should never throw.** It should still evaluate its arguments, which happens before the call anyway. The `FormatException` goldens in `bots/tests/core.rs` test a path that production doesn't take. Details are in [csharp-hooks.md](csharp-hooks.md#bot-logging).

## Fuzz traces are bot-checked (2026-11-11)

`differ --bots check` now runs the Rust bots on fuzz traces too, as it does on bots traces. The oracle already recorded a `bots` entry for every deliberation in fuzz mode, so it didn't change, and the trace format stays at 2. The differ checks a trace when its header has trace format 2 or later and either mode `bots`, or mode `fuzz` with `fuzz.profile` `client` (`replay::bots_checkable`). Fuzz traces with the `wide` profile aren't checked, because they submit forged values.

This closes the Rock Melter false alarm in [known-issues.md](known-issues.md). Fuzz seed 102 game 193, seed 109 game 40, seed 110 game 151 and seed 116 game 23 report a `hash` difference without `--bots check` and match with it.

It also found one bot bug. After an undo, `Do` validated a `DealAccepted` built from a deal offer of the pre-undo game in the live game, where C# validates it in the offer's game ([escapes.md](escapes.md#bot-decisions-validated-in-the-live-game-instead-of-the-events-own-game-2026-11)). `ClassicBot::do_` now validates and describes each decision in the event's own game.

Results: `differ live --mode fuzz --seed 301 --games 200 --jobs 4 --bots check` passes 200 of 200 traces, with 495,223 deliberations matched (one game hung in C# and matches up to the hang). Seed 302 (50 games) passes too.

Tests in `differ/tests/bots_harness.rs`: the sample fuzz traces are parsed, replayed and bot-checked like the bots traces; a tampered fuzz choice fails with key `bot`; `wide` and format-1 traces aren't bot-checked; and two trimmed fuzz goldens in `differ/tests/golden` replay the Rock Melter case (seed 109 game 40) and the deal-after-undo case (seed 301 game 24).
