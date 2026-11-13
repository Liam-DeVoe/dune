//! Loading fixtures and bot-game traces for view tests.

#![allow(dead_code)]

use std::path::PathBuf;

use treachery_engine::compat::cs::Shared;
use treachery_engine::game::Participation;
use treachery_engine::prelude::*;
pub use treachery_view::build::Viewer;
use treachery_view::replica::ViewReplica;

pub fn fixture(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scripts/fixtures").join(format!("{name}.json"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// The user id seated in a seat: seat + 1.
pub fn user_of_seat(seat: i32) -> i32 {
    seat + 1
}

pub const OBSERVER: Viewer = Viewer { user_id: 100 };

/// Replays a fixture and seats one user per player (user id = seat + 1, named "Player <seat>"),
/// with `host_seat`'s user as host, and user 100 as an observer.
pub fn load(name: &str, host_seat: i32) -> ViewReplica {
    load_text(&fixture(name), host_seat)
}

pub fn viewer_of(replica: &ViewReplica, faction: Faction) -> Viewer {
    let p = replica.game.get_player(Some(faction)).expect("faction in game");
    Viewer { user_id: user_of_seat(replica.game.player(p).seat) }
}

/// The fixture's savegame cut back to its first `n` events (negative: all but the last `-n`).
pub fn truncated(name: &str, n: i64) -> String {
    let mut v: serde_json::Value = serde_json::from_str(&fixture(name)).unwrap();
    let events = v["_events"].as_array_mut().unwrap();
    let keep = if n < 0 { events.len() as i64 + n } else { n } as usize;
    events.truncate(keep);
    v.to_string()
}

/// Like [`load`], from savegame text.
pub fn load_text(savegame: &str, host_seat: i32) -> ViewReplica {
    let participation = Shared::new(Participation::default());
    let replica = ViewReplica::load(savegame, &participation).expect("savegame replays");
    seat_everyone(&replica, &participation, host_seat);
    replica
}

/// A bot-game trace of the oracle (`rust/traces/sample/bots-*.jsonl.gz`): the game version and
/// the accepted events in savegame JSON.
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
            Some("step") if v["result"] == "accepted" => events.push(v["event"].to_string()),
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

/// The bot traces of `rust/traces/sample`, and three more in `tests/data` that reach what the
/// sample doesn't: a discovery token its faction may identify (`bots-3141-00002`), Ecaz with a
/// loyal leader and `SetShipmentPermission` (`bots-3141-00103`, cut after 128 events), and
/// Moritani terror on the planet and `LoserConcluded` (`bots-3141-00114`, cut after 245 events).
/// All from `Treachery.Oracle bots --seed 3141 --dump none --hash none --gzip true` (the last two
/// with `--versions latest`).
pub fn sample_bot_traces() -> Vec<PathBuf> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut files = bot_traces_in(&root.join("../traces/sample"));
    files.extend(bot_traces_in(&root.join("tests/data")));
    files
}

/// The first state of any sample bot game for which `pred` holds, as a replica whose players
/// are seated like [`load`]'s (user id = seat + 1, seat 0 host, user 100 observing). Seating
/// happens after the replay, so the bots' game runs as recorded.
pub fn first_bot_state(what: &str, pred: impl Fn(&Game) -> bool) -> ViewReplica {
    for path in sample_bot_traces() {
        let trace = read_trace(&path);
        let participation = Shared::new(Participation::default());
        let mut r = ViewReplica::new(trace.version, &participation);
        for e in &trace.events {
            r.apply(e).unwrap();
            if pred(&r.game) {
                seat_everyone(&r, &participation, 0);
                return r;
            }
        }
    }
    panic!("no sample bot game reaches a state with {what}")
}

fn seat_everyone(replica: &ViewReplica, participation: &Shared<Participation>, host_seat: i32) {
    let mut p = participation.borrow_mut();
    for pl in &replica.game.players {
        p.seated_players.add(user_of_seat(pl.seat), pl.seat);
        p.player_names.add(user_of_seat(pl.seat), format!("Player {}", pl.seat));
    }
    p.hosts.add(user_of_seat(host_seat));
    p.observers.add(OBSERVER.user_id);
}

/// A fixture with extra rules in its `EstablishPlayers` settings (rule numbers).
pub fn with_rules(name: &str, rules: &[i32]) -> String {
    let mut v: serde_json::Value = serde_json::from_str(&fixture(name)).unwrap();
    let initial = v["_events"][0]["Settings"]["InitialRules"].as_array_mut().expect("InitialRules");
    initial.extend(rules.iter().map(|r| serde_json::json!(r)));
    v.to_string()
}
