//! Behavioural differences between the foundation layer and the C# code,
//! found by review. Each test states the C# result, taken from
//! `cd rust/engine/csharp-ref && dotnet run -c Release -- review`
//! (see csharp-ref/Review.cs; the probe name is given in each test).
//!
//! Each test failed until its difference was fixed (see rust/docs/escapes.md),
//! except the ones marked as guards. See rust/docs/reviews/foundation.md.

use treachery_engine::compat::cs::{self, CsIterExt};
use treachery_engine::enumerations::*;
use treachery_engine::messages::expression::ExpressionElement;
use treachery_engine::compat::random_ext::{set_static_random_seed, RandomExt, UnseededRandomExt};
use treachery_engine::random::SharedRandom;
use treachery_engine::messages::message::{PlanSummary, Route};

fn exception_text<R>(f: impl FnOnce() -> R) -> String {
    match cs::catch_panic(f) {
        Ok(_) => "no exception".to_string(),
        Err(e) => format!("{}: {}", e.type_name, e.message),
    }
}

// ---------------------------------------------------------------- enumerations.rs

/// Probe `to_nexus_13`. C# `throw new ArgumentOutOfRangeException(nameof(faction), faction, null)`
/// produces the default message with the parameter and actual value; Rust uses
/// the parameter name as the whole message.
#[test]
fn to_nexus_undefined_message() {
    assert_eq!(
        exception_text(|| Faction(13).to_nexus()),
        "ArgumentOutOfRangeException: Specified argument was out of the range of valid values. (Parameter 'faction')\nActual value was 13."
    );
}

/// Probe `to_faction_13`.
#[test]
fn to_faction_undefined_message() {
    assert_eq!(
        exception_text(|| Nexus(13).to_faction()),
        "ArgumentOutOfRangeException: Specified argument was out of the range of valid values. (Parameter 'nexus')\nActual value was 13."
    );
}

// ---------------------------------------------------------------- extension_methods.rs

/// The next value of the static random, observed through a 1000-item pick.
fn next_static_pick() -> i32 {
    (0..1000).random_or_default_unseeded().unwrap()
}

/// Probes `take_random_n_0_draws` and `orderby_take0_key_calls`. C#
/// `source.OrderBy(_ => Random.Next()).Take(0)` never computes a key, so the
/// static Random is untouched. Rust draws one key per element first.
#[test]
fn take_random_n_zero_draws_nothing() {
    set_static_random_seed(7);
    let expected = next_static_pick();

    set_static_random_seed(7);
    let taken: Vec<i32> = (0..5).take_random_n(0).collect();
    assert!(taken.is_empty());
    assert_eq!(next_static_pick(), expected, "take_random_n(0) consumed random draws");
}

/// Probe `orderby_take_neg_key_calls`: the same for a negative count.
#[test]
fn take_random_n_negative_draws_nothing() {
    set_static_random_seed(7);
    let expected = next_static_pick();

    set_static_random_seed(7);
    let _: Vec<i32> = (0..5).take_random_n(-3).collect();
    assert_eq!(next_static_pick(), expected, "take_random_n(-3) consumed random draws");
}

/// Probe `take_random_n_lazy_draws_before_enumeration`. C# `TakeRandomN`
/// returns a lazy query: no draw happens until it is enumerated, and each
/// enumeration draws again. Rust draws when called.
#[test]
fn take_random_n_is_lazy() {
    set_static_random_seed(7);
    let expected = next_static_pick();

    set_static_random_seed(7);
    let _not_enumerated = (0..5).take_random_n(2);
    assert_eq!(next_static_pick(), expected, "creating the query consumed random draws");
}

/// Probe `highest_string`. C# compares `IComparable` keys with
/// `Comparer.Default`, which is culture-sensitive for strings ("a" < "B"), so
/// the highest of {"a", "B"} is "B". Rust `String: Ord` is ordinal ('B' < 'a').
#[test]
fn highest_or_default_string_keys_use_culture_order() {
    let random = SharedRandom::from_seed(1);
    let picked = ["a", "B"].into_iter().highest_or_default(&random, |s| s.to_string());
    assert_eq!(picked, Some("B"));
}

/// Probe `one_of_highest_1_string`.
#[test]
fn one_of_highest_n_string_keys_use_culture_order() {
    let random = SharedRandom::from_seed(1);
    let picked = ["a", "B"].into_iter().one_of_highest_n_or_default(&random, |s| s.to_string(), 1);
    assert_eq!(picked, Some("B"));
}

/// Probes `one_of_highest_selector_calls` / `one_of_lowest_selector_calls`.
/// `OrderBy` computes each key once (5 calls for 5 items); `sort_by` calls the
/// selector twice per comparison. Only visible with a selector that has side
/// effects, such as a bot selector that draws random numbers.
#[test]
fn one_of_highest_n_calls_selector_once_per_element() {
    let random = SharedRandom::from_seed(1);
    let calls = std::cell::Cell::new(0);
    let _ = [5, 3, 9, 1, 7].into_iter().one_of_highest_n_or_default(
        &random,
        |x| {
            calls.set(calls.get() + 1);
            *x
        },
        2,
    );
    assert_eq!(calls.get(), 5);
}

