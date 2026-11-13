//! Bot.cs and BotParameters.cs against C#: `LogError` formatting, `D`, `Do` and the parameters.
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
    cs::catch_panic(f).map_err(|e| format!("EX:{}: {}", cs::full_type_name(e.type_name), e.message))
}

/// A game without players and a bot whose player is never read (C#: `new Player(game, Atreides)`).
fn bot(seed: i32) -> (Game, ClassicBot) {
    let game = Game::new();
    let player = game.player_at(0);
    let bot = ClassicBot::new(&game, player, BotParameters::get_default_parameters(Faction::Atreides), Some(SharedRandom::from_seed(seed)));
    (game, bot)
}

/// `string.Format`'s parse on a formatting console, against .NET's results. `LogError` itself
/// never formats (see `ClassicBot::log_error`), so it never throws, whatever the text.
#[test]
fn log_error_never_throws_and_format_check_matches_dotnet() {
    let g = golden();
    for case in g["logError"].as_array().unwrap() {
        let (msg, expected) = (case[0].as_str().unwrap(), case[1].as_str().unwrap());
        let actual = match outcome(|| treachery_bots::bot::string_format_check(msg, 0)) {
            Ok(()) => "OK".to_string(),
            Err(e) => e,
        };
        assert_eq!(actual, expected, "Console.WriteLine({msg:?}, [])");
        assert!(outcome(|| ClassicBot::log_error(msg, vec![])).is_ok(), "LogError({msg:?}) threw");
    }
}

