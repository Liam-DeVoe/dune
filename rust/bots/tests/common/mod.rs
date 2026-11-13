//! Snapshot files for the bot tests: reading and rewriting `tests/golden`, replaying the recorded
//! games, and comparing probe results with the snapshot.
//!
//! A snapshot is the bots' current behavior, not a specification. After a deliberate change to
//! the bots or the engine, rewrite the snapshots and review the diff:
//!
//! ```sh
//! UPDATE_GOLDENS=1 cargo test --release -p treachery-bots
//! ```
//!
//! The update rewrites only the results (`r`, `d` and the like). The games and the probes, which
//! say what to call where, stay as they are.

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

use flate2::read::GzDecoder;
use serde_json::{json, Value};
use treachery_bots::prelude::*;
use treachery_engine::error::catch_panic;
use treachery_engine::game::Participation;

/// Whether this run rewrites the snapshots instead of comparing with them.
pub fn updating() -> bool {
    std::env::var("UPDATE_GOLDENS").is_ok()
}

pub fn golden_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden").join(name)
}

/// The lines of a gzipped JSON Lines file in `tests/golden`.
pub fn lines(name: &str) -> Vec<Value> {
    let path = golden_path(name);
    let file = std::fs::File::open(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    BufReader::new(GzDecoder::new(file)).lines().map(|l| serde_json::from_str(&l.unwrap()).unwrap()).collect()
}

pub fn write_lines(name: &str, lines: &[Value]) {
    let file = std::fs::File::create(golden_path(name)).unwrap();
    let mut w = flate2::write::GzEncoder::new(file, flate2::Compression::best());
    for l in lines {
        writeln!(w, "{}", serde_json::to_string(l).unwrap()).unwrap();
    }
    w.finish().unwrap();
}

/// A JSON file in `tests/golden`.
pub fn read_json(name: &str) -> Value {
    let path = golden_path(name);
    serde_json::from_str(&std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))).unwrap()
}

pub fn write_json(name: &str, v: &Value) {
    std::fs::write(golden_path(name), serde_json::to_string_pretty(v).unwrap() + "\n").unwrap();
}

/// The game's `Participation`, as the header of a recorded game holds it.
pub fn participation(o: &Value) -> Participation {
    let int = |v: &Value| v.as_i64().unwrap() as i32;
    let mut p = Participation { bots_speed: int(&o["BotsSpeed"]), ..Default::default() };
    for kv in o["PlayerNames"].as_array().unwrap() {
        p.player_names.insert(int(&kv[0]), kv[1].as_str().unwrap().to_string());
    }
    for kv in o["SeatedPlayers"].as_array().unwrap() {
        p.seated_players.insert(int(&kv[0]), int(&kv[1]));
    }
    for (key, set) in [
        ("Observers", &mut p.observers),
        ("Hosts", &mut p.hosts),
        ("AvailableSeats", &mut p.available_seats),
        ("Kicked", &mut p.kicked),
    ] {
        for v in o[key].as_array().unwrap() {
            set.insert(int(v));
        }
    }
    p
}

/// Replays a recorded game, `knowledge-<name>.jsonl.gz`, and calls `at_step` with the game and
/// the step number after each event.
pub fn replay(name: &str, mut at_step: impl FnMut(&mut Game, i64)) {
    let mut game: Option<Game> = None;
    for o in lines(&format!("knowledge-{name}.jsonl.gz")) {
        match o["k"].as_str().unwrap() {
            "header" => {
                let participation = participation(&o["participation"]);
                game = Some(Game::new_int32_participation(o["version"].as_i64().unwrap() as i32, &Shared::new(participation)));
            }
            "step" => {
                let game = game.as_mut().unwrap();
                let e: GameEvent = treachery_engine::json::deserialize(o["event"].as_str().unwrap()).unwrap();
                e.initialize_game(game);
                game.map.mark_neighbour_cache_stale();
                let (validate, is_host) = (o["validate"].as_bool().unwrap(), o["isHost"].as_bool().unwrap());
                let _ = catch_panic(|| e.execute(game, validate, is_host));
                at_step(game, o["n"].as_i64().unwrap());
            }
            _ => {}
        }
    }
}

/// A fresh bot for a probe's player, with the probe's seed.
pub fn bot_for(game: &Game, probe: &Value) -> ClassicBot {
    game.map.mark_neighbour_cache_stale();
    let player = game.player_at(probe["p"].as_i64().unwrap() as usize);
    let faction = game.player(player).faction;
    let seed = probe["seed"].as_i64().unwrap() as i32;
    ClassicBot::new(player, BotParameters::for_faction(faction), SharedRandom::from_seed(seed))
}

/// The probe's arguments.
pub fn args(probe: &Value) -> Vec<String> {
    probe["a"].as_array().unwrap().iter().map(|v| v.as_str().unwrap().to_string()).collect()
}

