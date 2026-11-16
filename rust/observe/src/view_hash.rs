//! The hashes of views in a trace's `views` lists.
//!
//! A game builds tens of thousands of views of about 80 KB each, so the hash is a good part of a
//! replay's time. Traces recorded before 2026-11-16 hold SHA-256 prefixes. Newer ones hold
//! [`Fold64`], a much faster hash, and say so in their header (`viewHash`). A replay hashes its
//! views the way the trace's header says, so old traces still replay.
//!
//! Neither is meant to resist an attacker: a hash only stands for a view that the replay builds
//! again, and two different views get the same hash with a chance of about 2^-64.

use std::io::{self, Write};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// How a trace hashes its views. Traces without `viewHash` in their header use SHA-256.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ViewHash {
    /// The first 8 bytes of the SHA-256 of the view's JSON, as 16 hex digits.
    #[default]
    Sha256,
    /// [`Fold64`] of the view's JSON, as 16 hex digits.
    Fold64,
}

impl ViewHash {
    /// What the recorder writes.
    pub const LATEST: ViewHash = ViewHash::Fold64;

    pub fn name(self) -> &'static str {
        match self {
            ViewHash::Sha256 => "sha256",
            ViewHash::Fold64 => "fold64",
        }
    }

    pub fn parse(name: &str) -> Option<ViewHash> {
        [ViewHash::Sha256, ViewHash::Fold64].into_iter().find(|h| h.name() == name)
    }

    pub fn hasher(self) -> Hasher {
        match self {
            #[cfg(target_arch = "aarch64")]
            ViewHash::Sha256 if crate::sha256_arm::available() => Hasher::Sha256Arm(crate::sha256_arm::Sha256Arm::new()),
            ViewHash::Sha256 => Hasher::Sha256(Sha256::new()),
            ViewHash::Fold64 => Hasher::Fold64(Fold64::new()),
        }
    }

    /// The hash of `bytes`, as written in a trace.
    pub fn of(self, bytes: &[u8]) -> String {
        let mut h = self.hasher();
        h.update(bytes);
        h.finish()
    }
}

/// A hash being computed. Serialize a view into it with `serde_json::to_writer`.
pub enum Hasher {
    Sha256(Sha256),
    /// SHA-256 with the CPU's SHA-2 instructions, several times faster.
    #[cfg(target_arch = "aarch64")]
    Sha256Arm(crate::sha256_arm::Sha256Arm),
    Fold64(Fold64),
}

impl Hasher {
    pub fn update(&mut self, bytes: &[u8]) {
        match self {
            Hasher::Sha256(h) => Digest::update(h, bytes),
            #[cfg(target_arch = "aarch64")]
            Hasher::Sha256Arm(h) => h.update(bytes),
            Hasher::Fold64(h) => h.update(bytes),
        }
    }

    /// The hash as 16 hex digits.
    pub fn finish(self) -> String {
        match self {
            Hasher::Sha256(h) => hex8(&h.finalize()),
            #[cfg(target_arch = "aarch64")]
            Hasher::Sha256Arm(h) => hex8(&h.finalize()),
            Hasher::Fold64(h) => format!("{:016x}", h.finish()),
        }
    }
}

fn hex8(digest: &[u8]) -> String {
    digest.iter().take(8).map(|b| format!("{b:02x}")).collect()
}

impl Write for Hasher {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.update(buf);
        Ok(buf.len())
    }

    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.update(buf);
        Ok(())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// A 64-bit hash of a byte stream, defined here so that it never changes with a crate or
/// compiler version. The stream is cut into 16-byte blocks, the last one padded with zeros. Each
/// block `(x, y)` (two little-endian u64) updates the state `s` to `fold(x ^ P0, y ^ s)`, where
/// `fold` multiplies to 128 bits and xors the halves (as wyhash and foldhash do). The hash is
/// `fold(s ^ P2, length ^ P1)`.
///
/// Changing anything here changes every hash: record such a change as a new [`ViewHash`].
pub struct Fold64 {
    state: u64,
    len: u64,
    buf: [u8; BUF],
    /// Bytes waiting in `buf`. The stream before them is a whole number of blocks.
    n: usize,
}

