//! Skin JSON as snapshots: `tests/golden/default_skin.json` is the default skin serialized with
//! `skin_to_string`, and `tests/golden/load.jsonl.gz` holds skin files (valid, partial and
//! broken) with what `Skin::load` makes of each, serialized again, or `!<error type>`.
//!
//! After a deliberate change, rewrite the snapshots and review the diff:
//!
//!     UPDATE_GOLDENS=1 cargo test --release -p treachery-skin --test skin_json

mod common;

use common::*;
use serde_json::Value;
use treachery_skin::{default_skin, Skin};

#[test]
fn default_skin_serializes_like_the_snapshot() {
    let actual = default_skin().skin_to_string();
    if updating() {
        std::fs::write(golden("default_skin.json"), &actual).unwrap();
        return;
    }
    let expected = std::fs::read_to_string(golden("default_skin.json")).unwrap();
    assert_eq!(actual, expected);
}

#[test]
fn load_matches_the_snapshot() {
    let mut diffs = Diffs::new();
    let mut lines = Vec::new();
    for line in read_lines(&golden("load.jsonl.gz")) {
        let mut v: Value = serde_json::from_str(&line).unwrap();
        let input = v["in"].as_str().unwrap().to_string();
        let actual = match Skin::load(&input, default_skin()) {
            Ok(s) => s.skin_to_string(),
            Err(e) => format!("!{}", e.type_name),
        };
        diffs.check(&format!("Load({input})"), v["out"].as_str().unwrap(), &actual);
        v["out"] = Value::String(actual);
        lines.push(v);
    }
    if updating() {
        write_lines(&golden("load.jsonl.gz"), &lines);
        return;
    }
    diffs.finish("Skin.Load");
}
