//! Phases that fuzz games never rest in, and what happens instead. rust/docs/native-harness.md,
//! "Phases the fuzzer doesn't reach", lists them. Wave 5 of rust/docs/rewrite-plan.md replaces
//! how phases suspend and resume, and the harness can't see these paths, so these tests hold
//! today's behavior in place, phase by phase. The intrusion phases that fuzz games reach only
//! rarely are in intrusion_phases.rs.

mod intrusion_table;

use intrusion_table::*;
use treachery_engine::prelude::*;

/// Moritani loses forces on its homeworld until it is at its low threshold.
fn moritani_to_low_threshold(g: &mut Game) {
    let moritani = idx(g, Faction::Moritani);
    let home = g.player(moritani).home_worlds[0];
    let above = g.player(moritani).forces_in(Some(home.location())) - home.threshold() + 1;
    Player::kill_forces(g, moritani, home.location(), above, 0, false).unwrap();
    assert!(g.has_low_threshold(Faction::Moritani));
}

// Terror triggered while Bene Gesserit accompany a shipment.
//
// `TerrorTriggeredByBeneGesseritAccompaniesNonGuildShip` and `...GuildShip` are never entered:
// `Interrupt::intrusion` counts a token triggered by accompanying forces as triggered by the
// shipment itself, as C# did. The game then goes on to the move without asking Bene Gesserit to
// accompany again.
//
// Bene Gesserit can only accompany into the shipment's territory (or the Polar Sink), so the
// shipment itself mustn't trigger the token: here Moritani, or its ally, ship.

#[test]
fn terror_triggered_by_bene_gesserit_accompanying_a_non_guild_shipment() {
    let mut g = table();
    pass_until_shipment_of(&mut g, Faction::Moritani);
    let to = open_sand(&g, &[]);
    put_terror(&mut g, to.territory(), TerrorType::Robbery);

    accept(&mut g, &shipment(MORITANI, to, 3));
    assert_eq!(phase(&g), Phase::BeneGesseritAccompaniesNonGuildShip, "Moritani's own token");

    accept(&mut g, &accompany(to));
    assert_eq!(phase(&g), Phase::TerrorTriggeredByNonGuildShip);
    assert_eq!(asked(&mut g, EventType::TerrorRevealed), [Faction::Moritani]);
    assert_eq!(g.turn.intrusions.len(), 1);
    assert_eq!(g.turn.intrusions[0].initiator(), Faction::BeneGesserit);

    accept(&mut g, &terror_passed());
    assert_eq!(phase(&g), Phase::NonGuildMove);
    assert_eq!(asked(&mut g, EventType::BeneGesseritAccompanies), []);
    assert_eq!(asked(&mut g, EventType::Move), [Faction::Moritani]);
}

#[test]
fn terror_triggered_by_bene_gesserit_accompanying_the_guilds_shipment() {
    let mut g = table();
    ally(&mut g, Faction::Guild, Faction::Moritani);
    let to = open_sand(&g, &[]);
    put_terror(&mut g, to.territory(), TerrorType::Robbery);

    accept(&mut g, &shipment(GUILD, to, 3));
    assert_eq!(phase(&g), Phase::BeneGesseritAccompaniesGuildShip, "the token of the Guild's ally");

    accept(&mut g, &accompany(to));
    assert_eq!(phase(&g), Phase::TerrorTriggeredByGuildShip);
    assert_eq!(asked(&mut g, EventType::TerrorRevealed), [Faction::Moritani]);
    assert_eq!(g.turn.intrusions[0].initiator(), Faction::BeneGesserit);

    accept(&mut g, &terror_passed());
    assert_eq!(phase(&g), Phase::GuildMove);
    assert_eq!(asked(&mut g, EventType::BeneGesseritAccompanies), []);
    assert_eq!(asked(&mut g, EventType::Move), [Faction::Guild]);
}

