//! Package A review (Game.cs, Game_Participation.cs, GameEvent.cs): tests that show
//! where the Rust port behaves differently from C#. See docs/reviews/pkg-a.md.
//!
//! Expected values come from the C# probe:
//!
//!     cd rust/engine/csharp-ref && dotnet run -c Release -- review-a
//!
//! Each test names the probe line it checks. These tests are meant to FAIL until the
//! difference is fixed.

use treachery_engine::canon;
use treachery_engine::events::GameEvent;
use treachery_engine::game::Game;
use treachery_engine::json;
use treachery_engine::cs::Shared;
use treachery_engine::participation::Participation;

// Steps 1 and 2 of rust/traces/sample/fuzz-2026-00018.jsonl.gz (same text as ReviewA.cs).
const ESTABLISH: &str = r#"{"$type":"EstablishPlayers","Settings":{"NumberOfPlayers":3,"MaximumTurns":8,"InitialRules":[301,300,65,60,20,1013,15,700,105,80,85,604,90,998,30,701,40,107,10,70,50,25,209,999,112],"AllowedFactionsInPlay":[50,10,120,60,40,100,20,90,70,80,30,110],"AutoOpenEmptySeats":true,"AsyncPlay":false,"AsyncPlayMessageIntervalMinutes":0},"Seed":1073548723,"MaximumNumberOfPlayers":3,"MaximumTurns":8,"ApplicableRules":[301,300,65,60,20,1013,15,700,105,80,85,604,90,998,30,701,40,107,10,70,50,25,209,999,112],"_players":"","_factionsInPlay":"Guild,Fremen,Moritani,BeneGesserit,Emperor,Richese,Atreides,Choam,Ixian,Tleilaxu,Harkonnen,Ecaz","Initiator":0,"Time":"2024-01-01T12:02:17+00:00"}"#;
const END_PHASE: &str = r#"{"$type":"EndPhase","Initiator":0,"Time":"2024-01-01T12:02:32+00:00"}"#;

fn trace_participation() -> Participation {
    let mut p = Participation::new();
    p.player_names.insert(100, "Human100".to_string());
    p.seated_players.insert(100, 2);
    p
}

fn run(events: &[&str]) -> Game {
    let mut g = Game::new_int32_participation(187, &Shared::new(trace_participation()));
    for text in events {
        let e: GameEvent = json::deserialize(text).expect("event json");
        e.initialize_game(&g);
        let r = e.execute(&mut g, true, true);
        assert!(r.is_none(), "event rejected: {text}");
    }
    g
}

/// The canonical dump of a game after `Undo`. `RecentlyUndoneEvents` hold events
/// that were initialized for the old game, so C# writes their `Game` as
/// `{"$ref":"OtherGame"}` (trace-format.md 7.2; seen in 8 of the 9 sample fuzz traces).
///
/// Probe: `undo.recentlyUndone[0].game_is_result = False`, `..._is_old = True`.
#[test]
fn undo_recently_undone_events_point_at_old_game_in_dump() {
    let g = run(&[ESTABLISH, END_PHASE]);
    let mut u = g;
    u.undo(1);
    assert_eq!(u.recently_undone_events.len(), 1);
    let dump = String::from_utf8(canon::dump_game(&u)).unwrap();
    let start = dump.find("\"RecentlyUndoneEvents\":").expect("field in dump");
    let field = &dump[start..start + 200.min(dump.len() - start)];
    assert!(
        field.contains(r#""Game":{"$ref":"OtherGame"}"#),
        "C# dumps the undone EndPhase's Game as OtherGame; Rust wrote: {field}"
    );
}

/// `Participation` is one shared object in C#: the game built by `Undo` and the
/// game it keeps as `PreviousGameState` use the same instance. A seat change made
/// on the undo game (the server's `AddPlayer`) is visible in the game that
/// `RestoreRecentlyUndone` returns.
///
/// Probe: `participation.shared = True`, `participation.old.isParticipant5 = True`,
/// `participation.old.seated = 1`.
#[test]
fn participation_is_shared_across_undo_and_restore() {
    let g = Game::new_int32_participation(187, &Shared::new(Participation::new()));
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
    let g = Game::new_int32_participation(187, &part);
    part.borrow_mut().hosts.add(7);
    assert!(g.is_host(Some(7)), "C#: the game reads the caller's Participation object");
}

/// EndPhase, PlacementEvent, Utilities, GameState and the small model types.
#[path = "review_pkg_a/part2.rs"]
mod part2;
