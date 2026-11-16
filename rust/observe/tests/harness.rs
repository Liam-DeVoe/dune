//! The harness checks itself: a clean recording replays without a difference, the recorder is
//! deterministic, and a trace changed in any observable value is caught.

use std::sync::Arc;
use std::time::Duration;

use observe::allow::Allow;
use observe::record::{record_game, RecordOptions};
use observe::replay::{self, Outcome};
use observe::view_hash::ViewHash;
use observe::watch::Budgets;
use serde_json::Value;

fn budgets() -> Budgets {
    Budgets { hang: Duration::from_secs(60), memory: 2048 << 20 }
}

fn record(mode: &str, index: u64, full_views: bool) -> Vec<Value> {
    record_hashed(mode, index, full_views, ViewHash::LATEST)
}

fn record_hashed(mode: &str, index: u64, full_views: bool, view_hash: ViewHash) -> Vec<Value> {
    let mut out = Vec::new();
    let o = RecordOptions { mode: mode.into(), seed: 7, index, full_views, budgets: budgets(), reach: false, view_hash };
    record_game(&o, &mut out).unwrap();
    String::from_utf8(out).unwrap().lines().map(|l| serde_json::from_str(l).unwrap()).collect()
}

fn replay_lines(lines: &[Value], allow: Allow) -> Outcome {
    let text: String = lines.iter().map(|l| format!("{l}\n")).collect();
    replay::replay(Box::new(std::io::Cursor::new(text.into_bytes())), Arc::new(allow), &budgets())
}

fn failure_key(o: &Outcome) -> String {
    match o {
        Outcome::Fail(m) => m.key.clone(),
        other => panic!("expected a difference, got {other:?}"),
    }
}

/// The index of the first line that matches `pred`, after line `after`.
fn find(lines: &[Value], after: usize, pred: impl Fn(&Value) -> bool) -> usize {
    (after..lines.len()).find(|&i| pred(&lines[i])).expect("a line to tamper with")
}

#[test]
fn clean_replays_pass_and_recording_is_deterministic() {
    for (mode, index) in [("bots", 1), ("bots", 2), ("fuzz", 1), ("fuzz", 3)] {
        let a = record(mode, index, false);
        let b = record(mode, index, false);
        assert_eq!(a, b, "{mode} game {index} recorded twice differs");
        let o = replay_lines(&a, Allow::default());
        assert!(o.passed(), "{mode} game {index}: {o:?}");
    }
}

/// A trace says how its views are hashed, and the replay hashes them the same way. Traces from
/// before the header said so use SHA-256.
#[test]
fn views_are_hashed_as_the_trace_says() {
    let fast = record_hashed("bots", 1, false, ViewHash::Fold64);
    let mut sha = record_hashed("bots", 1, false, ViewHash::Sha256);
    let views = |t: &[Value]| t.iter().find(|l| l["views"].is_array()).unwrap()["views"].clone();
    assert_ne!(views(&fast), views(&sha));
    assert_eq!(fast[0]["viewHash"], "fold64");
    assert!(replay_lines(&fast, Allow::default()).passed());
    assert!(replay_lines(&sha, Allow::default()).passed());
    sha[0].as_object_mut().unwrap().remove("viewHash");
    assert!(replay_lines(&sha, Allow::default()).passed());
    // A trace read with the other hash fails at its first views.
    sha[0]["viewHash"] = Value::String("fold64".into());
    assert_eq!(failure_key(&replay_lines(&sha, Allow::default())), "views");
}

