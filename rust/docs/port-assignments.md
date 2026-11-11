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
| `Game.cs` | 1121 | `src/game.rs` | 111 |
| `Game_Participation.cs` | 162 | `src/game_participation.rs` | 35 |
| `Events/GameEvent.cs` | 342 | `src/events/game_event.rs` | 21 |
| `Events/PassableGameEvent.cs` | 25 | `src/events/passable_game_event.rs` | 0 |
| `Events/PlacementEvent.cs` | 216 | `src/events/placement_event.rs` | 16 |
| `Events/IBid.cs` | 32 | `src/events/i_bid.rs` | 0 |
| `Events/ILocationEvent.cs` | 23 | `src/events/i_location_event.rs` | 0 |
| `Events/IPlacement.cs` | 19 | `src/events/i_placement.rs` | 0 |
| `Events/EndPhase.cs` | 430 | `src/events/end_phase.rs` | 17 |
| `Model/GameState.cs` | 43 | `src/game_state.rs` | 5 |
| `Model/Utilities.cs` | 137 | `src/utilities.rs` | 13 |
| `Model/GameSettings.cs` | 18 | `src/game_settings.rs` | 0 |
| `Model/Moment.cs` | 22 | `src/moment.rs` | 1 |
| `Model/Timer.cs` | 30 | `src/timer.rs` | 2 |
| `Model/ObjectCounter.cs` | 77 | `src/object_counter.rs` | 8 |
| `Model/Participation.cs` | 36 | `src/participation.rs` | 0 |

### B. Core: players and rules

Do first, alongside A.

| C# file | Lines | Rust module | `todo!`s |
|---|---:|---|---:|
| `Game_Rules.cs` | 420 | `src/game_rules.rs` | 12 |
| `Game_Events.cs` | 709 | `src/game_events.rs` | 6 |
| `Model/Player.cs` | 736 | `src/player.rs` | 95 |
| `Model/PlayerSequence.cs` | 248 | `src/player_sequence.rs` | 21 |
| `Model/Battalion.cs` | 91 | `src/battalion.rs` | 10 |

### C. Setup

| C# file | Lines | Rust module | `todo!`s |
|---|---:|---|---:|
| `Game_Setup.cs` | 366 | `src/game_setup.rs` | 20 |
| `Events/EstablishPlayers.cs` | 414 | `src/events/establish_players.rs` | 19 |
| `Events/PerformSetup.cs` | 77 | `src/events/perform_setup.rs` | 3 |
| `Events/PerformFremenSetup.cs` | 67 | `src/events/perform_fremen_setup.rs` | 3 |
| `Events/PerformBeneGesseritPlacement.cs` | 84 | `src/events/perform_bene_gesserit_placement.rs` | 7 |
| `Events/PerformMoritaniSetup.cs` | 71 | `src/events/perform_moritani_setup.rs` | 6 |
| `Events/PerformHmsPlacement.cs` | 74 | `src/events/perform_hms_placement.rs` | 6 |
| `Events/FactionSelected.cs` | 76 | `src/events/faction_selected.rs` | 4 |
| `Events/FactionTradeOffered.cs` | 99 | `src/events/faction_trade_offered.rs` | 5 |
| `Events/TraitorsSelected.cs` | 76 | `src/events/traitors_selected.rs` | 5 |
| `Events/CardsDetermined.cs` | 85 | `src/events/cards_determined.rs` | 8 |
| `Events/IxianSelectedStartingCard.cs` | 67 | `src/events/ixian_selected_starting_card.rs` | 5 |
| `Events/MulliganPerformed.cs` | 76 | `src/events/mulligan_performed.rs` | 5 |
| `Events/SkillAssigned.cs` | 92 | `src/events/skill_assigned.rs` | 8 |
| `Events/SwitchSkilledLeader.cs` | 66 | `src/events/switch_skilled_leader.rs` | 5 |
| `Events/PlayerReplaced.cs` | 60 | `src/events/player_replaced.rs` | 4 |
| `Events/HideSecrets.cs` | 48 | `src/events/hide_secrets.rs` | 3 |
| `Events/GameInfo.cs` | 45 | `src/events/game_info.rs` | 5 |
| `Events/AutomationConfigured.cs` | 143 | `src/events/automation_configured.rs` | 6 |

### D. Storm, spice blow, collection, charity

