//! Replaying one trace (trace-format.md section 6) and comparing every line.

use std::io::BufRead;
use std::panic::{self, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::Arc;
use std::time::Duration;
use std::path::Path;

use serde_json::value::RawValue;
use serde_json::Value;
use treachery_engine::canon::{self, Canon, CanonWriter};
use treachery_engine::compat::cs::{self, CsException};
use treachery_engine::enumerations::Milestone;
use treachery_engine::events::{GameEvent, GameEventType};
use treachery_engine::messages::expression::ExpressionElement;
use treachery_engine::game::Game;
use treachery_engine::game::GameStatus;
use treachery_engine::messages::message::Message;
use treachery_engine::game::Participation;
use treachery_engine::enumerations::Faction;
use treachery_engine::random::SharedRandom;
use treachery_bots::{BotParameters, ClassicBot, IBot};


use crate::diff;
use crate::memory;
use crate::panics;
use crate::trace::{self, BotsEntry, Line, Query};

/// Why a replay stopped.
#[derive(Clone, Debug)]
pub enum Failure {
    /// A Rust panic that is not a C# exception: a `todo!`, `unimplemented!`,
    /// `unwrap` or index out of bounds. Usually a part of the engine that is
    /// not ported yet.
    NotPorted { message: String, location: String },
    /// A C# exception escaped where the oracle did not see one (outside the
    /// places the trace records exceptions).
    Uncaught { during: String, exception: String, location: String },
    /// A recorded value differs.
    Mismatch(Mismatch),
    /// The trace itself could not be read.
    BadTrace(String),
    /// The replay thread went over its memory budget (`Options::memory_limit`) and was stopped.
    MemoryLimit { limit_mb: usize },
}

#[derive(Clone, Debug)]
pub struct Mismatch {
    /// The line's `n` (step number), or `end`.
    pub n: String,
    /// The line's `kind`.
    pub kind: String,
    /// The event type, for step lines.
    pub event_type: Option<String>,
    /// Which recorded value differs: `result`, `validation`, `exception`,
    /// `eventAfter`, `historyCount`, `random`, `applicable`, `hash`.
    pub key: String,
    pub expected: String,
    pub actual: String,
    /// For hash mismatches with a full dump on the line: the first fields that differ.
    pub state_diff: Vec<diff::Difference>,
    /// True when the line has no `state` to diff against.
    pub no_state: bool,
}

impl Failure {
    /// The key used to group failures across traces.
    pub fn group(&self) -> String {
        match self {
            Failure::NotPorted { message, location } => format!("not ported yet: {message} (at {location})"),
            Failure::Uncaught { during, exception, location } => {
                format!("uncaught C# exception during {during}: {exception} (at {location})")
            }
            Failure::Mismatch(m) => {
                let what = match &m.event_type {
                    Some(t) => format!("after {t}"),
                    None => format!("on {} line", m.kind),
                };
                if m.key == "bot" || m.key == "botRandom" {
                    // `event_type` holds the label of the deliberation: which decision C# made, at which level.
                    let label = m.event_type.as_deref().unwrap_or("?");
                    return if m.key == "bot" { format!("bot decision differs: {label}") } else { format!("bot random draws differ: {label}") };
                }
                let field = m.state_diff.first().map(|d| format!(", first at {}", diff::generic_path(&d.path))).unwrap_or_default();
                format!("{} differs {what}{field}", m.key)
            }
            Failure::BadTrace(m) => format!("bad trace: {m}"),
            Failure::MemoryLimit { limit_mb } => format!("memory limit: the Rust replay went over {limit_mb} MB"),
        }
    }

    pub fn is_not_ported(&self) -> bool {
        matches!(self, Failure::NotPorted { .. })
    }

    /// A failure of the bots rather than the engine: a bot decision or bot draw count that differs,
    /// or a panic inside the bots crate (an unported bot method).
    pub fn is_bot(&self) -> bool {
        match self {
            Failure::Mismatch(m) => m.key == "bot" || m.key == "botRandom",
            Failure::NotPorted { location, message } => location.contains("bots/src/") || message.contains("port: Bot"),
            _ => false,
        }
    }
}

/// The outcome of replaying one trace.
#[derive(Clone, Debug)]
pub struct Outcome {
    /// Lines replayed successfully (header excluded).
    pub lines_ok: usize,
    /// Total lines in the trace (header excluded), when known.
    pub failure: Option<Failure>,
    /// The line (by `n`) at which the failure happened.
    pub at: Option<String>,
    /// The end reason when C# never finished the game: `hung` (an `Execute`), `undoHung` or `botHung`.
    /// With no failure, Rust hung on the same call too (or, for `botHung`, every line before matched).
    pub csharp_hung: Option<String>,
    /// Bot deliberations (`bots` query entries) whose Rust result matched C#, with `Options::check_bots`.
    pub bot_decisions_ok: usize,
}

impl Outcome {
    pub fn passed(&self) -> bool {
        self.failure.is_none()
    }

    /// An outcome for a trace that could not be replayed at all.
    pub fn bad_trace(message: String) -> Outcome {
        Outcome { lines_ok: 0, failure: Some(Failure::BadTrace(message)), at: None, csharp_hung: None, bot_decisions_ok: 0 }
    }
}

/// Options for a replay.
#[derive(Clone, Debug)]
pub struct Options {
    /// How long the event of a `hung` end line may run before it counts as
    /// hanging in Rust too (a match).
    pub hang_budget: Duration,
    /// How long any other line may take before the replay gives up and
    /// reports that Rust did not return where C# did.
    pub step_timeout: Duration,
    /// How many state differences to collect on a hash mismatch.
    pub max_diffs: usize,
    /// Stop after this many lines (for `first --step`), 0 = no limit.
    pub stop_after: Option<i64>,
    /// When set, the replay stores the details of the failing line here.
    pub detail: bool,
    /// Memory budget of one replay in bytes (0 = none). See `memory.rs`.
    pub memory_limit: usize,
    /// Run the Rust bots at every `bots` query entry of a `bots`-mode trace and compare their
    /// decisions and draw counts (bots-plan.md, "Differ changes"). Off by default, so engine
    /// replays don't stop at unported bot code.
    pub check_bots: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            hang_budget: Duration::from_secs(10),
            step_timeout: Duration::from_secs(120),
            max_diffs: 10,
            stop_after: None,
            detail: false,
            memory_limit: 2048 << 20,
            check_bots: false,
        }
    }
}

