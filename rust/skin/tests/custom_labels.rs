//! A host's skin may put braces in its labels. They print as they are, in log texts, in the
//! skin's own texts and in the faction sheets.

use treachery_engine::enumerations::{Concept, Faction, FactionAdvantage, Rule};
use treachery_engine::express;
use treachery_engine::game::Game;
use treachery_skin::default_skin;

#[test]
fn braces_in_labels_print_as_they_are() {
    let mut skin = default_skin().clone();
    skin.faction_name_str.set(Faction::Atreides, "{0} House {".to_string());
    skin.concept_str.set(Some(Concept::Messiah), "{20}Kwisatz}".to_string());

    assert_eq!(skin.render(&express!(Faction::Atreides, " pays ", 3)), "{0} House { pays 3");
    assert_eq!(
        skin.describe_faction_advantage(Some(FactionAdvantage::AtreidesBiddingPrescience)),
        "{0} House { seeing the card on auction"
    );

    // The Messiah's advantage has the concept's label as its title, and its text names the
    // Messiah through sheet parameter 20.
    let mut g = Game::of_version(Game::LATEST_VERSION);
    g.rules.active.push(Rule::AtreidesMessiah);
    let sheet = skin.get_faction_info_html(&g, Faction::Atreides, Some(""));
    assert!(sheet.contains("<strong>{20}KWISATZ}</strong> - after losing a total of at least 7 forces in battle, you may use the {20}Kwisatz}."), "{sheet}");
    let advantages = skin.get_faction_sheet(&g, Faction::Atreides).expect("a sheet").advanced_advantages;
    assert_eq!(advantages, vec!["{20}Kwisatz}".to_string()]);
}
