//! Package B4 (Bot_Other) against C#: every `Determine*` method of Bot_Other.cs and its helpers,
//! with the bot's draw count after each call. The Nexus switches are called for every Nexus
//! faction (with `Player.Nexus` set to it for the call), and `DetermineClairVoyanceAnswered` for
//! a list of questions (with `Game.LatestClairvoyance` set for the call), so the branches that the
//! recorded games rarely reach are compared too.
//!
//! The events come from the knowledge goldens (`golden/knowledge-*.jsonl.gz`); `golden/b4-*.jsonl.gz`
//! holds only the probes, from `bots/csharp-ref` (see B4.cs):
//!
//! ```sh
//! export DOTNET_ROOT="$HOME/dotnet10" PATH="$HOME/dotnet10:$PATH"
//! cd rust/bots/csharp-ref
//! for g in 00008 77-00001 77-00002 77-00004; do
//!   dotnet run -c Release -- b4 ../tests/golden/knowledge-$g.jsonl.gz ../tests/golden/b4-$g.jsonl.gz 8
//! done
//! ```
//!
//! Every call gets a fresh bot with the seed C# used, so a difference in one call doesn't affect
//! the next. A call that reaches an unported method of another package (a `todo!`) is counted
//! as skipped, not compared; once that package is ported, the call is compared.

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

fn hero(h: Option<Hero>) -> String {
    h.map_or_else(|| "null".to_string(), |h| format!("H{}", h.id()))
}

fn b(v: bool) -> String {
    if v { "true".into() } else { "false".into() }
}

