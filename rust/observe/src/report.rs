//! Summaries and detailed reports.

use std::collections::BTreeMap;

use crate::live::GameResult;
use crate::replay::{Mismatch, Outcome};

/// How replay reports name the two sides.
pub const REPLAY_SIDES: [&str; 2] = ["reference", "working"];

/// A full description of one game's outcome, as a replay sees it.
pub fn describe(name: &str, o: &Outcome) -> String {
    describe_sides(name, o, REPLAY_SIDES)
}

/// The same, with the two sides named `sides`.
pub fn describe_sides(name: &str, o: &Outcome, sides: [&str; 2]) -> String {
    match o {
        Outcome::Pass { lines, end } => format!("{name}: no difference ({lines} lines, game ended with {end})\n"),
        Outcome::Bad(m) => format!("{name}: bad trace: {m}\n"),
        Outcome::Fail(m) => describe_mismatch(name, m, sides),
    }
}

fn describe_mismatch(name: &str, m: &Mismatch, sides: [&str; 2]) -> String {
    let mut s = format!("{name}: first difference at trace line {}, {}\n", m.line, m.context);
    s.push_str(&format!("  what:      {}\n", m.key));
    if let Some((i, label)) = &m.viewer {
        s.push_str(&format!("  viewer:    {label} (view {i})\n"));
    }
    let width = sides[0].len().max(sides[1].len()).max(9);
    s.push_str(&format!("  {:<w$} {}\n  {:<w$} {}\n", format!("{}:", sides[0]), m.expected, format!("{}:", sides[1]), m.actual, w = width + 1));
    if !m.view_diff.is_empty() {
        s.push_str(&format!("  view fields that differ (path: {} -> {}):\n", sides[0], sides[1]));
        for d in &m.view_diff {
            s.push_str(&format!("    {d}\n"));
        }
    }
    s
}

pub fn summary(results: &[GameResult], seconds: f64, sides: [&str; 2]) -> String {
    let passed = results.iter().filter(|r| r.outcome.passed()).count();
    let mut s = format!("{} games: {} passed, {} failed ({:.0} s)\n", results.len(), passed, results.len() - passed, seconds);
    let mut ends: BTreeMap<String, usize> = BTreeMap::new();
    for r in results {
        if let Outcome::Pass { end, .. } = &r.outcome {
            *ends.entry(end.clone()).or_default() += 1;
        }
    }
    if !ends.is_empty() {
        s.push_str(&format!("passed games ended with: {}\n", ends.iter().map(|(k, v)| format!("{k} {v}")).collect::<Vec<_>>().join(", ")));
    }
    let mut groups: BTreeMap<String, Vec<&GameResult>> = BTreeMap::new();
    for r in results.iter().filter(|r| !r.outcome.passed()) {
        groups.entry(r.outcome.group()).or_default().push(r);
    }
    let mut groups: Vec<_> = groups.into_iter().collect();
    groups.sort_by_key(|(_, v)| std::cmp::Reverse(v.len()));
    for (g, v) in groups {
        s.push_str(&format!("\n{} games: {g}\n", v.len()));
        for r in v.iter().take(3) {
            s.push_str(&format!("    {}\n", r.game.name()));
        }
    }
    if let Some(r) = results.iter().find(|r| !r.outcome.passed()) {
        s.push('\n');
        s.push_str(&describe_sides(&r.game.name(), &r.outcome, sides));
    }
    s
}
