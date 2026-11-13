//! `observe`: the observable-behavior harness. See rust/docs/native-harness.md.

use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use std::sync::Arc;

use observe::allow::Allow;
use observe::live::{self, Game, LiveOptions};
use observe::record::{self, RecordOptions};
use observe::replay;
use observe::report;
use observe::watch::Budgets;

const USAGE: &str = "\
observe: records games with the reference build and replays them through the working tree.

Usage:
  observe record --mode bots|fuzz --seed S --game I [--full-views] [--out FILE]
      Record one game and write its trace (JSON Lines) to FILE, or to stdout. This is the
      command the reference binary runs.
  observe live --mode bots|fuzz --seed S --games N [--start I] [--jobs J]
               [--reference BIN] [--allow FILE] [--keep-failures DIR] [-v]
      Record games with the reference binary and replay each one through the working tree as it
      arrives. Nothing is written to disk unless --keep-failures is given. Prints a summary and,
      for the first failing game, the command that shows its difference in detail.
  observe first --mode bots|fuzz --seed S --game I [--reference BIN] [--allow FILE]
  observe first FILE [--allow FILE]
      Replay one game (recorded now by the reference, with full views, or read from a trace
      file) and describe its first difference in detail, with a diff of the view that differs.
  observe replay FILE... [--allow FILE] [-v]
      Replay trace files (.jsonl or .jsonl.gz).

Common options: --hang-seconds S (default 120): a call that runs longer counts as hung;
--memory-mb M (default 2048): the memory one game may use (0 = no limit).
--reference defaults to $OBSERVE_REFERENCE, else rust/reference/observe, else ~/treachery-native-ref/observe.
--jobs defaults to 4 at most. live waits for a heavy slot (rust/tools/heavy.sh) unless it already
runs inside heavy.sh.
Exit code: 0 if every game matched, 1 otherwise, 2 on usage errors.
";

pub struct Args {
    positional: Vec<String>,
    flags: Vec<(String, Option<String>)>,
}

const SWITCHES: [&str; 3] = ["v", "full-views", "verbose"];

