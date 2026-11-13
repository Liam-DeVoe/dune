//! The bot's dice, decision dispatch and parameters.
//!
//! `golden/core.json` holds the expected dice results (with the draw count after each roll) and
//! every faction's parameters, as snapshots of the current behavior. After a deliberate change,
//! rewrite it with `UPDATE_GOLDENS=1 cargo test --release -p treachery-bots --test core` and
//! review the diff.

mod common;

use std::cell::Cell;

use serde_json::{json, Value};
use treachery_bots::prelude::*;
use treachery_engine::error::catch_panic;

use common::{read_json, updating, write_json};

const GOLDEN: &str = "core.json";

/// A game without players and a bot whose player is never read.
fn bot(seed: i32) -> (Game, ClassicBot) {
    let game = Game::new();
    let player = game.player_at(0);
    let bot = ClassicBot::new(player, BotParameters::for_faction(Faction::Atreides), SharedRandom::from_seed(seed));
    (game, bot)
}

/// Rewrites one section of the snapshot. The tests run in parallel, so the rewrites take turns.
fn update(key: &str, value: Value) {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _turn = LOCK.lock().unwrap();
    let mut g = read_json(GOLDEN);
    g[key] = value;
    write_json(GOLDEN, &g);
}

#[test]
fn dice() {
    let g = read_json(GOLDEN);
    let (_game, bot) = bot(12345);
    let mut actual = vec![];
    for case in g["d"].as_array().unwrap() {
        let (amount, sides) = (case[0].as_i64().unwrap() as i32, case[1].as_i64().unwrap() as i32);
        let result = catch_panic(|| bot.d(amount, sides)).map_or_else(|e| format!("EX:{e}"), |r| r.to_string());
        actual.push(json!([amount, sides, result, bot.random.get_count()]));
    }
    if updating() {
        update("d", json!(actual));
        return;
    }
    for (case, actual) in g["d"].as_array().unwrap().iter().zip(&actual) {
        assert_eq!(actual, case, "D({}, {}): [amount, sides, result, draws after]", case[0], case[1]);
    }
}

#[test]
fn parameters() {
    let g = read_json(GOLDEN);
    let f32s = |x: f32| dotnet_compat::fmt::f32_to_string(x);
    let mut actual = vec![];
    for o in g["parameters"].as_array().unwrap() {
        let f = Faction(o["faction"].as_i64().unwrap() as i32);
        let p = BotParameters::for_faction(f);
        actual.push(json!({
            "faction": f.0,
            "Bidding_ResourcesToKeepWhenCardIsPerfect": p.bidding_resources_to_keep_when_card_is_perfect.to_string(),
            "Bidding_ResourcesToKeepWhenCardIsntPerfect": p.bidding_resources_to_keep_when_card_isnt_perfect.to_string(),
            "Bidding_PassingTreshold": p.bidding_passing_threshold.to_string(),
            "Karma_SaveCardToUseSpecialKarmaAbility": p.karma_save_card_to_use_special_karma_ability.to_string(),
            "Shipment_MinimumOtherPlayersITrustToPreventAWin": p.shipment_minimum_other_players_i_trust_to_prevent_a_win.to_string(),
            "Shipment_DialShortageToAccept": f32s(p.shipment_dial_shortage_to_accept),
            "Shipment_MinimumResourcesToKeepForBattle": p.shipment_minimum_resources_to_keep_for_battle.to_string(),
            "Shipment_MaxEnemyForceStrengthFightingForSpice": f32s(p.shipment_max_enemy_force_strength_fighting_for_spice),
            "Shipment_ExpectedStormMovesWhenUnknown": p.shipment_expected_storm_moves_when_unknown.to_string(),
            "Shipment_DialForExtraForcesToShip": p.shipment_dial_for_extra_forces_to_ship.to_string(),
            "Battle_MaximumUnsupportedForces": p.battle_maximum_unsupported_forces.to_string(),
            "Battle_MimimumChanceToAssumeEnemyHeroSurvives": f32s(p.battle_minimum_chance_to_assume_enemy_hero_survives),
            "Battle_MimimumChanceToAssumeMyLeaderSurvives": f32s(p.battle_minimum_chance_to_assume_my_leader_survives),
            "Battle_DialShortageThresholdForThrowing": f32s(p.battle_dial_shortage_threshold_for_throwing),
        }));
    }
    let penalty = f32s(BotParameters::PENALTY_FOR_ATTACKING_BOTS);
    if updating() {
        update("parameters", json!(actual));
        update("penaltyForAttackingBots", json!(penalty));
        return;
    }
    assert_eq!(penalty, g["penaltyForAttackingBots"].as_str().unwrap(), "PENALTY_FOR_ATTACKING_BOTS");
    for (o, actual) in g["parameters"].as_array().unwrap().iter().zip(&actual) {
        assert_eq!(actual, o, "parameters of faction {}", o["faction"]);
    }
}

