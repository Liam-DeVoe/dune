# Known open issues

Problems that are known and not fixed yet. Reviewers: don't report these again, but do report any new instance of one of these classes that isn't listed under it.

## No test reaches the Retreat form

A loser may retreat only if their leader survives the battle and has the Diplomat skill. The `retreat` fixture reached that state until game version 189 changed every random draw. `observe refit` carried it over, but with different skills dealt it no longer ends in the retreat (it ends in the battle report of turn 8; a second refit that asked for Harkonnen's `Retreat` at the end, with 150 seeds, didn't get there either), and none of 700 bot games and 200 fuzz games recorded at version 189 reach it. `battle_forms_in_fixtures` (view/tests/battle_forms.rs) no longer asserts that the Retreat form is reached. Fix: a fixture or built state where the loser's surviving leader is a Diplomat.

## Found by 2c and left alone

- "X is a Y facedancer!" and "CHOAM The Economics Token flips ..." read badly.

## Checked and correct

2026-11-15: at low threshold, Moritani reveals a terror token only when at least 3 forces enter. The Grumman card says "Low (0-7): ... You can only reveal terror tokens when at least 3 forces enter" (web/assets/art/GrummanCard.gif), and `check_terror_triggered` does that.

2026-11-18: a Bene Gesserit bot's predicted win does change its battle plan. In `best_voice` (`rust/bots/src/bot/factions.rs`) the prediction doesn't change the voice chosen, but it starts the plan's two flags `player_hero_will_certainly_survive` and `opponent_hero_will_certainly_be_zero` true. Battle planning reads them in `chance_of_my_leader_surviving` and `chance_of_enemy_leader_dying` (`rust/bots/src/bot/battle.rs`) as certainty, a chance of 1 instead of 0.5, that the bot's leader survives the voiced defense and the opponent's dies to the voiced weapon. The C# version did the same; the known-issues entry that said nothing read the flags was wrong.

## Quirks kept from the C# version

Behavior that looks wrong or arbitrary and has no reason except that the C# version did the same. Each is unchanged for now and marked in the code with `Known quirk (known-issues.md)`. Fixing one changes what players see, so it needs the owner's approval and an entry in behavior-changes.md. Paths are under `rust/engine/src` unless they say otherwise.

### Bidding and payments

- **The Emperor's share of a Richese card is cut by Richese's homeworld.** When Richese are paid for a card and the Emperor gets their ally's share instead, the share is reduced by the low threshold and occupier of Richese's homeworld, not the Emperor's, and that occupier gets half. Where: `game/bidding.rs`, `Game::pay_emperor_instead_of_richese`. Players notice: the Emperor gets half when Richese's homeworld is low and occupied. Fix: use the Emperor's homeworld, or drop the cut.
- **The Emperor's share never counts as a recent payment.** The same function records the payment with an amount that is always 0, and `set_recent_payment` ignores 0. Where: `game/bidding.rs`, `Game::pay_emperor_instead_of_richese`. Players notice: the payment is missing wherever recent payments are shown or read, the bots included. Fix: pass the Emperor's amount.
- **A Nexus refund to Richese pays the Emperor's contribution to Richese's ally.** When the Richese Nexus card refunds a card Richese just won, the Emperor's contribution goes to Richese's ally instead of back to the Emperor. Where: `events/cards/nexus_played.rs`, `NexusPlayed::apply`, Richese arm. Players notice: the Emperor loses the spice and Richese's ally gains it. Fix: refund the Emperor.
- **Distrans during the black market plays a normal-auction pass.** When a Distrans fills the hand of a player who may still bid in the black market, the engine plays a full `Bid` pass (tasks and history entry), not a `BlackMarketBid` pass. Where: `events/cards/distrans_used.rs`, `DistransUsed::check_if_bidding_for_player_should_be_skipped`. Players notice: a normal-auction pass in the log and history during the black market. Fix: play a `BlackMarketBid` pass in that branch.
- **Charity is paid even when CHOAM can't pay it.** The claimant is credited before CHOAM's balance is checked. When CHOAM is short, the claimant keeps spice that came from nowhere, and the log says they "are unable to claim" it. Where: `game/charity.rs`, `Game::give_charity`. Players notice: the log contradicts the claimant's spice. Fix: credit only what CHOAM pays, or log that the bank paid.
- **Occupied homeworld income logs a loss of 0.** Whenever a homeworld has an occupier, income from it logs "0 ... goes to Y due to occupation", even when nothing is lost. Where: `game/collection.rs`, `Game::modify_income_because_of_low_threshold_or_occupation`. Players notice: a stray log line. Fix: log only when something is lost.

