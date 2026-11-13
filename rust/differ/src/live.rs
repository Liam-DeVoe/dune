//! `differ live`: runs the C# oracle and the Rust replayer together, so a
//! large corpus never has to exist on disk.
//!
//! Games are generated in batches into a temporary directory. While the
//! oracle writes batch k+1, the replayer checks batch k and deletes each
//! trace as soon as it is checked. At most two batches are on disk at once.
//!
//! A batch is checked whatever the oracle's exit status. A game that hung in
//! C# (exit status 4) has a trace ending in `hung`, `undoHung` or `botHung`,
//! which is replayed and reported as hung; the other games of the batch are
//! compared as usual. A game with no trace file is reported as a bad trace.

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use crate::replay::{self, Failure, Options};
use crate::report::{self, TraceResult};

pub struct LiveOptions {
    /// `bots` or `fuzz`.
    pub mode: String,
    pub seed: i64,
    pub start: i64,
    pub games: i64,
    pub jobs: usize,
    /// Games per oracle run (default `4 * jobs`).
    pub batch: i64,
    /// Path of `Treachery.Oracle.dll`, run with `dotnet`. Any other path is run directly with the
    /// same arguments (the tests use a script that stands in for the oracle).
    pub oracle: PathBuf,
    /// Copy failing traces here (regenerated with `--dump every`).
    pub keep_failures: Option<PathBuf>,
    /// Print one line per trace.
    pub verbose: bool,
    /// Replay options (deadlines).
    pub replay: Options,
    /// Pass `--status every` to the oracle: record and compare `GameStatus` after every line.
    pub status: bool,
}

/// A machine-wide slot for a CPU-heavy run, shared with `rust/tools/heavy.sh` (PORTING.md, "CPU").
/// The slot is the directory `/tmp/treachery-heavy-slots/slotN` holding the owner's pid; it is
/// removed when the guard is dropped, and a slot whose owner has died is reclaimed.
pub struct HeavySlot(Option<PathBuf>);

impl Drop for HeavySlot {
    fn drop(&mut self) {
        if let Some(dir) = &self.0 {
            let _ = std::fs::remove_dir_all(dir);
        }
    }
}

/// Waits for a free slot. Returns at once without one when the caller already holds a slot
/// (`heavy.sh` sets `TREACHERY_HEAVY_SLOT`).
pub fn acquire_heavy_slot() -> HeavySlot {
    if std::env::var_os("TREACHERY_HEAVY_SLOT").is_some() {
        return HeavySlot(None);
    }
    let slots: usize = std::env::var("HEAVY_SLOTS").ok().and_then(|s| s.parse().ok()).unwrap_or(2);
    let root = PathBuf::from("/tmp/treachery-heavy-slots");
    let _ = std::fs::create_dir_all(&root);
    let mut announced = false;
    loop {
        for i in 1..=slots {
            let slot = root.join(format!("slot{i}"));
            if std::fs::create_dir(&slot).is_ok() {
                let _ = std::fs::write(slot.join("pid"), std::process::id().to_string());
                eprintln!("differ live: running in heavy slot {i}");
                return HeavySlot(Some(slot));
            }
            let pid = std::fs::read_to_string(slot.join("pid")).ok().and_then(|p| p.trim().parse::<u32>().ok());
            let alive = pid.map_or(true, |p| Command::new("kill").args(["-0", &p.to_string()]).stderr(Stdio::null()).status().is_ok_and(|s| s.success()));
            if !alive {
                let _ = std::fs::remove_dir_all(&slot);
            }
        }
        if !announced {
            eprintln!("differ live: waiting for a free heavy slot (rust/tools/heavy.sh, PORTING.md \"CPU\")");
            announced = true;
        }
        std::thread::sleep(std::time::Duration::from_secs(10));
    }
}

/// The repository root (`rust/..`).
pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

pub fn default_oracle() -> PathBuf {
    repo_root().join("treachery-online/Oracle/bin/Release/net10.0/Treachery.Oracle.dll")
}

/// `dotnet`, preferring `$HOME/dotnet10` (where .NET 10 lives on the dev machine).
fn dotnet() -> Command {
    let home10 = std::env::var_os("HOME").map(|h| PathBuf::from(h).join("dotnet10"));
    match home10.filter(|p| p.join("dotnet").exists()) {
        Some(dir) => {
            let mut c = Command::new(dir.join("dotnet"));
            if std::env::var_os("DOTNET_ROOT").is_none() {
                c.env("DOTNET_ROOT", &dir);
            }
            c
        }
        None => Command::new("dotnet"),
    }
}

