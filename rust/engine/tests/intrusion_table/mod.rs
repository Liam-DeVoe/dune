//! A game set up for intrusions, for intrusion_phases.rs and phases_unreached.rs.
//!
//! `table()` seats Bene Gesserit, Ecaz, Moritani, the Guild, Fremen and Tleilaxu under the
//! advanced rules with homeworlds, skips the rest of setup and stops at the start of the shipment
//! and move phase of turn 1. Tests then place the forces, terror tokens and ambassadors they need
//! by hand and submit events as JSON with validation, as the server does. Events go through JSON
//! so the tests don't depend on how the event structs are built.

#![allow(dead_code)]


use treachery_engine::enum_type::GameEnum;
use treachery_legacy as json;
use treachery_engine::prelude::*;

pub const BENE_GESSERIT: i32 = 60;
pub const ECAZ: i32 = 110;
pub const MORITANI: i32 = 120;
pub const GUILD: i32 = 50;
pub const FREMEN: i32 = 10;
pub const TLEILAXU: i32 = 80;
pub const CHOAM: i32 = 90;
pub const ATREIDES: i32 = 20;

/// The sector the storm is in. Nothing the tests use lies in it.
pub const STORM: Sector = Sector::new(9);

/// The game at the start of the shipment and move phase of turn 1: the Guild's shipment.
pub fn table() -> Game {
    table_with(|_| {})
}

/// [`table`] with the rules changed by `change_rules`.
pub fn table_with(change_rules: impl FnOnce(&mut Vec<Rule>)) -> Game {
    let mut g = seated(change_rules);
    g.turn.current_main_phase = MainPhase::Resurrection;
    g.turn.current_phase = Phase::ResurrectionReport;

    // ResurrectionReport -> BeginningOfShipAndMove -> the Guild's shipment.
    host_ends_phase(&mut g);
    assert_eq!(g.turn.current_phase, Phase::BeginningOfShipAndMove);
    host_ends_phase(&mut g);
    assert_eq!(g.turn.current_phase, Phase::GuildShip);
    g
}

/// The game in the revival phase of turn 1.
pub fn at_resurrection() -> Game {
    let mut g = seated(|_| {});
    g.turn.current_main_phase = MainPhase::Resurrection;
    g.turn.current_phase = Phase::Resurrection;
    g
}

/// The game in the spice blow of turn 2, in `phase`.
pub fn at_blow(phase: Phase) -> Game {
    let mut g = seated(|_| {});
    g.turn.current_turn = Turn::new(2);
    g.turn.current_main_phase = MainPhase::Blow;
    g.turn.current_phase = phase;
    g
}

/// The factions seated with their forces, spice, leaders and ambassadors, in no phase yet.
pub fn seated(change_rules: impl FnOnce(&mut Vec<Rule>)) -> Game {
    let codes = [BENE_GESSERIT, ECAZ, MORITANI, GUILD, FREMEN, TLEILAXU, CHOAM, ATREIDES];
    let factions = [
        Faction::BeneGesserit,
        Faction::Ecaz,
        Faction::Moritani,
        Faction::Guild,
        Faction::Fremen,
        Faction::Tleilaxu,
        Faction::Choam,
        Faction::Atreides,
    ];
    assert_eq!(codes, factions.map(|f| f.value()), "the faction numbers in event JSON");
    seated_with(&factions[..6], change_rules)
}

