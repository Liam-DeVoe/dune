# Known open issues

Problems that are known and not fixed yet. Reviewers: don't report these again, but do report any new instance of one of these classes that isn't listed under it.

## No test reaches the Retreat form

A loser may retreat only if their leader survives the battle and has the Diplomat skill. The `retreat` fixture reached that state until game version 189 changed every random draw. `observe refit` carried it over, but with different skills dealt it no longer ends in the retreat (it ends in the battle report of turn 8; a second refit that asked for Harkonnen's `Retreat` at the end, with 150 seeds, didn't get there either), and none of 700 bot games and 200 fuzz games recorded at version 189 reach it. `battle_forms_in_fixtures` (view/tests/battle_forms.rs) no longer asserts that the Retreat form is reached. Fix: a fixture or built state where the loser's surviving leader is a Diplomat.

## Found by 2c and left alone

- `TraitorDiscarded` logs that the traitor deck is shuffled, but the deck isn't shuffled. Either the rule or the text is wrong. Check the rulebook.
- "X is a Y facedancer!" and "CHOAM The Economics Token flips ..." read badly.
- A custom skin missing any default key is replaced by the default skin entirely (C#'s `FixDictionary`), so its custom names are lost.

## Checked and correct

2026-11-15: at low threshold, Moritani reveals a terror token only when at least 3 forces enter. The Grumman card says "Low (0-7): ... You can only reveal terror tokens when at least 3 forces enter" (web/assets/art/GrummanCard.gif), and `check_terror_triggered` does that.

## A Karama prevention of revival can be played after the revivals

Tleilaxu may play their Karama to prevent a faction's revival during the whole revival main phase, the report included. Played in the report, after every revival, it does nothing and still uses up the card and the once-per-game Karama power. Before the fix in behavior-changes.md ("What a bidding or revival phase left behind"), it applied to the next turn's revivals instead. Fix: offer `KarmaRevivalPrevention` only in `Phase::Resurrection` (applicable_events.rs).
