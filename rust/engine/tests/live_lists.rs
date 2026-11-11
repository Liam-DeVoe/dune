//! Every C# site that can put a collection into a message has a row in
//! docs/live-lists.md that says why the Rust port matches (hazards 4.5).
//!
//! `tests/coverage/message_collections.txt` is written by
//! `csharp-gen --message-collections` from the Roslyn operation trees of
//! Shared. Regenerate it after a change to Shared; a new site then fails here
//! until it is classified.

use std::collections::BTreeMap;
use std::path::Path;

const CLASSES: [&str; 5] = ["copy", "live", "lazy", "forward", "none"];

fn read(rel: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join(rel);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

/// Site key -> (class, reason) from the table in live-lists.md.
fn doc_rows() -> BTreeMap<String, (String, String)> {
    let mut rows = BTreeMap::new();
    for line in read("../docs/live-lists.md").lines().filter(|l| l.starts_with("| `")) {
        let cells: Vec<&str> = line.trim_matches('|').split(" | ").map(str::trim).collect();
        assert_eq!(cells.len(), 3, "row with other than three cells: {line}");
        let key = cells[0].trim_matches('`').to_string();
        let prev = rows.insert(key.clone(), (cells[1].to_string(), cells[2].to_string()));
        assert!(prev.is_none(), "two rows for {key}");
    }
    rows
}

#[test]
fn every_message_collection_site_is_classified() {
    let sites: Vec<String> =
        read("tests/coverage/message_collections.txt").lines().map(|l| l.split('\t').next().unwrap().to_string()).collect();
    let rows = doc_rows();

    let missing: Vec<_> = sites.iter().filter(|s| !rows.contains_key(*s)).collect();
    assert!(missing.is_empty(), "sites without a row in docs/live-lists.md: {missing:#?}");
    let stale: Vec<_> = rows.keys().filter(|k| !sites.contains(k)).collect();
    assert!(stale.is_empty(), "rows in docs/live-lists.md for sites that no longer exist: {stale:#?}");

    let expression = read("src/expression.rs");
    for (key, (class, why)) in &rows {
        assert!(CLASSES.contains(&class.as_str()), "{key}: unknown class {class}");
        if class == "live" {
            let variant = why
                .split("LiveList::")
                .nth(1)
                .and_then(|s| s.split(|c: char| !c.is_alphanumeric()).next())
                .unwrap_or_else(|| panic!("{key}: a live row must name its LiveList variant"));
            assert!(
                expression.contains(&format!("    {variant}(")) || expression.contains(&format!("    {variant},")),
                "{key}: LiveList::{variant} is not a variant in expression.rs"
            );
        }
    }
}

/// Each `LiveList` variant is built somewhere outside expression.rs, so a
/// live row can't be satisfied by a variant that no port uses.
#[test]
fn every_live_list_variant_is_used_by_a_port() {
    let expression = read("src/expression.rs");
    let body = expression.split("pub enum LiveList {").nth(1).unwrap().split("\n}").next().unwrap();
    let variants: Vec<&str> = body
        .lines()
        .map(str::trim)
        .filter(|l| !l.starts_with("///") && !l.is_empty())
        .map(|l| l.split(|c: char| !c.is_alphanumeric()).next().unwrap())
        .collect();
    assert!(!variants.is_empty());

    let mut sources = String::new();
    let mut stack = vec![Path::new(env!("CARGO_MANIFEST_DIR")).join("src")];
    while let Some(dir) = stack.pop() {
        for e in std::fs::read_dir(dir).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x == "rs") && !p.ends_with("expression.rs") && !p.ends_with("canon.rs") {
                sources.push_str(&std::fs::read_to_string(&p).unwrap());
            }
        }
    }
    let wrappers = [("PlayerTreacheryCards", "live_player_treachery_cards"), ("GameAuditedCards", "live_game_audited_cards")];
    for v in variants {
        let wrapper = wrappers.iter().find(|(n, _)| *n == v).map(|(_, w)| *w);
        assert!(
            sources.contains(&format!("LiveList::{v}")) || wrapper.is_some_and(|w| sources.contains(w)),
            "LiveList::{v} is not used outside expression.rs"
        );
    }
}

/// `DealAccepted.cs:152` logs `offer.Player.DiscardedTraitors` itself: a
/// traitor discarded after the message shows up in the dump of that message.
#[test]
fn discarded_traitors_in_a_message_dump_the_current_set() {
    use treachery_engine::canon;
    use treachery_engine::expression::LiveList;
    use treachery_engine::prelude::*;

    let mut g = Game::new();
    g.version = Game::LATEST_VERSION;
    g.players.push(Player::new(&g, Faction::Harkonnen));
    let harkonnen = g.get_player(Some(Faction::Harkonnen)).unwrap();
    let mut leaders = LeaderManager::get_leaders(Faction::Emperor).map(Hero::Leader);
    let (first, second) = (leaders.next().unwrap(), leaders.next().unwrap());
    g.player_mut(harkonnen).discarded_traitors.add(first);

    let live = ExpressionElement::live(&g, LiveList::PlayerDiscardedTraitors(harkonnen));
    assert_eq!(live.to_cs_string(), "System.Collections.Generic.HashSet`1[Treachery.Shared.IHero]");
    let dump = |g: &Game| String::from_utf8(canon::dump_detached(&live, Some(g))).unwrap();
    let before = dump(&g);
    assert_eq!(before, String::from_utf8(canon::dump_detached(&ExpressionElement::collection("x", vec![first]), Some(&g))).unwrap());

    g.player_mut(harkonnen).discarded_traitors.add(second);
    assert_eq!(dump(&g), String::from_utf8(canon::dump_detached(&ExpressionElement::collection("x", vec![first, second]), Some(&g))).unwrap());
    // Without a game the snapshot taken at log time is written.
    assert_eq!(String::from_utf8(canon::dump_detached(&live, None)).unwrap(), before);
}
