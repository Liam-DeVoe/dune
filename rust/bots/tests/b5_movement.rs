//! Package B5 (Bot_Shipment, Bot_Move) against C#: the decisions, every shipment sub-strategy
//! (called directly, with the decided-shipment state cleared first), `DetermineShortageForShipment`,
//! `MaxPotentialForceShortage` and `DetermineMostSuitableNearbyLocation`, with the bot's shipment
//! state and draw count after each call. Calls on one bot run in order, so the shipment state that
//! `DetermineShipment` leaves is what `DetermineMove` and `DetermineCaravan` read.
//!
//! The events come from the knowledge goldens; `golden/b5-*.jsonl.gz` holds only the probes, from
//! `bots/csharp-ref` (see B5.cs):
//!
//! ```sh
//! export DOTNET_ROOT="$HOME/dotnet10" PATH="$HOME/dotnet10:$PATH"
//! cd rust/bots/csharp-ref
//! dotnet run -c Release -- b5 ../tests/golden/knowledge-00008.jsonl.gz ../tests/golden/b5-00008.jsonl.gz 2
//! dotnet run -c Release -- b5 ../tests/golden/knowledge-77-00001.jsonl.gz ../tests/golden/b5-77-00001.jsonl.gz 2
//! dotnet run -c Release -- b5 ../tests/golden/knowledge-77-00002.jsonl.gz ../tests/golden/b5-77-00002.jsonl.gz 3
//! dotnet run -c Release -- b5 ../tests/golden/knowledge-77-00004.jsonl.gz ../tests/golden/b5-77-00004.jsonl.gz 3
//! ```
//!
//! The differ in bots mode checks the decisions where bots really make them; these probes reach
//! the sub-strategies whatever the phase and faction.

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader};

use flate2::read::GzDecoder;
use serde_json::Value;
use treachery_bots::bot::EventKind;
use treachery_bots::prelude::*;
use treachery_engine::game::Participation;

fn l(x: Option<Location>) -> String {
    match x.and_then(CsNull::opt) {
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
            // The C# method takes an unused risk appetite as its second argument.
            "DetermineShipment_AttackWeakHMS" => b.plan_attack_weak_hms(game, i(0), i(2), i(3)),
            "DetermineShipment_AttackWeakStronghold" => b.plan_attack_weak_stronghold(game, i(0), i(1), i(2)),
            "DetermineShipment_ShipDirectlyToSpiceOrDiscoveryAsGuildOrGuildAlly" => b.plan_discounted_to_spice_or_discovery(game),
            "DetermineShipment_UnlockMoveBonus" => b.plan_unlock_move_bonus(game, i(0)),
            "DetermineShipment_BackToReserves" => b.plan_back_to_reserves(game),
            "DetermineShipment_DummyAttack" => b.plan_dummy_attack(game, i(0)),
            "DetermineShipment_PolarSinkAsFremen" => b.plan_fremen_to_polar_sink(game),
            _ => panic!("no Rust call for {m}"),
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
        _ => panic!("no Rust call for {m}"),
    }
}

/// As differ's `replay::participation` (and the oracle's Verifier).
fn participation(o: &Value) -> Participation {
    let int = |v: &Value| v.as_i64().unwrap() as i32;
    let mut p = Participation { bots_speed: int(&o["BotsSpeed"]), ..Default::default() };
    for kv in o["PlayerNames"].as_array().unwrap() {
        p.player_names.insert(int(&kv[0]), kv[1].as_str().unwrap().to_string());
    }
    for kv in o["SeatedPlayers"].as_array().unwrap() {
        p.seated_players.insert(int(&kv[0]), int(&kv[1]));
    }
    for (key, set) in [
        ("Observers", &mut p.observers),
        ("Hosts", &mut p.hosts),
        ("AvailableSeats", &mut p.available_seats),
        ("Kicked", &mut p.kicked),
    ] {
        for v in o[key].as_array().unwrap() {
            set.insert(int(v));
        }
    }
    p
}

fn lines(name: &str) -> impl Iterator<Item = Value> {
    let path = format!("{}/tests/golden/{name}", env!("CARGO_MANIFEST_DIR"));
    let file = std::fs::File::open(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
    BufReader::new(GzDecoder::new(file)).lines().map(|l| serde_json::from_str(&l.unwrap()).unwrap())
}

fn run(name: &str) {
    let mut probes: BTreeMap<i64, Vec<Value>> = BTreeMap::new();
    for o in lines(&format!("b5-{name}.jsonl.gz")) {
        probes.entry(o["n"].as_i64().unwrap()).or_default().push(o);
    }

    let mut game: Option<Game> = None;
    let mut count = 0;
    let mut failures: Vec<String> = Vec::new();

    for o in lines(&format!("knowledge-{name}.jsonl.gz")) {
        match o["k"].as_str().unwrap() {
            "header" => {
                let participation = participation(&o["participation"]);
                game = Some(Game::new_int32_participation(o["version"].as_i64().unwrap() as i32, &Shared::new(participation)));
            }
            "step" => {
                let game = game.as_mut().unwrap();
                let e: GameEvent = treachery_engine::json::deserialize(o["event"].as_str().unwrap()).unwrap();
                e.initialize_game(game);
                game.map.mark_neighbour_cache_stale();
                let (validate, is_host) = (o["validate"].as_bool().unwrap(), o["isHost"].as_bool().unwrap());
                let _ = cs::catch_panic(|| e.execute(game, validate, is_host));

                let n = o["n"].as_i64().unwrap();
                let Some(list) = probes.get(&n) else { continue };
                let game: &Game = game;
                let mut bot: Option<(i64, ClassicBot)> = None;
                // After a difference, the later probes of that bot depend on its draws: skip them.
                let mut skip: Option<i64> = None;
                for p in list {
                    let pi = p["p"].as_i64().unwrap();
                    if skip == Some(pi) {
                        continue;
                    }
                    if !matches!(&bot, Some((bp, _)) if *bp == pi) {
                        game.map.mark_neighbour_cache_stale();
                        let player = game.player_at(pi as usize);
                        let faction = game.player(player).faction;
                        let seed = p["seed"].as_i64().unwrap() as i32;
                        let b = ClassicBot::new(player, BotParameters::for_faction(faction), SharedRandom::from_seed(seed));
                        bot = Some((pi, b));
                    }
                    let b = &mut bot.as_mut().unwrap().1;
                    let m = p["m"].as_str().unwrap();
                    let a: Vec<String> = p["a"].as_array().unwrap().iter().map(|v| v.as_str().unwrap().to_string()).collect();
                    let actual = match cs::catch_panic(|| call(game, b, m, &a)) {
                        Ok(r) => r,
                        Err(e) => format!("EX:{}: {}", cs::full_type_name(e.type_name), e.message),
                    };
                    let draws = b.random.get_count() as i64;
                    let (expected, expected_draws) = (p["r"].as_str().unwrap(), p["d"].as_i64().unwrap());
                    count += 1;
                    if actual != expected || draws != expected_draws {
                        failures.push(format!(
                            "step {n} player {pi} {m}({}): C# {expected} ({expected_draws} draws), Rust {actual} ({draws} draws)",
                            a.join(", ")
                        ));
                        skip = Some(pi);
                    }
                }
            }
            _ => {}
        }
    }

    assert!(count > 0, "{name}: no probes");
    if !failures.is_empty() {
        panic!("{name}: {} of {count} probes differ:\n{}", failures.len(), failures.iter().take(40).cloned().collect::<Vec<_>>().join("\n"));
    }
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