| C# file | Lines | Rust module | `todo!`s |
|---|---:|---|---:|
| `Game_Storm.cs` | 248 | `src/game_storm.rs` | 11 |
| `Game_Blow.cs` | 516 | `src/game_blow.rs` | 25 |
| `Game_Collection.cs` | 218 | `src/game_collection.rs` | 9 |
| `Game_Charity.cs` | 85 | `src/game_charity.rs` | 5 |
| `Events/StormDialled.cs` | 152 | `src/events/storm_dialled.rs` | 7 |
| `Events/StormSpellPlayed.cs` | 60 | `src/events/storm_spell_played.rs` | 3 |
| `Events/ThumperPlayed.cs` | 52 | `src/events/thumper_played.rs` | 3 |
| `Events/HarvesterPlayed.cs` | 55 | `src/events/harvester_played.rs` | 3 |
| `Events/FremenSentMonster.cs` | 76 | `src/events/fremen_sent_monster.rs` | 6 |
| `Events/FremenRidesMonster.cs` | 199 | `src/events/fremen_rides_monster.rs` | 11 |
| `Events/KarmaMonster.cs` | 73 | `src/events/karma_monster.rs` | 6 |
| `Events/NexusVoted.cs` | 59 | `src/events/nexus_voted.rs` | 3 |
| `Events/NexusCardDrawn.cs` | 86 | `src/events/nexus_card_drawn.rs` | 6 |
| `Events/DivideResources.cs` | 94 | `src/events/divide_resources.rs` | 7 |
| `Events/DivideResourcesAccepted.cs` | 60 | `src/events/divide_resources_accepted.rs` | 5 |
| `Events/CharityClaimed.cs` | 55 | `src/events/charity_claimed.rs` | 4 |
| `Events/DiscoveryRevealed.cs` | 117 | `src/events/discovery_revealed.rs` | 7 |
| `Events/DiscoveryEntered.cs` | 85 | `src/events/discovery_entered.rs` | 5 |
| `Events/TestingStationUsed.cs` | 65 | `src/events/testing_station_used.rs` | 4 |
| `Model/Discovery.cs` | 26 | `src/discovery.rs` | 1 |
| `Model/ResourcesToBeDivided.cs` | 18 | `src/resources_to_be_divided.rs` | 1 |
| `Model/MonsterAppearence.cs` | 24 | `src/monster_appearence.rs` | 2 |
| `Model/Log/BlowLog.cs` | 57 | `src/log/blow_log.rs` | 0 |
| `Model/Log/StormLog.cs` | 30 | `src/log/storm_log.rs` | 0 |
| `Model/Log/CollectionLog.cs` | 8 | `src/log/collection_log.rs` | 0 |
| `Model/Log/CharityLog.cs` | 5 | `src/log/charity_log.rs` | 0 |

### E. Bidding

| C# file | Lines | Rust module | `todo!`s |
|---|---:|---|---:|
| `Game_Bidding.cs` | 639 | `src/game_bidding.rs` | 32 |
| `Events/Bid.cs` | 348 | `src/events/bid.rs` | 19 |
| `Events/BlackMarketBid.cs` | 258 | `src/events/black_market_bid.rs` | 15 |
| `Events/RicheseAnnouncesAuction.cs` | 64 | `src/events/richese_announces_auction.rs` | 3 |
| `Events/RicheseSpecifiesAuction.cs` | 133 | `src/events/richese_specifies_auction.rs` | 8 |
| `Events/RicheseAnnouncesBlackMarket.cs` | 102 | `src/events/richese_announces_black_market.rs` | 6 |
| `Events/RicheseKeepsUnsoldCard.cs` | 64 | `src/events/richese_keeps_unsold_card.rs` | 3 |
| `Events/RicheseGaveCard.cs` | 79 | `src/events/richese_gave_card.rs` | 6 |
| `Events/IxianRemovedCardFromAuction.cs` | 94 | `src/events/ixian_removed_card_from_auction.rs` | 5 |
| `Events/IxianSwappedCardOnBid.cs` | 78 | `src/events/ixian_swapped_card_on_bid.rs` | 5 |
| `Events/EmperorBidSupport.cs` | 57 | `src/events/emperor_bid_support.rs` | 3 |
| `Events/ReplacedCardWon.cs` | 74 | `src/events/replaced_card_won.rs` | 3 |
| `Events/KarmaRicheseBuy.cs` | 69 | `src/events/karma_richese_buy.rs` | 5 |
| `Events/KarmaHandSwapInitiated.cs` | 77 | `src/events/karma_hand_swap_initiated.rs` | 3 |
| `Events/KarmaHandSwap.cs` | 81 | `src/events/karma_hand_swap.rs` | 5 |
| `Events/CardGiven.cs` | 63 | `src/events/card_given.rs` | 4 |
| `Events/DistransUsed.cs` | 108 | `src/events/distrans_used.rs` | 9 |
| `Events/Bureaucracy.cs` | 66 | `src/events/bureaucracy.rs` | 4 |
| `Model/TriggeredBureaucracy.cs` | 16 | `src/triggered_bureaucracy.rs` | 0 |
| `Model/Log/BiddingLog.cs` | 33 | `src/log/bidding_log.rs` | 0 |

