//! Log and event texts that were fixed on purpose after the port (behavior-changes.md:
//! "Missing spaces", "Setting an automation rule", "Other small text quirks", "Typos and grammar").

use std::rc::Rc;

use treachery_engine::enumerations::{AutomationRuleType, DealType, Faction, FactionAdvantage, ItemAction, TreacheryCardType};
use treachery_engine::cards::treachery_card_manager::TreacheryCardManager;
use treachery_engine::game::Deal;
use treachery_engine::messages::part::Part;
use treachery_engine::events::game_event::GameEventMethods;
use treachery_engine::events::{AutomationConfigured, By, EventRules, FlightDiscoveryUsed, FlightUsed, HistoryEntry, Karma, KarmaChoamDiscard, Shipment, Voice};
use treachery_engine::game::Game;
use treachery_engine::json;
use treachery_engine::messages::message::{Message, MessagePart};
use treachery_skin::default_skin;

const ESTABLISH: &str = r#"{"$type":"EstablishPlayers","Settings":{"NumberOfPlayers":3,"MaximumTurns":8,"InitialRules":[301,300,65,60,20,1013,15,700,105,80,85,604,90,998,30,701,40,107,10,70,50,25,209,999,112],"AllowedFactionsInPlay":[50,10,120,60,40,100,20,90,70,80,30,110],"AutoOpenEmptySeats":true,"AsyncPlay":false,"AsyncPlayMessageIntervalMinutes":0},"Seed":1073548723,"MaximumNumberOfPlayers":3,"MaximumTurns":8,"ApplicableRules":[301,300,65,60,20,1013,15,700,105,80,85,604,90,998,30,701,40,107,10,70,50,25,209,999,112],"_players":"","_factionsInPlay":"Guild,Fremen,Moritani,BeneGesserit,Emperor,Richese,Atreides,Choam,Ixian,Tleilaxu,Harkonnen,Ecaz","Initiator":0,"Time":"2024-01-01T12:02:17+00:00"}"#;

/// A started three-player game whose players are Atreides (with the Harkonnen Nexus card),
/// Guild and Choam.
fn game() -> Game {
    // User 100 ("Human100") sits in seat 2.
    let mut g = Game::new();
    g.human_seats.insert(treachery_engine::game::Seat::new(2));
    let e: HistoryEntry = json::deserialize(ESTABLISH).expect("event json");
    assert!(g.play_for_message(e, true, true).is_none());
    for (player, faction) in g.players.iter_mut().zip([Faction::Atreides, Faction::Guild, Faction::Choam]) {
        player.faction = Some(faction);
        player.nexus = None;
    }
    g.players[0].nexus = Some(Faction::Harkonnen);
    g
}

fn text(m: &Message) -> String {
    default_skin().render(m)
}

fn log(game: &Game) -> Vec<String> {
    game.reports().iter().flat_map(|r| r.messages.iter()).map(text).collect()
}

#[test]
fn karama_against_guild_income_has_a_space_before_the_amount() {
    let mut g = game();
    let to = g.board.map.locations(false).find(|l| l.id() != 0 && !l.territory().is_stronghold()).unwrap();
    let mut s = Shipment::default();
    s.force_amount = 2;
    s.karma_card = None;
    s.to = Some(to);
    g.prevent(Faction::Atreides, FactionAdvantage::GuildReceiveShipment, false);
    let mut part = MessagePart::express(vec![]);
    let by = By::new(&g, Faction::Atreides);
    s.handle_received_shipment_payment(&mut g, by, &mut part);
    let rendered = default_skin().render_parts(&part.parts);
    assert_eq!(rendered, " → Karama prevents Guild from receiving 4 Spice");
}

#[test]
fn choam_karama_discard_reads_choam_using() {
    let g = game();
    let e = KarmaChoamDiscard::default();
    e.initialize(&g, Some(Faction::Choam));
    let rendered = text(&e.get_message(&g));
    assert!(rendered.starts_with("CHOAM using Karama, CHOAM discard"), "{rendered}");
}

#[test]
fn automation_rules_are_described() {
    let g = game();
    let message = |action: ItemAction, rule_type: AutomationRuleType, by: Option<Faction>| {
        let mut e = AutomationConfigured::default();
        e.action = action;
        e.rule_type = rule_type;
        e.bidding_above_amount = 5;
        e.initialize(&g, by);
        text(&e.get_message(&g))
    };
    assert_eq!(
        message(ItemAction::Create, AutomationRuleType::BiddingPassAboveAmount, Some(Faction::Atreides)),
        "Atreides add an automation rule: Auto pass if bid is equal to or higher than 5 Spice"
    );
    assert_eq!(message(ItemAction::Create, AutomationRuleType::Unknown, Some(Faction::Atreides)), "Atreides add an automation rule");
    // A seatless host has no player to describe the rule for.
    assert_eq!(message(ItemAction::Create, AutomationRuleType::RevivalAutoClaimFreeRevival, None), "None add an automation rule");
    assert_eq!(message(ItemAction::Delete, AutomationRuleType::Unknown, Some(Faction::Atreides)), "Atreides remove an automation rule");
    assert_eq!(message(ItemAction::Update, AutomationRuleType::CharityAutoClaim, Some(Faction::Atreides)), "Atreides change an automation rule");
}

#[test]
fn nexus_card_plays_name_the_card_type_once() {
    let mut g = game();
    let voice = Voice { type_: Some(TreacheryCardType::Laser), ..Default::default() };
    let by = treachery_engine::events::By::new(&g, Faction::Atreides);
    treachery_engine::events::EventRules::apply(&voice, &mut g, by).unwrap();
    let log = log(&g);
    assert!(log.iter().any(|m| m == "Atreides play Harkonnen Nexus Secret Ally to use Voice"), "{log:?}");
}

#[test]
fn ornithopter_and_discovery_texts_have_single_spaces() {
    let g = game();
    let flight = |three: bool| {
        let mut e = FlightUsed::default();
        e.move_three_territories = three;
        text(&e.describe(&g, By::new(&g, Faction::Atreides)))
    };
    assert!(flight(false).ends_with(" to move an additional group of forces"), "{}", flight(false));
    assert!(flight(true).ends_with(" to gain movement speed") && !flight(true).contains("  "), "{}", flight(true));
    let e = FlightDiscoveryUsed::default();
    let t = text(&e.describe(&g, By::new(&g, Faction::Atreides)));
    assert!(t.ends_with(" discovery token to gain movement speed"), "{t}");
}

#[test]
fn a_karama_card_play_has_single_spaces() {
    let g = game();
    let mut e = Karma::default();
    e.card = TreacheryCardManager::items().iter().copied().find(|c| c.type_() == TreacheryCardType::Karma);
    e.initialize(&g, Some(Faction::Atreides));
    assert_eq!(text(&e.get_message(&g)), "Atreides play a Karama card");
}

#[test]
fn the_spice_card_prescience_deal_has_a_space() {
    let t = text(&Deal::express(DealType::ShareResourceDeckPrescience, Part::Null));
    assert_eq!(t, "Share prescience of the top Spice card");
}
