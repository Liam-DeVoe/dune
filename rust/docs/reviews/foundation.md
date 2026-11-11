# Review: foundation layer

This review compares every module in `rust/engine/src/` with its C# counterpart, member by member, looking for behaviour the golden test (`tests/static_data.rs`) cannot see.

Each confirmed difference has a failing test in [`engine/tests/review_foundation.rs`](../../engine/tests/review_foundation.rs). The C# side comes from a new `review` mode of the reference program ([`engine/csharp-ref/Review.cs`](../../engine/csharp-ref/Review.cs)). To reproduce:

```sh
export DOTNET_ROOT="$HOME/dotnet10" PATH="$HOME/dotnet10:$PATH"
cd rust/engine/csharp-ref && dotnet run -c Release -- review
cd .. && cargo test --test review_foundation
```

Each test names the probe line it checks. 20 tests fail and 1 passes (a guard).

## Summary

The game logic itself holds up. Map data, pathfinding, the neighbour cache, `PointAt` and its version gates, the fetchers, leaders, cards, `CounteredBy`, `Deck` and `RandomOrDefault` all match C# line by line. I found no difference in anything an engine event reaches today.

The differences are in the support code that later phases will lean on:

- exception messages,
- the bot-facing random helpers,
- LINQ helper coverage in `cs.rs`,
- `ToString` of message elements,
- a few members that were never ported.

The golden test misses all of them for the same three reasons:

- It records only the exception type, not the message.
- It never calls the helpers with edge inputs.
- It iterates the Rust list of items rather than the C# one, so anything Rust lacks is silently skipped.

## Confirmed differences

### 1. `ToNexus` / `ToFaction` exception message

`EnumerationExtensions.ToNexus` throws `new ArgumentOutOfRangeException(nameof(faction), faction, null)` for an undefined faction. In .NET that message is:

```
Specified argument was out of the range of valid values. (Parameter 'faction')
Actual value was 13.
```

Rust throws with the message `faction`. The same applies to `ToFaction` and `nexus`.

- Tests: `to_nexus_undefined_message`, `to_faction_undefined_message`.
- Missed because: `enum_ordering_and_serde` asserts only `type_name`, and the golden `Try` helper stores only `exception:<Type>`.

### 2. `TakeRandomN` draws when C# does not

C# is `source.OrderBy(_ => Random.Next()).Take(n)`. That is lazy, and LINQ never computes a key for `Take(0)` or a negative count. The probe shows the static `Random` state unchanged in both cases, and unchanged after building the query without enumerating it.

Rust draws one number per element at call time, whatever `n` is.

- Tests: `take_random_n_zero_draws_nothing`, `take_random_n_negative_draws_nothing`, `take_random_n_is_lazy`.
- Only `Test/Tests.cs:497` calls it, so it matters only if test-generated traces are replayed.
- Missed because: no golden covers any `ExtensionMethods` helper except one `RandomOrDefault` call per seed.

### 3. `OneOfHighestNOrDefault` / `OneOfLowestNOrDefault` call the selector too often

`OrderBy` computes each key once, so 5 items means 5 selector calls. Rust's `sort_by` calls the selector twice per comparison: 16 calls for highest, 14 for lowest. This breaks the rule in `engine-conventions.md` ("compute each key once"). It is visible only with a side-effecting selector. Hazards 16.3 says the bots' current `OneOf*` selectors are pure, so this is latent. The `_unseeded` copies have the same code.

- Tests: `one_of_highest_n_calls_selector_once_per_element`, `one_of_lowest_n_calls_selector_once_per_element`.
- Guard: `highest_or_default_calls_selector_twice_per_element` passes; both sides make 10 calls.

### 4. String keys in the `HighestOrDefault` family compare by culture in C#

The module doc says string keys compared ordinally behave the same. They don't. `Comparer<IComparable>.Default` calls `string.CompareTo`, which is culture-sensitive. In C#, `{"a", "B"}.HighestOrDefault(r, s => s)` returns `"B"`. Rust's `String: Ord` returns `"a"`.

Hazards 11.7 found no string-keyed sort in Shared or Bots, so this is latent too. The fix is to drop the claim from the doc and reject string keys.

- Tests: `highest_or_default_string_keys_use_culture_order`, `one_of_highest_n_string_keys_use_culture_order`.