### Cards and Nexus cards

- **CHOAM's Karama discard uses up a different card than the one picked.** `KarmaChoamDiscard` validates `card_to_use` and then plays the first valid Karama card in hand. Where: `events/factions/karma_choam_discard.rs`, `KarmaChoamDiscard::apply`. Players notice: with two Karama cards, the other one is used up. Fix: play `card_to_use`.
- **The Tleilaxu Secret Ally's revival is free.** Validation checks that the player can pay `determine_tleilaxu_cost`, but nothing is paid. Where: `events/cards/nexus_played.rs`, `NexusPlayed::apply`, Tleilaxu arm. Players notice: forces revived with the card cost nothing. Fix: spend the cost (check the card's text first).
- **A `Shipment` without `karmaCard` is a Karama shipment.** `Shipment::default()` names card 0 (a Lasgun) as its Karama card, so event JSON that leaves the field out is refused as "Invalid Karama card". The web client always sends the field. Where: `events/movement/shipment.rs`, `impl Default for Shipment`. Players notice: nothing with the web client. Fix: default to `None`, and check the other handle fields that default to id 0, such as `Move::to`.
- **`TreacheryCard::countered_by` panics without a defense.** It expects a defense for Poison Tooth, Poison, Chemistry, Projectile, Weirding Way and Projectile+Poison. A bot caller can pass none: `determine_diplomacy` (`rust/bots/src/bot/other.rs`) passes the opponent's defense, which is absent when the opponent plays a weapon alone. Where: `cards/treachery_card.rs`, `TreacheryCard::countered_by`. Players notice: a bot's turn fails in a battle. Fix: treat no defense as not countering.

### Alliances and bribes

- **Bribes are accepted during Double Inflation.** The rejection "No bribes can be made during Double Inflation" is built and never returned. Where: `events/alliances/donated.rs`, `Donated::validate`. Players notice: bribes go through while the CHOAM Economics token is on Double. Fix: return the rejection (check the rule text first).
- **A card trade can be offered to an ally without cards.** "Your ally does not have cards to trade" is built and never returned. Where: `events/alliances/card_traded.rs`, `CardTraded::validate`. Players notice: the game waits for a trade the ally can't answer with a card. Fix: return the rejection.

### Battle, occupation and revival

- **Two rules for occupying a homeworld.** After a battle, a homeworld changes hands when exactly one battalion is left in it, whatever it holds. At the start and end of a turn the engine asks who controls it. Where: `game/battle.rs`, `Game::determine_occupation_location`, against `game/board.rs`, `determine_occupation_at_start_or_end_of_turn`. Players notice: the occupier set after a battle can change at the next turn boundary, likely wrongly with Bene Gesserit advisors. Fix: use one rule.
- **Tleilaxu earn free-revival income from their own free revival.** Where: `game/revival.rs`, `Game::claim_free_revival`. Players notice: Tleilaxu gain 1 spice when reviving their own forces for free. Fix: skip the income when the reviver is Tleilaxu (check the rulebook first).
- **`TraitorDiscarded` says the traitor is shuffled into the deck, but the deck isn't shuffled.** Where: `events/battle/traitor_discarded.rs`. Players notice: the log claims a shuffle. Fix: shuffle, or change the text (check the rulebook).

### Movement and spice blow

