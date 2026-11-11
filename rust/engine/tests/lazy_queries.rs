//! Every lazy LINQ query in Shared, and every site that consumes one, has a row
//! in docs/lazy-queries.md that says why the Rust port matches.
//!
//! `tests/coverage/lazy_queries.txt` is written by `csharp-gen --lazy-queries`
//! from the Roslyn operation trees of Shared. Regenerate it after a change to
//! Shared; a new site then fails here until it is classified.
//!
//! Besides the bookkeeping, the test checks the one rule that matters: a site
//! whose consumer stops early (`Any`, `First`, `Contains`, ...) must reach a
//! producer whose Rust function returns an iterator, so the predicate never
//! runs on elements that C# doesn't reach.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

const PRODUCER_CLASSES: [&str; 2] = ["iter", "vec"];
const SITE_CLASSES: [&str; 5] = ["short", "full", "foreach", "forward", "double"];

fn read(rel: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join(rel);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

struct Line {
    key: String,
    consumer: String,
}

fn lines() -> Vec<Line> {
    read("tests/coverage/lazy_queries.txt")
        .lines()
        .map(|l| {
            let mut cells = l.split('\t');
            Line { key: cells.next().unwrap().to_string(), consumer: cells.next().unwrap_or("").to_string() }
        })
        .collect()
}

/// Row key -> (class, third cell), from the tables after `## Producers` in lazy-queries.md.
fn doc_rows() -> BTreeMap<String, (String, String)> {
    let mut rows = BTreeMap::new();
    let doc = read("../docs/lazy-queries.md");
    let tables = doc.split("\n## Producers\n").nth(1).expect("lazy-queries.md has a `## Producers` section");
    for line in tables.lines().filter(|l| l.starts_with("| `")) {
        let cells: Vec<&str> = line.trim_matches('|').split(" | ").map(str::trim).collect();
        assert_eq!(cells.len(), 3, "row with other than three cells: {line}");
        let key = cells[0].trim_matches('`').to_string();
        let prev = rows.insert(key.clone(), (cells[1].to_string(), cells[2].to_string()));
        assert!(prev.is_none(), "two rows for {key}");
    }
    rows
}

/// `Type.Member` of a key: `Events/Karma.cs:241:Karma.ValidKarmaCards#2` -> `Karma.ValidKarmaCards`.
fn member(key: &str) -> &str {
    let name = key.splitn(3, ':').nth(2).unwrap();
    name.split('#').next().unwrap()
}

/// The consumer before any ` via ...` suffix, e.g. `short:Any`.
fn top(consumer: &str) -> &str {
    consumer.split(" via ").next().unwrap()
}

#[test]
fn every_lazy_query_site_is_classified() {
    let lines = lines();
    let rows = doc_rows();
    let keys: BTreeSet<&str> = lines.iter().map(|l| l.key.as_str()).collect();

    let missing: Vec<_> = lines.iter().filter(|l| !rows.contains_key(&l.key)).map(|l| &l.key).collect();
    assert!(missing.is_empty(), "lines of lazy_queries.txt without a row in docs/lazy-queries.md: {missing:#?}");
    let stale: Vec<_> = rows.keys().filter(|k| !keys.contains(k.as_str())).collect();
    assert!(stale.is_empty(), "rows in docs/lazy-queries.md for sites that no longer exist: {stale:#?}");

    for l in &lines {
        let (class, _) = &rows[&l.key];
        if l.consumer == "producer" {
            assert!(PRODUCER_CLASSES.contains(&class.as_str()), "{}: {class} is not a producer class", l.key);
        } else {
            assert!(SITE_CLASSES.contains(&class.as_str()), "{}: {class} is not a site class", l.key);
        }
    }
}

/// Producer member -> its producer keys (overloads have several).
fn producers(lines: &[Line]) -> BTreeMap<&str, Vec<&str>> {
    let mut m: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for l in lines.iter().filter(|l| l.consumer == "producer") {
        m.entry(member(&l.key)).or_default().push(&l.key);
    }
    m
}

#[test]
fn short_circuit_sites_reach_rust_iterators() {
    let lines = lines();
    let rows = doc_rows();
    let producers = producers(&lines);

    // Producers that some consumer stops early on, directly or through `return`.
    let mut needs_iter: BTreeMap<&str, String> = BTreeMap::new();
    for l in lines.iter().filter(|l| l.consumer != "producer") {
        if l.consumer.contains("short:") {
            let class = &rows[&l.key].0;
            assert!(
                class == "short" || class == "double",
                "{}: the consumer `{}` stops early, so the row must be `short` or `double`, not `{class}`",
                l.key,
                l.consumer
            );
            if top(&l.consumer).starts_with("short:") && producers.contains_key(member(&l.key)) {
                needs_iter.insert(member(&l.key), l.key.clone());
            }
        }
    }
    loop {
        let before = needs_iter.len();
        for l in lines.iter().filter(|l| l.consumer.starts_with("return ")) {
            let returned_by = l.consumer["return ".len()..].split(' ').next().unwrap();
            if needs_iter.contains_key(returned_by) && producers.contains_key(member(&l.key)) {
                needs_iter.entry(member(&l.key)).or_insert_with(|| format!("{} (returned by {returned_by})", l.key));
            }
        }
        if needs_iter.len() == before {
            break;
        }
    }
    assert!(!needs_iter.is_empty());

    for (m, why) in &needs_iter {
        for key in &producers[m] {
            let (class, rust) = &rows[*key];
            assert_eq!(class, "iter", "{key}: must be `iter`, because a consumer stops early at {why}");
            check_returns_iterator(key, rust);
        }
    }
}

/// Every `iter` row names a Rust function that really returns an iterator.
#[test]
fn iter_rows_name_iterator_functions() {
    let lines = lines();
    let rows = doc_rows();
    for l in lines.iter().filter(|l| l.consumer == "producer") {
        let (class, rust) = &rows[&l.key];
        if class == "iter" {
            check_returns_iterator(&l.key, rust);
        }
    }
}

/// `rust` starts with `` `src/path.rs::fn_name` ``.
fn check_returns_iterator(key: &str, rust: &str) {
    let path = rust.trim_start_matches('`').split('`').next().unwrap();
    let (file, func) = path.split_once("::").unwrap_or_else(|| panic!("{key}: the row must start with `src/file.rs::fn`"));
    let src = read(file);
    let needle = format!("fn {func}");
    let at = src
        .match_indices(&needle)
        .map(|(i, _)| i)
        .find(|i| src[i + needle.len()..].starts_with(['(', '<']))
        .unwrap_or_else(|| panic!("{key}: no `fn {func}` in {file}"));
    let signature = &src[at..at + src[at..].find('{').unwrap()];
    assert!(
        signature.contains("Iterator") || signature.contains("-> TakeRandomN"),
        "{key}: `{func}` in {file} must return an iterator: {signature}"
    );
}

/// `iter` producers that the boundary harness doesn't call (csharp-ref/LazyBoundary.cs),
/// with the reason. Every other `iter` producer must be in boundary.json's "lazy" section.
const NOT_PROBED: [(&str, &str); 17] = [
    ("Enumerations.GetValuesExceptDefault", "static enum values, no game state"),
    ("LeaderManager.GetLeaders", "static leader table"),
    ("Map.Locations", "static map data"),
    ("Map.Homeworlds", "static map data"),
    ("Map.Territories", "static map data"),
    ("Map.Strongholds", "static map data"),
    ("ExtensionMethods.TakeRandomN", "generic over the source; random_helpers.rs checks its draws"),
    ("Battle.CheapHeroes", "private; its caller Battle.ValidBattleHeroes needs a battle the scenarios don't set up"),
    ("Shipment.RegularShipmentLocations", "private; probed through Shipment.ValidShipmentLocations"),
    ("FremenRidesMonster.LocationsWithForcesThatCanRide", "private; probed through FremenRidesMonster.ValidSources"),
    ("Game.Strongholds", "private; its predicate reads static data and the special-stronghold flags"),
    ("Game.TerrorIn", "takes a Territory; probed through TerrorRevealed.GetTypes"),
    ("Game.DealRecipients", "takes a DealOffered, which the scenarios don't create"),
    ("ClairVoyancePlayed.ValidQuestions", "takes a target Faction; its only predicate reads the bot flag"),
    ("DiscoveryEntered.ValidSources", "takes a DiscoveredLocation; the scenarios attach none to a player's forces"),
    ("Player.LocationsWithAnyForcesInTerritory", "takes a Territory; its predicate reads the player's own forces"),
    ("PlayerSequence.GetPlayersInSequence", "needs a PlayerSequence; review_pkg_b.rs player_sequence_matches_csharp checks it"),
];

#[test]
fn iter_producers_are_probed_by_the_boundary_harness() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/boundary.json");
    let boundary: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let probed: BTreeSet<&str> = boundary["lazy"]["producers"].as_array().unwrap().iter().map(|v| v.as_str().unwrap()).collect();
    let excused: BTreeSet<&str> = NOT_PROBED.iter().map(|(n, _)| *n).collect();

    let lines = lines();
    let rows = doc_rows();
    let mut iter_members = BTreeSet::new();
    for l in lines.iter().filter(|l| l.consumer == "producer") {
        if rows[&l.key].0 == "iter" {
            iter_members.insert(member(&l.key));
        }
    }
    let unprobed: Vec<_> = iter_members.iter().filter(|m| !probed.contains(*m) && !excused.contains(*m)).collect();
    assert!(
        unprobed.is_empty(),
        "iter producers that neither csharp-ref/LazyBoundary.cs calls nor NOT_PROBED excuses: {unprobed:?}"
    );
    let stale: Vec<_> = excused.iter().filter(|m| !iter_members.contains(*m) || probed.contains(*m)).collect();
    assert!(stale.is_empty(), "NOT_PROBED entries that are probed or no longer iter producers: {stale:?}");
}
