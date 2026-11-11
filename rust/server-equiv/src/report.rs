//! The progress report: one line per scenario and a table per hub method.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use crate::compare::{ScenarioResult, Verdict};
use crate::decode::METHODS;

#[derive(Default, Clone, Copy)]
pub struct MethodCounts {
    pub calls: usize,
    pub pass: usize,
    pub fail: usize,
    pub todo: usize,
    pub not_reached: usize,
}

/// Counts per hub method (and per pseudo-method such as `(advance)`) over all scenarios.
pub fn method_counts(results: &[ScenarioResult]) -> BTreeMap<String, MethodCounts> {
    let mut counts: BTreeMap<String, MethodCounts> = METHODS.iter().map(|m| (m.to_string(), MethodCounts::default())).collect();
    for r in results {
        for s in &r.steps {
            let c = counts.entry(s.method.clone()).or_default();
            c.calls += 1;
            match &s.verdict {
                None => c.not_reached += 1,
                Some(Verdict::Pass) => c.pass += 1,
                Some(Verdict::Todo(_)) => c.todo += 1,
                Some(_) => c.fail += 1,
            }
        }
    }
    counts
}

pub fn render(results: &[ScenarioResult], verbose: bool) -> String {
    let mut out = String::new();
    let passed = results.iter().filter(|r| r.passed()).count();
    let _ = writeln!(out, "Scenarios: {passed} of {} pass", results.len());
    for r in results {
        if let Some(e) = &r.error {
            let _ = writeln!(out, "  ERROR {}: {e}", r.name);
            continue;
        }
        let done = r.steps.iter().filter(|s| matches!(s.verdict, Some(Verdict::Pass))).count();
        match r.failure() {
            None => {
                let _ = writeln!(out, "  pass  {} ({} steps)", r.name, r.steps.len());
            }
            Some((i, s)) => {
                let v = s.verdict.as_ref().unwrap();
                let kind = match v {
                    Verdict::Todo(_) => "todo",
                    _ => "FAIL",
                };
                let detail = match v {
                    Verdict::Todo(m) => first_line(m),
                    other if verbose => other.summary(),
                    other => first_line(&other.summary()),
                };
                let _ = writeln!(out, "  {kind:<5} {} at step {i} {} ({done}/{} steps pass): {detail}", r.name, s.method, r.steps.len());
            }
        }
    }

    let _ = writeln!(out, "\nPer hub method (steps over all scenarios; a scenario stops at its first failing step):");
    let _ = writeln!(out, "  {:<30} {:>5} {:>5} {:>5} {:>5} {:>11}", "method", "calls", "pass", "fail", "todo", "not reached");
    for (m, c) in method_counts(results) {
        let mark = if c.calls == 0 {
            "  (no scenario)"
        } else if c.pass == c.calls {
            "  done"
        } else {
            ""
        };
        let _ = writeln!(out, "  {m:<30} {:>5} {:>5} {:>5} {:>5} {:>11}{mark}", c.calls, c.pass, c.fail, c.todo, c.not_reached);
    }
    out
}

fn first_line(s: &str) -> String {
    let l = s.lines().next().unwrap_or("");
    if l.chars().count() > 160 { format!("{}...", l.chars().take(160).collect::<String>()) } else { l.to_string() }
}
