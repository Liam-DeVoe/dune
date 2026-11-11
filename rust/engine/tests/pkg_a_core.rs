//! Focused tests for package A (Game.cs, GameEvent, PlacementEvent, Utilities and
//! the small model types). Expected values marked "(.NET)" were produced by running
//! the C# code (Shared) on .NET 10.

use treachery_engine::cs;
use treachery_engine::date_time_offset::TimeSpan;
use treachery_engine::enumerations::*;
use treachery_engine::events::{GameEvent, GameEventType, PlacementEvent};
use treachery_engine::game::Game;
use treachery_engine::object_counter::ObjectCounter;
use treachery_engine::timer::Timer;
use treachery_engine::treachery_card_manager::TreacheryCardManager;
use treachery_engine::utilities::Utilities;

fn outcome<R>(f: impl FnOnce() -> R) -> Result<R, String> {
    cs::catch(f).map_err(|e| format!("{}: {}", e.type_name, e.message))
}

#[test]
fn legacy_newtonsoft_conversion_matches_dotnet() {
    // (input, .NET output of Utilities.ConvertNewtonsoftJsonToPlainJson)
    let cases: &[(&str, Result<&str, &str>)] = &[
        (
            r#"{"$type":"Treachery.Shared.GameState, Treachery.Shared","Version":150,"_events":{"$type":"System.Collections.Generic.List`1[[Treachery.Shared.GameEvent, Treachery.Shared]], System.Private.CoreLib","$values":[{"$type":"Treachery.Shared.EndPhase, Treachery.Shared","Initiator":0},null,{"$type":"Treachery.Shared.Removed, Treachery.Shared","X":1}]}}"#,
            Ok(r#"{"Version":150,"_events":[{"$type":"EndPhase","Initiator":0},{"X":1}]}"#),
        ),
        // A wrapper at the root has no parent and stays.
        (r#"{"$values":[1,2]}"#, Ok(r#"{"$values":[1,2]}"#)),
        // A wrapper inside a wrapper is not replaced, and its contents are not processed.
        (
            r#"{"a":{"$values":[{"$values":[{"$type":"Treachery.Shared.Bid"}]}]}}"#,
            Ok(r#"{"a":[{"$values":[{"$type":"Treachery.Shared.Bid"}]}]}"#),
        ),
        // A wrapper directly inside an array modifies the array during its enumeration.
        (
            r#"{"a":[{"$values":[1]}]}"#,
            Err("InvalidOperationException: Collection was modified; enumeration operation may not execute."),
        ),
        // $type keeps its position when renamed.
        (r#"{"b":1,"$type":"Treachery.Shared.Bid","c":2}"#, Ok(r#"{"b":1,"$type":"Bid","c":2}"#)),
        (r#"{"$type":" Treachery.Shared.Bid"}"#, Ok(r#"{"$type":"Bid"}"#)),
        (r#"{"$type":"Treachery.Shared.Bid , Treachery.Shared"}"#, Ok(r#"{}"#)),
        (r#"{"$type":"Treachery.Shared.PlacementEvent"}"#, Ok(r#"{}"#)),
        (r#"{"$type":"treachery.shared.Bid"}"#, Ok(r#"{}"#)),
        (r#"{"$type":"Treachery.Shared.Bid,Treachery.Shared,Version=1.0.0.0"}"#, Ok(r#"{"$type":"Bid"}"#)),
        (r#"{"$type":"Treachery.Shared.Bid, Other"}"#, Ok(r#"{}"#)),
        (
            r#"{"$type":5}"#,
            Err("InvalidOperationException: An element of type 'Number' cannot be converted to a 'System.String'."),
        ),
        (r#"{"$type":"Treachery.Shared.Bid "}"#, Ok(r#"{}"#)),
        ("{\"$type\":\"\\tTreachery.Shared.Bid\"}", Ok(r#"{"$type":"Bid"}"#)),
        (r#"{"$type":"Treachery.Shared.Bid,  Treachery.Shared "}"#, Ok(r#"{"$type":"Bid"}"#)),
        (r#"{"$type":"Treachery.Shared.Bid, treachery.shared"}"#, Ok(r#"{"$type":"Bid"}"#)),
        (r#"{"$type":"Treachery.Shared.Bid, Treachery.Shared, Version=9.9.9.9"}"#, Ok(r#"{"$type":"Bid"}"#)),
        (r#"{"$type":"Treachery.Shared.Bid[]"}"#, Ok(r#"{}"#)),
        (r#"{"$type":"Treachery.Shared.Bid+X"}"#, Ok(r#"{}"#)),
        ("null", Ok("")),
    ];

    for (input, expected) in cases {
        let actual = outcome(|| Utilities::convert_newtonsoft_json_to_plain_json(input));
        let expected = expected.map(|s| s.to_string()).map_err(|s| s.to_string());
        assert_eq!(actual, expected, "input {input}");
    }
}

#[test]
fn determine_game_event() {
    assert_eq!(Utilities::determine_game_event("Treachery.Shared.EndPhase"), Some(GameEventType::EndPhase));
    assert_eq!(Utilities::determine_game_event("Treachery.Shared.GameState"), None);
}

#[test]
fn object_counter_highest_takes_first_maximum_in_slot_order() {
    let mut c: ObjectCounter<i32> = ObjectCounter::new();
    assert_eq!(c.highest(), None);
    c.count(1);
    c.count2(2);
    c.count_n(3, 2);
    c.count(1);
    // 1 -> 2, 2 -> 2, 3 -> 2: the first in enumeration order wins
    assert_eq!(c.highest(), Some(1));
    c.set_to_n(3, 5);
    assert_eq!(c.highest(), Some(3));
    assert_eq!(c.count_of(9), 0);
    // OrderByDescending is stable: ties keep slot order
    assert_eq!(c.get_highest(3), vec![3, 1, 2]);
    assert_eq!(c.get_highest(-1), Vec::<i32>::new());
    assert_eq!(c.counted(), vec![1, 2, 3]);
}

#[test]
fn timer_accumulates() {
    let mut t: Timer<MainPhase> = Timer::new();
    assert_eq!(t.time_spent(MainPhase::Bidding), TimeSpan::default());
    t.add(MainPhase::Bidding, TimeSpan::from_ticks(5));
    t.add(MainPhase::Bidding, TimeSpan::from_ticks(-7));
    assert_eq!(t.time_spent(MainPhase::Bidding), TimeSpan::from_ticks(-2));
    let r = outcome(|| {
        let mut t: Timer<MainPhase> = Timer::new();
        t.add(MainPhase::Bidding, TimeSpan::from_ticks(i64::MAX));
        t.add(MainPhase::Bidding, TimeSpan::from_ticks(1));
    });
    assert_eq!(r, Err("OverflowException: TimeSpan overflowed because the duration is too long.".to_string()));
}

#[test]
fn time_span_total_hours_threshold() {
    let hour = 36_000_000_000i64;
    assert!(TimeSpan::from_ticks(hour - 1).total_hours() < 1.0);
    assert!(!(TimeSpan::from_ticks(hour).total_hours() < 1.0));
    assert!(TimeSpan::from_ticks(-hour * 5).total_hours() < 1.0);
}

#[test]
fn id_strings() {
    let lookup = TreacheryCardManager::lookup();
    assert!(GameEvent::id_string_to_objects(None, &lookup).is_empty());
    assert!(GameEvent::id_string_to_objects(Some(""), &lookup).is_empty());
    let cards = GameEvent::id_string_to_objects(Some("1;2"), &lookup);
    assert_eq!(cards.len(), 2);
    assert_eq!(GameEvent::objects_to_id_string(&cards, &lookup), "1;2");
    let r = outcome(|| GameEvent::id_string_to_objects(Some("1;;2"), &lookup).len());
    assert_eq!(r, Err("FormatException: The input string '' was not in a correct format.".to_string()));
}

#[test]
fn game_version_check() {
    let r = outcome(|| {
        Game::new_int32_participation(99, &Default::default());
    });
    assert_eq!(
        r,
        Err("ArgumentException: Game version 99 is not supported. The lowest supported version is: 100.".to_string())
    );
}

/// Needs a constructed `Game`, which needs package C's `EnterPhaseAwaitingPlayers`
/// and package K's `RemoveEndedDeals`. Remove the `ignore` once those are ported.
#[test]
#[ignore = "Game::new needs packages C and K"]
fn parse_force_locations_matches_dotnet() {
    let g = Game::new();
    // (input, .NET result)
    let cases: &[(&str, Result<&str, &str>)] = &[
        ("", Ok("")),
        ("5:1|2,7:3|0", Ok("5=1/2;7=3/0")),
        ("5:1|2,5:3|0", Err("ArgumentException: An item with the same key has already been added. Key: Treachery.Shared.Location")),
        ("5", Err("IndexOutOfRangeException: Index was outside the bounds of the array.")),
        ("5:1", Err("IndexOutOfRangeException: Index was outside the bounds of the array.")),
        ("x:1|2", Err("FormatException: The input string 'x' was not in a correct format.")),
        (
            "999:1|2",
            Err("ArgumentOutOfRangeException: Index was out of range. Must be non-negative and less than the size of the collection. (Parameter 'index')"),
        ),
        ("5:1|2|9", Ok("5=1/2")),
        ("5: 1 |2", Ok("5=1/2")),
        ("-1:1|2", Ok("")),
    ];
    for (input, expected) in cases {
        let actual = outcome(|| {
            let d = PlacementEvent::parse_force_locations(&g, Faction::Emperor, input);
            d.iter()
                .map(|(l, b)| format!("{}={}/{}", l.id(), b.amount_of_forces, b.amount_of_special_forces))
                .collect::<Vec<_>>()
                .join(";")
        });
        let expected = expected.map(|s| s.to_string()).map_err(|s| s.to_string());
        assert_eq!(actual, expected, "input {input}");
    }
}
