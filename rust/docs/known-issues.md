# Known open issues

Problems that are known and not fixed yet. Reviewers: don't report these again, but do report any new instance of one of these classes that isn't listed under it.

## No test reaches the Retreat form

A loser may retreat only if their leader survives the battle and has the Diplomat skill. The `retreat` fixture reached that state until game version 189 changed every random draw. `observe refit` carried it over, but with different skills dealt it no longer ends in the retreat (it ends in the battle report of turn 8; a second refit that asked for Harkonnen's `Retreat` at the end, with 150 seeds, didn't get there either), and none of 700 bot games and 200 fuzz games recorded at version 189 reach it. `battle_forms_in_fixtures` (view/tests/battle_forms.rs) no longer asserts that the Retreat form is reached. Fix: a fixture or built state where the loser's surviving leader is a Diplomat.

## Found by 2c and left alone

- A battle flash can read "null win!". Not investigated.
- `TraitorDiscarded` logs that the traitor deck is shuffled, but the deck isn't shuffled. Either the rule or the text is wrong. Check the rulebook.
- Plain-text output has no space between a number and a force name ("3forces"). The client draws them as separate tokens, so only mails and server texts show it.
- "X is a Y facedancer!" and "CHOAM The Economics Token flips ..." read badly.
- A custom skin missing any default key is replaced by the default skin entirely (C#'s `FixDictionary`), so its custom names are lost.
- The web client's Karama help text has typos ("an advantages ... occured").

## Bene Gesserit fighters that accompany a shipment never trigger an ambassador

The rulebook (Ecaz & Moritani, Ecaz faction rules) says an ambassador triggers when another faction "(other than your ally, advisors, or the faction matching the marker) enters a stronghold". Accompanying Bene Gesserit normally arrive as advisors, which don't trigger, so the usual case is right. But the engine sometimes places accompanying forces as fighters (into a territory where Bene Gesserit already have fighters, or with the advisors rule off), and `BeneGesseritAccompanies::forces_added_to_location` always returns 0, so `check_ambassador_triggered` never fires for them. Fighters entering should trigger it. The fuzzer never reaches this. `engine/tests/phases_unreached.rs` pins today's behavior. Fix it in wave 5 (ship and move), with a behavior-changes.md entry, and update that test.

Checked and correct (2026-11-15): at low threshold, Moritani reveals a terror token only when at least 3 forces enter. The Grumman card says "Low (0-7): ... You can only reveal terror tokens when at least 3 forces enter" (web/assets/art/GrummanCard.gif), and `check_terror_triggered` does that.

## What a bidding phase leaves behind

The code ported from C# never reset some bidding state when a bidding phase ended or the next one began, and kept reading it. Wave 5 keeps that behavior with `turn.last_bidding` (`BiddingLeftovers` in game/bidding.rs), which the next bidding phase starts from:

- The auction type of the last auction. If it was a Richese auction, `has_bidding_prescience` is true for every player until the next bidding phase starts: it decides whether deals sharing bidding prescience can be offered, and how bots value them in the charity phase.
- The bids of an auction that everyone passed on, and the "Atreides or their ally passed" flag. Until the first auction of the next bidding phase starts, the bidding view shows the old bids, Juice is offered only when they are empty, and the automation rule "pass when Atreides or their ally passed" can pass at once.
- `richese_bidding_just_finished`, which makes Richese's Nexus betrayal playable at the start of the next bidding phase if the last auction was Richese's and Richese were paid recently.
- `replacing_bought_card_using_nexus`, set when the Ixian Nexus card let a winner replace a card and they declined. A later replacement through the Ixian alliance then also plays the Nexus card.

Two revival fields last longer than the phase for the same reason (`turn.revivals`): a request to Tleilaxu that Tleilaxu never answered stays until the next revival phase, and a Karama prevention of revival played in the revival report applies to the next turn's revivals. Fix each as a behavior change, with a behavior-changes.md entry.
