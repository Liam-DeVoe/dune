# Known open issues

Problems that are known and not fixed yet. Reviewers: don't report these again, but do report any new instance of one of these classes that isn't listed under it.

## No test reaches the Retreat form

A loser may retreat only if their leader survives the battle and has the Diplomat skill. The `retreat` fixture reached that state until game version 189 changed every random draw. `observe refit` carried it over, but with different skills dealt it no longer ends in the retreat (it ends in the battle report of turn 8; a second refit that asked for Harkonnen's `Retreat` at the end, with 150 seeds, didn't get there either), and none of 700 bot games and 200 fuzz games recorded at version 189 reach it. `battle_forms_in_fixtures` (view/tests/battle_forms.rs) no longer asserts that the Retreat form is reached. Fix: a fixture or built state where the loser's surviving leader is a Diplomat.

## Found by 2c and left alone

- "X is a Y facedancer!" and "CHOAM The Economics Token flips ..." read badly.

## Checked and correct

2026-11-15: at low threshold, Moritani reveals a terror token only when at least 3 forces enter. The Grumman card says "Low (0-7): ... You can only reveal terror tokens when at least 3 forces enter" (web/assets/art/GrummanCard.gif), and `check_terror_triggered` does that.

2026-11-18: Tleilaxu earn 1 spice from their own free revival (`Game::claim_free_revival`, game/revival.rs). The Ixians & Tleilaxu rulebook says "Also, for each faction using free revival or a ghola card, you take 1 spice from the Spice Bank", which includes Tleilaxu. Only the tech token excludes them: "However, if only the Tleilaxu player takes free revival, you do not collect spice." The engine leaves Tleilaxu out of the tech token's income only (`revival_tech_token_income`).

2026-11-18: a homeworld has two occupation rules. After a battle, a faction left alone in another faction's homeworld occupies it at once (`Game::determine_occupation_location`, game/battle.rs). At the start and end of a turn, whoever controls it occupies it (`determine_occupation_at_start_or_end_of_turn`, game/board.rs). The Ecaz & Moritani rulebook has both: "If a player starts or ends a turn occupying another faction's homeworld with one or more of their forces, they are considered the 'Occupier'… If at any time, a faction occupies another faction's homeworld alone, they are immediately considered the Occupier for that turn." The worry was Bene Gesserit advisors, but the rulebook also says "Bene Gesserit cannot send advisors to Homeworlds", and the engine enforces that: intrusions and accompanying skip homeworlds (`check_bene_gesserit_intrusion`, `BeneGesseritAccompanies::valid_targets`), and Bene Gesserit forces shipped to a homeworld become fighters unless advisors are already there (`Shipment` apply, `must_be_advisors`).

## Quirks kept from the C# version

Behavior that looks wrong or arbitrary and has no reason except that the C# version did the same. Each is unchanged for now and marked in the code with `Known quirk (known-issues.md)`. Fixing one changes what players see, so it needs the owner's approval and an entry in behavior-changes.md. Paths are under `rust/engine/src` unless they say otherwise.

### Skin

- **A custom skin missing any key of a dictionary loses that whole dictionary.** It is replaced by the default skin's, so the custom names in it are lost. Where: `rust/skin/src/skin_json.rs`, `fix`. Players notice: a custom skin shows default names for a whole category. Fix: fill in only the missing keys.

### Bots

All in `rust/bots/src/bot/`.

- **Bots without an ally never reveal Atomics.** The bot reveals Atomics only if more than 5 enemy forces are there and its ally has none there, which is false without an ally. Where: `factions.rs`, `determine_terror_revealed`. Players notice: a Moritani bot without an ally never uses Atomics. Fix: treat no ally as no allied forces.
- **A bot offered two faction trades never answers.** Where: `other.rs`, `determine_faction_trade_offered`. Players notice: setup waits on the bot until the host moves on. Fix: pick one offer.
- **Testing Station scoring doesn't wrap around the board.** The sectors before 0 and after 17 score nothing, so a storm ending at sector 0 or 17 is misjudged. Where: `other.rs`, `determine_testing_station_used`. Players notice: a poor storm adjustment near the 17/0 boundary. Fix: use `Sector::moved`.
- **A predicted win doesn't change the bot's Voice.** The branch for a predicted win only sets two flags that nothing afterwards reads. Where: `factions.rs`, `best_voice`. Players notice: a slightly weaker bot. Fix: decide what the prediction should do, or drop the branch.

### Web client

All in `web/src/`.

- **The new-account form doesn't stop on a bad username.** Later checks overwrite earlier ones, and the last one clears any username error once the other fields pass. The server doesn't check username length either. Where: `components/lobby/LoginComponent.tsx`, `validateNewAccount`. Players notice: usernames of 1 to 3 or over 40 characters are accepted. Fix: stop at the first failing check, and check on the server too.
- **A typed number in a force picker doesn't change the placement.** With `onStep`, typing only changes the shown number; the buttons and arrow keys change the placement. Where: `components/NumberPicker.tsx`, used by `ForceSelection` in `components/battle/BattleForms.tsx`. Players notice: they type 3, see 3, and submit another amount. Fix: commit typed values through `onChange`.
- **The admin's "Persist data" always fails.** `adminPersistState` refuses outside a game, and the admin page is never in one. Where: `lobby/client.ts`. Players notice: the admin always gets "Not in game". Fix: drop the check.
- **Lobby player order can change on reload.** `SlotDict` puts a user who joins after someone left into the freed slot, while the server lists them last. Where: `lobby/participation.ts`. Players notice: the seat list reorders on reload. Fix: use insertion order (a `Map`).
- **The turn popover's times are UTC, unlabeled, and drop whole days.** `longDate` prints UTC without saying so, `hms` wraps hours at 24 (25 hours shows as 1:00:00), and the labels open with `<string>` instead of `<strong>`, so they aren't bold. Where: `components/board/popups.ts`. Players notice: wrong clock times outside UTC, short durations, plain labels. Fix: local time, no wrap, `<strong>`.
- **The story table shows no terror-token image.** Terror tokens get `src=""`. Where: `components/StoryComponent.tsx`. Players notice: alt text or a broken image. Fix: use the skin's terror token image.

