//! `db::date_time::parse` against what the C# server reads from a `DateTimeOffset` column:
//! EF Core 10.0.12 on Microsoft.Data.Sqlite, through the H1 probe's `dates` mode, which stores
//! each text in `Users.LastLogin` with a raw INSERT and reads the entity back.
//!
//! - `golden`: the hand-picked texts and 1,000 generated ones with the values .NET read
//!   (`tests/golden/ef_dates.jsonl`), each under several `TZ` settings. Runs without .NET.
//! - `against_dotnet` (ignored): generates `H1_DATES_N` texts (default 50,000) from seed
//!   `H1_DATES_SEED` (default 1) and compares them under each time zone in `TZ_SET`. With
//!   `H1_WRITE_GOLDEN=1` it rewrites the golden file from seed 1. Run it with
//!   `rust/tools/heavy.sh cargo test --release -p treachery-server --test db_dates -- --ignored`.
//!
//! What the probe shows: text without an offset reads as UTC (`+00:00`) under every `TZ`, the
//! same as `DateTimeOffset.Parse(text, InvariantCulture, DateTimeStyles.AssumeUniversal)`.
//! The machine's time zone plays no part, so the Rust server needs none either.

mod h1_probe;

use h1_probe::Rng;
use serde_json::{json, Value};
use treachery_server::db::date_time;

const GOLDEN_GENERATED: usize = 1000;

/// Three zones: UTC (the servers), one with daylight saving time, one with a half-hour offset.
/// If .NET used the local zone anywhere, the results would differ between them.
const TZ_SET: &[&str] = &["UTC", "America/New_York", "Asia/Kolkata"];

