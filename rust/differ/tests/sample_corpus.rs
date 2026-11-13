//! Replays every trace in `rust/traces/sample` through the Rust engine.
//!
//! `sample_corpus_report` always passes: it prints the summary (failures
//! grouped by reason) so the state of the port is visible in test output
//! (`cargo test -p differ --test sample_corpus -- --nocapture`). It fails only
//! when a trace cannot be read, which is a harness problem.
//!
//! `sample_corpus_passes` requires every trace to replay without a difference.

use std::path::PathBuf;

use differ::replay::{Failure, Options};
use differ::report;

fn run() -> Vec<report::TraceResult> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../traces/sample");
    let files = differ::trace::list(&dir);
    assert!(!files.is_empty(), "no traces in {}", dir.display());
    let jobs = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
    report::run_all(&files, jobs, &Options::default(), &|_| {})
}

#[test]
fn sample_corpus_report() {
    let results = run();
    println!("{}", report::summary(&results));
    let bad: Vec<_> = results
        .iter()
        .filter(|r| matches!(r.outcome.failure, Some(Failure::BadTrace(_))))
        .map(report::one_line)
        .collect();
    assert!(bad.is_empty(), "unreadable traces:\n{}", bad.join("\n"));
}

/// Traces that differ from C# on purpose, with the line (`n`) of their first difference. Each is a
/// C# bug fixed in Rust (rust/docs/escapes.md).
const DELIBERATE: &[(&str, &str, &str)] = &[
    // A Bid ran automatic passes after bidding ended, which broke a later event that C#'s undo
    // replays, so C# stopped replaying one event earlier ("Automatic bid passes that never stop").
    ("fuzz-2026-00009.jsonl.gz", "600", "automatic bid passes stop with the bidding phase"),
    // CardTraded must name the ally ("A card trade offered to a faction that isn't the ally").
    ("fuzz-2026-00045.jsonl.gz", "855", "card trade offers must name the ally"),
];

#[test]
fn sample_corpus_passes() {
    let results = run();
    let summary = report::summary(&results);
    let deliberate = |r: &report::TraceResult| {
        let name = r.path.file_name().and_then(|n| n.to_str()).unwrap_or_default();
        DELIBERATE.iter().any(|(file, n, _)| *file == name && r.outcome.at.as_deref() == Some(*n))
    };
    assert!(results.iter().all(|r| r.outcome.passed() || deliberate(r)), "{summary}");
}