fn call(game: &Game, bot: &mut ClassicBot, m: &str, a: &[String]) -> String {
    let n = |i: usize| a[i][1..].parse::<i32>().unwrap();
    let me = bot.player;
    match m {
        "DetermineResourcesTransferred" => opt(bot.determine_resources_transferred(game)),
        "DetermineDiscoveryEntered" => ev(bot.determine_discovery_entered(game)),
        "DetermineDiscoveryRevealed" => ev(bot.determine_discovery_revealed(game)),
        "DetermineTestingStationUsed" => opt(bot.determine_testing_station_used(game)),
        "DetermineDivideResources" => ev(bot.determine_divide_resources(game)),
        "DetermineCardGiven" => ev(bot.determine_card_given(game)),
        "DetermineDivideResourcesAccepted" => ev(bot.determine_divide_resources_accepted(game)),
        "DetermineDiscarded" => ev(bot.determine_discarded(game)),
        "DetermineTraitorDiscarded" => ev(bot.determine_traitor_discarded(game)),
        "DetermineWorstTraitor" => hero(bot.determine_worst_traitor(game)),
        "DetermineNexusPlayed" => opt(bot.determine_nexus_played(game)),
        "DetermineTerritoryToRemoveEcazFrom" => {
            bot.determine_territory_to_remove_ecaz_from(game).map_or_else(|| "null".to_string(), |t| format!("T{}", t.id()))
        }
        "DetermineCharityClaimed" => opt(bot.determine_charity_claimed(game)),
        "DetermineKarmaFreeRevival" => opt(bot.determine_karma_free_revival(game)),
        "DetermineKarmaHandSwap" => ev(bot.determine_karma_hand_swap(game)),
        "DetermineKarmaShipmentPrevention" => opt(bot.determine_karma_shipment_prevention(game)),
        "DetermineKarmaHandSwapInitiated" => opt(bot.determine_karma_hand_swap_initiated(game)),
        "DetermineHarvesterPlayed" => opt(bot.determine_harvester_played(game)),
        "DetermineMulliganPerformed" => ev(bot.determine_mulligan_performed(game)),
        "DetermineDelay" => opt(bot.determine_delay(game)),
        "DetermineClairvoyance" => opt(bot.determine_clairvoyance(game)),
        "DetermineClairVoyanceAnswered" | "ClairVoyanceAnswered" => ev(bot.determine_clair_voyance_answered(game)),
        "DetermineRaiseDeadPlayed" => opt(bot.determine_raise_dead_played(game)),
        "DetermineAmalPlayed" => opt(bot.determine_amal_played(game)),
        "DetermineRecruitsPlayed" => opt(bot.determine_recruits_played(game)),
        "DetermineMetheorPlayed" => opt(bot.determine_metheor_played(game)),
        "DetermineStormDialled" => ev(bot.determine_storm_dialled(game)),
        "DetermineTraitorsSelected" => ev(bot.determine_traitors_selected(game)),
        "DetermineFactionTradeOffered" => opt(bot.determine_faction_trade_offered(game)),
        "DetermineThumperPlayed" => ev(bot.determine_thumper_played(game)),
        "DetermineStormSpellPlayed" => opt(bot.determine_storm_spell_played(game)),
        "DetermineDistransUsed" => opt(bot.determine_distrans_used(game)),
        "DetermineRevival" => opt(bot.determine_revival(game)),
        "DetermineDiscardedSearchedAnnounced" => opt(bot.determine_discarded_searched_announced(game)),
        "DetermineDiscardedSearched" => ev(bot.determine_discarded_searched(game)),
        "DetermineDiscardedTaken" => opt(bot.determine_discarded_taken(game)),
        "DetermineJuicePlayed" => opt(bot.determine_juice_played(game)),
        "DetermineBureaucracy" => ev(bot.determine_bureaucracy(game)),
        "DetermineDiplomacy" => opt(bot.determine_diplomacy(game)),
        "DetermineSkillAssigned" => ev(bot.determine_skill_assigned(game)),
        "DeterminePlanetology" => ev(bot.determine_planetology(game)),
        "IsWinningOrIsOpponentInBattle" => b(bot.is_winning_or_is_opponent_in_battle(game, Faction(n(0)))),
        "OpponentMayNotUse" => b(bot.opponent_may_not_use(game, TreacheryCardType(n(0)), a[1] == "true")),
        "HeroRevivalPenalty" => bot.hero_revival_penalty(game, LeaderManager::hero_lookup().find(n(0)).unwrap()).to_string(),
        "BestRevivalLocation" => {
            let all: Vec<Location> = game.map.locations(false).collect();
            bot.best_revival_location(game, &all).map_or_else(|| "null".to_string(), |l| format!("L{}", l.id()))
        }
        "DetermineOptimalUseOfEmperorRevivals" => {
            let (f, s) = ClassicBot::determine_optimal_use_of_emperor_revivals(game, me, a[0] == "true");
            format!("{f},{s}")
        }
        "DetermineNexusPlayed_Betrayal" | "DetermineNexusPlayed_Cunning" | "DetermineNexusPlayed_SecretAlly" => {
            let mut result = NexusPlayed::new(game, game.player(me).faction);
            result.faction = Faction(n(0));
            opt(match m {
                "DetermineNexusPlayed_Betrayal" => bot.determine_nexus_played_betrayal(game, result),
                "DetermineNexusPlayed_Cunning" => bot.determine_nexus_played_cunning(game, result),
                _ => bot.determine_nexus_played_secret_ally(game, result),
            })
        }
        _ => panic!("no Rust call for {m}"),
    }
}

/// What a probe sets on the game before the call (as B4.cs does), undone afterwards.
enum Setup {
    None,
    Nexus(Faction),
    Clairvoyance(Option<Rc<ClairVoyancePlayed>>),
}

