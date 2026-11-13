//! Draw accounting for every random helper (ExtensionMethods and Deck<T>)
//! against tests/golden/random_helpers.json (csharp-ref/RandomGrid.cs).
//!
//! For each helper, seed, source size (0, 1, 2, 5), count n (-1, 0, 1, size,
//! size + 1) and key shape (distinct ints, ties, enums, bools, floats with NaN
//! and -0, doubles with NaN, culture-ordered strings), C# recorded the result,
//! how often the selector ran, LoggedRandom.GetCount() and the next random
//! value. Rust must match all four, so an extra draw or an extra selector call
//! fails even when the picked item agrees. Every entry in the golden file
//! must be handled here; a helper this file does not know fails the test.

mod common;

use std::cell::Cell;

use common::*;
use serde_json::{json, Value};
use treachery_engine::cards::deck::Deck;
use treachery_engine::enumerations::Faction;
use treachery_engine::compat::random_ext::{set_static_random_seed, static_random_next, RandomExt, UnseededRandomExt};
use treachery_engine::random::SharedRandom;

/// The key shapes of RandomGrid.Keys, by name. Calls `$body` with `$key`
/// bound to a selector of the right key type.
macro_rules! with_key {
    ($name:expr, $strings:expr, |$key:ident| $body:expr) => {
        match $name {
            // RandomOrDefault takes no selector; any key type will do.
            "-" | "distinct" => { let $key = |x: &i32| (x * 7 + 3) % 11; $body }
            "ties" => { let $key = |x: &i32| x % 2; $body }
            "enum" => { let $key = |x: &i32| if x % 2 == 0 { Faction::Atreides } else { Faction::Harkonnen }; $body }
            "bool" => { let $key = |x: &i32| x % 3 == 0; $body }
            "float" => { let $key = |x: &i32| if *x == 1 { f32::NAN } else if x % 3 == 2 { -0.0f32 } else { (x % 3) as f32 }; $body }
            "double" => { let $key = |x: &i32| if *x == 3 { f64::NAN } else { (x % 2) as f64 / 3.0 }; $body }
            "string" => { let s = $strings; let $key = move |x: &i32| s[*x as usize % s.len()].clone(); $body }
            other => panic!("random_helpers.json uses key shape {other:?}, which this test does not know"),
        }
    };
}

fn strings() -> Vec<String> {
    golden("random_helpers.json")["strings"].as_array().unwrap().iter().map(|s| s.as_str().unwrap().to_string()).collect()
}

/// Runs one helper; returns (result, selector calls).
fn run_seeded(helper: &str, key: &str, size: i32, n: i32, r: &SharedRandom) -> (Value, i32) {
    let calls = Cell::new(0);
    let source = || 0..size;
    let result = with_key!(key, strings(), |k| {
        let sel = |x: &i32| {
            calls.set(calls.get() + 1);
            k(x)
        };
        try_enc(|| match helper {
            "RandomOrDefault" => source().random_or_default(r),
            "HighestOrDefault" => source().highest_or_default(r, sel),
            "LowestOrDefault" => source().lowest_or_default(r, sel),
            "OneOfHighestNOrDefault" => source().one_of_highest_n_or_default(r, sel, n),
            "OneOfLowestNOrDefault" => source().one_of_lowest_n_or_default(r, sel, n),
            other => panic!("random_helpers.json has seeded helper {other:?}, which this test does not know"),
        }
        // C# returns default(int) when nothing is picked.
        .unwrap_or_default())
    });
    (result, calls.get())
}

fn run_unseeded(helper: &str, key: &str, size: i32, n: i32) -> (Value, i32) {
    let calls = Cell::new(0);
    let source = || 0..size;
    let result = with_key!(key, strings(), |k| {
        let sel = |x: &i32| {
            calls.set(calls.get() + 1);
            k(x)
        };
        try_enc(|| match helper {
            "RandomOrDefault" => source().random_or_default_unseeded(),
            "HighestOrDefault" => source().highest_or_default_unseeded(sel),
            "LowestOrDefault" => source().lowest_or_default_unseeded(sel),
            "OneOfHighestNOrDefault" => source().one_of_highest_n_or_default_unseeded(sel, n),
            "OneOfLowestNOrDefault" => source().one_of_lowest_n_or_default_unseeded(sel, n),
            other => panic!("random_helpers.json has unseeded helper {other:?}, which this test does not know"),
        }
        .unwrap_or_default())
    });
    (result, calls.get())
}

