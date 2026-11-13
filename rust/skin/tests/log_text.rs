//! Log and event texts that were fixed on purpose after the port (behavior-changes.md:
//! "Missing spaces", "Setting an automation rule", "Other small text quirks").

use std::rc::Rc;

use treachery_engine::shared::Shared;
use treachery_engine::enumerations::{AutomationRuleType, Faction, FactionAdvantage, ItemAction, TreacheryCardType};
use treachery_engine::events::game_event::GameEventMethods;
use treachery_engine::events::{AutomationConfigured, GameEvent, KarmaChoamDiscard, Shipment, Voice};
use treachery_engine::game::{Game, Participation};
use treachery_engine::json;
use treachery_engine::messages::message::{Message, MessagePart};
use treachery_skin::default_skin;

const ESTABLISH: &str = r#"{"$type":"EstablishPlayers","Settings":{"NumberOfPlayers":3,"MaximumTurns":8,"InitialRules":[301,300,65,60,20,1013,15,700,105,80,85,604,90,998,30,701,40,107,10,70,50,25,209,999,112],"AllowedFactionsInPlay":[50,10,120,60,40,100,20,90,70,80,30,110],"AutoOpenEmptySeats":true,"AsyncPlay":false,"AsyncPlayMessageIntervalMinutes":0},"Seed":1073548723,"MaximumNumberOfPlayers":3,"MaximumTurns":8,"ApplicableRules":[301,300,65,60,20,1013,15,700,105,80,85,604,90,998,30,701,40,107,10,70,50,25,209,999,112],"_players":"","_factionsInPlay":"Guild,Fremen,Moritani,BeneGesserit,Emperor,Richese,Atreides,Choam,Ixian,Tleilaxu,Harkonnen,Ecaz","Initiator":0,"Time":"2024-01-01T12:02:17+00:00"}"#;

/// A started three-player game whose players are Atreides (with the Harkonnen Nexus card),
/// Guild and Choam.
fn game() -> Game {
    let mut p = Participation::new();
    p.player_names.insert(100, "Human100".to_string());
    p.seated_players.insert(100, 2);
    let mut g = Game::new_int32_participation(Game::LATEST_VERSION, &Shared::new(p));
    let e: GameEvent = json::deserialize(ESTABLISH).expect("event json");
    e.initialize_game(&g);
    assert!(e.execute(&mut g, true, true).is_none());
    for (player, faction) in g.players.iter_mut().zip([Faction::Atreides, Faction::Guild, Faction::Choam]) {
        player.faction = faction;
        player.nexus = Faction::None;
    }
    g.players[0].nexus = Faction::Harkonnen;
    g
}

fn text(game: &Game, m: &Message) -> String {
    default_skin().message_to_string_in(m, Some(game))
}

fn log(game: &Game) -> Vec<String> {
    game.reports().iter().flat_map(|r| r.messages.iter()).map(|m| text(game, m)).collect()
}

#[test]
fn karama_against_guild_income_has_a_space_before_the_amount() {
    let mut g = game();
    let to = g.map.locations(false).find(|l| l.id() != 0 && !l.territory().is_stronghold()).unwrap();
    let mut s = Shipment::default();
    s.force_amount = 2;
    s.karma_card_id = -1;
    s.to_id = to.id();
    s.initialize(&g, Faction::Atreides);
    let s = Rc::new(s);
    g.prevent(Faction::Atreides, FactionAdvantage::GuildReceiveShipment, false);
    let mut part = MessagePart::express(vec![]);
    s.handle_received_shipment_payment(&mut g, &mut part);
    let rendered = default_skin().message_part_to_string_in(&part, Some(&g));
    assert_eq!(rendered, " → Karama prevents Guild from receiving 4 Spice");
}

#[test]
fn choam_karama_discard_reads_choam_using() {
    let g = game();
    let e = KarmaChoamDiscard::default();
    e.initialize(&g, Faction::Choam);
    let rendered = text(&g, &e.get_message(&g));
    assert!(rendered.starts_with("CHOAM using Karama, CHOAM discard"), "{rendered}");
}

#[test]
fn automation_rules_are_described() {
    let g = game();
    let message = |action: ItemAction, rule_type: AutomationRuleType, by: Faction| {
        let mut e = AutomationConfigured::default();
        e.action = action;
        e.rule_type = rule_type;
        e.bidding_above_amount = 5;
        e.initialize(&g, by);
        text(&g, &e.get_message(&g))
    };
    assert_eq!(
        message(ItemAction::Create, AutomationRuleType::BiddingPassAboveAmount, Faction::Atreides),
        "Atreides add an automation rule: Auto pass if bid is equal to or higher than 5 Spice"
    );
    assert_eq!(message(ItemAction::Create, AutomationRuleType::Unknown, Faction::Atreides), "Atreides add an automation rule");
    // A seatless host has no player to describe the rule for.
    assert_eq!(message(ItemAction::Create, AutomationRuleType::RevivalAutoClaimFreeRevival, Faction::None), "None add an automation rule");
    assert_eq!(message(ItemAction::Delete, AutomationRuleType::Unknown, Faction::Atreides), "Atreides remove an automation rule");
    assert_eq!(message(ItemAction::Update, AutomationRuleType::CharityAutoClaim, Faction::Atreides), "Atreides change an automation rule");
}

#[test]
fn nexus_card_plays_name_the_card_type_once() {
    let mut g = game();
    let mut voice = Voice::default();
    voice.type_ = TreacheryCardType::Laser;
    voice.initialize(&g, Faction::Atreides);
    Rc::new(voice).execute_concrete_event(&mut g).unwrap();
    let log = log(&g);
    assert!(log.iter().any(|m| m == "Atreides play Harkonnen Nexus Secret Ally to use Voice"), "{log:?}");
}
