//! `server-equiv run --scenarios DIR [--golden DIR] [--out DIR] [-v]`
//!
//! Runs every scenario in DIR through the Rust hub and compares with `<golden>/<name>.transcript.jsonl`
//! (by default next to the scenario, as `ServerOracle generate` writes them). `--out` keeps the Rust
//! transcripts for inspection. Exits with 1 when a scenario does not pass.

use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) != Some("run") {
        eprintln!("usage: server-equiv run --scenarios DIR [--golden DIR] [--out DIR] [-v]");
        std::process::exit(2);
    }
    let opt = |name: &str| args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).map(PathBuf::from);
    let verbose = args.iter().any(|a| a == "-v");
    let Some(scenarios) = opt("--scenarios") else {
        eprintln!("--scenarios is required");
        std::process::exit(2);
    };
    let golden = opt("--golden").unwrap_or_else(|| scenarios.clone());
    let out = opt("--out");

    let results: Vec<_> = server_equiv::scenario_files(&scenarios)
        .iter()
        .map(|s| server_equiv::compare::run_files(s, &server_equiv::transcript_for(s, &golden), out.as_deref()))
        .collect();
    print!("{}", server_equiv::report::render(&results, verbose));
    std::process::exit(if results.iter().all(|r| r.passed()) { 0 } else { 1 });
}