#[test]
fn tampered_traces_fail() {
    let lines = record("fuzz", 2, false);
    let is_op = |l: &Value, op: &str| l["op"] == op;

    // A log message.
    let mut t = lines.clone();
    let i = find(&t, 0, |l| l["log"].as_array().is_some_and(|a| !a.is_empty()));
    let original = t[i]["log"][0][1].as_str().unwrap().to_string();
    t[i]["log"][0][1] = Value::String("Something else happened".into());
    assert_eq!(failure_key(&replay_lines(&t, Allow::default())), "log");
    // ... which rules leaving out the old and the new text accept.
    let allow = Allow::parse(&format!("log Something else*\nlog {original}")).unwrap();
    assert!(replay_lines(&t, allow).passed());

    // One view.
    let mut t = lines.clone();
    let i = find(&t, 10, |l| l["views"].is_array());
    t[i]["views"][0] = Value::String("0123456789abcdef".into());
    assert_eq!(failure_key(&replay_lines(&t, Allow::default())), "views");

    // A validation message.
    let mut t = lines.clone();
    let i = find(&t, 0, |l| l["result"] == "rejected" && l["message"].is_string());
    t[i]["message"] = Value::String("You can't do that".into());
    assert_eq!(failure_key(&replay_lines(&t, Allow::default())), "message");
    assert!(!matches!(replay_lines(&t, Allow::parse("message You can't*").unwrap()), Outcome::Fail(ref m) if m.line == i + 1));

    // Validity.
    let mut t = lines.clone();
    let i = find(&t, 5, |l| is_op(l, "event") && l["result"] == "accepted");
    t[i]["result"] = Value::String("rejected".into());
    assert_eq!(failure_key(&replay_lines(&t, Allow::default())), "result");

    // A bot decision.
    let mut t = lines.clone();
    let i = find(&t, 0, |l| is_op(l, "bots") && l["choice"].is_object());
    t[i]["choice"]["event"]["Time"] = Value::String("2030-01-01T00:00:00+00:00".into());
    assert_eq!(failure_key(&replay_lines(&t, Allow::default())), "bot");
    assert!(!matches!(replay_lines(&t, Allow::parse("bots").unwrap()), Outcome::Fail(ref m) if m.line == i + 1));

    // The applicable events of a player.
    let mut t = lines.clone();
    let i = find(&t, 5, |l| is_op(l, "query") && l["types"].as_array().is_some_and(|a| !a.is_empty()));
    t[i]["types"].as_array_mut().unwrap().pop();
    assert_eq!(failure_key(&replay_lines(&t, Allow::default())), "applicable");

    // A cut-off trace.
    let t = lines[..lines.len() - 1].to_vec();
    assert!(matches!(replay_lines(&t, Allow::default()), Outcome::Bad(_)));
}

/// Writes an event differently without changing what it decodes to: the members after `$type`
/// in reverse order, and a member the reader doesn't know.
fn reencode(event: &Value) -> Value {
    let Value::Object(m) = event else { panic!("an event object: {event}") };
    let mut members: Vec<(String, Value)> = m.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
    members[1..].reverse();
    members.push(("NotAField".into(), Value::from(1)));
    Value::Object(members.into_iter().collect())
}

#[test]
fn events_are_compared_decoded_not_as_text() {
    for (mode, index) in [("bots", 1), ("fuzz", 2)] {
        let mut lines = record(mode, index, false);
        let mut changed = 0;
        for l in &mut lines {
            if l["op"] == "event" && l["result"] != "unreadable" {
                l["event"] = reencode(&l["event"]);
                changed += 1;
            }
            if l["op"] == "bots" && l["choice"].is_object() {
                let e = reencode(&l["choice"]["event"]);
                assert_ne!(e, l["choice"]["event"], "the JSON differs");
                l["choice"]["event"] = e;
                changed += 1;
            }
        }
        assert!(changed > 50, "{mode} game {index}: only {changed} events");
        let o = replay_lines(&lines, Allow::default());
        assert!(o.passed(), "{mode} game {index}: {o:?}");
    }
}

