//! `observe compare-refs A B`: records the same games with two reference binaries and compares
//! what players observe in each, without replaying anything through the working tree. Events are
//! not compared at all, so two recorders that write events in different formats still match
//! when the games play out the same: same calls, same acceptance, log, views and applicable types.

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::allow::Allow;
use crate::compare::{self, show, Diff};
use crate::live::{record_with_reference, Game, GameResult};
use crate::replay::{Mismatch, Outcome};
use crate::trace::{self, Line};
use crate::watch::Budgets;

/// How reports name the two sides.
pub const SIDES: [&str; 2] = ["A", "B"];

pub struct Options {
    pub a: PathBuf,
    pub b: PathBuf,
    pub mode: String,
    pub seed: u64,
    pub start: u64,
    pub games: u64,
    pub jobs: usize,
    pub allow: Arc<Allow>,
    pub budgets: Budgets,
    pub keep_failures: Option<PathBuf>,
    pub verbose: bool,
}

fn parse(trace: &[u8]) -> Result<Vec<Line>, String> {
    let text = std::str::from_utf8(trace).map_err(|e| e.to_string())?;
    text.lines().enumerate().filter(|(_, l)| !l.trim().is_empty()).map(|(i, l)| serde_json::from_str(l).map_err(|e| format!("line {}: {e}", i + 1))).collect()
}

/// Names the viewers of `n` views: the players, then the host, then an observer (session.rs).
fn labels(n: usize) -> Vec<String> {
    let mut v: Vec<String> = (0..n.saturating_sub(2)).map(|i| format!("player {i}")).collect();
    v.extend(["host".to_string(), "observer".to_string()]);
    v.truncate(n);
    v
}

fn obs(a: &trace::Obs, b: &trace::Obs, allow: &Allow) -> Result<(), Diff> {
    let n = a.views.as_ref().or(b.views.as_ref()).map_or(0, |v| v.len());
    compare::obs(a, b, allow, &labels(n))
}

/// Compares one line of each trace. The calls must be the same, apart from the event JSON.
fn compare_line(a: &Line, b: &Line, allow: &Allow) -> Result<(), Diff> {
    let calls = || Diff::new("calls", show(&a.without_outcome()), show(&b.without_outcome()));
    match (a, b) {
        (Line::Header(x), Line::Header(y)) => {
            for (h, side) in [(x, SIDES[0]), (y, SIDES[1])] {
                if h.format != trace::FORMAT {
                    return Err(Diff::new("header", format!("trace format {}", trace::FORMAT), format!("trace format {} from {side}", h.format)));
                }
            }
            let key = |h: &trace::Header| show(&(&h.mode, h.seed, h.index, h.version, &h.seated));
            if key(x) != key(y) {
                return Err(Diff::new("header", format!("mode, seed, game, version, seated: {}", key(x)), key(y)));
            }
            Ok(())
        }
        (Line::Query { p, host, result }, Line::Query { p: p2, host: host2, result: result2 }) => {
            if (p, host) != (p2, host2) {
                return Err(calls());
            }
            compare::query(result, result2)
        }
        (Line::Bots { order, end, result, .. }, Line::Bots { order: order2, end: end2, result: result2, .. }) => {
            if (order, end) != (order2, end2) {
                return Err(calls());
            }
            // Only which player's bot acted: the event is in the next line, and its encoding may differ.
            compare::bots(result, result2, &|_, _| Ok(serde_json::Value::Null), allow)
        }
        (Line::Event { src, p, host, outcome, .. }, Line::Event { src: src2, p: p2, host: host2, outcome: outcome2, .. }) => {
            if (src, p, host) != (src2, p2, host2) {
                return Err(calls());
            }
            compare::event_result(outcome, outcome2, allow)?;
            obs(&outcome.obs, &outcome2.obs, allow)
        }
        (Line::Undo { until, outcome }, Line::Undo { until: until2, outcome: outcome2 }) if until == until2 => special(outcome, outcome2, allow),
        (Line::Restore { outcome }, Line::Restore { outcome: outcome2 }) | (Line::Dismiss { outcome }, Line::Dismiss { outcome: outcome2 }) => special(outcome, outcome2, allow),
        (Line::End { reason, pending, .. }, Line::End { reason: reason2, pending: pending2, .. }) => {
            let pending_op = |p: &Option<Box<Line>>| p.as_ref().map(|l| l.op());
            if reason != reason2 || pending_op(pending) != pending_op(pending2) {
                return Err(Diff::new("end", show(a), show(b)));
            }
            // Both ran away at the same call. No real game may end like this, so the game fails,
            // as in a replay.
            if pending.is_some() {
                return Err(Diff::new("runaway", "the call returns".into(), format!("{reason} in both A and B")));
            }
            Ok(())
        }
        _ => Err(calls()),
    }
}

