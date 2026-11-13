//! `observe reach`: what a batch of games exercises. Each game is recorded by the working tree's
//! recorder in a child process (`observe record --reach`), which reports every submitted event
//! (type, phase, source, result) and every event type offered to a player. This tallies them:
//! accepted event types, (type, phase) pairs, and types offered but never accepted.

use std::collections::{BTreeMap, BTreeSet};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

#[derive(Default)]
pub struct Tally {
    pub games: usize,
    /// Event type -> (games where it was accepted, times accepted, times accepted from the fuzzer).
    pub types: BTreeMap<String, (usize, usize, usize)>,
    /// (type, phase) -> times accepted.
    pub pairs: BTreeMap<(String, String), usize>,
    /// Types offered to some player at least once -> games.
    pub offered: BTreeMap<String, usize>,
    /// Phases reached -> games.
    pub phases: BTreeMap<String, usize>,
    /// End reasons -> games.
    pub ends: BTreeMap<String, usize>,
    pub events: usize,
    pub accepted: usize,
}

impl Tally {
    fn add_game(&mut self, text: &str) {
        self.games += 1;
        let mut types: BTreeSet<&str> = BTreeSet::new();
        let mut offered: BTreeSet<&str> = BTreeSet::new();
        let mut phases: BTreeSet<&str> = BTreeSet::new();
        for line in text.lines() {
            let f: Vec<&str> = line.split('\t').collect();
            match f.as_slice() {
                ["event", t, phase, src, result] => {
                    self.events += 1;
                    phases.insert(phase);
                    if *result == "accepted" {
                        self.accepted += 1;
                        types.insert(t);
                        let e = self.types.entry(t.to_string()).or_default();
                        e.1 += 1;
                        if matches!(*src, "fuzz" | "mutate") {
                            e.2 += 1;
                        }
                        *self.pairs.entry((t.to_string(), phase.to_string())).or_default() += 1;
                    }
                }
                ["offered", t, phase] => {
                    offered.insert(t);
                    phases.insert(phase);
                }
                ["end", reason] => *self.ends.entry(reason.to_string()).or_default() += 1,
                _ => {}
            }
        }
        for t in types {
            self.types.entry(t.to_string()).or_default().0 += 1;
        }
        for t in offered {
            *self.offered.entry(t.to_string()).or_default() += 1;
        }
        for p in phases {
            *self.phases.entry(p.to_string()).or_default() += 1;
        }
    }

    /// A readable report; also machine-readable enough to diff two runs (`type <name> games n`).
    pub fn report(&self) -> String {
        let mut s = format!(
            "{} games, {} events, {} accepted; {} event types accepted, {} (type, phase) pairs, {} phases seen, {} types offered\n",
            self.games,
            self.events,
            self.accepted,
            self.types.len(),
            self.pairs.len(),
            self.phases.len(),
            self.offered.len()
        );
        s += &format!("ends: {:?}\n\n# type  games-accepted  times-accepted  from-fuzzer\n", self.ends);
        for (t, (g, n, f)) in &self.types {
            s += &format!("type\t{t}\t{g}\t{n}\t{f}\n");
        }
        s += "\n# offered to a player but never accepted (games offered)\n";
        for (t, g) in &self.offered {
            if !self.types.contains_key(t) {
                s += &format!("unexecuted\t{t}\t{g}\n");
            }
        }
        s += "\n# phase  games\n";
        for (p, g) in &self.phases {
            s += &format!("phase\t{p}\t{g}\n");
        }
        s += "\n# type  phase  times-accepted\n";
        for ((t, p), n) in &self.pairs {
            s += &format!("pair\t{t}\t{p}\t{n}\n");
        }
        s
    }
}

pub struct ReachOptions {
    pub mode: String,
    pub seed: u64,
    pub start: u64,
    pub games: u64,
    pub jobs: usize,
    /// The recorder to run: the working tree's own binary, or a reference.
    pub recorder: std::path::PathBuf,
    pub hang_seconds: u64,
}

pub fn run(o: &ReachOptions) -> Tally {
    let next = AtomicUsize::new(0);
    let tally = Mutex::new(Tally::default());
    std::thread::scope(|s| {
        for _ in 0..o.jobs.max(1) {
            s.spawn(|| loop {
                let k = next.fetch_add(1, Ordering::SeqCst) as u64;
                if k >= o.games {
                    break;
                }
                let out = Command::new(&o.recorder)
                    .args(["record", "--reach", "--quiet", "--mode", &o.mode, "--seed", &o.seed.to_string(), "--game", &(o.start + k).to_string()])
                    .args(["--hang-seconds", &o.hang_seconds.to_string()])
                    .stdin(Stdio::null())
                    .stderr(Stdio::null())
                    .output();
                let text = out.map(|o| String::from_utf8_lossy(&o.stdout).into_owned()).unwrap_or_default();
                tally.lock().unwrap().add_game(&text);
            });
        }
    });
    tally.into_inner().unwrap()
}
