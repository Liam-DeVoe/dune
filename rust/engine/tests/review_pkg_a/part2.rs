//! Package A review, part 2 (included from review_pkg_a.rs): EndPhase, PlacementEvent, Utilities, GameState and the
//! small model types. Expected values come from `csharp-ref` mode `review-a-part2`
//! (csharp-ref/ReviewA2.cs), run on .NET 10.

use treachery_engine::compat::cs;
use treachery_engine::enumerations::*;
use treachery_engine::events::PlacementEvent;
use treachery_engine::game::Game;
use treachery_engine::game::GameState;
use treachery_engine::json::utilities::Utilities;

fn outcome<R: ToString>(f: impl FnOnce() -> R) -> String {
    match cs::catch_panic(f) {
        Ok(r) => r.to_string(),
        Err(e) => format!("exception:{}:{}", e.type_name, e.message),
    }
}

fn parse(g: &Game, s: &str) -> String {
    PlacementEvent::parse_force_locations(g, Faction::Emperor, s)
        .iter()
        .map(|(l, b)| format!("{}={}/{}", l.id(), b.amount_of_forces, b.amount_of_special_forces))
        .collect::<Vec<_>>()
        .join(";")
}

/// Duplicate-key messages use `Location.ToString()`, which differs per subclass.
/// Ids from .NET: Emperor homeworld 90, Shrine 103, HMS 86.
#[test]
fn parse_force_locations_edge_cases_match_dotnet() {
    let g = Game::new();
    let cases: &[(&str, &str)] = &[
        ("90:1|0,90:2|0", "exception:ArgumentException:An item with the same key has already been added. Key: Homeworld"),
        (
            "103:1|0,103:2|0",
            "exception:ArgumentException:An item with the same key has already been added. Key: Treachery.Shared.DiscoveredLocation",
        ),
        (
            "86:1|0,86:2|0",
            "exception:ArgumentException:An item with the same key has already been added. Key: Treachery.Shared.HiddenMobileStronghold",
        ),
        ("5:1|2,", "exception:FormatException:The input string '' was not in a correct format."),
        ("+5:+1|-0", "5=1/0"),
        ("5:2147483648|0", "exception:OverflowException:Value was either too large or too small for an Int32."),
        (
            "-2:1|0",
            "exception:ArgumentOutOfRangeException:Index was out of range. Must be non-negative and less than the size of the collection. (Parameter 'index')",
        ),
    ];
    for (input, expected) in cases {
        assert_eq!(outcome(|| parse(&g, input)), *expected, "input {input}");
    }
}

/// Which savegames `GameState.Load` accepts (where .NET threw, loading fails).
#[test]
fn game_state_load_matches_dotnet() {
    let ty = |s: &str| -> String {
        match GameState::load(s) {
            Ok(st) => format!("ok:{}:{}", st.version, st.events.as_ref().map(|e| e.len() as i64).unwrap_or(-1)),
            Err(_) => "exception".to_string(),
        }
    };
    let pad = " ".repeat(130);
    let cases: Vec<(String, &str)> = vec![
        ("null".into(), "exception"),
        ("{}".into(), "ok:0:0"),
        (r#"{"_events":null}"#.into(), "ok:0:-1"),
        (r#"{"_events":[{}]}"#.into(), "exception"),
        (r#"{"_events":[{"$type":"Nope"}]}"#.into(), "exception"),
        (
            format!(r#"{{"$type":"Treachery.Shared.GameState, Treachery.Shared","_events":{{"$type":"x","$values":[{{"$type":"Treachery.Shared.Nope, Treachery.Shared"}}]}}}}{pad}"#),
            "exception",
        ),
        (format!(r#"{{"$values":[],"x":"Treachery.Shared.GameState"}}{pad}"#), "ok:0:0"),
        (format!(r#"{{"$type":"Treachery.Shared.GameState","$values":[]}}{pad}"#), "ok:0:0"),
        (r#"{"$values":[],"Version":3}"#.into(), "ok:3:0"),
        (r#"{"$id":"1","Version":3}"#.into(), "ok:3:0"),
    ];
    let failures: Vec<String> = cases
        .iter()
        .filter(|(input, expected)| ty(input) != *expected)
        .map(|(input, expected)| format!("input {input}: rust {} vs .NET {expected}", ty(input)))
        .collect();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn utilities_formatting_matches_dotnet() {
    assert_eq!(Utilities::px(f64::NAN), "NaNpx");
    assert_eq!(Utilities::px(1e20), "1E+20px");
    assert_eq!(Utilities::scale_font("Bold 1,000PX Arial 1.5px", 1.1), " bold 1100px arial 1.6500000953674316px");
}

/// .NET rejects `$`-prefixed members only in polymorphic types (events). In any
/// other object they are unknown members and are skipped (probed: `$foo`, `$id`,
/// `$ref`, `$type`, `$values` all accepted in GameState and in the GameSettings
/// nested in EstablishPlayers). `json::read_object` rejects every `$` member.
#[test]
fn dollar_members_in_non_polymorphic_objects_are_ignored() {
    use treachery_engine::events::GameEvent;
    let r = Utilities::deserialize::<GameEvent>(r#"{"$type":"EstablishPlayers","Settings":{"$foo":1,"MaximumTurns":4}}"#)
        .map(|e| match e.unwrap() {
            GameEvent::EstablishPlayers(ep) => ep.settings.as_ref().unwrap().borrow().maximum_turns,
            _ => -1,
        })
        .map_err(|e| format!("{}: {}", e.kind, e.message));
    // .NET: 4
    assert_eq!(r, Ok(4));

    for (input, version) in [(r#"{"$foo":1,"Version":3}"#, 3), (r#"{"$ref":"1","Version":3}"#, 3), (r#"{"$type":"x","Version":3}"#, 3)] {
        let r = GameState::load(input).map(|s| s.version).map_err(|e| format!("{}: {}", e.kind, e.message));
        assert_eq!(r, Ok(version), "input {input}");
    }

    // Events are polymorphic: .NET rejects these with JsonException too.
    for input in [r#"{"$type":"EndPhase","$foo":1,"Initiator":3}"#, r#"{"$type":"EndPhase","$id":"1","Initiator":3}"#] {
        let r = Utilities::deserialize::<GameEvent>(input).map(|e| e.is_some()).map_err(|e| e.kind.to_string());
        assert_eq!(r, Err("JsonException".to_string()), "input {input}");
    }
}
