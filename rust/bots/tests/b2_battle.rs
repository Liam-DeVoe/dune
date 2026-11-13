//! Package B2 (Bot_Battle) against C#: the battle decisions at the states where bots make them,
//! and `GetDialNeeded`, `DetermineDialShortageForBattle` and a few static helpers for every
//! player, with the bot's draw count after each call. Every probe gets a fresh bot.
//!
//! The events come from the knowledge goldens (`golden/knowledge-*.jsonl.gz`); `golden/b2-*.jsonl.gz`
//! holds only the probes, from `bots/csharp-ref` (see B2.cs):
//!
//! ```sh
//! export DOTNET_ROOT="$HOME/dotnet10" PATH="$HOME/dotnet10:$PATH"
//! cd rust/bots/csharp-ref
//! for g in 00008 77-00001 77-00002 77-00004; do
//!   dotnet run -c Release -- b2 ../tests/golden/knowledge-$g.jsonl.gz ../tests/golden/b2-$g.jsonl.gz 20
//! done
//! ```
//!
//! A probe that reaches an unported method of another package (a `todo!` in Bot_FactionSpecific,
//! for example `BestVoice`) is counted as blocked, not as a failure. The test prints the counts.

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader};
use std::panic::{catch_unwind, AssertUnwindSafe};

use flate2::read::GzDecoder;
use serde_json::Value;
use treachery_bots::bot::EventKind;
use treachery_bots::prelude::*;
use treachery_engine::game::Participation;

fn ev<T: EventKind>(e: Rc<T>) -> String {
    treachery_engine::json::serialize(&T::into_event(e))
}

fn opt<T: EventKind>(e: Option<Rc<T>>) -> String {
    e.map_or_else(|| "null".to_string(), ev)
}

fn f32s(x: f32) -> String {
    dotnet_compat::fmt::f32_to_string(x)
}

fn hero(h: &Hero) -> String {
    format!("H{}", h.id())
}

fn call(game: &mut Game, b: &ClassicBot, m: &str, a: &[String]) -> String {
    let n = |i: usize| a[i][1..].parse::<i32>().unwrap();
    let p = |game: &Game, i: usize| game.player_at(n(i) as usize);
    match m {
        "DetermineBattleClaimed" => ev(b.determine_battle_claimed(game)),
        "DetermineSwitchedSkilledLeader" => opt(b.determine_switched_skilled_leader(game)),
        "DetermineBattleInitiated" => ev(b.determine_battle_initiated(game)),
        "DetermineTreacheryCalled" => opt(b.determine_treachery_called(game)),
        "DetermineAuditCancelled" => ev(b.determine_audit_cancelled(game)),
        "DetermineAudited" => ev(b.determine_audited(game)),
        "DetermineBattleConcluded" => ev(b.determine_battle_concluded(game)),
        "DetermineBattle" => opt(b.determine_battle(game)),
        "DetermineBattlePlan" => ev(b.determine_battle_plan(game, a[0] == "true")),
        "DetermineRockWasMelted" => ev(b.determine_rock_was_melted(game)),
        "DetermineResidualPlayed" => ev(b.determine_residual_played(game)),
        "DeterminePortableAntidoteUsed" => opt(b.determine_portable_antidote_used(game)),
        "DetermineThought" => opt(b.determine_thought(game)),
        "DetermineThoughtAnswered" => ev(b.determine_thought_answered(game)),
        "DetermineHmsAdvantageChosen" => ev(b.determine_hms_advantage_chosen(game)),
        "DetermineRetreat" => opt(b.determine_retreat(game)),
        "DetermineLoserConcluded" => ev(b.determine_loser_concluded(game)),
        "CanOnlyUseTraitorsOrFacedancers" => {
            if ClassicBot::can_only_use_traitors_or_facedancers(game, p(game, 0)) { "true".into() } else { "false".into() }
        }
        "HeroesForBattle" => {
            let hs = ClassicBot::heroes_for_battle(game, p(game, 1), a[2] == "true");
            format!("[{}]", hs.iter().map(hero).collect::<Vec<_>>().join(","))
        }
        "NrOfUnknownOpponentCards" => ClassicBot::nr_of_unknown_opponent_cards(game, p(game, 1), p(game, 2)).to_string(),
        "GetDialNeeded" => {
            let t = game.map.territory_lookup().find(n(0)).unwrap();
            let o = if a[1] == "N" { None } else { Some(p(game, 1)) };
            f32s(b.dial_needed_against(game, t, o, a[2] == "true"))
        }
        "DetermineDialShortageForBattle" => {
            let dial: f32 = a[0].parse().unwrap();
            let t = game.map.territory_lookup().find(n(2)).unwrap();
            f32s(b.determine_dial_shortage_for_battle(game, dial, Faction(n(1)), t, n(3), n(4), n(5)))
        }
        _ => panic!("no Rust call for {m}"),
    }
}

