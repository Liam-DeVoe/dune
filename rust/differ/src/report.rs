//! Running many traces in parallel and summarizing the results.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use crate::replay::{self, Failure, Options, Outcome};

/// The result for one trace.
pub struct TraceResult {
    pub path: PathBuf,
    pub outcome: Outcome,
}

/// Replays `files` on `jobs` threads. `on_done` is called for each finished
/// trace (from the worker threads), for progress output and cleanup.
pub fn run_all(files: &[PathBuf], jobs: usize, opts: &Options, on_done: &(dyn Fn(&TraceResult) + Sync)) -> Vec<TraceResult> {
    let next = AtomicUsize::new(0);
    let results: Mutex<Vec<Option<TraceResult>>> = Mutex::new((0..files.len()).map(|_| None).collect());
    std::thread::scope(|s| {
        for _ in 0..jobs.max(1) {
            s.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::Relaxed);
                if i >= files.len() {
                    break;
                }
                let (outcome, _) = replay::replay_file(&files[i], opts);
                let r = TraceResult { path: files[i].clone(), outcome };
                on_done(&r);
                results.lock().unwrap()[i] = Some(r);
            });
        }
    });
    results.into_inner().unwrap().into_iter().map(|r| r.unwrap()).collect()
}

fn name(p: &Path) -> String {
    p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

/// One line per trace: `PASS file (n lines)` or `FAIL file at line n: reason`.
pub fn one_line(r: &TraceResult) -> String {
    match &r.outcome.failure {
        None => match r.outcome.csharp_hung.as_deref() {
            Some("botHung") => format!("HUNG {} (a C# bot never returned; the {} lines before it match)", name(&r.path), r.outcome.lines_ok),
            Some(reason) => format!("HUNG {} ({reason}: C# and Rust both never return; the {} lines before it match)", name(&r.path), r.outcome.lines_ok),
            None => format!("PASS {} ({} lines)", name(&r.path), r.outcome.lines_ok),
        },
        Some(f) => format!("FAIL {} at line {}: {}", name(&r.path), r.outcome.at.as_deref().unwrap_or("header"), f.group()),
    }
}

/// The summary: pass count, then failures grouped by reason, largest group first.
pub fn summary(results: &[TraceResult]) -> String {
    let mut groups: BTreeMap<String, Vec<&TraceResult>> = BTreeMap::new();
    let mut passed = 0;
    for r in results {
        match &r.outcome.failure {
            None => passed += 1,
            Some(f) => groups.entry(f.group()).or_default().push(r),
        }
    }
    let lines: usize = results.iter().map(|r| r.outcome.lines_ok).sum();
    let mut out = String::new();
    let _ = writeln!(out, "{} traces: {} passed, {} failed ({} lines replayed without a difference)", results.len(), passed, results.len() - passed, lines);
    // Games C# never finished. They pass when Rust hangs on the same call too, but they are not complete games.
    let hung: Vec<&TraceResult> = results.iter().filter(|r| r.outcome.csharp_hung.is_some()).collect();
    if !hung.is_empty() {
        let both = hung.iter().filter(|r| r.outcome.passed()).count();
        let _ = writeln!(
            out,
            "{} never finished in C# (hung, or over the memory limit): {} of them match up to that point, {} differ",
            hung.len(),
            both,
            hung.len() - both
        );
        for r in hung.iter().take(10) {
            let _ = writeln!(out, "    {} ({}{})", name(&r.path), r.outcome.csharp_hung.as_deref().unwrap_or(""), if r.outcome.passed() { "" } else { ", differs" });
        }
        if hung.len() > 10 {
            let _ = writeln!(out, "    ... and {} more", hung.len() - 10);
        }
    }
    let mut groups: Vec<_> = groups.into_iter().collect();
    groups.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then(a.0.cmp(&b.0)));
    out.push_str(&bot_summary(results, &groups));
    for (reason, rs) in &groups {
        let _ = writeln!(out, "\n{} trace{}: {}", rs.len(), if rs.len() == 1 { "" } else { "s" }, reason);
        for r in rs.iter().take(3) {
            let _ = writeln!(out, "    {} at line {}", name(&r.path), r.outcome.at.as_deref().unwrap_or("header"));
        }
        if rs.len() > 3 {
            let _ = writeln!(out, "    ... and {} more", rs.len() - 3);
        }
        if let Some(Failure::Mismatch(m)) = &rs[0].outcome.failure {
            let _ = writeln!(out, "    expected: {}", m.expected);
            let _ = writeln!(out, "    actual:   {}", m.actual);
            for d in m.state_diff.iter().take(5) {
                let _ = writeln!(out, "    {}: C# {} / Rust {}", d.path, d.csharp, d.rust);
            }
            if m.no_state {
                let _ = writeln!(out, "    (no full dump on this line; regenerate the trace with --dump every to see which fields differ)");
            }
        }
    }
    out
}

