# Review: package C (setup)

This review compares the package C files of [port-assignments.md](../port-assignments.md) (`Game_Setup.cs` and the 18 setup events) with their Rust modules, member by member, looking for behaviour that differs from C#.

## Result

I found no confirmed difference in package C. Every member was read side by side with the C#, and every suspicion below was checked against real C# output, either by a crafted trace or by a probe. All of them matched.

The C# code in this package is simple, and the port follows it closely. Statement order, throw points, the `Enter` overloads (including the static ones, inlined in Rust), the version gates (`< 113`, `<= 125`, `< 132`, `< 134`, `>= 144`, `<= 154`, `< 170`, `< 172`, `< 174`, `>= 121`), the random draw order, and the shared `Settings`/`FactionsInPlay` lists all match.

## How it was checked

### Crafted traces

The oracle's `replay` mode records a trace from any savegame, with a full state dump on every line. That let me drive paths the fuzzer and bots never reach. Each savegame below was recorded and then checked with `differ first`. Every one replayed with no difference.

| Savegame | What it exercises |
|---|---|
| v150, `_players` = `Alice>Bob>Carol`, `PlayersChooseFactions` | The legacy `Players` path of `AddPlayersToGame` (`LegacyNames`, "joins the game" logs), and `FactionSelected` by `InitiatorPlayerName` (a match, a non-matching name, and a second player) |
| v110, rules `GuildBot`, `EmperorBot`, `FillWithBots` | `AddBots` (version < 113) and the bot-faction `FillSeats` branch (version <= 125) |
| v150, `MaximumNumberOfPlayers` = 0 | No players: `EnterSetupPhase` throws "Sequence contains no elements" at `Players.OrderBy(...).First()` |
| v150, `_factionsInPlay` with `" Atreides "` and `12345` | `Enum.Parse` trimming and undefined numeric values, then "Cannot draw from an empty deck" in `DeterminePositionsAtTable` |
| v150, `_factionsInPlay` = `Atreides,atreides` | `ArgumentException` escaping `EstablishPlayers.Execute` (no try/catch) |
| v150, `ApplicableRules` = null | `ArgumentNullException` at `Settings.InitialRules = ApplicableRules.ToList()`, after the earlier settings writes |
| v150, `_players` = null | `NullReferenceException` in the `Players` getter, after decks, rules and alliance flags were already set |
| v150, three names but two seats | "Cannot draw from an empty deck" in `DeterminePositionsAtTable` |
| v120 and v140, `CustomDecks` | `CustomizingDecks` before (v120) and after (v140) faction trading, `CardsDetermined` with unknown, negative and malformed ids, the "Not enough cards selected" check, `RicheseCache` |
| v187, `LeaderSkills` | `SkillAssigned` with an unknown leader (`ArgumentNullException` 'key'), `-1`, a leader of a faction not in play, `Passed` in the initial phase, and an undefined `LeaderSkill` |
| v131, faction trades | `FactionTradeOffered` before v132 (the `IsPlaying` check), a matched swap, repeated self-offers, and `RemoveInvalidTradeOffers` |

The savegames were generated in `/tmp` and deleted afterwards. To repeat one, write a `{"Version":N,"_events":[...]}` file and run:

```sh
dotnet ../treachery-online/Oracle/bin/Release/net10.0/Treachery.Oracle.dll replay --in SAVE.json --out DIR --dump every
cargo run -p differ --release -- first DIR/replay-SAVE.jsonl
```

### Probe of members the engine never calls

[`engine/tests/review_pkg_c.rs`](../../engine/tests/review_pkg_c.rs) checks the static helpers and obsolete getters that no trace reaches, because only bots and the UI call them. The expected values come from the `review-c` mode of the reference program ([`ReviewC.cs`](../../engine/csharp-ref/ReviewC.cs)), saved as `tests/golden/review_c.txt`:

```sh
export DOTNET_ROOT="$HOME/dotnet10" PATH="$HOME/dotnet10:$PATH"
cd rust/engine/csharp-ref && dotnet run -c Release -- review-c > ../tests/golden/review_c.txt
cd .. && cargo test --test review_pkg_c
```

It covers, per player of a four-player game: `FactionTradeOffered.ValidTargets`, `AutomationConfigured.GetAvailableRuleTypes` and `GetValidBiddingFactions`, `PerformHmsPlacement.ValidLocations` and `SkillAssigned.ValidLeaders`. It also covers `FactionSelected.ValidFactions`, `PerformMoritaniSetup.ValidLocations`, `PerformBeneGesseritPlacement.ValidLocations`, `PlayerReplaced.ValidTargets`, `EstablishPlayers.AvailableRuleGroups` and `GetMaximumNumberOfPlayers`, `NextFactionToPerformCustomSetup`, and `AutomationConfigured.GetDescription` for every rule type, for a player who is in the game and one who isn't (the latter throws `NullReferenceException` for `RevivalAutoClaimFreeRevival`). Finally it covers the obsolete `_factionsInPlay` parser on 12 odd inputs, the `Players` getter, and `GameInfo.CanBeJoined`/`YourCurrentSeat`.

All 64 lines match. The test passes.

### Differential fuzzing

FUZZ_RESULT

## Suspected differences

None that affect the engine. Two notes:

1. **`GameInfo.AvailableSeats` has a placeholder element type.** In [`game_info.rs`](../../engine/src/events/game_info.rs) the field is `Vec<()>`, because `AvailableSeatInfo` (`Model/AvailableSeatInfo.cs`) was never ported. `CanBeJoined` only reads the length, so it matches. `GameInfo` is server data, never part of game state, so no trace can see this. It needs the real type before the server port (phase 3).
2. **`GameInfo.GetHashCode` differs on purpose.** .NET randomizes string hashes per process, so no port can match it. The Rust comment says so.

## Would the traces catch this class of bug?

Most of package C runs at the start of every game, so the bot and fuzz traces cover it densely. They see every setup phase the factions and rules allow, and the fuzzer sends odd values to every event field. But three areas are invisible to them:

1. **The legacy player list.** The oracle never sets `EstablishPlayers.Players` (`Oracle/GameConfig.cs`, `CreateEstablishPlayers`), so before version 170 every game starts with nameless bots. `AddPlayersToGame`'s name loop, `LegacyNames`, and `FactionSelected` matching by name are never exercised with a real name. Real savegames from before version 170 depend on exactly this path.

   Fix: in `GameConfig.Randomize`, when the version is below 170, set `Players` to a few random names (sometimes duplicated, sometimes more names than seats) and let the fuzzer's `FactionSelected` pick `InitiatorPlayerName` from those names.

2. **`Execute(performValidation: false)`.** The oracle always validates (`GameRunner.cs:347`). The engine skips validation when it replays history in `Undo` (`Game.cs:162-216`), and the client does too. With validation skipped, `EstablishPlayers` takes its second branch, and events that rely on `Validate` to reject bad ids (for example `PerformBeneGesseritPlacement` with a null `Target`) reach code paths that are otherwise unreachable.

   Fix: give the fuzzer a small chance of sending an event with `validate: false`. The trace format already records the flag.

3. **Members only bots and the UI call.** The static `Valid*` helpers, `GetDescription`, the obsolete getters and the setters are not called by the engine, so they show up in no state dump. A port mistake in them would surface only once Rust bots exist (phase 2).

   Fix: list the event types in `tests/coverage/ported_types.txt`, so that `boundary.rs` calls every public member with boundary inputs against C#. `review_pkg_c.rs` does this by hand for package C. The boundary test would do it for every event, and would keep failing for any new member until it gets an entry.
