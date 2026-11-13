//! Shipment and movement (`bot/shipment.rs`, `bot/movement.rs`): the decisions, every shipment
//! sub-strategy (called directly, with the decided-shipment state cleared first),
//! `determine_shortage_for_shipment`, `max_potential_force_shortage` and
//! `determine_most_suitable_nearby_location`, with the bot's shipment state and draw count after
//! each call. Calls on one bot run in order, so the shipment state that `determine_shipment`
//! leaves is what `determine_move` and `determine_caravan` read.
//!
//! The games are in `golden/knowledge-*.jsonl.gz`; `golden/b5-*.jsonl.gz` holds the probes and
//! their expected results. The probes reach the sub-strategies whatever the phase and faction.
//!
//! The expected results are snapshots of the current behavior. After a deliberate change, rewrite
//! them with `UPDATE_GOLDENS=1 cargo test --release -p treachery-bots` and review the diff
//! (see `common/mod.rs`).

mod common;

use treachery_bots::bot::EventKind;
use treachery_bots::prelude::*;

use common::{run_probes, Bots};

fn l(x: Option<Location>) -> String {
    match x {
        None => "null".into(),
        Some(l) => format!("L{}", l.id()),
    }
}

fn state(b: &ClassicBot) -> String {
    let s = match &b.decided_shipment {
        None => "null".to_string(),
        Some(s) => treachery_engine::json::serialize(&<Shipment as EventKind>::into_event(s.clone())),
    };
    format!("A{} {} {}", b.decided_shipment_action as i32, s, l(b.final_destination))
}

fn ev<T: EventKind>(b: &ClassicBot, e: Option<Rc<T>>) -> String {
    let e = e.map_or_else(|| "null".to_string(), |e| treachery_engine::json::serialize(&T::into_event(e)));
    format!("{e} {}", state(b))
}

fn reset(b: &mut ClassicBot) {
    b.decided_shipment_action = ShipmentDecision::None;
    b.decided_shipment = None;
    b.final_destination = None;
}

fn loc(game: &Game, s: &str) -> Location {
    game.map.location_lookup().find(s[1..].parse().unwrap()).unwrap()
}

fn call(game: &Game, b: &mut ClassicBot, m: &str, a: &[String]) -> String {
    let i = |k: usize| a[k].parse::<i32>().unwrap();
    let f = |k: usize| a[k].parse::<f32>().unwrap();
    let t = |k: usize| a[k] == "true";
    if m.starts_with("DetermineShipment_") {
        reset(b);
        let plan = match m {
            "DetermineShipment_PreventNormalWin" => b.plan_prevent_normal_win(game, i(0), i(1), f(2), i(3), i(4)),
            "DetermineShipment_ShipDirectlyToSpiceOrDiscoveryAsFremen" => b.plan_fremen_to_spice_or_discovery(game),
            "DetermineShipment_PreventFremenWin" => b.plan_prevent_fremen_win(game),
            "DetermineShipment_StrengthenWeakestStronghold" => b.plan_strengthen_weakest_stronghold(game, t(0), i(1), f(2), t(3)),
            "DetermineShipment_ShipToStrongholdNearSpice" => b.plan_stronghold_near_spice(game),
            "DetermineShipment_TakeVacantStronghold" => b.plan_take_vacant_stronghold(game, i(0), i(1), i(2)),
            "DetermineShipment_AttackEmptyHomeworld" => b.plan_attack_empty_homeworld(game, i(0), i(1)),
            // The probe's second argument (a risk appetite) is unused.
            "DetermineShipment_AttackWeakHMS" => b.plan_attack_weak_hms(game, i(0), i(2), i(3)),
            "DetermineShipment_AttackWeakStronghold" => b.plan_attack_weak_stronghold(game, i(0), i(1), i(2)),
            "DetermineShipment_ShipDirectlyToSpiceOrDiscoveryAsGuildOrGuildAlly" => b.plan_discounted_to_spice_or_discovery(game),
            "DetermineShipment_UnlockMoveBonus" => b.plan_unlock_move_bonus(game, i(0)),
            "DetermineShipment_BackToReserves" => b.plan_back_to_reserves(game),
            "DetermineShipment_DummyAttack" => b.plan_dummy_attack(game, i(0)),
            "DetermineShipment_PolarSinkAsFremen" => b.plan_fremen_to_polar_sink(game),
            _ => panic!("unknown probe method {m}"),
        };
        if let Some(plan) = plan {
            b.adopt_shipment(plan);
        }
        return state(b);
    }
    match m {
        "DetermineShipment" => {
            let s = b.determine_shipment(game);
            ev(b, Some(s))
        }
        "DetermineMove" => {
            let e = b.determine_move(game);
            ev(b, e)
        }
        "DetermineCaravan" => {
            let e = b.determine_caravan(game);
            ev(b, e)
        }
        "DetermineFlightUsed" => ev(b, b.determine_flight_used(game)),
        "DetermineFlightDiscoveryUsed" => ev(b, b.determine_flight_discovery_used(game)),
        "DetermineShortageForShipment" => {
            let p = game.player(b.player);
            let (fr, sfr) = (p.forces_in_reserve(), p.special_forces_in_reserve());
            let s = b.shipment_size(game, 5.5, true, loc(game, &a[0]), None, fr, sfr, 1, 5, t(1));
            let shortage = dotnet_compat::fmt::f32_to_string(s.shortage);
            format!("{shortage} {} {} {} {}", s.forces, s.special_forces, s.no_field_value, s.cunning_no_field_value)
        }
        "MaxPotentialForceShortage" => dotnet_compat::fmt::f32_to_string(b.max_potential_force_shortage(game, t(0), loc(game, &a[1]))),
        "DetermineMostSuitableNearbyLocation" => {
            let from = loc(game, &a[0]);
            let battalion = game.player(b.player).forces_on_planet()[&from].clone();
            l(b.most_suitable_nearby_location(game, from, &battalion, true, true))
        }
        _ => panic!("unknown probe method {m}"),
    }
}

fn run(name: &str) {
    run_probes("b5", name, Bots::Shared, |game, b, m, a| call(game, b, m, a));
}

#[test]
fn b5_00008() {
    run("00008");
}

#[test]
fn b5_77_00001() {
    run("77-00001");
}

#[test]
fn b5_77_00002() {
    run("77-00002");
}

#[test]
fn b5_77_00004() {
    run("77-00004");
}
