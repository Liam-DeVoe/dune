//! The replayer: makes the calls of a reference trace on the working tree's engine, view and bots,
//! and compares what players would observe with what the trace recorded.

use std::io::BufRead;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

use serde_json::Value;

use crate::allow::Allow;
use crate::compare::{self, show, Diff};
use crate::events;
use crate::session::Session;
use crate::trace::{self, EventOutcome, Line, Obs};
use crate::watch::{self, Budgets, Ending, Msg};

/// The first difference of a game.
#[derive(Clone, Debug)]
pub struct Mismatch {
    /// 1-based line number in the trace.
    pub line: usize,
    /// What differs: `result`, `message`, `log`, `views`, `bot`, `applicable`, `panic`, `hang`, ...
    pub key: String,
    /// One line about the call: op and event type.
    pub context: String,
    pub expected: String,
    pub actual: String,
    /// For `views`: index and label of the first viewer that differs.
    pub viewer: Option<(usize, String)>,
    /// For `views`, when both sides have full views: the differing paths.
    pub view_diff: Vec<String>,
}

impl Mismatch {
    /// A difference at trace line `n` (1-based).
    pub fn at(n: usize, line: &Line, d: Diff) -> Mismatch {
        Mismatch { line: n, key: d.key, context: context(line), expected: d.expected, actual: d.actual, viewer: d.viewer, view_diff: d.view_diff }
    }
}

#[derive(Clone, Debug)]
pub enum Outcome {
    Pass { lines: usize, end: String },
    Fail(Mismatch),
    /// The trace can't be read or doesn't match this replayer's format.
    Bad(String),
}

impl Outcome {
    pub fn passed(&self) -> bool {
        matches!(self, Outcome::Pass { .. })
    }

    /// The group a failure is counted under in a summary.
    pub fn group(&self) -> String {
        match self {
            Outcome::Pass { .. } => "pass".into(),
            Outcome::Bad(m) => format!("bad trace: {}", m.lines().next().unwrap_or("")),
            Outcome::Fail(m) => format!("{} differs at {}", m.key, m.context),
        }
    }
}

struct Replayer<'a> {
    sess: Option<Session>,
    allow: &'a Allow,
    n: usize,
    tx: &'a Sender<Msg>,
}

/// One line about a call, for reports.
pub fn context(line: &Line) -> String {
    match line {
        Line::Event { event, src, .. } => format!("event {} ({src})", events::type_name(event)),
        Line::Query { p, host, .. } => format!("query (player {}, host {host})", p.map_or("none".to_string(), |p| p.to_string())),
        other => other.op().to_string(),
    }
}

impl<'a> Replayer<'a> {
    fn fail(&self, line: &Line, key: &str, expected: String, actual: String) -> Mismatch {
        self.at(line, Diff::new(key, expected, actual))
    }

    fn at(&self, line: &Line, d: Diff) -> Mismatch {
        Mismatch::at(self.n, line, d)
    }

    fn sess(&mut self) -> &mut Session {
        self.sess.as_mut().expect("header read")
    }

    /// Runs one line. `Ok(Some(end))` at the end line.
    fn line(&mut self, line: &Line) -> Result<Option<String>, Mismatch> {
        let _ = self.tx.send(Msg::Begin(line.without_outcome()));
        let r = self.call(line);
        let _ = self.tx.send(Msg::Line(Line::Restore { outcome: Default::default() }));
        r
    }

