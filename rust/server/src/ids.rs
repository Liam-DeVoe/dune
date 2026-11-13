//! Random tokens, game ids and seeds, from the operating system's generator, and the digests
//! tokens are stored as.

use sha2::{Digest, Sha256};

pub fn random_bytes<const N: usize>() -> [u8; N] {
    let mut b = [0u8; N];
    getrandom::fill(&mut b).expect("the operating system's random number generator failed");
    b
}

/// Uniform in `0..n`, by rejection.
fn random_below(n: u32) -> u32 {
    let mask = if n <= 1 { 0 } else { u32::MAX >> (n - 1).leading_zeros() };
    loop {
        let r = u32::from_le_bytes(random_bytes::<4>()) & mask;
        if r < n {
            return r;
        }
    }
}

const TOKEN_CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|x| format!("{x:02x}")).collect()
}

/// A password reset token, which the user copies from a mail: 16 letters and digits.
pub fn new_reset_token() -> String {
    (0..16).map(|_| TOKEN_CHARS[random_below(TOKEN_CHARS.len() as u32) as usize] as char).collect()
}

/// A session or login link token: 32 random bytes as 64 lowercase hex digits.
pub fn new_token() -> String {
    hex(&random_bytes::<32>())
}

/// What the server keeps of a token: its SHA-256 as 64 lowercase hex digits, so a copy of the
/// database doesn't give the tokens away.
pub fn digest(token: &str) -> String {
    hex(&Sha256::digest(token.as_bytes()))
}

/// A random (version 4) UUID, lowercase: the id of a game or a scheduled game.
pub fn new_id() -> String {
    let mut b = random_bytes::<16>();
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    let h = hex(&b);
    format!("{}-{}-{}-{}-{}", &h[0..8], &h[8..12], &h[12..16], &h[16..20], &h[20..32])
}

/// A non-negative seed for the bots' random numbers and for new games.
pub fn new_seed() -> i32 {
    (u32::from_le_bytes(random_bytes::<4>()) >> 1) as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_and_ids() {
        let t = new_reset_token();
        assert_eq!(t.len(), 16);
        assert!(t.bytes().all(|c| TOKEN_CHARS.contains(&c)));
        assert_ne!(t, new_reset_token());
        let t = new_token();
        assert_eq!(t.len(), 64);
        assert_ne!(t, new_token());
        assert_eq!(digest("abc"), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
        let g = new_id();
        assert_eq!(g.len(), 36);
        assert_eq!(&g[14..15], "4");
        assert!(new_seed() >= 0);
    }

    #[test]
    fn token_characters_are_uniform() {
        let mut counts = [0u32; 62];
        for _ in 0..62_000 {
            counts[random_below(62) as usize] += 1;
        }
        assert!(counts.iter().all(|&c| (800..1200).contains(&c)), "{counts:?}");
        assert_eq!(random_below(1), 0);
    }
}