fn oracle_command(o: &LiveOptions, start: i64, games: i64, out: &Path, dump: &str, threads: usize) -> Command {
    let mut c = if o.oracle.extension().is_some_and(|e| e == "dll") {
        let mut c = dotnet();
        c.arg(&o.oracle);
        c
    } else {
        Command::new(&o.oracle)
    };
    c.arg(&o.mode)
        .args(["--seed", &o.seed.to_string()])
        .args(["--start", &start.to_string()])
        .args(["--games", &games.to_string()])
        .arg("--out")
        .arg(out)
        .args(["--dump", dump, "--hash", "every", "--gzip", "true", "--threads", &threads.to_string()])
        // The engine supports only Game::LATEST_VERSION.
        .args(["--versions", "latest"]);
    if o.status {
        c.args(["--status", "every"]);
    }
    c.stdout(Stdio::null())
        .stderr(Stdio::piped());
    c
}

/// The shell command that regenerates one game with full dumps.
pub fn reproduce_command(o: &LiveOptions, index: i64) -> String {
    format!(
        "dotnet {} {} --seed {} --start {} --games 1 --dump every{} --gzip true --versions latest --out /tmp/differ-repro && \\\n  cargo run -p differ --release -- first /tmp/differ-repro/{}-{}-{:05}.jsonl.gz{}",
        o.oracle.display(),
        o.mode,
        o.seed,
        index,
        if o.status { " --status every" } else { "" },
        o.mode,
        o.seed,
        index,
        if o.replay.check_bots { " --bots check" } else { "" }
    )
}

/// Waits for an oracle run. Returns `Ok(None)` on success and `Ok(Some(problem))` when it exited
/// with an error: the traces it did write are still usable. Exit code 3 means some game crashed the
/// oracle and 4 that some game hung (trace-format.md 5); those games' traces say so themselves.
fn wait(child: Child, what: &str) -> Result<Option<String>, String> {
    let out = child.wait_with_output().map_err(|e| format!("{what}: {e}"))?;
    if out.status.success() {
        return Ok(None);
    }
    let err = String::from_utf8_lossy(&out.stderr);
    let tail: Vec<&str> = err.lines().rev().take(15).collect::<Vec<_>>().into_iter().rev().collect();
    Ok(Some(match out.status.code() {
        Some(3) => format!("{what}: an oracle game crashed (exit 3)"),
        Some(4) => format!("{what}: an oracle game hung (exit 4)"),
        Some(5) => format!("{what}: an oracle game went over the oracle's memory limit (exit 5)"),
        _ => format!("{what} failed ({}):\n{}", out.status, tail.join("\n")),
    }))
}

/// Runs the whole live session and returns the per-trace results.
pub fn run(o: &LiveOptions) -> Result<Vec<TraceResult>, String> {
    if !o.oracle.exists() {
        return Err(format!(
            "oracle not found at {}. Build it with:\n  dotnet build -c Release treachery-online/Oracle/Treachery.Oracle.csproj",
            o.oracle.display()
        ));
    }
    let root = std::env::temp_dir().join(format!("differ-live-{}", std::process::id()));
    std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    let result = run_in(o, &root);
    let _ = std::fs::remove_dir_all(&root);
    result
}

fn run_in(o: &LiveOptions, root: &Path) -> Result<Vec<TraceResult>, String> {
    let batch = o.batch.max(1);
    let mut starts = Vec::new();
    let mut s = o.start;
    while s < o.start + o.games {
        starts.push((s, batch.min(o.start + o.games - s)));
        s += batch;
    }

    let spawn = |k: usize| -> Result<(Child, PathBuf), String> {
        let (start, games) = starts[k];
        let dir = root.join(format!("batch-{k}"));
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let child = oracle_command(o, start, games, &dir, "final", o.jobs).spawn().map_err(|e| format!("cannot start dotnet: {e}"))?;
        Ok((child, dir))
    };

    let opts = o.replay.clone();
    let mut all = Vec::new();
    let mut pending = if starts.is_empty() { None } else { Some(spawn(0)?) };
    for k in 0..starts.len() {
        let (child, dir) = pending.take().unwrap();
        let problem = wait(child, &format!("oracle batch {k}"))?;
        let files = crate::trace::list(&dir);
        if let Some(p) = &problem {
            // With no trace at all, the oracle didn't get to run a game (bad build, bad arguments).
            if files.is_empty() {
                return Err(p.clone());
            }
            eprintln!("warning: {p}");
        }
        if k + 1 < starts.len() {
            pending = Some(spawn(k + 1)?);
        }
        let done = |r: &TraceResult| {
            if o.verbose {
                eprintln!("{}", report::one_line(r));
            }
            let _ = std::fs::remove_file(&r.path);
        };
        let mut results = report::run_all(&files, o.jobs, &opts, &done);
        results.extend(missing_games(o, &dir, starts[k], &files, problem.as_deref()));
        // An oracle that crashed or stopped for memory takes the games still running with it.
        // Rerun those one at a time, each in its own oracle process.
        if problem.as_deref().is_some_and(|p| !p.contains("(exit 3)") && !p.contains("(exit 4)")) {
            for r in &mut results {
                if lost(r) {
                    rerun_lost(o, root, r);
                }
            }
        }
        for r in &mut results {
            if r.outcome.failure.is_some() {
                rerun_failure(o, root, r);
            }
        }
        let failed = results.iter().filter(|r| !r.outcome.passed()).count();
        eprintln!(
            "batch {}/{}: games {}..{}, {} failed",
            k + 1,
            starts.len(),
            starts[k].0,
            starts[k].0 + starts[k].1 - 1,
            failed
        );
        let _ = std::fs::remove_dir_all(&dir);
        all.extend(results);
    }
    Ok(all)
}

