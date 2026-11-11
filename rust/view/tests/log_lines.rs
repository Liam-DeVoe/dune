//! Every log line of every scene fixture, compared with the C# client's rendering.
//!
//! The golden file is written by `rust/view/csharp-ref` from the C# engine and `DefaultSkin`:
//! for each message, the parts `ExpressionComponent.razor` draws, its target and heading, and
//! `Message.Log`, `Route`, `Plans` and `Fact` serialised by reflection. The view must produce the
//! same JSON (numbers compared by value). The golden's `dom` field, the text the component shows,
//! is checked against the React client by `web/tools/check-rich-text.ts`.
//!
//! Regenerate after a change to the fixtures, the skin or the log records (from
//! `rust/view/csharp-ref`, with .NET 10 on the path):
//!
//!     dotnet build -c Release
//!     dotnet bin/Release/net10.0/ViewRef.dll log-lines ../../../scripts/fixtures ../tests/golden/log_lines.jsonl.gz
//!     ../../tools/heavy.sh dotnet bin/Release/net10.0/ViewRef.dll log-lines-traces ../tests/golden/log_lines_bots.jsonl.gz \
//!         ../../traces/sample/bots-*.jsonl.gz ../tests/data/bots-*.jsonl.gz ../tests/data/log/bots-*.jsonl.gz
//!
//! The second golden holds the first two messages of each shape (record type, part kinds) of
//! the bot games, which reach records and parts the fixtures don't. `tests/data/log` holds two
//! more games (`Treachery.Oracle bots --seed 777`, games 10 and 12) for `WormSentLog` and
//! `ShipMoveLoss`.

mod common;

use std::io::BufRead;
use std::path::PathBuf;

use serde_json::Value;
use treachery_engine::cs::Shared;
use treachery_engine::participation::Participation;
use treachery_view::describe::InGame;
use treachery_view::log::{log_line, NESTED_RECORD_TYPES, RECORD_TYPES};
use treachery_view::replica::ViewReplica;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn golden() -> Vec<Value> {
    read_golden("log_lines.jsonl.gz")
}

fn read_golden(name: &str) -> Vec<Value> {
    let f = std::fs::File::open(root().join("rust/view/tests/golden").join(name)).unwrap();
    std::io::BufReader::new(flate2::read::GzDecoder::new(f)).lines().map(|l| serde_json::from_str(&l.unwrap()).unwrap()).collect()
}

/// Numbers as f64, so that `3` and `3.0` compare equal.
fn normalize(v: &Value) -> Value {
    match v {
        Value::Number(n) => serde_json::json!(n.as_f64().unwrap()),
        Value::Array(a) => Value::Array(a.iter().map(normalize).collect()),
        Value::Object(o) => Value::Object(o.iter().map(|(k, v)| (k.clone(), normalize(v))).collect()),
        _ => v.clone(),
    }
}

#[test]
fn log_lines_match_csharp() {
    let golden = golden();
    let mut fixtures: Vec<&str> = golden.iter().map(|g| g["fixture"].as_str().unwrap()).collect();
    fixtures.dedup();
    let mut failures = Vec::new();
    let mut checked = 0;
    for fixture in fixtures {
        let text = std::fs::read_to_string(root().join(format!("scripts/fixtures/{fixture}.json"))).unwrap();
        let replica = ViewReplica::load(&text, &Shared::new(Participation::default())).unwrap();
        let skin = treachery_skin::default_skin();
        let d = InGame::new(&skin, &replica.game);
        let expected: Vec<&Value> = golden.iter().filter(|g| g["fixture"] == fixture).collect();
        let reports = replica.game.reports();
        let messages: Vec<(usize, usize, &treachery_engine::message::Message)> =
            reports.iter().enumerate().flat_map(|(r, rep)| rep.messages.iter().enumerate().map(move |(i, m)| (r, i, m))).collect();
        assert_eq!(messages.len(), expected.len(), "{fixture}: number of messages");
        for ((r, i, m), g) in messages.into_iter().zip(expected) {
            assert_eq!((g["report"].as_u64().unwrap() as usize, g["line"].as_u64().unwrap() as usize), (r, i), "{fixture}");
            if let Some(f) = compare(m, &d, g) {
                failures.push(format!("{fixture} report {r} line {i} {f}"));
            }
            checked += 1;
        }
    }
    assert!(checked > 5000, "only {checked} lines");
    assert!(failures.is_empty(), "{} of {checked} lines differ:\n{}", failures.len(), failures.iter().take(20).cloned().collect::<Vec<_>>().join("\n"));
}

/// The differences between the view's line for `m` and the golden line `g`, if any.
fn compare(m: &treachery_engine::message::Message, d: &InGame, g: &Value) -> Option<String> {
    let line = serde_json::to_value(log_line(m, d)).unwrap();
    let mut diffs = Vec::new();
    for key in ["text", "heading", "record", "route", "plans", "fact"] {
        if normalize(&line[key]) != normalize(&g[key]) {
            diffs.push(format!("  {key}:\n    rust {}\n    c#   {}", line[key], g[key]));
        }
    }
    if line["private"].as_bool().unwrap() != (g["target"] != "None") {
        diffs.push(format!("  private {} for target {}", line["private"], g["target"]));
    }
    (!diffs.is_empty()).then(|| format!("({}):\n{}", g["dom"], diffs.join("\n")))
}

#[test]
fn bot_game_log_lines_match_csharp() {
    let golden = read_golden("log_lines_bots.jsonl.gz");
    let mut failures = Vec::new();
    let mut paths = common::sample_bot_traces();
    paths.extend(common::bot_traces_in(&root().join("rust/view/tests/data/log")));
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
            if let Some(f) = compare(m, &d, g) {
                failures.push(format!("{} report {r} line {i} {f}", trace.name));
            }
        }
    }
    assert!(golden.len() > 100);
    assert!(failures.is_empty(), "{} lines differ:\n{}", failures.len(), failures.iter().take(20).cloned().collect::<Vec<_>>().join("\n"));
}

/// `RECORD_TYPES` must name every record declared in `Shared/Model/Log/`, so a record added in
/// C# fails here before it reaches a game.
#[test]
fn every_csharp_log_record_is_converted() {
    let dir = root().join("treachery-online/Shared/Model/Log");
    let mut declared = Vec::new();
    for entry in std::fs::read_dir(&dir).unwrap() {
        let text = std::fs::read_to_string(entry.unwrap().path()).unwrap();
        for line in text.lines() {
            let line = line.trim_start();
            if let Some(rest) = line.strip_prefix("public record ").or_else(|| line.strip_prefix("public sealed record ")) {
                let name: String = rest.chars().take_while(|c| c.is_alphanumeric()).collect();
                declared.push(name);
            }
        }
    }
    declared.sort();
    let mut ours: Vec<String> = RECORD_TYPES.iter().chain(NESTED_RECORD_TYPES).map(|s| s.to_string()).collect();
    ours.sort();
    assert_eq!(declared, ours);
}
