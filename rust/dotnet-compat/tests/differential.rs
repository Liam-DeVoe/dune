//! Randomized differential test against .NET 10. Ignored by default:
//!
//! ```text
//! cargo test -p dotnet-compat --test differential -- --ignored --nocapture
//! ```
//!
//! Environment:
//! - `COMPAT_FUZZ_SEED`: base seed (default: from the clock, printed).
//! - `COMPAT_FUZZ_SECONDS`: keep running batches until this many seconds have
//!   passed (default: one batch).
//!
//! Each batch generates thousands of cases in Rust and runs them through the
//! C# reference program in a single process.

mod common;

use common::*;
use serde_json::Value;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

fn batch(seed: u64) -> Vec<Value> {
    let mut g = Gen::new(seed);
    let mut cases = Vec::new();
    for _ in 0..200 {
        let s = if g.chance(0.2) { g.pick(EDGE_SEEDS) } else { g.i32() };
        let n = if g.chance(0.05) { 10000 } else { g.range(1, 500) as usize };
        cases.push(random_case(&mut g, s, n));
    }
    for _ in 0..1000 {
        let n = g.range(1, 600) as usize;
        cases.push(dict_case(&mut g, n));
    }
    for _ in 0..1000 {
        let n = g.range(1, 600) as usize;
        cases.push(set_case(&mut g, n));
    }
    cases.extend(math_cases(&mut g, 5000));
    cases
}

#[test]
#[ignore]
fn differential() {
    let base: u64 = match std::env::var("COMPAT_FUZZ_SEED") {
        Ok(s) => s.parse().unwrap(),
        Err(_) => SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as u64,
    };
    let seconds: u64 = std::env::var("COMPAT_FUZZ_SECONDS").ok().map(|s| s.parse().unwrap()).unwrap_or(0);
    let start = Instant::now();
    let mut i = 0u64;
    loop {
        let seed = base.wrapping_add(i);
        let cases = batch(seed);
        let expected = run_csharp(&cases);
        let failures = compare(&cases, &expected);
        assert!(
            failures.is_empty(),
            "batch seed {seed} (rerun with COMPAT_FUZZ_SEED={seed}): {} mismatches:\n{}",
            failures.len(),
            failures.join("\n")
        );
        eprintln!("batch seed {seed}: {} cases ok", cases.len());
        i += 1;
        if start.elapsed().as_secs() >= seconds {
            break;
        }
    }
}
