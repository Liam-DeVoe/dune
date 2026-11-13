//! Package B3 (Bot_FactionSpecific) against C#: every `Determine*` method of the file, plus
//! `PassAmbassadorActivated`, `IWantToBeFightersIn`, `IWantToAnnounceBattleIn`, `BestVoice` and
//! `BestPrescience`, with the bot's draw count after each call.
//!
//! The events come from the knowledge goldens (`golden/knowledge-*.jsonl.gz`); `golden/b3-*.jsonl.gz`
//! holds only the probes, from `bots/csharp-ref` (see B3.cs):
//!
//! ```sh
//! export DOTNET_ROOT="$HOME/dotnet10" PATH="$HOME/dotnet10:$PATH"
//! cd rust/bots/csharp-ref
//! dotnet run -c Release -- b3 ../tests/golden/knowledge-00008.jsonl.gz ../tests/golden/b3-00008.jsonl.gz 4
//! dotnet run -c Release -- b3 ../tests/golden/knowledge-77-00001.jsonl.gz ../tests/golden/b3-77-00001.jsonl.gz 6
//! dotnet run -c Release -- b3 ../tests/golden/knowledge-77-00002.jsonl.gz ../tests/golden/b3-77-00002.jsonl.gz 10
//! dotnet run -c Release -- b3 ../tests/golden/knowledge-77-00004.jsonl.gz ../tests/golden/b3-77-00004.jsonl.gz 10
//! ```
//!
//! The probes run whatever the phase, so they also reach states the differ in bots mode never
//! does: decisions outside their phase (and the exceptions they throw there), ResourcesAudited and
//! ChoamRemoveForce (rare in bot games), BestVoice and BestPrescience against every player.

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

fn b(v: bool) -> String {
    if v { "true".into() } else { "false".into() }
}

/// C# `bool.ToString()`
fn cs_bool(v: bool) -> &'static str {
    if v { "True" } else { "False" }
}

fn voice_plan(p: &VoicePlan) -> String {
    format!(
        "battle:{} voice:{} weapon:{} defense:{} survive:{} zero:{}",
        if p.battle.is_some() { "set" } else { "null" },
        ev(p.voice.clone()),
        p.weapon_to_use.map_or(-1, |c| c.id()),
        p.defense_to_use.map_or(-1, |c| c.id()),
        cs_bool(p.player_hero_will_certainly_survive),
        cs_bool(p.opponent_hero_will_certainly_be_zero)
    )
}