#[test]
fn d_matches_csharp() {
    let g = golden();
    let (game, bot) = bot(12345);
    for case in g["d"].as_array().unwrap() {
        let (amount, sides) = (case[0].as_i64().unwrap() as i32, case[1].as_i64().unwrap() as i32);
        let actual = match outcome(|| bot.d(&game, amount, sides)) {
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
        let p = BotParameters::get_default_parameters(f);
        let actual = [
            ("Bidding_ResourcesToKeepWhenCardIsPerfect", p.bidding_resources_to_keep_when_card_is_perfect.to_string()),
            ("Bidding_ResourcesToKeepWhenCardIsntPerfect", p.bidding_resources_to_keep_when_card_isnt_perfect.to_string()),
            ("Bidding_PassingTreshold", p.bidding_passing_treshold.to_string()),
            ("Karma_SaveCardToUseSpecialKarmaAbility", p.karma_save_card_to_use_special_karma_ability.to_string()),
            ("Shipment_MinimumOtherPlayersITrustToPreventAWin", p.shipment_minimum_other_players_i_trust_to_prevent_a_win.to_string()),
            ("Shipment_DialShortageToAccept", f32s(p.shipment_dial_shortage_to_accept)),
            ("Shipment_MinimumResourcesToKeepForBattle", p.shipment_minimum_resources_to_keep_for_battle.to_string()),
            ("Shipment_MaxEnemyForceStrengthFightingForSpice", f32s(p.shipment_max_enemy_force_strength_fighting_for_spice)),
            ("Shipment_ExpectedStormMovesWhenUnknown", p.shipment_expected_storm_moves_when_unknown.to_string()),
            ("Shipment_DialForExtraForcesToShip", p.shipment_dial_for_extra_forces_to_ship.to_string()),
            ("Battle_MaximumUnsupportedForces", p.battle_maximum_unsupported_forces.to_string()),
            ("Battle_MimimumChanceToAssumeEnemyHeroSurvives", f32s(p.battle_mimimum_chance_to_assume_enemy_hero_survives)),
            ("Battle_MimimumChanceToAssumeMyLeaderSurvives", f32s(p.battle_mimimum_chance_to_assume_my_leader_survives)),
            ("Battle_DialShortageThresholdForThrowing", f32s(p.battle_dial_shortage_threshold_for_throwing)),
        ];
        assert_eq!(o.as_object().unwrap().len(), actual.len() + 1, "C# has a parameter the test doesn't know");
        for (name, value) in actual {
            assert_eq!(value, o[name].as_str().unwrap(), "{name} of faction {}", f.0);
        }
    }
}

#[test]
fn do_matches_csharp() {
    let g = golden();
    for s in g["do"].as_array().unwrap() {
        let name = s["name"].as_str().unwrap();
        let (mut game, mut bot) = bot(1);
        let existing: GameEvent = Rc::new(EndPhase::new(&game, Faction::Atreides)).into();
        let called = Cell::new(false);
        let mut action: Option<GameEvent> = if name == "alreadyDecided" { Some(existing.clone()) } else { None };
        let before = action.clone();
        let end_phase = [GameEventType::EndPhase];
        let thought = [GameEventType::Thought];

        let result = outcome(|| match name {
            "valid" => bot.do_(&mut game, |_, g| { called.set(true); Some(Rc::new(EndPhase::new(g, Faction::Atreides))) }, &mut action, &end_phase),
            "notAllowed" => bot.do_(&mut game, |_, g| { called.set(true); Some(Rc::new(EndPhase::new(g, Faction::Atreides))) }, &mut action, &thought),
            "alreadyDecided" => bot.do_(&mut game, |_, g| { called.set(true); Some(Rc::new(EndPhase::new(g, Faction::Atreides))) }, &mut action, &end_phase),
            "returnsNull" => bot.do_(&mut game, |_, _| -> Option<Rc<Thought>> { called.set(true); None }, &mut action, &thought),
            "throws" => bot.do_(&mut game, |_, _| -> Option<Rc<EndPhase>> { called.set(true); treachery_engine::compat::cs::throw("InvalidOperationException", "boom".to_string()) }, &mut action, &end_phase),
            "throwsWithBrace" => bot.do_(&mut game, |_, _| -> Option<Rc<EndPhase>> { called.set(true); treachery_engine::compat::cs::throw("InvalidOperationException", "bo{om".to_string()) }, &mut action, &end_phase),
            "invalid" => bot.do_(&mut game, |_, g| { called.set(true); Some(Rc::new(Thought::new(g, Faction::Atreides))) }, &mut action, &thought),
            other => panic!("unknown scenario {other}"),
        });

        let result = match result {
            Ok(b) => b.to_string(),
            Err(e) => e,
        };
        // The golden ran `Do` on a formatting console without a describer, where `LogError`
        // throws on the brace. Production (the server sets `Message.DefaultDescriber`) and the
        // oracle (`TextWriter.Null`) never format, so `Do` returns false there, as the port does.
        let expected = if name == "throwsWithBrace" { "false" } else { s["result"].as_str().unwrap() };
        assert_eq!(result, expected, "{name}: result");
        assert_eq!(action.as_ref().map(|a| a.event_type().name().to_string()), s["action"].as_str().map(str::to_string), "{name}: action");
        let same = match (&action, &before) {
            (None, None) => true,
            (Some(a), Some(b)) => a == b,
            _ => false,
        };
        assert_eq!(same, s["sameAction"].as_bool().unwrap(), "{name}: same action object");
        assert_eq!(called.get(), s["called"].as_bool().unwrap(), "{name}: method called");
    }
}

/// An invalid decision blocks the rest of the tier (hazards 16.1): later `Do`s don't run.
#[test]
fn invalid_decision_blocks_the_tier() {
    let (mut game, mut bot) = bot(1);
    let mut action = None;
    let later = Cell::new(false);
    let first = bot.do_(&mut game, |_, g| Some(Rc::new(Thought::new(g, Faction::Atreides))), &mut action, &[GameEventType::Thought, GameEventType::EndPhase]);
    let second = bot.do_(
        &mut game,
        |_, g| {
            later.set(true);
            Some(Rc::new(EndPhase::new(g, Faction::Atreides)))
        },
        &mut action,
        &[GameEventType::Thought, GameEventType::EndPhase],
    );
    assert!(!first && !second && !later.get());
}

/// `Do` catches any panic in the decision, including a plain Rust panic such as an unported
/// `todo!`. The panic is logged and counts as no decision: `Do` returns false and leaves the
/// action empty.
#[test]
fn a_panicking_decision_counts_as_no_decision() {
    let (mut game, mut bot) = bot(1);
    let mut action = None;
    let decided = bot.do_(&mut game, |_, _| -> Option<Rc<EndPhase>> { todo!("port: somewhere") }, &mut action, &[GameEventType::EndPhase]);
    assert!(!decided);
    assert!(action.is_none());
}

/// Draws made before a decision panics stay (hazards 16.2).
#[test]
fn draws_before_an_exception_stay() {
    let (mut game, mut bot) = bot(1);
    let mut action = None;
    let r = bot.do_(
        &mut game,
        |b, g| -> Option<Rc<EndPhase>> {
            b.d(g, 2, 6);
            b.d(g, 1, -1);
            None
        },
        &mut action,
        &[GameEventType::EndPhase],
    );
    assert!(!r);
    assert_eq!(bot.random.get_count(), 3);
}
