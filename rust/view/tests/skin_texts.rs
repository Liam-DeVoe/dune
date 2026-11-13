//! `factionSheets`, `ruleLabels` and `recentMilestones` (view/src/skin_texts.rs): equal to what
//! the skin and the engine give for the client's popovers and sounds, and the same for every
//! viewer.

mod common;

use common::{fixture, load, truncated, load_text, viewer_of, OBSERVER};
use treachery_engine::messages::expression::ExpressionElement;
use treachery_engine::prelude::*;
use treachery_view::replica::ViewReplica;
use treachery_view::schema::GameView;
use treachery_view::skin_texts::SheetPart;

fn fixtures() -> Vec<String> {
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scripts/fixtures");
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .filter_map(|e| e.ok()?.file_name().to_str()?.strip_suffix(".json").map(str::to_string))
        .collect();
    names.sort();
    names
}

fn all_views(r: &mut ViewReplica) -> Vec<GameView> {
    let factions: Vec<Faction> = r.game.players.iter().map(|p| p.faction).collect();
    let mut out: Vec<GameView> = factions.into_iter().map(|f| r.view(viewer_of(r, f))).collect();
    out.push(r.view(OBSERVER));
    out
}

#[test]
fn sheets_and_rules_equal_the_skin() {
    let skin = treachery_skin::default_skin();
    let mut sheets_seen = 0;
    for name in fixtures() {
        let mut r = load(&name, 0);
        let views = all_views(&mut r);
        let g = &r.game;
        let v = &views[0];
        for other in &views[1..] {
            assert_eq!(other.faction_sheets, v.faction_sheets, "{name}: sheets differ between viewers");
            assert_eq!(other.rule_labels, v.rule_labels, "{name}: rule labels differ between viewers");
            assert_eq!(other.recent_milestones, v.recent_milestones, "{name}: milestones differ between viewers");
        }

        // One sheet per player that has one, in player order.
        let expected: Vec<Faction> = g.players.iter().map(|p| p.faction).filter(|f| skin.get_faction_sheet(g, *f).is_some()).collect();
        let got: Vec<String> = v.faction_sheets.iter().map(|s| s.faction.clone()).collect();
        assert_eq!(got, expected.iter().map(|f| f.to_string()).collect::<Vec<_>>(), "{name}");
        for (s, f) in v.faction_sheets.iter().zip(&expected) {
            let sheet = skin.get_faction_sheet(g, *f).unwrap();
            assert_eq!(s.html, skin.get_faction_info_html(g, *f, Some("")), "{name} {f}");
            assert_eq!(s.theme, sheet.theme);
            assert_eq!(s.advantages, sheet.advantages);
            assert_eq!(s.advanced_advantages, sheet.advanced_advantages);
            assert_eq!(s.free_revival, sheet.free_revival);
            // The parts put back together with places as their names equal the skin's own
            // Format of the summary line (which describes places by name).
            let joined: String = s
                .at_start
                .iter()
                .map(|p| match p {
                    SheetPart::Text { v } => v.clone(),
                    SheetPart::Location { id } => skin.describe_location(g.map.locations(true).find(|l| l.id() == *id).unwrap()),
                    SheetPart::Territory { id } => skin.describe_territory(g.map.territories(true).find(|t| t.id() == *id).unwrap()),
                })
                .collect();
            let template = format!("{}. Start with {} {{16}}.", sheet.at_start, sheet.resources);
            assert_eq!(joined, skin.format(&template, &sheet.parameters), "{name} {f}");
            assert!(!joined.contains('{'), "{name} {f}: {joined}");
            sheets_seen += 1;
        }

        // TurnPopover's groups, in order of their first rule.
        assert_eq!(v.rule_labels.ruleset, skin.describe(&ExpressionElement::from(g.ruleset)));
        let mut groups: Vec<RuleGroup> = vec![];
        for r in &g.rules {
            let gr = Game::get_rule_group(*r);
            if ![RuleGroup::CoreBasic, RuleGroup::Bots, RuleGroup::None].contains(&gr) && !groups.contains(&gr) {
                groups.push(gr);
            }
        }
        assert_eq!(v.rule_labels.groups.len(), groups.len(), "{name}");
        for (l, gr) in v.rule_labels.groups.iter().zip(&groups) {
            assert_eq!(l.group, skin.describe(&ExpressionElement::from(*gr)));
            let rules: Vec<String> = g.rules.iter().filter(|r| Game::get_rule_group(**r) == *gr).map(|r| skin.describe(&ExpressionElement::from(*r))).collect();
            assert_eq!(l.rules, rules, "{name}");
        }
        assert!(v.rule_labels.groups.iter().all(|gr| gr.rules.iter().all(|r| !r.is_empty())));
    }
    assert!(sheets_seen > 100, "only {sheets_seen} sheets");
}

/// After every event of a few fixtures: `recentMilestones` is `Game.RecentMilestones`, and at
/// least some events reach milestones.
#[test]
fn milestones_follow_each_event() {
    let mut nonempty = 0;
    for name in ["bid", "blow", "mc-discard", "battle"] {
        let v: serde_json::Value = serde_json::from_str(&fixture(name)).unwrap();
        let events = v["_events"].as_array().unwrap();
        let mut r = load_text(&truncated(name, 1), 0);
        for e in events.iter().skip(1) {
            r.apply(&e.to_string()).unwrap();
            let view = r.view(OBSERVER);
            let expected: Vec<String> = r.game.recent_milestones.iter().map(|m| m.to_string()).collect();
            assert_eq!(view.recent_milestones, expected);
            if !expected.is_empty() {
                nonempty += 1;
            }
        }
    }
    assert!(nonempty > 10, "{nonempty}");
}
