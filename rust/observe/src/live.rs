//! `observe live` and `observe first`: the reference binary records each game, the working tree
//! replays it at once. Traces stay in memory; nothing is written to disk unless asked.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::allow::Allow;
use crate::replay::{self, Outcome};
use crate::watch::Budgets;

/// Where `rust/tools/build-native-ref.sh` puts the reference binary.
pub fn default_reference() -> PathBuf {
    if let Some(p) = std::env::var_os("OBSERVE_REFERENCE") {
        return PathBuf::from(p);
    }
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("reference").join("observe")
}

#[derive(Clone)]
pub struct Game {
    pub mode: String,
    pub seed: u64,
    pub index: u64,
}

impl Game {
    pub fn name(&self) -> String {
        format!("{}-{}-{:05}", self.mode, self.seed, self.index)
    }
}

pub struct GameResult {
    pub game: Game,
    pub outcome: Outcome,
    /// The trace, when the game failed (recorded with full views).
    pub trace: Option<Vec<u8>>,
    pub seconds: f64,
}

/// Runs the reference recorder for one game and returns its trace.
pub fn record_with_reference(reference: &Path, g: &Game, full_views: bool, budgets: &Budgets) -> Result<Vec<u8>, String> {
    let mut c = Command::new(reference);
    c.args(["record", "--mode", &g.mode, "--seed", &g.seed.to_string(), "--game", &g.index.to_string()])
        .args(["--hang-seconds", &budgets.hang.as_secs().to_string(), "--memory-mb", &(budgets.memory >> 20).to_string()]);
    if full_views {
        c.arg("--full-views");
    }
    let out = c.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).output().map_err(|e| format!("cannot run the reference {}: {e}", reference.display()))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        return Err(format!("the reference recorder failed ({}): {}", out.status, err.lines().last().unwrap_or("")));
    }
    Ok(out.stdout)
}

fn replay_bytes(trace: Vec<u8>, allow: &Arc<Allow>, budgets: &Budgets) -> Outcome {
    replay::replay(Box::new(std::io::Cursor::new(trace)), allow.clone(), budgets)
}

/// Records and replays one game. A failure is recorded again with full views, so the report can
/// show which view fields differ and view rules of the allow file can apply.
pub fn check_game(reference: &Path, g: &Game, allow: &Arc<Allow>, budgets: &Budgets) -> GameResult {
    let start = Instant::now();
    let done = |outcome, trace| GameResult { game: g.clone(), outcome, trace, seconds: start.elapsed().as_secs_f64() };
    let trace = match record_with_reference(reference, g, false, budgets) {
        Ok(t) => t,
        Err(e) => return done(Outcome::Bad(e), None),
    };
    let outcome = replay_bytes(trace, allow, budgets);
    if outcome.passed() {
        return done(outcome, None);
    }
    match record_with_reference(reference, g, true, budgets) {
        Ok(full) => {
            let detailed = replay_bytes(full.clone(), allow, budgets);
            match detailed {
                Outcome::Pass { .. } if !matches!(outcome, Outcome::Fail(ref m) if m.key.starts_with("views")) => {
                    done(Outcome::Bad(format!("failed without full views but passed with them: {}", outcome.group())), Some(full))
                }
                d => done(d, Some(full)),
            }
        }
        Err(_) => done(outcome, None),
    }
}

pub struct LiveOptions {
    pub mode: String,
    pub seed: u64,
    pub start: u64,
    pub games: u64,
    pub jobs: usize,
    pub reference: PathBuf,
    pub allow: Arc<Allow>,
    pub budgets: Budgets,
    pub keep_failures: Option<PathBuf>,
    pub verbose: bool,
}

pub fn one_line(r: &GameResult) -> String {
    match &r.outcome {
        Outcome::Pass { lines, end } => format!("PASS {} ({lines} lines, {end}, {:.1} s)", r.game.name(), r.seconds),
        o => format!("FAIL {}: {}", r.game.name(), o.group()),
    }
}

pub fn run(o: &LiveOptions) -> Vec<GameResult> {
    let next = AtomicUsize::new(0);
    let results = Mutex::new(Vec::new());
    std::thread::scope(|s| {
        for _ in 0..o.jobs.max(1) {
            s.spawn(|| loop {
                let k = next.fetch_add(1, Ordering::SeqCst) as u64;
                if k >= o.games {
                    break;
                }
                let g = Game { mode: o.mode.clone(), seed: o.seed, index: o.start + k };
                let r = check_game(&o.reference, &g, &o.allow, &o.budgets);
                if o.verbose || !r.outcome.passed() {
                    eprintln!("{}", one_line(&r));
                }
                if let (Some(dir), Some(t)) = (&o.keep_failures, &r.trace) {
                    let _ = std::fs::create_dir_all(dir);
                    let _ = std::fs::write(dir.join(format!("{}.jsonl", g.name())), t);
                }
                results.lock().unwrap().push(r);
            });
        }
    });
    let mut v = results.into_inner().unwrap();
    v.sort_by_key(|r| r.game.index);
    v
}
