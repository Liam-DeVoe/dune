//! The replayer: makes the calls of a reference trace on the working tree's engine, view and bots,
//! and compares what players would observe with what the trace recorded.

use std::io::BufRead;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

use crate::allow::Allow;
use crate::session::Session;
use crate::trace::{self, Line, Obs};
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

fn context(line: &Line) -> String {
    match line {
        Line::Event { event, src, .. } => format!("event {} ({src})", event.get("$type").and_then(|t| t.as_str()).unwrap_or("?")),
        Line::Query { p, host, .. } => format!("query (player {}, host {host})", p.map_or("none".to_string(), |p| p.to_string())),
        other => other.op().to_string(),
    }
}

fn show<T: serde::Serialize>(v: &T) -> String {
    serde_json::to_string(v).unwrap_or_default()
}

impl<'a> Replayer<'a> {
    fn fail(&self, line: &Line, key: &str, expected: String, actual: String) -> Mismatch {
        Mismatch { line: self.n, key: key.into(), context: context(line), expected, actual, viewer: None, view_diff: vec![] }
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
                if result.panic {
                    return Err(self.fail(line, "panic", "a panic".into(), show(&actual)));
                }
                // An exception's text is not compared (native code reports these differently).
                let same = actual.types == result.types && actual.error.is_some() == result.error.is_some();
                if !same {
                    return Err(self.fail(line, "applicable", show(result), show(&actual)));
                }
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
                if result.panic {
                    return Err(self.fail(line, "panic", "a panic".into(), show(&actual)));
                }
                if self.allow.bots {
                    return Ok(None);
                }
                // The level a bot decided at is internal; the player and the event are what others see.
                let key = |r: &trace::BotsResult| (r.choice.as_ref().map(|c| (c.p, c.event.clone())), r.error.is_some(), r.panic);
                if key(&actual) != key(result) {
                    return Err(self.fail(line, "bot", show(result), show(&actual)));
                }
                Ok(None)
            }
            Line::Event { host, event, outcome, .. } => {
                let actual = match self.sess().submit(&event.to_string(), *host) {
                    Ok(o) => o,
                    Err(e) if outcome.result == "panic" => return Ok(self.expected_panic(e.0)),
                    Err(e) => return Err(panicked_actual(self, e.0)),
                };
                if outcome.result == "panic" {
                    return Err(self.fail(line, "panic", "a panic".into(), format!("{} {}", actual.result, actual.message.clone().unwrap_or_default())));
                }
                let class = |r: &str| if r == "rejected" || r == "error" { "not accepted".to_string() } else { r.to_string() };
                if class(&actual.result) != class(&outcome.result) {
                    return Err(self.fail(line, "result", format!("{} {}", outcome.result, outcome.message.clone().unwrap_or_default()), format!("{} {}", actual.result, actual.message.clone().unwrap_or_default())));
                }
                // Messages of engine exceptions ("Game Error: ...") are not compared: native code
                // reports these failures differently, and that is not a change players rely on.
                if actual.result == "rejected" && outcome.result == "rejected" && !self.allow.same_message(outcome.message.as_deref(), actual.message.as_deref()) {
                    return Err(self.fail(line, "message", outcome.message.clone().unwrap_or_default(), actual.message.clone().unwrap_or_default()));
                }
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
        if expected.log_cut != actual.log_cut {
            return Err(self.fail(line, "log", format!("log cut to {:?}", expected.log_cut), format!("log cut to {:?}", actual.log_cut)));
        }
        let e: Vec<&(String, String)> = expected.log.iter().filter(|m| !self.allow.ignores_log(&m.1)).collect();
        let a: Vec<&(String, String)> = actual.log.iter().filter(|m| !self.allow.ignores_log(&m.1)).collect();
        if e != a {
            let i = e.iter().zip(a.iter()).position(|(x, y)| x != y).unwrap_or(e.len().min(a.len()));
            let at = |v: &Vec<&(String, String)>| v.get(i).map_or("(no message)".to_string(), |(t, m)| if t.is_empty() { m.clone() } else { format!("[to {t}] {m}") });
            return Err(self.fail(line, "log", format!("message {}: {}", i + 1, at(&e)), format!("message {}: {}", i + 1, at(&a))));
        }
        if expected.replica.is_some() != actual.replica.is_some() {
            return Err(self.fail(line, "replica", show(&expected.replica), show(&actual.replica)));
        }
        match (&expected.views, &actual.views) {
            (None, None) => Ok(()),
            (Some(ev), Some(av)) => {
                let labels: Vec<String> = self.sess().viewers().into_iter().map(|(l, _)| l).collect();
                if ev.len() != av.len() {
                    return Err(self.fail(line, "views", format!("{} viewers", ev.len()), format!("{} viewers ({})", av.len(), labels.join(", "))));
                }
                for i in 0..ev.len() {
                    // A view that failed to build is compared as failed, not by its error text.
                    if ev[i] == av[i] || (ev[i].starts_with("error: ") && av[i].starts_with("error: ")) {
                        continue;
                    }
                    let label = labels.get(i).cloned().unwrap_or_default();
                    let mut m = self.fail(line, "views", ev[i].clone(), av[i].clone());
                    m.viewer = Some((i, label));
                    if let (Some(ef), Some(af)) = (&expected.full, &actual.full) {
                        let (ef, af) = (&ef[i], &af[i]);
                        m.view_diff = crate::jsondiff::diff(ef, af, &|path| self.allow.ignores_view(path), 30);
                        if m.view_diff.is_empty() {
                            // Every difference is allowed.
                            continue;
                        }
                    } else if self.allow.has_view_rules() {
                        m.key = "views (hash)".into();
                    }
                    return Err(m);
                }
                Ok(())
            }
            (e, a) => Err(self.fail(line, "views", if e.is_some() { "views built".into() } else { "no views (history unchanged)".into() }, if a.is_some() { "views built".into() } else { "no views (history unchanged)".into() })),
        }
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
            r.sess = Some(Session::new(h.version, &h.seated, h.full_views));
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