### 5. `cs.rs` has no predicate overloads, so the documented port gets the wrong message

C# `SingleOrDefault(pred)` and `Single(pred)` say "matching element": "Sequence contains more than one matching element" and "Sequence contains no matching element".

The foundation ports `SingleOrDefault(pred)` as `filter(pred).cs_single_or_default()`. All three fetchers and `TreacheryCardManager::get` do this. The result throws the predicate-less messages instead: "more than one element" and "no elements".

The ids are unique, so the fetchers can't reach the throw. But hazards 1.5 counts 8 `SingleOrDefault(` sites in Shared, and 16.2 lists a reachable throw at `Bots/Bot_Other.cs:796`. Each will copy this pattern.

- Tests: `single_or_default_with_predicate_message`, `single_with_predicate_no_match_message`, `single_with_predicate_two_matches_message`.

### 6. `ExpressionElement::to_cs_string` formats floats with Rust rules

The code carries a TODO saying the formatting is "fine for the small numbers the game uses". But `dotnet_compat::fmt::f32_to_string` and `f64_to_string` already implement .NET formatting, and `expression.rs` doesn't use them.

| Value | C# | Rust |
|---|---|---|
| `0.00001f` | `1E-05` | `0.00001` |
| `1e20f` | `1E+20` | `100000000000000000000` |
| `1e-7` | `1E-07` | `0.0000001` |
| `float.PositiveInfinity` | `Infinity` (invariant), `∞` (en-US) | `inf` |

- Tests: `float_to_cs_string_small`, `float_to_cs_string_big`, `double_to_cs_string_small`, `float_to_cs_string_infinity`.

### 7. Exception elements use the wrong namespace

`to_cs_string` prints `System.{type}: {message}`. `KeyNotFoundException` lives in `System.Collections.Generic`, so C# prints `System.Collections.Generic.KeyNotFoundException: k`. A thrown exception's `ToString()` also carries a stack trace, which Rust can never match, so this only matters if the comparison strips the trace.

- Test: `exception_to_cs_string_namespace`.

### 8. List elements print `[?]`

This is already documented in `expression.rs`. C# prints the element type, for example ``System.Collections.Generic.List`1[Treachery.Shared.Faction]``.

- Test: `list_to_cs_string_type_name`.

### 9. Record equality

- `PlanSummary` with a NaN `Dial`: C# record equality uses `EqualityComparer<float>.Default`, where NaN equals NaN, so the records are equal. The derived Rust `PartialEq` says they are not.
- `Route`: C# compares the `List<Location> From` field by reference, so two routes with separate empty lists are not equal. Rust says they are.

Nothing compares these records today. The tests are there so nobody relies on the derived `PartialEq`.

- Tests: `plan_summary_equality_with_nan_dial`, `route_equality_compares_list_by_reference`.

## Suspected or unproven

- **Messages snapshot live collections.** C# keeps the `List` object, and the oracle's `StateDumper` enumerates it at dump time. `AmbassadorActivated.cs:333` stores a player's live hand. `AuditCancelled.cs:89` stores `Game.AuditedCards`, which the next audit clears and refills. Once those events are ported, a dump taken after the list changes should differ. This can't be shown before the events exist. Hazards 4.5 describes the mechanism, but `expression.rs` picked snapshotting without settling when the oracle renders.
- **`CounteredBy` with a null defense.** C# `laser.CounteredBy(null, null)` returns `false`. `poison.CounteredBy(null, null)` throws NullReferenceException. Both are probed. Rust's `defense` parameter is not an `Option`, so the bot call `opponentPlan.Weapon.CounteredBy(opponentPlan.Defense, null)` (`Bots/Bot_Other.cs:1053`, where `Defense` can be null) cannot be ported faithfully. The signature should take `Option<TreacheryCard>` and throw only on the branches that read it.
- **Float keys in the `HighestOrDefault` family.** Rust requires `K: Ord`, so float keys don't compile. Bot selectors return `float` (hazards 16.3: `GetDialNeeded`), and C# gives NaN its own semantics: `{1, NaN, 2}` picks 2 as highest and NaN as lowest. This is an API gap rather than a wrong answer.
- **Force obstacles in pathfinding were never exercised against C#.** Every golden pathfinding call uses a game with no forces and no blocked territories. `DetermineForceObstacles`, `BelongsTo` with a Ecaz ally, and the `Distinct().Union()` order read correctly line by line but are unproven.
- **`params object?[]` forwarding.** In C#, `Message.Express(someObjectArray)` passes the array as the element list, but Rust turns a `Vec` into one list element. This matters for the `Log(params object[])` wrappers in `Game` and when a caller passes a `string[]`.