const HAND_PICKED: &[&str] = &[
    // What EF writes.
    "2026-09-19 19:51:27.837212-04:00",
    "0001-01-01 00:00:00+00:00",
    "2024-02-29 23:59:59.9999999+14:00",
    // sqlite3: datetime('now'), datetime('now', 'subsec'), date('now'), strftime('%Y-%m-%dT%H:%M:%fZ').
    "2026-09-17 19:30:14",
    "2026-09-17 19:30:14.123",
    "2026-09-17",
    "2026-09-17T19:30:14.123Z",
    // deploy/provision.sh: datetime('now') || '+00:00'.
    "2026-09-17 19:30:14+00:00",
    // JavaScript toISOString, Python isoformat, Go RFC 3339 nano, .NET "o".
    "2026-09-17T19:30:14.000Z",
    "2026-09-17T19:30:14.123456+00:00",
    "2026-09-17T19:30:14.123456789Z",
    "2026-09-17T19:30:14.1234567-07:00",
    // PostgreSQL text.
    "2026-09-17 19:30:14.123+02",
    // Other ISO shapes.
    "2026-09-17T19:30:14",
    "2026-09-17T19:30:14Z",
    "2026-09-17t19:30:14z",
    "2026-09-17 19:30:14 +00:00",
    "2026-09-17 19:30:14Z",
    "2026-09-17 19:30:14 Z",
    "2026-09-17 19:30:14+0000",
    "2026-09-17T19:30:14.123+0200",
    "2026-09-17 19:30:14-00:00",
    "2026-09-17 19:30:14+05:30",
    "2026-09-17 19:30:14-13:59",
    "2026-09-17 19:30:14+14:00",
    "2026-09-17 19:30:14-14:00",
    "2026-09-17 19:30",
    "2026-09-17T19:30Z",
    "2026-09-17T19:30+02:00",
    "2026-09-17Z",
    "2026-09-17 +02:00",
    "2026-09-17+02:00",
    "2026-09-17 ",
    // Whitespace.
    " 2026-09-17 19:30:14 ",
    "2026-09-17 19:30:14\n",
    "2026-09-17T19:30:14\r\n",
    "\t2026-09-17 19:30:14+00:00",
    "2026-09-17  19:30:14",
    "2026-09-17\t19:30:14",
    "2026-09-17T19:30:14  +01:00",
    // Fractions past seven digits are rounded, and can carry.
    "2026-09-17 19:30:14.12345678",
    "2026-09-17 19:30:14.12345665",
    "2026-09-17 19:30:14.12345675",
    "2026-09-17 19:30:14.00000005",
    "2026-09-17 19:30:14.000000049999",
    "2026-09-17 19:30:14.99999999",
    "2026-09-17 23:59:59.99999999",
    "2026-12-31T23:59:59.99999999Z",
    "9999-12-31 23:59:59.99999999",
    "2026-09-17 19:30:14.0000000000",
    // Ranges.
    "2026-09-17 24:00:00",
    "2026-09-17 23:59:60",
    "2026-09-17 23:60:00",
    "2026-02-29 00:00:00",
    "2024-02-29 00:00:00",
    "2026-13-01 00:00:00",
    "2026-00-10 00:00:00",
    "0000-01-01 00:00:00",
    "0001-01-01 00:00:00",
    "0001-01-01 00:00:00-01:00",
    "0001-01-01 00:00:00+01:00",
    "9999-12-31 23:59:59",
    "9999-12-31 23:59:59-01:00",
    "9999-12-31T23:59:59.9999999Z",
    "2026-09-17 19:30:14+14:01",
    "2026-09-17 19:30:14+15:00",
    "2026-09-17 19:30:14+13:60",
    "2026-09-17 19:30:14+99",
    // Daylight saving time edges in New York and London: no-offset text is UTC regardless.
    "2026-01-15 12:00:00",
    "2026-07-01 12:00:00",
    "2026-03-08 02:30:00",
    "2026-11-01 01:30:00",
    "2026-03-29 01:30:00",
    "2026-10-25 01:30:00",
    // Refused by both.
    "",
    " ",
    "now",
    "1789000000",
    "2461300.5",
    "2026-09-17T",
    "2026-09-17T19:30:14.",
    "2026-09-17T19:30:14.Z",
    "2026-09-17 19",
    "2026-09-17 19:30.5",
    "2026-09-17 19:30:14 UTC",
    "2026-09-17T19:30:14+00:00Z",
    "2026-09-17 19:30:14+05:30:00",
    "2026-09-17T9:03:04",
    "20260917T193014Z",
    "26-09-17 19:30:14",
    "17/09/2026",
    "2026-W38-4",
    "2026-260",
    "2026-09-17 19-30-14",
    "2026-09-17 019:30:14",
    "2026-009-17",
];

/// Texts .NET accepts and `parse` refuses on purpose: .NET's general date parser takes far
/// more than ISO 8601, and nothing that writes to this database produces these.
const KNOWN_REFUSED: &[&str] = &[
    "2026-09-17 19:30:14 GMT",
    "2026-09-17 19:30:14GMT",
    "2026-09-17 19:30:14+5:30",
    "2026-09-17 19:30:14+530",
    "2026-09-17 19:30:14+1",
    "2026-09-17 19:30:14+100",
    "2026-09-17 19:30:14+01:5",
    "2026-09-17 9:03",
    "2026-09-17 19:30:4",
    "2026-9-7 9:3:4",
    "2026-9-17T19:30:14Z",
    "2026-09-17T 19:30",
    "2026/09/17 19:30:14",
    "2026.09.17 19:30:14",
    "2026-09-17 7:30:14 PM",
    "02026-09-17 19:30:14",
    "09/17/2026 19:30:14",
    "Sep 17 2026",
    "Thu, 17 Sep 2026 19:30:14 GMT",
    "2026-09-17 19:30:14,5",
    "2026-09-17 19:30:14.",
    "2026-09-17 19:30:14.Z",
];

fn pick<'a>(rng: &mut Rng, items: &[&'a str]) -> &'a str {
    items[rng.below(items.len())]
}

fn two(rng: &mut Rng, below: usize, edge: &[usize]) -> String {
    let v = if rng.chance(10) { edge[rng.below(edge.len())] } else { rng.below(below) };
    format!("{v:02}")
}

