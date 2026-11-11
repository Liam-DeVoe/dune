# Package F review: revival and leaders

Scope: the 14 files in table F of [port-assignments.md](../port-assignments.md), C# and Rust side by side, member by member. Paths below are relative to `treachery-online/Shared/` (C#) and `rust/engine/src/` (Rust).

The port is careful. Every version gate, the two dead `Message.Express` checks in `Revival.Validate`, the double Recruits doubling at v179, the float/double split in the revival costs, the counting loops, the short-circuit order around `Hero` lookups, the lazy `Contains` on `ValidRevivedForceLocations` and the throw points for null players all match. I found one confirmed difference. It needs an event executed without validation.

## Confirmed

### 1. `SetIncreasedRevivalLimits` with null `Factions`, executed without validation

- C#: `Events/SetIncreasedRevivalLimits.cs:52`. `Game.FactionsWithIncreasedRevivalLimits = Factions;` stores null, then `Log()` throws `ArgumentNullException` in `GetMessage` (`Factions.Any()`).
- Rust: `events/set_increased_revival_limits.rs:61`. `self.factions.clone().unwrap_or_default()` stores an empty list, because `Game.factions_with_increased_revival_limits` is a `Vec` and can't be null.
- Effect: the state dump has `"FactionsWithIncreasedRevivalLimits":null` in C# and `[]` in Rust. Nothing else observable changes. `GetMaxRevivals` still returns the same value, because C# 14 binds `array.Contains(x)` to the span overload, which treats a null array as empty (probe line `sirl_null.max_revivals = 3`).
- Reachable only through `Execute(false, ...)`: `Game.TryLoad` with `performValidation: false`, or a host tool that skips validation. With validation, `Validate` throws first in both engines.
- Test: `set_increased_revival_limits_null_factions_without_validation_stores_null` in `engine/tests/review_pkg_f.rs`. Expected values from `csharp-ref review-f` (`ReviewF.cs`): `sirl_null.limits_is_null = True`, `sirl_null.history = 1`, `sirl_null.max_revivals = 3`.
- Would the traces catch it? No. The oracle always calls `Execute(true, isHost)` (`Oracle/GameRunner.cs:330`, `SavegameReplayer.cs:29`), so no trace executes an event that failed validation.
- What would catch the class: a fuzz mode that executes some fuzzed events with `performValidation: false` and compares state, like `TryLoad` of a tampered savegame. Any Rust field that can't be null where the C# field can, or any `Execute` that assumes `Validate` already ran, then shows up as a hash difference. A cheaper static check: grep for Rust code in `execute_concrete_event` that substitutes a default for a null event field (`unwrap_or_default`, comments like "Validate has already rejected"). This is the only such site in package F.

## Suspected, not confirmed

None that I could turn into a test. Things I checked and found equivalent, so the next reviewer doesn't repeat them:

- `ValidRevivalHeroes` returns a lazy `Union(...).Where(...)` in its empty-result branch (`Events/Revival.cs:226`). Rust collects it. Every caller enumerates once and immediately (`Game_LeaderState.cs:152`), and the lazy parts only read, so the result is the same.
- `RequestTleilaxuRevival.Validate` evaluates `ValidTargets` before `Hero`. Rust evaluates `Hero` only inside the `any` closure or the message. `Hero` never throws (`LeaderManager.cs:129-137`), so the order doesn't matter.
- `AcceptOrCancelTleilaxuRevival` (`:63`): a duplicate key in `EarlyRevivalsOffers.Add` throws after earlier adds and before `Clear()`. Rust does the same, with the same `Key: Treachery.Shared.Leader` text.
- `DetermineCostOfForcesForEmperor` (`Events/Revival.cs:130`): float products and sum, then `Math.Ceiling` in double, then a saturating `(int)` cast. Rust matches each step.
- `Game.Revive` and the ghola transfer when the owner is the initiator: remove then add at the end in both, so the leader moves to the end of the list.

## Differential runs

- Bots, `differ live --mode bots --seed 6162 --games 16`: 16 of 16 traces match (12,536 lines).
- Fuzz, `differ live --mode fuzz --seed 6161 --games 32`: no comparison. The first oracle batch exited with status 4 after two games hung inside `Execute`, and `live` stopped there. The machine was too loaded by other agents' runs to retry at a useful size.
