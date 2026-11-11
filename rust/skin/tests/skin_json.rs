//! `SkinToString` and `Skin.Load` against C# (`csharp-ref/Load.cs`).

mod common;

use common::*;
use serde_json::Value;
use treachery_skin::{default_skin, Skin};

/// The generated `default_skin.rs` serializes to exactly the C# `DefaultSkin.Default.SkinToString()`.
#[test]
fn default_skin_serializes_like_csharp() {
    let expected = std::fs::read_to_string(golden("default_skin.json")).unwrap();
    assert_eq!(default_skin().skin_to_string(), expected);
}

#[test]
fn load_matches_csharp() {
    let mut diffs = Diffs::new();
    for line in read_lines(&golden("load.jsonl.gz")) {
        let v: Value = serde_json::from_str(&line).unwrap();
        let input = v["in"].as_str().unwrap();
        let expected = v["out"].as_str().unwrap();
        let actual = match Skin::load(input, default_skin()) {
            Ok(s) => s.skin_to_string(),
            Err(e) => format!("!{}", e.type_name),
        };
        diffs.check(&format!("Load({input})"), expected, &actual);
    }
    diffs.finish("Skin.Load");
}
