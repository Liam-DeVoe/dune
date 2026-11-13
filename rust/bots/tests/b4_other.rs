//! The other decisions (`bot/other.rs`) and their helpers, with the bot's draw count after each
//! call. The Nexus switches are called for every Nexus faction (with the player's `nexus` set to
//! it for the call), and `determine_clair_voyance_answered` for a list of questions (with
//! `latest_clairvoyance` set for the call), so branches the recorded games rarely reach are
//! checked too.
//!
//! The games are in `golden/knowledge-*.jsonl.gz`; `golden/b4-*.jsonl.gz` holds the probes and
//! their expected results. Every call gets a fresh bot, so one call can't affect the next.
//!
//! The expected results are snapshots of the current behavior. After a deliberate change, rewrite
//! them with `UPDATE_GOLDENS=1 cargo test --release -p treachery-bots` and review the diff
//! (see `common/mod.rs`).

mod common;

use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};

use treachery_bots::bot::EventKind;
use treachery_bots::prelude::*;

use common::{run_probes, Bots};

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
        _ => panic!("unknown probe method {m}"),
    }
}

/// What a probe sets on the game before the call, undone afterwards.
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

fn run(name: &str) {
    run_probes("b4", name, Bots::Fresh, |game, bot, m, a| {
        let me = bot.player;
        let saved = setup(game, me, m, a);
        let g: &Game = game;
        let outcome = catch_unwind(AssertUnwindSafe(|| call(g, bot, m, a)));
        undo(game, me, saved);
        outcome.unwrap_or_else(|p| resume_unwind(p))
    });
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
