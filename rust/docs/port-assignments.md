# Port assignments

Every file in `treachery-online/Shared` that the foundation layer did not port has a Rust module with compiling signatures and `todo!("port: File.cs:line")` bodies. This page splits that work into packages by rules area. Read [engine-conventions.md](engine-conventions.md) before starting, especially Players, Events and Signatures in the skeleton.

Each table lists the C# file, its length, the Rust module and the number of `todo!`s in it. Events are placed with the phase or rules area they belong to. A few files fit more than one area; the choice only matters for who owns the file.

Packages A and B are the base every other package calls (`Game.Log`, `Game.GetPlayer`, `Player.Has`, `GameEvent.Execute`, `Game.Applicable`, `PlayerSequence`). Port them first, or at least the members other packages need, before tests in other areas can run. Inside a package, port the `Game_*.cs` partial before its events.

Files that are not ported at all:

- `DefaultSkin.cs`, `Model/Skin.cs` and `Model/IDescriber.cs`: rendering (see "Not ported" in the conventions).
- Server and hub types: `GameChatMessage.cs`, `Model/ChatMessage.cs`, `Model/GlobalChatMessage.cs`, `Model/AdminInfo.cs`, `Model/AvailableSeatInfo.cs`, `Model/GameInitInfo.cs`, `Model/IBot.cs`, `Model/IGameClient.cs`, `Model/IGameHub.cs`, `Model/LoggedInUserInfo.cs`, `Model/LoginInfo.cs`, `Model/ManagedGame.cs`, `Model/ObserverJoined.cs`, `Model/ObserverRejoined.cs`, `Model/PlayerJoined.cs`, `Model/PlayerRejoined.cs`, `Model/Result.cs` (its `ErrorType` enum is in `enumerations.rs`), `Model/ScheduledGame.cs`, `Model/ScheduledGameInfo.cs`, `Model/ServerInfo.cs`, `Model/ServerStatus.cs`, `Model/UserConnections.cs`, `Model/UserInfo.cs`. These belong to the server phase. `Game_Participation.cs` has one method returning `AvailableSeatInfo`; its return type is a `()` placeholder.

Some ported foundation files gained pieces with the skeleton: `Payment.reason` and `Payment::new_full`, `Report::express_event`, `cs::RcKey` and `Default` for `Shared<T>`. The `MapGame` impl for `Game` in `game.rs` has two `todo!`s that belong to package B (`Forces()` and `GetPlayer(f)?.Ally`).

## Packages

| Package | Files | C# lines | `todo!`s |
|---|---:|---:|---:|
| A. Core: game and events | 16 | 2733 | 229 |
| B. Core: players and rules | 5 | 2204 | 144 |
| C. Setup | 19 | 2086 | 127 |
| D. Storm, spice blow, collection, charity | 26 | 2523 | 134 |
| E. Bidding | 20 | 2503 | 138 |
| F. Revival and leaders | 14 | 1856 | 128 |
| G. Shipment and movement | 19 | 3045 | 200 |
| H. Battle: plans and resolution | 14 | 3738 | 237 |
| I. Battle: actions around the fight | 20 | 1695 | 106 |
| J. Faction specials: Karma, Nexus, Moritani, Ecaz, CHOAM, clairvoyance | 21 | 3234 | 195 |
| K. Alliances, deals, discards, status | 23 | 3456 | 218 |
| **Total** | 197 | 29073 | 1856 |

### A. Core: game and events

Do first. Everything else calls into these.

