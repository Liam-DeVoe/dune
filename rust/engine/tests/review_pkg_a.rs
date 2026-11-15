//! Edge cases of the game core: human seats, game events and their bookkeeping.

use treachery_engine::game::Game;

/// The human seats outlive undo and restore: a seat the server gives a user after an undo is
/// still a human's seat in the game that `RestoreRecentlyUndone` returns, as the server's
/// participation, shared by both games in C#, said.
#[test]
fn human_seats_survive_undo_and_restore() {
    let mut g = Game::of_version(Game::LATEST_VERSION);
    g.undo(0);
    g.human_seats.insert(1);
    g.restore_recently_undone();
    assert!(g.human_seats.contains(&1));
}

/// EndPhase, PlacementEvent, Utilities, GameState and the small model types.
#[path = "review_pkg_a/part2.rs"]
mod part2;
