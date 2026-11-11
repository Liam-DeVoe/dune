# Lazy queries

Many C# members in Shared return a deferred LINQ query (`Where`, `Select`, `Union`, an iterator with `yield`). The query does nothing until someone enumerates it, and it does only as much work as the consumer asks for. `Any()`, `First()`, `FirstOrDefault()`, `Contains()` and `All()` stop at the first element that decides the answer, so the predicate never runs on the elements after it.

If the Rust port returns a `Vec`, it runs the predicate on every element before the caller looks at any. The two engines then differ whenever a predicate on a later element throws (a null card in a hand, a null `AlliedPlayer`, an unattached discovered location), draws a random number, changes state, or fills the map's neighbour cache. [escapes.md](escapes.md#lazy-linq-queries-ported-as-vec) has the bugs this caused.

## The rule

A C# member that returns a lazy query returns `impl Iterator` in Rust, built from the same adapters in the same order, with `move` closures. The caller consumes it the way C# does:

| C# | Rust |
|---|---|
| `q.Any()` | `q.next().is_some()` |
| `!q.Any()` | `q.next().is_none()` |
| `q.Contains(x)` | `q.cs_contains(&x)` (`use crate::lazy::LazyQuery`) |
| `q.First()` / `q.FirstOrDefault()` | `q.cs_first()` / `q.next()` |
| `q.Count()`, `q.ToList()` | `q.count()`, `q.collect()` |
| `foreach (var x in q)` with a body that changes game state | an index loop that tests the predicate right before each body (hazards 4.1) |

What runs at the call and what runs per element must match C#:

- The receiver of the first operator is evaluated at the call. `p.TreacheryCards.Where(...)` dereferences `p` at the call, so `g.player(p)` goes before the iterator.
- Anything inside a lambda is evaluated per element. `Players.Where(x => x.Faction != p.Faction)` dereferences `p` only when there is an element to test, so a null `p` with no players doesn't throw. Read `g.player(p)` inside the closure in that case, and pass a null player as `PlayerIndex::NULL` rather than throwing up front.
- An iterator method (`yield return`) runs none of its body until the first `MoveNext`.
- A member that returns different queries on different branches returns `lazy::Either` (or a `Box<dyn Iterator>` for three or more), with `lazy::empty()` for `[]`.
- `Distinct` and `Union` are `lazy::distinct` and `lazy::union`, which yield as they go. `dotnet_compat::linq::distinct` and `union` collect.
- A lazy query held in a local and enumerated twice runs twice. Clone the iterator and run it again (`Game.TraitorsInPlay`).

A member whose every Shared caller enumerates the whole query once, with nothing in between (`ToList`, `Count`, a `List` constructor), may return a `Vec`. Its row in the table below says so.

`src/lazy.rs` has `Either`, `empty`, `distinct`, `union` and the `LazyQuery::cs_contains` method.

## How the sites are found

`engine/csharp-gen` lists them from the Roslyn operation trees of all of Shared:

```sh
export DOTNET_ROOT="$HOME/dotnet10" PATH="$HOME/dotnet10:$PATH"
cd rust/engine/csharp-gen && dotnet run -c Release -- --lazy-queries ../tests/coverage/lazy_queries.txt
```

A member is a **producer** when its declared return type is an `IEnumerable` interface and it is an iterator, or one of its `return` values is a deferred `System.Linq` operator, a call of another producer, or a conditional or `switch` with such a branch. A virtual or interface member is a producer when one of its implementations is. The tool finds producers to a fixed point.

A **site** is any call of a producer, any local initialized with an inline lazy query, and any `foreach` over an inline lazy query. For each site the tool follows the value up the operation tree, through further deferred operators, conditionals, locals and parameters of Shared methods, until something consumes it. The second column of `lazy_queries.txt` is that consumer:

- `short:Op`: an operator that can stop early.
- `full:Op`: an operator or BCL method that enumerates everything once.
- `foreach` and `foreach-exit(break|return|throw)`: a loop, and whether its body can leave early.
- `return M`: the query is the return value of producer `M`, which has its own sites.
- `arg:M(p)[...]`: passed to a Shared method, with how that method uses the parameter in brackets.
- `local x{...}`: held in a local, with each use. A `double` prefix means the local is enumerated more than once, or inside a loop or lambda.
- `via A>B`: the deferred operators applied between the call and the consumer.

Not covered:

- Inline chains consumed on the spot, such as `hand.Where(p).Any()`. Rust ports these as one iterator chain, which is lazy already.
- Bots. The tool reads Shared only. Rerun it with Bots included when the bots are ported, because bots call many of these producers with `Contains` and `Any`.

`engine/tests/lazy_queries.rs` fails if a line in `lazy_queries.txt` has no row below, if a row names a site that no longer exists, if a site whose consumer stops early isn't classed `short` (or `double`), or if a producer reached by such a site, directly or through `return`, isn't classed `iter` with a Rust function that returns an iterator. Regenerate the file after any change to Shared.

## Checking the behaviour

The `lazy` section of `tests/golden/boundary.json` calls each `iter` producer that takes a `Game` or a `Player` (`csharp-ref/LazyBoundary.cs`) on three game states:

- every hand is a Karama card, matching cards of other kinds, then a null card, then another Karama card, and traitor and face dancer lists end in null;
- the same, plus forces on the planet and every player allied to a faction nobody plays, so `AlliedPlayer` is null;
- the same, plus two just-revealed discovery strongholds, the second one unattached.

Each producer is called for every player and for a null player. The golden file records the items it yields, in order, up to the first exception. A port that builds a list first throws before yielding anything, so the prefix differs. `boundary.rs` (`lazy_queries_yield_like_csharp`) rebuilds the states and compares. `lazy_queries.rs` fails for an `iter` producer that the harness doesn't call and that isn't in its `NOT_PROBED` list with a reason.

## Classes

Producers:

- **iter**: the Rust function returns an iterator. The row names it.
- **vec**: the Rust function returns a collection. Allowed only when no Shared caller stops early. The row says why the copy is equivalent.

Sites:

- **short**: the consumer stops early. The producer is `iter`, and the Rust caller uses `next`, `any`, `find` or `cs_contains`.
- **full**: enumerated once, completely, with nothing in between.
- **foreach**: a loop whose body runs between predicate calls. The row says why the Rust loop matches: either it tests the predicate right before each body, or the predicate reads nothing the body changes and can't throw.
- **forward**: the query is returned or passed on, and another row covers the consumer.
- **double**: enumerated more than once. Rust runs the query again each time.

## Producers

