//! The per-replay memory budget (`differ::memory`): a thread that keeps allocating is stopped
//! near its budget, and its flag is raised for the thread that watches it.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[test]
fn runaway_thread_is_stopped_at_its_budget() {
    // Link the differ library, which installs the counting allocator.
    let _ = differ::replay::Options::default();

    let flag = Arc::new(AtomicBool::new(false));
    let held = Arc::new(AtomicUsize::new(0));
    let (f, h) = (flag.clone(), held.clone());
    std::thread::spawn(move || {
        let _budget = differ::memory::set_budget(64 << 20, f);
        let mut chunks: Vec<Vec<u8>> = Vec::new();
        loop {
            chunks.push(vec![1u8; 1 << 20]);
            h.store(chunks.len(), Ordering::SeqCst);
        }
    });

    let start = Instant::now();
    while !flag.load(Ordering::SeqCst) {
        assert!(start.elapsed() < Duration::from_secs(30), "the budget was never enforced ({} MB held)", held.load(Ordering::SeqCst));
        std::thread::sleep(Duration::from_millis(20));
    }
    let mb = held.load(Ordering::SeqCst);
    assert!((60..=64).contains(&mb), "stopped at {mb} MB, expected just under 64");

    // It stays stopped.
    std::thread::sleep(Duration::from_millis(300));
    assert_eq!(held.load(Ordering::SeqCst), mb);
}

#[test]
fn threads_without_a_budget_are_not_limited() {
    let v = vec![0u8; 256 << 20];
    assert_eq!(v.len(), 256 << 20);
}
