# Span binding and null arrays

Shared is compiled with C# 14 on .NET 10. C# 14 added implicit conversions from an array (and a `string`) to `Span<T>` and `ReadOnlySpan<T>`, and these also apply to the receiver of an extension method. So `array.Contains(x)` and `array.SequenceEqual(other)` on a `T[]` bind to the `System.MemoryExtensions` span overloads, not to `System.Linq.Enumerable`. The two differ on null. `Enumerable` throws `ArgumentNullException` for a null source. A null array converts to an empty span, so the `MemoryExtensions` overload returns as if the array were empty.

This file lists every place in Shared where that conversion happens, says whether the array can be null there, and says how the Rust port matches. A second table lists every place where game state stores a collection taken from an event, because an event field can be null and the stored null then reaches these calls.

## Evidence

Reading the source can't settle which overload a call binds to. These three checks can:

1. **Roslyn's semantic model.** `csharp-gen --span-binding` compiles Shared with `LanguageVersion.Latest` (C# 14 with Roslyn 5.0) and asks for the target method of every call. `Events/DealOffered.cs:87`, `offeredDeal.To.SequenceEqual(To)`, binds to `System.MemoryExtensions.SequenceEqual`.
2. **The compiled assembly.** The same tool scans the IL of the `Treachery.Shared.dll` that the .NET 10 SDK built, and fails unless it has exactly as many calls into `MemoryExtensions` as Roslyn found. On 2026-11-08 there were 8:

   ```
   Treachery.Shared.DealAccepted+<>c__DisplayClass36_0.<AcceptableDeals>b__0 -> MemoryExtensions.Contains
   Treachery.Shared.DealOffered.Matches -> MemoryExtensions.Contains
   Treachery.Shared.DealOffered.Same -> MemoryExtensions.SequenceEqual
   Treachery.Shared.Game+<>c__DisplayClass976_0.<DealRecipients>b__0 -> MemoryExtensions.Contains
   Treachery.Shared.Game.FindMostRecentEvent -> MemoryExtensions.Contains
   Treachery.Shared.Game.GetMaxRevivals -> MemoryExtensions.Contains
   Treachery.Shared.TreacheryCardManager+<>c__DisplayClass13_0.<GetCardsInPlay>b__0 -> MemoryExtensions.Contains
   Treachery.Shared.RicheseGaveCard+<>c.<ValidCards>b__7_0 -> MemoryExtensions.Contains
   ```

3. **Running it.** `csharp-ref review-k` (`engine/csharp-ref/ReviewK.cs`) calls the real `DealOffered.Same` from Shared on an offer with `"To":[]` and a cancellation with `"To":null`. It prints `deal_cancel_null_to.same = True`, and the cancellation removes the offer (`offers_after = 0`). The other direction, an offer with `"To":null` and a cancellation with `"To":[]`, also removes the offer. So C# does not throw for a null `To` in `Same`. The probe also prints `span_binding.contains_on_null_array = False` and `span_binding.sequence_equal_null_null = True`.

`engine/tests/review_pkg_k.rs` (`deal_cancel_with_null_to_matches_empty_to`, `deal_offer_with_null_to_cancelled_by_empty_to`) checks the Rust port against those values.

## How the sites are found

```sh
cd rust/engine/csharp-gen
dotnet run -c Release -- --span-binding ../tests/coverage/span_binding.txt
dotnet run -c Release -- --stored-event-collections ../tests/coverage/stored_event_collections.txt
```

`--span-binding` lists every conversion of a value that is not a span to `Span<T>` or `ReadOnlySpan<T>`, whether Roslyn records it as a conversion operation or on the argument. That covers extension receivers, ordinary arguments and `params ReadOnlySpan<T>` overloads, and arrays and strings alike. Roslyn crashes on three constructors of `Location` subclasses, as it does for [live-lists.md](live-lists.md). Those are listed as `UNANALYZED` and were checked by hand: they only assign properties.

`--stored-event-collections` lists every assignment of a collection-typed value to a field or property of an object that isn't an event, where the value is a member of an event or of an object reached through one (`Settings.InitialRules`), including both arms of `?:` and `??`. It also lists collection-typed parameters stored the same way, since a caller could pass an event member on.

`engine/tests/span_binding.rs` fails when a site in either file has no row below, when a row names a site that's gone, when a span site calls a `MemoryExtensions` method not in the methods table, or when a `nullable` row's `Game` field is not an `Option` in `game.rs`. Regenerate both files after any change to Shared ([csharp-sync.md](csharp-sync.md)).

## Methods

What each `MemoryExtensions` method does on an array that is not null, compared with the `Enumerable` method the code would bind to under C# 13.

| Method | Non-null semantics |
|---|---|
| `Contains` | Same. The span overload compares with `EqualityComparer<T>.Default`. `Enumerable.Contains` on an array goes to `ICollection<T>.Contains`, which is `Array.IndexOf` with the same comparer. Every receiver here is a `Faction[]`, `Rule[]` or `Type[]`: value equality for the enums, reference equality for `Type`. |
| `SequenceEqual` | Same. Both compare lengths, then elements in order with `EqualityComparer<T>.Default`. Only the null behaviour differs. |

## Span sites

Classes:

- **null-as-empty**: the array can be null when the call runs. Rust treats `None` as an empty slice there (`as_deref().unwrap_or(&[])`) with a `// C# parity:` comment.
- **not-null**: the array can't be null when the call runs, so only the non-null semantics matter, and those are the same (see Methods).

