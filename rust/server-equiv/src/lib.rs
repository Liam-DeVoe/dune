//! The server equivalence harness, Rust side. See rust/docs/server-plan.md, "Equivalence testing".
//!
//! `treachery-online/ServerOracle` runs a scenario (JSON lines of hub calls, connections, clock
//! advances and restarts) through the C# `GameHub` and writes a transcript: per step, the
//! result, the callbacks each connection received, mails, timers, every table, every running
//! game and the hub's in-memory state. [`runner`] runs the same scenario through the Rust
//! `GameHub` and writes the same lines; [`compare`] checks them byte for byte, and [`report`]
//! summarizes progress per hub method.

pub mod compare;
pub mod decode;
pub mod json_text;
pub mod report;
pub mod runner;

use std::path::{Path, PathBuf};

/// The scenario files of a directory, sorted, without transcripts.
pub fn scenario_files(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|rd| rd.filter_map(|e| e.ok().map(|e| e.path())).collect())
        .unwrap_or_default();
    files.retain(|p| {
        let n = p.file_name().unwrap().to_string_lossy();
        n.ends_with(".jsonl") && !n.ends_with(".transcript.jsonl") && !n.ends_with(".rust.jsonl")
    });
    files.sort();
    files
}

/// `<dir>/<scenario name>.transcript.jsonl`.
pub fn transcript_for(scenario: &Path, dir: &Path) -> PathBuf {
    let name = scenario.file_name().unwrap().to_string_lossy();
    dir.join(format!("{}.transcript.jsonl", name.trim_end_matches(".jsonl")))
}