- **Sandmaster pays once per source location.** Forces moved from two sectors of one territory collect the Sandmaster's spice twice and take it off the board twice. Where: `game/movement.rs`, `Game::perform_move_from_locations`, the call to `check_sandmaster`. Players notice: extra spice for the Sandmaster's owner. Fix: apply it once per move.
- **A sandtrout doubling lost to the storm carries over.** A blow doubled by a sandtrout that lands in the storm doesn't reset the doubling, so a later blow, possibly in another turn, is doubled. Where: `game/blow.rs`, `Game::process_blow_card`, storm branch. Players notice: an unexpected doubled blow. Fix: reset `sand_trout_doubles_resources` in the storm branch.

### Setup, host and rules

- **The Ixian starting-card pick accepts any card.** Validation accepts every card, so a crafted event can take one that wasn't offered. Where: `events/setup/ixian_selected_starting_card.rs`, `validate`. Players notice: only with a modified client. Fix: refuse a card that isn't in `decks.starting_treachery_cards`.
- **Any player can delete another player's automation rule.** Delete removes a rule by id without checking its owner, and Update is accepted and does nothing. Where: `events/host/automation_configured.rs`, `validate` and `apply`. Players notice: only with a modified client. Fix: refuse a Delete of someone else's rule, and refuse Update.
- **`StrongholdBonus` is listed twice in the Expansion2AdvancedGame ruleset.** The list goes into `game.rules.active` and the lobby data. Where: `game/rules.rs`, `Game::ruleset_definition`. Players notice: possibly "Stronghold bonus" twice in the rule list. Fix: drop the duplicate and update `ruleset_definition_keeps_duplicate_rule` (engine/tests/pkg_b_players.rs).
- **Some error texts show a type name instead of the object.** Refusing a hero that isn't in the game prints "Treachery.Shared.Leader" (`heroes/hero.rs`, `Hero::plain_text`, used by `raise_dead_played.rs` and `karma_free_revival.rs`), and placing an ambassador where one already is prints "There already is an ambassador in Treachery.Shared.Territory" (`events/factions/ambassador_placed.rs`, `apply`). Players notice: a Game Error naming an internal type, probably only past validation. Fix: name the hero or territory.

### Web client

All in `web/src/`.

- **The new-account form doesn't stop on a bad username.** Later checks overwrite earlier ones, and the last one clears any username error once the other fields pass. The server doesn't check username length either. Where: `components/lobby/LoginComponent.tsx`, `validateNewAccount`. Players notice: usernames of 1 to 3 or over 40 characters are accepted. Fix: stop at the first failing check, and check on the server too.
- **A typed number in a force picker doesn't change the placement.** With `onStep`, typing only changes the shown number; the buttons and arrow keys change the placement. Where: `components/NumberPicker.tsx`, used by `ForceSelection` in `components/battle/BattleForms.tsx`. Players notice: they type 3, see 3, and submit another amount. Fix: commit typed values through `onChange`.
- **The admin's "Persist data" always fails.** `adminPersistState` refuses outside a game, and the admin page is never in one. Where: `lobby/client.ts`. Players notice: the admin always gets "Not in game". Fix: drop the check.
- **Lobby player order can change on reload.** `SlotDict` puts a user who joins after someone left into the freed slot, while the server lists them last. Where: `lobby/participation.ts`. Players notice: the seat list reorders on reload. Fix: use insertion order (a `Map`).
- **The turn popover's times are UTC, unlabeled, and drop whole days.** `longDate` prints UTC without saying so, `hms` wraps hours at 24 (25 hours shows as 1:00:00), and the labels open with `<string>` instead of `<strong>`, so they aren't bold. Where: `components/board/popups.ts`. Players notice: wrong clock times outside UTC, short durations, plain labels. Fix: local time, no wrap, `<strong>`.
- **The story table shows no terror-token image.** Terror tokens get `src=""`. Where: `components/StoryComponent.tsx`. Players notice: alt text or a broken image. Fix: use the skin's terror token image.