/// Details of the failing line, for `differ first`.
#[derive(Clone, Debug, Default)]
pub struct Detail {
    /// The raw trace line (without `state`).
    pub line_summary: String,
    /// Every compared key that differs on this line (not just the first).
    pub all_mismatches: Vec<(String, String, String)>,
    pub expected_state: Option<String>,
    pub actual_state: Option<String>,
}

/// Progress reports from a replay thread.
enum Progress {
    /// Starting a line (its `n`, or its kind), and the bot decisions matched so far.
    Line(String, usize),
    /// Starting the call of a `hung` or `undoHung` end line, which C# never returned from (the end reason).
    Hung(String),
    Done(Outcome, Option<Detail>),
}

/// Replays the trace at `path` on a thread of its own, with deadlines.
///
/// A line that takes longer than `step_timeout` is a failure (C# returned,
/// Rust did not). The event of a `hung` end line that is still running after
/// `hang_budget` is a match, because C# never returned from it either. In
/// both cases the replay thread is left running, as the oracle does with its
/// hung threads; it ends with the process.
pub fn replay_file(path: &Path, opts: &Options) -> (Outcome, Option<Detail>) {
    let (tx, rx) = mpsc::channel();
    let path_buf = path.to_path_buf();
    let thread_opts = opts.clone();
    let over_budget = Arc::new(AtomicBool::new(false));
    let thread_flag = over_budget.clone();
    let spawned = std::thread::Builder::new()
        .name(format!("replay {}", path.display()))
        .stack_size(256 << 20)
        .spawn(move || {
            let _budget = memory::set_budget(thread_opts.memory_limit, thread_flag);
            let (outcome, detail) = match trace::open(&path_buf) {
                Ok(reader) => replay_with_progress(reader, &thread_opts, Some(tx.clone())),
                Err(e) => (Outcome::bad_trace(e), None),
            };
            let _ = tx.send(Progress::Done(outcome, detail));
        });
    if let Err(e) = spawned {
        return (Outcome::bad_trace(format!("cannot start a thread: {e}")), None);
    }

    let mut at: Option<String> = None;
    let mut lines = 0usize;
    let mut bot_decisions_ok = 0usize;
    let mut hung: Option<String> = None;
    let mut waited = Duration::ZERO;
    loop {
        let budget = if hung.is_some() { opts.hang_budget } else { opts.step_timeout };
        // Wait in short slices, to notice a replay that went over its memory budget.
        let slice = Duration::from_millis(250).min(budget.saturating_sub(waited));
        let received = rx.recv_timeout(slice);
        if over_budget.load(Ordering::SeqCst) {
            // The replay thread sleeps for good inside the allocator (memory.rs). When C#
            // gave up on this call for memory, or never returned from it, that is a match.
            let failure = match hung.as_deref() {
                Some("memoryLimit") | Some("hung") | Some("undoHung") => None,
                _ => Some(Failure::MemoryLimit { limit_mb: opts.memory_limit >> 20 }),
            };
            return (Outcome { lines_ok: lines + usize::from(failure.is_none()), failure, at, csharp_hung: hung, bot_decisions_ok }, None);
        }
        let received = match received {
            Err(RecvTimeoutError::Timeout) if waited + slice < budget => {
                waited += slice;
                continue;
            }
            other => {
                waited = Duration::ZERO;
                other
            }
        };
        match received {
            Ok(Progress::Line(n, bots_ok)) => {
                if at.is_some() {
                    lines += 1;
                }
                at = Some(n);
                bot_decisions_ok = bots_ok;
            }
            Ok(Progress::Hung(reason)) => hung = Some(reason),
            Ok(Progress::Done(outcome, detail)) => return (outcome, detail),
            Err(RecvTimeoutError::Timeout) => {
                let failure = match &hung {
                    // Both engines loop: equivalent.
                    Some(_) => None,
                    None => Some(Failure::Mismatch(Mismatch {
                        n: at.clone().unwrap_or_default(),
                        kind: "step".into(),
                        event_type: None,
                        key: "termination".into(),
                        expected: "the line finishes (C# returned)".into(),
                        actual: format!("Rust did not finish the line within {} s", budget.as_secs()),
                        state_diff: Vec::new(),
                        no_state: false,
                    })),
                };
                return (Outcome { lines_ok: lines + usize::from(failure.is_none()), failure, at, csharp_hung: hung, bot_decisions_ok }, None);
            }
            Err(RecvTimeoutError::Disconnected) => {
                return (Outcome { lines_ok: lines, failure: Some(Failure::BadTrace("replay thread vanished".into())), at, csharp_hung: hung, bot_decisions_ok }, None)
            }
        }
    }
}

/// Replays a trace from any line reader on the current thread, without deadlines.
pub fn replay_reader(reader: Box<dyn BufRead>, opts: &Options) -> (Outcome, Option<Detail>) {
    replay_with_progress(reader, opts, None)
}

fn replay_with_progress(reader: Box<dyn BufRead>, opts: &Options, progress: Option<Sender<Progress>>) -> (Outcome, Option<Detail>) {
    panics::install();
    let mut state = Replayer::new(opts.clone());
    state.progress = progress;
    let result = panic::catch_unwind(AssertUnwindSafe(|| state.run(reader)));
    let failure = match result {
        Ok(Ok(())) => None,
        Ok(Err(f)) => Some(f),
        Err(payload) => Some(failure_from_panic(payload, &state.during)),
    };
    let detail = state.detail.take();
    (Outcome { lines_ok: state.lines_ok, failure, at: state.at.clone(), csharp_hung: state.csharp_hung.clone(), bot_decisions_ok: state.bot_decisions_ok }, detail)
}

