//! The lobby data file the web client loads (`web/public/static/lobby.json`): current, and equal
//! to the engine's tables and the default skin. To update it after changing `src/lobby.rs`:
//!
//!     UPDATE_LOBBY_DATA=1 cargo test -p treachery-view --test lobby_data

use std::path::PathBuf;

use treachery_engine::enumerations::*;
use treachery_engine::game::Game;
use treachery_engine::net_enum::NetEnum;
use treachery_view::lobby::lobby_data;

fn path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../web/public/static/lobby.json")
}

#[test]
fn lobby_data_file_is_current() {
    let text = serde_json::to_string(&lobby_data()).unwrap() + "\n";
    if std::env::var("UPDATE_LOBBY_DATA").is_ok() {
        std::fs::write(path(), &text).unwrap();
        return;
    }
    let existing = std::fs::read_to_string(path()).unwrap_or_default();
    assert!(existing == text, "web/public/static/lobby.json is out of date. Run: UPDATE_LOBBY_DATA=1 cargo test -p treachery-view --test lobby_data");
}

#[test]
fn lobby_data_matches_the_engine_and_skin() {
    let d = lobby_data();
    assert_eq!(d.latest_version, Game::LATEST_VERSION);
    assert_eq!(d.maximum_players, 12);
    assert_eq!(d.maximum_turns, 20);

    // Numbers are the C# enum values the lobby protocol carries.
    let faction = |n: &str| d.enums["Faction"].iter().find(|m| m.name == n).unwrap().clone();
    assert_eq!(faction("Atreides").value, Faction::Atreides.value());
    assert_eq!(faction("BeneGesserit").label, "Bene Gesserit");
    let error = |v: ErrorType| d.enums["ErrorType"].iter().find(|m| m.value == v.value()).unwrap().label.clone();
    assert_eq!(error(ErrorType::InvalidUserNameOrPassword), "Invalid user name or password");
    assert_eq!(error(ErrorType::None), "");
    let ruleset = |v: Ruleset| d.enums["Ruleset"].iter().find(|m| m.value == v.value()).unwrap().label.clone();
    assert_eq!(ruleset(Ruleset::BasicGame), "Standard Dune - Basic");
    let sub = |v: SubscriptionType| d.enums["SubscriptionType"].iter().find(|m| m.value == v.value()).unwrap().label.clone();
    assert_eq!(sub(SubscriptionType::CertainAsHost), "yes*");

    // Every enum lists every member, in Enum.GetValues order.
    assert_eq!(d.enums["Rule"].len(), Rule::ALL.len());
    assert!(d.enums["UserStatus"].windows(2).all(|w| (w[0].value as u32) < (w[1].value as u32)));

    assert_eq!(d.available_factions.len(), 12);
    assert_eq!(d.ruleset_rules["BasicGame"], Game::ruleset_definition().iter().find(|(r, _)| **r == Ruleset::BasicGame).unwrap().1.iter().map(|r| r.to_string()).collect::<Vec<_>>());
    let bots: Vec<String> = Game::get_rules_in_group(RuleGroup::Bots, Game::EXPANSION_LEVEL).iter().map(|r| r.to_string()).collect();
    assert_eq!(d.rules_in_group["Bots"], bots);
    assert_eq!(d.rule_group["AdvancedCombat"], "CoreAdvanced");
}
