//! The bot-decision harness (bots-plan.md, package B0): `bots` query entries of trace format 2.
//!
//! - Every bots-mode sample trace has `bots` entries, and they parse.
//! - Without `--bots check`, a bot entry changes nothing: no failure is a bot failure.
//! - With `--bots check`, a bots-mode trace never fails as a bad trace or with a harness error
//!   (a missing query result, a player that doesn't exist), so the B2-B6 porters can trust the
//!   "bot" groups of the summary.
//! - A tampered recorded choice or draw count fails with key `bot` or `botRandom`, so the check
//!   can't pass without comparing.
//! - Fuzz traces (`client` profile) are bot-checked the same way; `wide` ones and format-1 traces
//!   are not.

use std::io::BufRead;
use std::path::PathBuf;

use differ::replay::{self, Failure, Options};
use differ::trace::{self, Line, Query};

fn sample_traces(prefix: &str) -> Vec<PathBuf> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../traces/sample");
    trace::list(&dir)
        .into_iter()
        .filter(|p| p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with(prefix)))
        .collect()
}

/// The bots and fuzz traces of the sample corpus: both are bot-checked.
fn sample_checked_traces() -> Vec<PathBuf> {
    let mut files = sample_traces("bots-");
    let fuzz = sample_traces("fuzz-");
    assert!(!fuzz.is_empty(), "no fuzz traces in traces/sample");
    files.extend(fuzz);
    files
}

#[test]
fn bots_entries_parse() {
    let files = sample_checked_traces();
    assert!(!files.is_empty(), "no bots traces in traces/sample");
    for f in files {
        let reader = trace::open(&f).unwrap();
        let mut entries = 0;
        for (i, text) in reader.lines().enumerate() {
            let line: Line = serde_json::from_str(&text.unwrap()).unwrap_or_else(|e| panic!("{}: line {}: {e}", f.display(), i + 1));
            if i == 0 {
                assert_eq!(line.trace_format, Some(2), "{}: regenerate the sample corpus with the format-2 oracle", f.display());
            }
            for q in &line.queries {
                if let Query::Bots(b) = q {
                    entries += 1;
                    let mut order = b.order.clone();
                    order.sort();
                    assert_eq!(order, (0..order.len()).collect::<Vec<_>>(), "{}: order is not a permutation", f.display());
                    if let Some(c) = &b.choice {
                        assert!(["Highest", "High", "Middle", "Low", "End"].contains(&c.priority.as_str()));
                        assert!(c.event.get().starts_with("{\"$type\":"));
                    }
                }
            }
        }
        assert!(entries > 0, "{}: no bots entries", f.display());
    }
}

#[test]
fn bot_entries_are_ignored_without_check() {
    for f in sample_checked_traces() {
        let (outcome, _) = replay::replay_file(&f, &Options::default());
        assert_eq!(outcome.bot_decisions_ok, 0);
        if let Some(fail) = &outcome.failure {
            assert!(!fail.is_bot() && !matches!(fail, Failure::BadTrace(_)), "{}: {}", f.display(), fail.group());
        }
    }
}

#[test]
fn bot_check_never_breaks_the_replay() {
    let opts = Options { check_bots: true, ..Options::default() };
    for f in sample_checked_traces() {
        let (outcome, _) = replay::replay_file(&f, &opts);
        let Some(fail) = &outcome.failure else { continue };
        if let Failure::Mismatch(m) = fail {
            assert!(!m.actual.starts_with("harness:"), "{}: {}", f.display(), m.actual);
        }
        // Engine differences can come first too; what must never come is an unreadable trace.
        assert!(!matches!(fail, Failure::BadTrace(_)), "{}: {}", f.display(), fail.group());
    }
}

/// The sample trace's lines, with `edit` applied to the first line it changes (returns Some).
fn tampered(edit: impl Fn(&str) -> Option<String>, tag: &str) -> PathBuf {
    tampered_from("bots-2026-00042", edit, tag)
}

/// The sample trace `name`'s lines, with `edit` applied to the first line it changes (returns Some).
fn tampered_from(name: &str, edit: impl Fn(&str) -> Option<String>, tag: &str) -> PathBuf {
    let src = sample_checked_traces().into_iter().find(|p| p.to_string_lossy().contains(name)).unwrap_or_else(|| panic!("{name} in traces/sample"));
    let mut out = String::new();
    let mut done = false;
    for text in trace::open(&src).unwrap().lines() {
        let text = text.unwrap();
        match (!done).then(|| edit(&text)).flatten() {
            Some(changed) => {
                out.push_str(&changed);
                done = true;
            }
            None => out.push_str(&text),
        }
        out.push('\n');
    }
    assert!(done, "{tag}: nothing to tamper with");
    let path = std::env::temp_dir().join(format!("bots_harness_{tag}_{}.jsonl", std::process::id()));
    std::fs::write(&path, out).unwrap();
    path
}