/// `check_terror_triggered` lets fewer than 3 forces trigger a token only while Moritani is *not*
/// at its low threshold. Bene Gesserit accompany with 1 or 2 forces, so at the low threshold they
/// never trigger one.
#[test]
fn bene_gesserit_accompanying_dont_trigger_terror_when_moritani_is_at_its_low_threshold() {
    let mut g = table();
    moritani_to_low_threshold(&mut g);
    pass_until_shipment_of(&mut g, Faction::Moritani);
    let to = open_sand(&g, &[]);
    put_terror(&mut g, to.territory(), TerrorType::Robbery);

    accept(&mut g, &shipment(MORITANI, to, 3));
    assert_eq!(phase(&g), Phase::BeneGesseritAccompaniesNonGuildShip);
    accept(&mut g, &accompany(to));
    assert_eq!(phase(&g), Phase::NonGuildMove);
    assert!(g.turn.intrusions.is_empty());
}

// Ambassadors and Bene Gesserit accompanying a shipment.
//
// Bene Gesserit accompany as fighters when they occupy the territory, when it is empty, or when
// the advisors rule is off. Fighters trigger an ambassador, advisors don't (behavior-changes.md,
// "Bene Gesserit fighters that accompany a shipment trigger an ambassador"). These tests use the
// advisors rule off. The fuzzer doesn't reach `AmbassadorTriggeredByBeneGesseritAccompanies*`.

fn bene_gesserit_fighters_accompany_into_an_ambassador(guild: bool) -> (Game, Location, Ambassador) {
    let mut g = table_with(|rules| rules.retain(|r| *r != Rule::BeneGesseritAdvisors));
    if guild {
        ally(&mut g, Faction::Guild, Faction::Ecaz);
    } else {
        pass_until_shipment_of(&mut g, Faction::Ecaz);
    }
    let to = open_sand(&g, &[]);
    let a = an_ambassador(&g);
    put_ambassador(&mut g, to.territory(), a);

    accept(&mut g, &shipment(if guild { GUILD } else { ECAZ }, to, 3));
    let accompanying = if guild { Phase::BeneGesseritAccompaniesGuildShip } else { Phase::BeneGesseritAccompaniesNonGuildShip };
    assert_eq!(phase(&g), accompanying, "the shipment of Ecaz or their ally doesn't trigger it");

    accept(&mut g, &accompany(to));
    let bg = idx(&g, Faction::BeneGesserit);
    assert_eq!((g.player(bg).forces_in(Some(to)), g.player(bg).special_forces_in(Some(to))), (1, 0), "one fighter");
    (g, to, a)
}

#[test]
fn bene_gesserit_fighters_accompanying_a_non_guild_shipment_trigger_an_ambassador() {
    let (mut g, to, a) = bene_gesserit_fighters_accompany_into_an_ambassador(false);
    assert_eq!(phase(&g), Phase::AmbassadorTriggeredByBeneGesseritAccompaniesNonGuildShip);
    assert_eq!(asked(&mut g, EventType::AmbassadorActivated), [Faction::Ecaz]);
    assert_eq!(g.turn.intrusions.len(), 1);
    assert_eq!(g.turn.intrusions[0].initiator(), Faction::BeneGesserit);

    accept(&mut g, &ambassador_passed());
    assert_eq!(phase(&g), Phase::NonGuildMove);
    assert!(g.turn.intrusions.is_empty());
    assert_eq!(g.ambassador_in(to.territory()), Some(a), "passed: the ambassador stays");
}

#[test]
fn bene_gesserit_fighters_accompanying_the_guilds_shipment_trigger_an_ambassador() {
    let (mut g, _, _) = bene_gesserit_fighters_accompany_into_an_ambassador(true);
    assert_eq!(phase(&g), Phase::AmbassadorTriggeredByBeneGesseritAccompaniesGuildShip);
    assert_eq!(asked(&mut g, EventType::AmbassadorActivated), [Faction::Ecaz]);

    accept(&mut g, &ambassador_passed());
    assert_eq!(phase(&g), Phase::GuildMove);
    assert!(g.turn.intrusions.is_empty());
}