### F. Revival and leaders

| C# file | Lines | Rust module | `todo!`s |
|---|---:|---|---:|
| `Game_Revival.cs` | 199 | `src/game_revival.rs` | 8 |
| `Game_LeaderState.cs` | 172 | `src/game_leader_state.rs` | 21 |
| `Events/Revival.cs` | 637 | `src/events/revival.rs` | 34 |
| `Events/RequestTleilaxuRevival.cs` | 76 | `src/events/request_tleilaxu_revival.rs` | 6 |
| `Events/AcceptOrCancelTleilaxuRevival.cs` | 93 | `src/events/accept_or_cancel_tleilaxu_revival.rs` | 6 |
| `Events/KarmaFreeRevival.cs` | 129 | `src/events/karma_free_revival.rs` | 6 |
| `Events/RaiseDeadPlayed.cs` | 193 | `src/events/raise_dead_played.rs` | 14 |
| `Events/SetIncreasedRevivalLimits.cs` | 65 | `src/events/set_increased_revival_limits.rs` | 4 |
| `Events/ChoamFreeRevivalPrevention.cs` | 78 | `src/events/choam_free_revival_prevention.rs` | 5 |
| `Events/KarmaRevivalPrevention.cs` | 64 | `src/events/karma_revival_prevention.rs` | 4 |
| `Events/RecruitsPlayed.cs` | 55 | `src/events/recruits_played.rs` | 4 |
| `Model/RevivalCost.cs` | 24 | `src/revival_cost.rs` | 3 |
| `Model/LeaderState.cs` | 56 | `src/leader_state.rs` | 7 |
| `Model/Log/RevivalLog.cs` | 15 | `src/log/revival_log.rs` | 6 |

### G. Shipment and movement

| C# file | Lines | Rust module | `todo!`s |
|---|---:|---|---:|
| `Game_Move.cs` | 930 | `src/game_move.rs` | 43 |
| `Events/Shipment.cs` | 751 | `src/events/shipment.rs` | 68 |
| `Events/Move.cs` | 103 | `src/events/move.rs` | 3 |
| `Events/Caravan.cs` | 59 | `src/events/caravan.rs` | 3 |
| `Events/BeneGesseritAccompanies.cs` | 151 | `src/events/bene_gesserit_accompanies.rs` | 13 |
| `Events/BeneGesseritFlip.cs` | 88 | `src/events/bene_gesserit_flip.rs` | 6 |
| `Events/ChoamExtraMove.cs` | 72 | `src/events/choam_extra_move.rs` | 5 |
| `Events/ChoamMovePrevention.cs` | 92 | `src/events/choam_move_prevention.rs` | 8 |
| `Events/KarmaShipmentPrevention.cs` | 64 | `src/events/karma_shipment_prevention.rs` | 4 |
| `Events/SetShipmentPermission.cs` | 79 | `src/events/set_shipment_permission.rs` | 5 |
| `Events/GuildDelay.cs` | 50 | `src/events/guild_delay.rs` | 3 |
| `Events/PerformHmsMovement.cs` | 86 | `src/events/perform_hms_movement.rs` | 6 |
| `Events/KarmaHmsMovement.cs` | 99 | `src/events/karma_hms_movement.rs` | 6 |
| `Events/FlightUsed.cs` | 65 | `src/events/flight_used.rs` | 5 |
| `Events/FlightDiscoveryUsed.cs` | 55 | `src/events/flight_discovery_used.rs` | 4 |
| `Events/HMSAdvantageChosen.cs` | 71 | `src/events/hms_advantage_chosen.rs` | 5 |
| `Events/ChoamRemoveForce.cs` | 134 | `src/events/choam_remove_force.rs` | 10 |
| `Model/Intrusion.cs` | 27 | `src/intrusion.rs` | 3 |
| `Model/Log/ShipMoveLog.cs` | 69 | `src/log/ship_move_log.rs` | 0 |

### H. Battle: plans and resolution