/// [`seated`] with other factions.
pub fn seated_with(factions: &[Faction], change_rules: impl FnOnce(&mut Vec<Rule>)) -> Game {
    let mut g = Game::of_version(Game::LATEST_VERSION);
    let mut s = GameSettings::new();
    s.number_of_players = factions.len() as i32;
    s.allowed_factions_in_play = Some(factions.to_vec());
    let mut rules = Game::ruleset_definition()[&Ruleset::AdvancedGame].clone();
    rules.extend([Rule::Homeworlds, Rule::BeneGesseritAccompaniesToShipmentLocation]);
    change_rules(&mut rules);
    s.initial_rules = Some(rules);
    let mut establish = EstablishPlayers::default();
    establish.seed = 7;
    establish.settings = Some(s);
    assert!(g.play_for_message(HistoryEntry::new(None, establish), false, true).is_none());
    // Out of faction trading into setup: leaders and Ecaz's ambassadors are assigned.
    host_ends_phase(&mut g);
    assert_eq!(g.turn.current_main_phase, MainPhase::Setup);

    // The rest of setup by hand: forces on the homeworlds, starting spice, and the Guild's,
    // Ecaz's and Bene Gesserit's starting forces on the planet. Then everyone gets 20 spice.
    for p in g.player_indices().collect::<Vec<_>>() {
        g.setup_player_homeworld(p).unwrap();
        g.setup_player_spice_and_forces_on_planet(p).unwrap();
        g.player_mut(p).resources = 20;
    }
    g.board.sector_in_storm = Some(STORM);
    g.turn.current_turn = Turn::new(1);
    g
}

/// Players pass their shipments and moves until it's `f`'s turn to ship (not the Guild).
pub fn pass_until_shipment_of(g: &mut Game, f: Faction) {
    for _ in 0..30 {
        let shippers = asked(g, EventType::Shipment);
        match phase(g) {
            Phase::NonGuildShip if shippers == [f] => return,
            Phase::GuildShip => accept(g, &shipment_passed(GUILD)),
            Phase::GuildMove => accept(g, &movement_passed(GUILD)),
            Phase::NonGuildShip => accept(g, &shipment_passed(shippers[0].value())),
            Phase::NonGuildMove => {
                let movers = asked(g, EventType::Move);
                accept(g, &movement_passed(movers[0].value()));
            }
            p => panic!("{p:?} while passing to {f:?}'s shipment"),
        }
    }
    panic!("{f:?} never got to ship");
}

pub fn host_ends_phase(g: &mut Game) {
    let e = HistoryEntry::new(None, EndPhase::default());
    assert_eq!(g.play_for_message(e, true, true).map(|m| m.plain_text()), None, "EndPhase in {:?}", g.turn.current_phase);
}

pub fn idx(g: &Game, f: Faction) -> PlayerId {
    g.get_player(Some(f)).expect("faction in game")
}

/// Submits an event given as JSON (without `Time`), with validation. The refusal, if any.
pub fn submit(g: &mut Game, event_json: &str) -> Option<String> {
    let text = format!("{},\"Time\":\"2026-11-15T00:00:00+00:00\"}}", event_json.trim_end().strip_suffix('}').expect("an object"));
    let e: HistoryEntry = json::deserialize(&text).unwrap_or_else(|e| panic!("{text}: {e:?}"));
    g.play_for_message(e, true, false).map(|m| m.plain_text())
}

/// Submits an event and fails the test if it's refused.
pub fn accept(g: &mut Game, event_json: &str) {
    let phase = g.turn.current_phase;
    if let Some(refusal) = submit(g, event_json) {
        panic!("refused in {phase:?}: {refusal}\n{event_json}");
    }
}

pub fn phase(g: &Game) -> Phase {
    g.turn.current_phase
}

/// Who may submit an event of this type now.
pub fn asked(g: &mut Game, t: EventType) -> Vec<Faction> {
    let players: Vec<PlayerId> = g.player_indices().collect();
    let mut result = Vec::new();
    for p in players {
        if g.get_applicable_events(Some(p), false).contains(&t) {
            result.push(g.player(p).faction.expect("faction"));
        }
    }
    result
}

/// A sand location nobody occupies, outside the storm, with no terror token or ambassador in its
/// territory and not in one of `avoid`. Each call with the same `avoid` gives the same location.
pub fn open_sand(g: &Game, avoid: &[Territory]) -> Location {
    g.board
        .map
        .locations(false)
        .find(|l| {
            let t = l.territory();
            !t.is_stronghold()
                && !t.is_homeworld()
                && !t.is_discovery()
                && *l != g.board.map.polar_sink()
                && l.visible(&g.board.map)
                && t.locations().iter().all(|x| x.sector(&g.board.map) != g.board.sector_in_storm)
                && g.players.iter().all(|p| p.any_forces_in_territory(Some(t)) == 0)
                && g.terror_in(t).next().is_none()
                && g.ambassador_in(t).is_none()
                && !avoid.contains(&t)
        })
        .expect("an open sand location")
}

