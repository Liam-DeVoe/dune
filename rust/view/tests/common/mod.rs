//! Loading fixtures and bot-game traces for view tests.

#![allow(dead_code)]

use std::path::PathBuf;

use treachery_engine::prelude::*;
pub use treachery_view::build::Viewer;
use treachery_view::cache::ViewedGame;

pub fn fixture(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scripts/fixtures").join(format!("{name}.json"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// The user id seated in a seat: seat + 1.
pub fn user_of_seat(seat: Seat) -> i64 {
    i64::from(seat.number()) + 1
}

/// The seat whose user hosts the games of [`load`].
pub const HOST_SEAT: Seat = Seat::new(0);

pub const OBSERVER: Viewer = Viewer { user_id: 100, seat: None, is_host: false, is_observer: true };

/// The user in a seat, as [`load`] seats them.
pub fn seated_viewer(seat: Seat) -> Viewer {
    Viewer { user_id: user_of_seat(seat), seat: Some(seat), is_host: seat == HOST_SEAT, is_observer: false }
}

/// Replays a fixture and seats one user per player (user id = seat + 1, named "Player <seat>"),
/// with the user of [`HOST_SEAT`] as host, and user 100 as an observer.
pub fn load(name: &str) -> ViewedGame {
    load_text(&fixture(name))
}

pub fn viewer_of(viewed: &ViewedGame, faction: Faction) -> Viewer {
    let p = viewed.game.get_player(Some(faction)).expect("faction in game");
    seated_viewer(viewed.game.player(p).seated())
}

/// The fixture's savegame cut back to its first `n` events (negative: all but the last `-n`).
pub fn truncated(name: &str, n: i64) -> String {
    let mut v: serde_json::Value = serde_json::from_str(&fixture(name)).unwrap();
    let events = v["events"].as_array_mut().unwrap();
    let keep = if n < 0 { events.len() as i64 + n } else { n } as usize;
    events.truncate(keep);
    v.to_string()
}

/// A game loaded from a savegame, with nobody seated.
pub fn load_raw(savegame: &str) -> Result<ViewedGame, String> {
    ViewedGame::load(savegame)
}

/// An event as the client sends it (`{"type":..}`), checked to decode.
pub fn draft(event: &str) -> String {
    let e: Event = serde_json::from_str(event).unwrap_or_else(|e| panic!("draft reads: {e}: {event}"));
    serde_json::to_string(&e).unwrap()
}

/// A history entry of a fixture or trace, as `ViewedGame::apply` takes it.
pub fn event_text(e: &serde_json::Value) -> String {
    let entry: HistoryEntry = serde_json::from_value(e.clone()).expect("event reads");
    serde_json::to_string(&entry).unwrap()
}

/// Like [`load`], from savegame text.
pub fn load_text(savegame: &str) -> ViewedGame {
    let mut viewed = load_raw(savegame).expect("savegame replays");
    seat_everyone(&mut viewed);
    viewed
}

/// A recorded game (`rust/testdata/games/*.jsonl.gz`): the game version and the accepted events
/// in savegame JSON.
///
/// The files are JSON Lines: a `header` line (`gameVersion`, `participation`, the configuration),
/// then one line per call on the game: `step` (an event with `validate`, `isHost`, `source`,
/// `result` and the `queries` made before it), `undo` (`untilEventNr`), `restoreRecentlyUndone`,
/// `dismissRecentlyUndone`, and an `end` line with the `reason` the game stopped.
pub struct Trace {
    pub name: String,
    pub version: i32,
    pub events: Vec<String>,
}

pub fn read_trace(path: &std::path::Path) -> Trace {
    use std::io::{BufRead, BufReader};
    let file = std::fs::File::open(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let reader: Box<dyn BufRead> = if path.extension().is_some_and(|e| e == "gz") {
        Box::new(BufReader::new(flate2::read::GzDecoder::new(file)))
    } else {
        Box::new(BufReader::new(file))
    };
    let mut version = 0;
    let mut events = vec![];
    for line in reader.lines() {
        let v: serde_json::Value = serde_json::from_str(&line.unwrap()).unwrap();
        match v["kind"].as_str() {
            Some("header") => version = v["gameVersion"].as_i64().unwrap() as i32,
            Some("step") if v["result"] == "accepted" => events.push(event_text(&v["event"])),
            Some("undo" | "restoreRecentlyUndone" | "dismissRecentlyUndone") => panic!("{}: bot traces have no undo", path.display()),
            _ => {}
        }
    }
    Trace { name: path.file_name().unwrap().to_string_lossy().to_string(), version, events }
}

/// The bot traces in a directory.
pub fn bot_traces_in(dir: &std::path::Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .map(|e| e.unwrap().path())
        .filter(|p| p.file_name().unwrap().to_string_lossy().starts_with("bots-"))
        .collect();
    files.sort();
    files
}

/// The bot games of `rust/testdata/games`, and four more in `tests/data` that reach what those
/// don't: a discovery token its faction may identify (`bots-3141-00002`), Ecaz with a
/// loyal leader and `SetShipmentPermission` (`bots-3141-00103`, cut after 128 events), Moritani
/// terror on the planet (`bots-3141-00114`, cut after 245 events), and `LoserConcluded`
/// (`bots-1-00101`, `observe record --mode bots --seed 1 --game 101` at game version 189,
/// converted with `rust/tools/record-to-test-trace.py` and cut after 293 events). The 3141 games
/// were refitted to version 189 (`observe refit`); `bots-3141-00114` no longer reaches
/// `LoserConcluded` since.
pub fn sample_bot_traces() -> Vec<PathBuf> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut files = bot_traces_in(&root.join("../testdata/games"));
    files.extend(bot_traces_in(&root.join("tests/data")));
    files
}

/// The first state of any sample bot game for which `pred` holds, as a game whose players
/// are seated like [`load`]'s (user id = seat + 1, seat 0 host, user 100 observing). Seating
/// happens after the replay, so the bots' game runs as recorded.
pub fn first_bot_state(what: &str, pred: impl Fn(&Game) -> bool) -> ViewedGame {
    for path in sample_bot_traces() {
        let trace = read_trace(&path);
        let mut r = ViewedGame::new(trace.version);
        for e in &trace.events {
            r.apply(e).unwrap();
            if pred(&r.game) {
                seat_everyone(&mut r);
                return r;
            }
        }
    }
    panic!("no sample bot game reaches a state with {what}")
}

fn seat_everyone(viewed: &mut ViewedGame) {
    let seats: Vec<Seat> = viewed.game.players.iter().map(|pl| pl.seated()).collect();
    for seat in &seats {
        viewed.game.human_seats.insert(*seat);
        viewed.seating.names.insert(*seat, format!("Player {seat}"));
    }
    viewed.seating.seated_users = seats.len() as i32;
}

/// A fixture with extra rules in its `EstablishPlayers` settings.
pub fn with_rules(name: &str, rules: &[Rule]) -> String {
    let mut v: serde_json::Value = serde_json::from_str(&fixture(name)).unwrap();
    let initial = v["events"][0]["event"]["settings"]["initialRules"].as_array_mut().expect("initialRules");
    initial.extend(rules.iter().map(|r| serde_json::to_value(r).unwrap()));
    v.to_string()
}
