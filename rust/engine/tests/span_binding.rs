//! Every C# site where an array becomes a span (C# 14 binds `array.Contains(x)` to
//! `MemoryExtensions`, where a null array is empty) and every site where game state
//! stores a collection taken from an event has a row in docs/span-binding.md.
//!
//! `tests/coverage/span_binding.txt` and `tests/coverage/stored_event_collections.txt`
//! are written by `csharp-gen --span-binding` and `--stored-event-collections` from the
//! Roslyn semantic model of Shared. Regenerate them after a change to Shared; a new
//! site then fails here until it is classified.

use std::collections::BTreeMap;
use std::path::Path;

fn read(rel: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join(rel);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

/// The rows of the table under `heading` in span-binding.md: first cell -> other cells.
fn table(heading: &str) -> BTreeMap<String, Vec<String>> {
    let doc = read("../docs/span-binding.md");
    let section = doc
        .split(&format!("\n## {heading}\n"))
        .nth(1)
        .unwrap_or_else(|| panic!("no section {heading:?} in span-binding.md"))
        .split("\n## ")
        .next()
        .unwrap();
    let mut rows = BTreeMap::new();
    for line in section.lines().filter(|l| l.starts_with("| `")) {
        // `\|` inside a cell is an escaped pipe, not a separator.
        let cells: Vec<String> =
            line.replace("\\|", "\u{1}").trim_matches('|').split(" | ").map(|c| c.trim().replace('\u{1}', "|")).collect();
        let key = cells[0].trim_matches('`').to_string();
        let prev = rows.insert(key.clone(), cells[1..].to_vec());
        assert!(prev.is_none(), "two rows for {key} under {heading}");
    }
    rows
}

/// Lines of a coverage file as (key, other columns).
fn sites(rel: &str) -> Vec<(String, Vec<String>)> {
    read(rel)
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| {
            let mut cols = l.split('\t').map(str::to_string);
            (cols.next().unwrap(), cols.collect())
        })
        .collect()
}

fn check_rows(file: &str, heading: &str, classes: &[&str]) -> (Vec<(String, Vec<String>)>, BTreeMap<String, Vec<String>>) {
    let sites = sites(file);
    let rows = table(heading);
    let missing: Vec<_> = sites.iter().map(|s| &s.0).filter(|s| !rows.contains_key(*s)).collect();
    assert!(missing.is_empty(), "sites without a row under {heading:?} in docs/span-binding.md: {missing:#?}");
    let stale: Vec<_> = rows.keys().filter(|k| !sites.iter().any(|s| &s.0 == *k)).collect();
    assert!(stale.is_empty(), "rows under {heading:?} in docs/span-binding.md for sites that no longer exist: {stale:#?}");
    for (key, cells) in &rows {
        assert_eq!(cells.len(), 2, "{key}: a row has three cells");
        assert!(classes.contains(&cells[0].as_str()), "{key}: unknown class {}", cells[0]);
    }
    (sites, rows)
}

#[test]
fn every_span_site_is_classified() {
    let (sites, _) = check_rows("tests/coverage/span_binding.txt", "Span sites", &["null-as-empty", "not-null"]);
    let methods = table("Methods");
    for (key, cols) in &sites {
        let Some(method) = cols[0].strip_prefix("System.MemoryExtensions.") else {
            assert!(key.ends_with(":UNANALYZED"), "{key}: converts to a span for {}, which is not a MemoryExtensions call", cols[0]);
            continue;
        };
        assert!(
            methods.contains_key(method),
            "{key}: MemoryExtensions.{method} has no row under \"Methods\" saying how it differs from Enumerable on a non-null array"
        );
    }
}

#[test]
fn every_stored_event_collection_is_classified() {
    let (sites, rows) =
        check_rows("tests/coverage/stored_event_collections.txt", "Event collections stored in game state", &["nullable", "never-null", "not-state"]);
    let game = read("src/game/mod.rs");
    for (key, cols) in &sites {
        if rows[key][0] != "nullable" {
            continue;
        }
        // `Game.FactionsWithIncreasedRevivalLimits` -> `factions_with_increased_revival_limits`.
        let member = cols[0].strip_prefix("Game.").unwrap_or_else(|| panic!("{key}: a nullable row must store into a Game field"));
        let mut field = String::new();
        for (i, c) in member.chars().enumerate() {
            if c.is_uppercase() && i > 0 {
                field.push('_');
            }
            field.push(c.to_ascii_lowercase());
        }
        let decl = game
            .lines()
            .map(str::trim)
            .find(|l| l.starts_with(&format!("pub {field}:")))
            .unwrap_or_else(|| panic!("{key}: no field `{field}` in game.rs"));
        assert!(decl.contains(": Option<"), "{key}: C# can store null here, so `{field}` must be an Option: {decl}");
    }
}
