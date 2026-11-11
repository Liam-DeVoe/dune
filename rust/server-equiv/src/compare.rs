//! Runs a scenario through the Rust hub and compares every step with the C# transcript.
//!
//! A step passes when every section of its Rust transcript line is byte-identical to the C#
//! one. A scenario stops at its first failing step: after a difference, later steps would
//! mostly report its consequences.

use std::collections::BTreeMap;
use std::path::Path;

use serde_json::value::RawValue;
use serde_json::Value;

use crate::json_text::sha256_hex;
use crate::runner::{Outcome, Runner};

/// The sections of a step line, in the order they are compared.
pub const SECTIONS: &[&str] = &["result", "callbacks", "mails", "timers", "fired", "randoms", "tables", "games", "hub"];

/// `Harness.TranscriptFormat`. 2: bot turns are compared (timers, the `randoms` section, the
/// bots of each game).
pub const TRANSCRIPT_FORMAT: i64 = 2;

/// State sections the C# transcript writes as `"="` when unchanged.
const STATE_SECTIONS: &[&str] = &["tables", "games", "hub"];

#[derive(Debug, Clone)]
pub enum Verdict {
    Pass,
    /// A section differs. `path` is the first differing JSON path inside it.
    Mismatch { section: String, path: String, csharp: String, rust: String },
    /// The Rust hub reached a `todo!()`.
    Todo(String),
    Panic(String),
    BadArgs(String),
}

impl Verdict {
    pub fn is_pass(&self) -> bool {
        matches!(self, Verdict::Pass)
    }

    pub fn summary(&self) -> String {
        match self {
            Verdict::Pass => "pass".into(),
            Verdict::Mismatch { section, path, csharp, rust } => format!("{section} differs at {path}\n      C#:   {csharp}\n      Rust: {rust}"),
            Verdict::Todo(m) => format!("todo: {m}"),
            Verdict::Panic(m) => format!("panic: {m}"),
            Verdict::BadArgs(m) => format!("bad arguments: {m}"),
        }
    }
}

/// One step of a scenario, as the report sees it.
#[derive(Debug, Clone)]
pub struct StepInfo {
    /// The hub method for calls; `(connect)`, `(advance)`, ... for other steps.
    pub method: String,
    /// `None`: not reached (the scenario stopped earlier).
    pub verdict: Option<Verdict>,
}

#[derive(Debug, Clone)]
pub struct ScenarioResult {
    pub name: String,
    pub steps: Vec<StepInfo>,
    /// A problem with the scenario or transcript files themselves.
    pub error: Option<String>,
}

impl ScenarioResult {
    pub fn passed(&self) -> bool {
        self.error.is_none() && self.steps.iter().all(|s| matches!(s.verdict, Some(Verdict::Pass)))
    }

    /// The first step that did not pass.
    pub fn failure(&self) -> Option<(usize, &StepInfo)> {
        self.steps.iter().enumerate().find(|(_, s)| !matches!(s.verdict, Some(Verdict::Pass)))
    }
}

fn step_method(step: &Value) -> String {
    match step.get("op").and_then(Value::as_str) {
        Some("call") => step.get("method").and_then(Value::as_str).unwrap_or("?").to_string(),
        Some(op) => format!("({op})"),
        None => "(?)".to_string(),
    }
}

/// Reads a transcript header and checks that it was written for exactly this scenario text.
pub fn check_transcript_header(scenario_text: &str, transcript_first_line: &str) -> Result<bool, String> {
    let header: Value = serde_json::from_str(transcript_first_line).map_err(|e| format!("bad transcript header: {e}"))?;
    if header.get("transcript").and_then(Value::as_i64) != Some(TRANSCRIPT_FORMAT) {
        return Err(format!("transcript format is not {TRANSCRIPT_FORMAT}; rerun rust/server-equiv/run-oracle.sh"));
    }
    let expected = sha256_hex(scenario_text.as_bytes());
    if header.get("scenarioSha256").and_then(Value::as_str) != Some(expected.as_str()) {
        return Err("the transcript was written for a different version of the scenario; rerun rust/server-equiv/run-oracle.sh".into());
    }
    Ok(header.get("full").and_then(Value::as_bool) == Some(true))
}

