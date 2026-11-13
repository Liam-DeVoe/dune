//! Shared harness for the golden and differential tests.
//!
//! A *case* is a JSON object understood by both `csharp-ref/Program.cs` and
//! [`run_rust`]. Both produce one JSON result per case; the results must be
//! identical. Doubles are passed as their bit patterns (i64), floats as i32
//! bit patterns.

#![allow(dead_code)]

use dotnet_compat::math::{self, MidpointRounding};
use dotnet_compat::parse;
use dotnet_compat::CsException;
use dotnet_compat::{NetDictionary, NetHashSet, NetRandom};
use serde_json::{Value, json};
use std::cmp::Ordering;
use std::io::Write;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;
use std::process::{Command, Stdio};

// ---------------------------------------------------------------------------
// Deterministic generator
// ---------------------------------------------------------------------------

pub struct Gen(u64);

impl Gen {
    pub fn new(seed: u64) -> Self {
        Gen(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xD1B5_4A32_D192_ED03 | 1)
    }
    pub fn next_u64(&mut self) -> u64 {
        // splitmix64
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    /// Uniform in [0, n).
    pub fn below(&mut self, n: u64) -> u64 {
        if n == 0 { 0 } else { self.next_u64() % n }
    }
    pub fn range(&mut self, lo: i64, hi_inclusive: i64) -> i64 {
        lo + self.below((hi_inclusive - lo + 1) as u64) as i64
    }
    pub fn pick<T: Copy>(&mut self, xs: &[T]) -> T {
        xs[self.below(xs.len() as u64) as usize]
    }
    pub fn chance(&mut self, p: f64) -> bool {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64 <= p
    }
    pub fn i32(&mut self) -> i32 {
        self.next_u64() as i32
    }
}

// ---------------------------------------------------------------------------
// Case generators
// ---------------------------------------------------------------------------

pub const EDGE_SEEDS: &[i32] = &[
    0, 1, -1, 2, -2, 42, 12345, i32::MIN, i32::MIN + 1, i32::MAX, i32::MAX - 1, 161803398, -161803398, 161803397,
    161803399, -2147483647, 1 << 30, -(1 << 30),
];

fn random_op(g: &mut Gen) -> Value {
    match g.below(10) {
        0 | 1 => json!([0, 0, 0]),
        2 | 3 | 4 => {
            let max = match g.below(8) {
                0 => 0,
                1 => 1,
                2 => g.range(2, 10) as i32,
                3 => g.range(0, 1000) as i32,
                4 => i32::MAX,
                5 => g.i32().abs().max(0),
                6 if g.chance(0.2) => -(g.range(1, 5) as i32),
                _ => g.range(1, 60) as i32,
            };
            json!([1, max, 0])
        }
        5 | 6 | 7 => {
            let (a, b) = match g.below(7) {
                0 => (i32::MIN, i32::MAX),
                1 => (g.i32(), g.i32()),
                2 => (-(g.range(0, 100) as i32), g.range(0, 100) as i32),
                3 => (i32::MIN, g.i32()),
                4 => (g.i32(), i32::MAX),
                5 => {
                    let x = g.i32();
                    (x, x)
                }
                _ => (g.range(-10, 10) as i32, g.range(-10, 10) as i32),
            };
            // Mostly valid ranges; occasionally min > max to check the exception.
            let (a, b) = if a > b && !g.chance(0.05) { (b, a) } else { (a, b) };
            json!([2, a, b])
        }
        _ => json!([3, 0, 0]),
    }
}

pub fn random_case(g: &mut Gen, seed: i32, n_ops: usize) -> Value {
    let ops: Vec<Value> = (0..n_ops).map(|_| random_op(g)).collect();
    json!({"t": "random", "seed": seed, "ops": ops})
}

pub fn dict_case(g: &mut Gen, n_ops: usize) -> Value {
    let keys = g.pick(&[3i64, 8, 30, 200]);
    let mut ops = Vec::new();
    for _ in 0..n_ops {
        let k = g.below(keys as u64) as i32;
        let v = g.range(-3, 50) as i32;
        let code = match g.below(100) {
            0..=19 => 0,
            20..=34 => 1,
            35..=44 => 2,
            45..=69 => 3,
            70..=71 => 4,
            72..=81 => 5,
            82..=84 => 6,
            85..=86 => 7,
            87..=90 => 8,
            91..=94 => 9,
            95..=96 => 11,
            _ => 10,
        };
        ops.push(json!([code, k, v]));
    }
    json!({"t": "dict", "ops": ops})
}

pub fn set_case(g: &mut Gen, n_ops: usize) -> Value {
    let keys = g.pick(&[3i64, 8, 30, 200]);
    let mut ops = Vec::new();
    for _ in 0..n_ops {
        let code = match g.below(100) {
            0..=29 => 0,
            30..=49 => 1,
            50..=51 => 2,
            52..=59 => 3,
            60..=63 => 4,
            64..=65 => 5,
            66..=67 => 6,
            68..=70 => 7,
            71..=73 => 8,
            74..=76 => 9,
            77..=79 => 10,
            80..=82 => 11,
            83..=85 => 12,
            86..=88 => 13,
            89..=91 => 14,
            92..=93 => 15,
            94 => 16,
            _ => 17,
        };
        let k = match code {
            14 => g.range(1, 5) as i32,
            16 => g.pick(&[0, 1, 3, 4, 10, 50]),
            _ => g.below(keys as u64) as i32,
        };
        let len = if g.chance(0.15) { 0 } else if g.chance(0.2) { g.range(10, 40) as usize } else { g.range(1, 10) as usize };
        let xs: Vec<i32> = (0..len).map(|_| g.below(keys as u64) as i32).collect();
        ops.push(json!([code, k, xs]));
    }
    json!({"t": "set", "ops": ops})
}

pub const INTERESTING_F32: &[f32] = &[
    0.0, -0.0, 1.0, -1.0, 0.5, -0.5, 1.5, 2.5, 3.0, 100.25, f32::NAN, f32::INFINITY, f32::NEG_INFINITY, f32::MAX,
    f32::MIN, f32::MIN_POSITIVE, 1e-45, 16777216.0, 16777217.0, 0.1, 0.2, 0.3,
];

pub const INTERESTING_F64: &[f64] = &[
    0.0, -0.0, 0.5, -0.5, 1.5, -1.5, 2.5, -2.5, 0.49999999999999994, -0.49999999999999994, 0.5000000000000001,
    1.0000000000000002, 4503599627370495.5, 4503599627370496.0, 4503599627370497.0, 9007199254740993.0, 1e15, 1e16,
    1e17, 9.999999999999999e15, 1e-5, 1e-4, 0.0001, 0.00001, 123456789.0, 1234567.0, 12345678.0, 2.675, 1.005,
    0.125, 0.3, 0.1 + 0.2, f64::NAN, f64::INFINITY, f64::NEG_INFINITY, f64::MAX, f64::MIN, f64::MIN_POSITIVE, 5e-324,
    2147483647.0, 2147483648.0, -2147483648.0, -2147483649.0, 2147483647.5, 1e10, -1e10, 9.2e18, 1e20, 1e100, -7.5,
    100.0, 1e14, 123456789012345.0, 1234567890123456.0, 0.001, 0.0012345,
];

fn random_double(g: &mut Gen) -> f64 {
    match g.below(8) {
        0 => g.pick(INTERESTING_F64),
        1 => f64::from_bits(g.next_u64()),
        2 => g.range(-40, 40) as f64 / 2.0,
        3 => g.range(-100000, 100000) as f64 / 1000.0,
        4 => g.range(-1000, 1000) as f64 / 8.0 + if g.chance(0.5) { 0.0 } else { 1e-9 },
        5 => (g.range(-1_000_000, 1_000_000) as f64) * 10f64.powi(g.range(-25, 25) as i32),
        6 => g.range(-100, 100) as f64 / 100.0,
        _ => g.range(-1 << 40, 1 << 40) as f64 + 0.5,
    }
}

fn random_float(g: &mut Gen) -> f32 {
    match g.below(6) {
        0 => g.pick(INTERESTING_F32),
        1 => f32::from_bits(g.next_u64() as u32),
        2 => g.range(-40, 40) as f32 / 2.0,
        3 => (g.range(-1_000_000, 1_000_000) as f32) * 10f32.powi(g.range(-12, 12) as i32),
        4 => random_double(g) as f32,
        _ => g.range(-1000, 1000) as f32 / 10.0,
    }
}

const PARSE_POOL: &[&str] = &[
    "", " ", "0", "-0", "+0", "1", "-1", "+1", " 1", "1 ", " \t1\n ", "1\0", "1\0\0", "1 \0", "1\0 ", "\01", "- 1",
    "+-1", "--1", "1-", "2147483647", "2147483648", "-2147483648", "-2147483649", "99999999999", "99999999999x",
    "00000000000000000000001", "1.5", "1,5", "1,000", ",1", "1,", "1,,2", "1.", ".5", "-.5", ".", "-", "+", "1e5",
    "1E5", "1e+5", "1e-5", "1e", "1e+", "1.e5", "1e1001", "1e99999", "0e99999", "1e-99999", "NaN", "nan", "-NaN",
    "+NaN", "Infinity", "-Infinity", "+Infinity", "infinity", " Infinity ", "∞", "1.5.5", "1,000.25", "1.000,25",
    "3.4028235e38", "3.4028236e38", "1e39", "1.401298464324817e-45", "7e-46", "0x10", "1_000", "１", "1e5.5", "(1)",
    "0.1", "16777217", "9007199254740993", "4.9e-324", "2.4703282292062328e-324",
];

pub fn parse_strings(g: &mut Gen, n: usize) -> Vec<String> {
    let alphabet: &[char] = &[' ', '\t', '+', '-', '.', ',', 'e', 'E', '0', '1', '5', '9', '\0', 'N', 'a', 'I', 'x'];
    (0..n)
        .map(|_| {
            if g.chance(0.3) {
                g.pick(PARSE_POOL).to_string()
            } else {
                let len = g.range(0, 8) as usize;
                (0..len).map(|_| g.pick(alphabet)).collect()
            }
        })
        .collect()
}

pub fn enum_strings(g: &mut Gen, n: usize) -> Vec<String> {
    let atoms: &[&str] = &[
        "None", "Atreides", "Harkonnen", "Emperor", "Fremen", "fremen", "FREMEN", "Guild", "Negative", "BeneGesserit", "1", "-3",
        "12345", "2147483648", "+2", "x", "", "0x1", "Atreides\0",
    ];
    let seps: &[&str] = &[",", ", ", " ,", " , ", ",,", " ", "\t,"];
    (0..n)
        .map(|_| {
            let mut s = String::new();
            if g.chance(0.3) {
                s.push_str(g.pick(&[" ", "  ", "\t", "\n"]));
            }
            for i in 0..g.range(1, 3) {
                if i > 0 {
                    s.push_str(g.pick(seps));
                }
                s.push_str(g.pick(atoms));
            }
            if g.chance(0.3) {
                s.push_str(g.pick(&[" ", "\0", "\t", ","]));
            }
            s
        })
        .collect()
}

pub fn math_cases(g: &mut Gen, n: usize) -> Vec<Value> {
    let mut cases = Vec::new();
    let doubles: Vec<i64> = INTERESTING_F64
        .iter()
        .copied()
        .chain((0..n).map(|_| random_double(g)))
        .map(|x| x.to_bits() as i64)
        .collect();
    cases.push(json!({"t": "math", "op": "round", "xs": doubles}));
    cases.push(json!({"t": "math", "op": "fmtd", "xs": doubles}));
    let floats: Vec<i64> = INTERESTING_F32
        .iter()
        .copied()
        .chain((0..n).map(|_| random_float(g)))
        .map(|x| x.to_bits() as i32 as i64)
        .collect();
    cases.push(json!({"t": "math", "op": "fmtf", "xs": floats}));
    let mut strs: Vec<Value> = PARSE_POOL.iter().map(|s| json!(s)).collect();
    strs.push(Value::Null);
    strs.extend(parse_strings(g, n).into_iter().map(|s| json!(s)));
    strs.extend(enum_strings(g, n / 2).into_iter().map(|s| json!(s)));
    strs.extend((0..n / 4).map(|_| json!(math::format_f64(random_double(g)))));
    strs.extend((0..n / 4).map(|_| json!(math::format_f32(random_float(g)))));
    cases.push(json!({"t": "math", "op": "parse", "xs": strs}));
    let pairs: Vec<i64> = (0..2 * n)
        .map(|_| if g.chance(0.5) { g.pick(INTERESTING_F64) } else { random_double(g) })
        // NaN payloads are platform-specific in Math.Max/Min; use quiet NaNs.
        .map(|x| if x.is_nan() { x.to_bits() | 0x0008_0000_0000_0000 } else { x.to_bits() } as i64)
        .collect();
    cases.push(json!({"t": "math", "op": "cmpd", "xs": pairs}));
    let ints: Vec<i32> =
        [0, 1, -1, i32::MIN, i32::MAX, i32::MIN + 1].into_iter().chain((0..20).map(|_| g.i32())).collect();
    cases.push(json!({"t": "math", "op": "abs", "xs": ints}));
    cases.push(json!({"t": "math", "op": "rounddigits", "xs": (-3..20).collect::<Vec<i32>>()}));
    cases
}

// ---------------------------------------------------------------------------
// Rust executor
// ---------------------------------------------------------------------------

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

/// Runs `f`; a panic becomes the string `tag` (C# exception).
/// Runs `f`; a C# exception becomes `"Type: message"`, as the C# side
/// prints it. Any other panic is a bug and becomes "RUST PANIC: ...".
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

/// `TestEnum` in csharp-ref/Program.cs.
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

// ---------------------------------------------------------------------------
// C# oracle
// ---------------------------------------------------------------------------

pub fn csharp_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("csharp-ref")
}