#[test]
fn bene_gesserit_advisors_accompanying_a_shipment_dont_trigger_an_ambassador() {
    let mut g = table();
    pass_until_shipment_of(&mut g, Faction::Ecaz);
    let to = open_sand(&g, &[]);
    let a = an_ambassador(&g);
    put_ambassador(&mut g, to.territory(), a);
    accept(&mut g, &shipment(ECAZ, to, 3));
    assert_eq!(phase(&g), Phase::BeneGesseritAccompaniesNonGuildShip);

    accept(&mut g, &accompany(to));
    let bg = idx(&g, Faction::BeneGesserit);
    assert_eq!((g.player(bg).forces_in(Some(to)), g.player(bg).special_forces_in(Some(to))), (0, 1), "one advisor");
    assert_eq!(phase(&g), Phase::NonGuildMove);
    assert!(g.turn.intrusions.is_empty());
}

// BlowA and BlowB.
//
// Neither is ever a resting phase: `enter_blow_a` and `enter_blow_b` draw spice cards until a
// blow and leave the phase within the same event. `StormLosses` also uses `BlowA` as a marker:
// `phase_before_storm_loss == BlowA` means the losses came from the storm phase.

#[test]
fn the_spice_blow_passes_through_blow_a_and_blow_b_in_one_event() {
    let mut g = at_blow(Phase::Thumper);
    host_ends_phase(&mut g);
    let log = log(&g);
    assert!(log.iter().any(|m| m.contains("Deck A")), "{log:?}");
    assert!(log.iter().any(|m| m.contains("Deck B")), "{log:?}");
    assert_eq!(phase(&g), Phase::BlowReport);
}

/// Fremen forces at a location in `sector` that the storm can reach.
fn fremen_in_sector(g: &mut Game, sector: Sector, forces: i32, avoid: &[Territory]) -> Location {
    let l = g
        .board
        .map
        .locations(false)
        .find(|l| {
            l.sector(&g.board.map) == Some(sector)
                && !l.territory().is_stronghold()
                && !g.is_protected_from_storm(*l)
                && l.visible(&g.board.map)
                && !avoid.contains(&l.territory())
                && g.players.iter().all(|p| p.any_forces_in_territory(Some(l.territory())) == 0)
        })
        .expect("a sand location in the sector");
    put_forces(g, Faction::Fremen, l, forces);
    l
}

#[test]
fn storm_losses_taken_in_the_storm_phase_end_in_the_storm_report() {
    let mut g = seated(|_| {});
    g.turn.current_main_phase = MainPhase::Storm;
    g.enter(Phase::MetheorAndStormSpell, false);
    g.board.next_storm_moves = 1;
    let first = fremen_in_sector(&mut g, STORM.moved(1), 4, &[]);
    let second = fremen_in_sector(&mut g, STORM.moved(1), 2, &[first.territory()]);

    host_ends_phase(&mut g);
    assert_eq!(phase(&g), Phase::StormLosses);
    assert_eq!(g.storm.phase_before_storm_loss, Some(Phase::BlowA), "the marker for the storm phase");
    assert_eq!(g.storm.storm_losses_to_take.len(), 2);
    assert_eq!(asked(&mut g, EventType::TakeLosses), [Faction::Fremen]);

    accept(&mut g, &take_losses(FREMEN, 2));
    assert_eq!(phase(&g), Phase::StormLosses, "one loss left");
    accept(&mut g, &take_losses(FREMEN, 1));
    assert_eq!(phase(&g), Phase::StormReport);
    let fremen = idx(&g, Faction::Fremen);
    assert_eq!((g.player(fremen).forces_in(Some(first)), g.player(fremen).forces_in(Some(second))), (2, 1));
}

#[test]
fn storm_losses_taken_during_a_shipment_go_back_to_the_shipment() {
    let mut g = table();
    let flat = g.board.map.the_great_flat();
    g.board.sector_in_storm = flat.sector(&g.board.map);
    // Forces out of the storm, so Fremen have something to move afterwards.
    let elsewhere = open_sand(&g, &[]);
    put_forces(&mut g, Faction::Fremen, elsewhere, 2);
    pass_until_shipment_of(&mut g, Faction::Fremen);

    accept(&mut g, &shipment(FREMEN, flat, 4));
    assert_eq!(phase(&g), Phase::StormLosses);
    assert_eq!(g.storm.phase_before_storm_loss, Some(Phase::NonGuildShip));
    assert_eq!(asked(&mut g, EventType::TakeLosses), [Faction::Fremen]);

    // Back in the shipment, which is over, so the game goes on to Fremen's move.
    accept(&mut g, &take_losses(FREMEN, 2));
    assert_eq!(phase(&g), Phase::NonGuildMove);
    assert_eq!(asked(&mut g, EventType::Move), [Faction::Fremen]);
    let fremen = idx(&g, Faction::Fremen);
    assert_eq!(g.player(fremen).forces_in(Some(flat)), 2);
}