impl Args {
    fn parse(raw: &[String]) -> Args {
        let mut positional = Vec::new();
        let mut flags = Vec::new();
        let mut i = 0;
        while i < raw.len() {
            let a = &raw[i];
            if a == "-v" {
                flags.push(("v".to_string(), None));
            } else if let Some(k) = a.strip_prefix("--") {
                if SWITCHES.contains(&k) {
                    flags.push((k.to_string(), None));
                } else {
                    flags.push((k.to_string(), raw.get(i + 1).cloned()));
                    i += 1;
                }
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

    fn required<T: std::str::FromStr>(&self, k: &str) -> Result<T, String> {
        let v = self.get(k).ok_or_else(|| format!("--{k} is required"))?;
        v.parse().map_err(|_| format!("--{k}: bad value {v}"))
    }

    fn mode(&self) -> Result<String, String> {
        match self.get("mode") {
            Some(m @ ("bots" | "fuzz")) => Ok(m.to_string()),
            _ => Err("--mode bots|fuzz is required".into()),
        }
    }

    fn budgets(&self) -> Result<Budgets, String> {
        Ok(Budgets { hang: Duration::from_secs(self.num("hang-seconds", 120)?), memory: self.num::<usize>("memory-mb", 2048)? << 20 })
    }
}

fn main() -> ExitCode {
    let raw: Vec<String> = std::env::args().skip(1).collect();
    if raw.is_empty() || raw[0] == "-h" || raw[0] == "--help" {
        eprint!("{USAGE}");
        return if raw.is_empty() { ExitCode::from(2) } else { ExitCode::SUCCESS };
    }
    let args = Args::parse(&raw[1..]);
    let result = match raw[0].as_str() {
        "record" => cmd_record(&args),
        "live" => cmd_live(&args),
        "first" => cmd_first(&args),
        "replay" => cmd_replay(&args),
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

fn cmd_record(args: &Args) -> Result<bool, String> {
    let o = RecordOptions {
        mode: args.mode()?,
        seed: args.required("seed")?,
        index: args.required("game")?,
        full_views: args.has("full-views"),
        budgets: args.budgets()?,
    };
    let mut out: Box<dyn Write> = match args.get("out") {
        None | Some("-") => Box::new(std::io::BufWriter::new(std::io::stdout().lock())),
        Some(path) => {
            let f = std::fs::File::create(PathBuf::from(path)).map_err(|e| format!("{path}: {e}"))?;
            if path.ends_with(".gz") {
                Box::new(flate2::write::GzEncoder::new(f, flate2::Compression::fast()))
            } else {
                Box::new(std::io::BufWriter::new(f))
            }
        }
    };
    let reason = record::record_game(&o, &mut out).map_err(|e| e.to_string())?;
    out.flush().map_err(|e| e.to_string())?;
    drop(out);
    if !args.has("quiet") {
        eprintln!("{} game {}: {reason}", o.mode, o.index);
    }
    // A hung game's thread is still running; exit without waiting for it.
    std::process::exit(0);
}

fn reference(args: &Args) -> Result<PathBuf, String> {
    let r = args.get("reference").map(PathBuf::from).unwrap_or_else(live::default_reference);
    if !r.exists() {
        return Err(format!("no reference binary at {}. Build it with rust/tools/build-native-ref.sh (native-harness.md).", r.display()));
    }
    Ok(r)
}

fn cmd_live(args: &Args) -> Result<bool, String> {
    // PORTING.md, "CPU": at most 4 jobs, and one of the machine-wide heavy slots.
    let cpus = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
    let o = LiveOptions {
        mode: args.mode()?,
        seed: args.required("seed")?,
        start: args.num("start", 0)?,
        games: args.required("games")?,
        jobs: args.num("jobs", cpus.min(4))?,
        reference: reference(args)?,
        allow: Arc::new(Allow::load(args.get("allow"))?),
        budgets: args.budgets()?,
        keep_failures: args.get("keep-failures").map(PathBuf::from),
        verbose: args.has("v"),
    };
    let _slot = observe::heavy::acquire_heavy_slot();
    let start = std::time::Instant::now();
    let results = live::run(&o);
    print!("{}", report::summary(&results, start.elapsed().as_secs_f64()));
    if let Some(r) = results.iter().find(|r| !r.outcome.passed()) {
        println!(
            "\nTo see it again:\n  cargo run -p observe --release -- first --mode {} --seed {} --game {}{}",
            r.game.mode,
            r.game.seed,
            r.game.index,
            args.get("allow").map(|a| format!(" --allow {a}")).unwrap_or_default()
        );
    }
    Ok(results.iter().all(|r| r.outcome.passed()))
}

fn cmd_first(args: &Args) -> Result<bool, String> {
    let allow = Arc::new(Allow::load(args.get("allow"))?);
    let budgets = args.budgets()?;
    let (name, outcome) = if let Some(file) = args.positional.first() {
        let reader = replay::open(std::path::Path::new(file)).map_err(|e| format!("{file}: {e}"))?;
        (file.clone(), replay::replay(reader, allow, &budgets))
    } else {
        let g = Game { mode: args.mode()?, seed: args.required("seed")?, index: args.required("game")? };
        let trace = live::record_with_reference(&reference(args)?, &g, true, &budgets)?;
        if let Some(dir) = args.get("keep") {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
            std::fs::write(PathBuf::from(dir).join(format!("{}.jsonl", g.name())), &trace).map_err(|e| e.to_string())?;
        }
        (g.name(), replay::replay(Box::new(std::io::Cursor::new(trace)), allow, &budgets))
    };
    print!("{}", report::describe(&name, &outcome));
    Ok(outcome.passed())
}

fn cmd_replay(args: &Args) -> Result<bool, String> {
    if args.positional.is_empty() {
        return Err("replay: give trace files".into());
    }
    let allow = Arc::new(Allow::load(args.get("allow"))?);
    let budgets = args.budgets()?;
    let mut ok = true;
    for file in &args.positional {
        let reader = replay::open(std::path::Path::new(file)).map_err(|e| format!("{file}: {e}"))?;
        let outcome = replay::replay(reader, allow.clone(), &budgets);
        if args.has("v") || !outcome.passed() {
            print!("{}", report::describe(file, &outcome));
        }
        ok &= outcome.passed();
    }
    if ok {
        println!("{} traces: no difference", args.positional.len());
    }
    Ok(ok)
}