| C# file | Lines | Rust module | `todo!`s |
|---|---:|---|---:|
| `Game.cs` | 1121 | `src/game/mod.rs` | 111 |
| `Game_Participation.cs` | 162 | `src/game/participation.rs` | 35 |
| `Events/GameEvent.cs` | 342 | `src/events/game_event.rs` | 21 |
| `Events/PassableGameEvent.cs` | 25 | `src/events/passable_game_event.rs` | 0 |
| `Events/PlacementEvent.cs` | 216 | `src/events/movement/placement_event.rs` | 16 |
| `Events/IBid.cs` | 32 | `src/events/bidding/bid_trait.rs` | 0 |
| `Events/ILocationEvent.cs` | 23 | `src/events/movement/location_event.rs` | 0 |
| `Events/IPlacement.cs` | 19 | `src/events/movement/placement.rs` | 0 |
| `Events/EndPhase.cs` | 430 | `src/events/host/end_phase.rs` | 17 |
| `Model/GameState.cs` | 43 | `src/game/state.rs` | 5 |
| `Model/Utilities.cs` | 137 | `src/json/utilities.rs` | 13 |
| `Model/GameSettings.cs` | 18 | `src/game/settings.rs` | 0 |
| `Model/Moment.cs` | 22 | `src/game/moment.rs` | 1 |
| `Model/Timer.cs` | 30 | `src/game/counters.rs` | 2 |
| `Model/ObjectCounter.cs` | 77 | `src/game/counters.rs` | 8 |
| `Model/Participation.cs` | 36 | `src/game/participation.rs` | 0 |

### B. Core: players and rules

Do first, alongside A.

| C# file | Lines | Rust module | `todo!`s |
|---|---:|---|---:|
| `Game_Rules.cs` | 420 | `src/game/rules.rs` | 12 |
| `Game_Events.cs` | 709 | `src/game/applicable_events.rs` | 6 |
| `Model/Player.cs` | 736 | `src/game/player.rs` | 95 |
| `Model/PlayerSequence.cs` | 248 | `src/game/player_sequence.rs` | 21 |
| `Model/Battalion.cs` | 91 | `src/game/battalion.rs` | 10 |

### C. Setup

| C# file | Lines | Rust module | `todo!`s |
|---|---:|---|---:|
| `Game_Setup.cs` | 366 | `src/game/setup.rs` | 20 |
| `Events/EstablishPlayers.cs` | 414 | `src/events/host/establish_players.rs` | 19 |
| `Events/PerformSetup.cs` | 77 | `src/events/host/perform_setup.rs` | 3 |
| `Events/PerformFremenSetup.cs` | 67 | `src/events/setup/perform_fremen_setup.rs` | 3 |
| `Events/PerformBeneGesseritPlacement.cs` | 84 | `src/events/setup/perform_bene_gesserit_placement.rs` | 7 |
| `Events/PerformMoritaniSetup.cs` | 71 | `src/events/setup/perform_moritani_setup.rs` | 6 |
| `Events/PerformHmsPlacement.cs` | 74 | `src/events/setup/perform_hms_placement.rs` | 6 |
| `Events/FactionSelected.cs` | 76 | `src/events/setup/faction_selected.rs` | 4 |
| `Events/FactionTradeOffered.cs` | 99 | `src/events/setup/faction_trade_offered.rs` | 5 |
| `Events/TraitorsSelected.cs` | 76 | `src/events/setup/traitors_selected.rs` | 5 |
| `Events/CardsDetermined.cs` | 85 | `src/events/host/cards_determined.rs` | 8 |
| `Events/IxianSelectedStartingCard.cs` | 67 | `src/events/setup/ixian_selected_starting_card.rs` | 5 |
| `Events/MulliganPerformed.cs` | 76 | `src/events/setup/mulligan_performed.rs` | 5 |
| `Events/SkillAssigned.cs` | 92 | `src/events/setup/skill_assigned.rs` | 8 |
| `Events/SwitchSkilledLeader.cs` | 66 | `src/events/battle/switch_skilled_leader.rs` | 5 |
| `Events/PlayerReplaced.cs` | 60 | `src/events/host/player_replaced.rs` | 4 |
| `Events/HideSecrets.cs` | 48 | `src/events/host/hide_secrets.rs` | 3 |
| `Events/GameInfo.cs` | 45 | `src/game/info.rs` | 5 |
| `Events/AutomationConfigured.cs` | 143 | `src/events/host/automation_configured.rs` | 6 |