/// Runs `scenario` (a file) and compares with `transcript` (a file).
pub fn run_files(scenario: &Path, transcript: &Path, rust_out: Option<&Path>) -> ScenarioResult {
    let name = scenario.file_name().unwrap().to_string_lossy().trim_end_matches(".jsonl").to_string();
    let fail = |e: String| ScenarioResult { name: name.clone(), steps: Vec::new(), error: Some(e) };
    let Ok(scenario_text) = std::fs::read_to_string(scenario) else { return fail(format!("cannot read {}", scenario.display())) };
    let Ok(transcript_text) = std::fs::read_to_string(transcript) else { return fail(format!("no transcript {}", transcript.display())) };
    run_texts(&name, &scenario_text, &transcript_text, rust_out)
}

pub fn run_texts(name: &str, scenario_text: &str, transcript_text: &str, rust_out: Option<&Path>) -> ScenarioResult {
    let fail = |e: String| ScenarioResult { name: name.to_string(), steps: Vec::new(), error: Some(e) };
    let mut scenario_lines = scenario_text.lines().filter(|l| !l.trim().is_empty());
    let mut transcript_lines = transcript_text.lines().filter(|l| !l.trim().is_empty());
    let (Some(header), Some(transcript_header)) = (scenario_lines.next(), transcript_lines.next()) else {
        return fail("empty scenario or transcript".into());
    };
    let full = match check_transcript_header(scenario_text, transcript_header) {
        Ok(f) => f,
        Err(e) => return fail(e),
    };
    let header: Value = match serde_json::from_str(header) {
        Ok(h) => h,
        Err(e) => return fail(format!("bad scenario header: {e}")),
    };
    let steps: Vec<Value> = match scenario_lines.map(serde_json::from_str).collect::<Result<_, _>>() {
        Ok(s) => s,
        Err(e) => return fail(format!("bad scenario step: {e}")),
    };
    let expected: Vec<&str> = transcript_lines.collect();
    if expected.len() != steps.len() {
        return fail(format!("{} steps but {} transcript lines", steps.len(), expected.len()));
    }

    let mut runner = match Runner::new(&header, full) {
        Ok(r) => r,
        Err(e) => return fail(e),
    };
    let mut infos: Vec<StepInfo> = steps.iter().map(|s| StepInfo { method: step_method(s), verdict: None }).collect();
    let mut previous: BTreeMap<String, String> = BTreeMap::new();
    let mut rust_lines = Vec::new();
    for (i, step) in steps.iter().enumerate() {
        let verdict = match runner.step(step) {
            Outcome::Line(line) => {
                let v = compare_line(expected[i], &line, &mut previous);
                rust_lines.push(line);
                v
            }
            Outcome::Todo(m) => Verdict::Todo(m),
            Outcome::Panic(m) => Verdict::Panic(m),
            Outcome::BadArgs(m) => Verdict::BadArgs(m),
        };
        let stop = !verdict.is_pass();
        infos[i].verdict = Some(verdict);
        if stop {
            break;
        }
    }
    if let Some(dir) = rust_out {
        let _ = std::fs::create_dir_all(dir);
        let _ = std::fs::write(dir.join(format!("{name}.rust.jsonl")), rust_lines.join("\n") + "\n");
    }
    ScenarioResult { name: name.to_string(), steps: infos, error: None }
}

fn sections(line: &str) -> Result<BTreeMap<String, &RawValue>, String> {
    serde_json::from_str(line).map_err(|e| format!("unparsable line: {e}"))
}

/// Compares one Rust line with the C# line. `previous` holds the C# state sections of the last
/// line, to resolve `"="`.
pub fn compare_line(csharp: &str, rust: &str, previous: &mut BTreeMap<String, String>) -> Verdict {
    let c = match sections(csharp) {
        Ok(c) => c,
        Err(e) => return Verdict::Panic(format!("C# transcript: {e}")),
    };
    let r = match sections(rust) {
        Ok(r) => r,
        Err(e) => return Verdict::Panic(format!("Rust transcript: {e}")),
    };

    if let Some(expected) = c.get("rustOnly") {
        return compare_rust_only(expected.get(), &r, previous);
    }

    let mut resolved: BTreeMap<String, String> = BTreeMap::new();
    for section in SECTIONS {
        let Some(cv) = c.get(*section) else { return Verdict::Panic(format!("C# line has no {section}")) };
        let mut text = cv.get().to_string();
        if STATE_SECTIONS.contains(section) {
            if text == "\"=\"" {
                match previous.get(*section) {
                    Some(p) => text = p.clone(),
                    None => return Verdict::Panic(format!("C# line repeats {section} with no earlier value")),
                }
            }
            resolved.insert(section.to_string(), text.clone());
        }
        let rv = r.get(*section).map(|v| v.get()).unwrap_or("");
        if text != rv {
            let mut v = mismatch(section, &text, rv);
            if let (Verdict::Mismatch { rust, .. }, Some(d)) = (&mut v, r.get("rustDetail")) {
                rust.push_str(&format!(" [{}]", d.get()));
            }
            return v;
        }
    }
    previous.extend(resolved);
    Verdict::Pass
}

