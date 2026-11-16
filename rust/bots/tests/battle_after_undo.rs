//! A battle initiated again after an undo is a new battle, even though its `BattleInitiated` is
//! at the same place in the history. A bot that used Voice in the undone battle must not plan
//! with that Voice in the new one. C# got this from comparing battles by reference. When battles
//! got an id, it was at first the event's place in the history, so the bot planned with the stale
//! Voice (fuzz seed 1 game 41; escapes.md, "A battle initiated again after an undo kept its
//! identity").
//!
//! `golden/battle-initiated-again-after-undo.json.gz` is that game's savegame just after Fremen
//! initiate a battle against the Guild (event 534). The Guild has Voice there.

use std::io::Read;

use treachery_bots::prelude::*;

fn savegame() -> Game {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/battle-initiated-again-after-undo.json.gz");
    let mut text = String::new();
    flate2::read::GzDecoder::new(std::fs::File::open(path).unwrap()).read_to_string(&mut text).unwrap();
    Game::load(&treachery_legacy::load_any(&text).unwrap(), &Default::default(), false, false).unwrap()
}

fn submit(game: &mut Game, event: &str) {
    let e: HistoryEntry = treachery_legacy::deserialize(event).unwrap();
    assert_eq!(game.play_for_message(e, true, false), None, "{event}");
}

const INITIATE: &str = r#"{"$type":"BattleInitiated","Target":50,"_territoryId":4,"Initiator":10,"Time":"2024-01-08T09:03:50+00:00"}"#;

#[test]
fn a_battle_initiated_again_after_an_undo_is_a_new_battle() {
    let mut game = savegame();
    assert_eq!(game.history.entries.len(), 535);
    let first = game.current_battle().cloned().unwrap();

    // Undoing a later event keeps the battle.
    submit(&mut game, r#"{"$type":"VoiceDeclined","Initiator":50,"Time":"2024-01-08T09:00:23+00:00"}"#);
    game.undo(535);
    assert_eq!(game.current_battle(), Some(&first));

    // Undoing the battle and initiating it again makes a new one.
    game.undo(534);
    submit(&mut game, INITIATE);
    let again = game.current_battle().cloned().unwrap();
    assert_eq!((again.initiator(), again.target, again.territory), (first.initiator(), first.target, first.territory));
    assert_ne!(again, first);
}

#[test]
fn a_bot_does_not_plan_with_its_voice_from_an_undone_battle() {
    let mut game = savegame();
    let guild = game.get_player(Some(Faction::Guild)).unwrap();
    let bot = || ClassicBot::new(guild, BotParameters::for_faction(Some(Faction::Guild)), treachery_engine::random::new_rng(1));

    // Both bots use Voice in the first battle, so their random draws stay in step. The second
    // forgets its Voice, as it would for a different battle.
    let (mut voiced, mut fresh) = (bot(), bot());
    assert!(voiced.determine_voice(&game).is_some());
    assert!(fresh.determine_voice(&game).is_some());
    fresh.voice_plan = None;

    game.undo(534);
    submit(&mut game, INITIATE);
    submit(&mut game, r#"{"$type":"VoiceDeclined","Initiator":60,"Time":"2024-01-08T09:03:59+00:00"}"#);
    submit(&mut game, r#"{"$type":"Prescience","Aspect":20,"Initiator":10,"Time":"2024-01-08T09:05:23+00:00"}"#);

    let plan = voiced.determine_battle(&game).unwrap();
    assert_eq!(plan, fresh.determine_battle(&game).unwrap());
    // What the reference build decides here.
    assert_eq!((plan.forces, plan.forces_at_half_strength, plan.hero.map(|h| h.id())), (8, 1, Some(1021)));
}