### D. Storm, spice blow, collection, charity

| C# file | Lines | Rust module | `todo!`s |
|---|---:|---|---:|
| `Game_Storm.cs` | 248 | `src/game/storm.rs` | 11 |
| `Game_Blow.cs` | 516 | `src/game/blow.rs` | 25 |
| `Game_Collection.cs` | 218 | `src/game/collection.rs` | 9 |
| `Game_Charity.cs` | 85 | `src/game/charity.rs` | 5 |
| `Events/StormDialled.cs` | 152 | `src/events/storm/storm_dialled.rs` | 7 |
| `Events/StormSpellPlayed.cs` | 60 | `src/events/storm/storm_spell_played.rs` | 3 |
| `Events/ThumperPlayed.cs` | 52 | `src/events/blow/thumper_played.rs` | 3 |
| `Events/HarvesterPlayed.cs` | 55 | `src/events/blow/harvester_played.rs` | 3 |
| `Events/FremenSentMonster.cs` | 76 | `src/events/blow/fremen_sent_monster.rs` | 6 |
| `Events/FremenRidesMonster.cs` | 199 | `src/events/blow/fremen_rides_monster.rs` | 11 |
| `Events/KarmaMonster.cs` | 73 | `src/events/blow/karma_monster.rs` | 6 |
| `Events/NexusVoted.cs` | 59 | `src/events/blow/nexus_voted.rs` | 3 |
| `Events/NexusCardDrawn.cs` | 86 | `src/events/blow/nexus_card_drawn.rs` | 6 |
| `Events/DivideResources.cs` | 94 | `src/events/collection/divide_resources.rs` | 7 |
| `Events/DivideResourcesAccepted.cs` | 60 | `src/events/collection/divide_resources_accepted.rs` | 5 |
| `Events/CharityClaimed.cs` | 55 | `src/events/charity/charity_claimed.rs` | 4 |
| `Events/DiscoveryRevealed.cs` | 117 | `src/events/blow/discovery_revealed.rs` | 7 |
| `Events/DiscoveryEntered.cs` | 85 | `src/events/blow/discovery_entered.rs` | 5 |
| `Events/TestingStationUsed.cs` | 65 | `src/events/storm/testing_station_used.rs` | 4 |
| `Model/Discovery.cs` | 26 | `src/map/discovery.rs` | 1 |
| `Model/ResourcesToBeDivided.cs` | 18 | `src/game/collection.rs` | 1 |
| `Model/MonsterAppearence.cs` | 24 | `src/game/blow.rs` | 2 |
| `Model/Log/BlowLog.cs` | 57 | `src/log/blow.rs` | 0 |
| `Model/Log/StormLog.cs` | 30 | `src/log/storm.rs` | 0 |
| `Model/Log/CollectionLog.cs` | 8 | `src/log/collection.rs` | 0 |
| `Model/Log/CharityLog.cs` | 5 | `src/log/charity.rs` | 0 |

### E. Bidding

