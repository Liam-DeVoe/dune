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
    Player::kill_forces(g, moritani, home.location(), above, 0, false);
    assert!(g.has_low_threshold(Faction::Moritani));
}

// Terror triggered while Bene Gesserit accompany a shipment.
//
// `TerrorTriggeredByBeneGesseritAccompaniesNonGuildShip` and `...GuildShip` are never entered:
// `determine_next_shipment_and_move_sub_phase_on_terror_triggered` sends a token triggered by
// accompanying forces to the shipment's own phase, as C# did. The game then goes on to the move
// without asking Bene Gesserit to accompany again.
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
    assert_eq!(asked(&mut g, GameEventType::TerrorRevealed), [Faction::Moritani]);
    assert_eq!(g.ship_move.intrusions.len(), 1);
    assert_eq!(g.ship_move.intrusions[0].initiator(), Faction::BeneGesserit);

    accept(&mut g, &terror_passed());
    assert_eq!(phase(&g), Phase::NonGuildMove);
    assert_eq!(asked(&mut g, GameEventType::BeneGesseritAccompanies), []);
    assert_eq!(asked(&mut g, GameEventType::Move), [Faction::Moritani]);
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
    assert_eq!(asked(&mut g, GameEventType::TerrorRevealed), [Faction::Moritani]);
    assert_eq!(g.ship_move.intrusions[0].initiator(), Faction::BeneGesserit);

    accept(&mut g, &terror_passed());
    assert_eq!(phase(&g), Phase::GuildMove);
    assert_eq!(asked(&mut g, GameEventType::BeneGesseritAccompanies), []);
    assert_eq!(asked(&mut g, GameEventType::Move), [Faction::Guild]);
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
    assert!(g.ship_move.intrusions.is_empty());
}

// Ambassadors and Bene Gesserit accompanying a shipment.
//
// `AmbassadorTriggeredByBeneGesseritAccompaniesNonGuildShip` and `...GuildShip` are never
// entered. `check_ambassador_triggered` lets Bene Gesserit trigger an ambassador only when the
// event adds fighters, and `BeneGesseritAccompanies::forces_added_to_location` is always 0: it
// counts the accompanying forces as advisors.
//
// RULES QUESTION for the owner: Bene Gesserit accompany as fighters when they occupy the
// territory, when it is empty, or when the advisors rule is off (`execute_concrete_event`). These
// tests use the last case. Should fighters that accompany a shipment trigger an ambassador? Today
// they don't, and the game goes on to the move.

#[test]
fn bene_gesserit_fighters_accompanying_a_non_guild_shipment_dont_trigger_an_ambassador() {
    let mut g = table_with(|rules| rules.retain(|r| *r != Rule::BeneGesseritAdvisors));
    pass_until_shipment_of(&mut g, Faction::Ecaz);
    let to = open_sand(&g, &[]);
    let a = an_ambassador(&g);
    put_ambassador(&mut g, to.territory(), a);

    accept(&mut g, &shipment(ECAZ, to, 3));
    assert_eq!(phase(&g), Phase::BeneGesseritAccompaniesNonGuildShip, "Ecaz's own ambassador");

    accept(&mut g, &accompany(to));
    let bg = idx(&g, Faction::BeneGesserit);
    assert_eq!((g.player(bg).forces_in(Some(to)), g.player(bg).special_forces_in(Some(to))), (1, 0), "one fighter");
    // Today's behavior; see the rules question above.
    assert_eq!(phase(&g), Phase::NonGuildMove);
    assert!(g.ship_move.intrusions.is_empty());
    assert_eq!(g.ambassador_in(to.territory()), Some(a));
}

#[test]
fn bene_gesserit_fighters_accompanying_the_guilds_shipment_dont_trigger_an_ambassador() {
    let mut g = table_with(|rules| rules.retain(|r| *r != Rule::BeneGesseritAdvisors));
    ally(&mut g, Faction::Guild, Faction::Ecaz);
    let to = open_sand(&g, &[]);
    let a = an_ambassador(&g);
    put_ambassador(&mut g, to.territory(), a);

    accept(&mut g, &shipment(GUILD, to, 3));
    assert_eq!(phase(&g), Phase::BeneGesseritAccompaniesGuildShip, "the ambassador of the Guild's ally");

    accept(&mut g, &accompany(to));
    let bg = idx(&g, Faction::BeneGesserit);
    assert_eq!(g.player(bg).forces_in(Some(to)), 1, "one fighter");
    // Today's behavior; see the rules question above.
    assert_eq!(phase(&g), Phase::GuildMove);
    assert!(g.ship_move.intrusions.is_empty());
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
fn fremen_in_sector(g: &mut Game, sector: i32, forces: i32, avoid: &[Territory]) -> Location {
    let l = g
        .board
        .map
        .locations(false)
        .find(|l| {
            l.sector(&g.board.map) == sector
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
    g.turn.current_phase = Phase::MetheorAndStormSpell;
    g.board.next_storm_moves = 1;
    let first = fremen_in_sector(&mut g, STORM + 1, 4, &[]);
    let second = fremen_in_sector(&mut g, STORM + 1, 2, &[first.territory()]);

    host_ends_phase(&mut g);
    assert_eq!(phase(&g), Phase::StormLosses);
    assert_eq!(g.storm.phase_before_storm_loss, Phase::BlowA, "the marker for the storm phase");
    assert_eq!(g.storm.storm_losses_to_take.len(), 2);
    assert_eq!(asked(&mut g, GameEventType::TakeLosses), [Faction::Fremen]);

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
    assert_eq!(g.storm.phase_before_storm_loss, Phase::NonGuildShip);
    assert_eq!(asked(&mut g, GameEventType::TakeLosses), [Faction::Fremen]);

    // Back in the shipment, which is over, so the game goes on to Fremen's move.
    accept(&mut g, &take_losses(FREMEN, 2));
    assert_eq!(phase(&g), Phase::NonGuildMove);
    assert_eq!(asked(&mut g, GameEventType::Move), [Faction::Fremen]);
    let fremen = idx(&g, Faction::Fremen);
    assert_eq!(g.player(fremen).forces_in(Some(flat)), 2);
}
