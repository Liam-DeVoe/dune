# Package H review: battle plans and resolution

Scope: the 14 files in table H of [port-assignments.md](../port-assignments.md), C# and Rust side by side, member by member. This is the second review of the package. It rechecks the first one and goes deeper on the lines that [coverage.md](../coverage.md) says the traces never run.

## Result

No confirmed differences, so there is no `tests/review_pkg_h.rs` and no `review-h` mode in `csharp-ref`.

Every C# member has a Rust counterpart. Every `Version` comparison matches in operator and constant: I counted them per file on both sides with `grep -o "Version *[<>=!]=* *[0-9]*"`, and the counts are identical for all 10 files that have any. A live fuzz run (seed 9301, 32 games) matched C# on every game.

Four differences remain suspected. Each needs a state that validated events can't produce today, so none has a failing test. Three were already in the first review and are still in the code. The fourth (item 4) is new.

## Suspected differences

### 1. `TreacheryCalled.HasTraitor` skips `IsTraitor` when the opponent's hero is null

- C#: `Events/TreacheryCalled.cs:66`, `p.Traitors.Any(t => t.IsTraitor(h))`
- Rust: `src/events/treachery_called.rs`, `has_traitor`: `pl.traitors.iter().any(|t| h.is_some_and(|h| t.is_traitor(h)))`

C# calls `t.IsTraitor(h)` for every traitor, even when `h` is null. All three `IsTraitor` implementations return false for null (`Leader.cs:68`, `Messiah.cs:36`, `TreacheryCard.cs:36`), so the result is the same. The one difference is a null element in `Player.Traitors`. C# throws NullReferenceException on it, and Rust never touches it. No event puts a null into `Traitors` today.

### 2. `BattleInitiated.ExecuteConcreteEvent` reads `Aggressor` before `StartReport`

- C#: `Events/BattleInitiated.cs:220-223`. `StartReport` runs first, then `Aggressor`, `Defender` and `Territory` are read.
- Rust: `src/events/battle_initiated.rs`, `execute_concrete_event`. It reads the three values first, then calls `start_report`.

The values are the same either way. Only an exception from `Aggressor` would show the difference, because C# would already have added a report to `ReportList`. `Aggressor` can't throw with a seated initiator (see item 4 for the one path that could).

### 3. Eager `.nre()` on `CurrentBattle.Territory`

In several places C# passes a possibly null battle territory along, and Rust dereferences it at once:

- `Game.ResolveBattle` (`Game_Battle.cs:163`). C# passes `b.Territory!` to `DetermineBattleOutcome`, which first runs `ActivateDynamicWeapons` (it changes the plans' cards, logs, and sets `CardUsedByDiplomat`) and only throws later. Rust throws before any of that (`src/game_battle.rs`, `resolve_battle`).
- `BattleClaimed.DetermineEcazContribution` (`Events/BattleClaimed.cs:56,58`). C# calls `ecaz.AnyForcesIn(null)`, which returns 0, so the contribution becomes 0. Rust throws NullReferenceException (`src/events/battle_claimed.rs`, `determine_ecaz_contribution`). This one is new.
- `Retreat.ValidTargets` (`Events/Retreat.cs:58`), `ResolveEffectOfOwnedTueksSietch`, `HandleHarassAndWithdraw` and `HandleLosses`, as in the first review.

`CurrentBattle` only comes from a `BattleInitiated` that passed validation, and validation rejects a null territory. The oracle always validates, so none of these is reachable.

### 4. `IsInitiatorByJuice`/`IsTargetByJuice` dereference a null initiator too early

- C#: `Events/BattleInitiated.cs:71-73` passes `Player`, which can be null, to `IsInitiatorByJuice`/`IsTargetByJuice`. Those call `PlayerSequence.IsAfter(g, initiator, target)` (`Model/PlayerSequence.cs:227`), which reads `b.Seat` before `a.Seat` and can return true without touching the null `a`.
- Rust: `src/events/battle_initiated.rs`, `is_initiator_by_juice_opt` and `is_target_by_juice_opt`, call `PlayerSequence::is_after(g, initiator.nre(), target)`. That throws NullReferenceException before the loop.

