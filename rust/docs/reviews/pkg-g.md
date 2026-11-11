# Review: package G (shipment and movement)

This review compares the package G files of [port-assignments.md](../port-assignments.md) with their Rust modules, member by member: `Game_Move.cs`, `Shipment.cs`, `Move.cs`, `Caravan.cs`, the Bene Gesserit, CHOAM, HMS, Karma, flight and permission events, `Intrusion.cs` and `ShipMoveLog.cs`.

Tests are in [`engine/tests/review_pkg_g.rs`](../../engine/tests/review_pkg_g.rs). The C# values come from the `review-g` mode of the reference program, [`ReviewG.cs`](../../engine/csharp-ref/ReviewG.cs):

```sh
export DOTNET_ROOT="$HOME/dotnet10" PATH="$HOME/dotnet10:$PATH"
cd rust/engine/csharp-ref && dotnet run -c Release -- review-g
cd .. && cargo test --test review_pkg_g
```

All 3 tests pass. They are guards, not open failures.

## Summary

I found no confirmed difference in package G that is still open.

The port matches C# in statement order, throw points, exception types and messages, version gates and argument evaluation order. I checked:

- Every version comparison. The operators and constants match in all five files that have them.
- The phase tables in `Game_Move.cs`, including the inconsistent BeneGesseritAccompanies mapping between the Terror and Ambassador tables (`Game_Move.cs:548-551` vs `623-626`), which is kept.
- The other C# bugs listed in hazards §18 for these files. Sandmaster runs per source location, `BeneGesseritFlip` flips one territory and logs another, shipping back to reserves charges for special forces but moves only normal ones, and `Shipment` changes sub-phase before paying and logging. All are ported as in C#.
- Collections. `ShipmentPermissions` uses `NetDictionary` remove and add, the Intrusions queue keeps FIFO order, and `Intersect` and `Union` keep first-occurrence order.

One difference was open when the review started and was fixed during it (item 1). One code-level difference exists but can't be reached (item 2).

## Fixed during the review

### 1. `ChoamRemoveForce` with no location threw the wrong exception

With `_locationId = -1` and no validation, C# passes the null location into `Player.ForcesToReserves`/`SpecialForcesToReserves` (`ChoamRemoveForce.cs:116-119`). Those first add to reserves (`Player.cs:252-265`) and then throw `ArgumentNullException` from `ForcesInLocations.TryGetValue(null)`. The Rust port called `self.location(game).nre()` before the call, so it threw `NullReferenceException` and skipped the reserve change.

The null-handle work for known issue 1 fixed this while the review ran: `choam_remove_force.rs` now passes `or_null()`. `ChoamRemoveForce` wasn't on the known-issues list, so add it there.

- Test: `choam_remove_force_without_location_throws_argument_null` (passes now).
- Reachable through `Game.TryLoad(..., performValidation: false, ...)`, which the server uses for uploaded and restored games (`Server/GameHub_GameManagement.cs:20/121`, `Server/GameHub_Other.cs:352`). Validation rejects a null location, so the oracle never produces this event.

## Suspected, not reachable

### 2. `ValidShipmentLocations(...).Contains(To)` enumerates every location in Rust

C# `Shipment.Validate` (`Shipment.cs:203`) calls `Contains` on a lazy `Where` (`RegularShipmentLocations`, `Shipment.cs:343-352`), or on a lazy `Union(...).Distinct()` for Fremen (`:339`). `Contains` stops at the first match. Rust (`shipment.rs`, `valid_shipment_locations` and `regular_shipment_locations`) builds the whole `Vec` first.

The predicate can throw. `IsEitherValidHomeworldOrNoHomeworld` (`Shipment.cs:362`) and `Game.IsNotFull` (`Game.cs:1008-1009`) read `p.AlliedPlayer.X` whenever `p.HasAlly` is true. If `AlliedPlayer` were null, C# would accept a destination that comes before the first homeworld, while Rust would throw `NullReferenceException`.

It can't happen. Alliances are only made between two seated players (`Game_Alliances.cs:49-50`), and players are never removed, so `HasAlly` implies a non-null `AlliedPlayer`. No test was written. If ally data ever comes from somewhere new, such as a savegame field, this becomes reachable.

## Would the differential traces catch these?

Item 1: no. The oracle always executes with validation (`Oracle/GameRunner.cs:340`, `SavegameReplayer.cs:29`), so an event that validation rejects never reaches `ExecuteConcreteEvent` in a trace. Every "validation makes this impossible" shortcut in the port is blind to the traces in the same way.

**Systematic test for that class:** a fuzz mode that submits events with `performValidation: false`, as `TryLoad` does. It would fill the fields with the fuzzer's invalid values and compare exception type, message and state. One cheap version: replay each fuzz step a second time on a cloned game with validation off.

Item 2: no, and nothing can, because no reachable state triggers it.

## Coverage of package G by the traces

I generated and replayed new traces with the differ binary as of this review:

- **Bots, seed 7708, 24 games:** 19,641 lines, all match. The package G events in them were `Shipment` (724), `Move` (636), `BeneGesseritAccompanies` (215), `GuildDelay` (79), `PerformHmsMovement` (71), `BeneGesseritFlip` (30), `SetShipmentPermission` (6) and `ChoamRemoveForce` (1).
- **Fuzz, seed 7709, 12 games:** 2,352 lines matched before each trace stopped. 9 traces stopped at their first undo line (known issue 2) and 2 at `EmperorBidSupport` (known issue 1), between lines 63 and 746. So fuzz traces rarely get past the first few turns, and package G gets little fuzz coverage until undo is fixed.

These events never appeared in the bot traces: `Caravan`, `ChoamExtraMove`, `ChoamMovePrevention`, `KarmaShipmentPrevention`, `KarmaHmsMovement`, `FlightUsed`, `FlightDiscoveryUsed` and `HMSAdvantageChosen`. Until fuzz traces run past their undo lines, their execution paths are covered by this side-by-side review and nothing else. A focused trace per event (or a fuzzer steering rule that prefers event types not yet seen in the game) would close this gap.

### Harness problem: `differ live` drops a whole batch when one game hangs

When a fuzz game hangs inside `Execute`, the oracle exits with code 4 (`Oracle/Program.cs:181-187`). `differ live` accepts only exit codes 0 and 3 (`differ/src/live.rs`, `wait`), so it discards the whole batch and checks nothing. My run of `differ live --mode fuzz --seed 7707 --games 300 --jobs 6` stopped after its first batch of 24 games with "oracle batch 0 failed (exit status: 4)" and checked no traces. Treat exit code 4 like exit code 3: the other traces in the batch are complete and usable.

## Probes that confirmed matching behaviour

`review_pkg_g.rs` also pins down these edge cases, which match C#:

- `Shipment.ParseForceLocations`: a non-numeric id (`FormatException`), a duplicate location (`ArgumentException ... Key: Treachery.Shared.Location`), missing amounts (`IndexOutOfRangeException`), unknown and negative ids (`ArgumentOutOfRangeException` from `LocationLookup.Find`), and an amount too large for an `Int32` (`OverflowException`).
- `Shipment.DetermineCost`: an `Abs(int.MinValue)` overflow from wrapped force sums, an `int` product that wraps before conversion to `double` (`-2147483648`), back-to-reserves rounding through `float` (16,777,217 forces cost 8,388,608), negative counts, and Karma halving.
- `Shipment.Validate` for an unknown shipment type whose force sum wraps negative: both treat it as a shipment back to reserves.
- The order and count of `ValidShipmentLocations` for a non-Fremen player.
