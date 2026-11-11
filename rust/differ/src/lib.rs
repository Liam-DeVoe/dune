//! The differential testing harness: replays C# oracle traces through the
//! Rust engine and reports the first difference. See `rust/docs/differ.md`.

pub mod diff;
pub mod live;
pub mod memory;
pub mod panics;
pub mod replay;
pub mod report;
pub mod trace;

use sha2::{Digest, Sha256};

/// Lowercase hex SHA-256, as `StateDumper.Sha256Hex`.
pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut s = String::with_capacity(64);
    for b in digest {
        s.push_str(&format!("{b:02x}"));
    }
    s
}