## Missing members

- **Enums.** `AsideKind`, `BeneGesseritReactionKind`, `ShipKind` and `NexusAllianceChange` (from `Model/Log/`) and `ErrorType` (`Model/Result.cs`) are in the golden dump but not in Rust. The enum test never notices, because it loops over a hand-written Rust list and looks each name up in the golden file.
- **`Model/Log/*.cs` records.** None of the 9 files is ported. `with_log` accepts any `Debug` type, so nothing forces them to exist.
- **`Payment.Reason` and the 4-argument constructor.** There is a TODO for this. Message dumps that contain a payment will differ until it is added.
- **`Report.Express(GameEvent)`.** Noted in a comment.
- **An array-index helper.** `cs::index` throws ArgumentOutOfRangeException, the `List<T>` behaviour. C# arrays throw IndexOutOfRangeException ("Index was outside the bounds of the array."), and hazards 1.5 lists reachable array sites (`Split` results). Without a helper, those sites become a different exception or a Rust panic.
- **Documented gaps that are fine:** `Skin`/`DefaultDescriber`, `Deck.Randomize`, unseeded `new LoggedRandom()`, `Territory.Clone`, and `Game.Clone` sharing its map.

## Recommended systematic tests

Each of these covers a whole class rather than one site.

1. **Record exception messages in the golden dump.** Change `Try` in `csharp-ref/Program.cs` to emit `exception:<Type>:<Message>`, and the Rust `try_value` to match. This alone would have caught difference 1, and it guards every future lookup and throw.

2. **A boundary-input table for every ported member.** Have the C# side reflect over the public members of each ported type. Call every method that takes an enum or an `int` with each defined value plus -1, 0, 13, `int.MinValue` and `int.MaxValue`, and record the result or the exception text.

   The Rust side keeps a dispatch table keyed by member name. The test fails on any C# member that has no Rust entry or no `"not ported: <reason>"` line. That also turns "missing members" into a failing test instead of a review finding.

3. **Draw accounting for every random helper.** For each `ExtensionMethods` helper and `Deck` method, run a grid in both languages:
   - seeds, plus source sizes 0, 1, 2 and 5;
   - `n` in -1, 0, 1, size and size+1;
   - keys with and without ties;
   - a counting selector.

   Record the result, the selector call count, `GetCount()` and the next sample. The unseeded overloads can use the probe in `Review.cs`, which compares `ExtensionMethods.Random`'s internal state through reflection, until the C# seeding hook exists. This would have caught differences 2 and 3.

4. **A LINQ-semantics table for `cs.rs`.** For every C# operator that a `cs.rs` helper stands in for, including each predicate overload: run empty, one, two-match and no-match inputs, and record the value or the exception text. The compat crate already does this for its own operators. `cs.rs` was never held to the same standard. This would have caught difference 5.

5. **A `Message.ToString()` corpus.** Have C# print `Message.Express(x).ToString()`, under the invariant culture, for generated values of each element kind:
   - floats and doubles from random bit patterns, including NaN, the infinities, -0, subnormals and large exponents;
   - every enum, including undefined values and flag combinations;
   - each model type and nested messages;
   - one exception of each type that `cs.rs` throws.

   Compare with `to_cs_string`. The simpler fix is to route numbers through `dotnet_compat::fmt`, which is already checked against .NET. The corpus then guards against a regression. This would have caught differences 6 to 8.

6. **Iterate the C# side in golden tests.** The enum test, and any section keyed by name, should loop over the golden entries and fail on a missing Rust counterpart, not the other way round.

7. **Pathfinding with forces.** Extend `DumpPathfinding` to place forces through `Player` APIs, including two-faction strongholds, a Ecaz ally and blocked territories, before calling `FindNeighbours(..., checkForceObstacles: true)` and `FindPaths`. This needs `Player` in Rust, so it belongs with the first `Game` port.
