# C# engine coverage of the trace corpus

The Rust port is checked by replaying traces from the C# oracle, so rules code the traces never run is ported unchecked. This page says how much of `Treachery.Shared` the bots and fuzz traces run, and why each remaining line isn't run.

## Numbers

Line coverage of the engine files, which is all of `Treachery.Shared` except `DefaultSkin.cs`, `Skin.cs`, `GameStatus.cs` and `GameStatistics.cs` (13,552 lines):

| Corpus | Covered |
|---|--:|
| 300 bots + 200 fuzz games, old fuzzer | 88.3% (11,963) |
| 300 bots + 200 fuzz games, new fuzzer | 91.0% (12,335) |
| 300 bots + 1,000 fuzz games, new fuzzer | 92.0% (12,472) |

The 1,080 lines the 1,000-game corpus misses break down as:

| Class | Lines | Meaning |
|---|--:|---|
| (a) unreachable | 153 | Dead code, a check that can't fail, or a `Version` below 100. |
| (b) old versions only | 0 | Fuzz games now play both sides of every version check, see below. |
| (c) reachable, not reached | 358 | Rules code that needs a rare combination of state. 57 of these lines were reached in other runs (other seeds, or earlier versions of the new fuzzer), marked "Reached with other seeds" below. |
| (d) outside the event pipeline | 569 | Called only by the client, the server, `GameStatus` or bots: lobby and seating code, savegame loading, `Clone`, text for the UI and e-mails. No event execution reaches it, so no trace can check it. |

So the rules code a trace can reach is 13,552 - 153 - 569 = 12,830 lines, and 1,000 fuzz games cover 97.2% of it.

Coverage was measured with coverlet over the oracle's `bots` and `fuzz` commands with `--hash none --discard true`; bots seed 100, fuzz seed 100 for the 200-game runs, and fuzz seeds 101 to 104 (250 games each) for the 1,000-game run. The bots part is the same in all three rows: the fuzzer changes don't touch bots mode.

## Client profile (in scope)

Only differences that real games can produce are in scope (PORTING.md, "Scope"). Since November 8 the fuzzer uses the `client` profile by default. Every event it builds could come from the client UI: ids of real objects, values from the event type's own helpers, the player's own faction. Invalid candidates are still submitted now and then, to check that both engines reject them. The old behaviour, which also draws forged ids, out-of-range amounts and garbage strings, is `--fuzz-profile wide`.

The fuzzer also got battle-action steering. While a battle is on, it plays an applicable, rarely executed action around the fight for a player in the battle (Voice, Prescience, Juice, Residual, Karama dial, audit, face dancing, plan revision, traitor calls). `JuicePlayed` usually gets `Aggressor` when that's a valid type, and `AuditCancelled` usually `Cancelled`.

300 bots + 200 fuzz games, seed 100, Shared at version 188, measured on November 8:

| Fuzz profile | Engine lines covered (of 13,605) |
|---|--:|
| client (default) | 89.9% (12,233) |
| wide, plus 2% unvalidated steps and forged ids (a short-lived experiment, now removed) | 90.9% (12,364) |

From version 188 the fuzzer also resubmits a plan that was withdrawn after the opponent saw parts of it through prescience, with a seen element changed. The engine must reject it as "revealed by prescience and can't change". In 40 fuzz games (seed 5, 15 of them at version 188) that rejection came up 14 times. The coverage run above predates this move.

Two trace format changes came with this work (rust/docs/trace-format.md). A `GetApplicableEvents` query that threw now records the exception as a third element of its `queries` entry, and replayers must throw the same. And the oracle has a memory guard, with end reasons `memoryLimit`, `botMemoryLimit` and `undoMemoryLimit`.

These numbers aren't comparable with the table above. Shared changed in between (version 188, `AlliancePassed`), and with it 91 of the 300 bots games now stall instead of finishing, so the bots part covers less than before.

The files the review of package I pointed at (rust/docs/reviews/pkg-i.md), in the client profile:

| File | Lines | Covered | Still uncovered |
|---|--:|--:|---|
| `Events/ResidualPlayed.cs` | 19 | 100.0 | |
| `Events/AuditCancelled.cs` | 31 | 96.8 | 65 (`GetCardsThatMayBeAudited` with no plan for the auditee) |
| `Events/JuicePlayed.cs` | 41 | 95.1 | 97-98 (Aggressor juice played by the defender, which swaps the plans) |
| `Events/KarmaEcazDial.cs` | 14 | 42.9 | 41-50 (`ExecuteConcreteEvent`: Ecaz in a battle under advanced Karama, with a Karama card and a plan without weapon or defense) |
| `Events/Voice.cs` | 74 | 97.3 | 145-146 |
| `Events/FaceDanced.cs` | 84 | 97.6 | 78, 90 |
| `Events/Prescience.cs` | 20 | 100.0 | |
| `Events/RicheseGaveCard.cs` | 19 | 100.0 | |

In the wide run with unvalidated steps, `KarmaEcazDial.cs` reached 100%, but only through `Execute(false, ...)` of an event that was never applicable, which is out of scope. The other differences between the two profiles are under 20 lines per file. The biggest are `Retreat.cs` (18 lines more in the wide run), a file that already varied a lot from seed to seed, and `BattleConcluded.cs` (15 lines more in the client run).

The unvalidated experiment found one thing worth knowing for the harness. A `PerformHmsMovement` to the HMS itself, executed without validation, attaches the HMS to itself. `AttachedLocation.Sector` then recurses until the stack overflows, which kills the whole oracle process with every game in it. Validation rejects that move, so it's out of scope.

## Commands

```
export DOTNET_ROOT="$HOME/dotnet10" PATH="$HOME/dotnet10:$PATH:$HOME/.dotnet/tools"
dotnet tool install --global coverlet.console    # once

# 300 bots + 200 fuzz games, prints totals and the least covered files:
treachery-online/Oracle/coverage.sh 300 200 100 /tmp/oracle-coverage

# 300 bots + 1,000 fuzz games in four parts, merged:
O=treachery-online/Oracle
dotnet build -c Release $O/Treachery.Oracle.csproj
W=/tmp/oracle-coverage-1000; mkdir -p $W; cp -R $O/bin/Release/net10.0 $W/bin
run() { coverlet $W/bin --target dotnet --targetargs "$W/bin/Treachery.Oracle.dll $1 --seed $2 --games $3 --out $W/out --hash none --discard true --hang-seconds 30" --include "[Treachery.Shared]*" --single-hit "${@:4}"; }
run bots 100 300 -f json -o $W/acc.json
for s in 101 102 103; do run fuzz $s 250 --merge-with $W/acc.json -f json -o $W/next.json; mv -f $W/next.json $W/acc.json; done
run fuzz 104 250 --merge-with $W/acc.json -f cobertura -o $W/coverage.cobertura.xml
dotnet $O/bin/Release/net10.0/Treachery.Oracle.dll coverage --in $W/coverage.cobertura.xml
rm -rf $W/bin $W/out $W/acc.json
```

A fuzz run with hung games exits with code 4, and coverlet then reports a failure, but the coverage file is still written. Delete the output directories afterwards; see PORTING.md, "Disk space".

To check that the fuzzer's own calls into the engine change no state, and that its traces replay:

