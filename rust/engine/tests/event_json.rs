//! Event JSON compatibility with System.Text.Json (hazards.md section 12).
//!
//! `tests/golden/events.jsonl` comes from `csharp-ref -- --events` (see
//! EventCorpus.cs): every event type with randomized members, serialized by
//! `Utilities.Serialize<GameEvent>`, plus malformed variants. For each line,
//! Rust must accept exactly the inputs .NET accepts and re-serialize them to
//! the same bytes.
//!
//! `tests/golden/event_errors.jsonl` comes from the same run: events whose
//! `$type` is no longer a known event (old savegames after an event class was
//! renamed). Rust must throw the exception .NET throws, message included.

use treachery_engine::events::GameEvent;
use treachery_engine::game::GameState;
use treachery_engine::json;

#[derive(serde::Deserialize)]
struct Case {
    kind: String,
    json: String,
    ok: bool,
    out: Option<String>,
}

#[test]
fn events_round_trip_like_dotnet() {
    let text = include_str!("golden/events.jsonl");
    let mut failures = Vec::new();
    let mut count = 0;
    for (i, line) in text.lines().enumerate() {
        let case: Case = serde_json::from_str(line).unwrap();
        count += 1;
        let result = match case.kind.as_str() {
            "event" => json::deserialize::<GameEvent>(&case.json).map(|e| json::serialize(&e)),
            "state" => json::deserialize::<GameState>(&case.json).map(|s| json::serialize(&s)),
            k => panic!("unknown kind {k}"),
        };
        match (case.ok, result) {
            (true, Ok(out)) => {
                let expected = case.out.as_deref().unwrap();
                if out != expected {
                    failures.push(format!("line {}: output differs\n  in:  {}\n  C#:  {}\n  Rust:{}", i + 1, case.json, expected, out));
                }
            }
            (true, Err(e)) => failures.push(format!("line {}: Rust rejects ({:?}) what C# accepts: {}", i + 1, e, case.json)),
            (false, Ok(out)) => failures.push(format!("line {}: Rust accepts what C# rejects: {}\n  Rust: {}", i + 1, case.json, out)),
            (false, Err(_)) => {}
        }
    }
    assert!(count > 1000, "corpus too small");
    if !failures.is_empty() {
        let n = failures.len();
        failures.truncate(15);
        panic!("{n} of {count} cases differ:\n{}", failures.join("\n"));
    }
}

#[derive(serde::Deserialize)]
struct ErrorCase {
    kind: String,
    json: String,
    error: String,
}

#[test]
fn unknown_event_types_fail_like_dotnet() {
    use treachery_engine::json::utilities::Utilities;
    let text = include_str!("golden/event_errors.jsonl");
    let mut count = 0;
    for line in text.lines() {
        let case: ErrorCase = serde_json::from_str(line).unwrap();
        count += 1;
        // The paths the C# server and Rust engine read savegames and events through.
        let result = match case.kind.as_str() {
            "state" => GameState::load(&case.json).map(drop),
            "event" => Utilities::deserialize::<GameEvent>(&case.json).map(drop),
            k => panic!("unknown kind {k}"),
        };
        let error = result.err().map(|e| format!("{}: {}", e.kind, e.message)).unwrap_or_default();
        assert_eq!(error, case.error, "input: {}", case.json);
        // json::deserialize reports the same error.
        let result = match case.kind.as_str() {
            "state" => json::deserialize::<GameState>(&case.json).map(|_| ()),
            _ => json::deserialize::<GameEvent>(&case.json).map(|_| ()),
        };
        let e = result.unwrap_err();
        assert_eq!(format!("{}: {}", e.kind, e.message), case.error);
    }
    assert!(count >= 10);
}