| Site | Class | Why |
|---|---|---|
| `Events/DealAccepted.cs:81:offer.To.Contains(p.Faction)` | not-null | `offer.To.Length == 0 \|\| ...` reads `Length` first, which throws NullReferenceException on a null `To`. Rust `deal_accepted.rs` does `offer.to.as_ref().nre()` before both. |
| `Events/DealOffered.cs:128:To.Contains(by)` | not-null | `To.Length == 0` comes first, as above. Rust `DealOffered::matches`. |
| `Events/DealOffered.cs:87:offeredDeal.To.SequenceEqual(To)` | null-as-empty | Either `To` can be null (JSON). `DealOffered::same` compares `as_deref().unwrap_or(&[])` on both sides, so null equals `[]` and nothing throws. |
| `Events/RicheseGaveCard.cs:49:c.Rules.Contains(Rule.RicheseTreacheryCards)` | not-null | `Rules` is static card data, the `params Rule[]` of each `new TreacheryCard(...)` in `TreacheryCardManager`. A null card `c` throws NullReferenceException on `c.Rules` before the call. |
| `Game.cs:285:types.Contains(History[i].GetType())` | not-null | A `params Type[]`; every caller passes separate `typeof(...)` arguments, and `types.Length` is read first. |
| `Game_Deals.cs:44:offer.To.Contains(p.Faction)` | not-null | `offer.To.Length == 0` first, as above. Rust `Game::deal_recipients`. |
| `Game_Revival.cs:114:FactionsWithIncreasedRevivalLimits.Contains(p.Faction)` | null-as-empty | Null after an unvalidated `SetIncreasedRevivalLimits` with `"Factions":null` (see the second table). `Game::get_max_revivals` reads `as_deref().unwrap_or(&[])`, so a null array gives the normal limit, as in C# (`review-f` probe `sirl_null.max_revivals = 3`). |
| `Model/DiscoveryStronghold.cs:16:UNANALYZED` | not-null | Roslyn crashed on the constructor. It only assigns `Territory` and `Discovery`. |
| `Model/HiddenMobileStronghold.cs:14:UNANALYZED` | not-null | Roslyn crashed on the constructor. It only assigns `Territory`. |
| `Model/Homeworld.cs:30:UNANALYZED` | not-null | Roslyn crashed on the constructor. It only assigns properties from its parameters. |
| `TreacheryCardManager.cs:129:c.Rules.Contains(rule)` | not-null | Static card data over `Items`, which holds no null. |

## Event collections stored in game state

An event's collection fields come from JSON and can be null. When game state stores one as is, C# keeps the null: the state dump writes `null`, and later code throws or treats it as empty depending on how it reads it. The Rust field must be an `Option` there. A Rust field that can't be null stores `[]` instead and dumps differently ([package F review](reviews/pkg-f.md) item 1, [package K review](reviews/pkg-k.md) item 4).

Classes:

- **nullable**: game state can hold null from this assignment. The Rust field is an `Option`, which the test checks in `game.rs`.
- **never-null**: the value can't be null here, or the code throws before the assignment can store a null.
- **not-state**: the target is not game state.

| Site | Class | Why |
|---|---|---|
| `Events/EstablishPlayers.cs:203:Game.Settings.AllowedFactionsInPlay = FactionsInPlay` | never-null | `FactionsInPlay` is a getter that builds a new list from `_factionsInPlay`, `[]` for null or empty. |
| `Events/EstablishPlayers.cs:211:Game.AllRules = Game.Version < 170 ? ApplicableRules.ToList() : Game.Settings.InitialRules` | nullable | From version 170, a null `Settings.InitialRules` is stored, and the next statement throws ArgumentNullException on `AllRules.Where`. The `ToList()` arm throws on a null `ApplicableRules` instead. Rust `game.all_rules` is an `Option`, and `cs::linq_source` throws the same exception. |
| `Events/EstablishPlayers.cs:268:Game.FactionsInPlay = Game.Settings.AllowedFactionsInPlay` | never-null | A null `AllowedFactionsInPlay` throws NullReferenceException earlier, in `DetermineApproximateRuleset` (`factions.Contains`, line 218). |
| `Events/EmperorBidSupport.cs:45:Game.PermittedUseOfEmperorSpice = Amounts` | nullable | `Validate` returns null, so `"Amounts":null` is stored with validation on. `Log()` then throws in `GetMessage` (`Amounts.Sum`), and `SpiceForBidsEmperorCanPay` (`TryGetValue`, `Game_Alliances.cs:160`) throws NullReferenceException. Rust `game.permitted_use_of_emperor_spice` is an `Option`, and the read uses `.nre()`. |
| `Events/SetIncreasedRevivalLimits.cs:52:Game.FactionsWithIncreasedRevivalLimits = Factions` | nullable | Only without validation (`Validate` throws ArgumentNullException on `Factions.Any`). C# stores null and `Log()` throws. The only engine read is the span `Contains` above. Rust `game.factions_with_increased_revival_limits` is an `Option` since this survey. |
| `Model/GameStatus.cs:39:WaitingForPlayers = waitingForPlayers` | not-state | A `GameStatus` is built per status query. Every caller passes a list it built (`[...]`, `ToList()`, `PlayerAndAlly`), as the [package K review](reviews/pkg-k.md) checked overload by overload. |
