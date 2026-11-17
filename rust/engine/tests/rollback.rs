//! An event that passes validation and then fails while it is applied (a Game Error) leaves the
//! game as it was before the event: `Game::play` replays the history and carries over what the
//! history doesn't hold. Before, the changes made up to the failure stayed while the event was
//! left out of the history, so an undo replayed a different game (escapes.md, "A Bene Gesserit
//! intrusion left over from another phase"; behavior-changes.md, "A failed event leaves the game
//! as it was").
//!
//! Games are compared by their whole `Debug` text, which holds every field of the game, the
//! undo state and the game before an undo included, plus the position of the random stream,
//! which the generator's `Debug` doesn't show.

use std::path::PathBuf;

use treachery_engine::prelude::*;

fn fixture(name: &str) -> Game {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scripts/fixtures").join(format!("{name}.json"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let state = treachery_legacy::load_any(&text).expect("savegame");
    let mut game = Game::of_version(state.version);
    game.human_seats.insert(Seat::new(1));
    for e in state.events.clone() {
        assert!(game.play_for_message(e, false, false).is_none());
    }
    game
}

/// Everything the game holds.
fn dump(g: &Game) -> String {
    format!("{g:#?}\nrandom stream: {:?} {:?} {}", g.rng.get_seed(), g.rng.get_stream(), g.rng.get_word_pos())
}

fn same(what: &str, actual: &str, want: &str) {
    if actual != want {
        let (a, w): (Vec<_>, Vec<_>) = (actual.lines().collect(), want.lines().collect());
        let at = a.iter().zip(&w).position(|(x, y)| x != y).unwrap_or(a.len().min(w.len()));
        let from = at.saturating_sub(10);
        panic!(
            "{what}: differs at line {at}\n  expected:\n{}\n  actual:\n{}",
            w[from..(at + 5).min(w.len())].join("\n"),
            a[from..(at + 5).min(a.len())].join("\n")
        );
    }
}

/// CHOAM's `ChoamRemoveForce` without a location, by the bidding phase's current bidder, ten
/// minutes after the last event. It logs itself, discards the card or uses the Nexus, and puts a
/// force of the target in reserves before it fails with "No location to remove a force from".
/// Savegames are replayed without validation, which is how it gets through.
fn failing_event(g: &Game) -> HistoryEntry {
    let bidder = g.bidding().unwrap().bid_sequence.as_ref().unwrap().current_player(g).expect("a bidder");
    let by = g.player(bidder).faction.unwrap();
    let target = g.players.iter().filter_map(|p| p.faction).find(|f| *f != by).unwrap();
    let mut e = ChoamRemoveForce::default();
    e.target = Some(target);
    e.location = None;
    let mut entry = HistoryEntry::new(by, e);
    let last = g.history.entries.last().unwrap().at;
    entry.at = DateTimeOffset::new(last.utc_ticks() + 10 * 60 * 10_000_000, 0);
    entry
}

fn fail(g: &mut Game) {
    let message = g.play_for_message(failing_event(g), false, false).map(|m| m.plain_text());
    assert_eq!(message.as_deref(), Some("Game Error: No location to remove a force from."));
}

#[test]
fn a_failed_event_leaves_the_game_as_it_was() {
    let mut g = fixture("bid");
    assert_eq!(g.phase(), Phase::Bidding, "a timed phase, so the event's time changes the timers");
    let before = dump(&g);

    // Without the rollback, the event changes the game before it fails.
    let mut unguarded = fixture("bid");
    let entry = failing_event(&unguarded);
    let by = By::new(&unguarded, entry.by);
    let Event::ChoamRemoveForce(e) = &entry.event else { unreachable!() };
    assert!(EventRules::apply(e, &mut unguarded, by).is_err());
    assert_ne!(dump(&unguarded), before, "the event fails halfway, after changing the game");

    fail(&mut g);
    same("after the failed event", &dump(&g), &before);

    // The same game as a replay of its history, with the entries' ids and the same human seats.
    let replayed = Game::load(&GameState::of(&g), &g.human_seats, false, false).unwrap();
    same("a replay of the history", &dump(&replayed), &dump(&g));
}

/// The undo state isn't in the history: the events undone and the game before the undo stay,
/// and a restore still returns to that game.
#[test]
fn a_failed_event_keeps_the_undo_state() {
    let mut g = fixture("bid");
    let at_end = dump(&g);
    let n = g.history.entries.len() as i32;
    g.undo(n - 1);
    let before = dump(&g);
    assert!(g.history.previous_game_state.is_some());

    fail(&mut g);
    same("after the failed event", &dump(&g), &before);

    g.restore_recently_undone();
    same("restored", &dump(&g), &at_end);
}

/// The human seats aren't in the history either.
#[test]
fn a_failed_event_keeps_the_human_seats() {
    let mut g = fixture("bid");
    g.human_seats.insert(Seat::new(0));
    let before = dump(&g);
    fail(&mut g);
    same("after the failed event", &dump(&g), &before);
}
