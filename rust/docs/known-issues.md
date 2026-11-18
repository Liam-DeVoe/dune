# Known open issues

Problems that are known and not fixed yet. Reviewers: don't report these again, but do report any new instance of one of these classes that isn't listed under it.

## No test reaches the Retreat form

A loser may retreat only if their leader survives the battle and has the Diplomat skill. The `retreat` fixture reached that state until game version 189 changed every random draw. `observe refit` carried it over, but with different skills dealt it no longer ends in the retreat (it ends in the battle report of turn 8; a second refit that asked for Harkonnen's `Retreat` at the end, with 150 seeds, didn't get there either), and none of 700 bot games and 200 fuzz games recorded at version 189 reach it. `battle_forms_in_fixtures` (view/tests/battle_forms.rs) no longer asserts that the Retreat form is reached. Fix: a fixture or built state where the loser's surviving leader is a Diplomat.

## Found by 2c and left alone

- "X is a Y facedancer!" and "CHOAM The Economics Token flips ..." read badly.

## Checked and correct

2026-11-15: at low threshold, Moritani reveals a terror token only when at least 3 forces enter. The Grumman card says "Low (0-7): ... You can only reveal terror tokens when at least 3 forces enter" (web/assets/art/GrummanCard.gif), and `check_terror_triggered` does that.

2026-11-18: a Bene Gesserit bot's predicted win does change its battle plan. In `best_voice` (`rust/bots/src/bot/factions.rs`) the prediction doesn't change the voice chosen, but it starts the plan's two flags `player_hero_will_certainly_survive` and `opponent_hero_will_certainly_be_zero` true. Battle planning reads them in `chance_of_my_leader_surviving` and `chance_of_enemy_leader_dying` (`rust/bots/src/bot/battle.rs`) as certainty, a chance of 1 instead of 0.5, that the bot's leader survives the voiced defense and the opponent's dies to the voiced weapon. The C# version did the same; the known-issues entry that said nothing read the flags was wrong.

2026-11-18: Tleilaxu earn 1 spice from their own free revival (`Game::claim_free_revival`, game/revival.rs). The Ixians & Tleilaxu rulebook says "Also, for each faction using free revival or a ghola card, you take 1 spice from the Spice Bank", which includes Tleilaxu. Only the tech token excludes them: "However, if only the Tleilaxu player takes free revival, you do not collect spice." The engine leaves Tleilaxu out of the tech token's income only (`revival_tech_token_income`).

2026-11-18: a homeworld has two occupation rules. After a battle, a faction left alone in another faction's homeworld occupies it at once (`Game::determine_occupation_location`, game/battle.rs). At the start and end of a turn, whoever controls it occupies it (`determine_occupation_at_start_or_end_of_turn`, game/board.rs). The Ecaz & Moritani rulebook has both: "If a player starts or ends a turn occupying another faction's homeworld with one or more of their forces, they are considered the 'Occupier'… If at any time, a faction occupies another faction's homeworld alone, they are immediately considered the Occupier for that turn." The worry was Bene Gesserit advisors, but the rulebook also says "Bene Gesserit cannot send advisors to Homeworlds", and the engine enforces that: intrusions and accompanying skip homeworlds (`check_bene_gesserit_intrusion`, `BeneGesseritAccompanies::valid_targets`), and Bene Gesserit forces shipped to a homeworld become fighters unless advisors are already there (`Shipment` apply, `must_be_advisors`).
