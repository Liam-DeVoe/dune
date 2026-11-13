//! Snapshots of the log lines the view renders: every message of the scene fixtures that
//! `tests/golden/log_lines.jsonl.gz` lists, and the messages of the bot games that
//! `log_lines_bots.jsonl.gz` lists (the first two of each shape, record type and part kinds, which
//! reach records and parts the fixtures don't). `tests/data/log` holds two games for
//! `WormSentLog` and `ShipMoveLoss`.
//!
//! Each line holds the rendered parts, the message's target and heading, and its `record`,
//! `route`, `plans` and `fact`. Its `dom` field is the text the client must show for the parts;
//! `npm run check:rich-text` in `web/` renders the parts with the React component and compares.
//! The snapshots were first written from the C# client's rendering, so they hold what players
//! saw there.
//!
//! After a deliberate change to log rendering, rewrite them and review the diff:
//!
//!     UPDATE_LOG_GOLDENS=1 cargo test --release -p treachery-view --test log_lines
//!
//! The update keeps each line's `dom`. A line that is new (its fixture has more messages now)
//! gets an empty `dom`, which `check:rich-text` reports with the text React shows; check it and
//! copy it in.

mod common;

use std::collections::BTreeMap;
use std::io::{BufRead, Write};
use std::path::PathBuf;

use serde_json::{json, Value};
use treachery_engine::shared::Shared;
use treachery_engine::game::Participation;
use treachery_engine::messages::message::Message;
use treachery_view::describe::InGame;
use treachery_view::log::log_line;
use treachery_view::replica::ViewReplica;

fn golden_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden").join(name)
}

fn read_golden(name: &str) -> Vec<Value> {
    let f = std::fs::File::open(golden_path(name)).unwrap();
    std::io::BufReader::new(flate2::read::GzDecoder::new(f)).lines().map(|l| serde_json::from_str(&l.unwrap()).unwrap()).collect()
}

fn write_golden(name: &str, lines: &[Value]) {
    let f = std::fs::File::create(golden_path(name)).unwrap();
    let mut w = flate2::write::GzEncoder::new(f, flate2::Compression::best());
    for l in lines {
        writeln!(w, "{}", serde_json::to_string(l).unwrap()).unwrap();
    }
    w.finish().unwrap();
}

fn updating() -> bool {
    std::env::var("UPDATE_LOG_GOLDENS").is_ok()
}

/// Numbers as f64, so that `3` and `3.0` compare equal.
fn normalize(v: &Value) -> Value {
    match v {
        Value::Number(n) => json!(n.as_f64().unwrap()),
        Value::Array(a) => Value::Array(a.iter().map(normalize).collect()),
        Value::Object(o) => Value::Object(o.iter().map(|(k, v)| (k.clone(), normalize(v))).collect()),
        _ => v.clone(),
    }
}

const RENDERED: [&str; 6] = ["text", "heading", "record", "route", "plans", "fact"];

/// The snapshot line for `m`: `source` names the fixture or trace, `dom` is carried over.
fn snapshot(source: (&str, &str), report: usize, line: usize, m: &Message, d: &InGame, dom: &str) -> Value {
    let rendered = serde_json::to_value(log_line(m, d)).unwrap();
    let mut out = serde_json::Map::new();
    out.insert(source.0.into(), json!(source.1));
    out.insert("report".into(), json!(report));
    out.insert("line".into(), json!(line));
    out.insert("target".into(), json!(m.target.to_string()));
    for key in RENDERED {
        out.insert(key.into(), rendered[key].clone());
    }
    out.insert("dom".into(), json!(dom));
    Value::Object(out)
}

/// The differences between a snapshot line and the one rendered now, if any.
fn compare(expected: &Value, actual: &Value) -> Option<String> {
    let diffs: Vec<String> = RENDERED
        .iter()
        .chain(&["target"])
        .filter(|k| normalize(&actual[**k]) != normalize(&expected[**k]))
        .map(|k| format!("  {k}:\n    now      {}\n    snapshot {}", actual[*k], expected[*k]))
        .collect();
    (!diffs.is_empty()).then(|| format!("({}):\n{}", expected["dom"], diffs.join("\n")))
}

