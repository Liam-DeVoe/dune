//! The hand-written server scenarios against their golden C# transcripts.
//!
//! - Every scenario in `scenarios/` must have a transcript in `golden/` written for its current
//!   text (`run-oracle.sh` rewrites them; it needs .NET).
//! - Every hub method must be called by at least one scenario.
//! - Scenarios listed in `expected_pass.txt` must pass. The others are reported but don't fail
//!   the test: the hub is being ported (packages H1-H4), and these scenarios define when each
//!   package is done. Run with `--nocapture` to see the progress report.

use std::collections::BTreeSet;
use std::path::PathBuf;

use server_equiv::compare::{check_transcript_header, run_files};
use server_equiv::decode::METHODS;

fn dir(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(name)
}

fn expected_pass() -> BTreeSet<String> {
    std::fs::read_to_string(dir("expected_pass.txt"))
        .unwrap()
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(String::from)
        .collect()
}

#[test]
fn goldens_are_current() {
    let scenarios = server_equiv::scenario_files(&dir("scenarios"));
    assert!(!scenarios.is_empty());
    let mut problems = Vec::new();
    for s in &scenarios {
        let golden = server_equiv::transcript_for(s, &dir("golden"));
        let text = std::fs::read_to_string(s).unwrap();
        match std::fs::read_to_string(&golden) {
            Err(_) => problems.push(format!("{}: no golden transcript", s.display())),
            Ok(g) => {
                if let Err(e) = check_transcript_header(&text, g.lines().next().unwrap_or("")) {
                    problems.push(format!("{}: {e}", s.display()));
                }
                let steps = text.lines().filter(|l| !l.trim().is_empty()).count() - 1;
                let lines = g.lines().filter(|l| !l.trim().is_empty()).count() - 1;
                if steps != lines {
                    problems.push(format!("{}: {steps} steps, {lines} transcript lines", s.display()));
                }
            }
        }
    }
    let names: BTreeSet<String> = scenarios.iter().map(|s| s.file_stem().unwrap().to_string_lossy().into_owned()).collect();
    for name in expected_pass() {
        if !names.contains(&name) {
            problems.push(format!("expected_pass.txt lists {name}, which is not a scenario"));
        }
    }
    assert!(problems.is_empty(), "run rust/server-equiv/run-oracle.sh:\n{}", problems.join("\n"));
}

#[test]
fn every_hub_method_has_a_scenario() {
    let mut called = BTreeSet::new();
    for s in server_equiv::scenario_files(&dir("scenarios")) {
        for line in std::fs::read_to_string(&s).unwrap().lines().skip(1).filter(|l| !l.trim().is_empty()) {
            let v: serde_json::Value = serde_json::from_str(line).unwrap_or_else(|e| panic!("{}: {e}: {line}", s.display()));
            if let Some(m) = v.get("method").and_then(|m| m.as_str()) {
                assert!(METHODS.contains(&m), "{}: unknown method {m}", s.display());
                called.insert(m.to_string());
            }
        }
    }
    let missing: Vec<_> = METHODS.iter().filter(|m| !called.contains(**m)).collect();
    assert!(missing.is_empty(), "hub methods without a scenario: {missing:?}");
}

/// Replaces every argument that is only a `${...}` reference (resolved from earlier results at run
/// time) with a stand-in: a number for `...UserId`, a string otherwise.
fn placeholders(v: &serde_json::Value) -> serde_json::Value {
    use serde_json::Value;
    match v {
        Value::String(s) if s.starts_with("${") && s.ends_with('}') && s.matches("${").count() == 1 => {
            if s.ends_with("UserId}") { serde_json::json!(1) } else { serde_json::json!("x") }
        }
        Value::Array(a) => Value::Array(a.iter().map(placeholders).collect()),
        Value::Object(o) => Value::Object(o.iter().map(|(k, x)| (k.clone(), placeholders(x))).collect()),
        other => other.clone(),
    }
}

/// Every call of every scenario decodes into a `HubCall`.
#[test]
fn every_call_decodes() {
    for s in server_equiv::scenario_files(&dir("scenarios")) {
        for line in std::fs::read_to_string(&s).unwrap().lines().skip(1).filter(|l| !l.trim().is_empty()) {
            let v: serde_json::Value = serde_json::from_str(line).unwrap();
            if let Some(m) = v.get("method").and_then(|m| m.as_str()) {
                let args = placeholders(&v.get("args").cloned().unwrap_or(serde_json::Value::Array(Vec::new())));
                if let Err(e) = server_equiv::decode::decode(m, args.as_array().map(Vec::as_slice).unwrap_or(&[])) {
                    panic!("{}: {e}\n{line}", s.display());
                }
            }
        }
    }
}

#[test]
fn scenarios_match_csharp() {
    let results: Vec<_> = server_equiv::scenario_files(&dir("scenarios"))
        .iter()
        .map(|s| run_files(s, &server_equiv::transcript_for(s, &dir("golden")), None))
        .collect();
    println!("{}", server_equiv::report::render(&results, true));

    let expected = expected_pass();
    let regressions: Vec<String> = results
        .iter()
        .filter(|r| expected.contains(&r.name) && !r.passed())
        .map(|r| match r.failure() {
            Some((i, s)) => format!("{} at step {i} ({}): {}", r.name, s.method, s.verdict.as_ref().map(|v| v.summary()).unwrap_or_default()),
            None => format!("{}: {}", r.name, r.error.clone().unwrap_or_default()),
        })
        .collect();
    let newly: Vec<&str> = results.iter().filter(|r| r.passed() && !expected.contains(&r.name)).map(|r| r.name.as_str()).collect();
    if !newly.is_empty() {
        println!("Passing but not in expected_pass.txt (add them): {newly:?}");
    }
    assert!(regressions.is_empty(), "scenarios in expected_pass.txt that fail:\n{}", regressions.join("\n"));
}