/// As differ's `replay::participation` (and the oracle's Verifier).
fn participation(o: &Value) -> Participation {
    let int = |v: &Value| v.as_i64().unwrap() as i32;
    let mut p = Participation { bots_speed: int(&o["BotsSpeed"]), ..Default::default() };
    for kv in o["PlayerNames"].as_array().unwrap() {
        p.player_names.add(int(&kv[0]), kv[1].as_str().unwrap().to_string());
    }
    for kv in o["SeatedPlayers"].as_array().unwrap() {
        p.seated_players.add(int(&kv[0]), int(&kv[1]));
    }
    for (key, set) in [
        ("Observers", &mut p.observers),
        ("Hosts", &mut p.hosts),
        ("AvailableSeats", &mut p.available_seats),
        ("Kicked", &mut p.kicked),
    ] {
        for v in o[key].as_array().unwrap() {
            set.add(int(v));
        }
    }
    p
}

fn lines(name: &str) -> impl Iterator<Item = Value> {
    let path = format!("{}/tests/golden/{name}", env!("CARGO_MANIFEST_DIR"));
    let file = std::fs::File::open(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
    BufReader::new(GzDecoder::new(file)).lines().map(|l| serde_json::from_str(&l.unwrap()).unwrap())
}

fn panic_text(e: &Box<dyn std::any::Any + Send>) -> String {
    e.downcast_ref::<String>().cloned().or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_default()
}

fn run(name: &str) {
    let mut probes: BTreeMap<i64, Vec<Value>> = BTreeMap::new();
    for o in lines(&format!("b2-{name}.jsonl.gz")) {
        probes.entry(o["n"].as_i64().unwrap()).or_default().push(o);
    }

    let mut game: Option<Game> = None;
    let mut count = 0;
    let mut blocked: BTreeMap<String, usize> = BTreeMap::new();
    let mut per_method: BTreeMap<String, usize> = BTreeMap::new();
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
                for p in list {
                    let pi = p["p"].as_i64().unwrap();
                    game.map.mark_neighbour_cache_stale();
                    let player = game.player_at(pi as usize);
                    let faction = game.player(player).faction;
                    let seed = p["seed"].as_i64().unwrap() as i32;
                    let b = ClassicBot::new(player, BotParameters::for_faction(faction), SharedRandom::from_seed(seed));
                    let m = p["m"].as_str().unwrap();
                    let a: Vec<String> = p["a"].as_array().unwrap().iter().map(|v| v.as_str().unwrap().to_string()).collect();
                    let result = catch_unwind(AssertUnwindSafe(|| cs::catch_panic(|| call(game, &b, m, &a))));
                    let actual = match result {
                        Ok(Ok(r)) => r,
                        Ok(Err(e)) => format!("EX:{}: {}", cs::full_type_name(e.type_name), e.message),
                        Err(panic) => {
                            let text = panic_text(&panic);
                            if text.contains("not yet implemented") && !text.contains("Bot_Battle.cs") {
                                *blocked.entry(text).or_default() += 1;
                                continue;
                            }
                            format!("PANIC: {text}")
                        }
                    };
                    let draws = b.random.get_count() as i64;
                    let (expected, expected_draws) = (p["r"].as_str().unwrap(), p["d"].as_i64().unwrap());
                    count += 1;
                    *per_method.entry(m.to_string()).or_default() += 1;
                    let mut ok = actual == expected && draws == expected_draws;
                    if let Some(messages) = p.get("messages") {
                        let rust_messages = game.current_report().messages.len() as i64;
                        if rust_messages != messages.as_i64().unwrap() {
                            ok = false;
                            failures.push(format!("step {n} player {pi} {m}: C# report has {messages} messages, Rust {rust_messages}"));
                        }
                    }
                    if !ok {
                        failures.push(format!(
                            "step {n} player {pi} {m}({}): C# {expected} ({expected_draws} draws), Rust {actual} ({draws} draws)",
                            a.join(", ")
                        ));
                    }
                }
            }
            _ => {}
        }
    }

    eprintln!("{name}: {count} probes checked: {per_method:?}");
    for (text, k) in &blocked {
        eprintln!("{name}: {k} probes blocked by {text}");
    }
    assert!(count > 0, "{name}: no probes");
    if !failures.is_empty() {
        panic!("{name}: {} of {count} probes differ:\n{}", failures.len(), failures.iter().take(40).cloned().collect::<Vec<_>>().join("\n"));
    }
}

#[test]
fn b2_00008() {
    run("00008");
}

#[test]
fn b2_77_00001() {
    run("77-00001");
}

#[test]
fn b2_77_00002() {
    run("77-00002");
}

#[test]
fn b2_77_00004() {
    run("77-00004");
}
