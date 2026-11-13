//! Paths where two JSON values differ, for reports of a view that differs.

use serde_json::Value;

fn short(v: Option<&Value>) -> String {
    match v {
        None => "(absent)".into(),
        Some(v) => {
            let s = v.to_string();
            if s.chars().count() > 160 { format!("{}...", s.chars().take(160).collect::<String>()) } else { s }
        }
    }
}

/// Up to `max` lines `path: expected -> actual`. Paths are like `/board/players/2/faction`.
/// `ignored(path)` drops a path and everything below it.
pub fn diff(expected: &Value, actual: &Value, ignored: &dyn Fn(&str) -> bool, max: usize) -> Vec<String> {
    let mut out = Vec::new();
    walk("", Some(expected), Some(actual), ignored, max, &mut out);
    out
}

fn walk(path: &str, e: Option<&Value>, a: Option<&Value>, ignored: &dyn Fn(&str) -> bool, max: usize, out: &mut Vec<String>) {
    if out.len() >= max || e == a || (!path.is_empty() && ignored(path)) {
        return;
    }
    match (e, a) {
        (Some(Value::Object(eo)), Some(Value::Object(ao))) => {
            let mut keys: Vec<&String> = eo.keys().collect();
            keys.extend(ao.keys().filter(|k| !eo.contains_key(*k)));
            for k in keys {
                walk(&format!("{path}/{k}"), eo.get(k), ao.get(k), ignored, max, out);
            }
        }
        (Some(Value::Array(ea)), Some(Value::Array(aa))) => {
            for i in 0..ea.len().max(aa.len()) {
                walk(&format!("{path}/{i}"), ea.get(i), aa.get(i), ignored, max, out);
            }
        }
        _ => out.push(format!("{}: {} -> {}", if path.is_empty() { "/" } else { path }, short(e), short(a))),
    }
}