fn failure_from_panic(payload: Box<dyn std::any::Any + Send>, during: &str) -> Failure {
    let info = panics::take_last().unwrap_or_default();
    if let Some(e) = as_exception(payload.as_ref()) {
        return Failure::Uncaught {
            during: during.to_string(),
            exception: format!("{}: {}", cs::full_type_name(e.type_name), e.message),
            location: info.location,
        };
    }
    Failure::NotPorted { message: info.message, location: info.location }
}

struct Replayer {
    opts: Options,
    game: Option<Game>,
    /// The trace's `typeLists` table: index -> list JSON.
    type_lists: Vec<String>,
    lines_ok: usize,
    at: Option<String>,
    during: String,
    detail: Option<Detail>,
    progress: Option<Sender<Progress>>,
    /// Set on reaching a `hung`, `undoHung` or `botHung` end line.
    csharp_hung: Option<String>,
    /// Whether this replay runs the Rust bots (`Options::check_bots` and a `bots`-mode trace).
    bots_active: bool,
    /// The bots, one per faction, in creation order, as the oracle's `_bots`. They outlive undo.
    bots: Vec<RegisteredBot>,
    /// The result of the latest `GetApplicableEvents` query of each player on the current line:
    /// the event lists the oracle handed its bots.
    queried: Vec<Option<Vec<GameEventType>>>,
    /// The label of the current line's bot deliberation, for a `botRandom` difference.
    deliberation: Option<String>,
    bot_decisions_ok: usize,
}

struct RegisteredBot {
    faction: Faction,
    /// The faction as the trace writes it, `"Atreides(20)"`.
    label: String,
    bot: ClassicBot,
}

/// The oracle's `Levels`, in order.
const BOT_LEVELS: [&str; 5] = ["Highest", "High", "Middle", "Low", "End"];

/// `"Atreides(20)"` -> `Faction(20)`.
fn parse_faction(label: &str) -> Option<Faction> {
    let open = label.rfind('(')?;
    let number = label[open + 1..].strip_suffix(')')?;
    number.parse().ok().map(Faction)
}

/// Whether `--bots check` runs the Rust bots on a trace with this header: format 2 or later (format 1
/// has no `bots` entries), recorded in `bots` mode or in `fuzz` mode with the `client` profile. A
/// `wide` fuzz trace submits forged values, outside PORTING.md's scope, so its bots aren't checked.
pub fn bots_checkable(header: &Line) -> bool {
    header.trace_format.is_some_and(|f| f >= 2)
        && match header.mode.as_deref() {
            Some("bots") => true,
            Some("fuzz") => header.fuzz.as_ref().and_then(|f| f["profile"].as_str()) == Some("client"),
            _ => false,
        }
}

/// The event types whose class overrides `GameEvent.Execute` (no try/catch,
/// exceptions escape). `StepExecutor.OverridesExecute` in the oracle.
fn overrides_execute(t: GameEventType) -> bool {
    t == GameEventType::EstablishPlayers
}

/// The marker milestone of `StepExecutor.Run` (`(Milestone)int.MinValue`).
const SENTINEL: Milestone = Milestone(i32::MIN);

impl Replayer {
    fn new(opts: Options) -> Self {
        Replayer {
            opts,
            game: None,
            type_lists: Vec::new(),
            lines_ok: 0,
            at: None,
            during: "header".into(),
            detail: None,
            progress: None,
            csharp_hung: None,
            bots_active: false,
            bots: Vec::new(),
            queried: Vec::new(),
            deliberation: None,
            bot_decisions_ok: 0,
        }
    }

    fn game(&mut self) -> &mut Game {
        self.game.as_mut().expect("header read")
    }

    fn run(&mut self, reader: Box<dyn BufRead>) -> Result<(), Failure> {
        let mut saw_end = false;
        for (i, text) in reader.lines().enumerate() {
            let text = text.map_err(|e| Failure::BadTrace(format!("read error: {e}")))?;
            if text.is_empty() {
                continue;
            }
            let line: Line = serde_json::from_str(&text).map_err(|e| Failure::BadTrace(format!("line {}: {e}", i + 1)))?;
            if i == 0 {
                if line.kind != "header" {
                    return Err(Failure::BadTrace("first line is not a header".into()));
                }
                self.header(&line)?;
                continue;
            }
            self.at = Some(line.n.map(|n| n.to_string()).unwrap_or_else(|| line.kind.clone()));
            if let Some(p) = &self.progress {
                let _ = p.send(Progress::Line(self.at.clone().unwrap(), self.bot_decisions_ok));
            }
            if let (Some(stop), Some(n)) = (self.opts.stop_after, line.n) {
                if n > stop {
                    return Ok(());
                }
            }
            self.line(&line, &text)?;
            self.lines_ok += 1;
            if line.kind == "end" {
                saw_end = true;
                break;
            }
        }
        if !saw_end && self.opts.stop_after.is_none() {
            return Err(Failure::BadTrace("no end line (the oracle was cut short)".into()));
        }
        Ok(())
    }

    fn header(&mut self, line: &Line) -> Result<(), Failure> {
        // Format 2 only adds the `bots` query entry.
        if !matches!(line.trace_format, Some(1) | Some(2)) || line.dump_format != Some(1) {
            return Err(Failure::BadTrace(format!(
                "unsupported traceFormat {:?} / dumpFormat {:?}",
                line.trace_format, line.dump_format
            )));
        }
        let version = line.game_version.ok_or_else(|| Failure::BadTrace("header has no gameVersion".into()))?;
        if version != Game::LATEST_VERSION {
            return Err(Failure::BadTrace(format!(
                "game version {version}: the engine supports only version {} (record with the oracle's --versions latest)",
                Game::LATEST_VERSION
            )));
        }
        let participation = participation(line.participation.as_ref())?;
        self.bots_active = self.opts.check_bots && bots_checkable(line);
        self.during = "new Game(version, participation)".into();
        self.game = Some(Game::new_int32_participation(version, &treachery_engine::compat::cs::Shared::new(participation)));
        Ok(())
    }

