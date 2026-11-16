# Known open issues

Problems that are known and not fixed yet. Reviewers: don't report these again, but do report any new instance of one of these classes that isn't listed under it.

## No test reaches the Retreat form

A loser may retreat only if their leader survives the battle and has the Diplomat skill. The `retreat` fixture reached that state until game version 189 changed every random draw. `observe refit` carried it over, but with different skills dealt it no longer ends in the retreat (it ends in the battle report of turn 8; a second refit that asked for Harkonnen's `Retreat` at the end, with 150 seeds, didn't get there either), and none of 700 bot games and 200 fuzz games recorded at version 189 reach it. `battle_forms_in_fixtures` (view/tests/battle_forms.rs) no longer asserts that the Retreat form is reached. Fix: a fixture or built state where the loser's surviving leader is a Diplomat.

## A leader who is alive can be revived

The fuzzer reaches a revival of a leader who isn't in the tanks. Validation accepts it, and the leader ends up dead with no death time (0), so in a view of the tanks it sorts before every leader who really died. Sweep 1c found it in fuzz games 77, 97, 113 and 126 (seed 1) and kept the old order by starting the per-game death count at 1. Fix: revival validation refuses a leader who isn't dead, with a test, and the death count can then start at 0.
