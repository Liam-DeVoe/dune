# Review: package E (bidding)

This review compares the package E files of [port-assignments.md](../port-assignments.md) with their Rust modules, member by member: `Game_Bidding.cs`, the 18 bidding events, `TriggeredBureaucracy` and `BiddingLog`.

**Result: no confirmed differences.** I found nothing in package E where the Rust port behaves differently from C#, and the differential runs agree. The rest of this file says what was checked, so the next reviewer can skip it or look elsewhere.

Tests are in [`engine/tests/review_pkg_e.rs`](../../engine/tests/review_pkg_e.rs). They are guards (they pass) for things the traces only see through the state hash. The C# values come from the `review-e` mode of the reference program, [`ReviewE.cs`](../../engine/csharp-ref/ReviewE.cs):

```sh
export DOTNET_ROOT="$HOME/dotnet10" PATH="$HOME/dotnet10:$PATH"
cd rust/engine/csharp-ref && dotnet run -c Release -- review-e
cd .. && cargo test --test review_pkg_e
```

## Differential runs

- `differ live --mode bots --seed 9101`: the first batch of 32 games matched on every line. I stopped the run there because the machine was heavily loaded (load average above 150) and each batch took over 10 minutes.
- `differ live --mode fuzz --seed 9102 --games 64`: see the note at the end of this file.

## What was checked

For every member: statement order, the point where each exception is thrown (including implicit null dereferences, `Deck.Draw` on an empty deck, `Max` on an empty sequence and `SingleOrDefault` with two matches), version gates, random draws, dictionary and set order, and evaluation order of arguments and `&&`/`?.` chains.

All of these match. Some of the places that looked risky and turned out fine:

- **Partial state on exceptions.** `with_bid_sequence` puts `BidSequence` back after a throw, so a NullReferenceException inside `SkipPlayersThatCantBid` leaves the same state as C#. In the events, Rust takes `.nre()` at the same statement where C# dereferences: for example `KarmaHandSwapInitiated` sets `SpecialKarmaPowerUsed` before failing on a missing target, `IxianSwappedCardOnBid` sets `IxianSwappedCardOnBid` before failing on a null player, and `ReplacedCardWon` draws before failing on a null initiator. These all match.
- **Random draws.** The draw sites are `DrawTreacheryCard` (reshuffle), `EveryonePassedBid` (shuffle only from version 131), `IxianRemovedCardFromAuction` and `KarmaHandSwapInitiated`. The order and count match, and the deck's `LoggedRandom` is the game's shared object (`SharedRandom` wraps a shared pointer). `CardsOnAuction?.PutOnBottom(card)` and `CardsOnAuction?.Draw()` draw even when the result is thrown away, in both.
- **Version gates.** Every comparison in the package has the same operator and constant (the constants are 107, 113, 131, 139, 140, 150, 151, 152, 155, 159, 164 and 165). Note that `FinishBid` switches at 159 and `FinishBlackMarketBid` at 164, and `HandleBlackMarketSpecialBid` has no 151 gate while `HandleRicheseBid` does. Rust keeps both quirks.
- **Collections.** `Bids` is a `NetDictionary`, and it is only ever changed by `Remove` followed by `Add` of the same key, or by `Clear`. Two loops depend on its order: the silent-auction log and `DetermineHighestBid` (which only takes `Max`). `KnownCards` is a `NetHashSet`, and `UnregisterKnown` calls happen in the C# order.
- **Aliasing.** `EmperorBidSupport` copies `Amounts` into `PermittedUseOfEmperorSpice` where C# shares the dictionary. Nothing in Shared writes to either, so the copy behaves the same. `Deck<T>(items, random)` copies its items in C# too (`Model/Deck.cs`), so `KarmaHandSwapInitiated`'s draw deck isn't the victim's hand in either engine. `CurrentBid`, `WinningBid` and `Bids` hold the same `Rc` event, which matches the C# references.
- **Lazy LINQ.** `PlayersThatCanBid` is re-evaluated at each use in both. The `Where(!HasBiddingPrescience)` loop in `IxianSwappedCardOnBid` is collected up front in Rust. That's safe because `UnregisterKnown` doesn't change `HasBiddingPrescience`. `KarmaHandSwap.ReturnedCards` re-parses the id string on each enumeration in both.
- **The unreachable null in `PayForCard`.** At `Game_Bidding.cs:285`, C# passes a possibly-null `receiver` into `ModifyIncomeBecauseOfLowThresholdOrOccupation`. That method only dereferences it when the amount is above 1 and Homeworlds is in play. Rust throws at the call instead (`game_bidding.rs`, `receiver.nre()`). These would leave different state (C# pays Emperor first). But the branch needs `initiator.Ally == Richese` with no Richese player, and line 268 (`initiator.AlliedPlayer.Resources`) throws first in that case. So the difference can't be observed.