/// `try_decide` returns the first decision of the round: a valid event, an event that isn't
/// allowed or doesn't validate (still a decision, since it ends the round), or nothing when the
/// decision method returns nothing or panics.
#[test]
fn try_decide_scenarios() {
    let end_phase = [GameEventType::EndPhase];
    let thought = [GameEventType::Thought];
    // (scenario, valid, event type that ended the round)
    let expected: [(&str, bool, Option<&str>); 5] = [
        ("valid", true, Some("EndPhase")),
        ("notAllowed", false, None),
        ("returnsNull", false, None),
        ("throws", false, None),
        ("invalid", false, Some("Thought")),
    ];
    for (name, valid, action) in expected {
        let (mut game, mut bot) = bot(1);
        let called = Cell::new(false);
        let decision = match name {
            "valid" => bot.try_decide(&mut game, &end_phase, |_, g| { called.set(true); Some(Rc::new(EndPhase::new(g, Faction::Atreides))) }),
            "notAllowed" => bot.try_decide(&mut game, &thought, |_, g| { called.set(true); Some(Rc::new(EndPhase::new(g, Faction::Atreides))) }),
            "returnsNull" => bot.try_decide(&mut game, &thought, |_, _| -> Option<Rc<Thought>> { called.set(true); None }),
            "throws" => bot.try_decide(&mut game, &end_phase, |_, _| -> Option<Rc<EndPhase>> { called.set(true); panic!("boom") }),
            "invalid" => bot.try_decide(&mut game, &thought, |_, g| { called.set(true); Some(Rc::new(Thought::new(g, Faction::Atreides))) }),
            _ => unreachable!(),
        };
        assert_eq!(matches!(decision, Some(Decision::Valid(_))), valid, "{name}: valid");
        let chosen = decision.map(|d| match d {
            Decision::Valid(e) | Decision::Invalid(e) => e.event_type().name().to_string(),
        });
        assert_eq!(chosen.as_deref(), action, "{name}: action");
        // A type that isn't allowed is never asked for.
        assert_eq!(called.get(), name != "notAllowed", "{name}: method called");
    }
}

/// An invalid event is still a decision (it ends the round), but it isn't submitted.
#[test]
fn invalid_decision_is_not_submitted() {
    let (mut game, mut bot) = bot(1);
    let allowed = [GameEventType::Thought];
    let decision = bot.try_decide(&mut game, &allowed, |_, g| Some(Rc::new(Thought::new(g, Faction::Atreides))));
    assert!(matches!(decision, Some(Decision::Invalid(_))));
    assert!(decision.unwrap().valid().is_none());
}

/// `try_decide` catches any panic in the decision, including a plain Rust panic such as a
/// `todo!`. The panic counts as no decision.
#[test]
fn a_panicking_decision_counts_as_no_decision() {
    let (mut game, mut bot) = bot(1);
    let decision = bot.try_decide(&mut game, &[GameEventType::EndPhase], |_, _| -> Option<Rc<EndPhase>> { todo!("somewhere") });
    assert!(decision.is_none());
}

/// Draws made before a decision panics stay.
#[test]
fn draws_before_an_exception_stay() {
    let (mut game, mut bot) = bot(1);
    let r = bot.try_decide(&mut game, &[GameEventType::EndPhase], |b, _| -> Option<Rc<EndPhase>> {
        b.d(2, 6);
        b.d(1, -1);
        None
    });
    assert!(r.is_none());
    assert_eq!(bot.random.get_count(), 3);
}