fn special(a: &trace::SpecialOutcome, b: &trace::SpecialOutcome, allow: &Allow) -> Result<(), Diff> {
    if a.panic != b.panic {
        return Err(Diff::new("panic", show(a), show(b)));
    }
    if a.error.is_some() != b.error.is_some() {
        return Err(Diff::new("result", show(&a.error), show(&b.error)));
    }
    obs(&a.obs, &b.obs, allow)
}

/// Compares two recordings of the same game.
pub fn compare_traces(a: &[u8], b: &[u8], allow: &Allow) -> Outcome {
    let a = match parse(a) {
        Ok(l) => l,
        Err(e) => return Outcome::Bad(format!("trace from A, {e}")),
    };
    let b = match parse(b) {
        Ok(l) => l,
        Err(e) => return Outcome::Bad(format!("trace from B, {e}")),
    };
    for (i, (x, y)) in a.iter().zip(b.iter()).enumerate() {
        if let Err(d) = compare_line(x, y, allow) {
            return Outcome::Fail(Mismatch::at(i + 1, x, d));
        }
        if let Line::End { reason, .. } = x {
            return Outcome::Pass { lines: i + 1, end: reason.clone() };
        }
    }
    if a.len() != b.len() {
        let n = a.len().min(b.len());
        let at = |l: &[Line]| l.get(n).map_or("the trace ends".to_string(), |l| show(&l.without_outcome()));
        let line = a.get(n).or(b.get(n)).expect("the longer trace has a line there");
        return Outcome::Fail(Mismatch::at(n + 1, line, Diff::new("calls", at(&a), at(&b))));
    }
    Outcome::Bad("no end line (the recording was cut short)".into())
}

/// Records one game with both binaries and compares them. A failure is recorded again with full
/// views, so the report can show which view fields differ. Returns both traces of a failure.
fn check_game(o: &Options, g: &Game) -> (GameResult, Option<(Vec<u8>, Vec<u8>)>) {
    let start = Instant::now();
    let done = |outcome, traces| (GameResult { game: g.clone(), outcome, trace: None, seconds: start.elapsed().as_secs_f64() }, traces);
    let record = |full| -> Result<(Vec<u8>, Vec<u8>), String> {
        let a = record_with_reference(&o.a, g, full, &o.budgets).map_err(|e| format!("A: {e}"))?;
        let b = record_with_reference(&o.b, g, full, &o.budgets).map_err(|e| format!("B: {e}"))?;
        Ok((a, b))
    };
    let (a, b) = match record(false) {
        Ok(t) => t,
        Err(e) => return done(Outcome::Bad(e), None),
    };
    let outcome = compare_traces(&a, &b, &o.allow);
    if outcome.passed() || matches!(outcome, Outcome::Bad(_)) {
        return done(outcome, None);
    }
    match record(true) {
        // With full views, view rules of the allow file can accept a view difference.
        // A long game can run out of memory with full views, so the detailed comparison is used
        // only when it finds the same difference, or accepts a view difference by a view rule.
        Ok((fa, fb)) => {
            let (Outcome::Fail(first), detailed) = (&outcome, compare_traces(&fa, &fb, &o.allow)) else { unreachable!("the first comparison failed") };
            match detailed {
                Outcome::Fail(m) if m.line == first.line && m.key.split(' ').next() == first.key.split(' ').next() =>done(Outcome::Fail(m), Some((fa, fb))),
                Outcome::Pass { .. } if first.key.starts_with("views") => done(detailed, None),
                _ => done(outcome, Some((a, b))),
            }
        }
        Err(_) => done(outcome, Some((a, b))),
    }
}

pub fn run(o: &Options) -> Vec<GameResult> {
    let next = AtomicUsize::new(0);
    let results = Mutex::new(Vec::new());
    std::thread::scope(|s| {
        for _ in 0..o.jobs.max(1) {
            s.spawn(|| loop {
                let k = next.fetch_add(1, Ordering::SeqCst) as u64;
                if k >= o.games {
                    break;
                }
                let g = Game { mode: o.mode.clone(), seed: o.seed, index: o.start + k };
                let (r, traces) = check_game(o, &g);
                if o.verbose || !r.outcome.passed() {
                    eprintln!("{}", crate::live::one_line(&r));
                }
                if let (Some(dir), Some((a, b))) = (&o.keep_failures, &traces) {
                    let _ = std::fs::create_dir_all(dir);
                    let _ = std::fs::write(dir.join(format!("{}-A.jsonl", g.name())), a);
                    let _ = std::fs::write(dir.join(format!("{}-B.jsonl", g.name())), b);
                }
                results.lock().unwrap().push(r);
            });
        }
    });
    let mut v = results.into_inner().unwrap();
    v.sort_by_key(|r| r.game.index);
    v
}
