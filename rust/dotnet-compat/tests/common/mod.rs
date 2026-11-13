//! Runs the cases of the golden files in Rust.
//!
//! A *case* is a JSON object naming a compat type and a sequence of operations on it (see
//! `run_rust`). Running it gives one JSON result. Doubles are passed as their bit patterns
//! (i64), floats as i32 bit patterns.

#![allow(dead_code)]

use dotnet_compat::math::{self, MidpointRounding};
use dotnet_compat::parse;
use dotnet_compat::CsException;
use dotnet_compat::{NetDictionary, NetHashSet, NetRandom};
use serde_json::{Value, json};
use std::cmp::Ordering;
use std::panic::{AssertUnwindSafe, catch_unwind};

fn ints(v: &Value) -> Vec<i32> {
    v.as_array().unwrap().iter().map(|x| x.as_i64().unwrap() as i32).collect()
}
fn longs(v: &Value) -> Vec<i64> {
    v.as_array().unwrap().iter().map(|x| x.as_i64().unwrap()).collect()
}
fn bd(x: f64) -> Value {
    json!(x.to_bits() as i64)
}
fn bf(x: f32) -> Value {
    json!(x.to_bits() as i32)
}

/// Runs `f`; a panic that stands for a .NET exception becomes `"Type: message"`.
/// Any other panic is a bug and becomes "RUST PANIC: ...".
fn catching(f: impl FnOnce() -> Value) -> Value {
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(v) => v,
        Err(e) => {
            let msg = dotnet_compat::exception::panic_message(e.as_ref());
            // .NET exceptions panic with "SomeException: message".
            match msg.split_once(": ") {
                Some((ty, _)) if ty.ends_with("Exception") => json!(msg),
                _ => json!(format!("RUST PANIC: {msg}")),
            }
        }
    }
}

pub fn run_rust(case: &Value) -> Value {
    match case["t"].as_str().unwrap() {
        "random" => rust_random(case),
        "dict" => rust_dict(case),
        "set" => rust_set(case),
        "math" => rust_math(case),
        t => panic!("unknown case {t}"),
    }
}

fn rust_random(case: &Value) -> Value {
    let mut r = NetRandom::new(case["seed"].as_i64().unwrap() as i32);
    let mut out = Vec::new();
    for op in case["ops"].as_array().unwrap() {
        let o = ints(op);
        let v = match o[0] {
            0 => json!(r.next()),
            1 => catching(|| json!(r.next_max(o[1]))),
            2 => catching(|| json!(r.next_range(o[1], o[2]))),
            _ => bd(r.next_double()),
        };
        out.push(v);
    }
    Value::Array(out)
}

fn snap_dict(d: &NetDictionary<i32, i32>) -> Value {
    Value::Array(d.iter().flat_map(|(k, v)| [json!(k), json!(v)]).collect())
}

fn rust_dict(case: &Value) -> Value {
    let mut d: NetDictionary<i32, i32> = NetDictionary::new();
    let mut out = Vec::new();
    for op in case["ops"].as_array().unwrap() {
        let o = ints(op);
        let (k, v) = (o[1], o[2]);
        match o[0] {
            0 => d.insert(k, v),
            1 => out.push(catching(|| {
                d.add(k, v);
                json!(1)
            })),
            2 => out.push(json!(d.try_add(k, v) as i32)),
            3 => out.push(json!(d.remove(&k).is_some() as i32)),
            4 => d.clear(),
            5 => out.push(snap_dict(&d)),
            6 => d = NetDictionary::from_dictionary(&d),
            7 => d = NetDictionary::to_dictionary(d.iter(), |kv| *kv.0, |kv| *kv.1),
            8 => out.push(json!(d.contains_value(&v) as i32)),
            9 => out.push(json!(d.get(&k).copied().unwrap_or(-1))),
            10 => out.push(json!(d.get_value_or_default(&k))),
            11 => out.push(catching(|| json!(d[&k]))),
            _ => unreachable!(),
        }
    }
    out.push(snap_dict(&d));
    Value::Array(out)
}

