//! Every accepted event in the recorded games the tests replay still decodes.
//!
//! The recorded games come from older builds, where any number was a value of every enum. An
//! event holding a number that names no member is refused now, so a test that replays it stops
//! there, and the forms or texts after it are no longer covered. This test names such events, so
//! the trace can be cut or rerecorded on purpose.

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use treachery_engine::events::HistoryEntry;
use treachery_legacy as json;

fn traces(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            traces(&path, out);
        } else if path.to_string_lossy().ends_with(".jsonl.gz") {
            out.push(path);
        }
    }
}

#[test]
fn accepted_events_of_recorded_games_decode() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut files = Vec::new();
    for dir in ["testdata/games", "view/tests/data", "bots/tests/golden", "skin/tests/golden"] {
        traces(&root.join(dir), &mut files);
    }
    files.sort();
    let mut failures = Vec::new();
    for path in &files {
        let reader = BufReader::new(flate2::read::GzDecoder::new(std::fs::File::open(path).unwrap()));
        for (i, line) in reader.lines().enumerate() {
            let v: serde_json::Value = serde_json::from_str(&line.unwrap()).unwrap();
            if v["kind"] != "step" || v["result"] != "accepted" {
                continue;
            }
            let text = v["event"].to_string();
            if let Err(e) = json::deserialize::<HistoryEntry>(&text) {
                failures.push(format!("{} line {}: {}: {text}", path.strip_prefix(&root).unwrap().display(), i + 1, e.message));
            }
        }
    }
    assert!(files.len() > 50, "found only {} recorded games", files.len());
    assert!(failures.is_empty(), "{} accepted events no longer decode:\n{}", failures.len(), failures.join("\n"));
}
