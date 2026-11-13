//! Bidding, deals and alliances (`bot/bidding.rs`, `bot/deals.rs`, `bot/alliance.rs`): every
//! `determine_*` method, plus `player_standing`, `wanna_have` and `determine_allowed_resources`,
//! with the bot's draw count after each call.
//!
//! The games are in `golden/knowledge-*.jsonl.gz`; `golden/b6-*.jsonl.gz` holds the probes and
//! their expected results. The probes run after each step (or every second step), whatever the
//! phase, so they also cover the panics these methods cause outside their phase (`determine_bid`
//! without a card on auction).
//!
//! The expected results are snapshots of the current behavior. After a deliberate change, rewrite
//! them with `UPDATE_GOLDENS=1 cargo test --release -p treachery-bots` and review the diff
//! (see `common/mod.rs`).

mod common;

use treachery_bots::bot::EventKind;
use treachery_bots::prelude::*;

use common::{run_probes, Bots};

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
        _ => panic!("unknown probe method {m}"),
    }
}

fn run(name: &str) {
    run_probes("b6", name, Bots::Shared, |game, b, m, a| call(game, b, m, a));
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