| Producer | Class | Rust and why |
|---|---|---|
| `Enumerations.cs:20:Enumerations.GetValuesExceptDefault` | iter | `src/enumerations.rs::get_values_except_default` |
| `Events/AllianceOffered.cs:41:AllianceOffered.ValidTargets` | iter | `src/events/alliance_offered.rs::valid_targets` |
| `Events/AmbassadorActivated.cs:231:AmbassadorActivated.ValidFremenTargets` | iter | `src/events/ambassador_activated.rs::valid_fremen_targets` |
| `Events/AmbassadorActivated.cs:239:AmbassadorActivated.ValidGuildTargets` | iter | `src/events/ambassador_activated.rs::valid_guild_targets` |
| `Events/AmbassadorPlaced.cs:52:AmbassadorPlaced.ValidStrongholds` | iter | `src/events/ambassador_placed.rs::valid_strongholds` |
| `Events/AuditCancelled.cs:57:AuditCancelled.GetCardsThatMayBeAudited` | vec | `src/events/audit_cancelled.rs::get_cards_that_may_be_audited`. Its Shared callers enumerate it fully once: `Count()` and the `Deck` constructor's `new List(items)`. |
| `Events/Battle.cs:475:Battle.AspectsSeenByOpponent` | vec | `src/events/battle.rs::aspects_seen_by_opponent`. An iterator in C#. Its only caller, the `foreach` in `BattleRevision`, enumerates it fully. The loop body writes only `PrescienceSeenPlans`, which the iterator doesn't read, and nothing in it throws once `CurrentBattle` is set (see the comment on the Rust fn). |
| `Events/Battle.cs:774:Battle.CheapHeroes` | iter | `src/events/battle.rs::cheap_heroes` |
| `Events/Battle.cs:804:Battle.CardsPlayableAsDefense` | vec | `src/events/battle.rs::cards_playable_as_defense`. The only caller calls `ToArray()`. |
| `Events/BattleConcluded.cs:336:BattleConcluded.ValidTraitorsToReplace` | iter | `src/events/battle_concluded.rs::valid_traitors_to_replace` |
| `Events/Bid.cs:109:Bid.PlayersToBid` | vec | `src/events/bid.rs::players_to_bid`. The callers (`GameStatus`) call `ToList()`. The `GetPlayersInSequence` query inside is lazy, so its selector runs only in the arms that return it. |
| `Events/Bid.cs:120:Bid.ValidKarmaCards` | iter | `src/events/bid.rs::valid_karma_cards` |
| `Events/BlackMarketBid.cs:240:BlackMarketBid.PlayersToBid` | vec | `src/events/black_market_bid.rs::players_to_bid`. As `Bid.PlayersToBid`. |
| `Events/BeneGesseritBattleAnnouncement.cs:53:BeneGesseritBattleAnnouncement.ValidTerritories` | iter | `src/events/bene_gesserit_battle_announcement.rs::valid_territories` |
| `Events/BeneGesseritPrediction.cs:44:BeneGesseritPrediction.ValidTargets` | iter | `src/events/bene_gesserit_prediction.rs::valid_targets` |
| `Events/ChoamDiscarded.cs:49:ChoamDiscarded.ValidCards` | iter | `src/events/choam_discarded.rs::valid_cards` |
| `Events/ChoamMovePrevention.cs:48:ChoamMovePrevention.ValidTerritories` | iter | `src/events/choam_move_prevention.rs::valid_territories` |
| `Events/ChoamRemoveForce.cs:55:ChoamRemoveForce.ValidLocations` | iter | `src/events/choam_remove_force.rs::valid_locations` |
| `Events/ChoamRemoveForce.cs:60:ChoamRemoveForce.ValidFactions` | iter | `src/events/choam_remove_force.rs::valid_factions` |
| `Events/ClairVoyancePlayed.cs:122:ClairVoyancePlayed.ValidTargets` | iter | `src/events/clair_voyance_played.rs::valid_targets` |
| `Events/ClairVoyancePlayed.cs:127:ClairVoyancePlayed.ValidQuestions` | iter | `src/events/clair_voyance_played.rs::valid_questions` |
| `Events/DealAccepted.cs:74:DealAccepted.AcceptableDeals` | iter | `src/events/deal_accepted.rs::acceptable_deals` |
| `Events/DealAccepted.cs:86:DealAccepted.CancellableDeals` | iter | `src/events/deal_accepted.rs::cancellable_deals` |
| `Events/DiscardedTaken.cs:49:DiscardedTaken.ValidCards` | iter | `src/events/discarded_taken.rs::valid_cards` |
| `Events/DiscoveryEntered.cs:36:DiscoveryEntered.ValidTargets` | iter | `src/events/discovery_entered.rs::valid_targets` |
| `Events/DiscoveryEntered.cs:41:DiscoveryEntered.ValidSources` | iter | `src/events/discovery_entered.rs::valid_sources` |
| `Events/DiscoveryRevealed.cs:54:DiscoveryRevealed.GetLocations` | iter | `src/events/discovery_revealed.rs::get_locations` |
| `Events/DistransUsed.cs:52:DistransUsed.ValidCards` | iter | `src/events/distrans_used.rs::valid_cards` |
| `Events/DistransUsed.cs:57:DistransUsed.ValidTargets` | iter | `src/events/distrans_used.rs::valid_targets` |
| `Events/Donated.cs:64:Donated.ValidTargets` | iter | `src/events/donated.rs::valid_targets` |
| `Events/FaceDanced.cs:93:FaceDanced.ValidSourceLocations` | iter | `src/events/face_danced.rs::valid_source_locations` |
| `Events/FaceDanced.cs:98:FaceDanced.ValidTargetLocations` | iter | `src/events/face_danced.rs::valid_target_locations` |
| `Events/FaceDancerReplaced.cs:68:FaceDancerReplaced.ValidFaceDancers` | iter | `src/events/face_dancer_replaced.rs::valid_face_dancers` |
| `Events/FactionSelected.cs:45:FactionSelected.ValidFactions` | iter | `src/events/faction_selected.rs::valid_factions` |
| `Events/FactionTradeOffered.cs:49:FactionTradeOffered.ValidTargets` | iter | `src/events/faction_trade_offered.rs::valid_targets` |
| `Events/Karma.cs:231:Karma.ValidKarmaCards` | iter | `src/events/karma.rs::valid_karma_cards` |
| `Events/KarmaHmsMovement.cs:51:KarmaHmsMovement.ValidLocations` | iter | `src/events/karma_hms_movement.rs::valid_locations` |
| `Events/KarmaMonster.cs:47:KarmaMonster.ValidTargets` | iter | `src/events/karma_monster.rs::valid_targets` |
| `Events/KarmaRevivalPrevention.cs:41:KarmaRevivalPrevention.GetValidTargets` | iter | `src/events/karma_revival_prevention.rs::get_valid_targets` |
| `Events/KarmaShipmentPrevention.cs:41:KarmaShipmentPrevention.GetValidTargets` | iter | `src/events/karma_shipment_prevention.rs::get_valid_targets` |
| `Events/NexusPlayed.cs:270:NexusPlayed.ValidChoamCards` | iter | `src/events/nexus_played.rs::valid_choam_cards` |
| `Events/NexusPlayed.cs:275:NexusPlayed.ValidEcazTerritories` | iter | `src/events/nexus_played.rs::valid_ecaz_territories` |
| `Events/NexusPlayed.cs:284:NexusPlayed.ValidEcazFactions` | iter | `src/events/nexus_played.rs::valid_ecaz_factions` |
| `Events/NexusPlayed.cs:289:NexusPlayed.ValidMoritaniTerritories` | iter | `src/events/nexus_played.rs::valid_moritani_territories` |
| `Events/PerformBeneGesseritPlacement.cs:54:PerformBeneGesseritPlacement.ValidLocations` | iter | `src/events/perform_bene_gesserit_placement.rs::valid_locations` |
| `Events/PerformMoritaniSetup.cs:49:PerformMoritaniSetup.ValidLocations` | iter | `src/events/perform_moritani_setup.rs::valid_locations` |
| `Events/PerformHmsMovement.cs:50:PerformHmsMovement.ValidLocations` | iter | `src/events/perform_hms_movement.rs::valid_locations` |
| `Events/PerformHmsPlacement.cs:49:PerformHmsPlacement.ValidLocations` | iter | `src/events/perform_hms_placement.rs::valid_locations` |
| `Events/PlayerReplaced.cs:39:PlayerReplaced.ValidTargets` | iter | `src/events/player_replaced.rs::valid_targets` |
| `Events/RequestTleilaxuRevival.cs:50:RequestTleilaxuRevival.ValidTargets` | iter | `src/events/request_tleilaxu_revival.rs::valid_targets` |
| `Events/ResourcesAudited.cs:40:ResourcesAudited.ValidFactions` | iter | `src/events/resources_audited.rs::valid_factions` |
| `Events/Retreat.cs:56:Retreat.ValidTargets` | iter | `src/events/retreat.rs::valid_targets` |
| `Events/Revival.cs:204:Revival.ValidRevivalHeroes` | iter | `src/events/revival.rs::valid_revival_heroes` |
| `Events/Revival.cs:449:Revival.ValidRevivedForceLocations` | iter | `src/events/revival.rs::valid_revived_force_locations` |
| `Events/SetIncreasedRevivalLimits.cs:41:SetIncreasedRevivalLimits.ValidTargets` | iter | `src/events/set_increased_revival_limits.rs::valid_targets` |
| `Events/SetShipmentPermission.cs:48:SetShipmentPermission.ValidTargets` | iter | `src/events/set_shipment_permission.rs::valid_targets` |
| `Events/Shipment.cs:257:Shipment.HomeworldsToShipFrom` | vec | `src/events/shipment.rs::homeworlds_to_ship_from`. Only `Count()` callers. |
| `Events/Shipment.cs:334:Shipment.ValidShipmentLocations` | iter | `src/events/shipment.rs::valid_shipment_locations` |
| `Events/Shipment.cs:343:Shipment.RegularShipmentLocations` | iter | `src/events/shipment.rs::regular_shipment_locations` |
| `Events/Shipment.cs:402:Shipment.ValidOwnKarmaCards` | vec | `src/events/shipment.rs::valid_own_karma_cards`. Only caller is `ValidKarmaCards`, which calls `ToList()` right away. |
| `Events/Shipment.cs:446:Shipment.ValidSourceLocations` | iter | `src/events/shipment.rs::valid_source_locations` |
| `Events/SkillAssigned.cs:58:SkillAssigned.ValidLeaders` | vec | `src/events/skill_assigned.rs::valid_leaders`. No caller in Shared. Returns a one-element array on one branch; the lazy branch reads only the player's leader list. |
| `Events/TerrorPlanted.cs:61:TerrorPlanted.ValidStrongholds` | iter | `src/events/terror_planted.rs::valid_strongholds` |
| `Events/TerrorPlanted.cs:83:TerrorPlanted.ValidTerrorTypes` | iter | `src/events/terror_planted.rs::valid_terror_types` |
| `Events/TerrorRevealed.cs:111:TerrorRevealed.GetTypes` | iter | `src/events/terror_revealed.rs::get_types` |
| `Events/TerrorRevealed.cs:121:TerrorRevealed.ValidSneakAttackTargets` | iter | `src/events/terror_revealed.rs::valid_sneak_attack_targets` |
| `Events/Thought.cs:49:Thought.ValidCards` | iter | `src/events/thought.rs::valid_cards` |
| `Events/RicheseGaveCard.cs:47:RicheseGaveCard.ValidCards` | iter | `src/events/richese_gave_card.rs::valid_cards` |
| `Events/FremenRidesMonster.cs:111:FremenRidesMonster.ValidTargets` | iter | `src/events/fremen_rides_monster.rs::valid_targets` |
| `Events/FremenRidesMonster.cs:75:FremenRidesMonster.LocationsWithForcesThatCanRide` | iter | `src/events/fremen_rides_monster.rs::locations_with_forces_that_can_ride` |
| `Events/FremenRidesMonster.cs:91:FremenRidesMonster.ValidSources` | iter | `src/events/fremen_rides_monster.rs::valid_sources` |
| `Events/FremenSentMonster.cs:50:FremenSentMonster.ValidTargets` | iter | `src/events/fremen_sent_monster.rs::valid_targets` |
| `ExtensionMethods.cs:150:ExtensionMethods.TakeRandomN` | iter | `src/extension_methods.rs::take_random_n` |
| `Game.cs:998:Game.LocationsWithAnyForcesNotInStorm` | iter | `src/game.rs::locations_with_any_forces_not_in_storm` |
| `Game_Bidding.cs:605:Game.PlayersThatCanBid` | vec | `src/game_bidding.rs::players_that_can_bid`. Only `Count()` callers. |
| `Game_Contemplate.cs:300:Game.Strongholds` | iter | `src/game_contemplate.rs::strongholds` |
| `Game_Contemplate.cs:434:Game.TerrorIn` | iter | `src/game_contemplate.rs::terror_in` |
| `Game_Deals.cs:41:Game.DealRecipients` | iter | `src/game_deals.rs::deal_recipients` |
| `Game_LeaderState.cs:110:Game.GetSkilledLeaders` | iter | `src/game_leader_state.rs::get_skilled_leaders` |
| `Game_Participation.cs:30:Game.PlayersOtherThan` | iter | `src/game_participation.rs::players_other_than` |
| `Game_Participation.cs:75:Game.PlayerNames` | vec | `src/game_participation.rs::player_names`. No caller in Shared (server and UI). |
| `Game_Participation.cs:80:Game.ObserverNames` | vec | `src/game_participation.rs::observer_names`. No caller in Shared (server and UI). |
| `Game_Rules.cs:16:Game.GetCustomRules` | vec | `src/game_rules.rs::get_custom_rules`. Only caller calls `ToList()`. |
| `Game_Rules.cs:305:Game.GetRulesInGroup` | vec | `src/game_rules.rs::get_rules_in_group`. Only caller is `List.AddRange`, which enumerates it fully. |
| `Game_Rules.cs:311:Game.GetAvailableRulesets` | vec | `src/game_rules.rs::get_available_rulesets`. No caller in Shared. Static data: the enum values. |
| `LeaderManager.cs:105:LeaderManager.GetLeaders` | iter | `src/leader_manager.rs::get_leaders` |
| `Map.cs:106:Map.Locations` | iter | `src/map.rs::locations` |
| `Map.cs:111:Map.Homeworlds` | iter | `src/map.rs::homeworlds` |
| `Map.cs:113:Map.Territories` | iter | `src/map.rs::territories` |
| `Map.cs:118:Map.Strongholds` | iter | `src/map.rs::strongholds` |
| `Map.cs:145:Map.GetResourceCardsInPlay` | iter | `src/map.rs::get_resource_cards_in_play` |
| `Model/ObjectCounter.cs:55:ObjectCounter.GetHighest` | vec | `src/object_counter.rs::get_highest`. No caller in Shared. `OrderByDescending` sorts the whole dictionary on the first pull anyway. |
| `Model/Player.cs:471:Player.OccupiedLocations` | vec | `src/player.rs::occupied_locations`. Only caller is the `foreach` in `Battle.BattlesToBeFought`, a pure function that only builds its result list. |
| `Model/Player.cs:473:Player.OccupiedTerritories` | vec | `src/player.rs::occupied_territories`. Only callers are `Intersect` (which reads all of its second argument) and `ToList()` in `Game_Move`. |
| `Model/Player.cs:489:Player.TerritoriesWithForces` | iter | `src/player.rs::territories_with_forces` |
| `Model/Player.cs:493:Player.LocationsWithAnyForcesInTerritory` | iter | `src/player.rs::locations_with_any_forces_in_territory` |
| `Model/Player.cs:634:Player.UnrevealedTraitors` | iter | `src/player.rs::unrevealed_traitors` |
| `Model/Player.cs:636:Player.UnrevealedFaceDancers` | iter | `src/player.rs::unrevealed_face_dancers` |
| `Model/PlayerSequence.cs:199:PlayerSequence.GetPlayersInSequence` | iter | `src/player_sequence.rs::get_players_in_sequence` |
| `TreacheryCardManager.cs:117:TreacheryCardManager.GetCardsInPlay` | iter | `src/treachery_card_manager.rs::get_cards_in_play` |
| `TreacheryCardManager.cs:127:TreacheryCardManager.GetCardsInPlay` | iter | `src/treachery_card_manager.rs::get_cards_in_play_by_rule` |

