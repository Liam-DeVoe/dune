//! `differ`: replays C# oracle traces through the Rust engine. See rust/docs/differ.md.

use std::path::PathBuf;
use std::process::ExitCode;

use differ::live::{self, LiveOptions};
use differ::replay::{self, Options};
use differ::report;

const USAGE: &str = "\
differ: replays C# oracle traces through the Rust engine and reports the first difference.

Usage:
  differ run PATH... [--jobs J] [--max-diffs N] [-v]
      Replay trace files or directories of traces. Prints a summary that groups
      failures by reason. With a single file, prints the full report instead.
  differ first FILE [--step N] [--max-diffs N] [--dump-dir DIR]
      Replay one trace and describe its first difference in detail: the trace
      line, every value that differs on it, and the state fields that differ.
      --step N stops after line N. --dump-dir writes csharp.json and rust.json
      (the two state dumps of the failing line).
  differ live --mode bots|fuzz --seed S --games N [--start I] [--jobs J]
              [--batch B] [--oracle DLL] [--keep-failures DIR] [--status every|none] [-v]
      Run the C# oracle and replay its traces as they are produced, deleting
      each one after it is checked. --status every records GameStatus after
      every line and compares it (trace-format.md section 3); it is the default
      for fuzz runs, --status none turns it off. Games that hung in C# are
      replayed up to the hang and listed in the summary.

Every command also takes --hang-seconds S (default 10): how long the event of
a `hung` end line must run in Rust to count as hanging too, and --step-timeout S
(default 120): how long any other line may take before Rust counts as stuck,
and --memory-mb M (default 2048): how much memory one replay may hold before it
is stopped and reported as a memory limit failure (0 = no limit), and
--bots check|skip (default skip): with check, bots-mode traces also run the Rust
bots at every recorded bot deliberation and compare the chosen event and the bot
draw counts (bots-plan.md); the summary then counts bot differences per method.

Defaults: --jobs = number of CPUs (live: at most 4), --max-diffs 10 (first: 40), --batch 4 * jobs.
live waits for one of the machine-wide slots of rust/tools/heavy.sh (PORTING.md, CPU)
unless it already runs inside heavy.sh.
Exit code: 0 if every trace replayed without a difference, 1 otherwise, 2 on usage errors.
";

struct Args {
    positional: Vec<String>,
    flags: Vec<(String, Option<String>)>,
}

impl Args {
    fn parse(raw: &[String]) -> Args {
        let mut positional = Vec::new();
        let mut flags = Vec::new();
        let mut i = 0;
        while i < raw.len() {
            let a = &raw[i];
            if a == "-v" || a == "--verbose" {
                flags.push(("v".to_string(), None));
            } else if let Some(k) = a.strip_prefix("--") {
                let v = raw.get(i + 1).cloned();
                flags.push((k.to_string(), v));
                i += 1;
            } else {
                positional.push(a.clone());
            }
            i += 1;
        }
        Args { positional, flags }
    }

    fn get(&self, k: &str) -> Option<&str> {
        self.flags.iter().rev().find(|(f, _)| f == k).and_then(|(_, v)| v.as_deref())
    }

    fn has(&self, k: &str) -> bool {
        self.flags.iter().any(|(f, _)| f == k)
    }

    fn num<T: std::str::FromStr>(&self, k: &str, default: T) -> Result<T, String> {
        match self.get(k) {
            None => Ok(default),
            Some(v) => v.parse().map_err(|_| format!("--{k}: not a number: {v}")),
        }
    }
}

/// `--hang-seconds` and `--step-timeout` (seconds).
fn timeouts(args: &Args) -> Result<Options, String> {
    let d = Options::default();
    Ok(Options {
        hang_budget: std::time::Duration::from_secs(args.num("hang-seconds", d.hang_budget.as_secs())?),
        step_timeout: std::time::Duration::from_secs(args.num("step-timeout", d.step_timeout.as_secs())?),
        memory_limit: args.num("memory-mb", d.memory_limit >> 20)? << 20,
        check_bots: match args.get("bots") {
            None | Some("skip") => false,
            Some("check") => true,
            Some(other) => return Err(format!("--bots must be check or skip, not {other}")),
        },
        ..d
    })
}

fn cpus() -> usize {
    std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4)
}

