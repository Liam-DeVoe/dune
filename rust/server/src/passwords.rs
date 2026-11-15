//! Account passwords. The client sends the SHA-256 of the password (protocol.md, Server and
//! accounts), and the server stores an Argon2id hash of that value with a random salt, as a PHC
//! string (`$argon2id$v=19$m=19456,t=2,p=1$<salt>$<hash>`). The parameters are OWASP's minimum
//! for Argon2id. A stored hash names its own parameters, so they can be raised later without a
//! migration.

use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::{Algorithm, Argon2, Params, Version};

use super::ids;

/// The `users.password_scheme` of the hashes made here (db.rs).
pub const SCHEME: i64 = 1;

fn argon2() -> Argon2<'static> {
    Argon2::new(Algorithm::Argon2id, Version::V0x13, Params::new(19 * 1024, 2, 1, None).expect("valid Argon2 parameters"))
}

/// The hash to store for what the client sent.
pub fn hash(client_hash: &str) -> String {
    let salt = SaltString::encode_b64(&ids::random_bytes::<16>()).expect("a 16-byte salt encodes");
    argon2().hash_password(client_hash.as_bytes(), &salt).expect("Argon2 hashes any input").to_string()
}

/// Whether what the client sent matches a stored hash.
pub fn verify(client_hash: &str, stored: &str) -> bool {
    match PasswordHash::new(stored) {
        Ok(h) => argon2().verify_password(client_hash.as_bytes(), &h).is_ok(),
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hashes_are_salted_and_verify() {
        let a = hash("abc");
        let b = hash("abc");
        assert_ne!(a, b, "a new salt each time");
        assert!(a.starts_with("$argon2id$v=19$m=19456,t=2,p=1$"), "{a}");
        assert!(verify("abc", &a) && verify("abc", &b));
        assert!(!verify("abd", &a));
        assert!(!verify("abc", "abc"), "a stored value that isn't a hash matches nothing");
    }
}
