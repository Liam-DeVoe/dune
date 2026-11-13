//! The static data file the web client loads (`web/public/static/data.json`): current, and
//! equal to the engine's tables. To update it after changing `src/static_data.rs`:
//!
//!     UPDATE_STATIC_DATA=1 cargo test -p treachery-view --test static_data

use std::path::PathBuf;

use treachery_engine::identity::IIdentifiable;
use treachery_engine::heroes::leader_manager::LeaderManager;
use treachery_engine::prelude::*;
use treachery_engine::cards::treachery_card_manager::TreacheryCardManager;
use treachery_view::static_data::static_data;

fn path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../web/public/static/data.json")
}

#[test]
fn static_data_file_is_current() {
    let text = serde_json::to_string(&static_data()).unwrap() + "\n";
    if std::env::var("UPDATE_STATIC_DATA").is_ok() {
        std::fs::write(path(), &text).unwrap();
        return;
    }
    let existing = std::fs::read_to_string(path()).unwrap_or_default();
    assert!(existing == text, "web/public/static/data.json is out of date. Run: UPDATE_STATIC_DATA=1 cargo test -p treachery-view --test static_data");
}

/// Every leader with its faction, every card with its type, every territory with its flags, as
/// the engine has them.
#[test]
fn static_data_matches_the_engine() {
    let data = static_data();

    assert_eq!(data.leaders.len(), LeaderManager::leaders().len());
    for (info, l) in data.leaders.iter().zip(LeaderManager::leaders()) {
        assert_eq!(info.id, l.id());
        assert_eq!(info.faction, l.faction().to_string());
    }
    assert!(data.leaders.iter().any(|l| l.faction == "Tleilaxu"), "face dancer leaders are listed");

    let cards = TreacheryCardManager::get_cards_in_and_outside_play();
    assert_eq!(data.treachery_cards.len(), cards.len());
    for (info, c) in data.treachery_cards.iter().zip(cards) {
        assert_eq!((info.id, info.card_type.as_str(), info.is_weapon, info.is_defense), (c.id(), c.type_().to_string().as_str(), c.is_weapon(), c.is_defense()));
    }
    let ids: std::collections::BTreeSet<i32> = data.treachery_cards.iter().map(|c| c.id).collect();
    assert_eq!(ids.len(), cards.len(), "treachery card ids are unique");

    let game = Game::new_int32_participation(Game::LATEST_VERSION, &treachery_engine::shared::Shared::new(Default::default()));
    for t in game.map.territories(true) {
        let info = data.territories.iter().find(|i| i.id == t.id()).expect("every territory");
        assert_eq!(
            (info.is_stronghold, info.is_homeworld, info.is_protected_from_storm, info.is_protected_from_worm, info.advantage.as_str()),
            (t.is_stronghold(), t.is_homeworld(), t.is_protected_from_storm(), t.is_protected_from_worm(), t.advantage().to_string().as_str())
        );
        for l in t.locations() {
            let li = data.locations.iter().find(|i| i.id == l.id()).expect("every location");
            assert_eq!(li.territory, t.id());
        }
    }
    assert_eq!(data.locations.len(), game.map.locations(true).count());
    assert_eq!(data.locations.iter().filter(|l| l.kind == "hiddenMobileStronghold").count(), 1);
    assert_eq!(data.locations.iter().filter(|l| l.kind == "discovered").count(), 5);
    assert!(data.territories.iter().filter(|t| t.is_stronghold).count() >= 5);

    // The view names resource cards by SkinId, the id of their image. Cards with the same image
    // (the Shai-Hulud cards) share it; spice cards, which the map points at, don't.
    let spice: Vec<_> = data.resource_cards.iter().filter(|c| c.location.is_some()).collect();
    let skin_ids: std::collections::BTreeSet<i32> = spice.iter().map(|c| c.skin_id).collect();
    assert_eq!(skin_ids.len(), spice.len(), "spice card skin ids are unique");
    for c in &data.resource_cards {
        for d in data.resource_cards.iter().filter(|d| d.skin_id == c.skin_id) {
            assert_eq!(c, d, "cards sharing a skin id are alike");
        }
    }

    let emperor = data.factions.iter().find(|f| f.faction == "Emperor").unwrap();
    assert_eq!((emperor.force.as_str(), emperor.special_force.as_str()), ("Emperor", "Emperor"));
    let atreides = data.factions.iter().find(|f| f.faction == "Atreides").unwrap();
    assert_eq!(atreides.special_force, "None");
    assert!(data.rules.iter().any(|r| r.rule == "AdvancedCombat" && r.group == "CoreAdvanced"));
    assert_eq!(data.map.polar_sink, game.map.polar_sink().id());
}
