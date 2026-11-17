//! The phases that interrupt a shipment, move, worm ride, Caravan or revival: Bene Gesserit
//! fighters, a Moritani terror token or an Ecaz ambassador in the destination
//! (`BeneGesseritIntrudedByX`, `TerrorTriggeredByX`, `AmbassadorTriggeredByX`).
//!
//! Each pair of tests pins one phase: `enters` submits the event that intrudes and checks the
//! phase and who is asked to react, and `resumes` reacts (Bene Gesserit stay fighters, Moritani
//! and Ecaz pass) and checks where the game goes on. Fuzz games reach several of these phases in
//! only a few games per 200 (rust/docs/native-harness.md, "Phases the fuzzer doesn't reach"),
//! and wave 5 of rust/docs/rewrite-plan.md replaces how they suspend and resume, so these tests
//! hold today's behavior in place. The phases entered while Bene Gesserit accompany a shipment
//! are in phases_unreached.rs.

mod intrusion_table;

use intrusion_table::*;
use treachery_engine::prelude::*;

#[derive(Clone, Copy, Debug)]
enum Trigger {
    GuildShip,
    NonGuildShip,
    GuildMove,
    NonGuildMove,
    Caravan,
    Revival,
    WormRideA,
    WormRideB,
}

#[derive(Clone, Copy, Debug)]
enum Obstacle {
    BeneGesserit,
    Terror,
    Ambassador,
}

/// A game where the next event, returned with it, puts forces at the returned location, which
/// nobody occupies yet.
fn prepare(trigger: Trigger) -> (Game, String, Location) {
    match trigger {
        Trigger::GuildShip => {
            let g = table();
            let to = open_sand(&g, &[]);
            let e = shipment(GUILD, to, 3);
            (g, e, to)
        }
        Trigger::NonGuildShip => {
            let mut g = table();
            pass_until_shipment_of(&mut g, Faction::Tleilaxu);
            let to = open_sand(&g, &[]);
            let e = shipment(TLEILAXU, to, 3);
            (g, e, to)
        }
        Trigger::GuildMove => {
            let mut g = table();
            accept(&mut g, &shipment_passed(GUILD));
            assert_eq!(phase(&g), Phase::GuildMove);
            let from = g.board.map.tueks_sietch();
            let to = open_neighbour(&g, Faction::Guild, from, 5);
            let e = movement(GUILD, from, to, 5);
            (g, e, to)
        }
        Trigger::NonGuildMove => {
            let mut g = table();
            let from = open_sand(&g, &[]);
            put_forces(&mut g, Faction::Tleilaxu, from, 3);
            pass_until_shipment_of(&mut g, Faction::Tleilaxu);
            accept(&mut g, &shipment_passed(TLEILAXU));
            assert_eq!(phase(&g), Phase::NonGuildMove);
            let to = open_neighbour(&g, Faction::Tleilaxu, from, 3);
            let e = movement(TLEILAXU, from, to, 3);
            (g, e, to)
        }
        Trigger::Caravan => {
            let mut g = table();
            let from = open_sand(&g, &[]);
            put_forces(&mut g, Faction::Tleilaxu, from, 3);
            give_caravan(&mut g, Faction::Tleilaxu);
            pass_until_shipment_of(&mut g, Faction::Tleilaxu);
            let to = open_neighbour(&g, Faction::Tleilaxu, from, 3);
            let e = caravan(TLEILAXU, from, to, 3);
            (g, e, to)
        }
        Trigger::Revival => {
            let mut g = at_resurrection();
            let tleilaxu = idx(&g, Faction::Tleilaxu);
            g.player_mut(tleilaxu).forces_killed = 3;
            let to = open_sand(&g, &[]);
            let e = revival_to(to, 1);
            (g, e, to)
        }
        Trigger::WormRideA | Trigger::WormRideB => {
            let mut g = at_blow(if matches!(trigger, Trigger::WormRideA) { Phase::FremenRidingMonsterA } else { Phase::FremenRidingMonsterB });
            let from = open_sand(&g, &[]);
            put_forces(&mut g, Faction::Fremen, from, 3);
            g.board.monsters.push(MonsterAppearance::new(from.territory(), false));
            let to = open_sand(&g, &[from.territory()]);
            let e = worm_ride(from, to, 3);
            (g, e, to)
        }
    }
}

fn give_caravan(g: &mut Game, f: Faction) {
    let card = *TreacheryCardManager::get_cards_in_and_outside_play()
        .iter()
        .find(|c| c.type_() == TreacheryCardType::Caravan)
        .expect("a Caravan card");
    g.decks.treachery_deck.as_mut().unwrap().items.retain(|c| *c != card);
    let p = idx(g, f);
    g.player_mut(p).treachery_cards.push(card);
}

