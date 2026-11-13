//! Golden tests: fixed cases whose .NET 10 results are committed under
//! `tests/golden/`. Regenerate with
//! `cargo test -p dotnet-compat --test golden -- --ignored regenerate`
//! (needs .NET 10 in $DOTNET_ROOT or ~/dotnet10).

mod common;

use common::*;
use serde_json::{Value, json};
use std::path::PathBuf;

fn golden_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden").join(format!("{name}.json"))
}

/// `new HashSet<T>(otherSet)` clones the slot array (holes included) or
/// compacts it depending on the source's internal capacity, which depends on
/// how the source was built. Walk the capacity boundaries systematically:
/// build from a List / a lazy enumerable with `len` items and `distinct`
/// distinct values, free a slot, copy, add, and look at the order.
fn set_capacity_cases() -> Vec<Value> {
    let mut cases = Vec::new();
    for build in [5, 6] {
        for len in 1..=40 {
            for distinct in 1..=len.min(12) {
                let xs: Vec<i32> = (0..len).map(|i| (i % distinct) as i32).collect();
                for removed in [0, distinct as i32 / 2] {
                    cases.push(json!({"t": "set", "ops": [
                        [build, 0, xs],
                        [1, removed, []],
                        [4, 0, []],
                        [0, 99, []],
                        [0, 98, []],
                    ]}));
                }
            }
        }
    }
    cases
}

fn groups() -> Vec<(&'static str, Vec<Value>)> {
    let mut g = Gen::new(1);
    let mut random = Vec::new();
    for &seed in EDGE_SEEDS {
        random.push(random_case(&mut g, seed, 3000));
    }
    for seed in [0, i32::MIN, i32::MAX] {
        random.push(random_case(&mut g, seed, 10000));
    }
    for _ in 0..100 {
        let seed = g.i32();
        random.push(random_case(&mut g, seed, 200));
    }

    let mut g = Gen::new(2);
    let dict: Vec<Value> = (0..200).map(|_| dict_case(&mut g, 300)).collect();
    let mut set: Vec<Value> = (0..200).map(|_| set_case(&mut g, 300)).collect();
    set.extend(set_capacity_cases());
    let mut g = Gen::new(5);
    let math = math_cases(&mut g, 1500);
    vec![("random", random), ("dict", dict), ("set", set), ("math", math)]
}

fn check(name: &str) {
    let text = std::fs::read_to_string(golden_path(name)).expect("golden file missing; run the regenerate test");
    let v: Value = serde_json::from_str(&text).unwrap();
    let cases = v["cases"].as_array().unwrap();
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

#[test]
#[ignore]
fn regenerate() {
    for (name, cases) in groups() {
        let expected = run_csharp(&cases);
        let text = serde_json::to_string(&json!({"cases": cases, "expected": expected})).unwrap();
        std::fs::write(golden_path(name), text).unwrap();
        eprintln!("wrote {name}: {} cases", cases.len());
    }
}
