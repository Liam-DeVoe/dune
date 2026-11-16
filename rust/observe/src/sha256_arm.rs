//! SHA-256 with the aarch64 SHA-2 instructions, for the views of traces that hash them with
//! SHA-256 (`view_hash.rs`). The `sha2` crate only uses these instructions with its `asm`
//! feature, which needs the `sha2-asm` crate. The compression function follows `sha2`'s
//! aarch64 backend (`sha2-0.10.9/src/sha256/aarch64.rs`), with the intrinsics Rust now has.
//! Tests check it against `sha2`.

use std::arch::aarch64::*;

const K32: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

const H0: [u32; 8] = [0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19];

/// Whether this CPU has the SHA-2 instructions.
pub fn available() -> bool {
    std::arch::is_aarch64_feature_detected!("sha2")
}

/// A SHA-256 being computed. Make one only when [`available`].
pub struct Sha256Arm {
    state: [u32; 8],
    buf: [u8; 64],
    /// Bytes waiting in `buf`.
    n: usize,
    len: u64,
}

impl Sha256Arm {
    pub fn new() -> Sha256Arm {
        assert!(available(), "the CPU has no SHA-2 instructions");
        Sha256Arm { state: H0, buf: [0; 64], n: 0, len: 0 }
    }

    fn compress(&mut self, blocks: &[u8]) {
        // SAFETY: `new` checked that the CPU has the SHA-2 instructions (NEON is part of aarch64).
        unsafe { compress(&mut self.state, blocks) }
    }

    pub fn update(&mut self, mut data: &[u8]) {
        self.len = self.len.wrapping_add(data.len() as u64);
        if self.n > 0 {
            let take = (64 - self.n).min(data.len());
            self.buf[self.n..self.n + take].copy_from_slice(&data[..take]);
            self.n += take;
            data = &data[take..];
            if self.n < 64 {
                return;
            }
            let buf = self.buf;
            self.compress(&buf);
            self.n = 0;
        }
        let whole = data.len() / 64 * 64;
        if whole > 0 {
            self.compress(&data[..whole]);
        }
        let rest = &data[whole..];
        self.buf[..rest.len()].copy_from_slice(rest);
        self.n = rest.len();
    }

    pub fn finalize(mut self) -> [u8; 32] {
        let bits = self.len.wrapping_mul(8);
        let mut tail = [0u8; 128];
        tail[..self.n].copy_from_slice(&self.buf[..self.n]);
        tail[self.n] = 0x80;
        let end = if self.n < 56 { 64 } else { 128 };
        tail[end - 8..end].copy_from_slice(&bits.to_be_bytes());
        self.compress(&tail[..end]);
        let mut out = [0u8; 32];
        for (o, s) in out.chunks_exact_mut(4).zip(self.state) {
            o.copy_from_slice(&s.to_be_bytes());
        }
        out
    }
}

/// Compresses whole 64-byte blocks into `state`.
#[target_feature(enable = "sha2")]
unsafe fn compress(state: &mut [u32; 8], blocks: &[u8]) {
    debug_assert!(blocks.len() % 64 == 0);
    unsafe {
        let mut abcd = vld1q_u32(state[0..4].as_ptr());
        let mut efgh = vld1q_u32(state[4..8].as_ptr());
        for block in blocks.chunks_exact(64) {
            let (abcd_orig, efgh_orig) = (abcd, efgh);
            let load = |i: usize| vreinterpretq_u32_u8(vrev32q_u8(vld1q_u8(block[i..i + 16].as_ptr())));
            let mut s = [load(0), load(16), load(32), load(48)];
            for t in (0..64).step_by(4) {
                let i = t / 4 % 4;
                if t >= 16 {
                    s[i] = vsha256su1q_u32(vsha256su0q_u32(s[i], s[(i + 1) % 4]), s[(i + 2) % 4], s[(i + 3) % 4]);
                }
                let wk = vaddq_u32(s[i], vld1q_u32(K32[t..t + 4].as_ptr()));
                let prev = abcd;
                abcd = vsha256hq_u32(prev, efgh, wk);
                efgh = vsha256h2q_u32(efgh, prev, wk);
            }
            abcd = vaddq_u32(abcd, abcd_orig);
            efgh = vaddq_u32(efgh, efgh_orig);
        }
        vst1q_u32(state[0..4].as_mut_ptr(), abcd);
        vst1q_u32(state[4..8].as_mut_ptr(), efgh);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};

    #[test]
    fn same_as_the_sha2_crate() {
        if !available() {
            return;
        }
        let data: Vec<u8> = (0..1000u32).map(|i| (i * 7 + i / 13) as u8).collect();
        for n in [0, 1, 55, 56, 57, 63, 64, 65, 119, 120, 128, 200, 1000] {
            let expected: [u8; 32] = Sha256::digest(&data[..n]).into();
            for step in [1, 5, 64, 100, 1000] {
                let mut h = Sha256Arm::new();
                for piece in data[..n].chunks(step) {
                    h.update(piece);
                }
                assert_eq!(h.finalize(), expected, "{n} bytes in pieces of {step}");
            }
        }
    }
}