    fn call(&mut self, line: &Line) -> Result<Option<String>, Mismatch> {
        let panicked_actual = |s: &Self, msg: String| s.fail(line, "panic", "no panic".into(), format!("the working tree panicked: {msg}"));
        match line {
            Line::Header(_) => Err(self.fail(line, "header", "one header".into(), "a second header".into())),
            Line::Query { p, host, result } => {
                let count = self.sess().player_count();
                if p.is_some_and(|p| p >= count) {
                    return Err(self.fail(line, "players", format!("a player {}", p.unwrap()), format!("{count} players")));
                }
                let actual = match self.sess().query(*p, *host) {
                    Ok((r, _)) => r,
                    Err(e) if result.panic => return Ok(self.expected_panic(e.0)),
                    Err(e) => return Err(panicked_actual(self, e.0)),
                };
                compare::query(result, &actual).map_err(|d| self.at(line, d))?;
                Ok(None)
            }
            Line::Bots { order, end, created, result } => {
                let mut seeds = created.clone();
                let mut seed_for = |f| {
                    let name = crate::session::faction_name(f);
                    let i = seeds.iter().position(|(n, _)| *n == name)?;
                    Some(seeds.remove(i).1)
                };
                let actual = match self.sess().deliberate(order, *end, &mut seed_for) {
                    Ok((r, made)) => {
                        if made.len() != created.len() {
                            return Err(self.fail(line, "bot", format!("bots created: {}", show(created)), format!("bots created: {}", show(&made))));
                        }
                        r
                    }
                    Err(e) if result.panic => return Ok(self.expected_panic(e.0)),
                    Err(e) => return Err(panicked_actual(self, e.0)),
                };
                // Each side's decision is decoded and encoded again by the working tree (events.rs).
                let normalize = |v: &Value, reference: bool| if reference { events::normalize_reference(v) } else { events::normalize_working(v) };
                compare::bots(result, &actual, &normalize, self.allow).map_err(|d| self.at(line, d))?;
                Ok(None)
            }
            Line::Event { host, event, outcome, .. } => {
                // The working tree decodes the reference's event itself. An event it can't decode
                // is `unreadable`, which compares like a rejection.
                let actual = match events::from_reference(event) {
                    Ok(ev) => self.sess().submit_event(ev, *host),
                    Err(e) => Ok(EventOutcome { result: "unreadable".into(), message: Some(e), obs: Obs::default() }),
                };
                let actual = match actual {
                    Ok(o) => o,
                    Err(e) if outcome.result == "panic" => return Ok(self.expected_panic(e.0)),
                    Err(e) => return Err(panicked_actual(self, e.0)),
                };
                compare::event_result(outcome, &actual, self.allow).map_err(|d| self.at(line, d))?;
                self.compare_obs(line, &outcome.obs, &actual.obs)?;
                Ok(None)
            }
            Line::Undo { outcome, .. } | Line::Restore { outcome } | Line::Dismiss { outcome } => {
                let until = if let Line::Undo { until, .. } = line { *until } else { 0 };
                let actual = match self.sess().special(line.op(), until) {
                    Ok(o) => o,
                    Err(e) if outcome.panic => return Ok(self.expected_panic(e.0)),
                    Err(e) => return Err(panicked_actual(self, e.0)),
                };
                if outcome.panic {
                    return Err(self.fail(line, "panic", "a panic".into(), "no panic".into()));
                }
                if actual.error.is_some() != outcome.error.is_some() {
                    return Err(self.fail(line, "result", show(&outcome.error), show(&actual.error)));
                }
                self.compare_obs(line, &outcome.obs, &actual.obs)?;
                Ok(None)
            }
            Line::End { reason, pending, .. } => {
                if let Some(p) = pending {
                    // The reference never returned from this call. Neither must the working tree:
                    // the watcher sees the call still running. That agrees with the reference, but
                    // `replay` still fails the game as a runaway.
                    let _ = self.tx.send(Msg::Text(format!("expect-hang {reason}")));
                    let _ = self.tx.send(Msg::Begin((**p).clone()));
                    let r = self.call(p);
                    return Err(match r {
                        Err(m) if m.key == "panic" => m,
                        _ => self.fail(line, "hang", format!("the reference didn't return ({reason})"), "the working tree returned".into()),
                    });
                }
                Ok(Some(reason.clone()))
            }
        }
    }

    /// The reference panicked here too: the game ends at this line on both sides.
    fn expected_panic(&self, _msg: String) -> Option<String> {
        Some("panic".into())
    }

    fn compare_obs(&mut self, line: &Line, expected: &Obs, actual: &Obs) -> Result<(), Mismatch> {
        let labels: Vec<String> = self.sess().viewers().into_iter().map(|(l, _)| l).collect();
        compare::obs(expected, actual, self.allow, &labels).map_err(|d| self.at(line, d))
    }
}

