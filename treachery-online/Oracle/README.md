# Treachery.Oracle

Records traces of the C# engine for the Rust port to replay. The trace format is specified in [rust/docs/trace-format.md](../../rust/docs/trace-format.md), and the C# hooks and interventions are in [rust/docs/csharp-hooks.md](../../rust/docs/csharp-hooks.md).

## Build and run

```
export DOTNET_ROOT="$HOME/dotnet10" PATH="$HOME/dotnet10:$PATH"
dotnet build -c Release treachery-online/Oracle/Treachery.Oracle.csproj
ORACLE=treachery-online/Oracle/bin/Release/net10.0/Treachery.Oracle.dll

dotnet $ORACLE bots   --seed 1 --games 1000 --out rust/traces/bots
dotnet $ORACLE fuzz   --seed 1 --games 1000 --out rust/traces/fuzz
dotnet $ORACLE replay --in savegame.json --out rust/traces/replay
dotnet $ORACLE enums  --out rust/traces/sample/enums.json
dotnet $ORACLE verify --in rust/traces/sample
```

`verify` replays traces using only their contents, the way the Rust replayer does, and checks every recorded value.

Options for `bots` and `fuzz`:

- `--dump every|every:N|final|none`: when to write full state dumps (default `final`).
- `--hash every|none`: whether every step carries a state hash (default `every`).
- `--start I`: first game index. Game `i` depends only on the mode, the seed and `i`, so runs can be split.
- `--threads T`: games run in parallel (default: processor count).
- `--gzip true`: write `.jsonl.gz`.
- `--check-bot-purity true`: flag steps where bot deliberation, or the fuzzer building an event, changed the state hash (`botMutatedState`), and name each bot ask that did (`botMutatedBy`: player, faction, level, returned event type). It is slow (a state hash before and after every bot ask); use it for spot checks.
- `--hang-seconds S`: give up on a game whose `Execute`, bot deliberation or undo call runs longer than S seconds (default 120). The trace ends with reason `hung`, `botHung` or `undoHung`, the run finishes the other games and exits with code 4. Fuzzing finds real endless loops in the engine.
- `--discard true`: compute everything but write no trace files (only the summary). For benchmarks and coverage runs.
- `--fuzz-profile client|wide` (fuzz only): see below. Default `client`.
- `--memory-mb M`: when the oracle's managed heap goes over M MB (default 2048), the game whose engine call has run longest ends with reason `memoryLimit` (`botMemoryLimit`, `undoMemoryLimit`), the summary is written and the oracle exits with 5. A .NET thread can't be stopped, so the games still running lose their traces; `differ live` reruns them one at a time.
- `--status every`: before each round of the main loop, call `GameStatus.DetermineStatus` for every player and for an observer, as the server and client do, and record the calls and their results in the next line's `queries` (trace-format.md section 3, "Status queries"). `DetermineStatus` can change state, so a game recorded with it can play out differently from the same seed without it. Traces get several times larger.

## What the fuzzer does

A fuzz game is a bot game with interference. Its configuration is drawn more widely than a bots game: more than half of fuzz games use an older engine version (60% pick one side or the other of a `Version` check in Shared, or a uniform version), about a third get a rule set drawn from every `Rule` value, and a quarter run 10 to 20 turns. Bots games are unaffected, so bots traces are the same as before. `--versions latest` (on `bots` and `fuzz`) plays every game at `Game.LatestVersion` instead; the rest of the configuration is drawn as before. `differ live` always passes it, because the Rust engine supports only the latest version.

Each step, the fuzzer may:

- submit a random event (`FuzzSettings.RandomEvent`). Usually this is steered: it queries every player, picks an event type that has rarely been executed in the current phase of this game, and fills it from the engine's own helpers for valid values (`Valid*`, `Max*`, `GetValid*`, ...). It builds up to eight candidates and usually submits one that passes `Validate()`. Otherwise the event type and values are drawn from broad domains, invalid values included.
- replace the bots' move with a steered event when an event type that is rare in this phase is applicable right now (30% of bot steps). This catches short windows, like a faction's turn with a card in hand, before the bots move on.
- mutate the bot's event (`MutateBotEvent`), half the time to another valid value.
- undo, restore or dismiss (`Undo`).
- while a battle is in progress, play a battle action for a player in the battle (`BattleAction`, 25% of bot steps during a battle). It picks an applicable, rarely executed type from the actions around the fight (`Voice`, `Prescience`, `JuicePlayed`, `ResidualPlayed`, `KarmaEcazDial`, `AuditCancelled`, `FaceDanced`, `BattleRevision`, `TreacheryCalled`, ...). It fills the fields with steered values (in the `wide` profile it sometimes overwrites them from the battle: the plans' cards, the opponent's leaders and the battle territory). `JuicePlayed` usually gets `Aggressor` when that is one of its valid types, `AuditCancelled` usually `Cancelled`. Now and then it picks one of `KarmaEcazDial`, `JuicePlayed`, `ResidualPlayed` or `AuditCancelled` even when it isn't applicable.
- from version 188, when a plan withdrawn with `BattleRevision` had parts the opponent saw through prescience (`Game.PrescienceSeenPlans`), resubmit that plan as `Battle` with a seen element (leader, weapon, defense or dial) changed to another valid choice (30% of bot steps while that applies). The engine must reject it as "revealed by prescience and can't change".

Fuzz games use the `client` profile by default (`--fuzz-profile client`). Every event the fuzzer builds is one the client UI could send: ids of real objects, values from the event type's `Valid*`, `Max*` and similar helpers, or the player's own cards, leaders and force locations where the type has no helper, and only as the player's own faction. Invalid candidates are still submitted now and then, and must be rejected. `--fuzz-profile wide` also draws raw values from broad domains, including forged ids (-1, unknown cards), out-of-range amounts and garbage strings. Differences that need those are out of scope (PORTING.md, "Scope"), so use it only to look for crashes.

Right after setup it also submits a few events of arbitrary types. Most are rejected as not applicable, which exercises each event's `GetMessage()`.

Steered generation only calls static helpers and `Validate()`. `--check-bot-purity true` checks that this changes nothing, and `verify` checks that the traces replay from their contents. See [rust/docs/coverage.md](../../rust/docs/coverage.md) for what it reaches.

Traces are large (a long fuzz game with hashes only is several MB). Generate, check and delete them; see PORTING.md, "Disk space".

`replay` takes the same `--dump` and `--hash` options. Progress and a summary of end reasons go to stderr. Each run also writes a `summary-*.json`.

## Coverage

`Oracle/coverage.sh [BOT_GAMES] [FUZZ_GAMES] [SEED] [WORKDIR]` (environment: `HANG_SECONDS`, default 30) measures line coverage of `Treachery.Shared` over a bots and fuzz run, and prints the least-covered files. It needs coverlet:

```
dotnet tool install --global coverlet.console
treachery-online/Oracle/coverage.sh 300 200 100 /tmp/oracle-coverage
```

The script copies the build output first, because coverlet instruments assemblies in place, and runs without hashing or trace files to keep it fast and small. `dotnet-coverage collect` didn't work on macOS arm64: it reported "Profiler was not initialized". To summarize an existing Cobertura file, run `dotnet $ORACLE coverage --in coverage.cobertura.xml`.

The skin and status files (`DefaultSkin.cs`, `Skin.cs`, `GameStatus.cs`, `GameStatistics.cs`) are only used by the client and server, so the summary also reports coverage without them.