fn dotnet() -> Command {
    let home = std::env::var("HOME").unwrap();
    let root = std::env::var("DOTNET_ROOT").unwrap_or_else(|_| format!("{home}/dotnet10"));
    let mut c = Command::new(format!("{root}/dotnet"));
    c.env("DOTNET_ROOT", &root).env("DOTNET_CLI_TELEMETRY_OPTOUT", "1").env("DOTNET_NOLOGO", "1");
    c
}

/// Builds the reference program once per test process.
pub fn build_csharp() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let status = dotnet()
            .args(["build", "-c", "Release", "-v", "q", "-nologo"])
            .current_dir(csharp_dir())
            .stdout(Stdio::null())
            .status()
            .expect("failed to run dotnet build");
        assert!(status.success(), "dotnet build of csharp-ref failed");
    });
}

/// Runs all `cases` through the C# program in one process.
pub fn run_csharp(cases: &[Value]) -> Vec<Value> {
    build_csharp();
    let dll = csharp_dir().join("bin/Release/net10.0/csharp-ref.dll");
    let mut child = dotnet()
        .arg(dll)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("failed to start csharp-ref");
    let input = serde_json::to_vec(cases).unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let writer = std::thread::spawn(move || stdin.write_all(&input).unwrap());
    let output = child.wait_with_output().unwrap();
    writer.join().unwrap();
    assert!(output.status.success(), "csharp-ref failed");
    let v: Value = serde_json::from_slice(&output.stdout).expect("csharp-ref produced invalid JSON");
    v.as_array().unwrap().clone()
}

/// Compares Rust against expected results; returns a description of the first
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
                detail = format!("first difference at result index {i}: rust {a} vs .NET {b}");
                if let Some(xs) = case["xs"].as_array() {
                    if let Some(input) = xs.get(i) {
                        detail.push_str(&format!(" (input {input})"));
                    }
                }
                break;
            }
        }
        if detail.is_empty() {
            detail = format!("length rust {} vs .NET {}", g.len(), e.len());
        }
    } else {
        detail = format!("rust {got} vs .NET {exp}");
    }
    let mut case_s = case.to_string();
    if case_s.len() > 600 {
        case_s.truncate(600);
        case_s.push_str("...");
    }
    format!("{detail}\n  case: {case_s}")
}