#[test]
fn bot_decisions_compare_through_the_working_trees_encoding() {
    let lines = record("bots", 1, false);
    let i = find(&lines, 0, |l| l["op"] == "bots" && l["choice"].is_object());
    let event = &lines[i]["choice"]["event"];
    // A member left out decodes to its default. If that's the value it had, the event is the same.
    let defaults: Vec<String> = event
        .as_object()
        .unwrap()
        .keys()
        .filter(|k| *k != "$type")
        .filter(|k| {
            let mut e = event.clone();
            e.as_object_mut().unwrap().shift_remove(k.as_str());
            observe::events::normalize_reference(&e).unwrap() == observe::events::normalize_reference(event).unwrap()
        })
        .cloned()
        .collect();
    assert!(!defaults.is_empty(), "no member of {event} has its default value");
    let mut t = lines.clone();
    for k in &defaults {
        t[i]["choice"]["event"].as_object_mut().unwrap().shift_remove(k.as_str());
    }
    assert!(replay_lines(&t, Allow::default()).passed());

    // A different value of a member that is read does fail.
    let mut t = lines.clone();
    t[i]["choice"]["event"]["Time"] = Value::String("2030-01-01T00:00:00+00:00".into());
    assert_eq!(failure_key(&replay_lines(&t, Allow::default())), "bot");

    // So does an event the working tree can't read.
    let mut t = lines.clone();
    t[i]["choice"]["event"]["$type"] = Value::String("NoSuchEvent".into());
    match replay_lines(&t, Allow::default()) {
        Outcome::Fail(m) => {
            assert_eq!(m.key, "bot");
            assert!(m.expected.contains("can't read"), "{}", m.expected);
        }
        other => panic!("expected a difference, got {other:?}"),
    }
}

#[test]
fn an_event_that_does_not_decode_counts_as_rejected() {
    let lines = record("fuzz", 2, false);
    let garbage = serde_json::json!({"$type": "NoSuchEvent", "Initiator": 1});

    // In place of a rejected event, the message isn't compared.
    let mut t = lines.clone();
    let i = find(&t, 5, |l| l["op"] == "event" && l["result"] == "rejected" && l["message"].is_string());
    t[i]["event"] = garbage.clone();
    t[i]["message"] = Value::String("A message the working tree never shows".into());
    let o = replay_lines(&t, Allow::default());
    assert!(o.passed(), "{o:?}");

    // In place of an accepted one, it's a difference.
    let mut t = lines.clone();
    let i = find(&t, 5, |l| l["op"] == "event" && l["result"] == "accepted");
    t[i]["event"] = garbage;
    match replay_lines(&t, Allow::default()) {
        Outcome::Fail(m) => assert_eq!((m.line, m.key.as_str()), (i + 1, "result")),
        other => panic!("expected a difference, got {other:?}"),
    }
}

#[test]
fn full_views_show_the_fields_that_differ() {
    let mut lines = record("bots", 3, true);
    let i = find(&lines, 20, |l| l["full"].is_array());
    lines[i]["full"][0]["turn"] = Value::from(99);
    lines[i]["views"][0] = Value::String("0123456789abcdef".into());
    match replay_lines(&lines, Allow::default()) {
        Outcome::Fail(m) => {
            assert_eq!(m.key, "views");
            assert!(m.view_diff.iter().any(|d| d.starts_with("/turn: 99 -> ")), "{:?}", m.view_diff);
        }
        other => panic!("expected a difference, got {other:?}"),
    }
    // A view rule for that path accepts it.
    assert!(replay_lines(&lines, Allow::parse("view /turn").unwrap()).passed());
}

#[test]
fn globs() {
    use observe::allow::glob;
    assert!(glob("a*c", "abbbc"));
    assert!(glob("*", ""));
    assert!(!glob("a*c", "abd"));
    assert!(glob("Game started*", "Game started!"));
    let a = Allow::parse("view /actions/*/validation").unwrap();
    assert!(a.ignores_view("/actions/3/validation/0"));
    assert!(!a.ignores_view("/actions/3/options"));
}
