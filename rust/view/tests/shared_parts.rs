//! Views built together (`ViewCache::batch`) share the parts that don't read the viewer, and a
//! view cache keeps the parts made from the rules between game states. Their JSON must be the
//! JSON of each view built on its own, after any change to the game: events, an undo and a
//! restore, a rejected event, and an event that failed after it changed the game.

mod common;

use common::{read_trace, sample_bot_traces, Viewer, OBSERVER};
use treachery_engine::events::rules::SubmitError;
use treachery_engine::prelude::*;
use treachery_view::build::{build_view, Seating};
use treachery_view::cache::ViewCache;
use treachery_view::describe::default_describer;

const TIMER: i32 = 30;

/// Every player seated, a seatless host and an observer.
fn viewers(game: &Game) -> Vec<Viewer> {
    let mut v: Vec<Viewer> = game.players.iter().map(|p| Viewer { user_id: i64::from(p.seated().number()) + 1, seat: Some(p.seated()), is_host: false, is_observer: false }).collect();
    v.push(Viewer { user_id: 99, seat: None, is_host: true, is_observer: true });
    v.push(OBSERVER);
    v
}

/// The views of `game` built together with `cache` equal the views built alone.
fn check(cache: &mut ViewCache, game: &Game, s: &Seating, what: &str) {
    let d = default_describer();
    let viewers = viewers(game);
    let mut batch = cache.batch(game, s, TIMER);
    for v in viewers {
        let together = serde_json::to_string(&batch.view(v)).unwrap();
        let alone = serde_json::to_string(&build_view(game, v, s, d.as_ref(), TIMER)).unwrap();
        assert!(together == alone, "{what}, viewer {v:?}: the view built with the others differs from the view built alone");
    }
}

fn entry(json: &str) -> HistoryEntry {
    serde_json::from_str(json).unwrap()
}

#[test]
fn views_built_together_equal_views_built_alone() {
    let trace = read_trace(&sample_bot_traces()[0]);
    let mut game = Game::of_version(trace.version);
    let mut seating = Seating::default();
    // One cache for everything below, as the server keeps one per game.
    let mut cache = ViewCache::default();
    let (mut undone, mut rejected, mut failed) = (0, 0, 0);
    for (i, e) in trace.events.iter().enumerate() {
        game.play(entry(e), false, false).unwrap();
        if i == 0 {
            for p in &game.players {
                game.human_seats.insert(p.seated());
                seating.names.insert(p.seated(), format!("Player {}", p.seated()));
            }
            seating.seated_users = game.players.len() as i32;
        }
        check(&mut cache, &game, &seating, &format!("after event {i}"));

        // An event nobody may submit is rejected and changes nothing.
        let mut stranger = entry(e);
        stranger.by = Faction::ALL.iter().copied().find(|f| !game.is_playing(*f));
        if game.play(stranger, true, false).is_err() {
            rejected += 1;
            check(&mut cache, &game, &seating, &format!("after a rejected event at {i}"));
        }

        if i > 0 && i % 40 == 0 {
            // Undo a few events and come back.
            game.undo(i as i32 - 5);
            check(&mut cache, &game, &seating, &format!("after undoing to {}", i - 5));
            game.restore_recently_undone();
            check(&mut cache, &game, &seating, &format!("after restoring at {i}"));
            undone += 1;

            // A copy of the game plays later events out of order, unvalidated, until one fails
            // after it changed the game.
            let mut copy = game.new_like();
            for e in &trace.events[..=i] {
                copy.play(entry(e), false, false).unwrap();
            }
            for (k, e) in trace.events.iter().enumerate().skip(i + 2).take(30) {
                let r = copy.play(entry(e), false, false);
                check(&mut cache, &copy, &seating, &format!("after event {k} out of order at {i}"));
                if matches!(r, Err(SubmitError::Failed(_))) {
                    failed += 1;
                    break;
                }
            }
            check(&mut cache, &game, &seating, &format!("back to the game at {i}"));
        }
    }
    assert!(undone > 0 && rejected > 0 && failed > 0, "undone {undone}, rejected {rejected}, failed {failed}");
}