fn obstruct(g: &mut Game, obstacle: Obstacle, at: Location) {
    match obstacle {
        Obstacle::BeneGesserit => put_bene_gesserit_fighters(g, at, 2),
        Obstacle::Terror => put_terror(g, at.territory(), TerrorType::Robbery),
        Obstacle::Ambassador => {
            let a = an_ambassador(g);
            put_ambassador(g, at.territory(), a);
        }
    }
}

fn reaction(obstacle: Obstacle) -> (EventType, Faction, String) {
    match obstacle {
        Obstacle::BeneGesserit => (EventType::BeneGesseritFlip, Faction::BeneGesserit, flip(false)),
        Obstacle::Terror => (EventType::TerrorRevealed, Faction::Moritani, terror_passed()),
        Obstacle::Ambassador => (EventType::AmbassadorActivated, Faction::Ecaz, ambassador_passed()),
    }
}

/// Submits the intruding event and checks the phase it enters.
fn enter(trigger: Trigger, obstacle: Obstacle, entered: Phase) -> Game {
    let (mut g, event, to) = prepare(trigger);
    obstruct(&mut g, obstacle, to);
    let before = phase(&g);
    accept(&mut g, &event);
    assert_eq!(phase(&g), entered, "{trigger:?} into {obstacle:?}, from {before:?}");
    assert_eq!(g.turn.intrusions.len(), 1, "one intrusion waits");
    let (reaction_type, reacting, _) = reaction(obstacle);
    assert_eq!(asked(&mut g, reaction_type), [reacting], "who may react");
    g
}

/// Reacts to the intrusion and checks the phase the game goes on in.
fn resume(trigger: Trigger, obstacle: Obstacle, entered: Phase, resumed: Phase) -> Game {
    let mut g = enter(trigger, obstacle, entered);
    let (_, _, reaction) = reaction(obstacle);
    accept(&mut g, &reaction);
    assert_eq!(phase(&g), resumed, "after {obstacle:?} reacted to {trigger:?}");
    assert!(g.turn.intrusions.is_empty());
    g
}

macro_rules! intrusion {
    ($name:ident, $trigger:ident, $obstacle:ident, $entered:ident, $resumed:ident, |$g:ident| $after:expr) => {
        mod $name {
            use super::*;

            #[test]
            fn enters() {
                enter(Trigger::$trigger, Obstacle::$obstacle, Phase::$entered);
            }

            #[test]
            fn resumes() {
                #[allow(unused_mut)]
                let mut $g = resume(Trigger::$trigger, Obstacle::$obstacle, Phase::$entered, Phase::$resumed);
                $after;
            }
        }
    };
}

// The Guild's shipment: Bene Gesserit may accompany it next.
fn bene_gesserit_may_accompany(g: &mut Game) {
    assert_eq!(asked(g, EventType::BeneGesseritAccompanies), [Faction::BeneGesserit]);
}

intrusion!(bene_gesserit_by_guild_ship, GuildShip, BeneGesserit, BeneGesseritIntrudedByGuildShip, BeneGesseritAccompaniesGuildShip, |g| bene_gesserit_may_accompany(&mut g));
intrusion!(terror_by_guild_ship, GuildShip, Terror, TerrorTriggeredByGuildShip, BeneGesseritAccompaniesGuildShip, |g| bene_gesserit_may_accompany(&mut g));
intrusion!(ambassador_by_guild_ship, GuildShip, Ambassador, AmbassadorTriggeredByGuildShip, BeneGesseritAccompaniesGuildShip, |g| bene_gesserit_may_accompany(&mut g));

intrusion!(bene_gesserit_by_non_guild_ship, NonGuildShip, BeneGesserit, BeneGesseritIntrudedByNonGuildShip, BeneGesseritAccompaniesNonGuildShip, |g| bene_gesserit_may_accompany(&mut g));
intrusion!(terror_by_non_guild_ship, NonGuildShip, Terror, TerrorTriggeredByNonGuildShip, BeneGesseritAccompaniesNonGuildShip, |g| bene_gesserit_may_accompany(&mut g));
intrusion!(ambassador_by_non_guild_ship, NonGuildShip, Ambassador, AmbassadorTriggeredByNonGuildShip, BeneGesseritAccompaniesNonGuildShip, |g| bene_gesserit_may_accompany(&mut g));

// The Guild's move: the first other faction ships next.
fn first_non_guild_ships(g: &mut Game) {
    let shippers = asked(g, EventType::Shipment);
    assert_eq!(shippers.len(), 1);
    assert_ne!(shippers[0], Faction::Guild);
}