/// With `--bots check`: how many bot deliberations matched, and the bot failures per bot method
/// (each trace stops at its first difference, so a trace counts once). Empty when no trace ran bots.
fn bot_summary(results: &[TraceResult], groups: &[(String, Vec<&TraceResult>)]) -> String {
    let matched: usize = results.iter().map(|r| r.outcome.bot_decisions_ok).sum();
    let bot_groups: Vec<_> = groups.iter().filter(|(_, rs)| rs[0].outcome.failure.as_ref().is_some_and(|f| f.is_bot())).collect();
    if matched == 0 && bot_groups.is_empty() {
        return String::new();
    }
    let stopped: usize = bot_groups.iter().map(|(_, rs)| rs.len()).sum();
    let mut out = String::new();
    let _ = writeln!(out, "\nbot deliberations: {matched} matched; {stopped} trace{} stopped at a bot difference", if stopped == 1 { "" } else { "s" });
    for (reason, rs) in &bot_groups {
        let _ = writeln!(out, "    {:>5}  {reason}", rs.len());
    }
    out
}

/// A full report of one failure, for `differ first`.
pub fn describe(outcome: &Outcome, detail: Option<&replay::Detail>) -> String {
    let mut out = String::new();
    let Some(f) = &outcome.failure else {
        let hung = match outcome.csharp_hung.as_deref() {
            Some("botHung") => "; a C# bot never returned, so the game stops there",
            Some(_) => "; C# never returned from the last call, and Rust didn't either",
            None => "",
        };
        let _ = writeln!(out, "no difference ({} lines replayed{hung})", outcome.lines_ok);
        return out;
    };
    let _ = writeln!(out, "first difference at line {}: {}", outcome.at.as_deref().unwrap_or("header"), f.group());
    if let Some(d) = detail {
        if !d.line_summary.is_empty() {
            let _ = writeln!(out, "\ntrace line:\n{}", d.line_summary);
        }
        if !d.all_mismatches.is_empty() {
            let _ = writeln!(out, "\nvalues that differ on this line:");
            for (k, e, a) in &d.all_mismatches {
                let _ = writeln!(out, "  {k}\n    C#:   {e}\n    Rust: {a}");
            }
        }
    }
    match f {
        Failure::Mismatch(m) => {
            if detail.map_or(true, |d| d.all_mismatches.is_empty()) {
                let _ = writeln!(out, "\n{}\n    C#:   {}\n    Rust: {}", m.key, m.expected, m.actual);
            }
            if !m.state_diff.is_empty() {
                let _ = writeln!(out, "\nstate differences (path: C# / Rust):");
                for d in &m.state_diff {
                    let _ = writeln!(out, "  {}\n    C#:   {}\n    Rust: {}", d.path, d.csharp, d.rust);
                }
            }
            if m.no_state {
                let _ = writeln!(out, "\nThis line has a hash but no full dump. Regenerate the trace with `--dump every` to see the fields.");
            }
        }
        Failure::NotPorted { message, location } => {
            let _ = writeln!(out, "\nRust panicked: {message}\n  at {location}");
        }
        Failure::Uncaught { during, exception, location } => {
            let _ = writeln!(out, "\nC# exception escaped during {during}: {exception}\n  at {location}");
        }
        Failure::MemoryLimit { limit_mb } => {
            let _ = writeln!(out, "\nThe Rust replay went over {limit_mb} MB and was stopped. Raise the limit with --memory-mb only after finding out what grows.");
        }
        Failure::BadTrace(m) => {
            let _ = writeln!(out, "\n{m}");
        }
    }
    out
}
