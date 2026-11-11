//! Reading trace files (trace-format.md sections 1 to 5).

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use serde::Deserialize;
use serde_json::value::RawValue;
use serde_json::Value;

/// One line of a trace. Only the keys the replayer reads; the rest is ignored.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Line {
    pub kind: String,
    pub n: Option<i64>,
    // header
    pub trace_format: Option<i64>,
    pub dump_format: Option<i64>,
    /// `bots`, `fuzz` or `replay`.
    pub mode: Option<String>,
    pub game_version: Option<i32>,
    pub participation: Option<Value>,
    /// Fuzz only: the fuzz settings, with the `profile` (`client` or `wide`).
    pub fuzz: Option<Value>,
    // step
    /// The engine calls since the previous line, in order (trace-format.md section 3).
    #[serde(default)]
    pub queries: Vec<Query>,
    pub validate: Option<bool>,
    pub is_host: Option<bool>,
    pub event: Option<Box<RawValue>>,
    pub event_after: Option<Box<RawValue>>,
    pub result: Option<String>,
    pub validation: Option<Box<RawValue>>,
    pub exception: Option<Box<RawValue>>,
    // end
    pub reason: Option<String>,
    /// `botException` end lines: the exception, as "System.Type: message".
    pub detail: Option<String>,
    // undo, and `undoHung` end lines
    pub until_event_nr: Option<i32>,
    /// `undoHung` end lines: `undo`, `restoreRecentlyUndone` or `dismissRecentlyUndone`.
    pub undo_kind: Option<String>,
    // after-state
    pub history_count: Option<i64>,
    pub random: Option<i64>,
    /// `[[faction, count], ...]`: draws from each bot's own random, in bot creation order.
    pub bot_random: Option<Vec<(String, i64)>>,
    #[serde(default)]
    pub type_lists: Vec<(i64, Box<RawValue>)>,
    #[serde(default)]
    pub applicable: Vec<(i64, i64)>,
    pub hash: Option<String>,
    pub state: Option<Box<RawValue>>,
}

/// One entry of a line's `queries`.
pub enum Query {
    /// `[player, isHost]`: `Game.GetApplicableEvents(player, isHost)`, result unused. When the call
    /// threw, `[player, isHost, exception]`, with the exception as in `applicable` (trace-format.md 7.4).
    Applicable { player: Option<usize>, is_host: bool, exception: Option<Box<RawValue>> },
    /// `["status", player, isPlayer, result]`: `GameStatus.DetermineStatus(game, player, isPlayer)`
    /// (only in traces recorded with `--status every`). `result` is the canonical dump of the
    /// returned `GameStatus`, or of the exception it threw.
    Status { player: Option<usize>, is_player: bool, result: Box<RawValue> },
    /// `["bots", {...}]` (trace format 2): one bot deliberation of the oracle's `DeliberateBots`.
    Bots(Box<BotsEntry>),
}

/// A `bots` query entry (trace-format.md section 3, "Bot deliberations").
#[derive(Deserialize)]
pub struct BotsEntry {
    /// The player indices in the order their bots were asked.
    pub order: Vec<usize>,
    /// Whether the End level was asked (false for `ServerHumanHost`).
    pub end: bool,
    /// Bots created during this deliberation, in creation order: `[faction, player, seed]`.
    pub created: Vec<(String, usize, i32)>,
    /// The event the bots chose, or null.
    pub choice: Option<BotChoice>,
    /// Present when deliberation threw: the exception as in 7.4.
    pub exception: Option<Box<RawValue>>,
}

#[derive(Deserialize)]
pub struct BotChoice {
    pub player: usize,
    pub faction: String,
    /// `Highest`, `High`, `Middle`, `Low` or `End`.
    pub priority: String,
    /// `Utilities.Serialize<GameEvent>` of the chosen event, before `Time` was set.
    pub event: Box<RawValue>,
}

impl<'de> Deserialize<'de> for Query {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        use serde::de::Error;
        let raw = <Box<RawValue>>::deserialize(d)?;
        let text = raw.get();
        if text.trim_start().trim_start_matches('[').trim_start().starts_with('"') {
            let parts: Vec<Box<RawValue>> = serde_json::from_str(text).map_err(D::Error::custom)?;
            let tag: String = serde_json::from_str(parts.first().map(|p| p.get()).unwrap_or("null")).map_err(D::Error::custom)?;
            match tag.as_str() {
                "status" => {
                    let (_, player, is_player, result): (String, Option<usize>, bool, Box<RawValue>) =
                        serde_json::from_str(text).map_err(D::Error::custom)?;
                    Ok(Query::Status { player, is_player, result })
                }
                "bots" => {
                    let (_, entry): (String, BotsEntry) = serde_json::from_str(text).map_err(D::Error::custom)?;
                    Ok(Query::Bots(Box::new(entry)))
                }
                _ => Err(D::Error::custom(format!("unknown query kind {tag}"))),
            }
        } else {
            let parts: Vec<Box<RawValue>> = serde_json::from_str(text).map_err(D::Error::custom)?;
            if parts.len() != 2 && parts.len() != 3 {
                return Err(D::Error::custom(format!("query entry with {} elements", parts.len())));
            }
            let player: Option<usize> = serde_json::from_str(parts[0].get()).map_err(D::Error::custom)?;
            let is_host: bool = serde_json::from_str(parts[1].get()).map_err(D::Error::custom)?;
            let exception = parts.into_iter().nth(2);
            Ok(Query::Applicable { player, is_host, exception })
        }
    }
}

/// Opens a `.jsonl` or `.jsonl.gz` trace.
pub fn open(path: &Path) -> Result<Box<dyn BufRead>, String> {
    let f = File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if path.extension().is_some_and(|e| e == "gz") {
        Ok(Box::new(BufReader::with_capacity(1 << 20, flate2::read::MultiGzDecoder::new(f))))
    } else {
        Ok(Box::new(BufReader::with_capacity(1 << 20, f)))
    }
}

/// The trace files in a directory (or the file itself), sorted by name.
pub fn list(path: &Path) -> Vec<std::path::PathBuf> {
    if path.is_file() {
        return vec![path.to_path_buf()];
    }
    let mut files: Vec<_> = std::fs::read_dir(path)
        .map(|rd| rd.filter_map(|e| e.ok()).map(|e| e.path()).collect())
        .unwrap_or_default();
    files.retain(|p| {
        let n = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
        n.ends_with(".jsonl") || n.ends_with(".jsonl.gz")
    });
    files.sort();
    files
}