fn snap_set(s: &NetHashSet<i32>) -> Value {
    Value::Array(s.iter().map(|x| json!(x)).collect())
}

fn rust_set(case: &Value) -> Value {
    let mut s: NetHashSet<i32> = NetHashSet::new();
    let mut out = Vec::new();
    for op in case["ops"].as_array().unwrap() {
        let code = op[0].as_i64().unwrap();
        let k = op[1].as_i64().unwrap() as i32;
        let xs = ints(&op[2]);
        match code {
            0 => out.push(json!(s.add(k) as i32)),
            1 => out.push(json!(s.remove(&k) as i32)),
            2 => s.clear(),
            3 => out.push(snap_set(&s)),
            4 => s = NetHashSet::from_set(&s),
            5 => s = NetHashSet::from_collection(&xs),
            6 => s = NetHashSet::from_enumerable(xs.iter().copied()),
            7 => s.union_with(xs.iter().copied()),
            8 => s.except_with(&xs),
            9 => s.intersect_with_collection(&xs),
            10 => s.intersect_with_enumerable(&xs),
            11 => s.intersect_with_set(&NetHashSet::from_collection(&xs)),
            12 => s.symmetric_except_with(xs.iter().copied()),
            13 => s.symmetric_except_with_set(&NetHashSet::from_collection(&xs)),
            14 => out.push(json!(s.remove_where(|x| x % k == 0))),
            15 => s.trim_excess(),
            16 => s = NetHashSet::with_capacity(k as usize),
            17 => out.push(json!(s.contains(&k) as i32)),
            _ => unreachable!(),
        }
    }
    out.push(snap_set(&s));
    Value::Array(out)
}

