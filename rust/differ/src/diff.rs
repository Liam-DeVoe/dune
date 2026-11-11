//! A structural diff of two state dumps, for hash mismatches.

use serde_json::Value;

/// One difference: where, and the two values (shortened).
#[derive(Clone, Debug)]
pub struct Difference {
    /// For example `Game.Players[1].Resources`.
    pub path: String,
    pub csharp: String,
    pub rust: String,
}

/// The first `max` differences between the C# dump `e` and the Rust dump `a`,
/// in document order.
pub fn diff(e: &Value, a: &Value, max: usize) -> Vec<Difference> {
    let mut out = Vec::new();
    walk("Game", e, a, max, &mut out);
    out
}

fn walk(path: &str, e: &Value, a: &Value, max: usize, out: &mut Vec<Difference>) {
    if out.len() >= max || e == a {
        return;
    }
    match (e, a) {
        (Value::Object(eo), Value::Object(ao)) if eo.get("$type") == ao.get("$type") && eo.get("$ref") == ao.get("$ref") => {
            for (k, ev) in eo {
                match ao.get(k) {
                    Some(av) => walk(&format!("{path}.{k}"), ev, av, max, out),
                    None => push(out, format!("{path}.{k}"), short(ev), "<field missing in Rust>".into()),
                }
                if out.len() >= max {
                    return;
                }
            }
            for (k, av) in ao {
                if !eo.contains_key(k) {
                    push(out, format!("{path}.{k}"), "<field missing in C#>".into(), short(av));
                }
            }
        }
        (Value::Array(ea), Value::Array(aa)) => {
            if ea.len() != aa.len() {
                push(out, format!("{path}.length"), ea.len().to_string(), aa.len().to_string());
            }
            for (i, (ev, av)) in ea.iter().zip(aa.iter()).enumerate() {
                walk(&format!("{path}[{i}]"), ev, av, max, out);
                if out.len() >= max {
                    return;
                }
            }
        }
        _ => push(out, path.to_string(), short(e), short(a)),
    }
}

fn push(out: &mut Vec<Difference>, path: String, csharp: String, rust: String) {
    out.push(Difference { path, csharp, rust });
}

/// Compact JSON, cut at 300 characters.
pub fn short(v: &Value) -> String {
    let s = serde_json::to_string(v).unwrap_or_default();
    if s.chars().count() > 300 { format!("{}...", s.chars().take(300).collect::<String>()) } else { s }
}

/// The path with list indices removed (`Game.Players[].Resources`), to group
/// the same difference across traces.
pub fn generic_path(path: &str) -> String {
    let mut out = String::new();
    let mut in_index = false;
    for c in path.chars() {
        match c {
            '[' => {
                in_index = true;
                out.push('[');
            }
            ']' => {
                in_index = false;
                out.push(']');
            }
            _ if in_index => {}
            _ => out.push(c),
        }
    }
    out
}