```
ORACLE=treachery-online/Oracle/bin/Release/net10.0/Treachery.Oracle.dll
dotnet $ORACLE fuzz --seed 3 --games 16 --out /tmp/f --gzip true --dump none --check-bot-purity true
gzcat /tmp/f/*.gz | grep -c botMutatedState     # 0
dotnet $ORACLE verify --in /tmp/f                # 0 mismatching
```

On the final build I ran 16 fuzz games (seed 11, about 36,000 trace lines) this way: no flagged step and no mismatch. The same 16 games run on 3 threads without the purity check gave byte-identical traces.

## What changed in the fuzzer

The fuzzer is in `treachery-online/Oracle/Fuzzer.cs` and the new `FuzzerSteering.cs`, and the per-game setup is in `GameConfig.cs` and `GameRunner.cs`. The Oracle README describes it. In short:

- **Values come from what is currently valid.** For each event field, including the typed properties behind id fields (`Card`, `Target`, `ForceLocations`, ...), the generator calls the event type's own static helpers by reflection: `Valid*`, `Max*`, `GetValid*`, `GetAvailable*`, `GetStandard*`. It matches helper to field by type and name and fills helper parameters from the game, the player and the event's other fields. Force dictionaries come from the player's battalions or reserves. It builds up to eight candidates and usually submits one that passes `Validate()`.
- **Rare event types are preferred.** A steered step queries every player, then weights each applicable (event type, current phase) pair by how rarely this game has executed it. When the bots are about to move and an event type that is rare in this phase is applicable, the fuzzer sometimes takes that window instead.
- **Wider configurations.** In fuzz games only: about 45% of games pick a version just at or below a `Version` check in Shared, 15% a uniform version, 40% the latest. A third get a rule set drawn from every `Rule` value. A quarter run 10 to 20 turns.
- **More event kinds.** Events are sometimes created with their `(Game, Faction)` constructor, as the client does. Right after setup a few events of arbitrary types are submitted, which reaches each event's `GetMessage()` through the "not applicable" rejection. Undo now usually goes back a few events, and restore and dismiss are also called without a preceding undo.
- Mutation of bot events now draws the new value from the valid choices half the time.

All of this stays deterministic. Generation only uses the per-game `System.Random`, sorted reflection metadata and engine state.

## Trace format changes

The format version stays 1. Two changes, both in rust/docs/trace-format.md:

1. **New end reason `undoHung`.** Fuzzing found an `Undo` call that never returned: it re-executes the history and loops in `Bid` via `PlayerSequence.DetermineCurrentPlayer`, although the original game had moved past that event. The cause is not diagnosed yet. The watchdog now also covers `Undo`, `RestoreRecentlyUndone` and `DismissRecentlyUndone`. Such a trace ends with `{"kind":"end","reason":"undoHung","detail":…,"steps":…,"queries":[…],"undoKind":"undo","untilEventNr":N}`. `undoKind` is `undo`, `restoreRecentlyUndone` or `dismissRecentlyUndone`, and `untilEventNr` is present only for `undo`. There are no after-state keys. A replayer makes the queries, then the call, which should not terminate. The verifier treats it like `hung`.
2. **`botMutatedState`** (only with `--check-bot-purity true`) now also flags state changes made while the fuzzer built an event.

The summary file now uses the end reason (`hung`, `botHung` or `undoHung`) for hung games instead of always `hung`, and its detail text no longer says "Execute".

Bots traces are byte-identical to before: I regenerated `bots-2026-00011`, `-00020` and `-00040` and compared them with rust/traces/sample. Fuzz traces for a given seed are different from before. The fuzz traces in rust/traces/sample are still valid traces, but regenerating them with the same seed now gives other games.

## Other findings

- **Endless loops in `Execute`.** Two of the 1,000 fuzz games ended as `hung`, and so did 2 of 200 in a run with an earlier version of the new fuzzer. The cut-off was 30 seconds, so these are endless loops or very slow `Execute` calls. Their traces end with the event that didn't return.
- **Bots throw more.** Fuzz games ended with `botException` 7 to 18 times per 250 games in my runs, against 5 per 200 with the old fuzzer. I haven't looked at them. Presumably the bots meet states that only steered events create. Each one ends a game early, which costs coverage, and they are worth a look when the bots are ported.
- **Rare paths depend on the seed.** `Retreat.cs` is at 87.5% in one 200-game run and 54.2% in the 1,000-game run. More games help; the (c) lines need targeted steering.
- **Nine lines the old fuzzer reached are not reached now**, all rare: `BlackMarketBid.cs` 183-185, `ClairVoyanceAnswered.cs` 102, `ClairVoyancePlayed.cs` 185, `Game_Move.cs` 571-572 and `Messiah.cs` 48 and 50. The `Messiah.cs` lines are its `ToString`, which the old fuzzer probably reached by putting the Messiah into events more often.

## GameStatus.cs and GameStatistics.cs

`GameStatistics.GetStatistics` is called only by `Test/Tests.cs`. Neither the engine, the client nor the server uses it, so the port doesn't need it and it stays out of the measurement.

`GameStatus.DetermineStatus` is called by the client (`Client/Client.cs:947`, `UndoSession.cs:66`, `SceneGameService.cs:154`) and by the server, on the live game, when it sends async-play e-mails (`Server/GameHub_GameEvents.cs:277`). It is not read-only. In the `BattleReport` and `BattlePhase` phases it reads `Game.NextPlayerToBattle` (`GameStatus.cs:482-483`, `604-605`), which advances `BattleSequence` (hazards.md 4.3). So when the server computes a status, later events see a different battle sequence. Apart from that it only reads state, including `RecentMilestones` for its flash messages.

The port needs `GameStatus` for the client and for the server's e-mails, and its side effect matters for the server. Its 661 lines are 0% covered because the oracle never calls it.

**Recording it.** The oracle now does this when given `--status every`: it calls `DetermineStatus` for every player and for an observer before each round of its main loop, and records each call and its canonical result as a `queries` entry, so the replayer repeats the calls in order (trace-format.md section 3, "Status queries"). `differ live --status every` compares the results. The coverage numbers on this page were measured without it, so `GameStatus.cs` is still 0% here.

## Per-file coverage

Before: 300 bots + 200 fuzz games with the old fuzzer. After: the same with the new fuzzer. The last column is 300 bots + 1,000 fuzz games. Engine files only, sorted by the last column.

