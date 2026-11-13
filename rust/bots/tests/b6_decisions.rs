//! Package B6 (Bot_Bidding, Bot_Deals, Bot_Alliance) against C#: the decision of every B6
//! `Determine*` method, plus `PlayerStanding`, `WannaHave` and `DetermineAllowedResources`,
//! with the bot's draw count after each call.
//!
//! The events come from the knowledge goldens (`golden/knowledge-*.jsonl.gz`); `golden/b6-*.jsonl.gz`
//! holds only the probes, from `bots/csharp-ref` (see B6.cs):
//!
//! ```sh
//! export DOTNET_ROOT="$HOME/dotnet10" PATH="$HOME/dotnet10:$PATH"
//! cd rust/bots/csharp-ref
//! dotnet run -c Release -- b6 ../tests/golden/knowledge-00008.jsonl.gz ../tests/golden/b6-00008.jsonl.gz 1
//! dotnet run -c Release -- b6 ../tests/golden/knowledge-77-00001.jsonl.gz ../tests/golden/b6-77-00001.jsonl.gz 1
//! dotnet run -c Release -- b6 ../tests/golden/knowledge-77-00002.jsonl.gz ../tests/golden/b6-77-00002.jsonl.gz 2
//! dotnet run -c Release -- b6 ../tests/golden/knowledge-77-00004.jsonl.gz ../tests/golden/b6-77-00004.jsonl.gz 2
//! ```
//!
//! The probes run after each step (or every second step), whatever the phase, so they also cover
//! the exceptions these methods throw outside their phase (DetermineBid without a card on
//! auction). The differ in bots mode covers them at the points where bots really decide.

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader};

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

fn call(game: &Game, b: &ClassicBot, m: &str, a: &[String]) -> String {
    let n = |i: usize| a[i][1..].parse::<i32>().unwrap();
    match m {
        "DetermineBid" => ev(b.determine_bid(game)),
        "DetermineBlackMarketBid" => ev(b.determine_black_market_bid(game)),
        "DetermineDealAccepted" => opt(b.determine_deal_accepted(game)),
        "DetermineDealCancelled" => opt(b.determine_deal_cancelled(game)),
        "DetermineDealOffered" => opt(b.determine_deal_offered(game)),
        "DetermineAllianceByTerror" => ev(b.determine_alliance_by_terror(game)),
        "DetermineAllianceByAmbassador" => ev(b.determine_alliance_by_ambassador(game)),
        "DetermineNexusVoted" => ev(b.determine_nexus_voted(game)),
        "DetermineAllianceOffered" => opt(b.determine_alliance_offered(game)),
        "DetermineAlliancePassed" => opt(b.determine_alliance_passed(game)),
        "DetermineAllianceBroken" => opt(b.determine_alliance_broken(game)),
        "DetermineNexusCardDrawn" => ev(b.determine_nexus_card_drawn(game)),
        "DetermineAlliancePermissions" => opt(b.determine_alliance_permissions(game)),
        "DetermineCardTraded" => opt(b.determine_card_traded(game)),
        "DetermineAllowedResources" => b.determine_allowed_resources(game).to_string(),
        "PlayerStanding" => b.player_standing(game, game.player_at(n(0) as usize)).to_string(),
        "WannaHave" => {
            let c = TreacheryCardManager::lookup().find(n(0)).unwrap();
            if b.wanna_have(game, c) { "true".into() } else { "false".into() }
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

fn run(name: &str) {
    let mut probes: BTreeMap<i64, Vec<Value>> = BTreeMap::new();
    for o in lines(&format!("b6-{name}.jsonl.gz")) {
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
                let _ = cs::catch(|| e.execute(game, validate, is_host));

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
                    let b = &bot.as_ref().unwrap().1;
                    let m = p["m"].as_str().unwrap();
                    let a: Vec<String> = p["a"].as_array().unwrap().iter().map(|v| v.as_str().unwrap().to_string()).collect();
                    let actual = match cs::catch(|| call(game, b, m, &a)) {
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
fn b6_00008() {
    run("00008");
}

#[test]
fn b6_77_00001() {
    run("77-00001");
}

#[test]
fn b6_77_00002() {
    run("77-00002");
}

#[test]
fn b6_77_00004() {
    run("77-00004");
}