| C# file | Lines | Rust module | `todo!`s |
|---|---:|---|---:|
| `Game_Bidding.cs` | 639 | `src/game/bidding.rs` | 32 |
| `Events/Bid.cs` | 348 | `src/events/bidding/bid.rs` | 19 |
| `Events/BlackMarketBid.cs` | 258 | `src/events/bidding/black_market_bid.rs` | 15 |
| `Events/RicheseAnnouncesAuction.cs` | 64 | `src/events/bidding/richese_announces_auction.rs` | 3 |
| `Events/RicheseSpecifiesAuction.cs` | 133 | `src/events/bidding/richese_specifies_auction.rs` | 8 |
| `Events/RicheseAnnouncesBlackMarket.cs` | 102 | `src/events/bidding/richese_announces_black_market.rs` | 6 |
| `Events/RicheseKeepsUnsoldCard.cs` | 64 | `src/events/bidding/richese_keeps_unsold_card.rs` | 3 |
| `Events/RicheseGaveCard.cs` | 79 | `src/events/alliances/richese_gave_card.rs` | 6 |
| `Events/IxianRemovedCardFromAuction.cs` | 94 | `src/events/bidding/ixian_removed_card_from_auction.rs` | 5 |
| `Events/IxianSwappedCardOnBid.cs` | 78 | `src/events/bidding/ixian_swapped_card_on_bid.rs` | 5 |
| `Events/EmperorBidSupport.cs` | 57 | `src/events/bidding/emperor_bid_support.rs` | 3 |
| `Events/ReplacedCardWon.cs` | 74 | `src/events/bidding/replaced_card_won.rs` | 3 |
| `Events/KarmaRicheseBuy.cs` | 69 | `src/events/bidding/karma_richese_buy.rs` | 5 |
| `Events/KarmaHandSwapInitiated.cs` | 77 | `src/events/bidding/karma_hand_swap_initiated.rs` | 3 |
| `Events/KarmaHandSwap.cs` | 81 | `src/events/bidding/karma_hand_swap.rs` | 5 |
| `Events/CardGiven.cs` | 63 | `src/events/bidding/card_given.rs` | 4 |
| `Events/DistransUsed.cs` | 108 | `src/events/cards/distrans_used.rs` | 9 |
| `Events/Bureaucracy.cs` | 66 | `src/events/factions/bureaucracy.rs` | 4 |
| `Model/TriggeredBureaucracy.cs` | 16 | `src/game/other.rs` | 0 |
| `Model/Log/BiddingLog.cs` | 33 | `src/log/bidding.rs` | 0 |

### F. Revival and leaders

| C# file | Lines | Rust module | `todo!`s |
|---|---:|---|---:|
| `Game_Revival.cs` | 199 | `src/game/revival.rs` | 8 |
| `Game_LeaderState.cs` | 172 | `src/game/leaders.rs` | 21 |
| `Events/Revival.cs` | 637 | `src/events/revival/revival.rs` | 34 |
| `Events/RequestTleilaxuRevival.cs` | 76 | `src/events/revival/request_tleilaxu_revival.rs` | 6 |
| `Events/AcceptOrCancelTleilaxuRevival.cs` | 93 | `src/events/revival/accept_or_cancel_tleilaxu_revival.rs` | 6 |
| `Events/KarmaFreeRevival.cs` | 129 | `src/events/revival/karma_free_revival.rs` | 6 |
| `Events/RaiseDeadPlayed.cs` | 193 | `src/events/cards/raise_dead_played.rs` | 14 |
| `Events/SetIncreasedRevivalLimits.cs` | 65 | `src/events/revival/set_increased_revival_limits.rs` | 4 |
| `Events/ChoamFreeRevivalPrevention.cs` | 78 | `src/events/factions/choam_free_revival_prevention.rs` | 5 |
| `Events/KarmaRevivalPrevention.cs` | 64 | `src/events/revival/karma_revival_prevention.rs` | 4 |
| `Events/RecruitsPlayed.cs` | 55 | `src/events/cards/recruits_played.rs` | 4 |
| `Model/RevivalCost.cs` | 24 | `src/game/revival.rs` | 3 |
| `Model/LeaderState.cs` | 56 | `src/game/leaders.rs` | 7 |
| `Model/Log/RevivalLog.cs` | 15 | `src/log/revival.rs` | 6 |

### G. Shipment and movement