fn main() -> ExitCode {
    let raw: Vec<String> = std::env::args().skip(1).collect();
    if raw.is_empty() || raw[0] == "-h" || raw[0] == "--help" {
        eprint!("{USAGE}");
        return if raw.is_empty() { ExitCode::from(2) } else { ExitCode::SUCCESS };
    }
    let args = Args::parse(&raw[1..]);
    let result = match raw[0].as_str() {
        "run" => cmd_run(&args),
        "first" => cmd_first(&args),
        "live" => cmd_live(&args),
        other => Err(format!("unknown command {other}\n\n{USAGE}")),
    };
    match result {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(1),
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(2)
        }
    }
}

fn cmd_run(args: &Args) -> Result<bool, String> {
    if args.positional.is_empty() {
        return Err("run: give trace files or directories".into());
    }
    let files: Vec<PathBuf> = args.positional.iter().flat_map(|p| differ::trace::list(&PathBuf::from(p))).collect();
    if files.is_empty() {
        return Err("run: no .jsonl or .jsonl.gz files found".into());
    }
    let opts = Options { max_diffs: args.num("max-diffs", 10)?, ..timeouts(args)? };
    if files.len() == 1 {
        let opts = Options { detail: true, ..opts };
        let (outcome, detail) = replay::replay_file(&files[0], &opts);
        print!("{}", report::describe(&outcome, detail.as_ref()));
        return Ok(outcome.passed());
    }
    let verbose = args.has("v");
    let jobs = args.num("jobs", cpus())?;
    let results = report::run_all(&files, jobs, &opts, &|r| {
        if verbose {
            eprintln!("{}", report::one_line(r));
        }
    });
    print!("{}", report::summary(&results));
    Ok(results.iter().all(|r| r.outcome.passed()))
}

fn cmd_first(args: &Args) -> Result<bool, String> {
    let file = args.positional.first().ok_or("first: give a trace file")?;
    let opts = Options {
        max_diffs: args.num("max-diffs", 40)?,
        stop_after: args.get("step").map(|s| s.parse::<i64>().map_err(|_| format!("--step: not a number: {s}"))).transpose()?,
        detail: true,
        ..timeouts(args)?
    };
    let (outcome, detail) = replay::replay_file(&PathBuf::from(file), &opts);
    print!("{}", report::describe(&outcome, detail.as_ref()));
    if let (Some(dir), Some(d)) = (args.get("dump-dir"), detail.as_ref()) {
        let dir = PathBuf::from(dir);
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        if let Some(s) = &d.expected_state {
            std::fs::write(dir.join("csharp.json"), s).map_err(|e| e.to_string())?;
        }
        if let Some(s) = &d.actual_state {
            std::fs::write(dir.join("rust.json"), s).map_err(|e| e.to_string())?;
        }
        println!("\nwrote the state dumps to {}", dir.display());
    }
    Ok(outcome.passed())
}

fn cmd_live(args: &Args) -> Result<bool, String> {
    // PORTING.md, "CPU": at most 4 jobs, and one of the machine-wide heavy slots.
    let jobs = args.num("jobs", cpus().min(4))?;
    let o = LiveOptions {
        mode: args.get("mode").ok_or("live: --mode bots|fuzz is required")?.to_string(),
        seed: args.num("seed", 0i64).and_then(|s| if args.has("seed") { Ok(s) } else { Err("live: --seed is required".into()) })?,
        start: args.num("start", 0)?,
        games: args.num("games", 0i64).and_then(|g| if g > 0 { Ok(g) } else { Err("live: --games N (N > 0) is required".into()) })?,
        jobs,
        batch: args.num("batch", 4 * jobs as i64)?,
        oracle: args.get("oracle").map(PathBuf::from).unwrap_or_else(live::default_oracle),
        keep_failures: args.get("keep-failures").map(PathBuf::from),
        verbose: args.has("v"),
        replay: timeouts(args)?,
        status: match args.get("status") {
            // Fuzz runs compare GameStatus by default; bots runs keep the traces of the bots sample.
            None => args.get("mode") == Some("fuzz"),
            Some("none") => false,
            Some("every") => true,
            Some(other) => return Err(format!("live: --status must be every or none, not {other}")),
        },
    };
    if o.mode != "bots" && o.mode != "fuzz" {
        return Err("live: --mode must be bots or fuzz".into());
    }
    let _slot = live::acquire_heavy_slot();
    let results = live::run(&o)?;
    print!("{}", report::summary(&results));
    if let Some(r) = results.iter().find(|r| !r.outcome.passed()) {
        let name = r.path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let index = name.split('.').next().and_then(|s| s.rsplit('-').next()).and_then(|s| s.parse().ok());
        if let Some(i) = index {
            println!("\nTo reproduce the first failing game ({name}):\n  {}", live::reproduce_command(&o, i));
        }
    }
    Ok(results.iter().all(|r| r.outcome.passed()))
}