#[test]
fn one_of_lowest_n_calls_selector_once_per_element() {
    let random = SharedRandom::from_seed(1);
    let calls = std::cell::Cell::new(0);
    let _ = [5, 3, 9, 1, 7].into_iter().one_of_lowest_n_or_default(
        &random,
        |x| {
            calls.set(calls.get() + 1);
            *x
        },
        2,
    );
    assert_eq!(calls.get(), 5);
}

/// Probe `highest_selector_calls`: 10 in both (Max pass plus filter pass).
/// Passes; kept as a guard.
#[test]
fn highest_or_default_calls_selector_twice_per_element() {
    let random = SharedRandom::from_seed(1);
    let calls = std::cell::Cell::new(0);
    let _ = [5, 3, 9, 1, 7].into_iter().highest_or_default(&random, |x| {
        calls.set(calls.get() + 1);
        *x
    });
    assert_eq!(calls.get(), 10);
}

// ---------------------------------------------------------------- cs.rs

/// Probe `single_or_default_pred_two`. `SingleOrDefault(predicate)` says
/// "matching element". The foundation first ported it as
/// `filter(..).cs_single_or_default()`, which gives the message of the
/// predicate-less overload; the predicate overload fixes that.
#[test]
fn single_or_default_with_predicate_message() {
    assert_eq!(
        exception_text(|| [1, 1].iter().cs_single_or_default_where(|x| **x == 1)),
        "InvalidOperationException: Sequence contains more than one matching element"
    );
}

/// Probe `single_pred_none`: `Single(predicate)` with no match.
#[test]
fn single_with_predicate_no_match_message() {
    assert_eq!(
        exception_text(|| [1].iter().cs_single_where(|x| **x == 2)),
        "InvalidOperationException: Sequence contains no matching element"
    );
}

/// Probe `single_pred_two`: `Single(predicate)` with two matches.
#[test]
fn single_with_predicate_two_matches_message() {
    assert_eq!(
        exception_text(|| [1, 1].iter().cs_single_where(|x| **x == 1)),
        "InvalidOperationException: Sequence contains more than one matching element"
    );
}

// ---------------------------------------------------------------- expression.rs / message.rs

/// Probe `msg_float_small`. .NET's shortest round-trip float formatting
/// switches to exponent notation; Rust's `Display` never does.
#[test]
fn float_to_cs_string_small() {
    assert_eq!(ExpressionElement::Float(0.00001).to_cs_string(), "1E-05");
}

/// Probe `msg_float_big`.
#[test]
fn float_to_cs_string_big() {
    assert_eq!(ExpressionElement::Float(1e20).to_cs_string(), "1E+20");
}

/// Probe `msg_double_small`.
#[test]
fn double_to_cs_string_small() {
    assert_eq!(ExpressionElement::Double(1e-7).to_cs_string(), "1E-07");
}

/// Probes `float_inf_invariant` ("Infinity") and `msg_float_inf` ("∞" under
/// en-US). Rust prints "inf", which matches neither culture. The oracle is
/// meant to render under the invariant culture (hazards 11.2), so this checks
/// that. `dotnet_compat::fmt::f32_to_string` already implements it.
#[test]
fn float_to_cs_string_infinity() {
    assert_eq!(ExpressionElement::Float(f32::INFINITY).to_cs_string(), "Infinity");
}

/// A game error prints as its message.
#[test]
fn game_error_to_cs_string() {
    let e = treachery_engine::error::GameError::new("Cannot draw from an empty deck");
    assert_eq!(ExpressionElement::Error(e).to_cs_string(), "Cannot draw from an empty deck");
}

/// Probe `msg_list`. Known and documented in expression.rs; listed for completeness.
#[test]
fn list_to_cs_string_type_name() {
    let l = ExpressionElement::from(vec![Faction::Atreides]);
    assert_eq!(l.to_cs_string(), "System.Collections.Generic.List`1[Treachery.Shared.Faction]");
}

/// Probe `plan_summary_nan_equal`. C# record equality uses
/// `EqualityComparer<float>.Default`, where NaN equals NaN.
#[test]
fn plan_summary_equality_with_nan_dial() {
    let p = PlanSummary {
        faction: Faction::Atreides,
        leader: None,
        messiah: false,
        dial: f32::NAN,
        spice: None,
        ally_contribution: 0,
        ally: Faction::None,
        weapon: None,
        defense: None,
    };
    assert!(p == p.clone());
}

/// Probe `route_equal_distinct_lists`. C# record equality compares the
/// `List<Location> From` by reference, so two routes with separate empty
/// lists are not equal.
#[test]
fn route_equality_compares_list_by_reference() {
    let a = Route { faction: Faction::Atreides, from: cs::Shared::new(Vec::new()), to: None };
    let b = Route { faction: Faction::Atreides, from: cs::Shared::new(Vec::new()), to: None };
    assert!(a != b);
    assert!(a == a.clone());
}
