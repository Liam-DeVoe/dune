//! `Undo`, `RestoreRecentlyUndone` and `DismissRecentlyUndone` as state snapshots: an
//! undo replays the kept events into a new game and keeps the current game as
//! `previous_game_state`; a restore returns to that game exactly; a dismiss replays the
//! history into a new game. See engine-conventions.md, "Undo".
//!
//! Games are compared by a fingerprint of what players and later events can see: the saved
//! history, the log, the phase, every player's resources, cards, leaders and forces, the events
//! each player may submit, and how many random draws the game made.

use treachery_engine::error::catch_panic;
use treachery_engine::shared::Shared;
use treachery_engine::events::GameEvent;
use treachery_engine::game::{Game, GameState, Participation};
use treachery_engine::json;

// The first two events of a three-player game.
const ESTABLISH: &str = r#"{"$type":"EstablishPlayers","Settings":{"NumberOfPlayers":3,"MaximumTurns":8,"InitialRules":[301,300,65,60,20,1013,15,700,105,80,85,604,90,998,30,701,40,107,10,70,50,25,209,999,112],"AllowedFactionsInPlay":[50,10,120,60,40,100,20,90,70,80,30,110],"AutoOpenEmptySeats":true,"AsyncPlay":false,"AsyncPlayMessageIntervalMinutes":0},"Seed":1073548723,"MaximumNumberOfPlayers":3,"MaximumTurns":8,"ApplicableRules":[301,300,65,60,20,1013,15,700,105,80,85,604,90,998,30,701,40,107,10,70,50,25,209,999,112],"_players":"","_factionsInPlay":"Guild,Fremen,Moritani,BeneGesserit,Emperor,Richese,Atreides,Choam,Ixian,Tleilaxu,Harkonnen,Ecaz","Initiator":0,"Time":"2024-01-01T12:02:17+00:00"}"#;
const END_PHASE: &str = r#"{"$type":"EndPhase","Initiator":0,"Time":"2024-01-01T12:02:32+00:00"}"#;

/// What players and later events can see of a game, one line per item.
fn dump(g: &mut Game) -> String {
    let mut lines = vec![
        format!("history: {}", GameState::get_state_as_string(g)),
        format!("phase: {} turn {}", g.current_phase, g.current_turn),
        format!("draws: {:?}", g.random.as_ref().map(|r| r.get_count())),
    ];
    for (i, report) in std::iter::once(&g.pre_game_report).chain(g.report_list.iter()).enumerate() {
        for m in &report.messages {
            lines.push(format!("report {i}: {} {}", m.target, m.to_cs_string()));
        }
    }
    let host: Vec<_> = g.get_applicable_events(None, true).into_iter().map(|t| t.name()).collect();
    lines.push(format!("host may: {}", host.join(" ")));
    for i in 0..g.players.len() {
        let index = g.player_at(i);
        let may: Vec<_> = g.get_applicable_events(Some(index), false).into_iter().map(|t| t.name()).collect();
        let p = g.player(index);
        let ids = |xs: &mut dyn Iterator<Item = i32>| xs.map(|x| x.to_string()).collect::<Vec<_>>().join(",");
        lines.push(format!(
            "player {i}: {} seat {} spice {} cards [{}] traitors [{}] leaders [{}] forces [{}] ally {} may: {}",
            p.faction,
            p.seat,
            p.resources,
            ids(&mut p.treachery_cards.iter().map(|c| c.id())),
            ids(&mut p.traitors.iter().map(|h| h.id())),
            ids(&mut p.leaders.iter().map(|l| l.id())),
            p.forces_in_locations
                .iter()
                .map(|(l, b)| format!("{}:{}/{}", l.id(), b.amount_of_forces, b.amount_of_special_forces))
                .collect::<Vec<_>>()
                .join(","),
            p.ally,
            may.join(" "),
        ));
    }
    lines.join("\n")
}

/// Compares two dumps, reporting the first difference with context.
fn same_dump(what: &str, actual: &str, want: &str) {
    if actual != want {
        let at = actual.bytes().zip(want.bytes()).position(|(a, b)| a != b).unwrap_or(actual.len().min(want.len()));
        let from = at.saturating_sub(300);
        panic!(
            "{what}: dump differs at byte {at}\n  expected: ...{}\n  actual:   ...{}",
            &want[from..(at + 200).min(want.len())],
            &actual[from..(at + 200).min(actual.len())]
        );
    }
}

fn participation() -> Participation {
    let mut p = Participation::new();
    p.player_names.insert(100, "Human100".to_string());
    p.seated_players.insert(100, 2);
    p
}