| C# file | Lines | Rust module | `todo!`s |
|---|---:|---|---:|
| `Game_Move.cs` | 930 | `src/game/movement.rs` | 43 |
| `Events/Shipment.cs` | 751 | `src/events/movement/shipment.rs` | 68 |
| `Events/Move.cs` | 103 | `src/events/movement/move.rs` | 3 |
| `Events/Caravan.cs` | 59 | `src/events/movement/caravan.rs` | 3 |
| `Events/BeneGesseritAccompanies.cs` | 151 | `src/events/movement/bene_gesserit_accompanies.rs` | 13 |
| `Events/BeneGesseritFlip.cs` | 88 | `src/events/movement/bene_gesserit_flip.rs` | 6 |
| `Events/ChoamExtraMove.cs` | 72 | `src/events/factions/choam_extra_move.rs` | 5 |
| `Events/ChoamMovePrevention.cs` | 92 | `src/events/factions/choam_move_prevention.rs` | 8 |
| `Events/KarmaShipmentPrevention.cs` | 64 | `src/events/movement/karma_shipment_prevention.rs` | 4 |
| `Events/SetShipmentPermission.cs` | 79 | `src/events/movement/set_shipment_permission.rs` | 5 |
| `Events/GuildDelay.cs` | 50 | `src/events/movement/guild_delay.rs` | 3 |
| `Events/PerformHmsMovement.cs` | 86 | `src/events/storm/perform_hms_movement.rs` | 6 |
| `Events/KarmaHmsMovement.cs` | 99 | `src/events/storm/karma_hms_movement.rs` | 6 |
| `Events/FlightUsed.cs` | 65 | `src/events/movement/flight_used.rs` | 5 |
| `Events/FlightDiscoveryUsed.cs` | 55 | `src/events/movement/flight_discovery_used.rs` | 4 |
| `Events/HMSAdvantageChosen.cs` | 71 | `src/events/battle/hms_advantage_chosen.rs` | 5 |
| `Events/ChoamRemoveForce.cs` | 134 | `src/events/factions/choam_remove_force.rs` | 10 |
| `Model/Intrusion.cs` | 27 | `src/game/movement.rs` | 3 |
| `Model/Log/ShipMoveLog.cs` | 69 | `src/log/ship_move.rs` | 0 |

### H. Battle: plans and resolution

| C# file | Lines | Rust module | `todo!`s |
|---|---:|---|---:|
| `Game_Battle.cs` | 1264 | `src/game/battle.rs` | 64 |
| `Events/Battle.cs` | 948 | `src/events/battle/battle.rs` | 67 |
| `Events/BattleInitiated.cs` | 240 | `src/events/battle/battle_initiated.rs` | 28 |
| `Events/BattleClaimed.cs` | 75 | `src/events/battle/battle_claimed.rs` | 4 |
| `Events/BattleRevision.cs` | 59 | `src/events/battle/battle_revision.rs` | 3 |
| `Events/BattleConcluded.cs` | 356 | `src/events/battle/battle_concluded.rs` | 23 |
| `Events/TreacheryCalled.cs` | 118 | `src/events/battle/treachery_called.rs` | 6 |
| `Events/LoserConcluded.cs` | 211 | `src/events/battle/loser_concluded.rs` | 14 |
| `Events/Captured.cs` | 60 | `src/events/battle/captured.rs` | 3 |
| `Events/Retreat.cs` | 125 | `src/events/battle/retreat.rs` | 9 |
| `Events/TakeLosses.cs` | 152 | `src/events/storm/take_losses.rs` | 9 |
| `Model/BattleOutcome.cs` | 88 | `src/game/battle_outcome.rs` | 4 |
| `Model/Fight.cs` | 25 | `src/game/battle.rs` | 2 |
| `Model/LossToTake.cs` | 17 | `src/game/battle.rs` | 1 |

### I. Battle: actions around the fight