    fn line(&mut self, line: &Line, text: &str) -> Result<(), Failure> {
        // The oracle's GetApplicableEvents and GameStatus.DetermineStatus calls since the previous line.
        self.during = format!("the queries of line {}", self.at.as_deref().unwrap_or("?"));
        self.queried.clear();
        self.deliberation = None;
        for (qi, q) in line.queries.iter().enumerate() {
            match q {
                Query::Applicable { player, is_host, exception } => self.query(line, text, qi, *player, *is_host, exception.as_deref())?,
                Query::Status { player, is_player, result } => self.status(line, text, qi, *player, *is_player, result)?,
                Query::Bots(entry) => self.bots_entry(line, text, qi, entry)?,
            }
            self.during = format!("the queries of line {}", self.at.as_deref().unwrap_or("?"));
        }
        // Bots draw only while deliberating, so the line's after-state `botRandom` is already final here.
        let bot_random_mismatch = if self.bots_active && line.kind != "end" { self.compare_bot_random(line) } else { None };

        let mut produced = Produced::default();
        let mut event_type = None;
        match line.kind.as_str() {
            "step" => {
                let ev = self.step(line, &mut produced)?;
                event_type = Some(ev.event_type().name().to_string());
            }
            "undo" | "restoreRecentlyUndone" | "dismissRecentlyUndone" => {
                self.during = line.kind.clone();
                let until = line.until_event_nr;
                let game = self.game();
                match line.kind.as_str() {
                    "undo" => game.undo(until.ok_or_else(|| Failure::BadTrace("undo without untilEventNr".into()))?),
                    "restoreRecentlyUndone" => game.restore_recently_undone(),
                    _ => game.dismiss_recently_undone(),
                }
            }
            "end" if matches!(line.reason.as_deref(), Some("hung") | Some("memoryLimit")) => {
                // C# never returned from this event. Rust must not return either;
                // the caller's deadline decides (see `replay_file`).
                let event_type = line.event.as_ref().and_then(|e| event_type_of(e.get()));
                let reason = line.reason.clone().unwrap_or_default();
                self.csharp_hung = Some(reason.clone());
                if let Some(p) = &self.progress {
                    let _ = p.send(Progress::Hung(reason));
                }
                let mut produced = Produced::default();
                self.step(line, &mut produced)?;
                return Err(Failure::Mismatch(Mismatch {
                    n: "end".into(),
                    kind: "end".into(),
                    event_type,
                    key: "termination".into(),
                    expected: "the event never returns (C# hung)".into(),
                    actual: format!("Rust returned: result {}", produced.result),
                    state_diff: Vec::new(),
                    no_state: false,
                }));
            }
            "end" if line.reason.as_deref() == Some("undoHung") => {
                // C# never returned from this undo call. Same rule as for `hung`.
                let kind = line.undo_kind.clone().ok_or_else(|| Failure::BadTrace("undoHung without undoKind".into()))?;
                self.csharp_hung = Some("undoHung".into());
                if let Some(p) = &self.progress {
                    let _ = p.send(Progress::Hung("undoHung".into()));
                }
                self.during = kind.clone();
                let until = line.until_event_nr;
                let game = self.game();
                match kind.as_str() {
                    "undo" => game.undo(until.ok_or_else(|| Failure::BadTrace("undoHung undo without untilEventNr".into()))?),
                    "restoreRecentlyUndone" => game.restore_recently_undone(),
                    "dismissRecentlyUndone" => game.dismiss_recently_undone(),
                    other => return Err(Failure::BadTrace(format!("unknown undoKind {other}"))),
                }
                return Err(Failure::Mismatch(Mismatch {
                    n: "end".into(),
                    kind: "end".into(),
                    event_type: None,
                    key: "termination".into(),
                    expected: format!("{kind} never returns (C# hung)"),
                    actual: format!("Rust returned from {kind}"),
                    state_diff: Vec::new(),
                    no_state: false,
                }));
            }
            "end" if line.hash.is_none() => {
                // `botHung` and similar: nothing to compare.
                if line.reason.as_deref() == Some("botHung") {
                    self.csharp_hung = Some("botHung".into());
                }
                return Ok(());
            }
            "end" => {
                self.during = "the final dump".into();
                let (hash, dump) = self.dump();
                return self.compare_hash(line, text, None, hash, dump);
            }
            other => return Err(Failure::BadTrace(format!("unknown line kind {other}"))),
        }

        // After-state keys.
        self.during = "the after-state of the line".into();
        let game = self.game.as_ref().unwrap();
        produced.history_count = Some(game.history.len() as i64);
        produced.random = game.random.as_ref().map(|r| r.get_count() as i64);

        let mut mismatches: Vec<(String, String, String)> = bot_random_mismatch.into_iter().collect();
        let mut check = |key: &str, expected: String, actual: String| {
            if expected != actual {
                mismatches.push((key.to_string(), expected, actual));
            }
        };
        if line.kind == "step" {
            check("result", line.result.clone().unwrap_or_default(), produced.result.clone());
            // An event that failed partway fails on both sides. C# records the exception's type and
            // message; the engine reports its own GameError, so the texts aren't compared.
            let both_failed = line.result.as_deref() == Some("error") && produced.result == "error";
            if !both_failed {
                check("validation", raw_text(&line.validation), produced.validation.clone());
                check("exception", raw_text(&line.exception), produced.exception.clone());
            }
            let expected_after = line.event_after.as_ref().or(line.event.as_ref()).map(|r| r.get().to_string()).unwrap_or_default();
            check("eventAfter", expected_after, produced.event_after.clone());
        }
        check("historyCount", opt(line.history_count), opt(produced.history_count));
        check("random", opt(line.random), opt(produced.random));

        // GetApplicableEvents in the oracle's order; this can change state.
        for (k, list) in &line.type_lists {
            let k = *k as usize;
            if self.type_lists.len() != k {
                return Err(Failure::BadTrace(format!("typeLists index {k} out of order")));
            }
            self.type_lists.push(list.get().to_string());
        }
        let actual_applicable = self.applicable();
        let expected_applicable: Vec<String> = line
            .applicable
            .iter()
            .flat_map(|pair| [pair.0, pair.1])
            .map(|i| self.type_lists.get(i as usize).cloned().unwrap_or_else(|| format!("<undefined typeLists entry {i}>")))
            .collect();
        if expected_applicable != actual_applicable {
            let (who, e, a) = first_applicable_difference(&expected_applicable, &actual_applicable);
            mismatches.push((format!("applicable"), format!("{who}: {e}"), format!("{who}: {a}")));
        }

        let mut hash_failure = None;
        if line.hash.is_some() {
            let (hash, dump) = self.dump();
            if let Err(f) = self.compare_hash(line, text, event_type.clone(), hash, dump) {
                hash_failure = Some(f);
            }
        }

        if mismatches.is_empty() && hash_failure.is_none() {
            return Ok(());
        }
        if self.opts.detail {
            let mut d = self.detail.take().unwrap_or_default();
            d.line_summary = summarize_line(text);
            d.all_mismatches = mismatches.clone();
            if let Some(Failure::Mismatch(m)) = &hash_failure {
                d.all_mismatches.push((m.key.clone(), m.expected.clone(), m.actual.clone()));
            }
            self.detail = Some(d);
        }
        if let Some((key, expected, actual)) = mismatches.into_iter().next() {
            let event_type = if key == "botRandom" { self.deliberation.clone().or(Some("no deliberation on this line".into())) } else { event_type };
            return Err(Failure::Mismatch(Mismatch {
                n: self.at.clone().unwrap_or_default(),
                kind: line.kind.clone(),
                event_type,
                key,
                expected,
                actual,
                state_diff: Vec::new(),
                no_state: false,
            }));
        }
        Err(hash_failure.unwrap())
    }