/// A step C# does not run (`rust_only`): the Rust result must contain `expected`'s members, and
/// nothing else may happen.
fn compare_rust_only(expected: &str, r: &BTreeMap<String, &RawValue>, previous: &BTreeMap<String, String>) -> Verdict {
    let expected: Value = serde_json::from_str(expected).unwrap_or(Value::Null);
    let result: Value = r.get("result").and_then(|v| serde_json::from_str(v.get()).ok()).unwrap_or(Value::Null);
    let actual = result.get("Result").cloned().unwrap_or(result.clone());
    if let Value::Object(members) = &expected {
        for (k, v) in members {
            if actual.get(k) != Some(v) {
                return Verdict::Mismatch {
                    section: "result".into(),
                    path: format!("rust_only.{k}"),
                    csharp: format!("(expected) {v}"),
                    rust: truncate(&result.to_string()),
                };
            }
        }
    }
    for (section, empty) in [("callbacks", "{}"), ("mails", "[]"), ("timers", "[]"), ("fired", "[]"), ("randoms", "[]")] {
        let rv = r.get(section).map(|v| v.get()).unwrap_or("");
        if rv != empty {
            return mismatch(section, empty, rv);
        }
    }
    for section in STATE_SECTIONS {
        let rv = r.get(*section).map(|v| v.get()).unwrap_or("");
        if let Some(p) = previous.get(*section) {
            if p != rv {
                return mismatch(section, p, rv);
            }
        }
    }
    Verdict::Pass
}

fn truncate(s: &str) -> String {
    if s.chars().count() > 300 { format!("{}...", s.chars().take(300).collect::<String>()) } else { s.to_string() }
}

fn mismatch(section: &str, csharp: &str, rust: &str) -> Verdict {
    let (path, c, r) = match (serde_json::from_str::<Value>(csharp), serde_json::from_str::<Value>(rust)) {
        (Ok(a), Ok(b)) => match first_difference(&a, &b, String::new()) {
            Some((p, a, b)) => (p, a, b),
            None => {
                // Same values, different text: an encoding difference (escaping, number format).
                let at = csharp.bytes().zip(rust.bytes()).take_while(|(a, b)| a == b).count();
                let from = at.saturating_sub(60);
                (
                    format!("byte {at} (same values, different JSON text)"),
                    csharp.get(from..).unwrap_or(csharp).to_string(),
                    rust.get(from..).unwrap_or(rust).to_string(),
                )
            }
        },
        _ => (String::new(), csharp.to_string(), rust.to_string()),
    };
    Verdict::Mismatch { section: section.to_string(), path, csharp: truncate(&c), rust: truncate(&r) }
}

/// The first place where two values differ, in document order.
fn first_difference(a: &Value, b: &Value, path: String) -> Option<(String, String, String)> {
    match (a, b) {
        (Value::Object(x), Value::Object(y)) => {
            let xk: Vec<&String> = x.keys().collect();
            let yk: Vec<&String> = y.keys().collect();
            for k in &xk {
                match y.get(*k) {
                    Some(yv) => {
                        if let Some(d) = first_difference(&x[*k], yv, format!("{path}.{k}")) {
                            return Some(d);
                        }
                    }
                    None => return Some((format!("{path}.{k}"), x[*k].to_string(), "(missing)".into())),
                }
            }
            if let Some(k) = yk.iter().find(|k| !x.contains_key(**k)) {
                return Some((format!("{path}.{k}"), "(missing)".into(), y[*k].to_string()));
            }
            if xk != yk {
                return Some((format!("{path} (member order)"), format!("{xk:?}"), format!("{yk:?}")));
            }
            None
        }
        (Value::Array(x), Value::Array(y)) => {
            for (i, (xv, yv)) in x.iter().zip(y.iter()).enumerate() {
                if let Some(d) = first_difference(xv, yv, format!("{path}[{i}]")) {
                    return Some(d);
                }
            }
            if x.len() != y.len() {
                return Some((format!("{path} (length {} vs {})", x.len(), y.len()), a.to_string(), b.to_string()));
            }
            None
        }
        _ if a == b => None,
        _ => Some((if path.is_empty() { "(top)".into() } else { path }, a.to_string(), b.to_string())),
    }
}
