# Escapes

This file records differences between C# and Rust that our automated tests should have caught but didn't. [PORTING.md](../../PORTING.md) rule 6 says every escaped bug is a testing bug. When something other than the differential tests finds a difference, or finds it late, we fix the code, then add an entry here. Each entry covers a whole class of bug: what went wrong, why the tests let it through, and which test now catches that class. The test change lands before the entry is written. When a new bug matches an existing entry, the guarding test has a gap, and the entry should say how that gap was closed.

## Foundation review (2026-11)

The [foundation review](reviews/foundation.md) compared the first Rust modules with C# by hand. The failing tests it wrote are in [`review_foundation.rs`](../engine/tests/review_foundation.rs) and pass now. The systematic tests below came out of it. They live in `engine/tests/` and read golden files that `engine/csharp-ref` writes.

### Exception messages

**Bugs.**
- `ToNexus` and `ToFaction` threw with the message `faction` instead of the .NET text: "Specified argument was out of the range of valid values. (Parameter 'faction')", then "Actual value was 13." on a new line.
- `throw!(NullReferenceException)` with no message produced an empty message. C# `throw new NullReferenceException()` has a default text, and `Game_Battle.cs` throws it that way.
- Nothing threw IndexOutOfRangeException for C# arrays. `cs::index` copies the `List<T>` exception, which is a different type with a different message.

**Why the tests missed them.** The golden dump recorded only the exception type (`exception:ArgumentOutOfRangeException`). Unit tests asserted `type_name` alone.

**What catches the class now.**
- `static_data.rs` records `exception:<Type>:<Message>` for every lookup.
- `boundary.rs` calls every member of every ported type with edge inputs and compares the full exception text.
- `linq_table.rs` checks the default message of each exception type thrown without one, the `(Parameter '...')` and "Actual value was" forms, and the List versus array indexer exceptions.

### Random draws and selector calls

**Bugs.**
- `TakeRandomN` drew one number per element as soon as it was called. C# builds a lazy `OrderBy(...).Take(n)`. It draws nothing until it is enumerated, draws again on each enumeration, and never draws for `n <= 0`.
- `OneOfHighestNOrDefault` and `OneOfLowestNOrDefault` called the selector twice per comparison, 14 to 16 times for 5 items. `OrderBy` calls it once per element.

**Why the tests missed them.** The only golden check was one `RandomOrDefault` call per seed. No test counted selector calls, used `n` outside `1..size`, or looked at the random state after a helper ran.

**What catches the class now.** `random_helpers.rs` runs every helper in `ExtensionMethods`, plus `Deck`, over a grid:
- seeds;
- source sizes 0, 1, 2 and 5;
- `n` of -1, 0, 1, size and size+1;
- seven key shapes.

It compares four things: the result, the selector call count, `GetCount()`, and the next random value. The unseeded overloads are covered too. The C# side reseeds the private static `Random` through reflection, without changing `Shared`.

### Key comparison in the `HighestOrDefault` family

**Bugs.**
- String keys used Rust's ordinal `Ord`. C# compares `IComparable` keys with `Comparer.Default`, which is culture-sensitive. So the highest of `"a"` and `"B"` is `"B"` in C# and `"a"` in Rust.
- Float keys did not compile. In C#, NaN is the lowest key, and NaN equals NaN when ties are checked.

**Why the tests missed them.** The helpers were only ever called with `int` keys. The module doc said ordinal strings behave the same, and no test checked that claim.

**What catches the class now.** `random_helpers.rs` uses key shapes for ties, enums, bools, floats with NaN and -0, doubles with NaN, and strings that differ in case and punctuation. Keys are `cs::CsComparable`, which separates ordering from equality. `cs::compare_invariant` implements .NET's ICU collation for ASCII and panics on other text instead of guessing.

### LINQ stand-ins in `cs.rs`

**Bugs.**
- `cs.rs` had no predicate overloads, so `SingleOrDefault(p)` was ported as `filter(p).cs_single_or_default()`. That throws "more than one element" where C# says "more than one matching element". All three fetchers and `TreacheryCardManager.Get` used the pattern.
- Writing the table turned up a trap of the same kind. .NET's `Last(p)` on a `List<T>` or array walks backwards and stops at the first match, so a predicate with side effects runs fewer times than a forward scan.

**Why the tests missed them.** The compat crate's LINQ was checked against .NET, but `cs.rs` never was. The fetchers' ids are unique, so the throw can't happen there.

**What catches the class now.** `linq_table.rs` runs each operator that `cs.rs` stands in for, predicate overloads included, on two kinds of source: a `List<int>` and a true lazy sequence. It compares the value or exception text and the number of predicate calls. An operator in `linq.json` without a Rust handler fails the test.

One more .NET detail turned up while building the table: `Where` over an empty array returns an empty array, which then takes the list code paths. The C# side therefore uses an iterator method for the lazy case. The detail is noted on `cs_element_at`.

### `ToString()` of message elements

