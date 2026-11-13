//! The .NET behaviour that src/compat/cs.rs stands in for, against
//! tests/golden/linq.json (csharp-ref/LinqTable.cs).
//!
//! Each LINQ operator with a cs.rs helper runs on small inputs from a
//! `List<int>` ("list") and from a lazy sequence ("lazy"); the result or
//! exception text and the number of predicate calls must match. .NET walks a
//! list backwards for `Last(predicate)`, so the shape matters. Indexers, int
//! division, dictionary misses and exception default messages are checked the
//! same way. An operator or exception in the golden file that this test does
//! not handle fails it.

mod common;

use std::cell::Cell;

use common::*;
use serde_json::{json, Value};
use treachery_engine::compat::cs::{self, CsException, CsIterExt};
use treachery_engine::enumerations::Faction;

/// Runs one operator on `input` with the predicate `x == 1`; (result, calls).
fn run_op(op: &str, shape: &str, input: &[i32]) -> (Value, i32) {
    let calls = Cell::new(0);
    let pred = |x: &i32| {
        calls.set(calls.get() + 1);
        *x == 1
    };
    let pred_ref = |x: &&i32| pred(x);
    let list = shape == "list";
    let it = || input.iter().copied();
    let result = match op {
        "First()" => try_enc(|| it().cs_first()),
        "First(pred)" => try_enc(|| it().cs_first_where(pred)),
        "FirstOrDefault()" => try_enc(|| it().next().unwrap_or_default()),
        "FirstOrDefault(pred)" => try_enc(|| it().cs_first_or_default_where(pred).unwrap_or_default()),
        "Last()" => try_enc(|| it().cs_last()),
        "Last(pred)" if list => try_enc(|| *cs::last_where_list(input, pred)),
        "Last(pred)" => try_enc(|| it().cs_last_where(pred)),
        "LastOrDefault()" => try_enc(|| it().last().unwrap_or_default()),
        "LastOrDefault(pred)" if list => try_enc(|| cs::last_or_default_where_list(input, pred).copied().unwrap_or_default()),
        "LastOrDefault(pred)" => try_enc(|| it().cs_last_or_default_where(pred).unwrap_or_default()),
        "Single()" => try_enc(|| it().cs_single()),
        "Single(pred)" => try_enc(|| it().cs_single_where(pred)),
        "SingleOrDefault()" => try_enc(|| it().cs_single_or_default().unwrap_or_default()),
        "SingleOrDefault(pred)" => try_enc(|| it().cs_single_or_default_where(pred).unwrap_or_default()),
        "Any(pred)" => try_enc(|| it().any(|x| pred(&x))),
        "All(pred)" => try_enc(|| it().all(|x| pred(&x))),
        "Count(pred)" => try_enc(|| input.iter().filter(pred_ref).count() as i32),
        "Max()" => try_enc(|| it().cs_max()),
        "Min()" => try_enc(|| it().cs_min()),
        "Distinct()" => try_enc(|| it().cs_distinct()),
        "ElementAt(1)" if list => try_enc(|| *cs::element_at_list(input, 1)),
        "ElementAt(1)" => try_enc(|| it().cs_element_at(1)),
        "ElementAt(-1)" if list => try_enc(|| *cs::element_at_list(input, -1)),
        "ElementAt(-1)" => try_enc(|| it().cs_element_at(-1)),
        "ElementAtOrDefault(1)" => try_enc(|| it().nth(1).unwrap_or_default()),
        other => panic!("linq.json has operator {other:?}, which this test does not know"),
    };
    (result, calls.get())
}

fn ints(v: &Value) -> Vec<i32> {
    v.as_array().unwrap().iter().map(int).collect()
}

#[test]
fn operators() {
    let mut failures = Failures::default();
    for e in golden("linq.json")["ops"].as_array().unwrap() {
        let (op, shape) = (e["op"].as_str().unwrap(), e["shape"].as_str().unwrap());
        let (result, calls) = run_op(op, shape, &ints(&e["input"]));
        failures.check(
            || format!("{op} on {shape} {}", e["input"]),
            &json!({ "result": e["result"], "calls": e["calls"] }),
            &json!({ "result": result, "calls": calls }),
        );
    }
    failures.finish("LINQ operators");
}

#[test]
fn indexers() {
    let mut failures = Failures::default();
    for e in golden("linq.json")["indexers"].as_array().unwrap() {
        let (input, i) = (ints(&e["input"]), int(&e["index"]));
        let actual = json!({
            "list": try_enc(|| *cs::index(&input, i)),
            "array": try_enc(|| *cs::index_array(&input, i)),
            "list_set": try_enc(|| { let mut l = input.clone(); *cs::index_mut(&mut l, i) = 9; l }),
            "array_set": try_enc(|| { let mut a = input.clone(); *cs::index_array_mut(&mut a, i) = 9; a }),
        });
        let expected = json!({ "list": e["list"], "array": e["array"], "list_set": e["list_set"], "array_set": e["array_set"] });
        failures.check(|| format!("index {i} of {}", e["input"]), &expected, &actual);
    }
    failures.finish("indexers");
}