| C# file | Lines | Rust module | `todo!`s |
|---|---:|---|---:|
| `Events/Voice.cs` | 181 | `src/events/battle/voice.rs` | 7 |
| `Events/Prescience.cs` | 76 | `src/events/battle/prescience.rs` | 5 |
| `Events/Thought.cs` | 80 | `src/events/battle/thought.rs` | 7 |
| `Events/ThoughtAnswered.cs` | 86 | `src/events/battle/thought_answered.rs` | 6 |
| `Events/Diplomacy.cs` | 88 | `src/events/battle/diplomacy.rs` | 7 |
| `Events/PoisonToothCancelled.cs` | 49 | `src/events/battle/poison_tooth_cancelled.rs` | 3 |
| `Events/PortableAntidoteUsed.cs` | 61 | `src/events/battle/portable_antidote_used.rs` | 4 |
| `Events/RockWasMelted.cs` | 65 | `src/events/battle/rock_was_melted.rs` | 4 |
| `Events/AuditCancelled.cs` | 105 | `src/events/battle/audit_cancelled.rs` | 7 |
| `Events/Audited.cs` | 53 | `src/events/battle/audited.rs` | 3 |
| `Events/FaceDanced.cs` | 202 | `src/events/battle/face_danced.rs` | 12 |
| `Events/FaceDancerReplaced.cs` | 72 | `src/events/collection/face_dancer_replaced.rs` | 6 |
| `Events/FaceDancerRevealed.cs` | 66 | `src/events/battle/face_dancer_revealed.rs` | 3 |
| `Events/BeneGesseritBattleAnnouncement.cs` | 81 | `src/events/battle/bene_gesserit_battle_announcement.rs` | 6 |
| `Events/ResidualPlayed.cs` | 71 | `src/events/battle/residual_played.rs` | 4 |
| `Events/Planetology.cs` | 66 | `src/events/movement/planetology.rs` | 5 |
| `Events/JuicePlayed.cs` | 110 | `src/events/cards/juice_played.rs` | 5 |
| `Events/KarmaEcazDial.cs` | 58 | `src/events/battle/karma_ecaz_dial.rs` | 3 |
| `Events/KarmaPrescience.cs` | 51 | `src/events/battle/karma_prescience.rs` | 3 |
| `Events/TraitorDiscarded.cs` | 74 | `src/events/battle/traitor_discarded.rs` | 6 |

### J. Faction specials: Karma, Nexus, Moritani, Ecaz, CHOAM, clairvoyance

| C# file | Lines | Rust module | `todo!`s |
|---|---:|---|---:|
| `Events/Karma.cs` | 303 | `src/events/cards/karma.rs` | 10 |
| `Events/KarmaChoamDiscard.cs` | 82 | `src/events/factions/karma_choam_discard.rs` | 7 |
| `Events/NexusPlayed.cs` | 678 | `src/events/cards/nexus_played.rs` | 36 |
| `Events/TerrorPlanted.cs` | 164 | `src/events/factions/terror_planted.rs` | 11 |
| `Events/TerrorRevealed.cs` | 272 | `src/events/factions/terror_revealed.rs` | 19 |
| `Events/AllianceByTerror.cs` | 86 | `src/events/alliances/alliance_by_terror.rs` | 3 |
| `Events/AmbassadorPlaced.cs` | 96 | `src/events/factions/ambassador_placed.rs` | 8 |
| `Events/AmbassadorActivated.cs` | 471 | `src/events/factions/ambassador_activated.rs` | 36 |
| `Events/AllianceByAmbassador.cs` | 70 | `src/events/alliances/alliance_by_ambassador.rs` | 3 |
| `Events/ChoamDiscarded.cs` | 84 | `src/events/factions/choam_discarded.rs` | 6 |
| `Events/ChoamEconomics.cs` | 56 | `src/events/factions/choam_economics.rs` | 3 |
| `Events/ChoamKarmaPrevention.cs` | 78 | `src/events/factions/choam_karma_prevention.rs` | 5 |
| `Events/ResourcesAudited.cs` | 68 | `src/events/factions/resources_audited.rs` | 4 |
| `Events/ClairVoyancePlayed.cs` | 251 | `src/events/cards/clair_voyance_played.rs` | 17 |
| `Events/ClairVoyanceAnswered.cs` | 119 | `src/events/cards/clair_voyance_answered.rs` | 6 |
| `Events/AmalPlayed.cs` | 60 | `src/events/cards/amal_played.rs` | 3 |
| `Events/MetheorPlayed.cs` | 95 | `src/events/storm/metheor_played.rs` | 5 |
| `Events/ExtortionPrevented.cs` | 60 | `src/events/collection/extortion_prevented.rs` | 4 |
| `Events/BeneGesseritPrediction.cs` | 72 | `src/events/setup/bene_gesserit_prediction.rs` | 5 |
| `Events/RicheseRevealedNoField.cs` | 46 | `src/events/movement/richese_revealed_no_field.rs` | 3 |
| `Model/ClairVoyanceQandA.cs` | 23 | `src/game/other.rs` | 1 |

