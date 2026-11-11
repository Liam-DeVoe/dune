# Review: package A (core game and events)

This review compares the package A files of [port-assignments.md](../port-assignments.md) with their Rust modules, member by member, looking for behaviour that differs from C#.

Tests are in [`engine/tests/review_pkg_a.rs`](../../engine/tests/review_pkg_a.rs), which includes [`review_pkg_a/part2.rs`](../../engine/tests/review_pkg_a/part2.rs). The failing ones fail until the difference is fixed. The C# values come from two modes of the reference program, [`ReviewA.cs`](../../engine/csharp-ref/ReviewA.cs) and [`ReviewA2.cs`](../../engine/csharp-ref/ReviewA2.cs):

```sh
export DOTNET_ROOT="$HOME/dotnet10" PATH="$HOME/dotnet10:$PATH"
cd rust/engine/csharp-ref && dotnet run -c Release -- review-a && dotnet run -c Release -- review-a-part2
cd .. && cargo test --test review_pkg_a
```

Result: 5 tests fail (the four confirmed differences below) and 2 pass (guards).

When this review was written, the engine didn't compile because other packages were in the middle of edits. The tests were run on a copy of the tree with three small compile fixes outside package A.

## Summary

The member-by-member port is faithful. In `Game.cs`, `Game_Participation.cs` and `GameEvent.cs`, `Execute`, the pre- and post-event tasks, the `Enter` overloads, `MainPhaseStart`/`Middle`/`End`, `Discard`, `TryLoad`, the timers and the force queries all match C# in statement order, exception points and version gates. `EndPhase`, `PlacementEvent` and the small model types match too, apart from one JSON difference (item 3).

Most of the differences come from object identity, which the Rust data model can't represent:

1. C# events remember which `Game` and which `Player` object they were last initialized with. After `Undo`, those can belong to another game. Rust's `GameRef` is a unit marker, and `PlayerIndex` has no game, so Rust always means "this game".
2. C# shares one `Participation` object between the caller, the game, and every game made from it by `Undo`. Rust copies it.

The first difference already breaks the differential traces: every fuzz trace with an undo fails at its first undo line.

## Confirmed differences

### 1. Events from before an `Undo` still belong to the old game

