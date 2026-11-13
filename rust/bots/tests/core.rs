//! The bot's dice, decision dispatch and parameters, against the C# bots.
//!
//! `golden/core.json` comes from `bots/csharp-ref`:
//!
//! ```sh
//! export DOTNET_ROOT="$HOME/dotnet10" PATH="$HOME/dotnet10:$PATH"
//! cd rust/bots/csharp-ref && dotnet run -c Release -- core ../tests/golden/core.json
//! ```

use std::cell::Cell;

use serde_json::Value;
use treachery_bots::prelude::*;

fn golden() -> Value {
    let path = format!("{}/tests/golden/core.json", env!("CARGO_MANIFEST_DIR"));
    serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap()
}

fn outcome<R>(f: impl FnOnce() -> R) -> Result<R, String> {
    cs::catch(f).map_err(|e| format!("EX:{}: {}", cs::full_type_name(e.type_name), e.message))
}

/// A game without players and a bot whose player is never read.
fn bot(seed: i32) -> (Game, ClassicBot) {
    let game = Game::new();
    let player = game.player_at(0);
    let bot = ClassicBot::new(player, BotParameters::for_faction(Faction::Atreides), SharedRandom::from_seed(seed));
    (game, bot)
}

#[test]
fn d_matches_csharp() {
    let g = golden();
    let (_game, bot) = bot(12345);
    for case in g["d"].as_array().unwrap() {
        let (amount, sides) = (case[0].as_i64().unwrap() as i32, case[1].as_i64().unwrap() as i32);
        let actual = match outcome(|| bot.d(amount, sides)) {
            Ok(r) => r.to_string(),
            Err(e) => e,
        };
        assert_eq!(actual, case[2].as_str().unwrap(), "D({amount}, {sides})");
        assert_eq!(bot.random.get_count() as i64, case[3].as_i64().unwrap(), "draws after D({amount}, {sides})");
    }
}

#[test]
fn parameters_match_csharp() {
    let g = golden();
    let f32s = |x: f32| dotnet_compat::fmt::f32_to_string(x);
    assert_eq!(f32s(BotParameters::PENALTY_FOR_ATTACKING_BOTS), g["penaltyForAttackingBots"].as_str().unwrap());
    for o in g["parameters"].as_array().unwrap() {
        let f = Faction(o["faction"].as_i64().unwrap() as i32);
        let p = BotParameters::for_faction(f);
        let actual = [
            ("Bidding_ResourcesToKeepWhenCardIsPerfect", p.bidding_resources_to_keep_when_card_is_perfect.to_string()),
            ("Bidding_ResourcesToKeepWhenCardIsntPerfect", p.bidding_resources_to_keep_when_card_isnt_perfect.to_string()),
            ("Bidding_PassingTreshold", p.bidding_passing_threshold.to_string()),
            ("Karma_SaveCardToUseSpecialKarmaAbility", p.karma_save_card_to_use_special_karma_ability.to_string()),
            ("Shipment_MinimumOtherPlayersITrustToPreventAWin", p.shipment_minimum_other_players_i_trust_to_prevent_a_win.to_string()),
            ("Shipment_DialShortageToAccept", f32s(p.shipment_dial_shortage_to_accept)),
            ("Shipment_MinimumResourcesToKeepForBattle", p.shipment_minimum_resources_to_keep_for_battle.to_string()),
            ("Shipment_MaxEnemyForceStrengthFightingForSpice", f32s(p.shipment_max_enemy_force_strength_fighting_for_spice)),
            ("Shipment_ExpectedStormMovesWhenUnknown", p.shipment_expected_storm_moves_when_unknown.to_string()),
            ("Shipment_DialForExtraForcesToShip", p.shipment_dial_for_extra_forces_to_ship.to_string()),
            ("Battle_MaximumUnsupportedForces", p.battle_maximum_unsupported_forces.to_string()),
            ("Battle_MimimumChanceToAssumeEnemyHeroSurvives", f32s(p.battle_minimum_chance_to_assume_enemy_hero_survives)),
            ("Battle_MimimumChanceToAssumeMyLeaderSurvives", f32s(p.battle_minimum_chance_to_assume_my_leader_survives)),
            ("Battle_DialShortageThresholdForThrowing", f32s(p.battle_dial_shortage_threshold_for_throwing)),
        ];
        assert_eq!(o.as_object().unwrap().len(), actual.len() + 1, "C# has a parameter the test doesn't know");
        for (name, value) in actual {
            assert_eq!(value, o[name].as_str().unwrap(), "{name} of faction {}", f.0);
        }
    }
}

/// `try_decide` against the C# `Do` scenarios. The golden's `result` says whether a valid event
/// was chosen, `action` which event ended the round. "alreadyDecided" has no counterpart: the
/// round returns at the first decision. "throwsWithBrace" ran on a formatting console, where the
/// C# error log throws on the brace; the bots never format, so it behaves like "throws".
#[test]
fn try_decide_matches_csharp() {
    let g = golden();
    for s in g["do"].as_array().unwrap() {
        let name = s["name"].as_str().unwrap();
        if name == "alreadyDecided" {
            continue;
        }
        let (mut game, mut bot) = bot(1);
        let called = Cell::new(false);
        let end_phase = [GameEventType::EndPhase];
        let thought = [GameEventType::Thought];

        let decision = match name {
            "valid" => bot.try_decide(&mut game, &end_phase, |_, g| { called.set(true); Some(Rc::new(EndPhase::new(g, Faction::Atreides))) }),
            "notAllowed" => bot.try_decide(&mut game, &thought, |_, g| { called.set(true); Some(Rc::new(EndPhase::new(g, Faction::Atreides))) }),
            "returnsNull" => bot.try_decide(&mut game, &thought, |_, _| -> Option<Rc<Thought>> { called.set(true); None }),
            "throws" => bot.try_decide(&mut game, &end_phase, |_, _| -> Option<Rc<EndPhase>> { called.set(true); throw!(InvalidOperationException, "boom") }),
            "throwsWithBrace" => bot.try_decide(&mut game, &end_phase, |_, _| -> Option<Rc<EndPhase>> { called.set(true); throw!(InvalidOperationException, "bo{{om") }),
            "invalid" => bot.try_decide(&mut game, &thought, |_, g| { called.set(true); Some(Rc::new(Thought::new(g, Faction::Atreides))) }),
            other => panic!("unknown scenario {other}"),
        };

        let expected_valid = if name == "throwsWithBrace" { "false" } else { s["result"].as_str().unwrap() };
        assert_eq!(matches!(decision, Some(Decision::Valid(_))).to_string(), expected_valid, "{name}: valid");
        let chosen = decision.map(|d| match d {
            Decision::Valid(e) | Decision::Invalid(e) => e.event_type().name().to_string(),
        });
        assert_eq!(chosen, s["action"].as_str().map(str::to_string), "{name}: action");
        assert_eq!(called.get(), s["called"].as_bool().unwrap(), "{name}: method called");
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

/// Only game exceptions are caught. A Rust panic must never become "chose nothing".
#[test]
fn try_decide_does_not_swallow_rust_panics() {
    let (mut game, mut bot) = bot(1);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        bot.try_decide(&mut game, &[GameEventType::EndPhase], |_, _| -> Option<Rc<EndPhase>> { todo!("somewhere") })
    }));
    assert!(result.is_err());
}

/// Draws made before a game exception stay.
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