### K. Alliances, deals, discards, status

| C# file | Lines | Rust module | `todo!`s |
|---|---:|---|---:|
| `Game_Alliances.cs` | 186 | `src/game/alliances.rs` | 10 |
| `Game_Deals.cs` | 69 | `src/game/deals.rs` | 8 |
| `Game_Other.cs` | 218 | `src/game/other.rs` | 19 |
| `Game_Contemplate.cs` | 440 | `src/game/contemplate.rs` | 30 |
| `GameStatistics.cs` | 98 | `src/game/statistics.rs` | 5 |
| `Events/AllianceOffered.cs` | 80 | `src/events/alliances/alliance_offered.rs` | 4 |
| `Events/AllianceBroken.cs` | 52 | `src/events/alliances/alliance_broken.rs` | 3 |
| `Events/AlliancePassed.cs` | 60 | `src/events/alliances/alliance_passed.rs` | 0 (added in the v188 sync, ported directly) |
| `Events/AllyPermission.cs` | 161 | `src/events/alliances/ally_permission.rs` | 11 |
| `Events/DealOffered.cs` | 196 | `src/events/alliances/deal_offered.rs` | 13 |
| `Events/DealAccepted.cs` | 185 | `src/events/alliances/deal_accepted.rs` | 12 |
| `Events/DealRejected.cs` | 89 | `src/events/alliances/deal_rejected.rs` | 5 |
| `Events/Donated.cs` | 138 | `src/events/alliances/donated.rs` | 9 |
| `Events/ResourcesTransferred.cs` | 77 | `src/events/alliances/resources_transferred.rs` | 6 |
| `Events/CardTraded.cs` | 114 | `src/events/alliances/card_traded.rs` | 8 |
| `Events/Discarded.cs` | 69 | `src/events/cards/discarded.rs` | 5 |
| `Events/DiscardedSearched.cs` | 83 | `src/events/cards/discarded_searched.rs` | 7 |
| `Events/DiscardedSearchedAnnounced.cs` | 57 | `src/events/cards/discarded_searched_announced.rs` | 4 |
| `Events/DiscardedTaken.cs` | 79 | `src/events/cards/discarded_taken.rs` | 7 |
| `Events/EmperorDiscarded.cs` | 67 | `src/events/bidding/emperor_discarded.rs` | 5 |
| `Model/Deal.cs` | 82 | `src/game/deals.rs` | 5 |
| `Model/GameStatus.cs` | 870 | `src/game/status.rs` | 39 |
| `Model/Log/AsideLog.cs` | 32 | `src/log/aside.rs` | 3 |
| `Model/Log/MentatLog.cs` | 14 | `src/log/mentat.rs` | 0 |

## Bots (phase 2, not started)

The bots in `treachery-online/Bots` are not ported. Nothing tracks them yet except this list of C# changes made after the port's first snapshot, which the bot port must include. The current C# is the spec; the list only flags code that is newer than the engine port's review.

- v188 sync (C# change of Nov 7, see [csharp-sync.md](csharp-sync.md)): `Classic/Bot_Alliance.cs` gains `DetermineAlliancePassed` (wait while one of the bot's own alliance offers is open, or while any human has not acted or passed; otherwise pass), and `Classic/Bot.cs` calls it right after `DetermineAllianceOffered`. Bot games in the differ still run the C# bots, so this is exercised there already.