**Fixed.** Games now have an identity, events and players carry it, and the restored game keeps the other games alive ([engine-conventions.md](../engine-conventions.md#game-identity-and-undo)). The sites below are tested against C# in [`undo_identity.rs`](../../engine/tests/undo_identity.rs).

`Game.Undo` (`Game.cs:150-173`) builds a new game and re-initializes only the events it replays (`History[0..untilEventNr)`). The events it keeps in `RecentlyUndoneEvents` still have `Game` and `Player` pointing at the old game. The probe confirms this: `undo.recentlyUndone[0].game_is_old = True`.

The state dump writes these as `{"$ref":"OtherGame"}`, and writes the old game's `Player` out in full instead of as an index ([trace-format.md 7.2](../trace-format.md#72-things-written-as-references)). Rust writes `{"$ref":"Game"}` and `{"$ref":"Player","index":i}`, because `GameRef` ([`game.rs`](../../engine/src/game.rs), end of file) and `PlayerIndex` have no notion of which game they belong to. `canon.rs` (`impl Canon for GameRef`) has a comment saying this needs a game identity "once undo is ported". Undo is now ported, but that identity was never added.

`RestoreRecentlyUndone` returns the old game object itself. By then `Undo` has re-initialized that game's history events to point at the undo game (probe: `restore.history[0].game_is_undo = True`). So the restored game holds events that belong to another game. In the sample trace `fuzz-2026-00045`, the C# dump of the restored game shows `OtherGame` under `AutomationRules[0]` and `DealOffers[0..5]`.

- Test: `undo_recently_undone_events_point_at_old_game_in_dump`.
- The differ also reports it. On `fuzz-2026-00018` and `fuzz-2026-00045` the first difference is the undo line, at `Game.RecentlyUndoneEvents[0].Game` and `.Player`. 8 of the 9 sample fuzz traces contain `OtherGame`.
- Caught by: the differential fuzz traces, as soon as the replayer gets that far. Nothing else covers undo.

**Behaviour after a restore (suspected, not yet shown in a trace).** The dump difference is the visible part. The restored C# game also behaves differently: wherever C# code reads `.Player` or `.Game` from an event that the game holds, it gets the undo game's object. Rust always reads the current game. Sites:

- Reference comparisons that become false in C#: `Game.cs:977-978` (`CurrentFlightUsed.Player == p`, which changes the move distance), `Game_Bidding.cs:636`, `Game_Revival.cs:80`, `Game_Battle.cs:274/280`, `Game_Blow.cs:508`, and `PlayerSequence.JuiceForcesLast`.
- Writes that go to the other game's player in C#: `Game_Bidding.cs:137/178` (the Karama card returned to `CurrentBid.Player`), `Game_Battle.cs:220-228`, `387`, `399`.
- Reads of the event's own `Game`, such as `NexusPlayed.IsSecretAlly`, which `PerformPreEventTasks` calls.

Fixing this needs a design decision. Rust would have to keep the other game alive, which C# does through these references (Rust's `undo` consumes `self`, and `restore_recently_undone` drops the undo game). Events would also need a game identity, and their player references would need one too.

### 2. `Participation` is copied instead of shared

**Fixed.** `Game.participation` is a `Shared<Participation>`.

`new Game(version, participation)` keeps the caller's object (`Game.cs:114`), and `Undo`, `RestoreRecentlyUndone` and `DismissRecentlyUndone` pass the same object to the new game. `Game::new_int32_participation` clones it ([`game.rs`](../../engine/src/game.rs), the `participation: participation.clone()` field). Its doc comment says nothing mutates `Participation` after construction, but `AddPlayer`, `AddObserver`, `RemoveUser`, `OpenOrCloseSeat`, `SetOrUnsetHost` and `ResetSeats` do (`Game_Participation.cs:95-161`). The server calls them (`Server/GameHub_GameManagement.cs:43/127/190`).

So in C#, a seat change made after `Undo` is visible in the game that `RestoreRecentlyUndone` returns. A change the server makes to its own `Participation` object is visible through the game too. In Rust, neither is.

- Tests: `participation_is_shared_across_undo_and_restore` (probe: `participation.old.isParticipant5 = True`), `participation_passed_to_constructor_is_aliased` (probe: `participation.caller_alias.isHost7 = True`).
- Missed by: everything. The oracle never calls the participation methods. Undo traces don't mix in seat changes, and no unit test does either. The engine never triggers this. It matters for the server port (phase 3).
- What would catch the class: give the fuzzer a "participation" step kind that calls these six methods with random users and seats, mixed with undo, restore and dismiss steps. Dump `Participation` after each step.

### 3. Rust rejects `$`-named JSON members that .NET ignores

System.Text.Json (with the options at `Model/Utilities.cs:21`) only refuses metadata members such as `$id`, `$values` or `$foo` in polymorphic types, meaning events. On any other object it skips them as unknown members. Rust's `read_object` ([`json.rs:200-202`](../../engine/src/json.rs)) throws `JsonException` for every one of them.

- `GameState.Load` of `{"$values":[],"Version":3}` or `{"$id":"1","Version":3}` gives `Version=3` in .NET and `JsonException` in Rust. This also hits legacy savegames: a top-level `$values` next to other members survives the Newtonsoft conversion and then fails to load.
- An `EstablishPlayers` event whose `Settings` holds `"$foo":1` loads in .NET and fails in Rust. The same key on the event itself is rejected by both, which matches.
- Tests: `part2::game_state_load_matches_dotnet`, `part2::dollar_members_in_non_polymorphic_objects_are_ignored`.
- Missed by: bot and fuzz traces, because the fuzzer fills in values for known fields and never invents member names. `event_json.rs` generates malformed inputs but doesn't add `$` keys inside nested plain objects. A real savegame with Newtonsoft leftovers would hit it.
- What would catch the class: have the `event_json.rs` generator insert random `$`-prefixed keys at every nesting level, and add a `GameState.Load` corpus checked against .NET.

## Suspected differences

### 4. `Undo`, `RestoreRecentlyUndone` and `DismissRecentlyUndone` consume the game

**Fixed.** They now change the game in place and put the old game back when an exception escapes (`undo_identity.rs::undo_that_throws_keeps_the_old_game`).

C# leaves the caller's game object usable after these calls. If `Undo` throws (`untilEventNr < 0` gives ArgumentOutOfRangeException at `Game.cs:169`, or `EstablishPlayers` throws while replaying), the caller still has the original game. In Rust that game has already been moved into the call, and a panic drops it. The exception type and message match.

This is an API difference, not a state difference. It matters to a server that catches the exception and keeps going. No test was written, because the observable difference is "the value no longer exists", which a Rust test can't check against C#.

### 5. Differences in code the engine never calls

None of these is reachable from engine code today, so traces can't see them. A boundary test per generic instantiation, or per helper, would.

- `ObjectCounter.Highest` on an empty counter returns the type's default in C# (`0` for `int`, `Faction.None` for `Faction`) and `None` in Rust (`object_counter.rs`). The only caller (`Bot_Battle.cs:661`) uses `TreacheryCard`, where null and `None` agree.
- `Utilities.CloneList` and `CloneObjectDictionary` (`Utilities.cs:8-19`) call each type's shallow `ICloneable.Clone`. `Player.Clone` keeps the reference to the original game, and `Territory.Clone` makes new territory objects that no longer equal the map's by reference. Rust's `Clone` is a deep copy that keeps handle equality. Only `Game.Clone` calls these, and only the client and the C# tests call that.
- `Utilities.ScaleFont` uses the current culture (`ToLower`, `float.TryParse`). Rust uses Unicode lowercase and an invariant parse. They match under the invariant culture, and the code is UI-only.

## Checked and found equivalent

- `GameEvent`: `Initialize` (sets `Player` only when it finds one), `Execute` (statement order, the catch, the applicability message, how `justEnteredStartOfPhase` is computed), `IsApplicable`, `By`/`ByAllyOf` (NullReferenceException), `IdStringToObjects`/`ObjectsToIdString`, the `Log*` helpers, the `EstablishPlayers.Execute` override dispatch. `GameEvent.Clone` has no callers.
- `Game.cs`: the constructor (initial values, version check, leader state), `PerformPreEventTasks`/`PerformPostEventTasks` (type tests and `NexusPlayed` property pattern), the replay loops in `Undo`/`Restore`/`Dismiss`, `UpdateTimers` (UTC equality, `TotalHours < 1`, negative gaps), `FindMostRecentEvent`, `StartReport`, `MainPhaseStart`/`Middle`/`End` (`Version >= 103`), all eight `Enter` overloads, card knowledge, the force queries (C# hands out live `Battalion` objects, but no caller mutates them), `ChangeResourcesOnPlanet` (wrapping), `FlipBeneGesseritAdvisorsWhenAlone` (lazy `Where`, `Version >= 160`), `DetermineOccupationAtStartOrEndOfTurn` (`KeyValuePair` `Contains`), Vidal handling, `PlayKarma`/`KarmaPlayed`, the three `Discard` overloads (`First`/`SingleOrDefault` exceptions, statement order, Ecaz bonus), `TryLoad`, `Log` (the single-`Message` case), payments, occupation queries, `ContainsConflictingAlly`, `CountDifferentFactions`, `HasOrnithopters`, `DetermineMaximumMoveDistance`, `IsNotFull`, the storm-distance loops (negative `%`), the prescience checks, `TraitorsInPlay` (`Version <= 140`).
- `EndPhase`: all 30 phase cases and their `Enter` calls, every version gate (108, 122, 132, 134, 107, 179), the single read of `NextPlayerToBattle` at lines 147 and 367, the lazy `Where` loops at lines 178 and 270, wrapping charity arithmetic, and `ResetBattle`.
- `PlacementEvent`: `ParseForceLocations` exception types and messages (including `Location.ToString` in duplicate-key errors), `ValidateMove` order, checked `Sum` overflow, version gates 153, 156 and 177, and the `FremenRidesMonster` override.
- `PassableGameEvent`, `IBid`, `ILocationEvent`, `IPlacement`, `GameSettings`, `Participation`, `Moment`, `Timer` (including TimeSpan overflow), `ObjectCounter` (stable descending order, first maximum), `GameState.Load`/`GetStateAsString` on null and empty inputs, `Utilities.Px`, and `DetermineGameEvent`.
- `Game_Participation.cs`: every member, including `GetPlayer(null)`, the default key from `GetUserIdByName`, and the `&&`/`||` precedence in `IsOpen`.

## A note on a convention

Some code takes a player sequence out of the game, calls a method on it, and puts it back (`game_battle.rs` in `NextPlayerToBattle`, `game_move.rs:23/315/320`, `events/move.rs:73/79`). If the method throws, the sequence is lost, while C# keeps it. `with_bid_sequence` and `juice_played.rs` already handle this with `cs::catch`. For the battle and movement sequences, the only throw is "Could not determine the current player." when there are no players, which isn't reachable in practice. Use the `cs::catch` form anyway, so the pattern is safe wherever it gets copied.
