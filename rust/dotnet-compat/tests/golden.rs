//! Golden tests: fixed cases for `NetRandom`, `NetDictionary`, `NetHashSet` and the number
//! functions, with their results under `tests/golden/`.
//!
//! The expected results were recorded from .NET 10 when the crate was written. They pin the
//! random sequence of each seed and the slot order of dictionaries and sets, which savegames
//! depend on: a savegame is replayed from its events, so a different draw or iteration order
//! turns it into a different game. Change them only together with a deliberate change to the
//! game's randomness or ordering. Then rewrite the results from the Rust code (the cases stay)
//! and review the diff:
//!
//!     UPDATE_GOLDENS=1 cargo test --release -p dotnet-compat --test golden

mod common;

use common::*;
use serde_json::{Value, json};
use std::path::PathBuf;

fn golden_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden").join(format!("{name}.json"))
}

fn check(name: &str) {
    let text = std::fs::read_to_string(golden_path(name)).unwrap();
    let v: Value = serde_json::from_str(&text).unwrap();
    let cases = v["cases"].as_array().unwrap();
    if std::env::var("UPDATE_GOLDENS").is_ok() {
        std::panic::set_hook(Box::new(|_| {}));
        let expected: Vec<Value> = cases.iter().map(run_rust).collect();
        let _ = std::panic::take_hook();
        let text = serde_json::to_string(&json!({"cases": cases, "expected": expected})).unwrap();
        std::fs::write(golden_path(name), text).unwrap();
        return;
    }
    let expected = v["expected"].as_array().unwrap();
    let failures = compare(cases, expected);
    assert!(failures.is_empty(), "{} {name} mismatches:\n{}", failures.len(), failures.join("\n"));
}

#[test]
fn golden_random() {
    check("random");
}

#[test]
fn golden_dict() {
    check("dict");
}

#[test]
fn golden_set() {
    check("set");
}

#[test]
fn golden_math() {
    check("math");
}