    /// Executes a step line's event the way `StepExecutor.Run` does.
    fn step(&mut self, line: &Line, produced: &mut Produced) -> Result<GameEvent, Failure> {
        let event_text = line.event.as_ref().ok_or_else(|| Failure::BadTrace("step without event".into()))?.get();
        self.during = "reading the event JSON".into();
        let ev: GameEvent = match treachery_engine::json::deserialize(event_text) {
            Ok(ev) => ev,
            Err(e) => {
                return Err(Failure::Mismatch(Mismatch {
                    n: self.at.clone().unwrap_or_default(),
                    kind: "step".into(),
                    event_type: event_type_of(event_text),
                    key: "event".into(),
                    expected: "the C# reader accepts the event".into(),
                    actual: format!("{}: {}", e.kind, e.message),
                    state_diff: Vec::new(),
                    no_state: false,
                }))
            }
        };
        let validate = line.validate.unwrap_or(true);
        let is_host = line.is_host.unwrap_or(false);
        let overrides = overrides_execute(ev.event_type());

        self.during = format!("{} execution", ev.event_type().name());
        let game = self.game.as_mut().unwrap();
        ev.initialize_game(game);
        game.map.mark_neighbour_cache_stale();
        let history_before = game.history.len();
        if !overrides {
            game.recent_milestones.push(SENTINEL);
        }

        if overrides {
            panics::capture_backtraces(true);
        }
        let result = catch(|| ev.execute(game, validate, is_host));
        panics::capture_backtraces(false);
        let escaped_bt = panics::take_last().and_then(|p| p.backtrace);

        let mut execution_entered = true;
        if !overrides {
            match game.recent_milestones.iter().rposition(|m| *m == SENTINEL) {
                Some(i) => {
                    execution_entered = false;
                    game.recent_milestones.remove(i);
                }
                None => execution_entered = true,
            }
        }

        let (message, escaped) = match result {
            Ok(m) => (m, None),
            Err(e) => (None, Some(e)),
        };
        let error: Option<CsException> = escaped.clone().or_else(|| message.as_ref().and_then(find_exception));
        let phase = error.as_ref().map(|_| {
            if escaped.is_some() || overrides {
                stack_phase(escaped_bt.as_deref())
            } else if execution_entered {
                "execute"
            } else {
                "validate"
            }
        });

        let accepted = game.history.len() > history_before;
        produced.result = if accepted { "accepted" } else if error.is_some() { "error" } else { "rejected" }.to_string();
        produced.validation = match &message {
            None => "null".into(),
            Some(m) => String::from_utf8(canon::dump_detached(m as &dyn Canon, Some(game))).unwrap(),
        };
        produced.exception = match &error {
            None => "null".into(),
            Some(e) => exception_json(phase.unwrap(), escaped.is_some(), e),
        };
        self.during = "serializing the event after execution".into();
        produced.event_after = match catch(|| treachery_engine::json::serialize(&ev)) {
            Ok(s) => s,
            Err(_) => event_text.to_string(),
        };
        Ok(ev)
    }