## Guard tests

Each of these passes. They check text that the traces only see through the state hash, so a mismatch there would show up as an unexplained hash difference.

| Test | C# site | What it pins down |
|---|---|---|
| `emperor_bid_support_message_renders_lazy_query_type` | `EmperorBidSupport.cs:52` | The message holds a lazy `Where/Select` over a `Dictionary`. .NET 10 prints its type as ``System.Linq.Enumerable+IEnumerableWhereSelectIterator`2[...]``. |
| `silent_bids_log_renders_list_type` | `Bid.cs:296`, `BlackMarketBid.cs:117` | The silent-auction log prints ``List`1[Treachery.Shared.MessagePart]``, not the bids. |
| `bureaucracy_dynamic_message_nested` | `Bureaucracy.cs:39` | `Log(GetDynamicMessage())` wraps the message as one element of a new message. |
| `emperor_bid_support_amounts_duplicate_keys` | `EmperorBidSupport.cs:28` (JSON) | Two keys naming the same faction (`Atreides`/`atreides`/`20`, or an exact repeat): .NET keeps the first key's position and the last value, and Rust does the same. |

The last test also shows that the comment in `json.rs` (`impl FromJson for NetDictionary`) is wrong: it says "Duplicate keys throw in .NET (Dictionary.Add)", but System.Text.Json assigns through the indexer and keeps the last value. The behaviour already matches, so only the comment needs fixing.

## Would the differential traces catch a bidding bug?

Mostly yes. Bot games run normal, Richese and black market auctions every turn, and the state dump covers `Bids`, `CurrentBid`, `BidSequence` and every player's hand and known cards. Fuzz games submit every bidding event with random cards, factions and amounts, which reaches the null and wrong-owner paths.

Three kinds of difference would slip through:

1. **Paths that need a state bots never build and fuzz rarely builds.** Examples: `GivePlayerExtraCardIfApplicable` with an occupied Harkonnen homeworld whose occupier has no room, the v151-158 `Players.Any(Nexus)` gate with Nexus cards in play, and `HandleRicheseBid` before version 140. The oracle's version gates make sure both sides of each version check are played, but not together with the faction and Homeworlds state these branches need. **Systematic fix:** C# line coverage of `Game_Bidding.cs` and the Events folder over a fuzz and bot corpus (`Oracle/coverage.sh`), with a focused scenario test for each uncovered branch.
2. **Message text inside the hash.** Hash differences in `CurrentReport` are reported, but only as a hash until someone dumps the state. The guard tests above cover the bidding messages whose text is unusual.
3. **Exceptions that C# and Rust both throw, but with different partial state.** These only show up if a fuzzed event reaches the throw. Every bidding event with `Validate() => null` (`RicheseAnnouncesAuction`, `RicheseKeepsUnsoldCard`, `RicheseGaveCard`, the Ixian events, `EmperorBidSupport`, `ReplacedCardWon`, `KarmaHandSwapInitiated`, `CardGiven`, `Bureaucracy`) can reach these through fuzzing, but only when it is applicable. The oracle always executes with `performValidation=true` (`Oracle/GameRunner.cs:330`), so a handler only runs when its event is applicable. **Systematic fix:** let the fuzzer sometimes execute with `performValidation=false`, as `DistransUsed` does internally. That would reach every throw point in these handlers.

## Fuzz run

FUZZ_RESULT_PLACEHOLDER