## Sites

| Site | Class | Why |
|---|---|---|
| `Events/AllianceByTerror.cs:62:Game.TerrorIn` | full | Enumerated once, completely, with nothing in between. |
| `Events/AllianceOffered.cs:36:AllianceOffered.ValidTargets` | short | `Contains` stops at the first match. |
| `Events/AmbassadorActivated.cs:137:AmbassadorActivated.ValidFremenTargets` | short | `Contains` stops at the first match. |
| `Events/AmbassadorActivated.cs:151:AmbassadorActivated.ValidGuildTargets` | short | `Contains` stops at the first match. |
| `Events/AmbassadorActivated.cs:233:Map.Locations` | forward | Returned by `AmbassadorActivated.ValidFremenTargets`; see its producer row. |
| `Events/AmbassadorActivated.cs:241:Map.Locations` | forward | Returned by `AmbassadorActivated.ValidGuildTargets`; see its producer row. |
| `Events/AmbassadorPlaced.cs:46:AmbassadorPlaced.ValidStrongholds` | short | `Contains` stops at the first match. |
| `Events/AmbassadorPlaced.cs:54:Map.Territories` | forward | Returned by `AmbassadorPlaced.ValidStrongholds`; see its producer row. |
| `Events/AmbassadorPlaced.cs:71:AmbassadorPlaced.ValidStrongholds` | short | `Any` stops at the first match. |
| `Events/AuditCancelled.cs:54:AuditCancelled.GetCardsThatMayBeAudited` | full | Enumerated once, completely, with nothing in between. |
| `Events/AutomationConfigured.cs:101:Enumerations.GetValuesExceptDefault` | full | Enumerated once, completely, with nothing in between. |
| `Events/Battle.cs:627:Player.OccupiedLocations` | foreach | Same loop as the row above. |
| `Events/Battle.cs:627:foreach occupiedLocation` | foreach | `Battle.BattlesToBeFought` only builds a local result list. The predicates read static map data and the player's forces, which the loop doesn't change, and can't throw. |
| `Events/Battle.cs:631:foreach battalions` | foreach | Same pure function; `OccupyingForcesOnPlanet` is a fresh dictionary. |
| `Events/Battle.cs:633:local defenders` | foreach | Same pure function; the `break` only stops adding to the local result. Predicate reads battalion factions. |
| `Events/Battle.cs:635:foreach faction` | foreach | Same pure function as above. |
| `Events/Battle.cs:666:Battle.CheapHeroes` | short | `Any` stops at the first match. |
| `Events/Battle.cs:668:Battle.CheapHeroes` | full | Enumerated once, completely, with nothing in between. |
| `Events/Battle.cs:673:Battle.CheapHeroes` | full | Enumerated once, completely, with nothing in between. |
| `Events/Battle.cs:737:Battle.CardsPlayableAsDefense` | full | Enumerated once, completely, with nothing in between. |
| `Events/Battle.cs:967:Map.Strongholds` | full | Enumerated once, completely, with nothing in between. |
| `Events/BattleRevision.cs:43:Battle.AspectsSeenByOpponent` | foreach | Full enumeration; see the producer row. |
| `Events/Bid.cs:111:PlayerSequence.GetPlayersInSequence` | forward | Held in a local and returned from one switch arm only. Rust keeps it lazy (`Either`) and collects it only in that arm, so the per-element `CurrentPlayer` runs only when C# runs it. |
| `Events/Bid.cs:124:Karma.ValidKarmaCards` | forward | Returned by `Bid.ValidKarmaCards`; see its producer row. |
| `Events/Bid.cs:130:Bid.ValidKarmaCards` | short | `Any` stops at the first match. |
| `Events/Bid.cs:199:Game.PlayersThatCanBid` | full | Enumerated once, completely, with nothing in between. |
| `Events/Bid.cs:87:Karma.ValidKarmaCards` | short | `Contains` stops at the first match. |
| `Events/BlackMarketBid.cs:242:PlayerSequence.GetPlayersInSequence` | forward | As in `Bid.PlayersToBid`. |
| `Events/BlackMarketBid.cs:98:Game.PlayersThatCanBid` | full | Enumerated once, completely, with nothing in between. |
| `Events/BeneGesseritBattleAnnouncement.cs:55:Map.Territories` | forward | Returned by `BeneGesseritBattleAnnouncement.ValidTerritories`; see its producer row. |
| `Events/BeneGesseritPrediction.cs:46:Game.PlayersOtherThan` | forward | Returned by `BeneGesseritPrediction.ValidTargets`; see its producer row. |
| `Events/ChoamDiscarded.cs:44:ChoamDiscarded.ValidCards` | short | `Contains` stops at the first match. |
| `Events/ChoamMovePrevention.cs:44:ChoamMovePrevention.ValidTerritories` | short | `Contains` stops at the first match. |
| `Events/ChoamMovePrevention.cs:50:Player.TerritoriesWithForces` | forward | Returned by `ChoamMovePrevention.ValidTerritories`; see its producer row. |
| `Events/ChoamMovePrevention.cs:55:ChoamMovePrevention.ValidTerritories` | short | `Any` stops at the first match. |
| `Events/ChoamRemoveForce.cs:48:ChoamRemoveForce.ValidLocations` | short | `Contains` stops at the first match. |
| `Events/ChoamRemoveForce.cs:49:ChoamRemoveForce.ValidFactions` | short | `Contains` stops at the first match. |
| `Events/ChoamRemoveForce.cs:64:Game.PlayersOtherThan` | forward | Returned by `ChoamRemoveForce.ValidFactions`; see its producer row. |
| `Events/CardTraded.cs:91:foreach p` | foreach | The predicate reads fields of `Game.Players`, which in Rust is a `Vec<Player>` that can't hold null, and the body doesn't change what it reads (`UnregisterKnown` only). |
| `Events/ClairVoyancePlayed.cs:124:Game.PlayersOtherThan` | forward | Returned by `ClairVoyancePlayed.ValidTargets`; see its producer row. |
| `Events/ClairVoyancePlayed.cs:221:Karma.ValidKarmaCards` | short | `FirstOrDefault` stops at the first match. |
| `Events/ClairVoyancePlayed.cs:227:Karma.ValidKarmaCards` | short | `FirstOrDefault` stops at the first match. |
| `Events/DealRejected.cs:57:DealAccepted.AcceptableDeals` | short | `FirstOrDefault` stops at the first match. |
| `Events/DiscardedTaken.cs:44:DiscardedTaken.ValidCards` | short | `Contains` stops at the first match. |
| `Events/DiscardedTaken.cs:56:DiscardedTaken.ValidCards` | short | `Any` stops at the first match. |
| `Events/DiscoveryEntered.cs:31:DiscoveryEntered.ValidTargets` | short | `Contains` stops at the first match. |
| `Events/DiscoveryEntered.cs:60:foreach fromTerritory` | foreach | Keys of the event's own `ForceLocations`, which the loop doesn't change. Static territory lookup can't throw. |
| `Events/DiscoveryEntered.cs:65:foreach fl` | foreach | Filters the event's `ForceLocations` by territory; the body moves the player's forces, not the event's. |
| `Events/DiscoveryEntered.cs:70:local fromLocations` | full | Passed through `LogMove` to `Message.WithRoute`, which calls `ToList()` at once. |
| `Events/DiscoveryRevealed.cs:44:DiscoveryRevealed.GetLocations` | short | `Contains` stops at the first match. |
| `Events/DiscoveryRevealed.cs:51:DiscoveryRevealed.GetLocations` | short | `Any` stops at the first match. |
| `Events/DistransUsed.cs:46:DistransUsed.ValidCards` | short | `Contains` stops at the first match. |
| `Events/DistransUsed.cs:47:DistransUsed.ValidTargets` | short | `Contains` stops at the first match. |
| `Events/DistransUsed.cs:64:DistransUsed.ValidCards` | short | `Any` stops at the first match. |
| `Events/DistransUsed.cs:64:DistransUsed.ValidTargets` | short | `Any` stops at the first match. |
| `Events/DistransUsed.cs:84:foreach p` | foreach | The predicate reads fields of `Game.Players`, which in Rust is a `Vec<Player>` that can't hold null, and the body doesn't change what it reads (`UnregisterKnown` only). |
| `Events/Donated.cs:106:foreach p` | foreach | The predicate reads fields of `Game.Players`, which in Rust is a `Vec<Player>` that can't hold null, and the body doesn't change what it reads (`UnregisterKnown` only). |
| `Events/EndPhase.cs:178:foreach p` | foreach | Rust loops by index and evaluates the predicate right before each body (hazards 4.1). |
| `Events/EndPhase.cs:253:foreach p` | foreach | Rust loops by index and evaluates the predicate right before each body (hazards 4.1). |
| `Events/EstablishPlayers.cs:216:Game.GetRulesInGroup` | full | Enumerated once, completely, with nothing in between. |
| `Events/EstablishPlayers.cs:221:Game.GetCustomRules` | full | Enumerated once, completely, with nothing in between. |
| `Events/EstablishPlayers.cs:301:foreach f` | foreach | The predicate reads fields of `Game.Players`, which in Rust is a `Vec<Player>` that can't hold null, and the body doesn't change what it reads (it removes from `AllowedFactionsInPlay`). |
| `Events/EstablishPlayers.cs:308:Map.GetResourceCardsInPlay` | foreach | `IsInPlay` reads rules and static card data only; the body puts cards in a new deck. |
| `Events/FaceDanced.cs:126:Player.UnrevealedFaceDancers` | short | `Any` stops at the first match. |
| `Events/FaceDanced.cs:95:Game.LocationsWithAnyForcesNotInStorm` | forward | Returned by `FaceDanced.ValidSourceLocations`; see its producer row. |
| `Events/FaceDancerReplaced.cs:70:Player.UnrevealedFaceDancers` | forward | Returned by `FaceDancerReplaced.ValidFaceDancers`; see its producer row. |
| `Events/FactionSelected.cs:40:FactionSelected.ValidFactions` | short | `Contains` stops at the first match. |
| `Events/FactionTradeOffered.cs:38:FactionTradeOffered.ValidTargets` | short | `Contains` stops at the first match. |
| `Events/IxianSwappedCardOnBid.cs:59:foreach p` | foreach | The predicate reads fields of `Game.Players`, which in Rust is a `Vec<Player>` that can't hold null, and the body doesn't change what it reads (`UnregisterKnown` only). |
| `Events/Karma.cs:241:Karma.ValidKarmaCards` | short | `Any` stops at the first match. |
| `Events/KarmaChoamDiscard.cs:53:Karma.ValidKarmaCards` | short | `FirstOrDefault` stops at the first match. |
| `Events/KarmaChoamDiscard.cs:68:Karma.ValidKarmaCards` | short | `FirstOrDefault` stops at the first match. |
| `Events/KarmaFreeRevival.cs:77:Karma.ValidKarmaCards` | short | `FirstOrDefault` stops at the first match. |
| `Events/KarmaHandSwap.cs:57:foreach p` | foreach | The predicate reads fields of `Game.Players`, which in Rust is a `Vec<Player>` that can't hold null, and the body doesn't change what it reads (`UnregisterKnown` only). |
| `Events/KarmaHandSwapInitiated.cs:49:Karma.ValidKarmaCards` | short | `FirstOrDefault` stops at the first match. |
| `Events/KarmaHmsMovement.cs:46:KarmaHmsMovement.ValidLocations` | short | `Contains` stops at the first match. |
| `Events/KarmaHmsMovement.cs:66:Karma.ValidKarmaCards` | short | `FirstOrDefault` stops at the first match. |
| `Events/KarmaHmsMovement.cs:68:Karma.ValidKarmaCards` | short | `FirstOrDefault` stops at the first match. |
| `Events/KarmaMonster.cs:49:Map.Territories` | forward | Returned by `KarmaMonster.ValidTargets`; see its producer row. |
| `Events/KarmaMonster.cs:58:Karma.ValidKarmaCards` | short | `FirstOrDefault` stops at the first match. |
| `Events/KarmaEcazDial.cs:41:Karma.ValidKarmaCards` | short | `FirstOrDefault` stops at the first match. |
| `Events/KarmaPrescience.cs:39:Karma.ValidKarmaCards` | short | `FirstOrDefault` stops at the first match. |
| `Events/KarmaRevivalPrevention.cs:36:KarmaRevivalPrevention.GetValidTargets` | short | `Contains` stops at the first match. |
| `Events/KarmaRevivalPrevention.cs:43:Game.PlayersOtherThan` | forward | Returned by `KarmaRevivalPrevention.GetValidTargets`; see its producer row. |
| `Events/KarmaRevivalPrevention.cs:53:Karma.ValidKarmaCards` | short | `FirstOrDefault` stops at the first match. |
| `Events/KarmaShipmentPrevention.cs:36:KarmaShipmentPrevention.GetValidTargets` | short | `Contains` stops at the first match. |
| `Events/KarmaShipmentPrevention.cs:43:Game.PlayersOtherThan` | forward | Returned by `KarmaShipmentPrevention.GetValidTargets`; see its producer row. |
| `Events/KarmaShipmentPrevention.cs:53:Karma.ValidKarmaCards` | short | `FirstOrDefault` stops at the first match. |
| `Events/KarmaRicheseBuy.cs:56:Karma.ValidKarmaCards` | short | `FirstOrDefault` stops at the first match. |
| `Events/LoserConcluded.cs:147:foreach c` | foreach | `c != KeptCard` compares references and can't throw, even for a null card. `Discard` doesn't touch `CardsToBeDiscardedByLoserAfterBattle`. |
| `Events/MetheorPlayed.cs:75:foreach location` | foreach | The body kills forces in the current location only; the predicate for later locations reads their own forces. |
| `Events/MulliganPerformed.cs:67:foreach p` | foreach | The predicate reads fields of `Game.Players`, which in Rust is a `Vec<Player>` that can't hold null, and the body doesn't change what it reads (adds traitors). |
| `Events/NexusPlayed.cs:136:Revival.ValidRevivedForceLocations` | short | `Contains` stops at the first match. |
| `Events/NexusPlayed.cs:147:NexusPlayed.ValidChoamCards` | short | `Contains` stops at the first match. |
| `Events/NexusPlayed.cs:155:NexusPlayed.ValidEcazTerritories` | short | `Contains` stops at the first match. |
| `Events/NexusPlayed.cs:159:NexusPlayed.ValidEcazFactions` | short | `Contains` stops at the first match. |
| `Events/NexusPlayed.cs:164:NexusPlayed.ValidMoritaniTerritories` | short | `Contains` stops at the first match. |
| `Events/NexusPlayed.cs:229:NexusPlayed.ValidChoamCards` | short | `Any` stops at the first match. |
| `Events/NexusPlayed.cs:239:NexusPlayed.ValidEcazTerritories` | short | `Any` stops at the first match. |
| `Events/NexusPlayed.cs:242:NexusPlayed.ValidMoritaniTerritories` | short | `Any` stops at the first match. |
| `Events/NexusPlayed.cs:279:Map.Territories` | forward | Returned by `NexusPlayed.ValidEcazTerritories`; see its producer row. |
| `Events/NexusPlayed.cs:297:Revival.ValidRevivedForceLocations` | short | `Any` stops at the first match. |
| `Events/NexusPlayed.cs:472:Game.TerrorIn` | full | `RandomOrDefault` calls `ToArray()` first. |
| `Events/PerformBeneGesseritPlacement.cs:44:PerformBeneGesseritPlacement.ValidLocations` | short | `Contains` stops at the first match. |
| `Events/PerformBeneGesseritPlacement.cs:58:Map.Locations` | forward | Returned by `PerformBeneGesseritPlacement.ValidLocations`; see its producer row. |
| `Events/PerformMoritaniSetup.cs:44:PerformMoritaniSetup.ValidLocations` | short | `Contains` stops at the first match. |
| `Events/PerformMoritaniSetup.cs:51:Map.Locations` | forward | Returned by `PerformMoritaniSetup.ValidLocations`; see its producer row. |
| `Events/PerformHmsMovement.cs:45:PerformHmsMovement.ValidLocations` | short | `Contains` stops at the first match. |
| `Events/PerformHmsPlacement.cs:44:PerformHmsPlacement.ValidLocations` | short | `Contains` stops at the first match. |
| `Events/PerformHmsPlacement.cs:52:Map.Locations` | forward | Returned by `PerformHmsPlacement.ValidLocations`; see its producer row. |
| `Events/PerformHmsPlacement.cs:53:Map.Locations` | forward | Returned by `PerformHmsPlacement.ValidLocations`; see its producer row. |
| `Events/PlacementEvent.cs:155:foreach fl` | foreach | The body only intersects location lists; the predicate reads the given battalions. |
| `Events/PlacementEvent.cs:200:Map.Territories` | full | Enumerated once, completely, with nothing in between. |
| `Events/RaiseDeadPlayed.cs:105:Revival.ValidRevivedForceLocations` | short | `Any` stops at the first match. |
| `Events/RaiseDeadPlayed.cs:89:Revival.ValidRevivedForceLocations` | short | `Contains` stops at the first match. |
| `Events/RequestTleilaxuRevival.cs:45:RequestTleilaxuRevival.ValidTargets` | short | `Contains` stops at the first match. |
| `Events/ResourcesAudited.cs:35:ResourcesAudited.ValidFactions` | short | `Contains` stops at the first match. |
| `Events/Retreat.cs:49:Retreat.ValidTargets` | short | `Contains` stops at the first match. |
| `Events/Revival.cs:118:Revival.ValidRevivedForceLocations` | short | `Contains` stops at the first match. |
| `Events/Revival.cs:451:Map.Locations` | forward | Returned by `Revival.ValidRevivedForceLocations`; see its producer row. |
| `Events/SetIncreasedRevivalLimits.cs:36:SetIncreasedRevivalLimits.ValidTargets` | short | `Contains` stops at the first match. |
| `Events/SetShipmentPermission.cs:39:SetShipmentPermission.ValidTargets` | short | `Contains` stops at the first match. |
| `Events/Shipment.cs:190:Shipment.ValidSourceLocations` | short | `Contains` stops at the first match. |
| `Events/Shipment.cs:194:Shipment.ValidSourceLocations` | short | `Contains` stops at the first match. |
| `Events/Shipment.cs:203:Shipment.ValidShipmentLocations` | short | `Contains` stops at the first match. |
| `Events/Shipment.cs:243:Shipment.HomeworldsToShipFrom` | full | Enumerated once, completely, with nothing in between. |
| `Events/Shipment.cs:249:Shipment.HomeworldsToShipFrom` | full | Enumerated once, completely, with nothing in between. |
| `Events/Shipment.cs:264:Map.Locations` | full | Enumerated once, completely, with nothing in between. |
| `Events/Shipment.cs:336:Shipment.RegularShipmentLocations` | forward | Returned by `Shipment.ValidShipmentLocations`; see its producer row. |
| `Events/Shipment.cs:339:Shipment.RegularShipmentLocations` | forward | Returned by `Shipment.ValidShipmentLocations`; see its producer row. |
| `Events/Shipment.cs:345:Map.Locations` | forward | Returned by `Shipment.RegularShipmentLocations`; see its producer row. |
| `Events/Shipment.cs:394:Shipment.ValidOwnKarmaCards` | full | Enumerated once, completely, with nothing in between. |
| `Events/Shipment.cs:403:Karma.ValidKarmaCards` | forward | Returned by `Shipment.ValidOwnKarmaCards`; see its producer row. |
| `Events/Shipment.cs:448:Game.LocationsWithAnyForcesNotInStorm` | forward | Returned by `Shipment.ValidSourceLocations`; see its producer row. |
| `Events/Shipment.cs:615:Shipment.HomeworldsToShipFrom` | full | Enumerated once, completely, with nothing in between. |
| `Events/Shipment.cs:617:foreach fl` | foreach | Filters the event's `ForceLocations`; the body ships the player's forces. |
| `Events/Shipment.cs:628:Shipment.HomeworldsToShipFrom` | full | Enumerated once, completely, with nothing in between. |
| `Events/Shipment.cs:630:foreach fl` | foreach | Filters the event's `ForceLocations`; the body ships the player's forces. |
| `Events/TerrorPlanted.cs:124:Game.TerrorIn` | short | `Any` stops at the first match. |
| `Events/TerrorPlanted.cs:54:TerrorPlanted.ValidStrongholds` | short | `Contains` stops at the first match. |
| `Events/TerrorPlanted.cs:55:TerrorPlanted.ValidTerrorTypes` | short | `Contains` stops at the first match. |
| `Events/TerrorPlanted.cs:66:Map.Territories` | forward | Returned by `TerrorPlanted.ValidStrongholds`; see its producer row. |
| `Events/TerrorPlanted.cs:70:Game.TerrorIn` | short | `Any` stops at the first match. |
| `Events/TerrorPlanted.cs:96:TerrorPlanted.ValidTerrorTypes` | short | `Any` stops at the first match. |
| `Events/TerrorPlanted.cs:97:TerrorPlanted.ValidStrongholds` | short | `Any` stops at the first match. |
| `Events/TerrorRevealed.cs:113:Game.TerrorIn` | forward | Returned by `TerrorRevealed.GetTypes`; see its producer row. |
| `Events/TerrorRevealed.cs:167:Game.TerrorIn` | short | `Any` stops at the first match. |
| `Events/TerrorRevealed.cs:78:TerrorRevealed.GetTypes` | short | `Contains` stops at the first match. |
| `Events/TerrorRevealed.cs:82:TerrorRevealed.ValidSneakAttackTargets` | short | `Contains` stops at the first match. |
| `Events/Thought.cs:44:Thought.ValidCards` | short | `Contains` stops at the first match. |
| `Events/Thought.cs:51:TreacheryCardManager.GetCardsInPlay` | forward | Returned by `Thought.ValidCards`; see its producer row. |
| `Events/RicheseGaveCard.cs:64:foreach p` | foreach | The predicate reads fields of `Game.Players`, which in Rust is a `Vec<Player>` that can't hold null, and the body doesn't change what it reads (`UnregisterKnown` only). |
| `Events/FremenRidesMonster.cs:115:Map.Locations` | forward | Returned by `FremenRidesMonster.ValidTargets`; see its producer row. |
| `Events/FremenRidesMonster.cs:45:FremenRidesMonster.ValidTargets` | short | `Contains` stops at the first match. |
| `Events/FremenRidesMonster.cs:70:FremenRidesMonster.LocationsWithForcesThatCanRide` | short | `Any` stops at the first match. |
| `Events/FremenRidesMonster.cs:97:FremenRidesMonster.LocationsWithForcesThatCanRide` | forward | Returned by `FremenRidesMonster.ValidSources`; see its producer row. |
| `Events/FremenSentMonster.cs:52:Map.Territories` | forward | Returned by `FremenSentMonster.ValidTargets`; see its producer row. |
| `ExtensionMethods.cs:26:foreach item` | foreach | `OrderBy` computes every key on the first pull, so the `break` doesn't skip any selector call. Rust sorts with cached keys. |
| `ExtensionMethods.cs:44:foreach item` | foreach | `OrderBy` computes every key on the first pull, so the `break` doesn't skip any selector call. Rust sorts with cached keys. |
| `Game.cs:1000:Map.Locations` | forward | Returned by `Game.LocationsWithAnyForcesNotInStorm`; see its producer row. |
| `Game.cs:1103:local factionsInPlay` | double | `Players.Select(p => p.Faction)` held lazily and re-run by `Contains` for each leader. Rust re-runs the iterator (`factions_in_play.clone().any(..)`) in the same order. |
| `Game.cs:1110:TreacheryCardManager.GetCardsInPlay` | short | `First` stops at the first match. |
| `Game.cs:468:Map.Locations` | foreach | Static data; the body fills a new dictionary. |
| `Game.cs:527:foreach locationAndBattalion` | foreach | The body adds to a new dictionary; the predicate reads the player's forces. |
| `Game.cs:594:Map.Territories` | foreach | Flipping advisors in one territory doesn't change the predicate for another, which reads that territory's forces only. |
| `Game.cs:594:local territoriesWhereAdvisorsAreInSinkOrAloneOrWithEcaz` | foreach | As the row above. |
| `Game.cs:608:Enumerations.GetValuesExceptDefault` | foreach | Static enum values. |
| `Game.cs:617:Map.Homeworlds` | foreach | Static data; the body logs and fills a new dictionary. |
| `Game.cs:957:foreach faction` | foreach | `Distinct` over a parameter list; the body only counts. |
| `Game_Battle.cs:1147:foreach leader` | foreach | The body changes `InFrontOfShield` of the current leader only; the predicate of later leaders reads their own state. |
| `Game_Battle.cs:341:BattleConcluded.ValidTraitorsToReplace` | short | `Any` stops at the first match. |
| `Game_Battle.cs:426:AuditCancelled.GetCardsThatMayBeAudited` | full | The `Deck` constructor copies it with `new List(items)`. |
| `Game_Battle.cs:565:Retreat.ValidTargets` | short | `Any` stops at the first match. |
| `Game_Bidding.cs:246:foreach p` | foreach | The predicate reads fields of `Game.Players`, which in Rust is a `Vec<Player>` that can't hold null, and the body doesn't change what it reads (`RegisterKnown` only). |
| `Game_Bidding.cs:78:Game.PlayersThatCanBid` | full | Enumerated once, completely, with nothing in between. |
| `Game_Blow.cs:164:foreach i` | foreach | Rust loops by index and tests each card right before adding it, so a card that throws leaves the earlier ones added, as in C#. |
| `Game_Blow.cs:169:foreach i` | foreach | Rust loops by index and tests each card right before adding it, so a card that throws leaves the earlier ones added, as in C#. |
| `Game_Collection.cs:106:Map.Homeworlds` | foreach | Static homeworld data. |
| `Game_Collection.cs:106:foreach homeworld` | foreach | Static homeworld data (`ResourceAmount`). |
| `Game_Contemplate.cs:300:Map.Territories` | forward | Returned by `Game.Strongholds`; see its producer row. |
| `Game_Contemplate.cs:305:Game.Strongholds` | full | Enumerated once, completely, with nothing in between. |
| `Game_Contemplate.cs:314:Game.Strongholds` | short | `Any` stops at the first match. |
| `Game_Contemplate.cs:325:Map.Territories` | full | Enumerated once, completely, with nothing in between. |
| `Game_Contemplate.cs:339:Map.Territories` | full | Enumerated once, completely, with nothing in between. |
| `Game_Events.cs:143:DiscoveryEntered.ValidTargets` | short | `Any` stops at the first match. |
| `Game_Events.cs:498:ResourcesAudited.ValidFactions` | short | `Any` stops at the first match. |
| `Game_Events.cs:534:ChoamDiscarded.ValidCards` | short | `Any` stops at the first match. |
| `Game_Events.cs:641:BeneGesseritBattleAnnouncement.ValidTerritories` | short | `Any` stops at the first match. |
| `Game_Events.cs:647:Donated.ValidTargets` | short | `Any` stops at the first match. |
| `Game_Events.cs:659:RicheseGaveCard.ValidCards` | short | `Any` stops at the first match. |
| `Game_LeaderState.cs:117:Game.GetSkilledLeaders` | short | `FirstOrDefault` stops at the first match. |
| `Game_LeaderState.cs:152:Revival.ValidRevivalHeroes` | short | `Any` stops at the first match. |
| `Game_Move.cs:362:Game.TerrorIn` | short | `Any` stops at the first match. |
| `Game_Move.cs:696:Player.OccupiedTerritories` | full | `Intersect` then `ToList()`: full. |
| `Game_Move.cs:697:Player.TerritoriesWithForces` | full | `Intersect` then `ToList()`: full. Rust collects the iterator. |
| `Game_Move.cs:699:Player.OccupiedTerritories` | full | `Intersect` then `ToList()`: full. |
| `Game_Move.cs:700:Player.TerritoriesWithForces` | full | `Intersect` then `ToList()`: full. Rust collects the iterator. |
| `Game_Move.cs:81:foreach fromTerritory` | foreach | As `DiscoveryEntered.cs`: keys of the move's `ForceLocations`. |
| `Game_Move.cs:862:Game.TerrorIn` | short | `Any` stops at the first match. |
| `Game_Move.cs:86:foreach fl` | foreach | As `DiscoveryEntered.cs`: the body moves the player's forces, not the event's. |
| `Game_Move.cs:97:local fromLocations` | full | Passed through `LogMove` to `Message.WithRoute`, which calls `ToList()` at once. |
| `Game_Rules.cs:313:Enumerations.GetValuesExceptDefault` | forward | Returned by `Game.GetAvailableRulesets`; see its producer row. |
| `Game_Rules.cs:400:foreach rule` | foreach | Rust iterates the same `filter` lazily and returns from inside the loop. |
| `Game_Setup.cs:112:foreach p` | foreach | The predicate reads fields of `Game.Players`, which in Rust is a `Vec<Player>` that can't hold null, and the body doesn't change what it reads. A deck that runs out throws in the body, after the same earlier bodies in both engines. |
| `Game_Setup.cs:174:Enumerations.GetValuesExceptDefault` | full | The `Deck` constructor copies it. |
| `Game_Setup.cs:220:Map.Homeworlds` | short | `First` stops at the first match. |
| `Game_Setup.cs:231:Map.Homeworlds` | short | `First` stops at the first match. |
| `Game_Setup.cs:347:foreach p` | foreach | The predicate reads fields of `Game.Players`, which in Rust is a `Vec<Player>` that can't hold null, and the body doesn't change what it reads. A deck that runs out throws in the body, after the same earlier bodies in both engines. |
| `Game_Setup.cs:41:foreach p` | foreach | The predicate reads fields of `Game.Players`, which in Rust is a `Vec<Player>` that can't hold null, and the body doesn't change what it reads. A deck that runs out throws in the body, after the same earlier bodies in both engines. |
| `Game_Storm.cs:143:foreach l` | foreach | `Forces()` is a fresh dictionary; the predicate reads sectors and static protection. |
| `Game_Storm.cs:187:Game.Strongholds` | foreach | The predicate reads static sectors and the storm; the body removes ambassadors and kills forces. |
| `Game_Storm.cs:187:foreach t` | foreach | As the row above. |
| `Game_Storm.cs:200:Map.Locations` | foreach | The predicate reads static sectors; the body removes spice. |
| `Game_Storm.cs:200:foreach l` | foreach | The predicate reads static sectors; the body removes spice. |
| `Map.cs:115:Map.Locations` | forward | Returned by `Map.Territories`; see its producer row. |
| `Map.cs:120:Map.Homeworlds` | short | `FirstOrDefault` stops at the first match. |
| `Map.cs:126:foreach location` | foreach | Static map data. |
| `Map.cs:1778:foreach neighbour` | foreach | The predicates read static neighbours, `previous`, the storm sector and obstacle lists that the loop doesn't change, or the current path, which the recursion changes; map.rs checks the filter inside the loop body for each neighbour, as C# does. |
| `Map.cs:1805:foreach neighbour` | foreach | The predicates read static neighbours, `previous`, the storm sector and obstacle lists that the loop doesn't change, or the current path, which the recursion changes; map.rs checks the filter inside the loop body for each neighbour, as C# does. |
| `Map.cs:1839:foreach neighbour` | foreach | The predicates read static neighbours, `previous`, the storm sector and obstacle lists that the loop doesn't change, or the current path, which the recursion changes; map.rs checks the filter inside the loop body for each neighbour, as C# does. |
| `Map.cs:1871:foreach neighbour` | foreach | The predicates read static neighbours, `previous`, the storm sector and obstacle lists that the loop doesn't change, or the current path, which the recursion changes; map.rs checks the filter inside the loop body for each neighbour, as C# does. |
| `Map.cs:1895:foreach neighbour` | foreach | The predicates read static neighbours, `previous`, the storm sector and obstacle lists that the loop doesn't change, or the current path, which the recursion changes; map.rs checks the filter inside the loop body for each neighbour, as C# does. |
| `Map.cs:1908:Map.Territories` | short | `SingleOrDefault` stops at the first match. |
| `Map.cs:86:Map.Locations` | short | `FirstOrDefault` stops at the first match. |
| `Model/GameStatus.cs:267:BlackMarketBid.PlayersToBid` | full | Enumerated once, completely, with nothing in between. |
| `Model/GameStatus.cs:302:Bid.PlayersToBid` | full | Enumerated once, completely, with nothing in between. |
| `Model/GameStatus.cs:334:PlayerSequence.GetPlayersInSequence` | full | Enumerated once, completely, with nothing in between. |
| `Model/GameStatus.cs:339:PlayerSequence.GetPlayersInSequence` | full | Enumerated once, completely, with nothing in between. |
| `Model/GameStatus.cs:344:PlayerSequence.GetPlayersInSequence` | full | Enumerated once, completely, with nothing in between. |
| `Model/GameStatus.cs:349:PlayerSequence.GetPlayersInSequence` | full | Enumerated once, completely, with nothing in between. |
| `Model/GameStatus.cs:413:PlayerSequence.GetPlayersInSequence` | full | Enumerated once, completely, with nothing in between. |
| `Model/GameStatus.cs:418:PlayerSequence.GetPlayersInSequence` | full | Enumerated once, completely, with nothing in between. |
| `Model/GameStatus.cs:763:Map.GetResourceCardsInPlay` | short | `FirstOrDefault` stops at the first match. |
| `Model/GameStatus.cs:775:Map.GetResourceCardsInPlay` | short | `FirstOrDefault` stops at the first match. |
| `Model/GameStatus.cs:780:Map.GetResourceCardsInPlay` | short | `FirstOrDefault` stops at the first match. |
| `Model/Player.cs:272:foreach l` | foreach | Static territory locations. Where the body changes forces, it changes those of the current location only, which the later predicates don't read. `OrderBy` computes all keys before the `break`. |
| `Model/Player.cs:298:foreach l` | foreach | Static territory locations. Where the body changes forces, it changes those of the current location only, which the later predicates don't read. `OrderBy` computes all keys before the `break`. |
| `Model/Player.cs:327:foreach l` | foreach | Static territory locations. Where the body changes forces, it changes those of the current location only, which the later predicates don't read. `OrderBy` computes all keys before the `break`. |
| `Model/Player.cs:350:foreach l` | foreach | Static territory locations. Where the body changes forces, it changes those of the current location only, which the later predicates don't read. `OrderBy` computes all keys before the `break`. |
| `Model/Player.cs:471:Map.Locations` | forward | Returned by `Player.OccupiedLocations`; see its producer row. |
| `Model/Player.cs:473:Map.Territories` | forward | Returned by `Player.OccupiedTerritories`; see its producer row. |
| `Model/Player.cs:489:Map.Territories` | forward | Returned by `Player.TerritoriesWithForces`; see its producer row. |
| `Model/Player.cs:606:foreach kvp` | foreach | The body adds to a new dictionary. |
| `Model/Player.cs:626:LeaderManager.GetLeaders` | full | Enumerated once, completely, with nothing in between. |
| `Model/Player.cs:627:LeaderManager.GetLeaders` | full | Enumerated once, completely, with nothing in between. |
| `Model/Player.cs:628:LeaderManager.GetLeaders` | full | Enumerated once, completely, with nothing in between. |
| `Model/Player.cs:643:Karma.ValidKarmaCards` | short | `Any` stops at the first match. |
| `Model/Utilities.cs:64:foreach item` | foreach | JSON parsing of a local array. |
| `TreacheryCardManager.cs:109:TreacheryCardManager.GetCardsInPlay` | full | The `Deck` constructor copies it. |
| `TreacheryCardManager.cs:114:TreacheryCardManager.GetCardsInPlay` | full | Enumerated once, completely, with nothing in between. |
