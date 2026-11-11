# Package K review: alliances, deals, discards, status

Scope: the 23 files in table K of [port-assignments.md](../port-assignments.md), C# and Rust side by side, member by member. C# paths are relative to `treachery-online/Shared/`, Rust paths to `rust/engine/src/`.

Tests are in `engine/tests/review_pkg_k.rs`. Expected values come from `csharp-ref review-k` (`engine/csharp-ref/ReviewK.cs`), run on .NET 10.

## Confirmed

### 1. `DealOffered.Same` throws on a null `To`, C# doesn't

`Same` ends with `offeredDeal.To.SequenceEqual(To)`, and both operands are `Faction[]` (`Events/DealOffered.cs:87`). With C# 14 on .NET 10, that call binds to `MemoryExtensions.SequenceEqual` on spans, not to `Enumerable.SequenceEqual`. A null array becomes an empty span, so nothing throws, and `null` equals `[]`. Rust ports the `Enumerable` version and throws `ArgumentNullException: Value cannot be null. (Parameter 'first')` (or `'second'`) (`events/deal_offered.rs`, `same`).

- Tests (both fail today):
  - `deal_cancel_with_null_to_matches_empty_to`: an offer with `"To":[]`, then a cancellation with `"To":null`. C# `Same` is True, the cancellation returns null and removes the offer (`offers_after = 0`, `History.Count = 3`). Rust returns "Game Error: Value cannot be null. (Parameter 'first')".
  - `deal_offer_with_null_to_cancelled_by_empty_to`: the other way round. C# removes the offer, Rust throws with `(Parameter 'second')`.
  - The probe also prints `span_binding.contains_on_null_array = False` and `span_binding.sequence_equal_null_null = True`, which confirms the binding for `Contains` too.
- Reachable with validation on: `DealOffered.Validate` always returns null, and deal events are applicable in every phase after setup from version 110. Only a fuzzer sends `"To":null`, and only on a cancellation that matches an open offer on every other field.
- Would the traces catch it? Fuzz traces could, as a result difference (`accepted` against `error`). It's unlikely, because the fuzzer has to repeat an earlier offer's price, type, parameters, text, benefit and end phase exactly.
- What would catch the class: this is a C# 14 language change, not an engine rule. In C# 14, `array.Contains(x)`, `array.SequenceEqual(y)` and similar calls on arrays bind to the `MemoryExtensions` span overloads, where a null array is empty instead of throwing `ArgumentNullException (Parameter 'source')`. The systematic check is a grep of the C# for extension calls on array-typed values (`Faction[]`, `Rule[]`, ...) whose value can be null, with a golden test of each call on a null array. The same root cause, a nullable C# array, has one more instance outside package K, and a differential run found it (item 4 below).

### 2. `List<T>.Contains(null)` finds a null card (fixed in the working tree during this review)

`CardTraded.Validate` checks `ValidCards(Player).Contains(Card)`, and `ValidCards` returns the hand itself (`Events/CardTraded.cs:55,66`). `DiscardedSearched.Validate` does the same on `TreacheryDiscardPile.Items` (`Events/DiscardedSearched.cs:44`). `Enumerable.Contains` on a `List<T>` calls `List<T>.Contains`, which returns true for a null `Card` when the list holds a null. Rust had `self.card().is_some_and(|c| valid_cards.contains(&c))`, which treats a null card as never contained, so it returned "Invalid card" where C# accepts.