#[derive(Clone, Copy)]
struct Item {
    key: i32,
    id: i32,
}
impl PartialEq for Item {
    fn eq(&self, o: &Self) -> bool {
        self.key == o.key
    }
}
impl Eq for Item {}
impl PartialOrd for Item {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for Item {
    fn cmp(&self, o: &Self) -> Ordering {
        self.key.cmp(&o.key)
    }
}

fn ints_v(xs: impl IntoIterator<Item = i32>) -> Value {
    Value::Array(xs.into_iter().map(|x| json!(x)).collect())
}

const MODES: [MidpointRounding; 5] = [
    MidpointRounding::ToEven,
    MidpointRounding::AwayFromZero,
    MidpointRounding::ToZero,
    MidpointRounding::ToNegativeInfinity,
    MidpointRounding::ToPositiveInfinity,
];

/// The enum the parse cases use: duplicate names in other case, a negative value, flags.
const TEST_ENUM: &[(&str, i32)] = &[
    ("None", 0),
    ("Atreides", 1),
    ("Harkonnen", 2),
    ("Emperor", 4),
    ("Fremen", 8),
    ("Guild", 16),
    ("fremen", 32),
    ("Negative", -5),
];

fn rust_math(case: &Value) -> Value {
    let op = case["op"].as_str().unwrap();
    let arr = &case["xs"];
    let mut out = Vec::new();
    match op {
        "round" => {
            for bits in longs(arr) {
                let x = f64::from_bits(bits as u64);
                let mut row = vec![bd(math::round_f64(x))];
                for m in MODES {
                    row.push(bd(math::round_f64_mode(x, m)));
                    for digits in [0, 1, 2, 5, 15] {
                        row.push(bd(math::round_f64_digits_mode(x, digits, m)));
                    }
                }
                row.push(bd(math::round_f64_digits(x, 1)));
                row.push(bf(math::round_f32(x as f32)));
                row.push(bd(x.ceil()));
                row.push(bd(x.floor()));
                row.push(json!(math::f64_to_i32(x)));
                row.push(json!(math::f32_to_i32(x as f32)));
                row.push(json!(x as i64));
                out.push(Value::Array(row));
            }
        }
        "fmtd" => {
            for bits in longs(arr) {
                let s = math::format_f64(f64::from_bits(bits as u64));
                out.push(json!([s, s]));
            }
        }
        "fmtf" => {
            for bits in longs(arr) {
                let s = math::format_f32(f32::from_bits(bits as i32 as u32));
                out.push(json!([s, s]));
            }
        }
        "parse" => {
            fn ex<T>(r: Result<T, CsException>, f: impl Fn(T) -> Value) -> Value {
                match r {
                    Ok(v) => f(v),
                    Err(e) => json!(e.to_string()),
                }
            }
            for s in arr.as_array().unwrap() {
                let s = s.as_str();
                out.push(json!([
                    ex(parse::try_int_parse(s), |v| json!(v)),
                    ex(parse::try_convert_to_int32(s), |v| json!(v)),
                    ex(parse::try_float_parse_invariant(s), bf),
                    ex(parse::try_double_parse_invariant(s), bd),
                    ex(parse::try_enum_parse(s, TEST_ENUM), |v| json!(v)),
                ]));
            }
        }
        "cmpd" => {
            let xs = longs(arr);
            for p in xs.chunks_exact(2) {
                let (a, b) = (f64::from_bits(p[0] as u64), f64::from_bits(p[1] as u64));
                out.push(json!([
                    match math::cmp_f64(a, b) {
                        Ordering::Less => -1,
                        Ordering::Equal => 0,
                        Ordering::Greater => 1,
                    },
                    math::equals_f64(a, b),
                    bd(math::math_max_f64(a, b)),
                    bd(math::math_min_f64(a, b)),
                    bf(math::math_max_f32(a as f32, b as f32)),
                    bf(math::math_min_f32(a as f32, b as f32)),
                ]));
            }
        }
        "rounddigits" => {
            for d in ints(arr) {
                out.push(catching(|| bd(math::round_f64_digits(1.25, d))));
                out.push(catching(|| bd(math::round_f64_digits_mode(1.25, d, MidpointRounding::AwayFromZero))));
            }
        }
        "abs" => {
            for x in ints(arr) {
                out.push(catching(|| json!(math::abs_i32(x))));
            }
        }
        _ => panic!("unknown math op {op}"),
    }
    Value::Array(out)
}

/// Compares the Rust results with the expected ones; returns a description of the first
/// few mismatches.
pub fn compare(cases: &[Value], expected: &[Value]) -> Vec<String> {
    assert_eq!(cases.len(), expected.len());
    // Expected .NET exceptions are Rust panics; keep them off stderr.
    std::panic::set_hook(Box::new(|_| {}));
    let mut failures = Vec::new();
    for (case, exp) in cases.iter().zip(expected) {
        let got = catch_unwind(AssertUnwindSafe(|| run_rust(case))).unwrap_or_else(|_| json!("RUST PANIC"));
        if &got != exp {
            failures.push(describe_mismatch(case, &got, exp));
            if failures.len() >= 5 {
                break;
            }
        }
    }
    let _ = std::panic::take_hook();
    failures
}

fn describe_mismatch(case: &Value, got: &Value, exp: &Value) -> String {
    let mut detail = String::new();
    if let (Some(g), Some(e)) = (got.as_array(), exp.as_array()) {
        for (i, (a, b)) in g.iter().zip(e).enumerate() {
            if a != b {
                detail = format!("first difference at result index {i}: now {a}, expected {b}");
                if let Some(xs) = case["xs"].as_array() {
                    if let Some(input) = xs.get(i) {
                        detail.push_str(&format!(" (input {input})"));
                    }
                }
                break;
            }
        }
        if detail.is_empty() {
            detail = format!("length now {}, expected {}", g.len(), e.len());
        }
    } else {
        detail = format!("now {got}, expected {exp}");
    }
    let mut case_s = case.to_string();
    if case_s.len() > 600 {
        case_s.truncate(600);
        case_s.push_str("...");
    }
    format!("{detail}\n  case: {case_s}")
}
