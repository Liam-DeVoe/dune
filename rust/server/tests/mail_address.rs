//! `MailAddress::parse` against .NET 10's `new MailAddress(address)`.
//!
//! - `golden`: the hand-picked cases and 2,000 generated addresses with the answers .NET gave
//!   (`tests/golden/mail_address.jsonl`). Runs without .NET.
//! - `against_dotnet` (ignored): generates `H1_MAIL_N` addresses (default 200,000) from seed
//!   `H1_MAIL_SEED` (default 1), runs them through the H1 probe and compares every answer.
//!   With `H1_WRITE_GOLDEN=1` it rewrites the golden file from seed 1. Run it with
//!   `rust/tools/heavy.sh cargo test --release -p treachery-server --test mail_address -- --ignored`.

mod h1_probe;

use h1_probe::{address, rust_mail_result, Rng, HAND_PICKED};
use serde_json::Value;

const GOLDEN_GENERATED: usize = 2000;

fn corpus(seed: u64, n: usize) -> Vec<String> {
    let mut rng = Rng::new(seed);
    let mut out: Vec<String> = HAND_PICKED.iter().map(|s| s.to_string()).collect();
    out.extend((0..n).map(|_| address(&mut rng)));
    out
}

fn compare(inputs: &[String], expected: &[Value]) -> Vec<String> {
    let mut failures = Vec::new();
    for (input, want) in inputs.iter().zip(expected) {
        let got = rust_mail_result(input);
        if &got != want {
            failures.push(format!("{input:?}\n  .NET: {want}\n  Rust: {got}"));
        }
    }
    failures
}

#[test]
fn golden() {
    let text = std::fs::read_to_string(h1_probe::golden_dir().join("mail_address.jsonl")).unwrap();
    let mut inputs = Vec::new();
    let mut expected = Vec::new();
    for line in text.lines() {
        let v: Value = serde_json::from_str(line).unwrap();
        inputs.push(v["in"].as_str().unwrap().to_string());
        expected.push(v["out"].clone());
    }
    assert!(inputs.len() > GOLDEN_GENERATED);
    let accepted = expected.iter().filter(|v| v["ok"] == true).count();
    assert!(accepted > 300 && accepted < inputs.len() - 300, "corpus is too one-sided: {accepted} of {} accepted", inputs.len());
    let failures = compare(&inputs, &expected);
    assert!(failures.is_empty(), "{} of {} differ:\n{}", failures.len(), inputs.len(), failures[..failures.len().min(20)].join("\n"));
}

#[test]
#[ignore]
fn against_dotnet() {
    let write_golden = std::env::var_os("H1_WRITE_GOLDEN").is_some();
    let (seed, n) = if write_golden {
        (1, GOLDEN_GENERATED)
    } else {
        (
            std::env::var("H1_MAIL_SEED").ok().map_or(1, |s| s.parse().unwrap()),
            std::env::var("H1_MAIL_N").ok().map_or(200_000, |s| s.parse().unwrap()),
        )
    };
    let inputs = corpus(seed, n);
    let stdin: String = inputs.iter().map(|s| serde_json::to_string(s).unwrap() + "\n").collect();
    let stdout = h1_probe::run_probe("mail", &stdin);
    let expected: Vec<Value> = stdout.lines().map(|l| serde_json::from_str(l).unwrap()).collect();
    assert_eq!(expected.len(), inputs.len());

    let accepted = expected.iter().filter(|v| v["ok"] == true).count();
    println!("{} addresses, {accepted} accepted by .NET", inputs.len());
    let failures = compare(&inputs, &expected);
    assert!(failures.is_empty(), "{} of {} differ:\n{}", failures.len(), inputs.len(), failures[..failures.len().min(20)].join("\n"));

    if write_golden {
        let text: String = inputs
            .iter()
            .zip(&expected)
            .map(|(i, o)| serde_json::to_string(&serde_json::json!({"in": i, "out": o})).unwrap() + "\n")
            .collect();
        std::fs::write(h1_probe::golden_dir().join("mail_address.jsonl"), text).unwrap();
    }
}