- Tests: `card_traded_null_card_in_hand_is_valid` and `discarded_searched_null_card_in_pile_is_valid` (C# probes `card_traded_null.validate = valid`, `discarded_searched_null.validate = valid`). Both failed at the start of this review. Someone changed both sites, and `DiscardedTaken`, to `valid_cards.contains(&self.card().or_null())` while I was working, and both tests pass now.
- A null gets into a hand through `RicheseGaveCard` with an unknown card id ([package J review](pkg-j.md)), and into the discard pile through `BattleConcluded.DiscardedCards` with an unknown id.
- Would the traces catch it? Rarely, for the same reason as package J item 1: the fuzzer must first plant a null card.
- What would catch the class: the null-card fuzzer move proposed in [pkg-j.md](pkg-j.md) item 1. With nulls in hands and piles, every `Contains(Card)` validation runs on them.

### 3. `GameStatus` highlights and messages read the event's location from the wrong game

**Fixed.** Held events' `To` is read through the event's own game (`GameEvent::to_location`), in `GameStatus` and in the movement, intrusion and Bene Gesserit code. Tested against C# in `undo_identity.rs::status_reads_the_last_movement_through_its_own_game`.

`DetermineHighlights` returns `[game.LastShipmentOrMovement!.To!.Territory]` for the Terror, Ambassador, BeneGesseritIntrudedBy and BeneGesseritAccompanies phases (`Model/GameStatus.cs:563-590`). `To` is resolved through the event's own `Game.Map`. After an undo, `LastShipmentOrMovement` can be an event that still belongs to the other game, so C# returns a territory of the other game's map, and the dump writes it with `"foreign":true`. Rust calls `e.i_location_event_to(game)` with the game in place (`game_status.rs`, `determine_highlights`), so it returns this game's territory, untagged. [engine-conventions.md](../engine-conventions.md#game-identity-and-undo) says members that read another game's map must tag what they return. This member doesn't.

- Differ trace: `differ live --mode fuzz --seed 9311 --games 48 --status every`, game `fuzz-9311-00002` at line 1739, phase `BeneGesseritAccompaniesNonGuildShip`. Key `status`, first at `HighlightedTerritories[0].foreign`: C# `{"$ref":"Territory","id":4,"foreign":true}`, Rust `{"$ref":"Territory","id":4}`. Reproduce with `Treachery.Oracle.dll fuzz --seed 9311 --start 2 --games 1 --dump every --status every`, then `differ first`.
- The same read happens in `DetermineStatus` for `BeneGesseritIntrudedByFremenRidingMonsterA/B` and the other `BeneGesseritIntrudedBy...` arms (`Model/GameStatus.cs:227-228,406-407`), which put the territory in the messages. I expect the same `"foreign"` difference there. It isn't tested.
- The status only goes into the trace, not into game state, so only `--status every` runs can see it. They do, as above.
- What would catch the class: grep for every `i_location_event_to(`, `.to(` and `location_lookup()` call on an event that game code holds (`last_shipment_or_movement`, `current_battle`, plans, `latest_*`) and check it goes through the event's own game. An undo-heavy fuzz mode (undo right before status queries) would make the existing differ catch these more often.

### 4. `SetIncreasedRevivalLimits` with `"Factions":null` (outside package K)

Found by the same differential run, and an instance of the nullable-array class in item 1. Executed without validation (the fuzzer's 2% unvalidated steps), `Game.FactionsWithIncreasedRevivalLimits = Factions` stores null in C# (`Events/SetIncreasedRevivalLimits.cs:52`). Rust's field is a plain `Vec<Faction>`, so it stores `[]`. Later, `FactionsWithIncreasedRevivalLimits.Contains(p.Faction)` (`Game_Revival.cs:114`) binds to the span overload and returns false on null, so the behaviour probably agrees, but the state dump doesn't.

- Differ trace: games `fuzz-9311-00000` (line 188) and `fuzz-9311-00043` (line 483). Key `hash` after `SetIncreasedRevivalLimits`, `Game.FactionsWithIncreasedRevivalLimits: C# null / Rust []`.
- The traces do catch it, with `--unvalidated` steps, which `differ live` gets through the oracle's default of 0.02.

## Suspected, not confirmed

- **`GameStatus` in `MeltingRock` with a null `CurrentBattle.Player`.** C# passes `game.CurrentBattle!.Player` (possibly null) to `Status(..., Player?)` (`Model/GameStatus.cs:441`). Rust calls `.player()`, which throws NullReferenceException on null (`game_status.rs`, `Phase::MeltingRock`). `BattleInitiated.Player` is null only if its initiator isn't seated, which I found no way to reach.
- **`GameStatus.WaitingForMe(null)`.** C# `WaitingForPlayers.Contains(null)` is true when the list holds a null (from `PlayerAndAlly` with an unseated ally, `Model/GameStatus.cs:543`). Rust takes a `PlayerIndex` and compares `Some(player)`, so `PlayerIndex::NULL` never matches `None`. Only clients call this, with their own player. The differ doesn't compare it.

## Not reported again

`make_alliance_hand_limit_skips_null_cards` in `review_pkg_k.rs` is the `MakeAlliance` site of [pkg-j.md](pkg-j.md) item 1 (`random_or_default` returning `Some(NULL)`), which is being fixed. It passes now. Lazy `Where` queries that Rust collects eagerly (`AllianceOffered.ValidTargets(...).Contains`, `DiscardedTaken.ValidCards(...).Contains`/`.Any()`) belong to pkg-j item 2.

## Checked and equivalent

So the next reviewer doesn't repeat them:

- **`GameStatus.DetermineStatus` and `NextPlayerToBattle`.** The getter advances `BattleSequence`. C# reads it once in the `BattleReport` arm when the first guard holds, twice when it doesn't (and falls to "Unknown phase" if the second read is non-null), and twice in `DetermineBattleStatus` with no current battle. Rust reads it the same number of times in the same order. The `NonGuildShip` and `GuildShip` guards evaluate `CurrentFaction` and `GuildMayDelay` once or twice, as in C#, and neither has side effects.
- **`GameStatus` overload resolution.** Every `Status(...)` call was matched to the C# overload it binds to: `Faction` vs `Faction?` (`GetPlayer(null)` is null), `List<Faction>` (`OfType<Player>` drops nulls), `List<Player>` (keeps nulls: `PlayerAndAlly`, `PlayersThatNeedToMakeABattlePlan`), and `List<SequenceElement>` for `Bid.PlayersToBid`, `BlackMarketBid.PlayersToBid` and `GetPlayersInSequence`. `Ally()` evaluates `player!.Ally` even when the condition is false and throws when Ecaz isn't playing, in both.
- **`GameStatus` duplicates.** `BeneGesseritIntrudedByFremenRidingMonsterA/B` take the earlier arm (with `LastShipmentOrMovement!`). The `when` guards in `DetermineHighlights` apply to the whole `or` list.
- **`DetermineFlash`.** Event type patterns can't match subclasses, because no event class derives from another concrete event. `Milestone.Karma` flashes only after a `Bid`. The `Resource` milestone logic, including `is Phase.AllianceB or Phase.HarvesterB or Phase.BlowReport`, matches.
- **`MakeAlliance`, `BreakAlliance`, `BreakAllAlliances`.** The version gates (`> 150`, `<= 150`), the `ToList` snapshot with a re-check of `HasAlly`, and the `PermittedUseOf*` removals match. `NetDictionary` is used for every `Dictionary`.
- **`DetermineStrongholdOwnership`** removes and re-adds the key, so the freed slot is reused in the same order as .NET.
- **Victory checks.** `CheckNormalWin` and `CheckOtherWinConditions` iterate `PlayerSequence` the same number of times. A null ally pushed into `Winners` (`AlliedPlayer` of an unseated faction) throws NullReferenceException at the same later statement (`CheckBeneGesseritPrediction`, `GameEnded`). The Ecaz conditions dereference `AlliedPlayer!` only after the player controls the stronghold.
- **`Donated`** with a null target: the payer pays, then the null target throws, in both. The Double Inflation message is built and dropped in both. `Math.Abs(int.MinValue)` throws OverflowException in both.
- **`DealAccepted`/`DealRejected`.** `OpenOffer` stops at the first match in both (`DealRejected` uses `is_acceptable_deal` per element). The null `Player` branches of the bribe exchange match. `IsBot(offer.Player)` throws on a null player in both.
- **`GameStatistics.RoundToHalves`** rounds in `double` with `AwayFromZero`, casts to `float`, then divides.
- **`AllyPermission`.** The JSON aliases (`GuildAllowsShippingDiscount`/`AllyMayShipAsGuild` and the two others) read and write the same field. `_permittedKarmaCardId` defaults to 0, which is a real card, so every permission without that field grants Karama use of card 0 in both engines.
- **`Deal.DealContentsDescription`.** `int.Parse("?")` and `int.Parse("")` throw FormatException for `DontShipOrMoveTo` in both. `string.IsNullOrWhiteSpace` and Rust's `char::is_whitespace` accept the same characters.

## Differential runs

Both runs used `--status every`, so every `GameStatus.DetermineStatus` call between events was compared too. The differ binary was built from the engine as of this review. A later rebuild failed while fix agents were editing.

- `differ live --mode bots --seed 4711 --games 200 --jobs 8 --status every`: 200 of 200 passed, 125540 lines.
- `differ live --mode fuzz --seed 9311 --games 48 --jobs 4 --status every` (through `tools/heavy.sh`): 39 passed, 9 failed, 105377 lines. The failures:
  - 1 trace: item 3 above (`status`, `HighlightedTerritories[].foreign`).
  - 2 traces: item 4 above (`FactionsWithIncreasedRevivalLimits`).
  - 6 traces: `exception differs after EstablishPlayers`. C# throws `InvalidOperationException: Cannot draw from an empty deck` in phase `execute`, Rust in phase `validate` (for example `fuzz-9311-00019` at line 400). That's package C's `EstablishPlayers`, outside this review, and I didn't investigate it.

No `NextPlayerToBattle` difference showed up in either run. A status query that reads the getter a different number of times would change `BattleSequence` and show up in the next line's `hash`.
