# Known open issues

Problems that are known and not fixed yet. Reviewers: don't report these again, but do report any new instance of one of these classes that isn't listed under it.

## No test reaches the Retreat form

A loser may retreat only if their leader survives the battle and has the Diplomat skill. The `retreat` fixture reached that state until game version 189 changed every random draw. `observe refit` carried it over, but with different skills dealt it no longer ends in the retreat (it ends in the battle report of turn 8; a second refit that asked for Harkonnen's `Retreat` at the end, with 150 seeds, didn't get there either), and none of 700 bot games and 200 fuzz games recorded at version 189 reach it. `battle_forms_in_fixtures` (view/tests/battle_forms.rs) no longer asserts that the Retreat form is reached. Fix: a fixture or built state where the loser's surviving leader is a Diplomat.

## Found by 2c and left alone

- A battle flash can read "null win!". Not investigated.
- `TraitorDiscarded` logs that the traitor deck is shuffled, but the deck isn't shuffled. Either the rule or the text is wrong. Check the rulebook.
- Plain-text output has no space between a number and a force name ("3forces"). The client draws them as separate tokens, so only mails and server texts show it.
- "X is a Y facedancer!" and "CHOAM The Economics Token flips ..." read badly.
- Faction sheets paste advantage titles into the sheet template before formatting it, so a custom skin label with `{N}` gets sheet parameter N, and a lone `{` leaves the whole sheet unformatted. 3b drops this.
- A custom skin missing any default key is replaced by the default skin entirely (C#'s `FixDictionary`), so its custom names are lost.
- The web client's Karama help text has typos ("an advantages ... occured").
