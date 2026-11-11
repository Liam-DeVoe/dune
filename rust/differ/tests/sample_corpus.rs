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

#[test]
fn sample_corpus_passes() {
    let results = run();
    let summary = report::summary(&results);
    assert!(results.iter().all(|r| r.outcome.passed()), "{summary}");
}
