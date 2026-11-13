//! The machine-wide heavy-run slots of `rust/tools/heavy.sh` (PORTING.md, "CPU"). Copied from
//! `differ/src/live.rs`, which will go away with the C# oracle.

use std::path::PathBuf;
use std::process::{Command, Stdio};


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
                eprintln!("observe live: running in heavy slot {i}");
                return HeavySlot(Some(slot));
            }
            let pid = std::fs::read_to_string(slot.join("pid")).ok().and_then(|p| p.trim().parse::<u32>().ok());
            let alive = pid.map_or(true, |p| Command::new("kill").args(["-0", &p.to_string()]).stderr(Stdio::null()).status().is_ok_and(|s| s.success()));
            if !alive {
                let _ = std::fs::remove_dir_all(&slot);
            }
        }
        if !announced {
            eprintln!("observe live: waiting for a free heavy slot (rust/tools/heavy.sh, PORTING.md \"CPU\")");
            announced = true;
        }
        std::thread::sleep(std::time::Duration::from_secs(10));
    }
}