| C# file | Lines | Rust module | `todo!`s |
|---|---:|---|---:|
| `Game_Battle.cs` | 1264 | `src/game_battle.rs` | 64 |
| `Events/Battle.cs` | 948 | `src/events/battle.rs` | 67 |
| `Events/BattleInitiated.cs` | 240 | `src/events/battle_initiated.rs` | 28 |
| `Events/BattleClaimed.cs` | 75 | `src/events/battle_claimed.rs` | 4 |
| `Events/BattleRevision.cs` | 59 | `src/events/battle_revision.rs` | 3 |
| `Events/BattleConcluded.cs` | 356 | `src/events/battle_concluded.rs` | 23 |
| `Events/TreacheryCalled.cs` | 118 | `src/events/treachery_called.rs` | 6 |
| `Events/LoserConcluded.cs` | 211 | `src/events/loser_concluded.rs` | 14 |
| `Events/Captured.cs` | 60 | `src/events/captured.rs` | 3 |
| `Events/Retreat.cs` | 125 | `src/events/retreat.rs` | 9 |
| `Events/TakeLosses.cs` | 152 | `src/events/take_losses.rs` | 9 |
| `Model/BattleOutcome.cs` | 88 | `src/battle_outcome.rs` | 4 |
| `Model/Fight.cs` | 25 | `src/fight.rs` | 2 |
| `Model/LossToTake.cs` | 17 | `src/loss_to_take.rs` | 1 |

### I. Battle: actions around the fight

| C# file | Lines | Rust module | `todo!`s |
|---|---:|---|---:|
| `Events/Voice.cs` | 181 | `src/events/voice.rs` | 7 |
| `Events/Prescience.cs` | 76 | `src/events/prescience.rs` | 5 |
| `Events/Thought.cs` | 80 | `src/events/thought.rs` | 7 |
| `Events/ThoughtAnswered.cs` | 86 | `src/events/thought_answered.rs` | 6 |
| `Events/Diplomacy.cs` | 88 | `src/events/diplomacy.rs` | 7 |
| `Events/PoisonToothCancelled.cs` | 49 | `src/events/poison_tooth_cancelled.rs` | 3 |
| `Events/PortableAntidoteUsed.cs` | 61 | `src/events/portable_antidote_used.rs` | 4 |
| `Events/RockWasMelted.cs` | 65 | `src/events/rock_was_melted.rs` | 4 |
| `Events/AuditCancelled.cs` | 105 | `src/events/audit_cancelled.rs` | 7 |
| `Events/Audited.cs` | 53 | `src/events/audited.rs` | 3 |
| `Events/FaceDanced.cs` | 202 | `src/events/face_danced.rs` | 12 |
| `Events/FaceDancerReplaced.cs` | 72 | `src/events/face_dancer_replaced.rs` | 6 |
| `Events/FaceDancerRevealed.cs` | 66 | `src/events/face_dancer_revealed.rs` | 3 |
| `Events/BeneGesseritBattleAnnouncement.cs` | 81 | `src/events/bene_gesserit_battle_announcement.rs` | 6 |
| `Events/ResidualPlayed.cs` | 71 | `src/events/residual_played.rs` | 4 |
| `Events/Planetology.cs` | 66 | `src/events/planetology.rs` | 5 |
| `Events/JuicePlayed.cs` | 110 | `src/events/juice_played.rs` | 5 |
| `Events/KarmaEcazDial.cs` | 58 | `src/events/karma_ecaz_dial.rs` | 3 |
| `Events/KarmaPrescience.cs` | 51 | `src/events/karma_prescience.rs` | 3 |
| `Events/TraitorDiscarded.cs` | 74 | `src/events/traitor_discarded.rs` | 6 |

### J. Faction specials: Karma, Nexus, Moritani, Ecaz, CHOAM, clairvoyance

