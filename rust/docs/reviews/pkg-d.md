# Review: package D (storm, spice blow, collection, charity)

This review compares the 26 package D files of [port-assignments.md](../port-assignments.md#d-storm-spice-blow-collection-charity) with their Rust modules, member by member.

Tests are in [`engine/tests/review_pkg_d.rs`](../../engine/tests/review_pkg_d.rs). The C# values come from the `review-d` mode of the reference program, [`ReviewD.cs`](../../engine/csharp-ref/ReviewD.cs):

```sh
export DOTNET_ROOT="$HOME/dotnet10" PATH="$HOME/dotnet10:$PATH"
cd rust/engine/csharp-ref && dotnet run -c Release -- review-d
cd .. && cargo test --test review_pkg_d
```

Result: 1 test fails (the confirmed difference below) and 2 pass (guards).

## Summary

The port is faithful. I found no difference that a real game can reach. One confirmed difference needs a state that the current rules can't produce, and two more sites have the same shape.

What I checked and found matching, so the next reviewer can skip it:

- Statement order and partial state before every throw: `DrawResourceCard` (including the `break` that skips `ThumperCalledShaiHulud = false`), `ReshuffleResourceDeck` (pile A is moved and cleared before pile B's null check), `ProcessBlowCard` (token drawn before the duplicate-key `Add` throws), `HarvesterPlayed` (`ArgumentNullException` "Value cannot be null. (Parameter 'key')", confirmed by the probe), `StormDialled` (`Dials[1]` throws after the dial is stored), `DealNexusCard`, `DivideResourcesFromCollection` (`tbd.Amount` read before `GainedByFirstFaction`).
- Version gates: all of them, including the `Version switch` with `when` arms in `ModifyIncomeBecauseOfLowThresholdOrOccupation`. A failing `when` on `>= 166` falls through to `>= 164`, and the Rust `if` chain does the same.
- Random draws: `DetermineLaterStormWithStormDeck`, the tech token deck, both resource deck shuffles, the Nexus deck reshuffle. Same calls, same order.
- Collection order: `ResourcesOnPlanet.Where(...).ToList()` is snapshotted in dictionary order; `DiscoveriesOnPlanet` and `AmbassadorsOnPlanet` stay `NetDictionary`.
- Live objects: `Game.Forces()` returns the players' live battalions. Rust copies them, which is safe because each battalion is read before its own player's forces change and nothing in the loop touches another player's battalion. `MonsterAppearence`, `Discovery` and `ResourcesToBeDivided` are never mutated after creation, so Rust's copies behave like the shared C# objects. `List.Remove` on `Monsters`, `CollectedResourcesToBeDivided`, the discard piles and `JustRevealedDiscoveryStrongholds` removes the same element.
- Floats: `(int)(0.5f * x)` in `DivideResources.GainedByFirstFaction` and in the occupation loss uses `f32` in Rust too.
- Lazy queries whose bodies change state: `PerformStorm`'s three loops, `CollectResourcesFromStrongholdsAndHomeworlds`, `PerformMonster`. Rust collects some of these first. The predicates only read data the loop bodies don't change, so the result is the same.
- C# bugs kept: the double `MainPhaseEnd`, the BlowA resume after storm losses, Sand Trout doubling surviving a blow lost to the storm, the "0 goes to occupier" log, the pre-162 occupation payment, charity paid when CHOAM can't pay, "A en B", "Discovery were removed", "11th".

## Confirmed differences

### 1. `FremenRidesMonster.ValidTargets` runs its predicate on every location; C# stops at the match

[`FremenRidesMonster.cs:45`](../../../treachery-online/Shared/Events/FremenRidesMonster.cs) calls `ValidTargets(Game, Player).Contains(To)`. `ValidTargets` (`:111-120`) is a lazy `Where` over `Map.Locations(false)`, so .NET's `ListWhereIterator.Contains` runs the predicate only up to the first match. Its last clause is `!p.HasAlly || Equals(l, g.Map.PolarSink) || !p.AlliedPlayer.Occupies(l)`, which throws a NullReferenceException when `Ally` names a faction with no player.

[`fremen_rides_monster.rs`](../../engine/src/events/fremen_rides_monster.rs) (`valid_targets`, `validate`) collects every target into a `Vec` first, so it evaluates that clause for locations after the match and throws where C# returns true.

Probe: a Fremen player with `Ally = Atreides`, no Atreides player, storm in sector 5. C# `ValidTargets(g, fremen).Contains(PolarSink)` is `True`. Rust throws NullReferenceException.

- Test: `rides_valid_targets_contains_is_lazy` (fails). Guards: `rides_valid_targets_later_location_throws_in_both`, `rides_valid_targets_count_without_ally`.
- Reachability: I don't see a way to get there in a game. Every `MakeAlliance` caller (`AllianceOffered`, `AllianceByTerror`, `AllianceByAmbassador`) passes two factions that have players. If one didn't, `Game_Alliances.cs:49-50` would set `playerA.Ally` and then throw on `playerB!`, leaving exactly this dangling ally. So it's latent: it becomes real as soon as any path produces such an `Ally`.
- Would the traces catch it: only if a trace produces a dangling ally, which the current rules don't.

## Suspected differences (same pattern, not shown)

The general rule behind item 1: when C# runs `Contains`, `Any` or `First` on a lazy `Where`, and Rust ports the `Where` as a function returning `Vec`, Rust runs the predicate on every element. That is only visible when the predicate can throw or has side effects. One more package D site has a predicate that can throw:

- `DiscoveryEntered.ValidTargets` ([`DiscoveryEntered.cs:31/38`](../../../treachery-online/Shared/Events/DiscoveryEntered.cs)): `ds.AttachedToLocation!.Territory` throws for an unattached discovered location. Rust ([`discovery_entered.rs`](../../engine/src/events/discovery_entered.rs), `valid_targets`) calls `.nre()` on every element. It diverges only if `JustRevealedDiscoveryStrongholds` holds an unattached location after the target, and `DiscoveryRevealed.cs:85-86` always calls `PointAt` before adding one. Not probed.

`KarmaMonster`'s `ValidKarmaCards(...).FirstOrDefault()` and `ToRide`'s `LocationsWithForcesThatCanRide(...).Any()` are eager in Rust too, but their predicates neither throw nor change state.

## What would catch this class

The differential traces only see it once a game reaches the throwing state, which neither bots nor fuzzing do today. A systematic check: for every `IEnumerable`-returning static (`ValidTargets`, `ValidSources`, `GetLocations` ...) that a `Validate` consumes with `Contains`/`Any`/`First`, have the boundary test call it on corrupted states (dangling `Ally`, unattached discovered locations, null `CurrentFremenSecretAlly.Player`) and compare the result or exception with C#. That fits `tests/boundary.rs` and `csharp-ref/Boundary.cs`, which already call public members on boundary inputs; they would need a "corrupt state" input generator besides the boundary values.

## Differential runs

On the engine as of this review, from `rust/`:

- `differ run traces/sample`: 17 of 17 traces match.
- `differ live --mode bots --seed 515 --games 16`: 16 of 16 match.
- `differ live --mode fuzz --seed 2027 --games 8`: 7 of 8 match. `fuzz-2027-00005` fails at its end-of-game queries with a Rust NullReferenceException at `engine/src/treachery_card.rs:107`, which is outside package D.
- Two larger fuzz runs (`--seed 4404 --games 200` and `--seed 9131 --games 40`) compared nothing. Each oracle batch had a game hang inside `Execute`, the oracle exited with status 4 ("oracle batch 0 failed"), and the differ dropped the whole batch without replaying the games that did finish. That is a harness gap: one hung game in a batch hides every other game in it. Small batches (`--games 8`) work around it.