fn check_fails_with(path: &PathBuf, key: &str) {
    let (outcome, _) = replay::replay_file(path, &Options { check_bots: true, ..Options::default() });
    let _ = std::fs::remove_file(path);
    match &outcome.failure {
        Some(f @ Failure::Mismatch(m)) if m.key == key => assert!(f.is_bot(), "{}", f.group()),
        other => panic!("expected a `{key}` difference, got {:?}", other.as_ref().map(|f| f.group())),
    }
}

/// A recorded choice at another level than C# used: the Rust bots must disagree with it.
#[test]
fn a_different_choice_is_reported() {
    let path = tampered(
        |line| {
            let at = line.find("[\"bots\",{")?;
            let rel = line[at..].find("\"priority\":\"")? + at + "\"priority\":\"".len();
            let end = line[rel..].find('"')? + rel;
            let other = if &line[rel..end] == "Highest" { "High" } else { "Highest" };
            Some(format!("{}{other}{}", &line[..rel], &line[end..]))
        },
        "choice",
    );
    check_fails_with(&path, "bot");
}

/// A recorded draw count one higher than C#'s: the registry's counts must disagree with it.
#[test]
fn a_different_draw_count_is_reported() {
    let path = tampered(
        |line| {
            let at = line.find("\"botRandom\":[[\"")? + "\"botRandom\":[[\"".len();
            let start = line[at..].find("\",")? + at + 2;
            let end = line[start..].find(']')? + start;
            let n: i64 = line[start..end].parse().ok()?;
            Some(format!("{}{}{}", &line[..start], n + 1, &line[end..]))
        },
        "draws",
    );
    check_fails_with(&path, "botRandom");
}

/// A fuzz trace is bot-checked: its deliberations are run and counted, and a tampered choice fails.
#[test]
fn fuzz_traces_are_checked() {
    let f = sample_traces("fuzz-2026-00018").pop().expect("fuzz-2026-00018 in traces/sample");
    let (outcome, _) = replay::replay_file(&f, &Options { check_bots: true, ..Options::default() });
    assert!(outcome.bot_decisions_ok > 0, "no deliberation checked in {}", f.display());

    let path = tampered_from(
        "fuzz-2026-00018",
        |line| {
            let at = line.find("[\"bots\",{")?;
            let rel = line[at..].find("\"priority\":\"")? + at + "\"priority\":\"".len();
            let end = line[rel..].find('"')? + rel;
            let other = if &line[rel..end] == "Highest" { "High" } else { "Highest" };
            Some(format!("{}{other}{}", &line[..rel], &line[end..]))
        },
        "fuzz_choice",
    );
    check_fails_with(&path, "bot");
}

/// A `wide` fuzz trace (forged values, outside the port's scope) and a format-1 trace are never bot-checked.
#[test]
fn wide_and_format_1_traces_are_not_checked() {
    for (tag, from, to) in [("wide", "\"profile\":\"client\"", "\"profile\":\"wide\""), ("format1", "\"traceFormat\":2", "\"traceFormat\":1")] {
        let path = tampered_from("fuzz-2026-00018", |line| line.contains(from).then(|| line.replacen(from, to, 1)), tag);
        let (outcome, _) = replay::replay_file(&path, &Options { check_bots: true, ..Options::default() });
        let _ = std::fs::remove_file(&path);
        assert_eq!(outcome.bot_decisions_ok, 0, "{tag}: bots were checked");
        if let Some(fail) = &outcome.failure {
            assert!(!fail.is_bot(), "{tag}: {}", fail.group());
        }
    }
}

/// Fuzz seed 109 game 40 (known-issues.md, "Rock Melter bot logs a message during deliberation"):
/// at line 1139 the Tleilaxu bot's `DetermineRockWasMelted` logs a message on the live game while
/// deliberating, and the fuzzer submits a rejected `DealAccepted`. Without the bots the replay can't
/// produce the message; with them, every line matches. The golden is the trace cut after line 1140.
#[test]
fn a_bot_that_changes_the_game_is_reproduced_in_fuzz() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden/fuzz-109-00040-rockmelter.jsonl.gz");
    let opts = Options { stop_after: Some(1140), ..Options::default() };

    let (outcome, _) = replay::replay_file(&path, &opts);
    match &outcome.failure {
        Some(Failure::Mismatch(m)) => assert!(m.key == "hash" && m.n == "1139", "expected the Rock Melter hash difference at 1139, got {} at {}", m.key, m.n),
        other => panic!("expected a hash difference without the bots, got {:?}", other.as_ref().map(|f| f.group())),
    }

    let (outcome, _) = replay::replay_file(&path, &Options { check_bots: true, ..opts });
    assert!(outcome.failure.is_none(), "{:?}", outcome.failure.as_ref().map(|f| f.group()));
    assert!(outcome.bot_decisions_ok > 900, "{} deliberations checked", outcome.bot_decisions_ok);
}