    /// A `[player, isHost]` query: `GetApplicableEvents`, whose result is unused but which can change
    /// state. When C# threw, the entry has the exception as a third element, and Rust must throw the
    /// same; when it has none, Rust must not throw.
    fn query(&mut self, line: &Line, text: &str, index: usize, player: Option<usize>, is_host: bool, expected: Option<&RawValue>) -> Result<(), Failure> {
        let game = self.game.as_mut().expect("header read");
        let p = player.map(|i| game.player_at(i));
        let actual = match catch(std::panic::AssertUnwindSafe(|| game.get_applicable_events(p, is_host))) {
            Ok(types) => {
                if let Some(i) = player {
                    if self.queried.len() <= i {
                        self.queried.resize(i + 1, None);
                    }
                    self.queried[i] = Some(types);
                }
                None
            }
            Err(e) => Some(exception_dump(&e)),
        };
        let expected = expected.map(|e| e.get());
        if actual.as_deref() == expected {
            return Ok(());
        }
        // Traces written before query exceptions were recorded: a `botException` end line's last
        // query is the one the bot threw from, with the exception only in `detail`.
        if expected.is_none() && line.kind == "end" && line.reason.as_deref() == Some("botException") && index + 1 == line.queries.len() {
            if let Some(a) = &actual {
                let v: Value = serde_json::from_str(a).unwrap_or(Value::Null);
                let text = format!("{}: {}", v["type"].as_str().unwrap_or(""), v["message"].as_str().unwrap_or(""));
                if line.detail.as_deref() == Some(text.as_str()) {
                    return Ok(());
                }
            }
        }
        let who = player.map(|i| format!("player {i}")).unwrap_or_else(|| "player null".into());
        let render = |x: Option<&str>| x.map(str::to_string).unwrap_or_else(|| "no exception".into());
        if self.opts.detail {
            let mut d = self.detail.take().unwrap_or_default();
            d.line_summary = summarize_line(text);
            d.all_mismatches = vec![(format!("GetApplicableEvents({who}, {is_host}), query {index}"), render(expected), render(actual.as_deref()))];
            self.detail = Some(d);
        }
        Err(Failure::Mismatch(Mismatch {
            n: self.at.clone().unwrap_or_default(),
            kind: line.kind.clone(),
            event_type: None,
            key: "query".into(),
            expected: format!("GetApplicableEvents({who}, {is_host}): {}", render(expected)),
            actual: format!("GetApplicableEvents({who}, {is_host}): {}", render(actual.as_deref())),
            state_diff: Vec::new(),
            no_state: false,
        }))
    }

    /// A `["status", player, isPlayer, result]` query: `GameStatus.DetermineStatus`, which can
    /// change state, compared with the recorded result on the spot.
    fn status(&mut self, line: &Line, text: &str, index: usize, player: Option<usize>, is_player: bool, expected: &RawValue) -> Result<(), Failure> {
        self.during = format!("GameStatus.DetermineStatus (query {index} of line {})", self.at.as_deref().unwrap_or("?"));
        let game = self.game.as_mut().expect("header read");
        let p = player.map(|i| game.player_at(i));
        let actual = match catch(|| GameStatus::determine_status(game, p, is_player)) {
            Ok(status) => canon::dump_detached(&status as &dyn Canon, Some(game)),
            Err(e) => exception_dump(&e).into_bytes(),
        };
        if actual == expected.get().as_bytes() {
            return Ok(());
        }
        let who = player.map(|i| format!("player {i}")).unwrap_or_else(|| "player null".into());
        let e: Value = serde_json::from_str(expected.get()).map_err(|e| Failure::BadTrace(format!("status: {e}")))?;
        let a: Value = serde_json::from_slice(&actual).unwrap_or(Value::Null);
        let actual_text = String::from_utf8_lossy(&actual).into_owned();
        if self.opts.detail {
            let mut d = self.detail.take().unwrap_or_default();
            d.line_summary = summarize_line(text);
            d.all_mismatches = vec![(format!("status of {who}"), expected.get().to_string(), actual_text.clone())];
            self.detail = Some(d);
        }
        Err(Failure::Mismatch(Mismatch {
            n: self.at.clone().unwrap_or_default(),
            kind: line.kind.clone(),
            event_type: None,
            key: "status".into(),
            expected: format!("{who}: {}", expected.get()),
            actual: format!("{who}: {actual_text}"),
            state_diff: diff::diff(&e, &a, self.opts.max_diffs),
            no_state: false,
        }))
    }

    /// A `bots` query entry: the oracle's `AskBots` at this point. With bot checking on, creates the
    /// bots it lists, runs the Rust bots on the live game (levels in order, players in `order`, the
    /// first event wins), and compares the outcome with the recorded choice or exception. A panic that
    /// isn't a C# exception (a `todo!` in an unported bot method) propagates as "not ported yet".
    fn bots_entry(&mut self, line: &Line, text: &str, index: usize, entry: &BotsEntry) -> Result<(), Failure> {
        if !self.bots_active {
            return Ok(());
        }
        self.during = format!("bot deliberation (query {index} of line {})", self.at.as_deref().unwrap_or("?"));
        let game = self.game.as_mut().expect("header read");
        for (label, player, seed) in &entry.created {
            let faction = parse_faction(label).ok_or_else(|| Failure::BadTrace(format!("bots entry: bad faction {label}")))?;
            if *player >= game.players.len() {
                return Err(Failure::BadTrace(format!("bots entry: no player {player}")));
            }
            let p = game.player_at(*player);
            let bot = ClassicBot::new(game, p, BotParameters::get_default_parameters(faction), Some(SharedRandom::from_seed(*seed)));
            self.bots.push(RegisteredBot { faction, label: label.clone(), bot });
        }

        // The bots run on the live game, not a copy: they can change it (bots-plan.md, "Bots are not pure"),
        // and they share its neighbour cache with the queries before and the event after.
        let levels = if entry.end { &BOT_LEVELS[..] } else { &BOT_LEVELS[..4] };
        let bots = &mut self.bots;
        let queried = &self.queried;
        let result = catch(AssertUnwindSafe(|| -> Result<Option<(usize, &'static str, GameEvent)>, String> {
            for &level in levels {
                for &i in &entry.order {
                    if i >= game.players.len() {
                        return Err(format!("harness: the bots entry asks player {i}, who doesn't exist in Rust"));
                    }
                    let faction = game.players[i].faction;
                    let Some(b) = bots.iter_mut().find(|b| b.faction == faction) else {
                        return Err(format!("Rust went on to ask the bot of player {i} ({faction}), which C# never asked at this point"));
                    };
                    let Some(events) = queried.get(i).and_then(|e| e.as_deref()) else {
                        return Err(format!("harness: no GetApplicableEvents result for player {i} before the bots entry"));
                    };
                    let p = game.player_at(i);
                    IBot::set_game_and_player(&mut b.bot, game, p);
                    let chosen = match level {
                        "Highest" => b.bot.determine_highest_priority_in_phase_action(game, events),
                        "High" => b.bot.determine_high_priority_in_phase_action(game, events),
                        "Middle" => b.bot.determine_middle_priority_in_phase_action(game, events),
                        "Low" => b.bot.determine_low_priority_in_phase_action(game, events),
                        _ => b.bot.determine_end_phase_action(game, events),
                    };
                    if let Some(ev) = chosen {
                        return Ok(Some((i, level, ev)));
                    }
                }
            }
            Ok(None)
        }));

        // Both sides rendered the same way: "player P at LEVEL: {event json}", "no event", or the exception.
        let expected = if let Some(e) = &entry.exception {
            format!("exception {}", e.get())
        } else if let Some(c) = &entry.choice {
            format!("player {} at {}: {}", c.player, c.priority, c.event.get())
        } else {
            "no event".to_string()
        };
        let mut rust_label = None;
        let actual = match &result {
            Ok(Ok(Some((i, level, ev)))) => {
                rust_label = Some(format!("{} at {level}", ev.event_type().name()));
                let json = match catch(AssertUnwindSafe(|| treachery_engine::json::serialize(ev))) {
                    Ok(s) => s,
                    Err(e) => format!("{{\"$serializationError\":\"{}\"}}", cs::full_type_name(e.type_name)),
                };
                format!("player {i} at {level}: {json}")
            }
            Ok(Ok(None)) => "no event".to_string(),
            Ok(Err(message)) => message.clone(),
            Err(e) => format!("exception {}", exception_dump(e)),
        };
        // What C# decided. Together with the level it names the Determine* method the difference is in.
        let label = if let Some(e) = &entry.exception {
            let v: Value = serde_json::from_str(e.get()).unwrap_or(Value::Null);
            format!("C# threw {}", v["type"].as_str().unwrap_or("?"))
        } else if let Some(c) = &entry.choice {
            format!("C# {} at {}", event_type_of(c.event.get()).unwrap_or_else(|| "?".into()), c.priority)
        } else {
            format!("C# no event, Rust {}", rust_label.as_deref().unwrap_or("?"))
        };
        self.deliberation = Some(label.clone());
        if expected == actual {
            self.bot_decisions_ok += 1;
            return Ok(());
        }
        if self.opts.detail {
            let mut d = self.detail.take().unwrap_or_default();
            d.line_summary = summarize_line(text);
            d.all_mismatches = vec![(format!("bot deliberation (query {index})"), expected.clone(), actual.clone())];
            self.detail = Some(d);
        }
        Err(Failure::Mismatch(Mismatch {
            n: self.at.clone().unwrap_or_default(),
            kind: line.kind.clone(),
            event_type: Some(label),
            key: "bot".into(),
            expected,
            actual,
            state_diff: Vec::new(),
            no_state: false,
        }))
    }