**Bugs.**
- Floats printed with Rust's `Display`, for example `0.00001` and `inf` where .NET prints `1E-05` and `Infinity`.
- Exceptions always printed as `System.<Type>`, so `KeyNotFoundException` lost its `System.Collections.Generic` namespace.
- Lists printed ``List`1[?]`` instead of their element type.

**Why the tests missed them.** The only message test used `2.5`. Its expected string, including the `[?]`, was copied from the Rust output rather than taken from C#.

**What catches the class now.** `message_corpus.rs` compares `to_cs_string` with C# `Message.ToString()` under the invariant culture for more than 5,000 elements:
- floats and doubles from random bit patterns, plus every special value;
- every value of every enum, including undefined values and flag combinations;
- every game object;
- collections of each runtime type;
- each exception type, with and without a message;
- nested messages.

Numbers now go through `dotnet_compat::fmt`.

### `params object?[]` arguments

**Bug.** `Message.Express(array)` with a single reference-type array spreads its items into the elements. That covers `object[]`, and through array covariance also `string[]` and `Location[]`. A value-type array, a `List<T>`, or an array followed by more arguments stays one element. The macros had no spread form, so a port could only produce one list element.

**Why the tests missed it.** No test passed an array to `Express`.

**What catches the class now.** The `params` section of `message_corpus.rs` checks the elements and the text for each kind of argument against the macros, including `express!(@params array)`. A new kind of argument on the C# side fails the test until it has a Rust case. C# passing a null array gives null elements and a `ToString()` that throws. Rust can't express that, and the test asserts that C# still behaves that way.

### Null where the Rust signature has no `Option`

**Bug.** `TreacheryCard.CounteredBy` takes a non-nullable `defense` in C#, but `Bots/Bot_Other.cs:1053` passes a plan's `Defense`, which can be null. With a null defense, C# returns a value for some weapon types and throws NullReferenceException for others. The Rust signature had no way to receive null.

**Why the tests missed it.** The golden test passed null only for `combinedWithWeapon`, and the C# signature declares both parameters non-null.

**What catches the class now.** `boundary.rs` passes null to every parameter that C# annotates as nullable. It also passes null to parameters in `NullInPractice` in `csharp-ref/Boundary.cs`, a hand-kept list of non-nullable parameters that callers pass null to. That list is the weak spot: a parameter only gets null inputs once someone lists it. The hazard survey and new bot ports should add to it.

### Record equality

**Bugs.**
- `PlanSummary` with a NaN `Dial` was unequal to itself under the derived `PartialEq`. C# record equality treats NaN as equal to NaN.
- `Route` compared its `From` list by contents. C# compares the `List<Location>` by reference.

**Why the tests missed them.** No test compared records.

**What catches the class now.** The review tests cover the two cases, and `PlanSummary`, `BattleFact` and `ExpressionElement` implement equality by hand with the C# semantics. The guard is partial. `boundary.rs` can't generate record instances, so it only forces every record `Equals` and `==` member to have an explicit entry naming the test that covers it. A new record type with float or list fields needs a hand-written equality test.

### Members and enums that were never ported

**Bugs.**
- Five enums (`AsideKind`, `BeneGesseritReactionKind`, `ShipKind`, `NexusAllianceChange`, `ErrorType`) were missing.
- The array-index helper was missing.
- `Payment.Reason` and `Report.Express(GameEvent)` were missing.

**Why the tests missed them.** The enum test looped over a hand-written Rust list and looked each name up in the golden file, so anything Rust lacked was skipped.

**What catches the class now.**
- The enum test, and every other golden test, loops over the C# entries and fails on any entry Rust doesn't handle.
- `boundary.rs` reflects over every public member of each type in `engine/tests/coverage/ported_types.txt`. It fails for any member that has neither a Rust table entry nor a reason in `not_ported.txt`. It also fails for entries that name members C# doesn't have. A new type is checked by adding it to `ported_types.txt` and regenerating.

### Live lists logged by `AuditCancelled`

**Bug.** `AuditCancelled.cs:89` passes `Game.AuditedCards` itself to `LogTo`. The next audit clears and refills that list (`Game_Battle.cs:427-429`), so from then on the C# state dump shows the new cards in the old message. The Rust port stored a copy taken when the message was logged.

**Why the tests missed it.** It shows up only in games where CHOAM audits twice. No sample trace does, and the 120 live bot games at seed 31 reached no second audit. The same class was found in `AmbassadorActivated` through its own differ failures. This site was found by reading hazards 4.5 during review.

**What catches the class now.** `LiveList::GameAuditedCards` makes the message read `Game.AuditedCards` when the state is dumped, as `LiveList::PlayerTreacheryCards` does for hands. `audit_cancelled_logs_audited_cards_as_a_live_list` in `pkg_i_battle_actions.rs` changes the list after logging and checks the dump. Hazards 4.5 lists every C# site that logs a live collection. Each one needs a `LiveList` variant, or a comment explaining why a copy is equivalent.

## Collections in messages (2026-11)

**Bugs.** `DealAccepted.cs:152` logs a player's `DiscardedTraitors` set itself, `TraitorsSelected` adds to that set, and deals can be accepted in any phase from game version 110 on (`Game_Events.cs:668-673`), including while traitors are still being picked. The Rust port stored a copy, with a comment saying the set doesn't change. The two earlier sites of this class, `AmbassadorActivated.cs:333` and `AuditCancelled.cs:89`, are fixed above. All other sites turned out to be equivalent, but nobody had checked them.

**Why the guidance let them through.**
- Hazards 4.5 named the risk, but its advice was conditional: copying is fine *if* messages are rendered right after their event. Nobody went back to answer that. The oracle's dump answered it. It enumerates every collection in every message at every step (trace-format.md 7.5), so copying is wrong for any collection that changes later.
- The engine conventions told porters that `Vec<T>` "becomes a list element" and didn't link to 4.5. A porter following the conventions copied, which is the right default but was never checked against the sites.
- The 4.5 site list came from a hand search. The `AuditCancelled` entry above then said 4.5 "lists every C# site", which nothing verified. Two of its sites were real escapes, and one was dismissed as "effectively fixed".
- The differ only sees the bug when a game changes the collection after logging it. Bot games rarely do, and the sample corpus never did.

**What catches the class now.**
- `csharp-gen --message-collections` finds every site from the Roslyn operation trees of Shared: each collection converted to `object`, each array passed as a whole `params` array, and each collection given to a log record. It writes `engine/tests/coverage/message_collections.txt`.
- `engine/tests/live_lists.rs` fails for any site without a row in [live-lists.md](live-lists.md), for any row without a site, and for a `live` row whose `LiveList` variant doesn't exist or isn't used by any port.
- The conventions now send porters to that table ([engine-conventions.md](engine-conventions.md#collections-in-messages)), and `LiveList` makes a live element a one-line change.

The weak spot is the generated file. It only changes when someone reruns the tool after changing Shared. Shared is the spec and rarely changes, so we accepted that.

## Null in collections of game objects (2026-11)

**Bugs.** C# stores `null` in collections of game objects and passes it along until a member access throws. About 30 sites did this, and the Rust port threw early or skipped the insert at each one. Some examples:

- `Player.TreacheryCards.Add(Card!)` with an unknown card id, in `IxianSelectedStartingCard`, `RicheseGaveCard`, `CardTraded`, `DiscardedSearched` and others. `Card!` doesn't throw, so C# puts null in the hand. Rust threw NullReferenceException, or added nothing.
- `Winners.Add(AlliedPlayer)` in `Game_Contemplate` adds a null player when the ally faction isn't playing.
- `MoveForces(from, To)` with a null `To`, in `FremenRidesMonster` and `Retreat`. C# removes the forces from `from` first and then throws ArgumentNullException on the null key. Rust threw NullReferenceException before changing anything.
- `KillAmbassadorIn(null)` for Atomics without a terror trigger throws ArgumentNullException in C#, and threw NullReferenceException in Rust.
- `EmperorBidSupport.GetMessage` with a null `Amounts`. `Amounts.Sum(...)` throws ArgumentNullException (Parameter 'source') in C#, and Rust threw NullReferenceException. The fuzzer found this one.
- `EstablishPlayers` with a null `Settings` or `InitialRules` stores the null in C#. Rust threw at the assignment, so the game kept its old settings.

**Why the conventions let them through.**

- The skeleton made every handle a plain `Copy` value with no way to be null. Only fields that C# annotates as nullable became `Option`, so a collection had nowhere to put a null.
- The conventions taught `x!.Foo` as `x.nre()`. Porters applied that to a bare `x!` passed to `Add` or to a function, where C# doesn't throw. Many sites carry a comment saying "a Rust hand cannot hold null", so the porters saw the difference but had no way to express it.
- `nre()` became the answer to every null, including the ones where .NET throws something else: ArgumentNullException from a dictionary or from a LINQ operator.
- `boundary.rs` passed null only to parameters that C# annotates as nullable.
- Most of these paths need an invalid id or an event executed without validation (replaying history). Bot games never get there. Fuzz games get there only through ids the fuzzer rarely picks.

**What catches the class now.**

- Every handle has a null value (`TreacheryCard::NULL`, `PlayerIndex::NULL`, ...). It throws from every member, throws ArgumentNullException as a dictionary key, and dumps as `null` ([engine-conventions.md](engine-conventions.md#null-handles)). The conventions now say `x!` is `x.or_null()` wherever C# stores or passes the value on.
- `csharp-ref/Boundary.cs` now passes null to every parameter of a game-object type, whatever its annotation. `every_member_matches_csharp` compares those cases with .NET.
- `boundary.rs::null_receivers_throw_null_reference` calls every ported member of every game-object type on the null handle, with the arguments of each C# case. Each call must throw NullReferenceException with the .NET message.
- `tests/null_handles.rs` checks the collection behaviour against .NET 10: `List.Remove(null)`, `HashSet.Add(null)`, null dictionary keys for every key-taking method, the dump, message elements, `GetId(null)`, and `linq_source`.

The weak spot is event code that turns an `Option` into a handle with `nre()` and passes it on. No test can tell whether C# throws at that point or later. The guard is the conventions plus the differ.

## `$` members in plain JSON objects (2026-11)

**Bug.** System.Text.Json rejects `$` members other than the leading `$type` only in polymorphic types, which here means events. In `GameSettings` (inside `EstablishPlayers`) and in `GameState`, it skips them as unknown members. `json::read_object` rejected every one of them, so a savegame that still had Newtonsoft `$id` or `$values` members failed to load.

**Why the tests missed it.** `event_json.rs` mutated only the top level of each event. The fuzzer fills in known members and never invents names.

**What catches the class now.** `csharp-ref/EventCorpus.cs` adds `$foo`, `$id`, `$ref`, `$type` and `$values` to every nested object of every event, both first and last, and to the top level of a savegame. .NET's decision is recorded for each case, and `event_json.rs` compares against it.

## Game identity across undo (2026-11)

**Bugs.** `Game.Undo`, `RestoreRecentlyUndone` and `DismissRecentlyUndone` build a new game that shares event objects with the old one. Rust had no way to say which game an event or a player belongs to.

- After `Undo`, the undone events still belong to the old game. C# dumps their `Game` as `{"$ref":"OtherGame"}` and their `Player` as the old game's player, in full. Rust wrote `{"$ref":"Game"}` and an index into the new game. Every fuzz trace with an undo failed at its first undo line: 7 of the 9 sample fuzz traces, and most failing live fuzz games.
- `GameEvent.Initialize` sets `Player` only when the initiator is seated. A replayed event whose initiator isn't seated in the new game keeps the old game's player, and executing it changes that player. Rust kept a bare index, which then pointed at whichever player of the new game had that position. In `fuzz-2026-00084` the first `EstablishPlayers` throws after seating players, so the original game has more players than the replay. The replay read the wrong players and made 4 more random draws than C#.
- `RestoreRecentlyUndone` returns the old game, whose history events now belong to the undo game. So do the events in its state (`CurrentBid`, `CurrentBattle`, `AggressorPlan`, ...). In C#, comparing their `Player` with one of this game's players is false, writes to it go to the undo game, and `BattleInitiated` members read the undo game's state.
- Each game's map has its own `Territory` objects, and C# compares territories by reference. After a restore in the middle of a battle, `CurrentBattle.Territory` comes from the undo game's map, and C# stored it in this game's `LeaderState` (dumped with `"foreign":true`, live fuzz seed 501 game 3). Rust territory handles had no map.
- `Location` objects are per map too. A location passed to a player of another game is the caller's map object, so C# dumps it without `"foreign":true` inside that player. Rust marked every location of another game's player foreign.
- `GameStatus` flash messages read `e.Game` of the last history event. After a restore that is the undo game, and Rust read the current game's `TreacheryDiscardPile` (NullReferenceException in Rust only).
- `ILocationEvent.To` of a held event (`LastShipmentOrMovement`) is read through the event's own game in C#, so `GameStatus` highlights and Bene Gesserit intrusion messages named the undo game's territory ([pkg-k review](reviews/pkg-k.md) item 3). `RevealCurrentNoField` and `BeneGesseritAccompanies` compare such locations with `ReferenceEquals`.
- The first version of the Rust model kept every game ever made from the current one, so memory grew with each undo, where C# frees unreachable games. Rust now drops games no reference reaches after each undo, restore and dismiss.
- `Participation` was copied instead of shared ([pkg-a review](reviews/pkg-a.md) item 2).

**Why the design missed it.**

- The skeleton generator mapped each C# reference by what it points at within one game: a `Player` became an index into "the" game, and a `Game` field became a marker with no value. That assumes one game per object graph. `Undo` is the only C# code that breaks the assumption, and it wasn't ported when the conventions were written, so nobody checked them against it.
- The field-mapping test did notice. Its value-kind check found `ref:OtherGame` and `obj:Player` in the sample dumps, where Rust could only write `ref:Game` and `ref:Player`. But those two findings went on an allowlist (`KNOWN_GAPS`) instead of failing the test. The design question turned into a "later" item, and undo was ported on top of the marker.
- The value-kind check only looks at sample traces, so a form shows up only once a trace reaches it. It also treated `{"$ref":"Location","id":5,"foreign":true}` as a plain location reference.
- The pkg-a review described the problem and the differ confirmed it, after undo had been ported.

**What catches the class now.**

- `field_mapping.rs::reference_types_write_every_form_they_claim` writes a real value of every form (this game, another game, null) through `GameRef`, `PlayerIndex`, `Location` and `Territory`, and compares the kinds written with the kinds the type's shape claims. The check below compared shapes only, so `Territory` once claimed `"foreign":true` while its writer could never produce it. The same test checks that a player of another game writes its own map's locations as foreign and the locations this game passed to it as not foreign.
- `field_mapping.rs::reference_fields_can_write_every_dumper_form` is a static check. For every field whose C# type the dumper writes as a reference (`Game`, `Player`, `Location`, `Territory`, read from `canon_schema.json`), the Rust field must be able to write every form `StateDumper` can produce, including the ones only another game produces. It needs no trace, so it would have failed as soon as the skeleton was generated.
- The value-kind check has no allowlist any more, and `"foreign":true` counts as its own kind. A form the C# dumper writes and Rust can't is a design gap to fix, not a known gap.
- Live fuzzing with recorded `GameStatus` calls (`differ live --status every`) found the map-object and flash cases; the differ's status comparison now covers code that only runs between events.
- `engine/tests/undo_identity.rs` runs undo, restore (both branches), dismiss, two undos in a row, an undo that throws, the replayed-event case from `fuzz-2026-00084`, the stored-event sites from the pkg-a review, stored-event members that read or change their own game, an event of the undo game holding a player of this game, map objects handed back from another game, and the shared `Participation`. It compares full state dumps with the C# reference program (`csharp-ref undo-identity`).

## `Some(NULL)` from helpers that return a collection element (2026-11)

**Bug.** `RandomOrDefault` over a hand that holds a null card returns that null. C# callers then skip it: `if (card != null)`, or `Game.Discard(TreacheryCard?)`, which returns on null. Rust's `random_or_default` returned `Some(TreacheryCard::NULL)`. `if let Some(card)` accepted it, and `discard_treachery_card_boolean` treated it as a real card. It logged the discard, removed the null from the hand and then threw ArgumentNullException on `RecentlyDiscarded.Add(null)`. The [package J review](reviews/pkg-j.md) confirmed this for CHOAM betrayal with a Nexus card (`NexusPlayed.cs:421-424`). Sabotage and the Atomics hand limit in `TerrorRevealed`, CHOAM's secret-ally audit in `NexusPlayed`, and the hand limit in `MakeAlliance` have the same shape.

**Why the null-handle fix let it through.** The conventions banned writing `Some(h)` for a handle that may be null, and the earlier entry said "the guard is the conventions plus the differ". But the `Some` was inside a generic helper, where no reviewer reads it as a handle, and none of the earlier tests took an element out of a collection that held a null.

**What catches the class now.**

- In C#, `Some(NULL)` and `None` are the same null, so the engine now handles them the same way on both sides:
  - Every function with an `Option<Handle>` parameter maps a null handle to `None` on entry. `tests/null_option_params.rs::every_option_handle_parameter_is_normalized` scans every engine source file and fails for a function that doesn't.
  - The "OrDefault" helpers map a null element to `None` (`cs::null_to_none`).
- `game_methods_treat_null_like_none` calls 30 `Game` and `Player` methods with `Some(NULL)` and with `None` on identical games. It requires the same result and a byte-identical state dump. `or_default_helpers_map_null_to_none` checks each helper.
- `review_pkg_j.rs::richese_gave_null_card_then_nexus_choam_betrayal` replays the C# scenario end to end and checks the C# state hashes.

The remaining gap is a handle taken from a collection without a helper (`hand[0]`, `.first()`) and wrapped in `Some` inline. The conventions cover that case, and no test does.

## C# changes after the port was generated (2026-11)

**Bug.** On Nov 7 the C# engine moved to game version 188, a day after the port was generated. Part of the change was ported by hand: `AlliancePassed`, the v188 alliance phase, `LatestVersion`, the `Revival.Validate` fix and the `GameStatus` changes. `PrescienceSeenPlans` was missing completely:

- the `Game` field;
- `BattleRevision` recording the withdrawn plan;
- the `Battle.Validate` check, which since this change dereferences `CurrentBattle.OpponentOf(p)` at every version, so a plan by a faction outside the battle now throws NullReferenceException in C#;
- `Battle.AspectsSeenByOpponent` and `Matches`;
- `EndPhase.ResetBattle` clearing the field.

Every artifact derived from the C# was also stale. The canon schema had no `PrescienceSeenPlans` field. `static_data.json` and `static_data.rs` hard-coded version 187. `events.jsonl` had no `AlliancePassed`. The line numbers in `message_collections.txt` and `lazy_queries.txt` were off, and the lazy-query list lacked `AspectsSeenByOpponent`. The oracle's `VersionGates` lacked 188. Every sample trace failed `Treachery.Oracle verify`, because the new field changes every state hash.

**Why nothing failed.** Every check against C# compared Rust with a snapshot: goldens written by `csharp-ref`, the canon schema, the coverage lists and the sample traces. No test read the current C#. So a C# change made the snapshots stale, and the Rust kept passing against the old C#. Only `differ live` runs the current C#. It would have caught the missing field in the end, on its first step. But nobody runs it on every change, and it says nothing about stale goldens.

**What catches the class now.**

- `engine/tests/csharp_sync.rs::csharp_sources_match_the_synced_manifest` hashes every `.cs` file in `Shared` and `Bots`. It compares them with `engine/tests/golden/csharp_sync.sha256`, written at the last sync, and names each changed, added or removed file. Run against the pre-Nov-7 C# (the `HEAD` blobs), it lists the 17 changed files and the new `AlliancePassed.cs`.
- `csharp_sync.rs::fuzzer_version_gates_cover_every_version_check` scans Shared for `Version <op> N` and fails for any `N` that `Oracle/GameConfig.cs` `VersionGates` leaves out. It would have failed for 188.
- [csharp-sync.md](csharp-sync.md) lists every artifact to regenerate and the order to do it in. The manifest is updated last.
- `static_data.rs` uses `Game::LATEST_VERSION` where the C# side uses `Game.LatestVersion`.
- `engine/tests/sync_v188.rs` runs the v188 rules at versions 187 and 188 and compares 366 lines with the C# (`csharp-ref sync-v188`). The lines cover the alliance phase in A and B, passing, breaking and offering, the status text, the seen plans after each revision, the validation messages, the outsider NullReferenceException and the reset.

## Lazy LINQ queries ported as `Vec` (2026-11)

**Bugs.** About 60 C# members return a lazy `Where` or `Select`, and their callers stop early with `Any()`, `First()`, `FirstOrDefault()` or `Contains()`. The Rust ports built a `Vec` first, so the predicate ran on elements that C# never reaches. When one of those throws, Rust throws where C# returns a value. Three reviews found instances:

- `Karma.ValidKarmaCards`, `ChoamDiscarded.ValidCards` and `NexusPlayed.ValidChoamCards` on a hand with a null card after the match. C# `CanKarma` returns true, and Rust threw NullReferenceException ([package J review](reviews/pkg-j.md), confirmed item 2).
- `FremenRidesMonster.ValidTargets(...).Contains(To)` with an ally faction nobody plays. The last clause dereferences the null `AlliedPlayer` on every location after the match ([package D review](reviews/pkg-d.md)).
- The same shape in `DiscoveryEntered.ValidTargets`, `TerrorRevealed.ValidSneakAttackTargets`, `NexusPlayed.ValidEcazTerritories`, `PlayerSequence.GetPlayersInSequence`, and the `.Any()` calls in `GetApplicableEvents` (reviews D, J and B, suspected).
- The new harness found two more. `DiscardedTaken.CanBePlayed` used `Count() > 0` where C# has `Any()`. `Game.IsNotFull` looked up the player before the stronghold test, where C# dereferences `p` only for strongholds and homeworlds, so `AmbassadorActivated.ValidFremenTargets` with a null player threw before its first location.

A related case is evaluation order inside the query. Several ports read `p.Faction` once before the query, where C# reads it inside the lambda. With a null player and no elements to test, C# doesn't throw.

**Why the hazard survey, the conventions and the reviews missed it.**

- Hazards 4.4 counted the lazy members and concluded that "returning an iterator or a `Vec` is equivalent *today*", because callers consume the result right away. That is true for the result. It is false for the work: `Any()` on a lazy query does less than `Any()` on a list. The survey thought about when the query runs, not how much of it runs.
- The engine conventions then made that a rule: methods over game state return `Vec`. The skeleton generator emitted `Vec` for every `IEnumerable` return, so every porter started from the wrong shape.
- Reviewers checked each site by asking whether the predicate can throw on a later element, and concluded it can't. That held only until null elements appeared. Hands can hold null cards, and an ally can name a faction with no player. Each review tested its own sites by hand, so nothing re-ran the old conclusions once the null handles landed.
- The differential runs only see this when a game holds the null element at the right position, which bots never produce and the fuzzer rarely does. `boundary.rs` gave up on any member with a `Game` parameter, and its inputs had no null after a matching element.

**What catches the class now.**

- `csharp-gen --lazy-queries` finds every lazy producer and every site from the Roslyn operation trees of Shared, with the consumer of each site: short-circuit operator, full enumeration, `foreach` with or without early exit, a local enumerated twice, or a pass to another member. It writes `engine/tests/coverage/lazy_queries.txt`.
- `engine/tests/lazy_queries.rs` fails for a site without a row in [lazy-queries.md](lazy-queries.md), for a stale row, and for a short-circuited site whose producer isn't `iter` with a Rust function that returns an iterator. That holds through `return` chains too.
- The `lazy` section of `boundary.json` (`csharp-ref/LazyBoundary.cs`) runs every `iter` producer that takes a `Game` or a `Player` on states with a null card after a match, a dangling ally, an unattached discovery stronghold, and a null player. It records the items yielded up to the exception. `boundary.rs::lazy_queries_yield_like_csharp` compares, and fails on 18 cases if `Karma.ValidKarmaCards` collects before yielding. `lazy_queries.rs` fails for an `iter` producer the harness doesn't call, unless it is listed with a reason.
- The conventions now say lazy producers return `impl Iterator` ([engine-conventions.md](engine-conventions.md#ienumerable-and-laziness)), and `lazy.rs` has `Either`, lazy `distinct` and `union`, and `cs_contains`.

The weak spots: the tool reads Shared only, so the bot port has to rerun it with Bots included. Inline chains such as `hand.Where(p).Any()` aren't listed, because a Rust port of them is one iterator chain and lazy already. And the harness's three scenarios exercise only the nulls we know about.

## Null arrays and C# 14 span binding (2026-11)

**Bug.** `DealOffered.Same` ends with `offeredDeal.To.SequenceEqual(To)` on two `Faction[]` (`Events/DealOffered.cs:87`). Shared is compiled with C# 14, which converts an array to a span implicitly, also as the receiver of an extension method. So the call binds to `MemoryExtensions.SequenceEqual`, a null array is an empty span, and a cancellation with `"To":null` matches an offer with `"To":[]`. Rust ported `Enumerable.SequenceEqual` and threw ArgumentNullException ([package K review](reviews/pkg-k.md) item 1). `Game.GetMaxRevivals` reads a possibly null `FactionsWithIncreasedRevivalLimits` the same way and returns the normal limit.

**Why the hazard survey, the conventions and the tests missed it.**

- The survey and every porter read `x.Contains(...)` and `x.SequenceEqual(...)` as LINQ, because that's how they read in the source. Which overload a call binds to depends on the compiler version, and nobody checked it. Hazards 1.5 and section 8 describe `Enumerable`, and the conventions say a LINQ call on a possibly null source throws ArgumentNullException (`cs::linq_source`). That's true for LINQ, but these calls aren't LINQ.
- `linq.json` compares the `cs.rs` helpers with `Enumerable` called by name in `csharp-ref`, which can't show which method Shared's own calls bind to.
- `boundary.rs` passes null to parameters, not to event fields that a member reads.
- The differ can only see the `Same` case when a fuzzed cancellation repeats an open offer's price, type, parameters, text, benefit and end phase, with a null `To`. That almost never happens.

**What catches the class now.**

- `csharp-gen --span-binding` asks Roslyn's semantic model which method every call binds to and lists every conversion to `Span<T>` or `ReadOnlySpan<T>` in Shared (`engine/tests/coverage/span_binding.txt`). It also scans the IL of the compiled `Treachery.Shared.dll` and fails unless the number of `MemoryExtensions` calls matches, so the list follows the real compiler, not Roslyn's idea of it.
- `engine/tests/span_binding.rs` fails for a site without a row in [span-binding.md](span-binding.md), for a stale row, and for a `MemoryExtensions` method that the doc doesn't compare with `Enumerable` on non-null arrays. Each row says whether the array can be null there. There are 8 sites: 2 can see null, and the other 6 read `Length` first or work on static data.

The weak spot: the tool reads Shared only. The bot port must run it over Bots too (`Bot_FactionSpecific.cs:713` reads `FactionsWithIncreasedRevivalLimits.Length`, which throws on null).

## Event collections stored as game state (2026-11)

**Bug.** `SetIncreasedRevivalLimits` executed without validation with `"Factions":null` stores null in `Game.FactionsWithIncreasedRevivalLimits` (`Events/SetIncreasedRevivalLimits.cs:52`), and C# dumps `null`. The Rust field was a `Vec`, so Rust stored `[]` ([package F review](reviews/pkg-f.md) item 1, [package K review](reviews/pkg-k.md) item 4; fuzz seed 9311, games 0 and 43). `Execute` had a comment saying "Validate has already rejected a null array", which is true only with validation on.

**Why the hazard survey, the conventions and the tests missed it.**

- Hazards 5.3 lists this assignment and `PermittedUseOfEmperorSpice = Amounts`, but only as aliasing: the game and the event share one object. The conventions then said a copy is equivalent because nothing mutates either. Neither asked whether the shared value can be null.
- The skeleton made a game field an `Option` only when C# annotates it nullable. `Faction[] FactionsWithIncreasedRevivalLimits` isn't annotated, and the null comes in through an event field, which JSON can always set to null.
- The "Null in collections of game objects" fix above made `EstablishPlayers` store a null `Settings` and `InitialRules`, and `PermittedUseOfEmperorSpice` an `Option`, one site at a time. Nothing listed the other sites of the same shape.
- The differ caught this as soon as the oracle executed 2% of fuzz steps without validation. Before that, no trace executed an event that would fail validation.

**What catches the class now.**

- `csharp-gen --stored-event-collections` lists every assignment of a collection taken from an event (directly, through an object reached from one, or from a collection parameter) to a field or property of anything that isn't an event (`engine/tests/coverage/stored_event_collections.txt`).
- `engine/tests/span_binding.rs::every_stored_event_collection_is_classified` fails for a site without a row in [span-binding.md](span-binding.md), and for a `nullable` row whose `Game` field isn't an `Option` in `game.rs`. There are 6 sites. Three can store null, and all three are `Option` now.
- The fuzzer's unvalidated steps keep finding the runtime effects.

The weak spot: the tool sees assignments only. A collection passed to a constructor of a model object, or added whole to another collection, isn't listed.

## Bene Gesserit flipping in a territory of another game's map (2026-11)

**Bug.** `BeneGesseritFlip` flips Bene Gesserit in `LastShipmentOrMovement.To.Territory`, which C# reads through the event's own game. After an undo and restore, that event belongs to the undo game, so the territory and its `Locations` come from the undo game's map. When the flip empties a battalion, `ChangeForces` removes it from `ForcesInLocations` and `ChangeSpecialForces` creates a new one keyed by the other map's location. C# then dumps that key with `"foreign":true`. Rust's `Player::flip_forces_territory_boolean` looped over `Territory::locations()`, which returns this map's locations whatever map the territory came from, so the new key had no map (fuzz seed 100 game 64, step 561).

**Why the tests missed it.** The undo identity entry above tagged `Territory` and `Location` handles with their map, and the conventions say `Location::territory` keeps the map. Nobody checked the other direction. `Territory::locations()` gives untagged locations, and `locations_of_map()`, which keeps the tag, was used only by `RevealCurrentNoField`. `undo_identity.rs` handed a location of another map to a player, but never passed a territory of another map to a `Player` method that loops over its locations. In the differ the bug needs a Bene Gesserit intrusion whose move was replayed by an undo, then a restore, then a flip that empties a battalion. That is rare even in live fuzzing.

**What catches the class now.**
- `undo_identity.rs::bene_gesserit_flip_in_a_territory_of_another_games_map` calls every `Player` method that takes a territory and changes forces (`FlipForces`, `KillForces`, `ForcesToReserves` in both forms, `KillAllForces`) with a territory of the undo game's map. It compares the full state dump with C# (`csharp-ref undo-identity`). Only `FlipForces` can leave a battalion keyed by the other map's location. The other four are in the test so that a change to them gets checked too.

The weak spot: game code outside `Player` still loops over `t.locations()` at 24 sites (`game.rs`, `game_battle.rs`, `game_blow.rs` and several events). Most only read force counts, which compare locations by id and so don't care about the map. A site that stores such a location, or compares it with `is_same_object`, needs `locations_of_map()`, and no test lists those sites yet.

## Dates without an offset in the database (2026-11)

**Bug.** The Rust server refused a `DateTimeOffset` column whose text had no offset, such as `2026-09-17 19:30:14` from SQLite's `datetime('now')`. `get_date_time` failed with `column LastLogin: not a DateTimeOffset`, so that user could never log in. `deploy/provision.sh` used to create the admin row this way, so production databases have such rows. The C# server reads them. EF Core reads the column with Microsoft.Data.Sqlite's `GetDateTimeOffset`, which behaves like `DateTimeOffset.Parse(text, InvariantCulture, DateTimeStyles.AssumeUniversal)`. Text without an offset is UTC whatever the machine's time zone, and ISO shapes with `T`, `Z`, `+hh`, `+hhmm`, any number of fraction digits and surrounding whitespace all parse. `db/date_time.rs` accepted only the exact text EF writes.

**Why the tests missed it.** The parser's tests used values copied from a database the C# server wrote, and `schema.rs::reads_a_real_database` reads the developer's own `Server/treachery.db`. Every row in both came from EF, which always writes `yyyy-MM-dd HH:mm:ss.FFFFFFFzzz`. A real database looked like proof of compatibility, but it only proved that we read what EF writes. The C# side of the comparison was EF's writer, never EF's reader, and nothing put text from another tool into a column.

**What catches the class now.**
- `server/tests/db_dates.rs` compares `date_time::parse` with EF itself. The H1 probe's `dates` mode inserts each text raw into `Users.LastLogin` and reads the entity back through EF Core 10.0.12. The golden (`tests/golden/ef_dates.jsonl`) holds 118 hand-picked texts (sqlite3, JavaScript, Python, Go, PostgreSQL and .NET output, range edges, rounding of long fractions) and 1,000 generated ISO-shaped texts. Each is read under `TZ=UTC`, `America/New_York` and `Asia/Kolkata`, and the test fails if any text reads differently between time zones. The ignored `against_dotnet` test runs 50,000 generated texts the same way. They all agreed.
- `schema.rs::reads_rows_written_by_other_tools` fills every date column of every table (`Users.LastLogin`, `Users.PasswordResetTokenCreated`, `UserTokens.Created`, `PersistedGames.CreationDate`, `LastAction`, `LastAsyncPlayMessageSent`, `ScheduledGames.DateTime`) with SQLite's own date functions. It reads them through the server's queries, including the login query, and checks each value against the instant SQLite sees in the same text.

The weak spot: .NET's parser takes far more than ISO 8601 (month names, `09/17/2026`, AM/PM, one-digit fields). Rust refuses those on purpose. `KNOWN_REFUSED` in `db_dates.rs` lists the ones we tried, and the test checks that .NET does accept them. A tool that writes one of those shapes would still break logins. The other columns (JSON, booleans, enums) have no test with text from other tools either. They are only ever written by the servers.

## Bot decisions validated in the live game instead of the event's own game (2026-11)

**Bug.** The bots' `Do` validates each decision before returning it. C# calls `action.Validate()` and `action.GetMessage()`, which read the event's own `Game`. The Rust `ClassicBot::do_` passed them the live game instead. Almost every bot event is built on the live game, so the two agree, but `DealOffered.Acceptance` builds the `DealAccepted` on the offer's game. After an undo, the offers in `DealOffers` belong to the game before the undo, so C# validated the acceptance in that old game, found it invalid, and the bot chose nothing. Rust validated it in the live game, found it valid, and the bot accepted the deal. Seen in fuzz seed 301 game 24, line 1960 (a `restoreRecentlyUndone` two lines earlier, a Richese bot and a one-card bidding-prescience offer). The fix is in `bots/src/bot.rs`: `do_` enters the event's game with `enter_own_game` before validating and describing it.

**Why the tests missed it.** Bot decisions were checked only in bots-mode games, which never undo. The B6 golden tests call `DetermineDealAccepted` on recorded states, but those come from bots games too, so no deal offer ever belonged to another game. The differ didn't run the bots on fuzz traces, where undo, restore and dismiss happen.

**What catches the class now.**
- `differ --bots check` runs the bots on fuzz traces of the `client` profile ([differ.md](differ.md#checking-bot-decisions)). Fuzz games undo, restore and dismiss often, so any bot code that reads an object of another game is exercised after undo.
- `differ/tests/bots_harness.rs::a_bot_event_of_another_game_is_validated_in_that_game` replays the seed 301 game 24 trace up to line 1960 with the bots on (golden `differ/tests/golden/fuzz-301-00024-deal-after-undo.jsonl.gz`).