/// As a client submits an event: initialize, then execute with validation.
fn run(g: &mut Game, events: &[&str]) {
    for text in events {
        let e: GameEvent = json::deserialize(text).expect("event json");
        e.initialize_game(g);
        let _ = catch_panic(|| e.execute(g, true, true));
    }
}

fn game_after(events: &[&str]) -> Game {
    let mut g = Game::new_int32_participation(Game::LATEST_VERSION, &Shared::new(participation()));
    run(&mut g, events);
    g
}

fn base() -> Game {
    game_after(&[ESTABLISH, END_PHASE])
}

#[test]
fn undo_replays_the_kept_events() {
    let mut g = base();
    g.undo(1);
    assert_eq!(g.history.len(), 1);
    assert_eq!(g.recently_undone_event_count, 1);
    assert_eq!(g.recently_undone_events.len(), 1);
    assert!(g.previous_game_state.is_some());

    // Apart from the undo bookkeeping, the state is that of a game that only ran the kept event.
    g.recently_undone_event_count = 0;
    g.recently_undone_events.clear();
    g.previous_game_state = None;
    same_dump("undo", &dump(&mut g), &dump(&mut game_after(&[ESTABLISH])));
}

#[test]
fn restore_returns_to_the_state_before_the_undo() {
    let mut g = base();
    let before = dump(&mut g);
    g.undo(1);
    g.restore_recently_undone();
    same_dump("restore", &dump(&mut g), &before);
    assert!(g.previous_game_state.is_none());
}

/// Events played after an undo are dropped by a restore.
#[test]
fn restore_drops_events_played_after_the_undo() {
    let before = dump(&mut base());
    let mut g = base();
    g.undo(1);
    run(&mut g, &[END_PHASE]);
    assert_eq!(g.history.len(), 2);
    g.restore_recently_undone();
    same_dump("restore after play", &dump(&mut g), &before);
}

#[test]
fn dismiss_replays_into_a_new_game() {
    let mut g = base();
    g.undo(1);
    g.dismiss_recently_undone();
    assert!(g.previous_game_state.is_none());
    assert_eq!(g.recently_undone_events.len(), 0);
    same_dump("dismiss", &dump(&mut g), &dump(&mut game_after(&[ESTABLISH])));
}

/// The branch of `RestoreRecentlyUndone` that replays history and the undone events.
#[test]
fn restore_without_previous_game_replays() {
    let mut g = base();
    g.undo(1);
    g.previous_game_state = None;
    g.restore_recently_undone();
    same_dump("restore replay", &dump(&mut g), &dump(&mut base()));
}

/// Each restore returns to the game the matching undo was made from.
#[test]
fn two_undos_restore_in_order() {
    let mut g = base();
    let at_two = dump(&mut g);
    g.undo(2);
    let after_first = dump(&mut g);
    g.undo(1);
    assert_eq!(g.history.len(), 1);
    g.restore_recently_undone();
    same_dump("first restore", &dump(&mut g), &after_first);
    g.restore_recently_undone();
    same_dump("second restore", &dump(&mut g), &at_two);
}

/// A negative event number panics and leaves the game as it was.
#[test]
fn undo_that_throws_keeps_the_game() {
    let mut g = base();
    let before = dump(&mut g);
    catch_panic(|| g.undo(-1)).expect_err("panics");
    same_dump("undo -1", &dump(&mut g), &before);
}

/// The game keeps the caller's `Participation`, and every game made by undo, restore
/// and dismiss shares it.
#[test]
fn participation_is_shared() {
    let part = Shared::new(Participation::new());
    let g = Game::new_int32_participation(Game::LATEST_VERSION, &part);
    part.borrow_mut().hosts.insert(7);
    assert!(g.is_host(Some(7)));
    let mut g = g;
    g.undo(0);
    g.add_player(5, "Late", 1);
    g.restore_recently_undone();
    assert!(g.is_participant(5));
    g.dismiss_recently_undone();
    g.set_or_unset_host(9);
    assert!(part.borrow().hosts.contains(&9));
}

/// Dismiss forgets the previous game, so repeated undos keep a bounded chain of games.
#[test]
fn repeated_undos_keep_a_bounded_number_of_games() {
    let mut g = base();
    for _ in 0..200 {
        g.undo(1);
        g.restore_recently_undone();
        g.undo(1);
        g.dismiss_recently_undone();
    }
    let mut depth = 0;
    let mut p = &g.previous_game_state;
    while let Some(prev) = p {
        depth += 1;
        p = &prev.previous_game_state;
    }
    assert_eq!(depth, 0);
    same_dump("after 200 rounds", &dump(&mut g), &dump(&mut game_after(&[ESTABLISH])));
}