    /// The line's `botRandom` against the registry's draw counts, as `(key, expected, actual)` if they differ.
    fn compare_bot_random(&self, line: &Line) -> Option<(String, String, String)> {
        let expected: Vec<String> = line.bot_random.iter().flatten().map(|(f, c)| format!("{f}: {c}")).collect();
        let actual: Vec<String> = self.bots.iter().map(|b| format!("{}: {}", b.label, b.bot.random.get_count())).collect();
        (expected != actual).then(|| ("botRandom".to_string(), format!("[{}]", expected.join(", ")), format!("[{}]", actual.join(", "))))
    }

    /// `GetApplicableEvents` for every player and null, non-host then host,
    /// rendered as in the trace's `typeLists`.
    fn applicable(&mut self) -> Vec<String> {
        let game = self.game.as_mut().unwrap();
        let mut out = Vec::new();
        let count = game.players.len();
        for i in 0..=count {
            let p = if i < count { Some(game.player_at(i)) } else { None };
            for host in [false, true] {
                let r = catch(|| game.get_applicable_events(p, host));
                out.push(match r {
                    Ok(types) => {
                        let mut w = CanonWriter::new(None);
                        w.arr_start();
                        for (j, t) in types.iter().enumerate() {
                            if j > 0 {
                                w.comma();
                            }
                            w.string(t.name());
                        }
                        w.arr_end();
                        String::from_utf8(w.out).unwrap()
                    }
                    Err(e) => exception_dump(&e),
                });
            }
        }
        out
    }

    fn dump(&mut self) -> (String, Vec<u8>) {
        let dump = canon::dump_game(self.game.as_ref().unwrap());
        (crate::sha256_hex(&dump), dump)
    }

    fn compare_hash(&mut self, line: &Line, text: &str, event_type: Option<String>, hash: String, dump: Vec<u8>) -> Result<(), Failure> {
        let expected = line.hash.clone().unwrap_or_default();
        if expected == hash {
            return Ok(());
        }
        let mut m = Mismatch {
            n: self.at.clone().unwrap_or_default(),
            kind: line.kind.clone(),
            event_type,
            key: "hash".into(),
            expected,
            actual: hash,
            state_diff: Vec::new(),
            no_state: line.state.is_none(),
        };
        if let Some(state) = &line.state {
            let e: Value = serde_json::from_str(state.get()).map_err(|e| Failure::BadTrace(format!("state: {e}")))?;
            let a: Value = match serde_json::from_slice(&dump) {
                Ok(v) => v,
                Err(err) => {
                    m.actual = format!("{} (the Rust dump is not valid JSON: {err})", m.actual);
                    Value::Null
                }
            };
            m.state_diff = diff::diff(&e, &a, self.opts.max_diffs);
            if self.opts.detail {
                let mut d = self.detail.take().unwrap_or_default();
                d.expected_state = Some(state.get().to_string());
                d.actual_state = Some(String::from_utf8_lossy(&dump).into_owned());
                if d.line_summary.is_empty() {
                    d.line_summary = summarize_line(text);
                }
                self.detail = Some(d);
            }
        } else if self.opts.detail {
            let mut d = self.detail.take().unwrap_or_default();
            d.actual_state = Some(String::from_utf8_lossy(&dump).into_owned());
            d.line_summary = summarize_line(text);
            self.detail = Some(d);
        }
        Err(Failure::Mismatch(m))
    }
}