const BLOCK: usize = 16;
const BUF: usize = 4 * BLOCK;
const P0: u64 = 0xa076_1d64_78bd_642f;
const P1: u64 = 0xe703_7ed1_a0b4_28db;
const P2: u64 = 0x8ebc_6af0_9c88_c6e3;

fn fold(a: u64, b: u64) -> u64 {
    let r = u128::from(a) * u128::from(b);
    (r as u64) ^ ((r >> 64) as u64)
}

impl Default for Fold64 {
    fn default() -> Self {
        Fold64::new()
    }
}

impl Fold64 {
    pub fn new() -> Fold64 {
        Fold64 { state: 0, len: 0, buf: [0; BUF], n: 0 }
    }

    fn block(state: u64, b: &[u8]) -> u64 {
        let x = u64::from_le_bytes(b[..8].try_into().expect("8 bytes"));
        let y = u64::from_le_bytes(b[8..BLOCK].try_into().expect("8 bytes"));
        fold(x ^ P0, y ^ state)
    }

    pub fn update(&mut self, mut data: &[u8]) {
        self.len = self.len.wrapping_add(data.len() as u64);
        if self.n > 0 {
            let take = (BUF - self.n).min(data.len());
            self.buf[self.n..self.n + take].copy_from_slice(&data[..take]);
            self.n += take;
            data = &data[take..];
            if self.n < BUF {
                return;
            }
            for b in self.buf.chunks_exact(BLOCK) {
                self.state = Fold64::block(self.state, b);
            }
            self.n = 0;
        }
        let blocks = data.chunks_exact(BLOCK);
        let rest = blocks.remainder();
        for b in blocks {
            self.state = Fold64::block(self.state, b);
        }
        self.buf[..rest.len()].copy_from_slice(rest);
        self.n = rest.len();
    }

    pub fn finish(&self) -> u64 {
        let mut state = self.state;
        for b in self.buf[..self.n].chunks(BLOCK) {
            let mut last = [0u8; BLOCK];
            last[..b.len()].copy_from_slice(b);
            state = Fold64::block(state, &last);
        }
        fold(state ^ P2, self.len ^ P1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(n: usize) -> Vec<u8> {
        (0..n).map(|i| b"{\"view\":[1,2,3],\"x\":\"abc\"}"[i % 26]).collect()
    }

    /// The hash of a stream doesn't depend on how it is written.
    #[test]
    fn pieces_hash_like_the_whole() {
        for kind in [ViewHash::Sha256, ViewHash::Fold64] {
            for n in [0, 1, 15, 16, 17, 63, 64, 65, 200, 1000] {
                let data = text(n);
                let whole = kind.of(&data);
                for step in [1, 3, 7, 16, 50] {
                    let mut h = kind.hasher();
                    for piece in data.chunks(step) {
                        h.write_all(piece).unwrap();
                    }
                    assert_eq!(h.finish(), whole, "{kind:?}, {n} bytes in pieces of {step}");
                }
            }
        }
    }

    /// The hashes in recorded traces must never change.
    #[test]
    fn hashes_are_pinned() {
        assert_eq!(ViewHash::Sha256.of(b"abc"), "ba7816bf8f01cfea");
        assert_eq!(ViewHash::Fold64.of(b""), "e6d24d7e0390ae94");
        assert_eq!(ViewHash::Fold64.of(b"abc"), "39278e2fd636752b");
        assert_eq!(ViewHash::Fold64.of(&text(1000)), "11e51b5b9dfb9ade");
    }

    #[test]
    fn length_and_content_matter() {
        assert_ne!(ViewHash::Fold64.of(b"a"), ViewHash::Fold64.of(b"a\0"));
        assert_ne!(ViewHash::Fold64.of(&text(100)), ViewHash::Fold64.of(&text(101)[1..]));
    }
}