| C# file | Lines | Rust module | `todo!`s |
|---|---:|---|---:|
| `Events/Karma.cs` | 303 | `src/events/karma.rs` | 10 |
| `Events/KarmaChoamDiscard.cs` | 82 | `src/events/karma_choam_discard.rs` | 7 |
| `Events/NexusPlayed.cs` | 678 | `src/events/nexus_played.rs` | 36 |
| `Events/TerrorPlanted.cs` | 164 | `src/events/terror_planted.rs` | 11 |
| `Events/TerrorRevealed.cs` | 272 | `src/events/terror_revealed.rs` | 19 |
| `Events/AllianceByTerror.cs` | 86 | `src/events/alliance_by_terror.rs` | 3 |
| `Events/AmbassadorPlaced.cs` | 96 | `src/events/ambassador_placed.rs` | 8 |
| `Events/AmbassadorActivated.cs` | 471 | `src/events/ambassador_activated.rs` | 36 |
| `Events/AllianceByAmbassador.cs` | 70 | `src/events/alliance_by_ambassador.rs` | 3 |
| `Events/ChoamDiscarded.cs` | 84 | `src/events/choam_discarded.rs` | 6 |
| `Events/ChoamEconomics.cs` | 56 | `src/events/choam_economics.rs` | 3 |
| `Events/ChoamKarmaPrevention.cs` | 78 | `src/events/choam_karma_prevention.rs` | 5 |
| `Events/ResourcesAudited.cs` | 68 | `src/events/resources_audited.rs` | 4 |
| `Events/ClairVoyancePlayed.cs` | 251 | `src/events/clair_voyance_played.rs` | 17 |
| `Events/ClairVoyanceAnswered.cs` | 119 | `src/events/clair_voyance_answered.rs` | 6 |
| `Events/AmalPlayed.cs` | 60 | `src/events/amal_played.rs` | 3 |
| `Events/MetheorPlayed.cs` | 95 | `src/events/metheor_played.rs` | 5 |
| `Events/ExtortionPrevented.cs` | 60 | `src/events/extortion_prevented.rs` | 4 |
| `Events/BeneGesseritPrediction.cs` | 72 | `src/events/bene_gesserit_prediction.rs` | 5 |
| `Events/RicheseRevealedNoField.cs` | 46 | `src/events/richese_revealed_no_field.rs` | 3 |
| `Model/ClairVoyanceQandA.cs` | 23 | `src/clair_voyance_q_and_a.rs` | 1 |

### K. Alliances, deals, discards, status

| C# file | Lines | Rust module | `todo!`s |
|---|---:|---|---:|
| `Game_Alliances.cs` | 186 | `src/game_alliances.rs` | 10 |
| `Game_Deals.cs` | 69 | `src/game_deals.rs` | 8 |
| `Game_Other.cs` | 218 | `src/game_other.rs` | 19 |
| `Game_Contemplate.cs` | 440 | `src/game_contemplate.rs` | 30 |
| `GameStatistics.cs` | 98 | `src/game_statistics.rs` | 5 |
| `Events/AllianceOffered.cs` | 80 | `src/events/alliance_offered.rs` | 4 |
| `Events/AllianceBroken.cs` | 52 | `src/events/alliance_broken.rs` | 3 |
| `Events/AlliancePassed.cs` | 60 | `src/events/alliance_passed.rs` | 0 (added in the v188 sync, ported directly) |
| `Events/AllyPermission.cs` | 161 | `src/events/ally_permission.rs` | 11 |
| `Events/DealOffered.cs` | 196 | `src/events/deal_offered.rs` | 13 |
| `Events/DealAccepted.cs` | 185 | `src/events/deal_accepted.rs` | 12 |
| `Events/DealRejected.cs` | 89 | `src/events/deal_rejected.rs` | 5 |
| `Events/Donated.cs` | 138 | `src/events/donated.rs` | 9 |
| `Events/ResourcesTransferred.cs` | 77 | `src/events/resources_transferred.rs` | 6 |
| `Events/CardTraded.cs` | 114 | `src/events/card_traded.rs` | 8 |
| `Events/Discarded.cs` | 69 | `src/events/discarded.rs` | 5 |
| `Events/DiscardedSearched.cs` | 83 | `src/events/discarded_searched.rs` | 7 |
| `Events/DiscardedSearchedAnnounced.cs` | 57 | `src/events/discarded_searched_announced.rs` | 4 |
| `Events/DiscardedTaken.cs` | 79 | `src/events/discarded_taken.rs` | 7 |
| `Events/EmperorDiscarded.cs` | 67 | `src/events/emperor_discarded.rs` | 5 |
| `Model/Deal.cs` | 82 | `src/deal.rs` | 5 |
| `Model/GameStatus.cs` | 870 | `src/game_status.rs` | 39 |
| `Model/Log/AsideLog.cs` | 32 | `src/log/aside_log.rs` | 3 |
| `Model/Log/MentatLog.cs` | 14 | `src/log/mentat_log.rs` | 0 |

## Bots (phase 2, not started)

The bots in `treachery-online/Bots` are not ported. Nothing tracks them yet except this list of C# changes made after the port's first snapshot, which the bot port must include. The current C# is the spec; the list only flags code that is newer than the engine port's review.

- v188 sync (C# change of Nov 7, see [csharp-sync.md](csharp-sync.md)): `Classic/Bot_Alliance.cs` gains `DetermineAlliancePassed` (wait while one of the bot's own alliance offers is open, or while any human has not acted or passed; otherwise pass), and `Classic/Bot.cs` calls it right after `DetermineAllianceOffered`. Bot games in the differ still run the C# bots, so this is exercised there already.

