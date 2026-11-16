//! `compare-refs` compares two recordings of a game by what players observe. It ignores how events
//! are written and catches any other change.

use std::time::Duration;

use observe::allow::Allow;
use observe::compare_refs::compare_traces;
use observe::record::{record_game, RecordOptions};
use observe::replay::Outcome;
use observe::view_hash::ViewHash;
use observe::watch::Budgets;
use serde_json::Value;

fn record(mode: &str, index: u64) -> Vec<Value> {
    let mut out = Vec::new();
    let budgets = Budgets { hang: Duration::from_secs(60), memory: 2048 << 20 };
    let o = RecordOptions { mode: mode.into(), seed: 7, index, full_views: false, budgets, full_views_lines: None, reach: false, view_hash: ViewHash::LATEST };
    record_game(&o, &mut out).unwrap();
    String::from_utf8(out).unwrap().lines().map(|l| serde_json::from_str(l).unwrap()).collect()
}

fn bytes(lines: &[Value]) -> Vec<u8> {
    lines.iter().map(|l| format!("{l}\n")).collect::<String>().into_bytes()
}

fn compare(a: &[Value], b: &[Value]) -> Outcome {
    compare_traces(&bytes(a), &bytes(b), &Allow::default())
}

fn failure(o: &Outcome) -> (usize, String) {
    match o {
        Outcome::Fail(m) => (m.line, m.key.clone()),
        other => panic!("expected a difference, got {other:?}"),
    }
}

fn find(lines: &[Value], after: usize, pred: impl Fn(&Value) -> bool) -> usize {
    (after..lines.len()).find(|&i| pred(&lines[i])).expect("a line to tamper with")
}

/// Every event in another format, as a recorder after the format change would write it.
fn other_format(lines: &[Value]) -> Vec<Value> {
    let convert = |e: &Value| serde_json::json!({"type": e["$type"], "play": {"fields": e.as_object().unwrap().len()}});
    let mut t = lines.to_vec();
    for l in &mut t {
        if l["op"] == "event" {
            l["event"] = convert(&l["event"]);
        }
        if l["op"] == "bots" && l["choice"].is_object() {
            l["choice"]["event"] = convert(&l["choice"]["event"]);
        }
    }
    t
}

#[test]
fn the_same_games_in_another_event_format_match() {
    for (mode, index) in [("bots", 1), ("fuzz", 2)] {
        let a = record(mode, index);
        assert!(compare(&a, &a).passed(), "{mode} game {index}");
        let b = other_format(&a);
        assert_ne!(a, b);
        let o = compare(&a, &b);
        assert!(o.passed(), "{mode} game {index}: {o:?}");
    }
}

#[test]
fn any_observed_difference_fails() {
    let a = record("fuzz", 2);
    let b = other_format(&a);
    let is_op = |l: &Value, op: &str| l["op"] == op;

    let mut t = b.clone();
    t[0]["version"] = Value::from(1);
    assert_eq!(failure(&compare(&a, &t)), (1, "header".into()));

    let mut t = b.clone();
    let i = find(&t, 0, |l| l["log"].as_array().is_some_and(|a| !a.is_empty()));
    t[i]["log"][0][1] = Value::String("Something else happened".into());
    assert_eq!(failure(&compare(&a, &t)), (i + 1, "log".into()));

    let mut t = b.clone();
    let i = find(&t, 10, |l| l["views"].is_array());
    t[i]["views"][1] = Value::String("0123456789abcdef".into());
    assert_eq!(failure(&compare(&a, &t)), (i + 1, "views".into()));

    let mut t = b.clone();
    let i = find(&t, 5, |l| is_op(l, "event") && l["result"] == "accepted");
    t[i]["result"] = Value::String("rejected".into());
    assert_eq!(failure(&compare(&a, &t)), (i + 1, "result".into()));

    let mut t = b.clone();
    let i = find(&t, 0, |l| l["result"] == "rejected" && l["message"].is_string());
    t[i]["message"] = Value::String("You can't do that".into());
    assert_eq!(failure(&compare(&a, &t)), (i + 1, "message".into()));

    let mut t = b.clone();
    let i = find(&t, 5, |l| is_op(l, "query") && l["types"].as_array().is_some_and(|a| !a.is_empty()));
    t[i]["types"].as_array_mut().unwrap().pop();
    assert_eq!(failure(&compare(&a, &t)), (i + 1, "applicable".into()));

    // Which player's bot acted.
    let mut t = b.clone();
    let i = find(&t, 0, |l| is_op(l, "bots") && l["choice"].is_object());
    t[i]["choice"]["p"] = Value::from(t[i]["choice"]["p"].as_u64().unwrap() + 1);
    assert_eq!(failure(&compare(&a, &t)), (i + 1, "bot".into()));

    // A different call, and a call missing.
    let mut t = b.clone();
    let i = find(&t, 5, |l| is_op(l, "event") && l["result"] == "accepted");
    t[i]["host"] = Value::Bool(!t[i]["host"].as_bool().unwrap());
    assert_eq!(failure(&compare(&a, &t)), (i + 1, "calls".into()));
    let mut t = b.clone();
    t.remove(i);
    assert_eq!(failure(&compare(&a, &t)).0, i + 1);

    // The way the game ended.
    let mut t = b.clone();
    let n = t.len();
    t[n - 1]["reason"] = Value::String(if t[n - 1]["reason"] == "stalled" { "cap" } else { "stalled" }.into());
    assert_eq!(failure(&compare(&a, &t)), (n, "end".into()));
}

#[test]
fn a_game_that_runs_away_in_both_fails() {
    let mut a = record("bots", 2);
    let n = a.len();
    let pending = serde_json::json!({"op": "restore"});
    a[n - 1] = serde_json::json!({"op": "end", "reason": "hung", "pending": pending});
    assert_eq!(failure(&compare(&a, &a)), (n, "runaway".into()));
}

#[test]
fn a_cut_off_trace_is_bad() {
    let a = record("bots", 2);
    let t = a[..a.len() - 1].to_vec();
    assert!(matches!(compare(&t, &t), Outcome::Bad(_)));
}
