# Known open issues

Problems that are known and not fixed yet. Reviewers: don't report these again, but do report any new instance of one of these classes that isn't listed under it.

## No test reaches the Retreat form

A loser may retreat only if their leader survives the battle and has the Diplomat skill. The `retreat` fixture reached that state until game version 189 changed every random draw. `observe refit` carried it over, but with different skills dealt it no longer ends in the retreat (it ends in the battle report of turn 8; a second refit that asked for Harkonnen's `Retreat` at the end, with 150 seeds, didn't get there either), and none of 700 bot games and 200 fuzz games recorded at version 189 reach it. `battle_forms_in_fixtures` (view/tests/battle_forms.rs) no longer asserts that the Retreat form is reached. Fix: a fixture or built state where the loser's surviving leader is a Diplomat.

## Two battle flags outlive their battle

`decipherer_may_replace_traitor` and `secret_ally_allows_keeping_cards_after_losing_battle` are set during a battle and never reset (`game.battle_leftovers`, game/battle.rs). So a Decipherer's permission to replace a traitor stays for every later battle: `BattleConcluded` then rejects a conclusion that names only one of `new_traitor` and `traitor_to_replace`, and replaces a traitor when it names both, though no Decipherer looked at the traitor deck. And a Moritani secret ally that let a loser keep a card, unused, makes a later loser's kept card play the Nexus card. Fix: move both into `CurrentBattle`, so they go with the battle, with a behavior-changes.md entry. Found by the battle agent of wave 5, which kept them as they were to stay behavior-neutral.

## Found by 2c and left alone

- `TraitorDiscarded` logs that the traitor deck is shuffled, but the deck isn't shuffled. Either the rule or the text is wrong. Check the rulebook.
- "X is a Y facedancer!" and "CHOAM The Economics Token flips ..." read badly.
- A custom skin missing any default key is replaced by the default skin entirely (C#'s `FixDictionary`), so its custom names are lost.
- The web client's Karama help text has typos ("an advantages ... occured").

## Checked and correct

2026-11-15: at low threshold, Moritani reveals a terror token only when at least 3 forces enter. The Grumman card says "Low (0-7): ... You can only reveal terror tokens when at least 3 forces enter" (web/assets/art/GrummanCard.gif), and `check_terror_triggered` does that.

## What a bidding phase leaves behind

The code ported from C# never reset some bidding state when a bidding phase ended or the next one began, and kept reading it. Wave 5 keeps that behavior with `turn.last_bidding` (`BiddingLeftovers` in game/bidding.rs), which the next bidding phase starts from:

- The auction type of the last auction. If it was a Richese auction, `has_bidding_prescience` is true for every player until the next bidding phase starts: it decides whether deals sharing bidding prescience can be offered, and how bots value them in the charity phase.
- The bids of an auction that everyone passed on, and the "Atreides or their ally passed" flag. Until the first auction of the next bidding phase starts, the bidding view shows the old bids, Juice is offered only when they are empty, and the automation rule "pass when Atreides or their ally passed" can pass at once.
- `richese_bidding_just_finished`, which makes Richese's Nexus betrayal playable at the start of the next bidding phase if the last auction was Richese's and Richese were paid recently.
- `replacing_bought_card_using_nexus`, set when the Ixian Nexus card let a winner replace a card and they declined. A later replacement through the Ixian alliance then also plays the Nexus card.

Two revival fields last longer than the phase for the same reason (`turn.revivals`): a request to Tleilaxu that Tleilaxu never answered stays until the next revival phase, and a Karama prevention of revival played in the revival report applies to the next turn's revivals. Fix each as a behavior change, with a behavior-changes.md entry.
