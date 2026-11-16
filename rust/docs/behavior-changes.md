# Behavior changes approved by the user

Step 4 of native-plan.md. The user reviewed the C# bugs the port copied on purpose (2026-11-11) and chose these to fix. Each entry below is copied from the review; locations are from before step 2 and may have moved.

## Shipping special forces back to reserves charges for them but leaves them on the planet

- Area: Shipment and movement
- What happens: A player shipping Fedaykin, Sardaukar or other special forces back to reserves (with the Guild's permission or the Guild Secret Ally) pays for them, but only the normal forces move. The special forces stay where they were.
- Intended: Move both kinds of forces back.
- When: The shipment form offers special forces for shipping back. Needs Guild as ally or the Guild Secret Ally Nexus card.
- Where: rust/engine/src/events/movement/shipment.rs:891
- Note: Players lose spice for nothing in a normal alliance.
- Done (579e70d): the special forces go back too. Bene Gesserit advisors return as fighters, as when they go to reserves in other ways, and Richese's No-Field token leaves the planet unrevealed. Validation also refuses positive amounts and more forces than the sector holds. Before, a hand-made request could ship forces from reserves to the planet at half price this way, or create forces by shipping back more than were there. Test: `shipping_back_to_reserves_moves_special_forces_too` in engine/tests/rule_fixes.rs.

## Bene Gesserit flip asks about one territory and flips another

- Area: Shipment and movement
- What happens: The flip prompt and log name the territory of the pending intrusion, but the forces flipped are those in the territory of the most recent shipment or move.
- Intended: Flip in the intruded territory.
- When: The two only differ if another move happens while an older intrusion is still waiting.
- Where: rust/engine/src/events/movement/bene_gesserit_flip.rs:58
- Note: Cheap: use the same territory in both places.
- Done (579e70d): the flip uses the territory of the pending intrusion. A terror sneak attack is one way to get there, since it queues an intrusion without becoming the latest shipment or move. Test: `bene_gesserit_flip_happens_in_the_intruded_territory` in engine/tests/rule_fixes.rs.

## Revival form keeps a chosen leader after none can be revived

- Area: Revival
- What happens: In the revival form, a previously picked leader stays selected when the list of revivable leaders becomes empty. The C# picker had the same gap.
- Intended: Clear the choice.
- When: Changing other inputs so no leader is revivable.
- Where: rust/view/src/actions/revival.rs:196
- Note: Validation still rejects the stale choice.
- Done (579e70d): the comment this entry came from is in the Raise Dead form (`RaiseDeadPlayed`), not the Revival form, which already cleared the leader. The Raise Dead form now drops a leader that isn't among the revivable ones. Test: `raise_dead_drops_a_leader_that_cant_be_revived` in view/tests/revival_forms.rs.

## An invalid bot decision skips the rest of its options

- Area: Bots
- What happens: If a bot's first choice in a priority group fails validation, it doesn't try the other options in that group, and the server ends the bot's turn for that tick.
- Intended: Fall through to the next option.
- When: Whenever a bot produces an invalid event (hazards.md §16.1).
- Where: rust/server/src/hub/game_events.rs:396
- Note: Makes bots less likely to stall or pass where they had a valid move.
- Done (0e82687): the dispatch in rust/bots/src/bot/mod.rs (`first_decision!`) passes over a rejected decision and asks the next method. The server code it pointed at is gone; `Server::bot_turn` (rust/server/src/server/bots.rs) only sees valid events now. Test: `an_invalid_decision_falls_through_to_the_next_method` (bot/mod.rs).

## Bots don't count Bene Gesserit Worthless cards as Karama

- Area: Bots
- What happens: When estimating whether an opponent can Karama a shipment, the bot checks for Karama twice instead of Karama or Worthless (for Bene Gesserit).
- Intended: Include Worthless for Bene Gesserit.
- When: Bots against Bene Gesserit with the Worthless-as-Karama rule.
- Where: rust/bots/src/bot/knowledge.rs, `max_reinforced_dial_to`
- Note: Bot quality; one condition.
- Done (0e82687): `max_reinforced_dial_to` counts a known Worthless card as Karama for Bene Gesserit under the rule. Test: `worthless_cards_count_as_karama_for_bene_gesserit` (bots/tests/behavior_changes.rs).

## Smaller bot reasoning slips

- Area: Bots
- What happens: Dial estimates for an ally are computed for the bot itself (bot/battle.rs, `get_dial_needed`). Known traitors of an opponent use the bot's own told traitors (bot/battle.rs, `select_hero_for_battle`). Moving towards weak strongholds uses the first occupying opponent (hazards.md §18.5).
- Intended: Use the right player.
- When: Various bot decisions.
- Where: rust/bots/src/bot/battle.rs, `get_dial_needed` and `select_hero_for_battle`
- Note: Low priority; small effect on bot play.
- Done (0e82687): `get_dial_needed` estimates for `p` (its aggressor check, leader, cards and `best_voice` all use `p`); `select_hero_for_battle` takes known traitors from the opponent's own told traitors; `winnable_nearby_stronghold` estimates against the opponent it picked. Tests: `dial_needed_for_another_player_is_theirs`, `traitors_told_by_the_opponent_count_as_known_traitors`, `a_weak_stronghold_is_judged_against_the_opponent_picked_to_fight` (bots/tests/behavior_changes.rs).

## Harmless bot oddities

- Area: Bots
- What happens: A Mirror Weapon pick is re-drawn at random (bot/battle.rs, `chance_of_enemy_leader_dying`). A No-Field choice could take the maximum of an empty list and throw, which only skips that decision (bot/shipment.rs, `shipment_size`). Duplicate conditions (bot/movement.rs, `determine_moved_battalion`; bot/shipment.rs, `determine_shipment`). Bene Gesserit's prediction flips a coin between two identical choices (bot/factions.rs, `determine_bene_gesserit_prediction`).
- Intended: Same.
- Where: rust/bots/src/
- Note: No meaningful effect on play.
- Done (0e82687): a Mirror Weapon pick is kept, with an even chance of killing (test: `a_mirror_weapon_pick_is_kept`, bot/battle.rs); the No-Field sizing falls back to the biggest No-Field instead of the maximum of an empty list (that branch can't be reached: the forces are capped at the biggest No-Field, so a big enough one always exists); the duplicate conditions are gone (the one in `determine_shipment` was already gone); the prediction no longer flips a coin first (test: `bene_gesserit_prediction_draws_no_coin`). The coin flip changes Bene Gesserit's draws from the first turn, so every game with a Bene Gesserit bot plays differently from then on.
- Also done (c0f4762), found by the harness: keeping the Mirror Weapon pick let a bot hang while deciding. `TreacheryCard::countered_by` (rust/engine/src/cards/treachery_card.rs) asks the weapon a Mirror Weapon copies, so with a Mirror Weapon on both sides it asked itself forever. It now counts that as countered. Only bots call it. The old bots could reach it too, more rarely, through their second pick. Test: `a_mirror_weapon_against_a_mirror_weapon_is_countered` (bots/tests/behavior_changes.rs).

## Missing spaces: "receiving5" and "ChoamUsing"

- Area: Log and message text
- What happens: "Karama prevents Guild from receiving5" after a Karama-blocked shipment payment, and "ChoamUsing Karama" in CHOAM's discard message.
- Intended: Spaces.
- When: Karama against Guild income; CHOAM's Karama discard.
- Where: rust/engine/src/events/movement/shipment.rs:995, karma_choam_discard.rs:113
- Note: Text only.
- Done (bfef920): "from receiving 5" and "CHOAM using Karama". Test: skin/tests/log_text.rs.

## Setting an automation rule is described as "change ally permissions"

- Area: Log and message text
- What happens: The event's description says "X change ally permissions". It is not written to the game log.
- Intended: Describe the automation rule.
- When: Shows wherever event descriptions appear (history, undo list).
- Where: rust/engine/src/events/host/automation_configured.rs:90
- Note: Rarely seen; fix if touching the file.
- Done (bfef920): "X add an automation rule: <the rule's description>", "X remove an automation rule", "X change an automation rule". A refused event's message quotes this text too. Test: skin/tests/log_text.rs.

## Other small text quirks without comments

- Area: Log and message text
- What happens: Leading spaces in some Nexus-card log texts and a lowercase "cunning" (hazards.md §11.8).
- Intended: Clean text.
- Where: rust/docs/hazards.md §11.8
- Note: Fix opportunistically.
- Done (bfef920): Nexus card plays no longer start their text with a space or repeat the card type. C# passed "Secret Ally" or "Cunning" as the first word, so the log read "Nexus Secret Ally to Secret Allyget this card for free" and "Nexus Cunning to cunningto ride from any territory on the planet". Now it reads "Nexus Secret Ally to get this card for free". The other §11.8 quirks have their own `// C# parity:` comments and aren't part of this entry. CHOAM's " The Economics Token flips ..." (Game_Contemplate) isn't a Nexus text and is unchanged. Test: skin/tests/log_text.rs.

## Bot speed changes are sent to every connected client

- Area: Server and client
- What happens: When a host changes bot speed, every client on the server gets the notice, and the web client stores it into whatever game it has open. The menu reads bot speed from the game view, so I don't expect a visible effect.
- Intended: Send to the game's players only.
- When: Any bot speed change.
- Where: rust/server/src/hub/game_management.rs:708
- Note: Cheap; avoids a cross-game leak of state.
- Done (6b4368b): sent only to the connections following the game. Test: `bots_take_the_empty_seats` in server/tests/play.rs.

## Starting a game with invalid settings reports success

- Area: Server and client
- What happens: If the host's start request is rejected, the seats are still reassigned and the host is told it worked, with no error shown.
- Intended: Return the error.
- When: The web client's form validation normally prevents it.
- Where: rust/server/src/hub/game_events.rs:63
- Note: Silent failure; easy to return the result.
- Done in the server rewrite (step 3, see server.md): a refused `EstablishPlayers` returns its validation message and leaves the seats alone. 6b4368b adds a test, `starting_with_invalid_settings_is_refused` in server/tests/play.rs.

## Deleted users stay logged in

- Area: Server and client
- What happens: After an account is deleted, its login token still works until the server restarts, and games it joined still list it.
- Intended: Log the user out everywhere and remove them from games.
- When: Account deletion.
- Where: rust/server/src/hub/other.rs:471
- Note: A deleted account should not keep working.
- Done (6b4368b): the server rewrite already ended the user's sessions. Now they also leave every game they joined, as with `RequestLeaveGame`, and their connections stop following those games. Test: `users_delete_themselves` in server/tests/admin.rs.

## A special force revived with Emperor's money doesn't count toward the one-per-turn limit

- Area: Revival
- What happens: If a player's only special force revival this turn is paid by Emperor, the game doesn't note that they revived a special force, so they can revive another one later in the turn.
- Intended: Count special forces paid by Emperor too.
- When: Emperor's ally with special forces (Fremen, for example) reviving in two steps.
- Where: rust/engine/src/events/revival/revival.rs:671
- Note: Breaks a revival limit. One condition to extend.
- Done (579e70d). Test: `special_force_paid_by_emperor_counts_toward_the_limit` in engine/tests/rule_fixes.rs.

## Fremen can spend Karama to send a worm without choosing a territory

- Area: Requests the server accepts
- What happens: The Karama worm form reports no error before a territory is picked. Confirming then spends the Karama card and Fremen's special Karama power, and no worm appears. A hand-made request could also send the worm to a protected territory.
- Intended: Require a territory the worm can reach.
- When: Clicking confirm too early in the web client (rust/view/tests/other_forms.rs:403 shows the form accepts it).
- Where: rust/engine/src/events/blow/karma_monster.rs:51
- Note: A player can waste their Karama by accident.
- Done (579e70d, 2ecf60f): `KarmaMonster` is refused without a territory ("No territory selected"), in a territory protected from the worm ("Selected territory is protected", as for `FremenSentMonster`) or elsewhere outside `valid_targets` ("Invalid territory"). The form starts with no territory (it used to start with territory 0, the Polar Sink) and shows the first message until one is picked. Tests: `karama_worm_needs_a_territory` in engine/tests/rule_fixes.rs, `dependent_inputs` in view/tests/other_forms.rs.

## Karama shipment-prevention bot ignores opponents without an ally

- Area: Bots
- What happens: The bot only blocks shipments of a winning opponent whose spice plus ally's spice exceeds 2; with no ally the sum is treated as false.
- Intended: Count the opponent's own spice when there is no ally.
- When: Bots holding Karama.
- Where: rust/bots/src/bot/other.rs, `determine_karma_shipment_prevention`
- Done (0e82687): the opponent's spice plus their ally's, if any. Test: `karama_shipment_prevention_blocks_a_winning_opponent_without_ally` (bots/tests/behavior_changes.rs).

## Ally permissions without an ally are stored under "no faction"

- No visible effect; the entry is never read. Drop it in the step 3 cleanup (rust/engine/src/events/alliances/ally_permission.rs:150 before step 2).
- Done (579e70d): nothing is stored without an ally. Test: `ally_permissions_without_an_ally_store_nothing` in engine/tests/rule_fixes.rs.

## Bot shipment sizing prices normal forces as special forces

- Area: Bots
- What happens: When adding normal forces to a planned shipment, the bot checks the cost of one more special force instead, so it can ship too few or too many.
- Intended: Price one more normal force.
- When: Every bot shipment that adds normal forces.
- Where: rust/bots/src/bot/shipment.rs, `ForceSizing::fill`
- Done (0e82687): prices one more normal force. No game changes: a shipment's cost depends only on the total number of forces, so both prices are the same. Test: `shipment_sizing_prices_the_normal_forces_it_adds` (bots/tests/behavior_changes.rs).

## Bene Gesserit bot never voices a useless defense against poison

- Area: Bots
- What happens: The condition for finding a projectile defense that won't stop poison is "is a projectile defense and is not a projectile defense", so that voice is never chosen.
- Intended: Probably "is a projectile defense and not a poison defense".
- When: Bene Gesserit bot with a poison weapon.
- Where: rust/bots/src/bot/factions.rs, `voice_against_defenses`
- Done (0e82687): a known projectile defense that isn't a poison defense is forced. Test: `bene_gesserit_voices_a_useless_defense_against_poison` (bots/tests/behavior_changes.rs).

## For the step 3 cleanup (no visible effect, simplify if the behavior check stays clean)

- Setup briefly enters deck customizing or faction trading before the real next phase (rust/engine/src/game/setup.rs:73 before step 2).
- End of storm runs its cleanup twice when storm losses are taken first; the comment saying it skips the storm report is wrong (rust/engine/src/game/storm.rs:219, :223 before step 2).
- Done (579e70d): setup goes straight to its next phase. The extra entry only removed deals ending in `CustomizingDecks` or `TradingFactions`, which the deal form never offers.
- Done (579e70d), partly: without storm losses the phase now ends once instead of twice in a row, which did nothing the second time. With storm losses it still ends before the losses and again after them, because the second time can end Karama preventions made while losses are taken. The comment now says what happens: `TakeLosses` comes back to `end_storm_phase`, and the storm report follows.

## Bots change the game while deciding

- Area: Bots (native-plan.md step 4, not part of the review)
- What happened: the Rock Melter decision worked out the battle on the live plans, which logged "<Mirror Weapon> becomes a <card>" and set the Diplomat's card (known-issues.md).
- Done (0e82687): `ClassicBot::decide` takes `&Game`, so no decision can change the game. The Rock Melter decision uses `Battle::predict_battle_outcome` on copies of the plans. Engine change: rust/engine/src/events/battle/battle.rs splits `determine_battle_outcome` into the card swap, the recording of the Mirror Weapon and Diplomat (`activate_dynamic_weapons`), and a pure part shared with `predict_battle_outcome`. Tests: `deciding_never_changes_the_game` (every fixture, player and priority), `rock_melter_decision_with_a_mirror_weapon_leaves_the_game_alone` (bots/tests/behavior_changes.rs).

## Order of spice, forces and other lists on the board

- Area: views, the log and the bots (wave 2a of [rewrite-plan.md](rewrite-plan.md))
- What happened: thirteen maps and sets of the game state copied the order of .NET's `Dictionary` and `HashSet` (`NetDictionary` and `NetHashSet` in dotnet-compat). After a removal, the next new entry took the freed slot, so a bid or token added later could be listed before older ones.
- Now: spice, discoveries, stronghold owners and each player's forces are in map order (by location id), and so is the board view's list of forces (`Game::forces_on_planet_excluding_empty_locations`). Before, the board listed forces player by player. Terror tokens, ambassadors, occupied homeworlds, bids, the Guild's shipment permissions, recently discarded cards, Tleilaxu's early revival offers, known cards and revealed traitors are in the order they were added, and an entry that is removed and added again goes last. Each field's comment says why it has its order.
- Effect: players see these lists in a different order, on the board and in the bidding panel, and spice collection logs one line per location in map order. A bot that takes the first of several equal options can take a different one, for example the spice location it ships to. No rule depends on these orders, with two exceptions that stay deterministic: the Moritani betrayal picks a random terror token by its index in the territory's tokens, and collection queues the spice divisions of shared locations in map order.
- Tests: `board_lists_in_map_order` (view/tests/view_core.rs), `moritani_betrayal_picks_in_the_order_tokens_were_placed` (engine/tests/pkg_j_faction_specials.rs).
- Harness (seed 1, against the shared reference): every one of 200 bots and 200 fuzz games differs, and every first difference is a view. Each game was recorded by the reference and replayed through builds of the branch with fields put back on `NetDictionary`/`NetHashSet`. With all thirteen put back, every game matches, so nothing else changed. With one put back, the field whose revert moves the first difference later explains it: forces 200 bots and 197 fuzz games, bids 2 fuzz, spice 1 fuzz. With the forces put back, the next first differences are bids (146 bots, 123 fuzz), spice (40, 59), stronghold owners (1, 10), discoveries (3, 1) and ambassadors (0, 1), and 9 bots and 3 fuzz games match. In bots game 76, spice and discoveries change order at the same event, so neither alone moves it (`observe first` shows both). The other seven fields explain none of these differences. Their order changes only after a removal, which these games don't reach before a difference of the fields above.

## Bugs found while doing the fixes above

Not part of the review. Each has an escapes.md entry with the details.

- Richese's cards were in its cache and also in the treachery deck under `Rule::RicheseTreacheryCards`, so one could be sold twice. A card drawn from the deck now leaves the cache, and one Richese sells leaves the deck (`Game::drop_copy_in_richese_cache`, `drop_copy_in_treachery_deck`). Test: `a_richese_card_taken_from_the_deck_or_the_cache_leaves_the_other` (engine/tests/rule_fixes.rs).
- `Bid` refuses a pass that is also a Karama bid, and a Karama bid without a Karama card of the bidder's ("You can't pass and use Karama in the same bid", "Invalid Karama card"). Both threw halfway before. A new main phase drops any intrusion still queued. Tests: `a_karama_bid_needs_a_karama_card_and_no_pass`, `an_intrusion_left_over_from_an_earlier_phase_is_dropped` (engine/tests/rule_fixes.rs).
- The Guild bot's shipment back to reserves is a `ShipmentBack`, so validation accepts it and the bot ships back. Test: `the_guild_ships_a_stranded_battalion_back_to_reserves` (bots/tests/behavior_changes.rs).
- The Testing Station decision counts the bot's own and its ally's forces lost to the storm. Test: `the_testing_station_counts_the_bots_own_storm_losses` (bots/tests/behavior_changes.rs).
- `Shipment` refuses an ally contribution larger than the spice the ally has or has offered (`resources_your_ally_can_pay`), with "Your ally won't pay that much", as `Bid` and `Battle` already did. Before, the ally could go below zero spice. The web form and the bots never asked for more, so only hand-made requests and replayed logs change. Test: `an_ally_pays_for_a_shipment_only_spice_it_has_and_offered` (engine/tests/rule_fixes.rs).

## Random number generator

- Area: the whole game, and the bots (native-plan.md step 4, approved by the user 2026-11-13)
- What happened: every random draw (shuffles, dealt cards and traitors, the storm deck, random picks, the bots' dice) came from a copy of .NET's `System.Random` (`NetRandom` in dotnet-compat), through copies of the C# selection helpers (`RandomOrDefault`, `HighestOrDefault` and the like, in `engine/src/compat/random_ext.rs`) that kept .NET's culture-sensitive key order and called each selector twice.
- Now: a ChaCha8 generator from `rand_chacha`, seeded with the game's seed (the `Seed` of `EstablishPlayers`), and rand's sampling (`random_range`, `SliceRandom::shuffle`, `IteratorRandom::choose`). Bots get the same kind of generator with their own seeds. The helpers are `random_item`, `random_highest`, `random_lowest` and `random_of_lowest_n` in `engine/src/random.rs`; they compute each key once. [engine-conventions.md](engine-conventions.md#randomness) says why ChaCha8 and what would change its draws.
- Effect: every game draws different numbers from the same seed, so a game saved before plays out differently when its events are replayed. `Game::LATEST_VERSION` is 189, so the engine refuses older savegames ("Game version 188 is not supported. Only version 189 is supported.") instead of replaying them into a wrong state. The server leaves such games out at startup with a "no longer loads" log line and keeps them in the database (`opens_a_database_written_by_the_csharp_server` in server/tests/persistence.rs checks a version-188 game). The web client sends 189 at login.
- The savegame fixtures (`scripts/fixtures`) and the recorded games the tests replay were carried over with `observe refit` ([native-harness.md](native-harness.md#refitting-recorded-games)).
- Tests: `engine/tests/random.rs` (each helper picks every allowed item about equally often, the same seed gives the same game), `deck_shuffle_and_random_item` in engine/tests/static_data.rs (pins the streams), and `clean_replays_pass_and_recording_is_deterministic` in observe/tests/harness.rs (a whole bot game recorded twice is the same).

## Logged lists no longer change after the fact

- Area: log text (rewrite-plan.md 2c; the owner chose the snapshot on 2026-11-15, section 4 question 2)
- What happened: three messages showed a list from the game as it was when the log was drawn, not as it was when the message was logged, as the C# client did. The engine marked them with `LiveList`, and the skin and the views read the list from the game again on every render. They are the visitor's hand that the Atreides ambassador shows to Ecaz (`AmbassadorActivated`, "X own: ..."), the cards CHOAM's audit shows (`AuditCancelled`, "CHOAM see: ..."), and a bot's discarded traitors when it carries out a "Tell which traitors were discarded" deal (`DealAccepted`, "X discarded: ..."). So a hand seen in turn 3 showed the cards held in turn 5, and an old audit showed the cards of the latest one.
- Now: each of these messages holds a copy of the list taken when it is logged. `LiveList` is gone, and so are the renderers that took the game for it (`Skin::describe_in`, `format_in`, `join_in`, `message_to_string_in`, `message_part_to_string_in`, and `describe_in` on the view's `Describer`).
- Why: a message that reads the game when it's rendered can't be plain data, which wave 3b needs.
- Test: `engine/tests/logged_lists.rs` logs each of the three messages, changes the list, and checks that the message still holds the old contents.
- Harness (200 bots and 200 fuzz games, seed 1, against the shared reference, replayed by a build with the text fixes reverted): 398 games show no difference. Fuzz games 127 and 196 differ, both in the "X own: ..." line the Atreides ambassador logs for Ecaz. In 127 the line, read back after an undo, still shows a Trip to Gamont the visitor has since lost. In 196 Ecaz's log no longer gains the Stunner the visitor received later with a `Donated`.

## Typos and grammar in log, status, form and skin texts

- Area: log and message text (rewrite-plan.md 2c). Most were copied from C# on purpose and marked `// C# parity`.
- Log:
  - "N cards were shuffled from Spice discard piles A en B into a new deck." → "... A and B ..."
  - "Discovery were removed from the game" → "Discovery cards were removed from the game"
  - "Shai-Hulud appears a 21th time" → "21st" (also "22nd", "23rd"; 11 to 13 keep "th")
  - "X play  a Karama card" → "X play a Karama card"
  - "X use an Ornithopter to  move an additional group of forces " and "... to  gain movement speed" → one space, no trailing space
  - "X use the Flight discovert token to gain movement speed" → "discovery token"
  - "Richese put select a Silent auction" → "Richese select a Silent auction"
  - The Guild Nexus card's refund: "Treachery.Shared.Playerget 3 from the Bank" → "Guild get 3 from the Bank" (the faction that played the card)
  - A won card: "X get card 1 for 7 Spice(7 Spice)  → Y get 7 Spice" → "X get card 1 for 7 Spice (7 Spice) → Y get 7 Spice", and a Black Market sale "X get card  for 2 Spice" → "X get card for 2 Spice"
  - Deals: "Share prescience of the top Spicecard" → "top Spice card"
  - Suk Graduate: "Suk Graduate rescues  and 3 to reserves" → "Suk Graduate rescues 3 to reserves" (the " and " now only joins forces kept on site and forces sent to reserves)
  - "X force Y to reveal amount of weapons, defenses and Spice" → "reveal the amount of"
  - Face dancers: "X replace 3 forces by their own" → "with their own"
  - Bene Gesserit Nexus card: "be able to flip advisor to fighters" → "advisors to fighters"
- Status:
  - "You may now select a card to sell on the Harkonnen Markt, or pass." → "on the Black Market" (it's Richese's)
  - "Waiting factions to discard..." → "Waiting for factions to discard..."
  - "Factions are voting on about a Nexus..." → "Factions are voting on a Nexus..."
  - "Please conclude the audit when done inspecting your opponents cards." → "opponent's cards"
  - "You may now replace opponent forces by your own." → "with your own"
- Forms: the Harkonnen Nexus questions "then shufle one of your traitors" → "shuffle", and the Bene Gesserit one "flip advisor to fighters" → "advisors".
- Refusals:
  - "Place 3 or less forces" → "or fewer forces"
  - "Invalid decision" and "You need to decide to either force keeping or discarding cards" no longer end with the kept card's name glued on
  - "Select the source of in total 3 forces" → "Select the sources of 3 forces in total"
- Skin: "You are not ad administrator" → "an administrator"; the house rule "Alliances are  broken at the start of each alliance phase" → one space; card texts "cause both factions to loose the battle" → "causing both factions to lose the battle" (Lasgun), "Instead of normal storm moved" → "normal storm movement" (Weather Control), "The revival limits is increased to 7" → "limit is" (Recruits); the faction sheets "as long has" → "as long as", "If that factions wins" → "faction wins", "your revive at half price" → "you revive", "you split the it" → "split it", "one of your leader (selected randomly)" → "leaders". The card texts are also fixed in the client's copy (web/public/skin/default.json). Skin JSON keys don't change, so custom skins load as before.
- Harness (200 bots and 200 fuzz games, seed 1, against the shared reference, with `docs/allow-2c-log-text.txt`): every game but 5 bots games differs, almost always from the first view, because the faction sheets in every view changed. Each first difference is at the same place with the snapshot change reverted and goes away with the text fixes reverted, so all 395 are these texts. The views also show the log, so the allow file can't hide log text there.
- Left alone: "X is a Y facedancer!", "CHOAM The Economics Token flips to ...", and the missing space between a number and a force name in plain text ("3forces"), which the client draws as separate tokens.
- Tests: `skin/tests/log_text.rs` (the Ornithopter, discovery token, Karama, deal and ordinal texts); `natural_numbers` in engine/tests/pkg_d_storm_blow.rs (ordinals); the snapshots in `skin/tests/golden`, `view/tests/golden/log_lines*` and web/public/static/lobby.json were rewritten with their update modes, keeping each unchanged line as it was, and each changed line was read. The new log lines' `dom` texts were filled with `npm run check:rich-text -- --fill` and read.
## Pathfinding from a stale cache

- Area: Shipment and movement (rewrite-plan.md change 2b)
- What happened: `Map::find_neighbours`, which finds where forces can move, cached its answers until the history grew. Within one event, or after an event that failed halfway, it could answer for the storm, forces and blocked territories as they were before. escapes.md, "Paths found for an earlier state", lists when.
- Now: every call works from the current state, and `Map` has no cache.
- Effect: none seen. No game in 200 bots and 200 fuzz games (seed 1) got a different answer from the cache, and with the cache put back every failing game of the harness run fails the same way.
- Tests: `engine/tests/pathfinding.rs`.

## Reviving a leader who isn't dead

- Area: Revival
- What happened: `Revival` and `KarmaFreeRevival` accepted a leader who was alive. The revival put them in the tanks, with a time of death of 0, so the tanks listed them before every leader who was killed. A Tleilaxu revival of their own living leader just cost spice. The web form and the bots offer only dead leaders, with one exception: `valid_revival_heroes` counts the Tleilaxu's own living leaders among the gholas a faction can take back, when an early revival offer allows it, so the Tleilaxu bot picked one in fuzz games 48 and 73 (seed 1).
- Now: both events refuse a leader who is alive, with "You can't revive a leader who isn't dead". The check comes before the other checks of the event. A faction other than the Tleilaxu can still take back its living leader from the Tleilaxu with a `Revival`.
- The per-game leader death count (`game.counters.leader_deaths`) starts at 0 instead of 1. It started at 1 only so that leaders made dead this way came first, and it orders the tanks the same way otherwise.
- Effect: 46 of 200 fuzz games (seed 1) first differ at such a revival. In 22 the reference accepted it, in 22 it refused it for another reason and the message changes, and in 2 (games 48 and 73) the Tleilaxu bot's decision changes. With the fix reverted all 46 pass. The 200 bots games (seed 1) are unchanged. Three recorded traces in `rust/testdata/games` and the bot tests' knowledge game 77-00004 contain such a revival, so their snapshots change from that event on.
- Tests: `only_a_dead_leader_can_be_revived` and `a_living_ghola_can_be_taken_back` in engine/tests/rule_fixes.rs, `games_on_one_thread_count_leader_deaths_separately` in engine/tests/pkg_f_revival.rs.

## Event values that name no enum member

- Area: event JSON from clients, savegames and the fuzzer (rewrite-plan.md change 2d, in progress)
- What happened: every enum was an open `i32`, so an event could hold any number in an enum field, such as `AutomationConfigured` with `"Action":70`. The engine accepted such events, and most of them did nothing or logged the number. The fuzzer sends them all the time. No client or bot does.
- Now: the closed enums (all but `Faction`, `Phase`, `MainPhase`, `ShipmentPermission` and the four lobby enums, see engine-conventions.md, "Enums") refuse a number that names no member when the event is decoded: "70 is not a valid ItemAction". The server answers such an event like any other malformed one. 0 still means the C# `None` member where the enum had one.
- Also changed, on paths only an unvalidated or malformed event reaches: a Clairvoyance parameter that names no card type fails to parse, like other malformed parameters. A Moritani Nexus betrayal in a territory without terror, which validation refuses, no longer adds a `None` token to the unplaced terror tokens. `AmbassadorPlaced`, `AmbassadorActivated`, `TerrorPlanted` and `TerrorRevealed` executed without an ambassador or terror type fail with a Game Error instead of storing `None`. A skin dictionary key that is a number naming no member is refused when the skin loads.
- Effect (seed 1, against the shared reference): all 20 fuzz games tried first differ at an `AutomationConfigured` whose `Action` names no `ItemAction` member, which the reference accepted. Five sampled games refuse actions 30, 70, 80, 120 and 120. Against a reference whose reader refuses the same undefined values (below), 20 bots and 20 fuzz games pass, so nothing else changed.
- That reference is a copy of the shared reference's sources in `~/2d-ref/rust`, whose `json/mod.rs` refuses a number that names no member for the enums listed in `TREACHERY_CLOSED_ENUMS` (comma-separated). The patch is `~/2d-ref/closed-enums.patch`. It was built with `tools/build-native-ref.sh --source ~/2d-ref/rust --out ~/2d-ref/bin` and is run as `TREACHERY_CLOSED_ENUMS=<the closed enums> observe live --reference ~/2d-ref/bin/observe`.
- Tests: `view/tests/trace_events_decode.rs` checks that every accepted event of the recorded test games still decodes. Seven `AutomationConfigured` steps with an undefined `Action` in two of those games are marked refused (`tools/drop-undecodable-steps.py`). The event JSON snapshot (`engine/tests/golden/events.jsonl`) records the cases with undefined values as refused.

## The card swap after an auction told everyone who holds a Nexus card

- Area: Bidding, Nexus cards (approved by the user 2026-11-15)
- What happened: after a card was sold, the engine entered `ReplacingCardJustWon` whenever the winner held any Nexus card, and the server's pause waited only when the winner held the Ixian Nexus as a secret ally (`nexus_allows_replacing_bought_cards`). Nexus cards are private, but the phase shows in the status and the log, and the pause shows in everyone's status card, so every auction told the table whether the winner held a Nexus card, and which.
- Now: the engine enters the step on public grounds only, `Game::winner_might_replace_with_nexus`: Nexus cards are in play and Ixian isn't, so anyone might hold the Ixian Nexus. Then every winner gets the step, and the pause waits for them (`pause_rules.rs`). Ixian's ally still gets the step when the alliance allows the swap, as before. Who may actually swap is unchanged. In games with Nexus cards and no Ixian, each auction costs the winner one more pause.
- Effect: every recorded game with Nexus cards and no Ixian gains an `EndPhase` after each sale. 27 recorded games replay differently in the two builds. 26 of them (in `rust/testdata/games` and `rust/view/tests/data`) and the bot knowledge game 00008 (with its probes) were refitted (native-harness.md, "Refitting recorded games"), each with `--old` from the base commit, and end in the same phase and turn as before. The fixtures `nexus-prescience` and `nexus-prescience-base` were refitted the same way. `mc-discard` and `nexus-moritani` already fail validation at the base commit for other reasons, so refitting them lost their endings; they keep their events instead, with an `EndPhase` inserted wherever the step is new, and end exactly as before. Recorded games that replay the same in both builds were left alone. Snapshots were rewritten with their update modes.

## The Emperor's homeworld abilities counted Salusa Secundus as Kaitain

- Area: Homeworlds (approved by the user 2026-11-15)
- What happened: the Emperor's Kaitain abilities checked whether any Emperor homeworld was at the threshold. With Salusa Secundus high and Kaitain low, the Emperor could discard cards for 2 spice each at the end of Bidding, which only Kaitain's high threshold gives. With Salusa Secundus low and Kaitain high, the Emperor's revival limit rose by one, which only Kaitain's low threshold gives (Salusa Secundus at low threshold gives no free revival). The C# client did the same (`Player.HasHighThreshold()` without a world).
- Now: `EmperorDiscarded` needs Kaitain at high threshold (applicable_events.rs), and the revival limit's low-threshold bonus needs Kaitain at low threshold for the Emperor (`get_max_revivals`, revival.rs), as the free revival already did. The pause at the end of Bidding waits for the Emperor only with Kaitain high and cards in hand.
- Every other `has_any_high_threshold` and `has_any_low_threshold` call names a faction with one homeworld, so it was right already.
||||||| 3c20931

## Voice before Prescience

- Area: battle (approved by the user 2026-11-15)
- What happened: Voice and Prescience could be used in either order. The forms only showed "Note: Voice should be used before Prescience." The C# engine refused Voice after Prescience in game versions 108 to 177, and from 178 on had no check. The rulebook FAQ (p. 22, `web/assets/docs/Dune-Rulebook.pdf`) says "The Bene Gesserit Voice must always be used before the Atreides Prescience."
- Now: Prescience waits while Voice may still come from the Bene Gesserit, or from their ally when Voice is shared (`Voice::voice_pending`). Either of them can end the wait with Voice or with the new `VoiceDeclined` event ("Bene Gesserit don't use Voice"). One decline closes Voice for both. `VoiceDeclined` is offered only while someone could use Prescience. While Prescience waits, a player who could otherwise use it sees a "Use Prescience" form without a button, saying which factions it waits for (the view's `waitingActions`). While it is offered, the battle status lists the player who may decline among those waited for, and tells them "Please decide whether to use Voice. Prescience waits for it." when they aren't fighting.
- A secret ally with the Bene Gesserit Nexus card isn't waited for and isn't offered `VoiceDeclined`, since either would show everyone that they hold the card. Instead they may not Voice once Prescience has been used.
- Not gated: the Atreides Karma power to see the whole battle plan (`KarmaPrescience`), and the Atreides Nexus card's Prescience. The Karma power shows the plan as it changes, so a later Voice can't make what it saw wrong.
- Bots: the Bene Gesserit bot declines when it won't Voice (it only Voices in battles it fights). The Atreides bot still holds back its battle plan until it has used Prescience, and estimates as if it will (`Prescience::prescience_right` ignores the wait).
- Effect: `Game::LATEST_VERSION` is 190, so version 189 games no longer load. The two note fields left the Voice and Prescience forms (`voiceNote`, `prescienceNote`).
- Fixtures: 12 Prescience events in 10 fixtures came before a Voice that could still come. Where the Bene Gesserit Voiced later in the battle, the Voice now comes first. In `retreat` and `worm` this also drops an Atreides plan without cards that the Voice used to revoke. Where nobody Voiced (once in `mc-ended`, once in `worm`), a Bene Gesserit `VoiceDeclined` comes first. Each fixture ends in the same phase and turn, with the same players' state, as before, and its log has the same lines apart from the order within a battle and the two new declines (`view/tests/golden/log_lines.jsonl.gz`). The recorded test games were left as they are; one fuzz step, a Prescience in fuzz-2026-00020, is now refused, and its skin snapshot records that.
- Tests: `engine/tests/voice_before_prescience.rs`.
