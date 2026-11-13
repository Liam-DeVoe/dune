//! Reading and writing event and savegame JSON, against snapshots.
//!
//! `tests/golden/events.jsonl` holds every event type with randomized members, plus malformed
//! variants, each with whether the engine accepts it and what it writes back. Savegames and the
//! client's events go through this format, so the engine must keep accepting exactly these
//! inputs and write them back byte for byte.
//!
//! `tests/golden/event_errors.jsonl` holds events whose `$type` is no longer a known event (old
//! savegames after an event was renamed), with the error the engine reports.
//!
//! After a deliberate change, rewrite both files and review the diff:
//!
//! ```sh
//! UPDATE_GOLDENS=1 cargo test --release -p treachery-engine --test event_json
//! ```

use std::path::PathBuf;

use serde_json::{json, Value};
use treachery_engine::events::GameEvent;
use treachery_engine::game::GameState;
use treachery_engine::json;
use treachery_engine::json::utilities::Utilities;

fn path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden").join(name)
}

fn read(name: &str) -> Vec<Value> {
    std::fs::read_to_string(path(name)).unwrap().lines().map(|l| serde_json::from_str(l).unwrap()).collect()
}

fn write(name: &str, cases: &[Value]) {
    let text: String = cases.iter().map(|c| serde_json::to_string(c).unwrap() + "\n").collect();
    std::fs::write(path(name), text).unwrap();
}

fn updating() -> bool {
    std::env::var_os("UPDATE_GOLDENS").is_some()
}

#[test]
fn events_round_trip() {
    let mut cases = read("events.jsonl");
    let mut failures = Vec::new();
    for (i, case) in cases.iter_mut().enumerate() {
        let input = case["json"].as_str().unwrap().to_string();
        let result = match case["kind"].as_str().unwrap() {
            "event" => json::deserialize::<GameEvent>(&input).map(|e| json::serialize(&e)),
            "state" => json::deserialize::<GameState>(&input).map(|s| json::serialize(&s)),
            k => panic!("unknown kind {k}"),
        };
        let (ok, out) = match &result {
            Ok(out) => (true, json!(out)),
            Err(_) => (false, Value::Null),
        };
        if updating() {
            case["ok"] = json!(ok);
            case["out"] = out;
            continue;
        }
        match (case["ok"].as_bool().unwrap(), result) {
            (true, Ok(out)) => {
                let expected = case["out"].as_str().unwrap();
                if out != expected {
                    failures.push(format!("line {}: output differs\n  in:       {input}\n  expected: {expected}\n  actual:   {out}", i + 1));
                }
            }
            (true, Err(e)) => failures.push(format!("line {}: rejected ({e:?}), expected accepted: {input}", i + 1)),
            (false, Ok(out)) => failures.push(format!("line {}: accepted, expected rejected: {input}\n  output: {out}", i + 1)),
            (false, Err(_)) => {}
        }
    }
    assert!(cases.len() > 1000, "corpus too small");
    if updating() {
        write("events.jsonl", &cases);
        return;
    }
    if !failures.is_empty() {
        let n = failures.len();
        failures.truncate(15);
        panic!("{n} of {} cases differ:\n{}", cases.len(), failures.join("\n"));
    }
}

#[test]
fn unknown_event_types_fail() {
    let mut cases = read("event_errors.jsonl");
    for case in cases.iter_mut() {
        let (kind, input) = (case["kind"].as_str().unwrap().to_string(), case["json"].as_str().unwrap().to_string());
        // The paths the server reads savegames and events through.
        let result = match kind.as_str() {
            "state" => GameState::load(&input).map(drop),
            "event" => Utilities::deserialize::<GameEvent>(&input).map(drop),
            k => panic!("unknown kind {k}"),
        };
        let error = result.err().map(|e| format!("{}: {}", e.kind, e.message)).unwrap_or_default();
        // json::deserialize reports the same error.
        let result = match kind.as_str() {
            "state" => json::deserialize::<GameState>(&input).map(|_| ()),
            _ => json::deserialize::<GameEvent>(&input).map(|_| ()),
        };
        let e = result.unwrap_err();
        assert_eq!(format!("{}: {}", e.kind, e.message), error, "input: {input}");
        if updating() {
            case["error"] = json!(error);
        } else {
            assert_eq!(error, case["error"].as_str().unwrap(), "input: {input}");
        }
    }
    assert!(cases.len() >= 10);
    if updating() {
        write("event_errors.jsonl", &cases);
    }
}