/// Replays a trace read from `reader`.
pub fn replay(reader: Box<dyn BufRead + Send>, allow: Arc<Allow>, budgets: &Budgets) -> Outcome {
    let result: Arc<Mutex<Option<Outcome>>> = Arc::new(Mutex::new(None));
    let out = result.clone();
    let mut expect_hang: Option<String> = None;
    let mut last_line = 0usize;
    let ending = watch::run(
        budgets,
        move |tx| {
            crate::session::quiet_panics();
            let r = replay_on_thread(reader, &allow, tx);
            *out.lock().unwrap() = Some(r);
        },
        |m| match m {
            Msg::Text(t) => {
                if let Some(r) = t.strip_prefix("expect-hang ") {
                    expect_hang = Some(r.to_string());
                }
            }
            Msg::Line(_) => last_line += 1,
            Msg::Begin(_) => {}
        },
    );
    match ending {
        Ending::Finished => result.lock().unwrap().take().unwrap_or_else(|| Outcome::Bad("the replay thread stopped without a result".into())),
        // The reference ran away at the same call. That agrees, but no real game may end like this
        // (on the server it freezes every game), so it fails the game (escapes.md, "Automatic bid
        // passes that never stop").
        Ending::Hung(pending) | Ending::MemoryLimit(pending) if expect_hang.is_some() => Outcome::Fail(Mismatch {
            line: last_line + 1,
            key: "runaway".into(),
            context: pending.as_ref().map(context).unwrap_or_default(),
            expected: "the call returns".into(),
            actual: format!("{} in both the reference and the working tree", expect_hang.unwrap()),
            viewer: None,
            view_diff: vec![],
        }),
        Ending::Hung(pending) => Outcome::Fail(Mismatch {
            line: last_line + 1,
            key: "hang".into(),
            context: pending.as_ref().map(context).unwrap_or_default(),
            expected: "the call returns".into(),
            actual: format!("still running after {} s", budgets.hang.as_secs()),
            viewer: None,
            view_diff: vec![],
        }),
        Ending::MemoryLimit(pending) => Outcome::Fail(Mismatch {
            line: last_line + 1,
            key: "memory".into(),
            context: pending.as_ref().map(context).unwrap_or_default(),
            expected: "the call returns".into(),
            actual: format!("over the memory budget of {} MB", budgets.memory >> 20),
            viewer: None,
            view_diff: vec![],
        }),
    }
}

fn replay_on_thread(reader: Box<dyn BufRead + Send>, allow: &Allow, tx: &Sender<Msg>) -> Outcome {
    let mut r = Replayer { sess: None, allow, n: 0, tx };
    for text in reader.lines() {
        r.n += 1;
        let text = match text {
            Ok(t) => t,
            Err(e) => return Outcome::Bad(format!("line {}: {e}", r.n)),
        };
        if text.trim().is_empty() {
            continue;
        }
        let line: Line = match serde_json::from_str(&text) {
            Ok(l) => l,
            Err(e) => return Outcome::Bad(format!("line {}: {e}", r.n)),
        };
        if let Line::Header(h) = &line {
            if r.sess.is_some() {
                return Outcome::Bad("a second header".into());
            }
            if h.format != trace::FORMAT {
                return Outcome::Bad(format!("trace format {} (this replayer reads {})", h.format, trace::FORMAT));
            }
            let mut sess = Session::new(h.version, &h.seated, h.full_views);
            sess.view_hash = h.view_hash;
            r.sess = Some(sess);
            continue;
        }
        if r.sess.is_none() {
            return Outcome::Bad("no header".into());
        }
        match r.line(&line) {
            Ok(None) => {}
            Ok(Some(end)) => return Outcome::Pass { lines: r.n, end },
            Err(m) => return Outcome::Fail(m),
        }
    }
    Outcome::Bad("no end line (the recording was cut short)".into())
}

/// Reads a trace file (`.jsonl` or `.jsonl.gz`).
pub fn open(path: &std::path::Path) -> std::io::Result<Box<dyn BufRead + Send>> {
    let f = std::fs::File::open(path)?;
    if path.extension().is_some_and(|e| e == "gz") {
        Ok(Box::new(std::io::BufReader::new(flate2::read::MultiGzDecoder::new(f))))
    } else {
        Ok(Box::new(std::io::BufReader::new(f)))
    }
}