fn dom_of(lines: &[&Value]) -> BTreeMap<(u64, u64), String> {
    lines.iter().map(|g| ((g["report"].as_u64().unwrap(), g["line"].as_u64().unwrap()), g["dom"].as_str().unwrap_or("").to_string())).collect()
}

fn report(failures: &[String], checked: usize) {
    assert!(
        failures.is_empty(),
        "{} of {checked} log lines differ from the snapshot (rewrite with UPDATE_LOG_GOLDENS=1 if the change is deliberate):\n{}",
        failures.len(),
        failures.iter().take(20).cloned().collect::<Vec<_>>().join("\n")
    );
}

#[test]
fn fixture_log_lines_match_the_snapshot() {
    let golden = read_golden("log_lines.jsonl.gz");
    let mut fixtures: Vec<&str> = golden.iter().map(|g| g["fixture"].as_str().unwrap()).collect();
    fixtures.dedup();
    let mut failures = Vec::new();
    let mut written = Vec::new();
    for fixture in fixtures {
        let replica = ViewReplica::load(&common::fixture(fixture), &Shared::new(Participation::default())).unwrap();
        let skin = treachery_skin::default_skin();
        let d = InGame::new(&skin, &replica.game);
        let expected: Vec<&Value> = golden.iter().filter(|g| g["fixture"] == fixture).collect();
        let dom = dom_of(&expected);
        let actual: Vec<Value> = replica
            .game
            .reports()
            .iter()
            .enumerate()
            .flat_map(|(r, rep)| rep.messages.iter().enumerate().map(move |(i, m)| (r, i, m)))
            .map(|(r, i, m)| snapshot(("fixture", fixture), r, i, m, &d, dom.get(&(r as u64, i as u64)).map_or("", String::as_str)))
            .collect();
        if actual.len() != expected.len() {
            failures.push(format!("{fixture}: {} messages, the snapshot has {}", actual.len(), expected.len()));
        }
        for (a, e) in actual.iter().zip(&expected) {
            if let Some(f) = compare(e, a) {
                failures.push(format!("{fixture} report {} line {} {f}", a["report"], a["line"]));
            }
        }
        written.extend(actual);
    }
    if updating() {
        write_golden("log_lines.jsonl.gz", &written);
        return;
    }
    assert!(written.len() > 5000, "only {} lines", written.len());
    report(&failures, written.len());
}

#[test]
fn bot_game_log_lines_match_the_snapshot() {
    let golden = read_golden("log_lines_bots.jsonl.gz");
    let mut failures = Vec::new();
    let mut written = Vec::new();
    let mut paths = common::sample_bot_traces();
    paths.extend(common::bot_traces_in(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data/log")));
    for path in paths {
        let trace = common::read_trace(&path);
        let expected: Vec<&Value> = golden.iter().filter(|g| g["trace"] == trace.name.as_str()).collect();
        if expected.is_empty() {
            continue; // every shape of this game came earlier
        }
        let mut replica = ViewReplica::new(trace.version, &Shared::new(Participation::default()));
        for e in &trace.events {
            replica.apply(e).unwrap();
        }
        let skin = treachery_skin::default_skin();
        let d = InGame::new(&skin, &replica.game);
        for g in expected {
            let (r, i) = (g["report"].as_u64().unwrap() as usize, g["line"].as_u64().unwrap() as usize);
            let m = replica.game.reports()[r].messages.iter().nth(i).expect("message");
            let actual = snapshot(("trace", &trace.name), r, i, m, &d, g["dom"].as_str().unwrap_or(""));
            if let Some(f) = compare(g, &actual) {
                failures.push(format!("{} report {r} line {i} {f}", trace.name));
            }
            written.push(actual);
        }
    }
    if updating() {
        write_golden("log_lines_bots.jsonl.gz", &written);
        return;
    }
    assert!(golden.len() > 100);
    assert_eq!(written.len(), golden.len(), "a trace named in the snapshot is missing");
    report(&failures, written.len());
}
