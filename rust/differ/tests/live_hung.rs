//! `differ live` when the oracle exits with status 4 because a game hung.
//!
//! A script stands in for the oracle. For a batch of three games it writes one
//! finished trace, one trace that ends in `botHung`, no trace for the third,
//! and exits with 4, as `Treachery.Oracle` does when the watchdog gives up on
//! a game. The finished game must still be compared, the hung game reported as
//! hung, and the missing game reported as a bad trace. Before this was fixed,
//! status 4 made `live` drop the whole batch.

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use differ::live::{self, LiveOptions};
use differ::replay::{Failure, Options};
use differ::report;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;

fn sample(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../traces/sample").join(name)
}

fn write_gz(path: &Path, lines: &[String]) {
    let mut enc = GzEncoder::new(std::fs::File::create(path).unwrap(), flate2::Compression::fast());
    for l in lines {
        enc.write_all(l.as_bytes()).unwrap();
        enc.write_all(b"\n").unwrap();
    }
    enc.finish().unwrap();
}

#[cfg(unix)]
#[test]
fn hung_game_does_not_drop_the_batch() {
    use std::os::unix::fs::PermissionsExt;

    let work = std::env::temp_dir().join(format!("differ-live-hung-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&work);
    let src = work.join("src");
    std::fs::create_dir_all(&src).unwrap();

    // Game 0: a complete trace.
    let source = sample("bots-2026-00040.jsonl.gz");
    std::fs::copy(&source, src.join("bots-77-00000.jsonl.gz")).unwrap();

    // Game 1: the header of the same game, then a bot that never returned. No step lines, so the
    // outcome doesn't depend on whether the engine currently matches the sample traces.
    let header = BufReader::new(GzDecoder::new(std::fs::File::open(&source).unwrap())).lines().next().unwrap().unwrap();
    let hung = vec![header, r#"{"kind":"end","reason":"botHung","detail":"Bot deliberation did not return within 120 s","steps":0}"#.to_string()];
    write_gz(&src.join("bots-77-00001.jsonl.gz"), &hung);

    // Game 2: no trace at all.

    let script = work.join("fake-oracle.sh");
    std::fs::write(
        &script,
        format!(
            r#"#!/bin/bash
mode=$1; shift
while [ $# -gt 0 ]; do
  case $1 in --seed) seed=$2;; --start) start=$2;; --games) games=$2;; --out) out=$2;; esac
  shift 2
done
mkdir -p "$out"
for ((i=start; i<start+games; i++)); do
  f=$(printf "%s-%s-%05d.jsonl.gz" "$mode" "$seed" "$i")
  if [ -f "{src}/$f" ]; then cp "{src}/$f" "$out/"; fi
done
echo "$mode: game 1 hung inside Execute; continuing without it" >&2
exit 4
"#,
            src = src.display()
        ),
    )
    .unwrap();
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();

    let o = LiveOptions {
        mode: "bots".into(),
        seed: 77,
        start: 0,
        games: 3,
        jobs: 2,
        batch: 3,
        oracle: script,
        keep_failures: None,
        verbose: false,
        replay: Options::default(),
        status: false,
    };
    let results = live::run(&o).expect("an oracle exit status of 4 must not fail the run");
    let summary = report::summary(&results);
    let _ = std::fs::remove_dir_all(&work);

    assert_eq!(results.len(), 3, "{summary}");
    let by_index = |i: usize| {
        let name = format!("bots-77-{i:05}.jsonl.gz");
        results.iter().find(|r| r.path.file_name().unwrap().to_str() == Some(name.as_str())).unwrap_or_else(|| panic!("no result for {name}:\n{summary}"))
    };

    // The finished game was replayed (whether it matches depends on the engine, not on `live`).
    let done = by_index(0);
    assert!(!matches!(done.outcome.failure, Some(Failure::BadTrace(_))), "{summary}");
    assert!(done.outcome.at.is_some(), "{summary}");
    assert_eq!(done.outcome.csharp_hung, None);

    // The hung game was replayed up to the hang and is reported as hung.
    let h = by_index(1);
    assert_eq!(h.outcome.csharp_hung.as_deref(), Some("botHung"), "{summary}");
    assert!(h.outcome.passed(), "{summary}");
    assert!(report::one_line(h).starts_with("HUNG "), "{}", report::one_line(h));
    assert!(summary.contains("1 never finished in C#"), "{summary}");

    // The game with no trace is a failure, not silently skipped.
    match &by_index(2).outcome.failure {
        Some(Failure::BadTrace(m)) => assert!(m.contains("wrote no trace") && m.contains("exit 4"), "{m}"),
        other => panic!("expected a bad trace for the missing game, got {other:?}"),
    }
}