| File | Lines | Before | After | After, 1000 fuzz games |
|---|--:|--:|--:|--:|
| `Events/GameInfo.cs` | 23 | 0.0 | 0.0 | 0.0 |
| `GameChatMessage.cs` | 15 | 0.0 | 0.0 | 0.0 |
| `Model/AdminInfo.cs` | 4 | 0.0 | 0.0 | 0.0 |
| `Model/AvailableSeatInfo.cs` | 3 | 0.0 | 0.0 | 0.0 |
| `Model/ChatMessage.cs` | 2 | 0.0 | 0.0 | 0.0 |
| `Model/GameInitInfo.cs` | 4 | 0.0 | 0.0 | 0.0 |
| `Model/GameState.cs` | 9 | 0.0 | 0.0 | 0.0 |
| `Model/GlobalChatMessage.cs` | 8 | 0.0 | 0.0 | 0.0 |
| `Model/LoggedInUserInfo.cs` | 4 | 0.0 | 0.0 | 0.0 |
| `Model/LoginInfo.cs` | 5 | 0.0 | 0.0 | 0.0 |
| `Model/ManagedGame.cs` | 17 | 0.0 | 0.0 | 0.0 |
| `Model/ObserverJoined.cs` | 5 | 0.0 | 0.0 | 0.0 |
| `Model/ObserverRejoined.cs` | 2 | 0.0 | 0.0 | 0.0 |
| `Model/PlayerJoined.cs` | 3 | 0.0 | 0.0 | 0.0 |
| `Model/PlayerRejoined.cs` | 2 | 0.0 | 0.0 | 0.0 |
| `Model/Point.cs` | 9 | 0.0 | 0.0 | 0.0 |
| `Model/Result.cs` | 4 | 0.0 | 0.0 | 0.0 |
| `Model/ScheduledGame.cs` | 10 | 0.0 | 0.0 | 0.0 |
| `Model/ScheduledGameInfo.cs` | 11 | 0.0 | 0.0 | 0.0 |
| `Model/ServerInfo.cs` | 2 | 0.0 | 0.0 | 0.0 |
| `Model/ServerStatus.cs` | 4 | 0.0 | 0.0 | 0.0 |
| `Model/UserConnections.cs` | 16 | 0.0 | 0.0 | 0.0 |
| `Model/UserInfo.cs` | 5 | 0.0 | 0.0 | 0.0 |
| `Enumerations.cs` | 36 | 5.6 | 5.6 | 5.6 |
| `Model/Utilities.cs` | 49 | 8.2 | 8.2 | 8.2 |
| `Game_Participation.cs` | 75 | 21.3 | 21.3 | 21.3 |
| `Model/Log/ShipMoveLog.cs` | 27 | 25.9 | 25.9 | 25.9 |
| `ExtensionMethods.cs` | 66 | 31.8 | 31.8 | 31.8 |
| `Model/Messiah.cs` | 13 | 61.5 | 30.8 | 38.5 |
| `Model/ObjectCounter.cs` | 33 | 51.5 | 51.5 | 51.5 |
| `Events/ResourcesTransferred.cs` | 17 | 35.3 | 47.1 | 52.9 |
| `Events/CardGiven.cs` | 15 | 40.0 | 53.3 | 53.3 |
| `Events/Retreat.cs` | 48 | 43.8 | 87.5 | 54.2 |
| `Model/Timer.cs` | 7 | 57.1 | 57.1 | 57.1 |
| `Events/AutomationConfigured.cs` | 42 | 45.2 | 66.7 | 66.7 |
| `LoggedRandom.cs` | 9 | 66.7 | 66.7 | 66.7 |
| `Model/Expression.cs` | 3 | 66.7 | 66.7 | 66.7 |
| `Model/Payment.cs` | 21 | 66.7 | 66.7 | 66.7 |
| `Model/Homeworld.cs` | 30 | 73.3 | 73.3 | 73.3 |
| `Events/AuditCancelled.cs` | 31 | 71.0 | 96.8 | 74.2 |
| `Events/LoserConcluded.cs` | 81 | 65.4 | 75.3 | 75.3 |
| `Events/RicheseSpecifiesAuction.cs` | 45 | 75.6 | 75.6 | 75.6 |
| `Model/Message.cs` | 69 | 76.8 | 76.8 | 76.8 |
| `Model/ResourceCard.cs` | 13 | 76.9 | 76.9 | 76.9 |
| `Events/BlackMarketBid.cs` | 119 | 66.4 | 84.0 | 79.8 |
| `Events/ChoamExtraMove.cs` | 17 | 70.6 | 82.4 | 82.4 |
| `Events/DealOffered.cs` | 88 | 69.3 | 83.0 | 83.0 |
| `Events/ChoamFreeRevivalPrevention.cs` | 18 | 33.3 | 83.3 | 83.3 |
| `Events/ChoamMovePrevention.cs` | 21 | 38.1 | 85.7 | 85.7 |
| `Game_Deals.cs` | 21 | 85.7 | 85.7 | 85.7 |
| `Model/Fight.cs` | 7 | 85.7 | 85.7 | 85.7 |
| `Events/GameEvent.cs` | 62 | 87.1 | 87.1 | 87.1 |
| `Events/NexusPlayed.cs` | 342 | 73.1 | 78.9 | 87.1 |
| `Events/ChoamRemoveForce.cs` | 39 | 84.6 | 87.2 | 87.2 |
| `Game.cs` | 522 | 87.4 | 87.7 | 87.7 |
| `Model/Territory.cs` | 33 | 87.9 | 87.9 | 87.9 |
| `Model/Battalion.cs` | 35 | 88.6 | 88.6 | 88.6 |
| `Events/FactionSelected.cs` | 20 | 90.0 | 90.0 | 90.0 |
| `Model/Report.cs` | 20 | 90.0 | 90.0 | 90.0 |
| `Game_Move.cs` | 419 | 86.9 | 86.6 | 90.5 |
| `Model/Location.cs` | 21 | 90.5 | 90.5 | 90.5 |
| `Events/ClairVoyanceAnswered.cs` | 43 | 93.0 | 93.0 | 90.7 |
| `Model/Log/BlowLog.cs` | 11 | 90.9 | 90.9 | 90.9 |
| `Model/Player.cs` | 333 | 89.8 | 91.6 | 91.6 |
| `Model/Deck.cs` | 37 | 91.9 | 91.9 | 91.9 |
| `Model/Leader.cs` | 25 | 92.0 | 92.0 | 92.0 |
| `Events/RaiseDeadPlayed.cs` | 78 | 78.2 | 91.0 | 92.3 |
| `Events/EstablishPlayers.cs` | 197 | 91.9 | 92.4 | 92.4 |
| `Events/BattleConcluded.cs` | 161 | 77.6 | 90.7 | 92.5 |
| `Events/FaceDanced.cs` | 84 | 83.3 | 95.2 | 92.9 |
| `Events/TerrorPlanted.cs` | 58 | 87.9 | 87.9 | 93.1 |
| `Game_Storm.cs` | 116 | 93.1 | 93.1 | 93.1 |
| `Events/BattleInitiated.cs` | 88 | 92.0 | 93.2 | 93.2 |
| `Game_Blow.cs` | 251 | 92.0 | 92.8 | 93.2 |
| `Events/Captured.cs` | 15 | 93.3 | 93.3 | 93.3 |
| `Events/DealAccepted.cs` | 78 | 91.0 | 93.6 | 93.6 |
| `Events/Donated.cs` | 47 | 85.1 | 93.6 | 93.6 |
| `Game_LeaderState.cs` | 64 | 93.8 | 93.8 | 93.8 |
| `Game_Rules.cs` | 351 | 93.7 | 94.0 | 94.0 |
| `Events/ClairVoyancePlayed.cs` | 119 | 74.8 | 93.3 | 94.1 |
| `Events/Shipment.cs` | 340 | 88.2 | 94.1 | 94.1 |
| `Events/Move.cs` | 35 | 94.3 | 94.3 | 94.3 |
| `Game_Bidding.cs` | 319 | 91.2 | 93.4 | 94.4 |
| `Events/TakeLosses.cs` | 54 | 83.3 | 83.3 | 94.4 |
| `Events/Bid.cs` | 166 | 92.2 | 94.6 | 94.6 |
| `Game_Battle.cs` | 680 | 94.0 | 94.1 | 94.7 |
| `Events/JuicePlayed.cs` | 41 | 70.7 | 100.0 | 95.1 |
| `Events/Battle.cs` | 461 | 94.6 | 93.5 | 95.2 |
| `Model/LeaderState.cs` | 22 | 95.5 | 95.5 | 95.5 |
| `Events/ChoamDiscarded.cs` | 24 | 79.2 | 95.8 | 95.8 |
| `Model/BattleOutcome.cs` | 24 | 95.8 | 95.8 | 95.8 |
| `Events/DealRejected.cs` | 29 | 65.5 | 96.6 | 96.6 |
| `Events/FremenRidesMonster.cs` | 91 | 96.7 | 96.7 | 96.7 |
| `Events/DistransUsed.cs` | 32 | 96.9 | 96.9 | 96.9 |
| `Model/Deal.cs` | 40 | 97.5 | 97.5 | 97.5 |
| `Events/Karma.cs` | 133 | 94.7 | 97.7 | 97.7 |
| `Model/TreacheryCard.cs` | 45 | 95.6 | 97.8 | 97.8 |
| `Events/AmbassadorActivated.cs` | 193 | 97.9 | 96.9 | 97.9 |
| `Model/PlayerSequence.cs` | 101 | 92.1 | 96.0 | 98.0 |
| `Game_Other.cs` | 103 | 97.1 | 98.1 | 98.1 |
| `Events/KarmaFreeRevival.cs` | 52 | 86.5 | 96.2 | 98.1 |
| `Events/TerrorRevealed.cs` | 108 | 94.4 | 94.4 | 98.1 |
| `Map.cs` | 1567 | 98.2 | 98.3 | 98.3 |
| `Game_Contemplate.cs` | 211 | 98.6 | 98.6 | 98.6 |
| `Events/Voice.cs` | 74 | 97.3 | 97.3 | 98.6 |
| `Events/EndPhase.cs` | 223 | 97.8 | 97.8 | 99.1 |
| `Game_Collection.cs` | 112 | 95.5 | 97.3 | 99.1 |
| `Game_Events.cs` | 402 | 98.0 | 97.3 | 99.3 |
| `Events/Revival.cs` | 308 | 98.4 | 97.7 | 99.4 |
| `Events/AcceptOrCancelTleilaxuRevival.cs` | 25 | 100.0 | 88.0 | 100.0 |
| `Events/AllianceBroken.cs` | 11 | 81.8 | 100.0 | 100.0 |
| `Events/AllianceByAmbassador.cs` | 19 | 89.5 | 94.7 | 100.0 |
| `Events/BattleClaimed.cs` | 23 | 91.3 | 100.0 | 100.0 |
| `Events/BattleRevision.cs` | 14 | 85.7 | 100.0 | 100.0 |
| `Events/BeneGesseritPrediction.cs` | 17 | 88.2 | 100.0 | 100.0 |
| `Events/ChoamKarmaPrevention.cs` | 18 | 88.9 | 83.3 | 100.0 |
| `Events/CardTraded.cs` | 37 | 24.3 | 100.0 | 100.0 |
| `Events/Diplomacy.cs` | 25 | 100.0 | 84.0 | 100.0 |
| `Events/DivideResources.cs` | 25 | 92.0 | 92.0 | 100.0 |
| `Events/FactionTradeOffered.cs` | 28 | 82.1 | 100.0 | 100.0 |
| `Events/FlightDiscoveryUsed.cs` | 11 | 36.4 | 54.5 | 100.0 |
| `Events/FlightUsed.cs` | 13 | 46.2 | 100.0 | 100.0 |
| `Events/IxianRemovedCardFromAuction.cs` | 27 | 88.9 | 88.9 | 100.0 |
| `Events/HideSecrets.cs` | 8 | 75.0 | 100.0 | 100.0 |
| `Events/KarmaChoamDiscard.cs` | 18 | 77.8 | 100.0 | 100.0 |
| `Events/KarmaHmsMovement.cs` | 28 | 21.4 | 42.9 | 100.0 |
| `Events/KarmaMonster.cs` | 16 | 25.0 | 100.0 | 100.0 |
| `Events/KarmaEcazDial.cs` | 14 | 21.4 | 35.7 | 100.0 |
| `Events/KarmaPrescience.cs` | 11 | 81.8 | 100.0 | 100.0 |
| `Events/KarmaRevivalPrevention.cs` | 14 | 28.6 | 50.0 | 100.0 |
| `Events/KarmaRicheseBuy.cs` | 16 | 37.5 | 100.0 | 100.0 |
| `Events/NexusVoted.cs` | 14 | 92.9 | 92.9 | 100.0 |
| `Events/PerformBeneGesseritPlacement.cs` | 19 | 94.7 | 100.0 | 100.0 |
| `Events/PerformSetup.cs` | 24 | 87.5 | 100.0 | 100.0 |
| `Events/PlacementEvent.cs` | 88 | 88.6 | 94.3 | 100.0 |
| `Events/PlayerReplaced.cs` | 11 | 72.7 | 100.0 | 100.0 |
| `Events/PoisonToothCancelled.cs` | 9 | 77.8 | 100.0 | 100.0 |
| `Events/EmperorBidSupport.cs` | 12 | 83.3 | 100.0 | 100.0 |
| `Events/ResourcesAudited.cs` | 18 | 50.0 | 61.1 | 100.0 |
| `Events/RicheseGaveCard.cs` | 19 | 84.2 | 100.0 | 100.0 |
| `Game_Alliances.cs` | 91 | 98.9 | 100.0 | 100.0 |
| `LeaderManager.cs` | 100 | 99.0 | 100.0 | 100.0 |

