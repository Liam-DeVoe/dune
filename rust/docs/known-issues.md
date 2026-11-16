# Known open issues

Problems that are known and not fixed yet. Reviewers: don't report these again, but do report any new instance of one of these classes that isn't listed under it.

## No test reaches the Retreat form

A loser may retreat only if their leader survives the battle and has the Diplomat skill. The `retreat` fixture reached that state until game version 189 changed every random draw. `observe refit` carried it over, but with different skills dealt it no longer ends in the retreat (it ends in the battle report of turn 8; a second refit that asked for Harkonnen's `Retreat` at the end, with 150 seeds, didn't get there either), and none of 700 bot games and 200 fuzz games recorded at version 189 reach it. `battle_forms_in_fixtures` (view/tests/battle_forms.rs) no longer asserts that the Retreat form is reached. Fix: a fixture or built state where the loser's surviving leader is a Diplomat.

## Two battle flags outlive their battle

`decipherer_may_replace_traitor` and `secret_ally_allows_keeping_cards_after_losing_battle` are set during a battle and never reset (`game.battle_leftovers`, game/battle.rs). So a Decipherer's permission to replace a traitor stays for every later battle: `BattleConcluded` then rejects a conclusion that names only one of `new_traitor` and `traitor_to_replace`, and replaces a traitor when it names both, though no Decipherer looked at the traitor deck. And a Moritani secret ally that let a loser keep a card, unused, makes a later loser's kept card play the Nexus card. Fix: move both into `CurrentBattle`, so they go with the battle, with a behavior-changes.md entry. Found by the battle agent of wave 5, which kept them as they were to stay behavior-neutral.

## Found by 2c and left alone

- `TraitorDiscarded` logs that the traitor deck is shuffled, but the deck isn't shuffled. Either the rule or the text is wrong. Check the rulebook.
- Plain-text output has no space between a number and a force name ("3forces"). The client draws them as separate tokens, so only mails and server texts show it.
- "X is a Y facedancer!" and "CHOAM The Economics Token flips ..." read badly.
- A custom skin missing any default key is replaced by the default skin entirely (C#'s `FixDictionary`), so its custom names are lost.
- The web client's Karama help text has typos ("an advantages ... occured").

## Bene Gesserit fighters that accompany a shipment never trigger an ambassador

The rulebook (Ecaz & Moritani, Ecaz faction rules) says an ambassador triggers when another faction "(other than your ally, advisors, or the faction matching the marker) enters a stronghold". Accompanying Bene Gesserit normally arrive as advisors, which don't trigger, so the usual case is right. But the engine sometimes places accompanying forces as fighters (into a territory where Bene Gesserit already have fighters, or with the advisors rule off), and `BeneGesseritAccompanies::forces_added_to_location` always returns 0, so `check_ambassador_triggered` never fires for them. Fighters entering should trigger it. The fuzzer never reaches this. `engine/tests/phases_unreached.rs` pins today's behavior. Fix it in wave 5 (ship and move), with a behavior-changes.md entry, and update that test.

Checked and correct (2026-11-15): at low threshold, Moritani reveals a terror token only when at least 3 forces enter. The Grumman card says "Low (0-7): ... You can only reveal terror tokens when at least 3 forces enter" (web/assets/art/GrummanCard.gif), and `check_terror_triggered` does that.