This needs both the battle initiator's `Player` and the Juice event's `Player` to be null, because the check before it compares them by reference. Neither can happen with seated players, and after Undo an event keeps its old player instead of becoming null ([engine-conventions.md](../engine-conventions.md#game-identity-and-undo)).

## Checked and equal

These areas were compared statement by statement.

- **Lines the traces never run.** The `Retreat.ExecuteConcreteEvent` move loops, the Karama keep/discard loop in `LoserConcluded`, the pre-v179 Diplomat branch of `ActivateDynamicWeapons`, the Ecaz rock-melter totals, smuggler, sandmaster, Tueks Sietch, Sietch Tabr, Jacurutu, the special-force branch of reinforcements, the Graduate branch of Ixian substitution, the Juice strength log, the force-supplier branches of one and two traitors, the Messiah in a Lasgun/Shield explosion, and the Cunning branch of `TakeLosses`.
- **Weakly validated event fields that a fuzzer can set.** These are negative `Forces`, `AllyContributionAmount` and `BankerBonus` in `Battle`, and negative `SpecialForceLossesReplaced` in `BattleConcluded`, which flows into `KillForces(Territory, negative)`. Also arbitrary `StolenToken`, `DiscardedCards`, `NewTraitor`/`TraitorToReplace` (any hero id, including cards and the Messiah), and arbitrary `ForcedKeptOrDiscardedCards`. The integer arithmetic wraps the same way, and the list operations (`Remove`, `PutOnTop`, `Contains`) match.
- **Exception points.** These are the explicit throws, `RemoveAt(0)` on `StormLossesToTake` (it never fires first, because the indexer before it throws), `KillForces(null)` raising ArgumentNullException before any change, the `Enumerable.Range` count check, the duplicate-key `Add` on `CapturedLeaders` and `RecentlyDiscarded`, and `Discard(Player, Karma)`.
- **Float arithmetic.** `ForceValue` computes `(a*b)*c` in `float`. The totals add left to right in `float`. `Math.Ceiling`/`Math.Floor` widen to `double`, and `HalfOf` is `double`.
- **Random draws.** Decipherer draws two cards, then puts them back and shuffles. `PrepareAudit` shuffles and then draws. `SelectVictimOfHarkonnenWinner` builds the deck in list order.
- **Nullable comparisons.** `CurrentEcazOrAllyFighter == / != Faction.None`, `IHero? == Leader?` for Vidal (true when both are null), `WinnerHero` with a null winner, and `GetPlayer(Tleilaxu)?.Ally != Harkonnen`.
- **Laziness.** The `Where` filters in `LoserConcluded`, `ReturnSkilledLeadersInFrontOfShieldAfterBattle` and `KillAllForces(Territory)` read collections that their loop bodies don't change. `IdStringToObjects` is eager.
- **Kept C# bugs.** `TwoTraitorsCalled` never sets `BattleWinner`, the `break` in `BattlesToBeFought` leaves only the inner loop, the v146–v178 Rockmelter/Portable Antidote mix-up is kept, and the stray `KeptCard` message arguments are kept.

## Would the differential traces catch this class?

Items 1 to 4 need a state that validation forbids (a null territory in `CurrentBattle`, a null player on a stored event, a null element in `Traitors`). Bot and fuzz traces can't produce any of them, however many games run. What would catch the class is a probe in `engine/csharp-ref` that sets battle state through reflection and runs the same member on both sides, comparing the exception and the state change before it. Its inputs would be a `BattleInitiated` with territory id -1, an event with no `Player`, and null elements in the hero lists. The rule it enforces is "dereference where C# dereferences". So it should loop over every member that takes or reads a nullable battle object, rather than over hand-picked cases.

The unreached lines listed above are the real risk for ordinary porting slips. They read as correct, but only a seed that reaches them would prove it. The first review proposed a fix that still applies: a steered scenario generator that builds a battle with chosen plans, leader skills and strongholds and runs it through both engines.

## Live runs

- Fuzz, seed 9301, games 0 to 31: all 32 matched. I stopped the run there, because other differ runs were sharing the machine and each batch of 32 games took about 30 minutes.