/// Somewhere `f`'s forces at `from` may move to, one territory away, that nobody occupies.
pub fn open_neighbour(g: &Game, f: Faction, from: Location, forces: i32) -> Location {
    let p = idx(g, f);
    let b = Battalion::new(f, u16::try_from(forces).expect("forces"), 0, from);
    PlacementEvent::valid_targets(g, p, from, &b)
        .into_iter()
        .find(|l| {
            let t = l.territory();
            t != from.territory()
                && !t.is_stronghold()
                && !t.is_homeworld()
                && !t.is_discovery()
                && *l != g.board.map.polar_sink()
                && t.locations().iter().all(|x| x.sector(&g.board.map) != g.board.sector_in_storm)
                && g.players.iter().all(|p| p.any_forces_in_territory(Some(t)) == 0)
                && g.terror_in(t).next().is_none()
                && g.ambassador_in(t).is_none()
        })
        .expect("an open neighbour")
}

/// Puts forces of `f` from its reserves at `l`.
pub fn put_forces(g: &mut Game, f: Faction, l: Location, n: i32) {
    let p = idx(g, f);
    Player::add_forces(g, p, l, n, true).unwrap();
    assert_eq!(g.player(p).forces_in(Some(l)), n, "{f:?} forces at {l:?}");
}

/// Bene Gesserit fighters (not advisors) at `l`.
pub fn put_bene_gesserit_fighters(g: &mut Game, l: Location, n: i32) {
    put_forces(g, Faction::BeneGesserit, l, n);
}

/// A Moritani terror token in `t`.
pub fn put_terror(g: &mut Game, t: Territory, ty: TerrorType) {
    let i = g.board.unplaced_terror_tokens.iter().position(|x| *x == ty).expect("an unplaced token");
    g.board.unplaced_terror_tokens.remove(i);
    g.board.terror_on_planet.insert(ty, t);
}

/// An Ecaz ambassador in `t`.
pub fn put_ambassador(g: &mut Game, t: Territory, a: Ambassador) {
    let ecaz = idx(g, Faction::Ecaz);
    let ambassadors = &mut g.player_mut(ecaz).ambassadors;
    let i = ambassadors.iter().position(|x| *x == a).unwrap_or_else(|| panic!("Ecaz hold {a:?}: {ambassadors:?}"));
    ambassadors.remove(i);
    g.board.ambassadors_on_planet.insert(t, a);
}

/// The ambassador put down by the tests: one that does nothing for the factions that trigger it
/// here, so passing on it is the only choice that matters.
pub fn an_ambassador(g: &Game) -> Ambassador {
    let ecaz = idx(g, Faction::Ecaz);
    *g.player(ecaz)
        .ambassadors
        .iter()
        .find(|a| **a != Ambassador::Ecaz)
        .expect("an ambassador other than Ecaz's own")
}

// Events, as JSON.

pub fn shipment(f: i32, to: Location, forces: i32) -> String {
    format!(
        r#"{{"$type":"Shipment","ShipmentType":0,"ForceAmount":{forces},"SpecialForceAmount":0,"SmuggledAmount":0,"SmuggledSpecialAmount":0,"AllyContributionAmount":0,"UseRicheseSecretAlly":false,"_toId":{},"_fromId":-1,"_noFieldValue":0,"_cunningNoFieldValue":0,"_karmaCardId":-1,"_forceLocations":"","Passed":false,"Initiator":{f}}}"#,
        to.id()
    )
}

pub fn shipment_passed(f: i32) -> String {
    format!(
        r#"{{"$type":"Shipment","ShipmentType":0,"ForceAmount":0,"SpecialForceAmount":0,"SmuggledAmount":0,"SmuggledSpecialAmount":0,"AllyContributionAmount":0,"UseRicheseSecretAlly":false,"_toId":-1,"_fromId":-1,"_noFieldValue":0,"_cunningNoFieldValue":0,"_karmaCardId":-1,"_forceLocations":"","Passed":true,"Initiator":{f}}}"#
    )
}

