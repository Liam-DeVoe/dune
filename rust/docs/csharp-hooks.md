# C# test hooks

PORTING.md allows the C# code to gain test hooks that change nothing when unused. This file lists every such hook, and every place where the oracle reaches into the engine without one.

## Hooks added to Shared and Bots

None. The oracle needed no change to `Shared` or `Bots`. `git diff` on those two directories is empty.

The one hook the hazard survey asked for, a way to seed bot randomness (hazards.md 2.4), already exists. `ClassicBot`'s constructor takes an optional `LoggedRandom? random` and falls back to `new LoggedRandom()` only when it is null (`Bots/Treachery.Bots/Classic/Bot.cs:12` and `:300`). No caller passed one before. The oracle passes `new LoggedRandom(seed)` for every bot (trace-format.md section 9).

## Hooks added to Server

The server equivalence harness (`treachery-online/ServerOracle`; its Rust side, `rust/server-equiv`, was removed on 2026-11-11 when the server was rewritten, see [server.md](server.md)) runs the real `GameHub` on a virtual clock. It needs five hooks, all in `Server/TestHooks.cs`. Each is null by default, and every call site then runs the line it ran before, unchanged. The ServerOracle sets them for a run and clears them afterwards.

| Hook | Replaces | Call sites |
|---|---|---|
| `Clock` | `DateTimeOffset.Now` and `DateTime.Now` | Every one in `Server`: the hub files and `LoggedInUser.cs`. They now call `TestHooks.Now()` and `TestHooks.LocalNow()`. With a clock set, `LocalNow()` is `Clock().LocalDateTime`, so the harness runs with `TZ` matching the clock's offset |
| `NewToken`, `NewGuid` | `GenerateToken`'s `RandomNumberGenerator` loop and `Guid.NewGuid().ToString()` | `GameHub.cs` (`GenerateToken`), `RequestCreateGame` and `RequestScheduleGame` (through `TestHooks.NewGuidString()`) |
| `Delay` | `_ = Task.Delay(ms).ContinueWith(_ => action())` | `SendAsyncPlayMessagesIfApplicable` and `ScheduleBotEvent`. The original line is kept in the `else` branch. The hook also gets a name for the delayed call, such as `"SendAsyncPlayMessagesIfApplicable <gameId>"` |
| `MailSink` | `SmtpClient.SendMailAsync` in `SendMail` | `GameHub.cs`, after the `GameEndEmailUsername` check and inside the same try/catch |
| `NewRandom` | The bots' unseeded `new LoggedRandom()` | `PerformBotEvent` (the bot order: with the hook set it builds the `Deck<Player>` on the hook's random and shuffles it, which is what `Deck<Player>.Randomize` does on a new unseeded one; the original line is kept in the `else` branch) and `GetOrInitializeBot` (passes `TestHooks.NewRandom?.Invoke(...)` as the `ClassicBot` constructor's optional `random`, which is null without the hook, as before). The hook gets a name for the use: `"BotOrder <gameId>"` or `"Bot <gameId> <faction>"`. The ServerOracle returns `new LoggedRandom(n)` for n = 1, 2, 3, ... and records the names in each step's `randoms` |

`ConnectionInfo.Created` (`Shared/Model/UserConnections.cs`) also reads `DateTimeOffset.Now`. Shared has no hook for it; see the ServerOracle interventions below.

### ServerOracle interventions

These are in `treachery-online/ServerOracle` and use reflection or public members. None of them changes the server's code.

1. **Static state reset.** The hub keeps its state in static properties. Before each scenario and at every `restart` step, the harness clears the `ConcurrentDictionary`s, empties the arrays, and sets the `DateTimeOffset`s to `default` and `Restoring` to false. It fails on a static property of any other type, so a new static can't leak from one scenario into the next.
2. **One hub object per call, disposed afterwards**, as SignalR does. `Context`, `Clients` and `Groups` are fakes set through the hub's public setters. Delayed calls keep the disposed hub object, as in production.
3. **`ConnectionInfo.Created` aged by reflection.** When a scenario moves the virtual clock forward, the harness moves each `Created` back by the same amount (the `<Created>k__BackingField`). Its age measured against the real clock then equals its virtual age, which is what the Rust port computes with the hub clock. Scenarios should not advance to exactly 90 days after a connection was made: the real time that passes during the run would decide that case.
4. **`SubmitEvent` dispatch.** The harness calls the hub method whose third parameter has the event's type. The method names don't always match the class names, for example `RequestClairvoyancePlayed` takes a `ClairVoyancePlayed`.
5. **Reflection reads**: the hub's static properties, `UserConnections.ConnectionInfoByGameId` and each bot's `ClassicBot._random` (its draw count, in the `games` section), for the transcript.
6. **Process settings**: invariant culture and `Message.DefaultDescriber = DefaultSkin.Default`, as `Program.Main` sets it.

## Unseeded randomness, and how the oracle avoids it

| Source | Where | What the oracle does |
|---|---|---|
| `Game.Random` | Created from `EstablishPlayers.Seed` (`EstablishPlayers.cs:190`) | Nothing needed. The seed is part of the event, and the oracle sets it from the game seed. |
| Bot `_random` | `Bot.cs:300` | Passes a seeded `LoggedRandom` to the constructor. |
| `Deck<T>.Randomize` (`new LoggedRandom()`) | Only `Server/GameHub_GameEvents.cs` `PerformBotEvent`, to order bots | Not called. The oracle shuffles bot order with its own seeded `System.Random` and records the order in `botOrder`. The ServerOracle seeds it through `TestHooks.NewRandom`. |
| `ExtensionMethods.Random` (static `new Random()`) | Only reachable through `RandomOrDefault()` without arguments, the selector-only `HighestOrDefault` family and `TakeRandomN`. Nothing in Shared or Bots calls them. | Nothing needed. |
| `Message.Counter`, `LeaderState.moment` | Process-wide static counters behind `Message.Nr` and `LeaderState.TimeOfDeath` | Left out of the state dump (trace-format.md section 8). Nothing in Shared or Bots reads either field. |

I checked determinism by running each mode twice. Same command twice, and the same games with a different `--threads` value, or split with `--start`, give byte-identical trace files.

## Oracle-side interventions

These live in `treachery-online/Oracle` and use reflection or public members. They don't modify the engine, but a replayer must know about the first three because they affect state.

1. **Neighbour cache marked stale before every `Execute`.** `StepExecutor.InvalidateNeighbourCache` sets the private `Map._neighbourCacheTimestamp` to -1. The cache is otherwise filled by bot deliberation and by the oracle's own queries, which a replayer can't see (hazards.md 6.1). Setting it to -1 makes the first `FindNeighbours` call in the event clear the cache, exactly as a `History.Count` change would.
   It differs from production only after an event that threw. There, production can serve a result cached before the throw, at the same `History.Count`, to the next event. The trace semantics are "the cache starts cold at every event".
2. **Marker in `Game.RecentMilestones` during `Execute`.** The oracle appends `(Milestone)int.MinValue` before calling `Execute` and removes it afterwards if it is still there. `GameEvent.Execute` clears that list exactly when it starts executing, so this tells a validation exception from an execution exception. Only `GameStatus` (client UI) reads `RecentMilestones`, never validation, so the marker can't change any outcome. It is never present in a dump.
3. **The executed event is a deserialized copy.** The oracle serializes every event (bot, host or fuzz), deserializes it, calls `Initialize(game)` on the copy and executes that. So the engine never sees a value that isn't in the trace's `event` JSON. In production, bot events are executed as constructed.
4. **Extra `GetApplicableEvents` calls.** After every step the oracle calls `GetApplicableEvents` for every player and for null, with and without host rights. These calls can change state, because they read `Game.NextPlayerToBattle` (hazards.md 4.3). They are part of the trace semantics (trace-format.md section 3), and the hash is taken after them. All other calls the oracle makes are listed in each line's `queries`.
5. **Reflection reads, no writes:** `Game.Random` (internal), `ClassicBot._random` (bot draw counts), `Map._locations` (the location list in id order).
6. **Process settings.** The oracle builds with `InvariantGlobalization` and also sets `CultureInfo.CurrentCulture`, `CurrentUICulture`, `DefaultThreadCurrentCulture` and `DefaultThreadCurrentUICulture` to the invariant culture (hazards.md 11.2). It checks at startup that `Message.DefaultDescriber` is null (hazards.md 6.3). It redirects `Console.Out` to `TextWriter.Null` during runs, because bots print diagnostics there. See "Bot logging" below for why this matches production.

## Bot logging

Decision (2026-11-11): bot logging must behave as in production, because real games run on the server.

`ClassicBot.LogError(msg, pars)` (`Bot.cs:291`) has two branches:

- `Message.DefaultDescriber != null`: `Console.WriteLine(Message.DefaultDescriber.Format(msg, pars))`.
- otherwise: `Console.WriteLine(msg, pars)`, which runs `string.Format(msg, pars)` on any writer except `TextWriter.Null` (its override does nothing). `string.Format` throws `FormatException` on an unbalanced brace in `msg`.

The server sets `Message.DefaultDescriber = DefaultSkin.Default` first thing in `Server/Program.cs:20`, so production takes the first branch. Both `LogError` calls (`Bot.cs:232` and `:240`) pass no `pars`, and `Skin.Format` returns `msg` unchanged when `pars` is empty (and catches any exception otherwise). `Console.WriteLine(string)` doesn't format. So in production `LogError` never throws, whatever braces the exception text or the event message contain.

The oracle keeps `DefaultDescriber` null (hazards.md 6.3) and takes the second branch, on `TextWriter.Null`. That doesn't format either, so it never throws: the same outcome as production. Swapping in a `StreamWriter` on `Stream.Null`, as first proposed, would make the oracle's `LogError` throw where the server's can't, so the oracle keeps `TextWriter.Null`.

What still matters is the arguments: the interpolated string is built before `LogError` runs, so `action.GetMessage()` and the `error` message's `ToString()` are evaluated (and can throw) in both settings. `LogInfo` returns before formatting (`LogBotInfo` is false), but its arguments are evaluated too. The Rust bots port both: evaluate every argument, never format.

## Bots that change engine state

Bot deliberation can change the game: `DetermineRockWasMelted` runs `Battle.DetermineBattleOutcome` on the live plans (bots-plan.md, "Bots are not pure"). The server runs bots on the live game too, so this is real behaviour. A replayer that only executes events misses the change. Trace format 2 records each deliberation (`bots` query entry), and `differ --bots check` runs the Rust bots at that point to reproduce it.

`--check-bot-purity true` measures how often it happens. It hashes the state before and after each deliberation, and before and after each bot ask. It marks the next line `botMutatedState` and names each ask that changed the state in `botMutatedBy`. Runs with the check on:

- 64 bot games (50,110 steps) and 24 fuzz games (30,359 steps), before format 2: no flag.
- 100 bot games (seed 7, 69,805 deliberations, 2026-11-11): no flag. 11 of the deliberations chose `RockWasMelted`.
- Fuzz seed 102 game 193 (current fuzzer): steps 1669 and 1670 are flagged, `botMutatedBy` `[6,"Tleilaxu(80)","Low","RockWasMelted"]`, which is `DetermineRockWasMelted`. Fuzz seed 1 game 61, flagged by an older fuzzer, no longer is.