#[test]
fn arithmetic() {
    let mut failures = Failures::default();
    for e in golden("linq.json")["arithmetic"].as_array().unwrap() {
        let (a, b) = (int(&e["a"]), int(&e["b"]));
        let actual = json!({ "div": try_enc(|| cs::div(a, b)), "rem": try_enc(|| cs::rem(a, b)) });
        failures.check(|| format!("{a} / {b}"), &json!({ "div": e["div"], "rem": e["rem"] }), &actual);
    }
    failures.finish("int arithmetic");
}

#[test]
fn dictionary_misses() {
    let mut failures = Failures::default();
    for e in golden("linq.json")["dictionary"].as_array().unwrap() {
        let key = match e["key"].as_str().unwrap() {
            "int 5" => 5.to_string(),
            "int -1" => (-1).to_string(),
            "Faction.Atreides" => Faction::Atreides.to_string(),
            "Faction 13" => Faction(13).to_string(),
            "string abc" => "abc".to_string(),
            other => panic!("linq.json has dictionary key {other:?}, which this test does not know"),
        };
        failures.check(|| format!("key {key}"), &e["result"], &try_enc(|| -> i32 { cs::key_not_found(&key) }));
    }
    failures.finish("dictionary misses");
}

fn thrown(type_name: &'static str, message: String) -> CsException {
    CsException { type_name, message }
}

/// The exception each constructor call of LinqTable.cs produces in Rust.
#[test]
fn exception_messages() {
    let mut failures = Failures::default();
    for e in golden("linq.json")["exceptions"].as_array().unwrap() {
        let what = e["what"].as_str().unwrap();
        let actual = match what {
            "ArgumentOutOfRangeException(param)" => thrown(
                "ArgumentOutOfRangeException",
                cs::with_parameter(&cs::default_message("ArgumentOutOfRangeException"), "faction"),
            ),
            "ArgumentOutOfRangeException(param, Faction 13, null)" => {
                cs::catch_panic(|| cs::argument_out_of_range_actual("faction", &Faction(13).to_string())).unwrap_err()
            }
            "ArgumentOutOfRangeException(param, Faction.Atreides, null)" => {
                cs::catch_panic(|| cs::argument_out_of_range_actual("faction", &Faction::Atreides.to_string())).unwrap_err()
            }
            "ArgumentOutOfRangeException(param, int -5, msg)" => {
                thrown("ArgumentOutOfRangeException", format!("{}\nActual value was -5.", cs::with_parameter("must be positive", "n")))
            }
            "ArgumentOutOfRangeException(param, msg)" => {
                thrown("ArgumentOutOfRangeException", cs::with_parameter("must be positive", "n"))
            }
            "ArgumentNullException(param)" => {
                thrown("ArgumentNullException", cs::with_parameter(&cs::default_message("ArgumentNullException"), "source"))
            }
            "ArgumentException(msg, param)" => thrown("ArgumentException", cs::with_parameter("bad", "x")),
            "ArgumentException(msg)" => cs::catch_panic(|| cs::throw("ArgumentException", "bad".to_string())).unwrap_err(),
            "Exception(\"\")" => thrown("Exception", String::new()),
            parameterless => {
                // `throw new T()`, through the throw! macro.
                let type_name = parameterless.strip_suffix("()").unwrap_or_else(|| {
                    panic!("linq.json has exception {parameterless:?}, which this test does not know")
                });
                let e = cs::catch_panic(|| cs::throw(leak(type_name), cs::default_message(type_name))).unwrap_err();
                assert_eq!(
                    cs::catch_panic(|| match type_name {
                        "NullReferenceException" => cs::throw("NullReferenceException", cs::default_message("NullReferenceException")),
                        "InvalidOperationException" => cs::throw("InvalidOperationException", cs::default_message("InvalidOperationException")),
                        "ArgumentException" => cs::throw("ArgumentException", cs::default_message("ArgumentException")),
                        "Exception" => cs::throw("Exception", cs::default_message("Exception")),
                        _ => cs::throw(leak(type_name), cs::default_message(type_name)),
                    })
                    .unwrap_err(),
                    e,
                    "throw!({type_name}) without a message"
                );
                e
            }
        };
        failures.check(|| what.to_string(), &e["result"], &exception(&actual));
    }
    failures.finish("exception messages");
}

fn leak(s: &str) -> &'static str {
    Box::leak(s.to_string().into_boxed_str())
}