fn setup(game: &mut Game, me: PlayerIndex, m: &str, a: &[String]) -> Setup {
    match m {
        "DetermineNexusPlayed_Betrayal" | "DetermineNexusPlayed_Cunning" | "DetermineNexusPlayed_SecretAlly" => {
            let saved = game.player(me).nexus;
            game.player_mut(me).nexus = Faction(a[0][1..].parse().unwrap());
            Setup::Nexus(saved)
        }
        "ClairVoyanceAnswered" => {
            let saved = game.latest_clairvoyance.clone();
            let initiator = Faction(a[0][1..].parse().unwrap());
            let mut cv = ClairVoyancePlayed::new(game, initiator);
            cv.target = game.player(me).faction;
            cv.question = ClairvoyanceQuestion(dotnet_compat::parse::enum_parse(Some(&a[1]), ClairvoyanceQuestion::INFO.declared));
            cv.question_parameter1 = if a[4] == "null1" { None } else { Some(a[2].clone()) };
            cv.question_parameter2 = if a[5] == "null2" { None } else { Some(a[3].clone()) };
            game.latest_clairvoyance = Some(Rc::new(cv));
            Setup::Clairvoyance(saved)
        }
        _ => Setup::None,
    }
}

fn undo(game: &mut Game, me: PlayerIndex, s: Setup) {
    match s {
        Setup::None => {}
        Setup::Nexus(f) => game.player_mut(me).nexus = f,
        Setup::Clairvoyance(cv) => game.latest_clairvoyance = cv,
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

fn panic_text(p: &(dyn std::any::Any + Send)) -> String {
    p.downcast_ref::<String>().cloned().or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_default()
}

/// A `todo!` of another package panics on purpose; don't print those.
fn quiet_todo_panics() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            if panic_text(info.payload()).contains("not yet implemented") {
                return;
            }
            previous(info)
        }));
    });
}

fn run(name: &str) {
    let mut probes: BTreeMap<i64, Vec<Value>> = BTreeMap::new();
    for o in lines(&format!("b4-{name}.jsonl.gz")) {
        probes.entry(o["n"].as_i64().unwrap()).or_default().push(o);
    }

    quiet_todo_panics();

    let mut game: Option<Game> = None;
    let mut count = 0;
    let mut skipped: BTreeMap<String, usize> = BTreeMap::new();
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
                for p in list {
                    let pi = p["p"].as_i64().unwrap();
                    let player = game.player_at(pi as usize);
                    let m = p["m"].as_str().unwrap();
                    let a: Vec<String> = p["a"].as_array().unwrap().iter().map(|v| v.as_str().unwrap().to_string()).collect();

                    game.map.mark_neighbour_cache_stale();
                    let faction = game.player(player).faction;
                    let seed = p["seed"].as_i64().unwrap() as i32;
                    let mut bot = ClassicBot::new(player, BotParameters::for_faction(faction), SharedRandom::from_seed(seed));

                    let saved = setup(game, player, m, &a);
                    let g: &Game = game;
                    let outcome = catch_unwind(AssertUnwindSafe(|| call(g, &mut bot, m, &a)));
                    undo(game, player, saved);

                    // A probe that throws in C# must panic in Rust. The exception type and message
                    // aren't compared.
                    let actual = match outcome {
                        Ok(r) => r,
                        Err(payload) => {
                            let text = panic_text(&*payload);
                            if text.contains("not yet implemented") {
                                *skipped.entry(text).or_default() += 1;
                                continue;
                            }
                            "EX".to_string()
                        }
                    };
                    let draws = bot.random.get_count() as i64;
                    let (expected, expected_draws) = (p["r"].as_str().unwrap(), p["d"].as_i64().unwrap());
                    let expected = if expected.starts_with("EX:") { "EX" } else { expected };
                    count += 1;
                    if actual != expected || draws != expected_draws {
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

    for (text, k) in &skipped {
        eprintln!("{name}: {k} calls skipped, they reach an unported method: {text}");
    }
    assert!(count > 0, "{name}: no probes");
    if !failures.is_empty() {
        panic!("{name}: {} of {count} probes differ:\n{}", failures.len(), failures.iter().take(40).cloned().collect::<Vec<_>>().join("\n"));
    }
}

#[test]
fn b4_00008() {
    run("00008");
}

#[test]
fn b4_77_00001() {
    run("77-00001");
}

#[test]
fn b4_77_00002() {
    run("77-00002");
}

#[test]
fn b4_77_00004() {
    run("77-00004");
}