intrusion!(bene_gesserit_by_guild_move, GuildMove, BeneGesserit, BeneGesseritIntrudedByGuildMove, NonGuildShip, |g| first_non_guild_ships(&mut g));
intrusion!(terror_by_guild_move, GuildMove, Terror, TerrorTriggeredByGuildMove, NonGuildShip, |g| first_non_guild_ships(&mut g));
intrusion!(ambassador_by_guild_move, GuildMove, Ambassador, AmbassadorTriggeredByGuildMove, NonGuildShip, |g| first_non_guild_ships(&mut g));

// Tleilaxu's move: the next faction in order ships.
fn next_after_tleilaxu_ships(g: &mut Game) {
    let shippers = asked(g, EventType::Shipment);
    assert_eq!(shippers.len(), 1);
    assert!(g.turn.has_acted_or_passed.contains(&Faction::Tleilaxu));
    assert!(!g.turn.has_acted_or_passed.contains(&shippers[0]));
}

intrusion!(bene_gesserit_by_non_guild_move, NonGuildMove, BeneGesserit, BeneGesseritIntrudedByNonGuildMove, NonGuildShip, |g| next_after_tleilaxu_ships(&mut g));
intrusion!(terror_by_non_guild_move, NonGuildMove, Terror, TerrorTriggeredByNonGuildMove, NonGuildShip, |g| next_after_tleilaxu_ships(&mut g));
intrusion!(ambassador_by_non_guild_move, NonGuildMove, Ambassador, AmbassadorTriggeredByNonGuildMove, NonGuildShip, |g| next_after_tleilaxu_ships(&mut g));

// Caravan, played instead of shipping: Tleilaxu still ship.
fn tleilaxu_still_ship(g: &mut Game) {
    assert_eq!(asked(g, EventType::Shipment), [Faction::Tleilaxu]);
}

intrusion!(bene_gesserit_by_caravan, Caravan, BeneGesserit, BeneGesseritIntrudedByCaravan, NonGuildShip, |g| tleilaxu_still_ship(&mut g));
intrusion!(terror_by_caravan, Caravan, Terror, TerrorTriggeredByCaravan, NonGuildShip, |g| tleilaxu_still_ship(&mut g));
intrusion!(ambassador_by_caravan, Caravan, Ambassador, AmbassadorTriggeredByCaravan, NonGuildShip, |g| tleilaxu_still_ship(&mut g));

// A revival to the planet: back to the revival phase, where Tleilaxu, with two forces left in
// the tanks, may revive again.
fn back_to_revivals(g: &mut Game) {
    assert_eq!(asked(g, EventType::Revival), [Faction::Tleilaxu]);
}

intrusion!(bene_gesserit_by_revival, Revival, BeneGesserit, BeneGesseritIntrudedByRevival, Resurrection, |g| back_to_revivals(&mut g));
intrusion!(terror_by_revival, Revival, Terror, TerrorTriggeredByRevival, Resurrection, |g| back_to_revivals(&mut g));
intrusion!(ambassador_by_revival, Revival, Ambassador, AmbassadorTriggeredByRevival, Resurrection, |g| back_to_revivals(&mut g));

// The first worm ride, with no other worm to ride: the spice blow goes on with deck B.
fn after_ride_a(g: &mut Game) {
    assert!(g.board.monsters.is_empty());
    let log = log(g);
    assert!(log.iter().any(|m| m.contains("Deck B")), "{log:?}");
}

intrusion!(bene_gesserit_by_worm_ride_a, WormRideA, BeneGesserit, BeneGesseritIntrudedByFremenRidingMonsterA, BlowReport, |g| after_ride_a(&mut g));
intrusion!(terror_by_worm_ride_a, WormRideA, Terror, TerrorTriggeredByFremenRidingMonsterA, BlowReport, |g| after_ride_a(&mut g));
intrusion!(ambassador_by_worm_ride_a, WormRideA, Ambassador, AmbassadorTriggeredByFremenRidingMonsterA, BlowReport, |g| after_ride_a(&mut g));

// The second worm ride: the spice blow ends.
fn after_ride_b(g: &mut Game) {
    assert!(g.board.monsters.is_empty());
    let log = log(g);
    assert!(!log.iter().any(|m| m.contains("Deck B")), "{log:?}");
}

intrusion!(bene_gesserit_by_worm_ride_b, WormRideB, BeneGesserit, BeneGesseritIntrudedByFremenRidingMonsterB, BlowReport, |g| after_ride_b(&mut g));
intrusion!(terror_by_worm_ride_b, WormRideB, Terror, TerrorTriggeredByFremenRidingMonsterB, BlowReport, |g| after_ride_b(&mut g));
intrusion!(ambassador_by_worm_ride_b, WormRideB, Ambassador, AmbassadorTriggeredByFremenRidingMonsterB, BlowReport, |g| after_ride_b(&mut g));