/// The lines of a probe file: each probe names a step `n`, a player `p`, a bot seed, a method
/// `m` and its arguments `a`, and holds the result `r` (`EX` when the call panics) and the bot's
/// draw count `d` after the call. Lines without `m` (the events of a recorded game) pass through.
pub struct Probes {
    file: String,
    lines: Vec<Value>,
    by_step: BTreeMap<i64, Vec<usize>>,
    count: usize,
    failures: Vec<String>,
}

impl Probes {
    pub fn load(file: &str) -> Self {
        let lines = lines(file);
        let mut by_step: BTreeMap<i64, Vec<usize>> = BTreeMap::new();
        for (i, o) in lines.iter().enumerate() {
            if o.get("m").is_some() {
                by_step.entry(o["n"].as_i64().unwrap()).or_default().push(i);
            }
        }
        Probes { file: file.to_string(), lines, by_step, count: 0, failures: vec![] }
    }

    /// The indices of the probes made after step `n`, in file order.
    pub fn at(&self, n: i64) -> Vec<usize> {
        self.by_step.get(&n).cloned().unwrap_or_default()
    }

    pub fn probe(&self, i: usize) -> &Value {
        &self.lines[i]
    }

    /// Compares a probe's result and draw count with the snapshot, or records them when updating.
    /// Returns whether they matched.
    pub fn check(&mut self, i: usize, actual: &str, draws: i64) -> bool {
        self.count += 1;
        let p = &mut self.lines[i];
        if updating() {
            p["r"] = json!(actual);
            p["d"] = json!(draws);
            return true;
        }
        let (expected, expected_draws) = (p["r"].as_str().unwrap(), p["d"].as_i64().unwrap());
        let ok = actual == expected && draws == expected_draws;
        if !ok {
            self.failures.push(format!(
                "step {} player {} {}({}): expected {expected} ({expected_draws} draws), actual {actual} ({draws} draws)",
                p["n"], p["p"], p["m"].as_str().unwrap(), args(p).join(", ")
            ));
        }
        ok
    }

    /// Compares an extra number the probe records (`key`) with the snapshot, or records it.
    pub fn check_number(&mut self, i: usize, key: &str, actual: i64) {
        let p = &mut self.lines[i];
        if updating() {
            p[key] = json!(actual);
        } else if p[key].as_i64() != Some(actual) {
            self.failures.push(format!("step {} player {} {}: expected {key} {}, actual {actual}", p["n"], p["p"], p["m"].as_str().unwrap(), p[key]));
        }
    }

    /// Writes the snapshot when updating, otherwise fails with the differences.
    pub fn finish(self) {
        assert!(self.count > 0, "{}: no probes", self.file);
        if updating() {
            write_lines(&self.file, &self.lines);
            return;
        }
        if !self.failures.is_empty() {
            panic!(
                "{}: {} of {} probes differ:\n{}",
                self.file,
                self.failures.len(),
                self.count,
                self.failures.iter().take(40).cloned().collect::<Vec<_>>().join("\n")
            );
        }
    }
}

/// Runs a probe call: its result, or `EX` when it panics.
pub fn result_of(f: impl FnOnce() -> String) -> String {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| catch_panic(f))).ok().and_then(Result::ok).unwrap_or_else(|| "EX".to_string())
}

/// Whether consecutive probes of one player after a step share a bot.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Bots {
    /// Every probe gets a fresh bot with its seed, so one call can't affect the next.
    Fresh,
    /// The probes of a player after a step run in order on one bot (created with the first
    /// probe's seed), so a decision sees the state the previous one left. After a difference, the
    /// later probes of that bot are skipped, because they depend on its state and draws.
    Shared,
}

/// Replays the game `knowledge-<name>.jsonl.gz` and, after each step, runs the probes of
/// `<prefix>-<name>.jsonl.gz` made at that step through `call`, which returns the result text.
/// A probe that panics has the result `EX`. A probe with a `messages` field also records how
/// many messages the current report holds after the call.
pub fn run_probes(prefix: &str, name: &str, bots: Bots, mut call: impl FnMut(&mut Game, &mut ClassicBot, &str, &[String]) -> String) {
    let mut probes = Probes::load(&format!("{prefix}-{name}.jsonl.gz"));
    replay(name, |game, n| {
        let mut bot: Option<(i64, ClassicBot)> = None;
        let mut skip: Option<i64> = None;
        for i in probes.at(n) {
            let p = probes.probe(i).clone();
            let pi = p["p"].as_i64().unwrap();
            if skip == Some(pi) {
                continue;
            }
            if bots == Bots::Fresh || !matches!(&bot, Some((bp, _)) if *bp == pi) {
                bot = Some((pi, bot_for(game, &p)));
            }
            let b = &mut bot.as_mut().unwrap().1;
            let (m, a) = (p["m"].as_str().unwrap(), args(&p));
            let actual = result_of(|| call(game, b, m, &a));
            let draws = b.random.get_count() as i64;
            if !probes.check(i, &actual, draws) && bots == Bots::Shared {
                skip = Some(pi);
            }
            if p.get("messages").is_some() {
                probes.check_number(i, "messages", game.current_report().messages.len() as i64);
            }
        }
    });
    probes.finish();
}