89 other engine files are at 100% in all three runs.

The four UI files are at 0% in every run: `DefaultSkin.cs` (1,357 lines), `Skin.cs` (1,219), `GameStatus.cs` (661), `GameStatistics.cs` (57).

## Uncovered lines

Every line the 300 bots + 1,000 fuzz corpus misses, grouped by the member it is in. Line numbers are as of this change. I found callers by searching all four projects for the member name, so "only the client calls it" can occasionally mean `GameStatus` as well.

### (a) Unreachable

| Where | Lines | Why |
|---|--:|---|
| `Game.cs` 544 (`ResourcesIn`) | 1 | Internal, and nothing in Shared calls it. Bots have their own method of that name. |
| `Game.cs` 185,188,190-191,195,197-198,202-203,205 (`RestoreRecentlyUndone`) | 10 | Needs `RecentlyUndoneEvents` without `PreviousGameState`. `Undo` always sets both, `DismissRecentlyUndone` clears both, a new game has neither, and no event touches them. |
| `Model/Homeworld.cs` 62,64-66 (`HomeworldStatus`) | 4 | Never created. |
| `Enumerations.cs` 61-77 (`ToFaction`); `Events/Battle.cs` 796,798,800,802-805,809-812,817-820,822 (`DetermineForces`); `Events/DealAccepted.cs` 181 (`GetDealContents`); `Game.cs` 1045,1047,1049,1053 (`DistanceFromStorm`); `Model/BattleOutcome.cs` 46 (`WinnerHeroKilled`); `Model/Player.cs` 107 (`GetHomeworld`), 493 (`LocationsWithAnyForcesInTerritory`); `Model/Territory.cs` 39 (`IsDiscoveredStronghold`) | 42 | No callers in any project. |
| `ExtensionMethods.cs` 9,11,13,15,17 (`HighestOrDefault`), 22,24-26,28-30,33-35,133,135,137 (`OneOfHighestNOrDefault`), 40,42-44,46-48,51-53 (`OneOfLowestNOrDefault`), 58,60,62,64,66 (`LowestOrDefault`), 70 (`Random`), 75,77,79-84 (`RandomOrDefault`), 152-154 (`TakeRandomN`) | 45 | Overloads that use the unseeded static `Random`. Nothing calls them; Shared and Bots use the overloads that take a `LoggedRandom`. |
| `Model/Log/BlowLog.cs` 43; `Model/Log/ShipMoveLog.cs` 18-30, 38-44; `Model/Message.cs` 156-163 | 29 | Positional parameters of log records. Coverlet counts their generated property getters, and nothing reads these properties back. |
| `Model/Report.cs` 51-52 (`Express`) | 2 | The `Express(GameEvent)` overload has no callers; the engine logs through `Game.Log`. |
| `Game.cs` 473 (`Forces`) | 1 | The only caller (`Map.cs:1759`) passes the default `includeHomeworlds: false`. With `true` this line would throw on duplicate keys, because `Map.Locations(true)` already contains the homeworlds. |
| `Model/Payment.cs` 27,29-30,32,34-36 (`Payment`) | 7 | These two constructors have no callers; `Game.cs:828` uses the four-argument one. |
| `Model/Homeworld.cs` 56 (`IsHigh`), 58 (`IsLow`), 60 (`Occupant`) | 3 | `HomeworldStatus` record is never created. |
| `Map.cs` 1696-1701 (`Equals`) | 6 | `NeighbourCacheKey.Equals(object)`. The cache dictionary uses `IEquatable<NeighbourCacheKey>.Equals`. |
| `Game.cs` 111 (`Game`) | 1 | `throw` for a version below 100, outside the supported range. |
| `Events/ChoamDiscarded.cs` 63 (`ExecuteConcreteEvent`) | 1 | `throw` when `Card` is null. `Validate` requires the card to be in the player's hand, and undo only re-executes events that were accepted. |
| `Game_Battle.cs` 877 (`TraitorCalled`) | 1 | `throw` when a leader is null after both traitor calls succeeded. A traitor call only succeeds against a leader. |

