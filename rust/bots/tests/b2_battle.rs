//! The battle decisions (`bot/battle.rs`) at the states of four recorded games, and
//! `dial_needed_against`, `determine_dial_shortage_for_battle` and a few static helpers for every
//! player, with the bot's draw count after each call. Every probe gets a fresh bot.
//!
//! The games are in `golden/knowledge-*.jsonl.gz`; `golden/b2-*.jsonl.gz` holds the probes and
//! their expected results.
//!
//! The expected results are snapshots of the current behavior. After a deliberate change, rewrite
//! them with `UPDATE_GOLDENS=1 cargo test --release -p treachery-bots` and review the diff
//! (see `common/mod.rs`).

mod common;

use treachery_bots::prelude::*;

use common::{run_probes, Bots};

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
        "DetermineBattleClaimed" => common::decision(b, game, b.determine_battle_claimed(game)),
        "DetermineSwitchedSkilledLeader" => common::decision(b, game, b.determine_switched_skilled_leader(game)),
        "DetermineBattleInitiated" => common::decision(b, game, b.determine_battle_initiated(game)),
        "DetermineTreacheryCalled" => common::decision(b, game, b.determine_treachery_called(game)),
        "DetermineAuditCancelled" => common::decision(b, game, b.determine_audit_cancelled(game)),
        "DetermineAudited" => common::decision(b, game, b.determine_audited(game)),
        "DetermineBattleConcluded" => common::decision(b, game, b.determine_battle_concluded(game)),
        "DetermineBattle" => common::decision(b, game, b.determine_battle(game)),
        "DetermineBattlePlan" => common::decision(b, game, b.determine_battle_plan(game, a[0] == "true")),
        "DetermineRockWasMelted" => common::decision(b, game, b.determine_rock_was_melted(game)),
        "DetermineResidualPlayed" => common::decision(b, game, b.determine_residual_played(game)),
        "DeterminePortableAntidoteUsed" => common::decision(b, game, b.determine_portable_antidote_used(game)),
        "DetermineThought" => common::decision(b, game, b.determine_thought(game)),
        "DetermineThoughtAnswered" => common::decision(b, game, b.determine_thought_answered(game)),
        "DetermineHmsAdvantageChosen" => common::decision(b, game, b.determine_hms_advantage_chosen(game)),
        "DetermineRetreat" => common::decision(b, game, b.determine_retreat(game)),
        "DetermineLoserConcluded" => common::decision(b, game, b.determine_loser_concluded(game)),
        "CanOnlyUseTraitorsOrFacedancers" => {
            if ClassicBot::can_only_use_traitors_or_facedancers(game, p(game, 0)) { "true".into() } else { "false".into() }
        }
        "HeroesForBattle" => {
            let hs = ClassicBot::heroes_for_battle(game, p(game, 1), a[2] == "true");
            format!("[{}]", hs.iter().map(hero).collect::<Vec<_>>().join(","))
        }
        "NrOfUnknownOpponentCards" => ClassicBot::nr_of_unknown_opponent_cards(game, p(game, 1), p(game, 2)).to_string(),
        "GetDialNeeded" => {
            let t = game.board.map.territory_lookup().find(n(0)).unwrap();
            let o = if a[1] == "N" { None } else { Some(p(game, 1)) };
            f32s(b.dial_needed_against(game, t, o, a[2] == "true"))
        }
        "DetermineDialShortageForBattle" => {
            let dial: f32 = a[0].parse().unwrap();
            let t = game.board.map.territory_lookup().find(n(2)).unwrap();
            f32s(b.determine_dial_shortage_for_battle(game, dial, treachery_engine::enum_type::from_value_or_absent::<Faction>(n(1)).unwrap().expect("faction"), t, n(3), n(4), n(5)))
        }
        _ => panic!("unknown probe method {m}"),
    }
}

fn run(name: &str) {
    run_probes("b2", name, Bots::Fresh, |game, b, m, a| call(game, b, m, a));
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
