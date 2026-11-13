//! Runs one game (recording or replaying) on its own thread and watches it: a call that runs
//! longer than the hang budget, or a thread that goes over its memory budget, ends the game.
//! A thread can't be stopped, so it is left behind (the recorder exits the process right after).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::trace::Line;

pub enum Msg {
    /// A call is about to start; the line holds its inputs.
    Begin(Line),
    /// A finished line.
    Line(Line),
    /// Anything else that should reach the watcher (the replayer's report).
    Text(String),
}

pub enum Ending {
    /// The thread returned.
    Finished,
    /// The pending call (if one was announced) didn't return within the budget.
    Hung(Option<Line>),
    /// The thread went over its memory budget during the pending call.
    MemoryLimit(Option<Line>),
}

pub struct Budgets {
    pub hang: Duration,
    /// Bytes; 0 for none.
    pub memory: usize,
}

/// Runs `work` on a new thread and passes every message to `on_msg` as it arrives.
pub fn run(budgets: &Budgets, work: impl FnOnce(&Sender<Msg>) + Send + 'static, mut on_msg: impl FnMut(Msg)) -> Ending {
    let (tx, rx) = mpsc::channel::<Msg>();
    let flag = Arc::new(AtomicBool::new(false));
    let thread_flag = flag.clone();
    let memory = budgets.memory;
    let spawned = std::thread::Builder::new().stack_size(256 << 20).spawn(move || {
        let _budget = crate::memory::set_budget(memory, thread_flag);
        work(&tx);
    });
    if spawned.is_err() {
        return Ending::Finished;
    }
    let mut pending: Option<Line> = None;
    let mut since = Instant::now();
    loop {
        match rx.recv_timeout(Duration::from_millis(250)) {
            Ok(Msg::Begin(l)) => {
                pending = Some(l);
                since = Instant::now();
            }
            Ok(m) => {
                if matches!(m, Msg::Line(_)) {
                    pending = None;
                }
                since = Instant::now();
                on_msg(m);
            }
            Err(RecvTimeoutError::Disconnected) => return Ending::Finished,
            Err(RecvTimeoutError::Timeout) => {
                if flag.load(Ordering::SeqCst) {
                    return Ending::MemoryLimit(pending);
                }
                if since.elapsed() > budgets.hang {
                    return Ending::Hung(pending);
                }
            }
        }
    }
}
