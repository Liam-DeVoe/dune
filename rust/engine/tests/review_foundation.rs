//! Edge cases of the foundation layer (messages, random helpers, JSON, map lookups). Each test
//! failed until its bug was fixed (see rust/docs/escapes.md), except the ones marked as guards.

use treachery_engine::error::catch_panic;
use treachery_engine::enumerations::*;
use treachery_engine::messages::part::Part;

fn exception_text<R>(f: impl FnOnce() -> R) -> String {
    match catch_panic(f) {
        Ok(_) => "no exception".to_string(),
        Err(e) => e,
    }
}

// ---------------------------------------------------------------- enumerations.rs

/// Probe `to_nexus_13`: an undefined faction panics.
#[test]
fn to_nexus_undefined_message() {
    assert_eq!(
        exception_text(|| Faction(13).to_nexus()),
        "faction out of range: 13"
    );
}

// ---------------------------------------------------------------- extension_methods.rs

// ---------------------------------------------------------------- part.rs / message.rs

/// Probe `msg_float_small`. .NET's shortest round-trip float formatting
/// switches to exponent notation; Rust's `Display` never does.
#[test]
fn float_plain_text_small() {
    assert_eq!(Part::Float(0.00001).plain_text(), "1E-05");
}

/// Probe `msg_float_big`.
#[test]
fn float_plain_text_big() {
    assert_eq!(Part::Float(1e20).plain_text(), "1E+20");
}

/// Probe `msg_double_small`.
#[test]
fn double_plain_text_small() {
    assert_eq!(Part::Double(1e-7).plain_text(), "1E-07");
}

/// Probes `float_inf_invariant` ("Infinity") and `msg_float_inf` ("∞" under
/// en-US). Rust prints "inf", which matches neither culture. Messages render under the
/// invariant culture, so this checks that. `dotnet_compat::fmt::f32_to_string` already implements it.
#[test]
fn float_plain_text_infinity() {
    assert_eq!(Part::Float(f32::INFINITY).plain_text(), "Infinity");
}

/// A game error prints as its message.
#[test]
fn game_error_plain_text() {
    let e = treachery_engine::error::GameError::new("Cannot draw from an empty deck");
    assert_eq!(Part::Error(e).plain_text(), "Cannot draw from an empty deck");
}

#[test]
fn list_plain_text() {
    let l = Part::from(vec![Faction::Atreides, Faction::Fremen]);
    assert_eq!(l.plain_text(), "[Atreides, Fremen]");
}
