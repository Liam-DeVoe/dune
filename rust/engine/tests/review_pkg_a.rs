//! Package A review (Game.cs, Game_Participation.cs, GameEvent.cs): tests that show
//! where the Rust port behaves differently from C#. See docs/reviews/pkg-a.md.
//!
//! Expected values come from the C# probe:
//!
//!     cd rust/engine/csharp-ref && dotnet run -c Release -- review-a
//!
//! Each test names the probe line it checks. These tests are meant to FAIL until the
//! difference is fixed.

use treachery_engine::game::Game;
use treachery_engine::compat::cs::Shared;
use treachery_engine::game::Participation;

/// `Participation` is one shared object in C#: the game built by `Undo` and the
/// game it keeps as `PreviousGameState` use the same instance. A seat change made
/// on the undo game (the server's `AddPlayer`) is visible in the game that
/// `RestoreRecentlyUndone` returns.
///
/// Probe: `participation.shared = True`, `participation.old.isParticipant5 = True`,
/// `participation.old.seated = 1`.
#[test]
fn participation_is_shared_across_undo_and_restore() {
    let g = Game::new_int32_participation(Game::LATEST_VERSION, &Shared::new(Participation::new()));
    let mut u = g;
    u.undo(0);
    u.add_player(5, "Late", 1);
    let mut r = u;
    r.restore_recently_undone();
    assert!(r.is_participant(5), "C#: the restored game sees the player added after Undo");
    assert_eq!(r.number_of_seated_players(), 1);
}

/// `new Game(version, participation)` keeps the caller's object. In C# a later change
/// to it (the server keeps and edits it) is visible through the game.
///
/// Probe: `participation.caller_alias.isHost7 = True`.
#[test]
fn participation_passed_to_constructor_is_aliased() {
    let part = Shared::new(Participation::new());
    let g = Game::new_int32_participation(Game::LATEST_VERSION, &part);
    part.borrow_mut().hosts.insert(7);
    assert!(g.is_host(Some(7)), "C#: the game reads the caller's Participation object");
}

/// EndPhase, PlacementEvent, Utilities, GameState and the small model types.
#[path = "review_pkg_a/part2.rs"]
mod part2;
