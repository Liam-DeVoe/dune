# Behavior changes approved by the user

Step 4 of native-plan.md. The user reviewed the C# bugs the port copied on purpose (2026-11-11) and chose these to fix. Each entry below is copied from the review; locations are from before step 2 and may have moved.

## Shipping special forces back to reserves charges for them but leaves them on the planet

- Area: Shipment and movement
- What happens: A player shipping Fedaykin, Sardaukar or other special forces back to reserves (with the Guild's permission or the Guild Secret Ally) pays for them, but only the normal forces move. The special forces stay where they were.
- Intended: Move both kinds of forces back.
- When: The shipment form offers special forces for shipping back. Needs Guild as ally or the Guild Secret Ally Nexus card.
- Where: rust/engine/src/events/movement/shipment.rs:891
- Note: Players lose spice for nothing in a normal alliance.

## Bene Gesserit flip asks about one territory and flips another

- Area: Shipment and movement
- What happens: The flip prompt and log name the territory of the pending intrusion, but the forces flipped are those in the territory of the most recent shipment or move.
- Intended: Flip in the intruded territory.
- When: The two only differ if another move happens while an older intrusion is still waiting.
- Where: rust/engine/src/events/movement/bene_gesserit_flip.rs:58
- Note: Cheap: use the same territory in both places.

## Revival form keeps a chosen leader after none can be revived

- Area: Revival
- What happens: In the revival form, a previously picked leader stays selected when the list of revivable leaders becomes empty. The C# picker had the same gap.
- Intended: Clear the choice.
- When: Changing other inputs so no leader is revivable.
- Where: rust/view/src/actions/revival.rs:196
- Note: Validation still rejects the stale choice.

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

## Missing spaces: "receiving5" and "ChoamUsing"

- Area: Log and message text
- What happens: "Karama prevents Guild from receiving5" after a Karama-blocked shipment payment, and "ChoamUsing Karama" in CHOAM's discard message.
- Intended: Spaces.
- When: Karama against Guild income; CHOAM's Karama discard.
- Where: rust/engine/src/events/movement/shipment.rs:995, karma_choam_discard.rs:113
- Note: Text only.

## Setting an automation rule is described as "change ally permissions"

- Area: Log and message text
- What happens: The event's description says "X change ally permissions". It is not written to the game log.
- Intended: Describe the automation rule.
- When: Shows wherever event descriptions appear (history, undo list).
- Where: rust/engine/src/events/host/automation_configured.rs:90
- Note: Rarely seen; fix if touching the file.

## Other small text quirks without comments

- Area: Log and message text
- What happens: Leading spaces in some Nexus-card log texts and a lowercase "cunning" (hazards.md §11.8).
- Intended: Clean text.
- Where: rust/docs/hazards.md §11.8
- Note: Fix opportunistically.

## Bot speed changes are sent to every connected client

- Area: Server and client
- What happens: When a host changes bot speed, every client on the server gets the notice, and the web client stores it into whatever game it has open. The menu reads bot speed from the game view, so I don't expect a visible effect.
- Intended: Send to the game's players only.
- When: Any bot speed change.
- Where: rust/server/src/hub/game_management.rs:708
- Note: Cheap; avoids a cross-game leak of state.

## Starting a game with invalid settings reports success

- Area: Server and client
- What happens: If the host's start request is rejected, the seats are still reassigned and the host is told it worked, with no error shown.
- Intended: Return the error.
- When: The web client's form validation normally prevents it.
- Where: rust/server/src/hub/game_events.rs:63
- Note: Silent failure; easy to return the result.

## Deleted users stay logged in

- Area: Server and client
- What happens: After an account is deleted, its login token still works until the server restarts, and games it joined still list it.
- Intended: Log the user out everywhere and remove them from games.
- When: Account deletion.
- Where: rust/server/src/hub/other.rs:471
- Note: A deleted account should not keep working.

## A special force revived with Emperor's money doesn't count toward the one-per-turn limit

- Area: Revival
- What happens: If a player's only special force revival this turn is paid by Emperor, the game doesn't note that they revived a special force, so they can revive another one later in the turn.
- Intended: Count special forces paid by Emperor too.
- When: Emperor's ally with special forces (Fremen, for example) reviving in two steps.
- Where: rust/engine/src/events/revival/revival.rs:671
- Note: Breaks a revival limit. One condition to extend.

## Fremen can spend Karama to send a worm without choosing a territory

- Area: Requests the server accepts
- What happens: The Karama worm form reports no error before a territory is picked. Confirming then spends the Karama card and Fremen's special Karama power, and no worm appears. A hand-made request could also send the worm to a protected territory.
- Intended: Require a territory the worm can reach.
- When: Clicking confirm too early in the web client (rust/view/tests/other_forms.rs:403 shows the form accepts it).
- Where: rust/engine/src/events/blow/karma_monster.rs:51
- Note: A player can waste their Karama by accident.

## Karama shipment-prevention bot ignores opponents without an ally

- Area: Bots
- What happens: The bot only blocks shipments of a winning opponent whose spice plus ally's spice exceeds 2; with no ally the sum is treated as false.
- Intended: Count the opponent's own spice when there is no ally.
- When: Bots holding Karama.
- Where: rust/bots/src/bot/other.rs, `determine_karma_shipment_prevention`
- Done (0e82687): the opponent's spice plus their ally's, if any. Test: `karama_shipment_prevention_blocks_a_winning_opponent_without_ally` (bots/tests/behavior_changes.rs).

## Ally permissions without an ally are stored under "no faction"

- No visible effect; the entry is never read. Drop it in the step 3 cleanup (rust/engine/src/events/alliances/ally_permission.rs:150 before step 2).

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

## Bots change the game while deciding

- Area: Bots (native-plan.md step 4, not part of the review)
- What happened: the Rock Melter decision worked out the battle on the live plans, which logged "<Mirror Weapon> becomes a <card>" and set the Diplomat's card (known-issues.md).
- Done (0e82687): `ClassicBot::decide` takes `&Game`, so no decision can change the game. The Rock Melter decision uses `Battle::predict_battle_outcome` on copies of the plans. Engine change: rust/engine/src/events/battle/battle.rs splits `determine_battle_outcome` into the card swap, the recording of the Mirror Weapon and Diplomat (`activate_dynamic_weapons`), and a pure part shared with `predict_battle_outcome`. Tests: `deciding_never_changes_the_game` (every fixture, player and priority), `rock_melter_decision_with_a_mirror_weapon_leaves_the_game_alone` (bots/tests/behavior_changes.rs).