pub fn movement(f: i32, from: Location, to: Location, forces: i32) -> String {
    format!(r#"{{"$type":"Move","AsAdvisors":false,"_toId":{},"_forceLocations":"{}:{forces}|0","Passed":false,"Initiator":{f}}}"#, to.id(), from.id())
}

pub fn movement_passed(f: i32) -> String {
    format!(r#"{{"$type":"Move","AsAdvisors":false,"_toId":-1,"_forceLocations":"","Passed":true,"Initiator":{f}}}"#)
}

pub fn caravan(f: i32, from: Location, to: Location, forces: i32) -> String {
    format!(r#"{{"$type":"Caravan","AsAdvisors":false,"_toId":{},"_forceLocations":"{}:{forces}|0","Passed":false,"Initiator":{f}}}"#, to.id(), from.id())
}

pub fn worm_ride(from: Location, to: Location, forces: i32) -> String {
    format!(
        r#"{{"$type":"FremenRidesMonster","ForcesFromReserves":0,"SpecialForcesFromReserves":0,"_toId":{},"_forceLocations":"{}:{forces}|0","Passed":false,"Initiator":{FREMEN}}}"#,
        to.id(),
        from.id()
    )
}

/// Tleilaxu revive `forces` and put them all at `to`.
pub fn revival_to(to: Location, forces: i32) -> String {
    format!(
        r#"{{"$type":"Revival","AmountOfForces":{forces},"AmountOfSpecialForces":0,"ExtraForcesPaidByEmperor":0,"ExtraSpecialForcesPaidByEmperor":0,"NumberOfForcesInLocation":{forces},"NumberOfSpecialForcesInLocation":0,"AssignSkill":false,"UsesEmperorSecretAlly":false,"_locationId":{},"_heroId":-1,"Initiator":{TLEILAXU}}}"#,
        to.id()
    )
}

pub fn accompany(to: Location) -> String {
    format!(r#"{{"$type":"BeneGesseritAccompanies","Accompanies":true,"ExtraAdvisor":false,"_targetId":{},"Initiator":{BENE_GESSERIT}}}"#, to.id())
}

pub fn dont_accompany() -> String {
    format!(r#"{{"$type":"BeneGesseritAccompanies","Accompanies":false,"ExtraAdvisor":false,"_targetId":-1,"Initiator":{BENE_GESSERIT}}}"#)
}

/// Bene Gesserit answer an intrusion. `as_advisors` false keeps them fighters.
pub fn flip(as_advisors: bool) -> String {
    format!(r#"{{"$type":"BeneGesseritFlip","AsAdvisors":{as_advisors},"Initiator":{BENE_GESSERIT}}}"#)
}

pub fn terror_passed() -> String {
    format!(
        r#"{{"$type":"TerrorRevealed","AllianceOffered":false,"Type":0,"ForcesInSneakAttack":0,"RobberyTakesCard":false,"_sneakAttackTo":-1,"_cardToGiveInSabotageId":-1,"Passed":true,"Initiator":{MORITANI}}}"#
    )
}

pub fn ambassador_passed() -> String {
    format!(r#"{{"$type":"AmbassadorActivated","Passed":true,"Initiator":{ECAZ}}}"#)
}

pub fn take_losses(f: i32, forces: i32) -> String {
    format!(r#"{{"$type":"TakeLosses","ForceAmount":{forces},"SpecialForceAmount":0,"UseUselessCard":false,"Initiator":{f}}}"#)
}

pub fn ally(g: &mut Game, a: Faction, b: Faction) {
    let (pa, pb) = (idx(g, a), idx(g, b));
    g.player_mut(pa).ally = Some(b);
    g.player_mut(pb).ally = Some(a);
}

/// The texts logged in the current report.
pub fn log(g: &Game) -> Vec<String> {
    g.current_report().messages.iter().map(|m| m.plain_text()).collect()
}