#[derive(Default)]
struct Produced {
    result: String,
    validation: String,
    exception: String,
    event_after: String,
    history_count: Option<i64>,
    random: Option<i64>,
}

fn opt(v: Option<i64>) -> String {
    v.map(|v| v.to_string()).unwrap_or_else(|| "null".into())
}

fn raw_text(v: &Option<Box<RawValue>>) -> String {
    v.as_ref().map(|r| r.get().to_string()).unwrap_or_else(|| "null".into())
}

fn event_type_of(event_json: &str) -> Option<String> {
    let v: Value = serde_json::from_str(event_json).ok()?;
    v.get("$type")?.as_str().map(|s| s.to_string())
}

/// Runs `f`; a panic whose message names a .NET exception (`"NullReferenceException: ..."`, as the
/// engine's .NET helpers panic) becomes that exception, which the oracle's traces record. Any other
/// panic continues.
fn catch<R>(f: impl FnOnce() -> R) -> Result<R, CsException> {
    match panic::catch_unwind(AssertUnwindSafe(f)) {
        Ok(r) => Ok(r),
        Err(payload) => match as_exception(payload.as_ref()) {
            Some(e) => Err(e),
            None => panic::resume_unwind(payload),
        },
    }
}

/// The .NET exception a panic payload names.
pub fn as_exception(payload: &(dyn std::any::Any + Send)) -> Option<CsException> {
    cs::exception_of(payload)
}

/// The first exception element of a "Game Error" message (`StepExecutor.FindException`).
fn find_exception(m: &Message) -> Option<CsException> {
    m.expression.elements.iter().find_map(|e| match e {
        ExpressionElement::Error(ex) => Some(CsException { type_name: "Exception", message: ex.message().to_string() }),
        _ => None,
    })
}

/// `StepExecutor.StackPhase`: the first event method on the stack decides.
fn stack_phase(backtrace: Option<&str>) -> &'static str {
    let Some(bt) = backtrace else { return "execute" };
    for frame in bt.lines() {
        let f = frame.trim();
        if !f.contains("treachery_engine::events::") {
            continue;
        }
        if f.contains("::validate") || f.contains("::is_applicable") {
            return "validate";
        }
        if f.contains("::execute_concrete_event") {
            return "execute";
        }
    }
    "execute"
}

/// An exception as the oracle dumps it: `{"$type":"$Exception","message":...,"type":<full name>}`.
fn exception_dump(e: &CsException) -> String {
    let mut w = CanonWriter::new(None);
    w.obj_start();
    w.key("$type");
    w.string("$Exception");
    w.comma();
    w.key("message");
    w.string(&e.message);
    w.comma();
    w.key("type");
    w.string(&cs::full_type_name(e.type_name));
    w.obj_end();
    String::from_utf8(w.out).unwrap()
}

fn exception_json(phase: &str, escaped: bool, e: &CsException) -> String {
    let mut w = CanonWriter::new(None);
    w.obj_start();
    w.key("phase");
    w.string(phase);
    w.comma();
    w.key("escaped");
    w.bool(escaped);
    w.comma();
    w.key("type");
    w.string(&cs::full_type_name(e.type_name));
    w.comma();
    w.key("message");
    w.string(&e.message);
    w.obj_end();
    String::from_utf8(w.out).unwrap()
}

fn first_applicable_difference(expected: &[String], actual: &[String]) -> (String, String, String) {
    let players = expected.len().max(actual.len()) / 2;
    for i in 0..expected.len().max(actual.len()) {
        let e = expected.get(i).cloned().unwrap_or_else(|| "<missing>".into());
        let a = actual.get(i).cloned().unwrap_or_else(|| "<missing>".into());
        if e != a {
            let who = if i / 2 + 1 == players { "player null".to_string() } else { format!("player {}", i / 2) };
            let host = if i % 2 == 1 { "host" } else { "non-host" };
            return (format!("{who} ({host})"), e, a);
        }
    }
    ("?".into(), String::new(), String::new())
}

/// The trace line without its (large) `state`.
fn summarize_line(text: &str) -> String {
    match serde_json::from_str::<serde_json::Map<String, Value>>(text) {
        Ok(mut m) => {
            if m.contains_key("state") {
                m.insert("state".into(), Value::String("<omitted>".into()));
            }
            serde_json::to_string(&m).unwrap_or_default()
        }
        Err(_) => text.chars().take(2000).collect(),
    }
}

/// Builds the `Participation` of the header, in the dumped enumeration order.
pub fn participation(v: Option<&Value>) -> Result<Participation, Failure> {
    let bad = |what: &str| Failure::BadTrace(format!("header participation: {what}"));
    let o = v.and_then(|v| v.as_object()).ok_or_else(|| bad("missing"))?;
    let mut p = Participation::default();
    p.bots_speed = o.get("BotsSpeed").and_then(|v| v.as_i64()).ok_or_else(|| bad("BotsSpeed"))? as i32;
    let arr = |k: &str| o.get(k).and_then(|v| v.as_array()).cloned().ok_or_else(|| bad(k));
    for kv in arr("PlayerNames")? {
        let k = kv[0].as_i64().ok_or_else(|| bad("PlayerNames key"))? as i32;
        let v = kv[1].as_str().ok_or_else(|| bad("PlayerNames value"))?.to_string();
        p.player_names.add(k, v);
    }
    for kv in arr("SeatedPlayers")? {
        let k = kv[0].as_i64().ok_or_else(|| bad("SeatedPlayers key"))? as i32;
        let v = kv[1].as_i64().ok_or_else(|| bad("SeatedPlayers value"))? as i32;
        p.seated_players.add(k, v);
    }
    for (key, set) in [
        ("Observers", &mut p.observers),
        ("Hosts", &mut p.hosts),
        ("AvailableSeats", &mut p.available_seats),
        ("Kicked", &mut p.kicked),
    ] {
        for v in arr(key)? {
            set.add(v.as_i64().ok_or_else(|| bad(key))? as i32);
        }
    }
    Ok(p)
}
