# Review: package B (players and rules)

This review compares the package B files of [port-assignments.md](../port-assignments.md) with their Rust modules, member by member: `Game_Rules.cs`, `Game_Events.cs`, `Model/Player.cs`, `Model/PlayerSequence.cs` and `Model/Battalion.cs`.

## Result

No confirmed differences. Every member of the five files is ported, in C# order, with the same statement order, version gates, short-circuit order and exception points. The C# bugs listed in hazards §18 for these files are kept (`ForcesToReserves(Territory, int, bool)` using `toRemoveInTotal`, Richese special forces vanishing on the way to reserves, the silent no-op shipment when no single homeworld has enough forces, `StrongholdBonus` twice in `Expansion2AdvancedGame`).

Four places are equivalent only because of how the rest of the engine uses them today. They are listed under [suspected differences](#suspected-differences), with what would catch them if that changes.

## Tests

The tests are in [`engine/tests/review_pkg_b.rs`](../../engine/tests/review_pkg_b.rs). The expected lines come from the `review-b` mode of the reference program, [`ReviewB.cs`](../../engine/csharp-ref/ReviewB.cs):

```sh
export DOTNET_ROOT="$HOME/dotnet10" PATH="$HOME/dotnet10:$PATH"
cd rust/engine/csharp-ref && dotnet run -c Release -- review-b ../tests/review_pkg_b/probes.txt
cd .. && cargo test --release --test review_pkg_b
```

Both sides run the same pseudo-random operations from an identical xorshift32 generator and print one line per step, so the first differing line points at the operation that diverged.

- `player_force_operations_match_csharp`: six players (Emperor, Bene Gesserit, Richese, Fremen, Ixian, CHOAM) with their homeworlds, the Homeworlds and special-forces rules on, versions 163 and 187. 20 seeds per version, 120 random calls each, over all 25 public force methods of `Player` (add, remove, kill, revive, ship, move, flip, to reserves, per location and per territory, with negative amounts). After each call it prints the player's `ForcesInLocations` in dictionary order, the kill counters, reserves, both thresholds per homeworld, the homeworld battle bonus, `NoFieldIsActive`, `OccupiedTerritories`, `TerritoriesWithForces`, `ForcesOnPlanet` and `HaveForcesOnEachOthersHomeWorlds`. This covers dictionary slot reuse after removals (hazards §3.2), the Salusa Secundus threshold flag and its version gate, and the §18 bugs above.
- `determine_first_player_matches_csharp`: `PlayerSequence.DetermineFirstPlayer` for 1 to 12 seats, storm sectors -2 to 19, with every third seat empty. This covers the `float` arithmetic and the modulo, including the "Could not determine the first player." exception.
- `player_sequence_matches_csharp`: 150 random games per version (117 and 187) with random seats (including duplicates), full hands, `GoFirst`/`GoLast`/`Aggressor` juice, direction, start player (including null where C# doesn't hang) and all three constructors, followed by 14 random `NextPlayer`/`NextRound`/`CheckCurrentPlayer` calls with hand changes in between. Each step prints `FirstPlayer`, the `CurrentPlayer` backing field, `_played`, `_round`, `_fullyCircled`, `CurrentPlayer`, `HasPassedRichese`, `GetPlayersInSequence` and finally `IsAfter`. About 70 lines are exceptions ("Could not find a player after ...", "Could not determine the first player.").

All three pass. They guard against regressions, and they cover states the traces rarely reach.

I also ran the differ over `traces/sample`. All 8 failing traces fail on undo lines, which is known issue 2. Nothing in package B shows up before that. A `live --mode fuzz` run of 150 games didn't finish in two hours because the machine was heavily loaded (load average around 180), so I stopped it and have no result from it.

## Suspected differences

None of these changes behaviour today. Each one is a spot where Rust is equivalent only because of how the callers use it.

### 1. `GetPlayersInSequence` is eager

C# returns a lazy `Select` over `PlayersInOrder` (`Model/PlayerSequence.cs:199-202`). Rust builds the list right away ([`player_sequence.rs`](../../engine/src/player_sequence.rs), `get_players_in_sequence`). `Bid.PlayersToBid` (`Events/Bid.cs:111`) and `BlackMarketBid.PlayersToBid` (`Events/BlackMarketBid.cs:242`) call it and then throw the result away for silent auctions and unknown auction types. C# never runs `PlayersInOrder` in that case, but Rust does, so a throw or an endless loop in `PlayersInOrder` would only happen in Rust.

That can't happen today. For the sequences `StartBidSequenceAndAuctionType` builds for silent auctions, `PlayersInOrder` always terminates and doesn't throw. The only callers are in `GameStatus`, which the oracle never runs.

- Caught by traces: no. The oracle doesn't call `GameStatus`.
- What would catch the class: a status probe in the oracle that calls every `GameStatus` getter after each step and records the result or exception, or a boundary test that calls each lazy `IEnumerable` member and discards the result.

### 2. Player methods that need their own index

`name`, `is_bot`, `has_karma` and `maximum_number_of_cards` find the player's index with `game.index_of(self)`, which compares addresses ([`player.rs`](../../engine/src/player.rs)). On a `Player` copied out of `game.players`, `index_of` returns `None`. The first three then throw NullReferenceException, and `maximum_number_of_cards` quietly drops the CHOAM homeworld occupation bonus. In C# these work on any player object. No engine code copies a `Player` today (`grep` for `.clone()` on players finds only `cs_clone`, which is unused).

- Caught by traces: only once some code copies a player.
- What would catch the class: a test that fails on any `Player` clone in engine code (or a `debug_assert!` in `index_of` that the address is in `game.players`).

### 3. Copies instead of live battalions

`ForcesOnPlanet`, `BattalionsIn(Territory)`, both `BattalionIn` overloads and `Battalion.Take` return C# objects that are, at least sometimes, the player's live `Battalion` (hazards §5.4). Rust returns copies. That's equivalent only while nobody mutates the result or reads it after changing the player's forces. I checked every caller (`Game.cs:475/489/515/527`, `Game_Other.cs:53`, `Retreat.cs:58`, `FremenRidesMonster.cs:82`, `Battle.cs:604`, and the bots for `Take`/`TakeHalf`). They all read right away.

- Caught by traces: a future caller that reads after a mutation would show up as a state difference, but only if the fuzzer reaches it.
- What would catch the class: return `&Battalion`, or a read-only view, so that mutating or holding the result across a mutation doesn't compile.

### 4. `.Any()` on Vec-returning helpers in `GetApplicableEvents`

`AddPlayerActions` calls `.Any()` on `DiscoveryEntered.ValidTargets`, `Revival.NormallyRevivableHeroes`, `KilledHeroes`, `ResourcesAudited.ValidFactions`, `ChoamDiscarded.ValidCards`, `BeneGesseritBattleAnnouncement.ValidTerritories`, `Donated.ValidTargets`, `RicheseGaveCard.ValidCards` and (through `HasKarma`) `Karma.ValidKarmaCards`. C# stops at the first match. Rust computes the whole `Vec` and checks `is_empty()`. I checked each body: none draws random numbers, touches the neighbour cache, or can throw on a later element without throwing on the first one too. So nothing differs today.

- Caught by traces: only if a later element throws, which none does today.
- What would catch the class: port `X().Any()` as an iterator with `.any()` (or `.next().is_some()`) wherever the callee is lazy in C#, and have the `boundary.rs` table check that the number of predicate calls matches C#.

## What the traces cover

`GetApplicableEvents` is checked for every player after every trace step, and the full state dump includes every `Player` and `PlayerSequence` field. So differences in package B are caught whenever bots or the fuzzer reach the state. The oracle varies the game version over the version gates (`Oracle/GameConfig.cs:68-77`), so the `Version < 103` (Amal), `< 107`, `< 110`, `<= 117`, `<= 123`, `<= 150`, `>= 158`, `<= 163`/`>= 164` and `< 170` branches are reachable.

Two things limit this coverage today. Fuzz traces stop at their first undo line (known issue 2). And `PlayerSequence` with juice, duplicate seats or full hands needs very specific states. The probe tests above cover both of those directly.