// Two other phases that fuzz games reach in only a few games per 200, and that suspend another
// phase: `Discarding` and `Auditing`.

/// Robbery that takes a card puts Moritani over its hand limit. The terror resolves first, so the
/// discard interrupts the phase the shipment went on to, and the game comes back to it.
#[test]
fn discarding_after_robbery_returns_to_the_phase_after_the_terror() {
    use treachery_engine::enum_type::GameEnum;

    let mut g = table();
    let moritani = idx(&g, Faction::Moritani);
    for _ in 0..4 {
        let card = g.draw_treachery_card().expect("a card");
        g.player_mut(moritani).treachery_cards.push(card);
    }
    pass_until_shipment_of(&mut g, Faction::Tleilaxu);
    let to = open_sand(&g, &[]);
    put_terror(&mut g, to.territory(), TerrorType::Robbery);
    accept(&mut g, &shipment(TLEILAXU, to, 3));
    assert_eq!(phase(&g), Phase::TerrorTriggeredByNonGuildShip);

    let robbery = format!(
        r#"{{"$type":"TerrorRevealed","AllianceOffered":false,"Type":{},"ForcesInSneakAttack":0,"RobberyTakesCard":true,"_sneakAttackTo":-1,"_cardToGiveInSabotageId":-1,"Passed":false,"Initiator":{MORITANI}}}"#,
        TerrorType::Robbery.value()
    );
    accept(&mut g, &robbery);
    assert_eq!(phase(&g), Phase::Discarding);
    assert_eq!(g.phase_under(Interrupt::Discarding), Some(Phase::BeneGesseritAccompaniesNonGuildShip));
    assert_eq!(asked(&mut g, EventType::Discarded), [Faction::Moritani]);
    assert_eq!(g.player(moritani).treachery_cards.len(), 5);

    let card = g.player(moritani).treachery_cards[0];
    accept(&mut g, &format!(r#"{{"$type":"Discarded","_cardId":{},"Initiator":{MORITANI}}}"#, card.id()));
    assert_eq!(phase(&g), Phase::BeneGesseritAccompaniesNonGuildShip);
    assert_eq!(asked(&mut g, EventType::BeneGesseritAccompanies), [Faction::BeneGesserit]);
    assert_eq!(g.player(moritani).treachery_cards.len(), 4);
}

/// After CHOAM's audit the battle goes on to its conclusion when there is a winner and Harkonnen
/// don't need to decide on a capture. The test starts in `Auditing`, as `AuditCancelled` leaves
/// it, in a battle built by hand.
#[test]
fn auditing_goes_on_to_the_battle_conclusion() {
    let mut g = seated_with(&[Faction::Choam, Faction::Atreides], |_| {});
    assert!(g.applicable(Rule::ChoamAuditor));
    let choam = idx(&g, Faction::Choam);
    let atreides = idx(&g, Faction::Atreides);
    let card = g.draw_treachery_card().expect("a card");
    g.player_mut(atreides).treachery_cards.push(card);
    g.turn.current_main_phase = MainPhase::Battle;
    g.enter_battle_state(BattleStep::Auditing, Some(BattleInitiated::default().battle(By::new(&g, Faction::Atreides), EntryId::default())));
    g.fight_mut().winner = Some(Faction::Atreides);
    g.fight_mut().audited_cards = vec![card];
    assert_eq!(asked(&mut g, EventType::Audited), [Faction::Choam]);

    accept(&mut g, &format!(r#"{{"$type":"Audited","Initiator":{CHOAM}}}"#));
    assert_eq!(phase(&g), Phase::BattleConclusion);
    assert!(g.player(choam).known_cards.contains(&card));
}