/// A bad-trace result for each game of the batch the oracle wrote no trace file for.
fn missing_games(o: &LiveOptions, dir: &Path, (start, games): (i64, i64), files: &[PathBuf], problem: Option<&str>) -> Vec<TraceResult> {
    let have: std::collections::BTreeSet<i64> = files.iter().filter_map(|f| index_of(f)).collect();
    let why = problem.map(|p| format!(" ({})", p.lines().next().unwrap_or(p))).unwrap_or_default();
    (start..start + games)
        .filter(|i| !have.contains(i))
        .map(|i| TraceResult {
            path: dir.join(format!("{}-{}-{:05}.jsonl.gz", o.mode, o.seed, i)),
            outcome: replay::Outcome::bad_trace(format!("the oracle wrote no trace for this game{why}")),
        })
        .collect()
}

/// A trace the oracle didn't finish writing, or didn't write at all.
fn lost(r: &TraceResult) -> bool {
    matches!(&r.outcome.failure, Some(Failure::BadTrace(m)) if m.contains("no end line") || m.contains("read error") || m.contains("wrote no trace"))
}

/// Regenerates one lost game alone and replays it in place of the lost result.
fn rerun_lost(o: &LiveOptions, root: &Path, r: &mut TraceResult) {
    let Some(index) = index_of(&r.path) else { return };
    let dir = root.join(format!("lost-{index}"));
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    let ok = oracle_command(o, index, 1, &dir, "final", 1).spawn().map_err(|e| e.to_string()).and_then(|c| wait(c, "oracle rerun"));
    if let Ok(problem) = ok {
        if let Some(file) = crate::trace::list(&dir).into_iter().next() {
            let (outcome, _) = replay::replay_file(&file, &o.replay);
            eprintln!("rerun of lost game {index}: {}", match &outcome.failure { None => "ok".to_string(), Some(f) => f.group() });
            r.outcome = outcome;
            r.path = file;
        } else if let Some(p) = problem {
            r.outcome = replay::Outcome::bad_trace(format!("the oracle wrote no trace for this game, also when run alone ({})", p.lines().next().unwrap_or("")));
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// Game index from `<mode>-<seed>-<index>.jsonl.gz`.
fn index_of(path: &Path) -> Option<i64> {
    let name = path.file_name()?.to_str()?;
    let stem = name.split('.').next()?;
    stem.rsplit('-').next()?.parse().ok()
}

/// For a hash mismatch on a line without a dump, regenerates the game with a
/// dump on every line so the report can show which fields differ. Copies the
/// failing trace to `--keep-failures` if given.
fn rerun_failure(o: &LiveOptions, root: &Path, r: &mut TraceResult) {
    let Some(index) = index_of(&r.path) else { return };
    let needs_state = matches!(&r.outcome.failure, Some(Failure::Mismatch(m)) if m.no_state);
    if !needs_state && o.keep_failures.is_none() {
        return;
    }
    let dir = root.join(format!("rerun-{index}"));
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    let ok = oracle_command(o, index, 1, &dir, "every", 1).spawn().map_err(|e| e.to_string()).and_then(|c| wait(c, "oracle rerun"));
    // A rerun of a hung game exits with 4 as well; its trace is complete up to the hang.
    if ok.is_ok() {
        if let Some(file) = crate::trace::list(&dir).into_iter().next() {
            if needs_state {
                let (outcome, _) = replay::replay_file(&file, &o.replay);
                if outcome.failure.is_some() {
                    r.outcome = outcome;
                }
            }
            if let Some(keep) = &o.keep_failures {
                let _ = std::fs::create_dir_all(keep);
                let _ = std::fs::copy(&file, keep.join(file.file_name().unwrap()));
            }
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
}