### (b) Reachable only from old versions

None. Fuzz games pick a version on each side of every `Version` check in Shared (the list is in `GameConfig.VersionGates`). The uncovered lines that mention a version are in dead code (`Battle.DetermineForces`), are the constructor's check for versions below 100 (`Game.cs:111`), or are a rare branch that other seeds reach (`Retreat.cs:52`, `Version >= 164`).

### (d) Outside the event pipeline

| Where | Lines | Why |
|---|--:|---|
| `Events/GameEvent.cs` 259 (`GetMessage`) | 1 | Base implementation. Every concrete event overrides it except `EstablishPlayers`, whose own `Execute` never calls it. |
| `Model/ObjectCounter.cs` 27,29-32 (`CountN`), 36,38-40 (`SetToN`), 44,46,48 (`CountOf`), 52 (`Counted`), 57,59,61 (`GetHighest`) | 16 | Bots-only helper class; the classic bot creates one but these members were not reached in any bot game. |
| `Model/Homeworld.cs` 49 (`ToString`) | 1 | Branch for a non-null `Message.DefaultDescriber`, which only the client sets. The oracle requires it to be null. |
| `GameChatMessage.cs` 14 (`TargetUserId`), 18,20,22,24-26,29,31,33-35,38 (`GetBodyIncludingPlayerInfo`), 42 (`GetPlayerName`), 44 (`GetFaction`) | 15 | Chat DTO (server/client). |
| `Game.cs` 263 (`Started`), 274,276 (`LatestEvent`) | 3 | Client and server only. |
| `Game.cs` 254 (`Duration`) | 1 | Client and statistics only. |
| `Model/Messiah.cs` 48-50 (`ToString`) | 3 | Client describer branch, and the plain `ToString` that only reaches the trace through an exception message. |
| `Model/Leader.cs` 81 (`ToString`); `Model/Location.cs` 59 (`ToString`); `Model/ResourceCard.cs` 33-34,36 (`ToString`); `Model/Territory.cs` 74 (`ToString`); `Model/TreacheryCard.cs` 101 (`ToString`) | 7 | Client describer branch. |
| `Game.cs` 258-260 (`TimeSpent`), 278 (`EventCount`); `Model/Fight.cs` 18 (`Id`); `Model/Location.cs` 44 (`Advantage`); `Model/Player.cs` 599 (`NumberOfTraitors`), 601 (`NumberOfFaceDancers`); `Model/Timer.cs` 20-22 (`TimeSpent`); `Model/Utilities.cs` 107-108,110,112,114,116,120,125,129 (`ScaleFont`), 133 (`Px`), 136 (`Round`) | 22 | Client only. |
| `Model/Player.cs` 534-536 (`HasSpecialForces`) | 3 | Client, and `AutomationConfigured.GetDescription` (client). |
| `Model/Utilities.cs` 30-32 (`Deserialize`) | 3 | Conversion of old Newtonsoft-format savegames. `replay` mode reaches it given such a savegame; none is in the repo. |
| `Game.cs` 759,761-762,764,766-767,769,771,774,776,778-779,781 (`TryLoad`) | 13 | Loads a savegame (server and client). |
| `Events/FactionSelected.cs` 16,18 (`FactionSelected`) | 2 | One-argument constructor used by the client. |
| `Events/GameEvent.cs` 159,161-162 (`GameEvent`) | 3 | One-argument constructor; only `FactionSelected(Game)` uses it. |
| `Enumerations.cs` 43-59 (`ToNexus`) | 17 | Only `GameStatus` and the client call it. |
| `Game_Rules.cs` 355-363 (`GetRulesetExpansion`), 372,375-376,384-389,391-392 (`GetRuleExpansion`) | 20 | Only called by the client helpers above. |
| `Game_Bidding.cs` 636 (`GetCardSetAsideForBid`) | 1 | Only called from `CardsOwnedBy`. |
| `Map.cs` 1834,1836-1837,1839-1843,1845,1847,1850-1851,1854-1856 (`FindPath`) | 15 | Only called from `FindFirstShortestPath`. |
| `Game_LeaderState.cs` 112 (`GetSkilledLeaders`) | 1 | Only called from `GetSkill`. |
| `Events/GameEvent.cs` 308 (`Clone`); `Game.cs` 807,809,811-817,819 (`Clone`) | 11 | Only the client (story view) calls it. |
| `Events/AutomationConfigured.cs` 113,116,119-120,123,126-129,132-133,135,138,141 (`GetDescription`); `Events/BattleConcluded.cs` 325-326 (`MayCaptureOrKill`); `Events/BattleInitiated.cs` 166-167,169-171 (`PreviousPlanOf`); `Events/Bid.cs` 111-117 (`PlayersToBid`); `Events/BlackMarketBid.cs` 242-248 (`PlayersToBid`); `Events/ClairVoyancePlayed.cs` 109 (`GetQuestion`); `Events/DealAccepted.cs` 88,90 (`CancellableDeals`), 95 (`CurrentDeals`); `Events/DealOffered.cs` 92-102 (`Rejection`), 161 (`GetDealDescription`); `Events/Donated.cs` 78-80 (`MinAmount`); `Events/EstablishPlayers.cs` 81 (`GetMaximumNumberOfPlayers`), 86 (`GetMaximumNumberOfTurns`), 126,130-132,137-138,143-144,149-150,153,155 (`AvailableRuleGroups`); `Events/GameEvent.cs` 298 (`ByAllyOf`); `Events/Karma.cs` 241 (`CanKarma`); `Events/LoserConcluded.cs` 115-116,118-119,121-123,125 (`CardsThatMayBeKeptOrDiscarded`); `Events/Shipment.cs` 314 (`DefaultNoFieldSpecialForceAmount`), 409 (`CanKarma`); `Game.cs` 1017-1018 (`AssistedNoteKeepingEnabled`), 1032,1034,1036,1040 (`NextStormWillPassOver`); `Game_Bidding.cs` 603 (`NumberOfCardsOnRegularAuction`), 624-625,627,630-632 (`CardsOwnedBy`); `Game_Deals.cs` 43-45 (`DealRecipients`); `Game_LeaderState.cs` 117-118 (`GetSkill`); `Game_Move.cs` 715,717-718,720,722,724-725,727-728,731 (`ThreatenedByAllyPresence`); `Game_Other.cs` 209,214 (`YieldsSecrets`); `Game_Rules.cs` 313 (`GetAvailableRulesets`); `Map.cs` 1822-1824,1826-1827,1829 (`FindFirstShortestPath`); `Model/Battalion.cs` 53-55 (`Clear`); `Model/Expression.cs` 18 (`Express`) | 121 | Only the client calls it. |
| `Events/BattleConcluded.cs` 312,314,316,318,320 (`ValidMaxReplacementForceAmount`) | 5 | Only the client calls it; the engine checks the amount differently. |
| `Game.cs` 50 (`Reports`); `Game_LeaderState.cs` 146 (`MessiahIsAlive`) | 2 | Only the client reads it. |
| `Model/Battalion.cs` 89 (`Clone`); `Model/LeaderState.cs` 54 (`Clone`); `Model/Player.cs` 722,724-730,732 (`Clone`); `Model/Territory.cs` 81 (`Clone`); `Model/Utilities.cs` 9 (`CloneList`), 14 (`CloneObjectDictionary`), 19 (`CloneEnumDictionary`) | 15 | Part of `Game.Clone` (client). |
| `Model/Message.cs` 88,94-96,134-136,142 (`ToString`) | 8 | Rendering with a describer (client and server). |
| `Model/Utilities.cs` 37-38,40-41 (`ConvertNewtonsoftJsonToPlainJson`), 46,48,50-51,53,57,61,63-64,66-68,70,74,76-77,79,84,86,88,90,94 (`ReplaceTypeMetadataAndNestedArrays`), 98-99 (`DetermineGameEvent`) | 28 | Same savegame conversion. |
| `Model/GameState.cs` 16 (`Version`), 18 (`_events`), 23-24 (`Events`), 29-30 (`Load`), 35-36 (`GetStateAsString`), 41 (`FixGameStateString`) | 9 | Savegame container, used by the server and client to save and load games. |
| `Game_Participation.cs` 24 (`GetPlayerInSeat`), 26 (`GetFactionInSeat`), 32 (`GetUserIdOfPlayer`), 34 (`GetPlayerName`), 40 (`GetPlayerByUserId`), 42 (`GetPlayerByName`), 44 (`GetUserIdByName`), 54 (`UserIdInSeat`), 57 (`SeatOf`), 60-61 (`IsOpen`), 65 (`SeatIsAvailable`), 67 (`NumberOfHosts`), 69 (`NumberOfBots`), 71 (`NumberOfObservers`), 73 (`NumberOfSeatedPlayers`), 76-78 (`PlayerNames`), 81-83 (`ObserverNames`), 85 (`IsPlayer`), 87 (`IsHost`), 89 (`IsObserver`), 91 (`IsParticipant`), 93 (`WasKicked`), 97,99,101-102,105-109 (`AddPlayer`), 113-116 (`AddObserver`), 120,122-125,127,129,131 (`RemoveUser`), 135,137,141,143 (`OpenOrCloseSeat`), 147,149,153,155 (`SetOrUnsetHost`), 159-161 (`ResetSeats`) | 59 | Seating, hosts, observers and kicks. Called by the server between events, never by an event. |
| `Game.cs` 1118 (`LastAction`) | 1 | Server and statistics only. |
| `Events/GameInfo.cs` 16 (`CreatorId`), 17 (`CreatorUsername`), 18 (`CreationDate`), 19 (`GameId`), 20 (`HasPassword`), 21 (`Name`), 22 (`MaxPlayers`), 23 (`MaxTurns`), 24 (`MainPhase`), 25 (`Phase`), 26 (`Turn`), 27 (`NrOfBots`), 28 (`NrOfPlayers`), 29 (`FactionsInPlay`), 30 (`Ruleset`), 31 (`LastActivity`), 32 (`SeatedPlayers`), 33 (`AvailableSeats`), 36 (`CanBeJoined`), 38 (`YourCurrentSeat`), 40 (`YouAreIn`), 42 (`Equals`), 44 (`GetHashCode`) | 23 | Server lobby DTO. |
| `Model/Deck.cs` 76-78 (`Randomize`) | 3 | Server only. |
| `Model/AdminInfo.cs` 14 (`Users`), 16 (`UsersByUserTokenCount`), 18 (`ConnectionInfoByUserIdCount`), 20 (`GamesByGameIdCount`); `Model/AvailableSeatInfo.cs` 14 (`Seat`), 15 (`Faction`), 16 (`IsBot`); `Model/ChatMessage.cs` 15 (`SourceUserId`), 17 (`Body`); `Model/GameInitInfo.cs` 5 (`GameId`), 6 (`GameState`), 7 (`GameName`), 8 (`Participation`); `Model/GlobalChatMessage.cs` 16-17,19-21,24-26 (`GetBodyIncludingPlayerInfo`); `Model/LoggedInUserInfo.cs` 16 (`Id`), 18 (`Name`), 20 (`Status`), 22 (`LastSeen`); `Model/LoginInfo.cs` 5 (`UserId`), 7 (`Token`), 9 (`PlayerName`), 11 (`UserName`), 13 (`Email`); `Model/ManagedGame.cs` 9 (`_eventSemaphore`), 11 (`CreationDate`), 13 (`LastActivity`), 15 (`LastPersisted`), 17 (`CreatorUserId`), 19 (`GameId`), 21 (`Game`), 23 (`Name`), 25 (`HashedPassword`), 27 (`ObserversRequirePassword`), 29 (`StatisticsSent`), 31 (`LastAsyncPlayMessageSent`), 33 (`Bots`), 37,40,44,46 (`ProcessEventAsync`); `Model/ObserverJoined.cs` 14 (`HashedPassword`), 16 (`Name`), 20-21,23 (`ValidName`); `Model/ObserverRejoined.cs` 14 (`HashedPassword`), 16 (`Name`); `Model/PlayerJoined.cs` 16-17,19 (`ValidName`); `Model/PlayerRejoined.cs` 14 (`HashedPassword`), 16 (`Name`); `Model/Point.cs` 19-21 (`PointD`), 25,30 (`Equals`), 35 (`GetHashCode`), 40 (`operator`), 45 (`public static bool operator !=(PointD lh`), 50 (`Translate`); `Model/Result.cs` 14 (`Success`), 15 (`Error`), 16 (`ErrorDetails`), 17 (`Contents`); `Model/ScheduledGame.cs` 5 (`ScheduledGameId`), 7 (`DateTime`), 9 (`CreatorUserId`), 11 (`CreatorPlayerName`), 13 (`NumberOfPlayers`), 15 (`MaximumTurns`), 17 (`Ruleset`), 19 (`AsyncPlay`), 21 (`AllowedFactionsInPlay`), 23 (`SubscribedUsers`); `Model/ScheduledGameInfo.cs` 8 (`ScheduledGameId`), 10 (`DateTime`), 12 (`CreatorUserId`), 14 (`CreatorName`), 16 (`NumberOfPlayers`), 18 (`MaximumTurns`), 20 (`Ruleset`), 22 (`AsyncPlay`), 24 (`AllowedFactionsInPlay`), 26 (`Subscribers`), 28 (`SubscriberNames`); `Model/ServerInfo.cs` 14 (`ScheduledMaintenance`), 16 (`AdminName`); `Model/ServerStatus.cs` 14 (`RunningGames`), 16 (`OwnGames`), 18 (`ScheduledGames`), 20 (`RecentlySeenUsers`); `Model/UserConnections.cs` 7 (`ConnectionInfoByGameId`), 11-12 (`SetConnectionId`), 16-19 (`GetGameIdsWithOldConnections`), 24,26,28-29,32 (`TryGetConnectionId`), 37-38 (`RemoveConnectionId`), 43 (`Created`), 45 (`ConnectionId`); `Model/UserInfo.cs` 16 (`Id`), 18 (`Username`), 20 (`Email`), 22 (`LastLogin`), 24 (`PlayerName`) | 120 | Server/client DTO; nothing in the engine creates or reads it. |
| `Model/Leader.cs` 56 (`SkinId`); `Model/Territory.cs` 31 (`SkinId`) | 2 | Skin (client) only. |
| `Game.cs` 1116 (`FirstAction`) | 1 | Statistics only. |
| `Events/ClairVoyancePlayed.cs` 247 (`GetShortMessage`); `Events/DealAccepted.cs` 171 (`GetShortMessage`); `Events/DealOffered.cs` 190-192 (`GetShortMessage`); `Events/DealRejected.cs` 85 (`GetShortMessage`); `Events/GameEvent.cs` 264 (`GetShortMessage`) | 7 | Text for the client and the server's e-mails. |
| `Model/Deal.cs` 48 (`DealContentsDescription`) | 1 | Text for the client. |
| `Events/GameEvent.cs` 303 (`By`) | 1 | The `By(Player)` overload; only the client calls it. |
| `Game.cs` 932 (`OwnerOf`) | 1 | This overload is only called by `GameStatus`. |
| `LoggedRandom.cs` 24,26-27 (`LoggedRandom`) | 3 | Unseeded constructor, used by `Deck.Randomize` (server). |
| `Events/EstablishPlayers.cs` 47 (`Players`) | 1 | `init` accessor used when the server builds the event; JSON goes through `_players`. |