/// A text in the shape `parse` documents, with values that may be out of range.
fn generated(rng: &mut Rng) -> String {
    let mut s = String::new();
    if rng.chance(10) {
        s.push_str(pick(rng, &[" ", "\t", "\n", "  "]));
    }
    let year = match rng.below(6) {
        0 => pick(rng, &["0000", "0001", "9999", "1969", "2024"]).to_string(),
        _ => format!("{:04}", if rng.chance(50) { 2020 + rng.below(10) } else { 1 + rng.below(9999) }),
    };
    let month = if rng.chance(10) { pick(rng, &["00", "13"]).to_string() } else { format!("{:02}", 1 + rng.below(12)) };
    let day = if rng.chance(15) { pick(rng, &["00", "29", "30", "31", "32"]).to_string() } else { format!("{:02}", 1 + rng.below(28)) };
    s.push_str(&format!("{year}-{month}-{day}"));
    let has_time = rng.chance(85);
    if has_time {
        s.push_str(pick(rng, &[" ", " ", "T", "T", "t", "  ", "\t"]));
        s.push_str(&format!("{}:{}", two(rng, 24, &[24, 23]), two(rng, 60, &[60, 59])));
        if rng.chance(85) {
            s.push_str(&format!(":{}", two(rng, 60, &[60, 59])));
            if rng.chance(60) {
                s.push('.');
                let n = 1 + rng.below(12);
                match rng.below(4) {
                    0 => s.push_str(&"9".repeat(n)),
                    1 => {
                        s.push_str(&(0..n.max(8) - 1).map(|_| (b'0' + rng.below(10) as u8) as char).collect::<String>());
                        s.push('5');
                    }
                    _ => s.extend((0..n).map(|_| (b'0' + rng.below(10) as u8) as char)),
                }
            }
        }
    }
    if rng.chance(65) {
        if rng.chance(25) {
            s.push_str(pick(rng, &[" ", "  ", "\t"]));
        }
        match rng.below(10) {
            0 | 1 => s.push_str(pick(rng, &["Z", "z"])),
            k => {
                s.push_str(pick(rng, &["+", "-"]));
                let hh = two(rng, 15, &[14, 15, 0]);
                let mm = if rng.chance(50) { pick(rng, &["00", "30", "45", "59", "60"]).to_string() } else { two(rng, 60, &[0]) };
                match k {
                    2 => s.push_str(&hh),
                    3 => s.push_str(&format!("{hh}{mm}")),
                    _ => s.push_str(&format!("{hh}:{mm}")),
                }
            }
        }
    }
    if rng.chance(10) {
        s.push_str(pick(rng, &[" ", "\n", "\r\n"]));
    }
    s
}

fn corpus(seed: u64, n: usize) -> Vec<String> {
    let mut rng = Rng::new(seed);
    let mut out: Vec<String> = HAND_PICKED.iter().chain(KNOWN_REFUSED).map(|s| s.to_string()).collect();
    out.extend((0..n).map(|_| generated(&mut rng)));
    out
}

/// `parse` in the probe's output format.
fn rust_result(text: &str) -> Value {
    match date_time::parse(text) {
        Some(v) => json!({"ok": true, "ticks": v.ticks, "offset": v.offset_minutes}),
        None => json!({"ok": false}),
    }
}

/// The parts of a probe line that `rust_result` gives.
fn dotnet_result(line: &Value) -> Value {
    if line["ok"] == true {
        json!({"ok": true, "ticks": line["ticks"], "offset": line["offset"]})
    } else {
        json!({"ok": false})
    }
}

