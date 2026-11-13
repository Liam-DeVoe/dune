# Known open issues

Problems that are known and not fixed yet. Reviewers: don't report these again, but do report any new instance of one of these classes that isn't listed under it.

- **The Rock Melter bot changes the game while it decides.** `ClassicBot::determine_rock_was_melted` (`bots/src/bot/battle.rs`) runs the battle outcome on the live plans. That can log "<mirror weapon> becomes a <card>" and, with a Diplomat, set `card_used_by_diplomat`, all while the bot is only deciding. A replay of a game that doesn't run the bots doesn't make that change. The observe harness runs the bots on both sides, so it doesn't report this. Bots that never change the game while deciding are a planned behavior change (native-plan.md step 4).