### (c) Reachable, not reached

These need smarter steering, or more games: states that depend on a particular Nexus card, Karama power, leader skill, stronghold bonus or intrusion sequence. A next step would be scenario steering, for example giving a player a specific Nexus or Karama card through the custom deck rules, then steering towards the phase where it applies.

| Where | Lines | Why |
|---|--:|---|
| `Events/Battle.cs` 211-214 (`ActivateDynamicWeapons`), 352 (`DetermineBattleOutcome`); `Events/BattleConcluded.cs` 228-229,231-232 (`ProcessIxianForceLossesAndSubstitutions`); `Game_Battle.cs` 103 (`HandleRevealedBattlePlans`), 152 (`DiscardOneTimeCardsUsedInBattle`), 227-228 (`HandleReinforcements`), 243 (`ActivateSmuggler`), 290 (`ActivateSandmasterIfApplicable`), 304,311-312,314-316 (`ActivateSmugglerIfApplicable`), 356 (`ResolveEffectOfOwnedTueksSietch`), 377 (`ResolveEffectOfOwnedSietchTabr`), 393,398-400 (`ResolveEffectOfOccupiedJacurutu`), 419 (`PrepareAudit`), 535-536 (`HandleBattleOutcome`), 591,595 (`HandleLosses`), 929-930 (`OneTraitorCalled`), 953-954,964-965 (`TwoTraitorsCalled`), 1141 (`ReturnSkilledLeadersInFrontOfShieldAfterBattle`), 1253 (`WinnerBattleAction`) | 40 | Battle detail: a leader skill, stronghold bonus, traitor or Lasgun/Shield outcome with particular forces and cards. |
| `Game_Battle.cs` 1001-1002 (`LasgunShieldExplosion`) | 2 | Battle detail: a leader skill, stronghold bonus, traitor or Lasgun/Shield outcome with particular forces and cards. Reached with other seeds. |
| `Events/BlackMarketBid.cs` 35 (`UsesEmperorSecretAlly`), 41 (`UsingKarmaToRemoveBidLimit`), 44 (`KarmaCard`), 47 (`KarmaBid`); `Events/CardGiven.cs` 30 (`Validate`), 44,46,48-49,52-53 (`ExecuteConcreteEvent`); `Events/Retreat.cs` 87-88,90-93,96-97,99-102,105-109 (`ExecuteConcreteEvent`); `Events/RicheseSpecifiesAuction.cs` 96-97,102-103,105-106,108,111,113-114,116 (`ExecuteConcreteEvent`); `Model/PlayerSequence.cs` 155 (`HasPassedRichese`), 240 (`IsAfter`) | 41 | Event or auction that needs an uncommon rule and faction combination, then valid choices in a short window. |
| `Events/AuditCancelled.cs` 44 (`Cost`), 65 (`GetCardsThatMayBeAudited`), 75,81-83,93,95 (`ExecuteConcreteEvent`); `Events/BlackMarketBid.cs` 100-104 (`HandleBlackMarketNormalBid`); `Events/ResourcesTransferred.cs` 38-39,41 (`Validate`), 65-69 (`ExecuteConcreteEvent`); `Events/Retreat.cs` 49-53 (`Validate`) | 26 | Event or auction that needs an uncommon rule and faction combination, then valid choices in a short window. Reached with other seeds. |
| `Game_Move.cs` 313,330-331 (`SkipMoveIfNothingToMove`), 443-444,446 (`DetermineNextShipmentAndMoveSubPhaseOnNoIntrusion`), 471-472,509-510,515-516 (`DetermineNextShipmentAndMoveSubPhaseOnBeneGesseritIntruded`), 551-552,571-572,590-591 (`DetermineNextShipmentAndMoveSubPhaseOnTerrorTriggered`), 606-607,659-660,665-666 (`DetermineNextShipmentAndMoveSubPhaseOnAmbassadorTriggered`), 754-755,759-760 (`DetermineNextSubPhaseAfterGuildShipAndMove`) | 28 | Intrusion and sub-phase sequencing: several intrusions (Bene Gesserit, terror, ambassador) from one placement, or an unusual sub-phase order. |
| `Game_Move.cs` 363,365 (`CleanupObsoleteIntrusions`) | 2 | Intrusion and sub-phase sequencing: several intrusions (Bene Gesserit, terror, ambassador) from one placement, or an unusual sub-phase order. Reached with other seeds. |
| `Events/BattleConcluded.cs` 179 (`DecideFateOfCapturedLeader`); `Events/BlackMarketBid.cs` 171,173,175-177,183-185 (`FinishBlackMarketBid`); `Events/Captured.cs` 44 (`ExecuteConcreteEvent`); `Events/Karma.cs` 265,269 (`ExecuteConcreteEvent`); `Events/KarmaFreeRevival.cs` 81 (`ExecuteConcreteEvent`); `Events/LoserConcluded.cs` 157-159,161-162,164-165,167,169-170,172-173 (`ExecuteConcreteEvent`); `Game_Battle.cs` 207-208 (`CaptureLeaderIfApplicable`); `Game_Bidding.cs` 435-437 (`FinishBid`); `Game_Blow.cs` 295-297 (`LetMonsterAppear`), 334,338 (`PerformMonster`); `Game_Storm.cs` 59-61 (`MoveHmsBeforeDiallingStorm`) | 38 | Needs a faction advantage prevented by Karama, or a Karama special power, at the moment the advantage applies. |
| `Game_Storm.cs` 160-164 (`PerformStorm`) | 5 | Needs a faction advantage prevented by Karama, or a Karama special power, at the moment the advantage applies. Reached with other seeds. |
| `Events/Bid.cs` 271-272 (`BidWonByOnlyPlayer`); `Events/ChoamExtraMove.cs` 43-45 (`ExecuteConcreteEvent`); `Events/ChoamFreeRevivalPrevention.cs` 59-61 (`ExecuteConcreteEvent`); `Events/ChoamMovePrevention.cs` 73-75 (`ExecuteConcreteEvent`); `Events/ChoamRemoveForce.cs` 108-109,119 (`ExecuteConcreteEvent`); `Events/Move.cs` 65-66 (`ExecuteConcreteEvent`); `Events/NexusPlayed.cs` 136,138 (`Validate`), 296-297 (`MaySelectLocationForRevivedForces`), 363,378,397,416,427-428,449,453,457 (`HandleBetrayal`), 520-522,525-526,528-529,531,533-534,536-537,539,541,544,546,548,550-551,554 (`HandleCunning`), 582,617-618,621,656,659 (`HandleSecretAlly`); `Events/RaiseDeadPlayed.cs` 104-105 (`MaySelectLocationForRevivedForces`); `Events/Shipment.cs` 477-480,501,504-505 (`ExecuteConcreteEvent`); `Events/TakeLosses.cs` 108-109 (`ExecuteConcreteEvent`) | 66 | Needs a particular Nexus card played (cunning, betrayal or secret ally) in a particular phase or battle state. |
| `Events/NexusPlayed.cs` 255 (`ValidTleilaxuMaxAmount`) | 1 | Needs a particular Nexus card played (cunning, betrayal or secret ally) in a particular phase or battle state. Reached with other seeds. |
| `Events/ChoamRemoveForce.cs` 74-75 (`ValidSpecialForceChoices`) | 2 | Only the client calls it. The generator draws bools at random instead of from this helper. |
| `Events/AmbassadorActivated.cs` 395 (`HandleAmbassador`); `Events/Battle.cs` 434 (`Validate`); `Events/ClairVoyanceAnswered.cs` 49,54,56 (`IsQuestionedBy`), 102 (`ExecuteConcreteEvent`); `Events/ClairVoyancePlayed.cs` 157 (`Express`), 175,180,182,185 (`IsInScopeOf`); `Events/DistransUsed.cs` 98 (`CheckIfBiddingForPlayerShouldBeSkipped`); `Events/EndPhase.cs` 82-83 (`ExecuteConcreteEvent`); `Events/Shipment.cs` 605,607-609 (`PerformSiteToSiteShipment`), 617,619-620,630,632,634 (`PerformNormalShipment`); `Events/TakeLosses.cs` 79 (`ValidUselessCardToPreventLosses`); `Events/Voice.cs` 146 (`IsVoicedBy`); `Events/FremenRidesMonster.cs` 82,85 (`LocationsWithForcesThatCanRide`), 124 (`IsEitherValidDiscoveryOrNoDiscovery`); `Game.cs` 287 (`FindMostRecentEvent`), 659,664,668-669 (`CheckIfOccupierTakesVidal`), 711 (`Discard`); `Game_Bidding.cs` 323-324 (`PayForCard`), 342,344-345,348,350 (`GivePlayerExtraCardIfApplicable`); `Game_Blow.cs` 157-158 (`ReshuffleResourceDeck`), 200,212 (`ProcessBlowCard`), 477,482-487 (`Natural`), 509 (`ProtectedFromMonster`); `Game_Collection.cs` 38 (`DivideResourcesFromCollection`); `Game_Contemplate.cs` 261-262,264 (`LogNormalWin`); `Game_Events.cs` 109,412-413 (`AddPlayerActions`); `Model/Messiah.cs` 18 (`ValueInCombatAgainst`), 27 (`Is`), 38 (`IsTraitor`), 43 (`IsFaceDancer`); `Model/Player.cs` 386-387 (`ShipForces`), 403,405,409,411 (`ShipSpecialForces`), 711 (`GetHomeworldBattleContributionAndLasgunShieldLimit`) | 72 | Rare game state for this branch. |
| `Events/BattleInitiated.cs` 69 (`Aggressor`); `Events/FaceDanced.cs` 78 (`MaximumNumberOfForces`), 115-116 (`ExecuteConcreteEvent`), 157-159 (`ReplaceForces`); `Events/JuicePlayed.cs` 97-98 (`ExecuteConcreteEvent`); `Events/Shipment.cs` 250 (`Validate`); `Events/TerrorPlanted.cs` 118-120 (`ExecuteConcreteEvent`), 151 (`GetVerboseMessage`); `Model/Messiah.cs` 23 (`HeroType`); `Model/Player.cs` 198-199 (`RemoveForces`), 203-204 (`RemoveSpecialForces`), 261 (`SpecialForcesToReserves`) | 20 | Rare game state for this branch. Reached with other seeds. |
| `Events/AmbassadorActivated.cs` 97 (`SpecialForcesAddedToLocation`), 100 (`ForceLocations`); `Events/NexusPlayed.cs` 88 (`To`), 91 (`TotalAmountOfForcesAddedToLocation`), 94 (`ForcesAddedToLocation`), 97 (`SpecialForcesAddedToLocation`); `Events/RaiseDeadPlayed.cs` 57 (`To`), 60 (`TotalAmountOfForcesAddedToLocation`), 63 (`ForcesAddedToLocation`), 66 (`SpecialForcesAddedToLocation`); `Events/Revival.cs` 69 (`ForcesAddedToLocation`), 72 (`SpecialForcesAddedToLocation`); `Events/TerrorRevealed.cs` 63 (`ForcesAddedToLocation`), 66 (`SpecialForcesAddedToLocation`) | 14 | Read only by the intrusion checks (`Game_Move.cs:846`, `:858`), when this event places forces where a Ecaz ambassador or Moritani terror token is. |
| `Events/AmbassadorActivated.cs` 94 (`ForcesAddedToLocation`) | 1 | Read only by the intrusion checks (`Game_Move.cs:846`, `:858`), when this event places forces where a Ecaz ambassador or Moritani terror token is. Reached with other seeds. |