fn field(e: &Value, name: &str) -> i32 {
    int(&e[name])
}

#[test]
fn seeded_helpers() {
    let mut failures = Failures::default();
    for e in golden("random_helpers.json")["seeded"].as_array().unwrap() {
        let (helper, key) = (e["helper"].as_str().unwrap(), e["key"].as_str().unwrap());
        let r = SharedRandom::from_seed(field(e, "seed"));
        let (result, calls) = run_seeded(helper, key, field(e, "size"), field(e, "n"), &r);
        let actual = json!({
            "result": result, "calls": calls, "count": r.get_count(), "next": r.next(1000),
        });
        let expected = json!({ "result": e["result"], "calls": e["calls"], "count": e["count"], "next": e["next"] });
        failures.check(|| format!("{e}"), &expected, &actual);
    }
    failures.finish("seeded random helpers");
}

#[test]
fn unseeded_helpers() {
    let mut failures = Failures::default();
    for e in golden("random_helpers.json")["unseeded"].as_array().unwrap() {
        let (helper, key) = (e["helper"].as_str().unwrap(), e["key"].as_str().unwrap());
        set_static_random_seed(field(e, "seed"));
        let (result, calls) = run_unseeded(helper, key, field(e, "size"), field(e, "n"));
        let actual = json!({ "result": result, "calls": calls, "next": static_random_next(1000) });
        let expected = json!({ "result": e["result"], "calls": e["calls"], "next": e["next"] });
        failures.check(|| format!("{e}"), &expected, &actual);
    }
    failures.finish("unseeded random helpers");
}

/// `TakeRandomN` is a lazy query: no draws when it is created, fresh draws on
/// every enumeration, none at all for n <= 0.
#[test]
fn take_random_n() {
    let mut failures = Failures::default();
    for e in golden("random_helpers.json")["take_random_n"].as_array().unwrap() {
        let (seed, size, n) = (field(e, "seed"), field(e, "size"), field(e, "n"));

        // Did creating the query draw? Compare the next value with and without it.
        set_static_random_seed(seed);
        let _query = (0..size).take_random_n(n);
        let after_call = static_random_next(i32::MAX);
        set_static_random_seed(seed);
        let drew_on_call = static_random_next(i32::MAX) != after_call;

        set_static_random_seed(seed);
        let query = (0..size).take_random_n(n);
        let first = try_enc(|| query.clone().collect::<Vec<_>>());
        let second = try_enc(|| query.clone().collect::<Vec<_>>());
        let actual = json!({ "drew_on_call": drew_on_call, "first": first, "second": second, "next": static_random_next(1000) });
        let expected = json!({ "drew_on_call": e["drew_on_call"], "first": e["first"], "second": e["second"], "next": e["next"] });
        failures.check(|| format!("{e}"), &expected, &actual);
    }
    failures.finish("TakeRandomN");
}

#[test]
fn deck() {
    let mut failures = Failures::default();
    for e in golden("random_helpers.json")["deck"].as_array().unwrap() {
        let (seed, size) = (field(e, "seed"), field(e, "size"));
        let r = SharedRandom::from_seed(seed);
        let mut d = Deck::new(0..size, Some(r.clone()));
        d.shuffle();
        // Drawing from the empty deck at the end fails. C# threw an exception there, Rust returns
        // a GameError: both are encoded as "x:error", without the exception type or message.
        let drawn: Vec<Value> = (0..=size).map(|_| d.try_draw().map_or_else(|_| json!("x:error"), |v| v.enc())).collect();
        let expected_drawn: Vec<Value> = e["drawn"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| if v.as_str().is_some_and(|s| s.starts_with("x:")) { json!("x:error") } else { v.clone() })
            .collect();
        // Deck<int>.Top of an empty deck is default(int).
        let top = try_enc(|| d.top().unwrap_or_default());
        let actual = json!({ "drawn": drawn, "top": top, "count": r.get_count(), "next": r.next(1000) });
        let expected = json!({ "drawn": expected_drawn, "top": e["top"], "count": e["count"], "next": e["next"] });
        failures.check(|| format!("{e}"), &expected, &actual);
    }
    failures.finish("Deck");
}