fn call(game: &Game, bot: &mut ClassicBot, m: &str, a: &[String]) -> String {
    let n = |i: usize| a[i][1..].parse::<i32>().unwrap();
    let player = |i: usize| game.player_at(n(i) as usize);
    let territory = |i: usize| game.map.territory_lookup().find(n(i)).unwrap();
    match m {
        "DetermineSetShipmentPermission" => opt(bot.determine_set_shipment_permission(game)),
        "DetermineEmperorDiscarded" => opt(bot.determine_emperor_discarded(game)),
        "DetermineReplacedCardWon" => ev(bot.determine_replaced_card_won(game)),
        "DetermineIxianRemovedCardFromAuction" => ev(bot.determine_ixian_removed_card_from_auction(game)),
        "DetermineIxianSwappedCardOnBid" => ev(bot.determine_ixian_swapped_card_on_bid(game)),
        "DetermineIxianSelectedStartingCard" => ev(bot.determine_ixian_selected_starting_card(game)),
        "DeterminePerformHmsMovement" => ev(bot.determine_perform_hms_movement(game)),
        "DeterminePerformHmsPlacement" => ev(bot.determine_perform_hms_placement(game)),
        "DetermineTakeLosses" => ev(bot.determine_take_losses(game)),
        "DeterminePerformFremenSetup" => ev(bot.determine_perform_fremen_setup(game)),
        "DetermineFremenSentMonster" => ev(bot.determine_fremen_sent_monster(game)),
        "DetermineFremenRidesMonster" => ev(bot.determine_fremen_rides_monster(game)),
        "DetermineBeneGesseritPlacement" => ev(bot.determine_bene_gesserit_placement(game)),
        "DetermineBeneGesseritFlip" => ev(bot.determine_bene_gesserit_flip(game)),
        "DetermineBeneGesseritBattleAnnouncement" => opt(bot.determine_bene_gesserit_battle_announcement(game)),
        "DetermineBeneGesseritAccompanies" => ev(bot.determine_bene_gesserit_accompanies(game)),
        "DetermineBeneGesseritPrediction" => ev(bot.determine_bene_gesserit_prediction(game)),
        "DetermineVoice" => opt(bot.determine_voice(game)),
        "DeterminePrescience" => opt(bot.determine_prescience(game)),
        "DetermineSetIncreasedRevivalLimits" => opt(bot.determine_set_increased_revival_limits(game)),
        "DetermineFaceDancerRevealed" => ev(bot.determine_face_dancer_revealed(game)),
        "DetermineFaceDanced" => ev(bot.determine_face_danced(game)),
        "DetermineFaceDancerReplaced" => ev(bot.determine_face_dancer_replaced(game)),
        "DetermineRequestTleilaxuRevival" => opt(bot.determine_request_tleilaxu_revival(game)),
        "DetermineAcceptOrCancelTleilaxuRevival" => opt(bot.determine_accept_or_cancel_tleilaxu_revival(game)),
        "DetermineResourcesAudited" => opt(bot.determine_resources_audited(game)),
        "DetermineChoamEconomics" => opt(bot.determine_choam_economics(game)),
        "DetermineChoamRemoveForce" => opt(bot.determine_choam_remove_force(game)),
        "DetermineExtortionPrevented" => opt(bot.determine_extortion_prevented(game)),
        "DeterminePerformMoritaniSetup" => ev(bot.determine_perform_moritani_setup(game)),
        "DetermineTerrorPlanted" => opt(bot.determine_terror_planted(game)),
        "DetermineTerrorRevealed" => ev(bot.determine_terror_revealed(game)),
        "DetermineAmbassadorPlaced" => opt(bot.determine_ambassador_placed(game)),
        "DetermineAmbassadorActivated" => opt(bot.determine_ambassador_activated(game)),
        "PassAmbassadorActivated" => opt(bot.pass_ambassador_activated(game)),
        "DetermineRicheseAnnouncesBlackMarket" => ev(bot.determine_richese_announces_black_market(game)),
        "DetermineRicheseRevealedNoField" => opt(bot.determine_richese_revealed_no_field(game)),
        "DetermineRicheseAnnouncesAuction" => ev(bot.determine_richese_announces_auction(game)),
        "DetermineRicheseSpecifiesAuction" => ev(bot.determine_richese_specifies_auction(game)),
        "DetermineRicheseKeepsUnsoldCard" => ev(bot.determine_richese_keeps_unsold_card(game)),
        "DetermineCaptured" => ev(bot.determine_captured(game)),
        "IWantToBeFightersIn" => b(bot.i_want_to_be_fighters_in(game, territory(0))),
        "IWantToAnnounceBattleIn" => b(bot.i_want_to_announce_battle_in(game, territory(0))),
        "BestVoice" => voice_plan(&bot.best_voice(game, None, player(1), player(2))),
        "BestPrescience" => {
            let strength = a[2][1..].parse::<f32>().unwrap();
            let aspect = PrescienceAspect(n(3));
            bot.best_prescience(game, player(0), player(1), strength, aspect, territory(4)).0.to_string()
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
    for o in lines(&format!("b3-{name}.jsonl.gz")) {
        probes.entry(o["n"].as_i64().unwrap()).or_default().push(o);
    }

    let mut game: Option<Game> = None;
    let mut count = 0;
    let mut by_method: BTreeMap<String, (usize, usize)> = BTreeMap::new();
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
                e.execute(game, validate, is_host);

                let n = o["n"].as_i64().unwrap();
                let Some(list) = probes.get(&n) else { continue };
                let game: &Game = game;
                let mut bot: Option<(i64, ClassicBot)> = None;
                // After a difference, the later probes of that bot depend on its state and draws: skip them.
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
                    // A probe that throws in C# must panic in Rust. The exception type and message
                    // aren't compared.
                    let actual = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| call(game, b, m, &a))) {
                        Ok(r) => r,
                        Err(_) => "EX".to_string(),
                    };
                    let draws = b.random.get_count() as i64;
                    let (expected, expected_draws) = (p["r"].as_str().unwrap(), p["d"].as_i64().unwrap());
                    let expected = if expected.starts_with("EX:") { "EX" } else { expected };
                    count += 1;
                    let entry = by_method.entry(m.to_string()).or_default();
                    entry.0 += 1;
                    if expected != "EX" && expected != "null" {
                        entry.1 += 1;
                    }
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
    eprintln!("{name}: {count} probes (method: calls, non-null results)");
    for (m, (c, nn)) in &by_method {
        eprintln!("  {m}: {c}, {nn}");
    }
    if !failures.is_empty() {
        panic!("{name}: {} of {count} probes differ:\n{}", failures.len(), failures.iter().take(40).cloned().collect::<Vec<_>>().join("\n"));
    }
}

#[test]
fn b3_00008() {
    run("00008");
}

#[test]
fn b3_77_00001() {
    run("77-00001");
}

#[test]
fn b3_77_00002() {
    run("77-00002");
}

#[test]
fn b3_77_00004() {
    run("77-00004");
}