/// Every difference, except that a `KNOWN_REFUSED` text must be accepted by .NET and refused here.
fn compare(lines: &[Value]) -> Vec<String> {
    let mut failures = Vec::new();
    for line in lines {
        let input = line["in"].as_str().unwrap();
        let (want, got) = (dotnet_result(line), rust_result(input));
        let ok = if KNOWN_REFUSED.contains(&input) { want["ok"] == true && got["ok"] == false } else { want == got };
        if !ok {
            failures.push(format!("{input:?} (TZ={})\n  .NET: {want}\n  Rust: {got}", line["tz"]));
        }
    }
    failures
}

fn check(lines: &[Value]) {
    // The local time zone never matters: each text reads the same under every TZ.
    let mut by_input: std::collections::BTreeMap<&str, Vec<&Value>> = std::collections::BTreeMap::new();
    for line in lines {
        by_input.entry(line["in"].as_str().unwrap()).or_default().push(line);
    }
    for (input, results) in &by_input {
        assert!(
            results.windows(2).all(|w| dotnet_result(w[0]) == dotnet_result(w[1])),
            "{input:?} reads differently under different TZ: {results:?}"
        );
    }
    let failures = compare(lines);
    assert!(failures.is_empty(), "{} of {} differ:\n{}", failures.len(), lines.len(), failures[..failures.len().min(20)].join("\n"));
}

#[test]
fn golden() {
    let text = std::fs::read_to_string(h1_probe::golden_dir().join("ef_dates.jsonl")).unwrap();
    let lines: Vec<Value> = text.lines().map(|l| serde_json::from_str(l).unwrap()).collect();
    let inputs: std::collections::BTreeSet<&str> = lines.iter().map(|l| l["in"].as_str().unwrap()).collect();
    assert!(inputs.len() > GOLDEN_GENERATED, "golden has {} distinct texts", inputs.len());
    for tz in TZ_SET {
        assert_eq!(lines.iter().filter(|l| l["tz"] == *tz).count(), lines.len() / TZ_SET.len(), "golden lines under TZ={tz}");
    }
    let accepted = lines.iter().filter(|v| v["ok"] == true).count();
    assert!(accepted > lines.len() / 4 && accepted < lines.len() * 3 / 4, "corpus is too one-sided: {accepted} of {} accepted", lines.len());
    check(&lines);
}

#[test]
#[ignore]
fn against_dotnet() {
    let write_golden = std::env::var_os("H1_WRITE_GOLDEN").is_some();
    let (seed, n) = if write_golden {
        (1, GOLDEN_GENERATED)
    } else {
        (
            std::env::var("H1_DATES_SEED").ok().map_or(1, |s| s.parse().unwrap()),
            std::env::var("H1_DATES_N").ok().map_or(50_000, |s| s.parse().unwrap()),
        )
    };
    let inputs = corpus(seed, n);
    let stdin: String = inputs.iter().map(|s| serde_json::to_string(s).unwrap() + "\n").collect();
    let mut lines = Vec::new();
    for tz in TZ_SET {
        let stdout = h1_probe::run_probe_with_env("dates", &stdin, &[("TZ", tz)]);
        let results: Vec<Value> = stdout.lines().map(|l| serde_json::from_str(l).unwrap()).collect();
        assert_eq!(results.len(), inputs.len());
        for (input, r) in inputs.iter().zip(results) {
            assert_eq!(r["tz"], *tz, "the probe did not run under TZ={tz}");
            assert_eq!(r["input"].as_str(), Some(input.as_str()));
            let mut line = json!({"tz": tz, "in": input, "ok": r["ok"]});
            if r["ok"] == true {
                line["ticks"] = r["ticks"].clone();
                line["offset"] = r["offset"].clone();
            }
            lines.push(line);
        }
    }
    let accepted = lines.iter().filter(|v| v["ok"] == true).count();
    println!("{} texts x {} time zones, {accepted} reads accepted by .NET", inputs.len(), TZ_SET.len());
    if write_golden {
        let text: String = lines.iter().map(|l| serde_json::to_string(l).unwrap() + "\n").collect();
        std::fs::write(h1_probe::golden_dir().join("ef_dates.jsonl"), text).unwrap();
    }
    check(&lines);
}