// Alliances offered instead of terror or by the Ecaz ambassador: the offer interrupts the
// intrusion, and the game goes back to it when the offer is answered.

fn terror_offering_alliance() -> String {
    format!(
        r#"{{"by":"{MORITANI}","event":{{"type":"TerrorRevealed","play":{{"allianceOffered":true,"terrorType":null,"forcesInSneakAttack":0,"sneakAttackTo":null,"robberyTakesCard":false,"cardToGiveInSabotage":null}}}}}}"#
    )
}

fn terror_robbery() -> String {
    format!(
        r#"{{"by":"{MORITANI}","event":{{"type":"TerrorRevealed","play":{{"allianceOffered":false,"terrorType":"Robbery","forcesInSneakAttack":0,"sneakAttackTo":null,"robberyTakesCard":false,"cardToGiveInSabotage":null}}}}}}"#
    )
}

fn alliance_by_terror(f: &str, accept: bool) -> String {
    format!(r#"{{"by":"{f}","event":{{"type":"AllianceByTerror","play":{}}}}}"#, if accept { "{}" } else { "null" })
}

#[test]
fn a_refused_alliance_by_terror_goes_back_to_the_terror_which_must_be_revealed() {
    let mut g = enter(Trigger::NonGuildShip, Obstacle::Terror, Phase::TerrorTriggeredByNonGuildShip);
    assert!(TerrorRevealed::may_pass(&g));
    accept(&mut g, &terror_offering_alliance());
    assert_eq!(phase(&g), Phase::AllianceByTerror);
    assert_eq!(asked(&mut g, EventType::AllianceByTerror), [Faction::Tleilaxu]);

    accept(&mut g, &alliance_by_terror(TLEILAXU, false));
    assert_eq!(phase(&g), Phase::TerrorTriggeredByNonGuildShip);
    assert!(!TerrorRevealed::may_pass(&g), "after a refused offer, Moritani must reveal");
    assert!(!TerrorRevealed::may_offer_alliance(&g));

    accept(&mut g, &terror_robbery());
    assert_eq!(phase(&g), Phase::BeneGesseritAccompaniesNonGuildShip);
    assert!(g.turn.intrusions.is_empty());
    assert!(g.interrupts.is_empty());
}

#[test]
fn an_accepted_alliance_by_terror_returns_the_terror_and_goes_on() {
    let mut g = enter(Trigger::NonGuildShip, Obstacle::Terror, Phase::TerrorTriggeredByNonGuildShip);
    accept(&mut g, &terror_offering_alliance());
    accept(&mut g, &alliance_by_terror(TLEILAXU, true));
    assert_eq!(g.player(idx(&g, Faction::Tleilaxu)).ally, Some(Faction::Moritani));
    assert_eq!(phase(&g), Phase::BeneGesseritAccompaniesNonGuildShip);
    assert!(g.turn.intrusions.is_empty());
}

fn ecaz_offering_alliance() -> String {
    format!(r#"{{"by":"{ECAZ}","event":{{"type":"AmbassadorActivated","play":{{"ecazOfferAlliance":true,"ecazGiveVidalToAlly":false,"ecazTakeVidal":false}}}}}}"#)
}

fn alliance_by_ambassador(f: &str, accept: bool) -> String {
    format!(r#"{{"by":"{f}","event":{{"type":"AllianceByAmbassador","play":{}}}}}"#, if accept { "{}" } else { "null" })
}

fn after_ecaz_ambassador(accept_offer: bool) -> Game {
    let (mut g, event, to) = prepare(Trigger::NonGuildShip);
    put_ambassador(&mut g, to.territory(), Ambassador::Ecaz);
    accept(&mut g, &event);
    assert_eq!(phase(&g), Phase::AmbassadorTriggeredByNonGuildShip);
    accept(&mut g, &ecaz_offering_alliance());
    assert_eq!(phase(&g), Phase::AllianceByAmbassador);
    assert_eq!(asked(&mut g, EventType::AllianceByAmbassador), [Faction::Tleilaxu]);
    accept(&mut g, &alliance_by_ambassador(TLEILAXU, accept_offer));
    g
}

#[test]
fn an_alliance_by_ambassador_goes_on_from_the_shipment_either_way() {
    let g = after_ecaz_ambassador(true);
    assert_eq!(g.player(idx(&g, Faction::Tleilaxu)).ally, Some(Faction::Ecaz));
    assert_eq!(phase(&g), Phase::BeneGesseritAccompaniesNonGuildShip);
    assert!(g.interrupts.is_empty());

    let g = after_ecaz_ambassador(false);
    assert_eq!(g.player(idx(&g, Faction::Tleilaxu)).ally, None);
    assert_eq!(phase(&g), Phase::BeneGesseritAccompaniesNonGuildShip);
    assert!(g.interrupts.is_empty());
}
