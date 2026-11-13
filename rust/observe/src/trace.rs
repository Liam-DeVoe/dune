//! Trace lines. One game is one JSON Lines stream: a header, then one line per call the driver
//! made on the game (queries, bot deliberations, events, undo), then an end line. Each line holds
//! the call's inputs and what a player or observer could see of its outcome. The format is
//! described in rust/docs/native-harness.md ("Trace format").

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Bumped when a change makes old traces unreadable. The replayer refuses other formats.
pub const FORMAT: u32 = 1;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "op", rename_all = "camelCase")]
pub enum Line {
    Header(Header),
    /// `Game::get_applicable_events(player, host)`. `p` is a player index, or null for no player.
    Query {
        p: Option<usize>,
        host: bool,
        #[serde(flatten)]
        result: QueryResult,
    },
    /// One round of asking the bots, as the server's `perform_bot_event`: levels Highest to Low,
    /// then End if `end`, players in `order` within each level, the first event wins. Each bot
    /// gets the result of the latest `Query` line for its player.
    Bots {
        order: Vec<usize>,
        end: bool,
        /// Bots created during this round: `[faction, seed]`. A bot is created the first time
        /// a player of its faction is asked.
        created: Vec<(String, i32)>,
        #[serde(flatten)]
        result: BotsResult,
    },
    /// An event submitted with validation, as a client or bot submission is.
    Event {
        /// Who made it: `host`, `bot`, `human`, `fuzz`, `mutate`. Informational.
        src: String,
        /// The player it was made for, if any. Informational.
        p: Option<usize>,
        host: bool,
        event: Value,
        #[serde(flatten)]
        outcome: EventOutcome,
    },
    Undo {
        until: i32,
        #[serde(flatten)]
        outcome: SpecialOutcome,
    },
    Restore {
        #[serde(flatten)]
        outcome: SpecialOutcome,
    },
    Dismiss {
        #[serde(flatten)]
        outcome: SpecialOutcome,
    },
    End {
        /// `gameEnded`, `cap`, `stalled`, `botException`, `panic` (the previous line panicked),
        /// `hung` or `memoryLimit` (`pending` never returned or grew too large).
        reason: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        detail: Option<String>,
        /// For `hung` and `memoryLimit`: the line whose call never finished, without outcome.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pending: Option<Box<Line>>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Header {
    pub format: u32,
    pub mode: String,
    pub seed: u64,
    pub index: u64,
    pub version: i32,
    /// `[userId, seat, name]` of every seated human (Participation.SeatedPlayers and PlayerNames).
    pub seated: Vec<(i32, i32, String)>,
    /// How the game was configured. Informational: everything the engine sees is in the events.
    pub config: Value,
    /// Whether lines carry the full views (`--full-views`), not just their hashes.
    #[serde(default)]
    pub full_views: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct QueryResult {
    /// The applicable event type names, in the engine's order.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub types: Option<Vec<String>>,
    /// The engine threw instead.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub panic: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct BotsResult {
    /// The chosen event, or none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub choice: Option<BotChoice>,
    /// A bot threw (the game ends with `botException`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub panic: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct BotChoice {
    pub p: usize,
    pub level: String,
    pub event: Value,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct EventOutcome {
    /// `accepted` (history grew), `rejected` (a validation message), `error` (the engine threw;
    /// the message is the "Game Error" text), `unreadable` (the event JSON doesn't deserialize)
    /// or `panic`.
    pub result: String,
    /// The returned message rendered with the default skin, as the server shows it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(flatten)]
    pub obs: Obs,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct SpecialOutcome {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub panic: bool,
    #[serde(flatten)]
    pub obs: Obs,
}

/// What players can see after a call.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Obs {
    /// The log shrank to this many messages (after undo).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub log_cut: Option<usize>,
    /// Messages added to the log: `[target faction or "", text]`, rendered with the default skin.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub log: Vec<(String, String)>,
    /// Hash of each view, when the history changed: one per player in `Game.Players` order, then
    /// the seatless host, then an observer. See `session::VIEWERS`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub views: Option<Vec<String>>,
    /// With `--full-views`: the views themselves, in the same order.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub full: Option<Vec<Value>>,
    /// The view replica could not apply an event the game accepted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replica: Option<String>,
}

fn is_false(b: &bool) -> bool {
    !*b
}

impl Line {
    pub fn op(&self) -> &'static str {
        match self {
            Line::Header(_) => "header",
            Line::Query { .. } => "query",
            Line::Bots { .. } => "bots",
            Line::Event { .. } => "event",
            Line::Undo { .. } => "undo",
            Line::Restore { .. } => "restore",
            Line::Dismiss { .. } => "dismiss",
            Line::End { .. } => "end",
        }
    }

    /// The line's inputs only: what a replayer needs to make the same call.
    pub fn without_outcome(&self) -> Line {
        match self {
            Line::Query { p, host, .. } => Line::Query { p: *p, host: *host, result: QueryResult::default() },
            Line::Bots { order, end, created, .. } => Line::Bots { order: order.clone(), end: *end, created: created.clone(), result: BotsResult::default() },
            Line::Event { src, p, host, event, .. } => Line::Event { src: src.clone(), p: *p, host: *host, event: event.clone(), outcome: EventOutcome::default() },
            Line::Undo { until, .. } => Line::Undo { until: *until, outcome: SpecialOutcome::default() },
            Line::Restore { .. } => Line::Restore { outcome: SpecialOutcome::default() },
            Line::Dismiss { .. } => Line::Dismiss { outcome: SpecialOutcome::default() },
            other => other.clone(),
        }
    }

    /// Whether this line's call ended the game abnormally (a panic in the engine or a bot).
    pub fn panicked(&self) -> bool {
        match self {
            Line::Query { result, .. } => result.panic,
            Line::Bots { result, .. } => result.panic,
            Line::Event { outcome, .. } => outcome.result == "panic",
            Line::Undo { outcome, .. } | Line::Restore { outcome, .. } | Line::Dismiss { outcome, .. } => outcome.panic,
            _ => false,
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("trace lines serialize")
    }
}
