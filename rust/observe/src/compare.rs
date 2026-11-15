//! What counts as the same observation. The replay (`replay.rs`) compares a trace with what the
//! working tree observes, and `compare-refs` (`compare_refs.rs`) compares two traces. Both use
//! these rules.

use crate::allow::Allow;
use crate::trace::{BotsResult, EventOutcome, Obs, QueryResult};

/// One difference, without the trace position (the caller adds it).
#[derive(Clone, Debug, Default)]
pub struct Diff {
    pub key: String,
    pub expected: String,
    pub actual: String,
    pub viewer: Option<(usize, String)>,
    pub view_diff: Vec<String>,
}

impl Diff {
    pub fn new(key: &str, expected: String, actual: String) -> Diff {
        Diff { key: key.into(), expected, actual, ..Default::default() }
    }
}

pub fn show<T: serde::Serialize>(v: &T) -> String {
    serde_json::to_string(v).unwrap_or_default()
}

/// The applicable types of a query. An exception's text is not compared (native code reports
/// these differently).
pub fn query(expected: &QueryResult, actual: &QueryResult) -> Result<(), Diff> {
    if expected.panic != actual.panic {
        return Err(Diff::new("panic", show(expected), show(actual)));
    }
    if actual.types != expected.types || actual.error.is_some() != expected.error.is_some() {
        return Err(Diff::new("applicable", show(expected), show(actual)));
    }
    Ok(())
}

/// A bot round: which player's bot acted and with which event, or that a bot threw. The level a
/// bot decided at is internal. `event` turns each side's event into the form compared, or None
/// to compare only who acted.
pub fn bots(expected: &BotsResult, actual: &BotsResult, event: &dyn Fn(&serde_json::Value, bool) -> Result<serde_json::Value, String>, allow: &Allow) -> Result<(), Diff> {
    if expected.panic != actual.panic {
        return Err(Diff::new("panic", show(expected), show(actual)));
    }
    if allow.bots {
        return Ok(());
    }
    let key = |r: &BotsResult, reference: bool| (r.choice.as_ref().map(|c| (c.p, event(&c.event, reference))), r.error.is_some());
    if key(expected, true) != key(actual, false) {
        let side = |r: &BotsResult, reference: bool| match r.choice.as_ref().map(|c| event(&c.event, reference)) {
            Some(Err(e)) => format!("{} (the working tree can't read its event: {e})", show(r)),
            _ => show(r),
        };
        return Err(Diff::new("bot", side(expected, true), side(actual, false)));
    }
    Ok(())
}

/// Accepted or not, then the rejection message. An event the engine threw on (`error`) or that
/// doesn't decode (`unreadable`) counts as rejected, and its message is not compared: native code
/// reports these failures differently, and that is not a change players rely on.
pub fn event_result(expected: &EventOutcome, actual: &EventOutcome, allow: &Allow) -> Result<(), Diff> {
    fn class(r: &str) -> &str {
        match r {
            "rejected" | "error" | "unreadable" => "not accepted",
            other => other,
        }
    }
    let text = |o: &EventOutcome| format!("{} {}", o.result, o.message.clone().unwrap_or_default());
    if (expected.result == "panic") != (actual.result == "panic") {
        return Err(Diff::new("panic", text(expected), text(actual)));
    }
    if class(&actual.result) != class(&expected.result) {
        return Err(Diff::new("result", text(expected), text(actual)));
    }
    if actual.result == "rejected" && expected.result == "rejected" && !allow.same_message(expected.message.as_deref(), actual.message.as_deref()) {
        return Err(Diff::new("message", expected.message.clone().unwrap_or_default(), actual.message.clone().unwrap_or_default()));
    }
    Ok(())
}

/// The log and the views after a call. `labels` names each viewer for the report.
pub fn obs(expected: &Obs, actual: &Obs, allow: &Allow, labels: &[String]) -> Result<(), Diff> {
    if expected.log_cut != actual.log_cut {
        return Err(Diff::new("log", format!("log cut to {:?}", expected.log_cut), format!("log cut to {:?}", actual.log_cut)));
    }
    let e: Vec<&(String, String)> = expected.log.iter().filter(|m| !allow.ignores_log(&m.1)).collect();
    let a: Vec<&(String, String)> = actual.log.iter().filter(|m| !allow.ignores_log(&m.1)).collect();
    if e != a {
        let i = e.iter().zip(a.iter()).position(|(x, y)| x != y).unwrap_or(e.len().min(a.len()));
        let at = |v: &Vec<&(String, String)>| v.get(i).map_or("(no message)".to_string(), |(t, m)| if t.is_empty() { m.clone() } else { format!("[to {t}] {m}") });
        return Err(Diff::new("log", format!("message {}: {}", i + 1, at(&e)), format!("message {}: {}", i + 1, at(&a))));
    }
    if expected.replica.is_some() != actual.replica.is_some() {
        return Err(Diff::new("replica", show(&expected.replica), show(&actual.replica)));
    }
    let built = |v: &Option<Vec<String>>| if v.is_some() { "views built".to_string() } else { "no views (history unchanged)".to_string() };
    let (ev, av) = match (&expected.views, &actual.views) {
        (None, None) => return Ok(()),
        (Some(ev), Some(av)) => (ev, av),
        _ => return Err(Diff::new("views", built(&expected.views), built(&actual.views))),
    };
    if ev.len() != av.len() {
        return Err(Diff::new("views", format!("{} viewers", ev.len()), format!("{} viewers ({})", av.len(), labels.join(", "))));
    }
    for i in 0..ev.len() {
        // A view that failed to build is compared as failed, not by its error text.
        if ev[i] == av[i] || (ev[i].starts_with("error: ") && av[i].starts_with("error: ")) {
            continue;
        }
        let mut d = Diff::new("views", ev[i].clone(), av[i].clone());
        d.viewer = Some((i, labels.get(i).cloned().unwrap_or_default()));
        if let (Some(ef), Some(af)) = (&expected.full, &actual.full) {
            d.view_diff = crate::jsondiff::diff(&ef[i], &af[i], &|path| allow.ignores_view(path), 30);
            if d.view_diff.is_empty() {
                // Every difference is allowed.
                continue;
            }
        } else if allow.has_view_rules() {
            d.key = "views (hash)".into();
        }
        return Err(d);
    }
    Ok(())
}
