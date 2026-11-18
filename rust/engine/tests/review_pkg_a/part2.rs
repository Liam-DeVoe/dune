//! Part 2 of review_pkg_a.rs (included from it): parsing force lists.

use treachery_engine::error::catch_panic;
use treachery_engine::enumerations::*;
use treachery_engine::events::PlacementEvent;
use treachery_engine::game::Game;

fn outcome<R: ToString>(f: impl FnOnce() -> R) -> String {
    match catch_panic(f) {
        Ok(r) => r.to_string(),
        Err(e) => format!("exception:{}", e.replacen(": ", ":", 1)),
    }
}

fn parse(g: &Game, s: &str) -> String {
    PlacementEvent::parse_force_locations(g, Faction::Emperor, s)
        .iter()
        .map(|(l, b)| format!("{}={}/{}", l.id(), b.amount_of_forces, b.amount_of_special_forces))
        .collect::<Vec<_>>()
        .join(";")
}

/// Duplicate locations of each kind (Emperor homeworld 90, Shrine 103, HMS 86), signs, overflow
/// and negative ids.
#[test]
fn parse_force_locations_edge_cases() {
    let g = Game::new();
    let cases: &[(&str, &str)] = &[
        ("90:1|0,90:2|0", "exception:duplicate key Location(90)"),
        (
            "103:1|0,103:2|0",
            "exception:duplicate key Location(103)",
        ),
        (
            "86:1|0,86:2|0",
            "exception:duplicate key Location(86)",
        ),
        ("5:1|2,", "exception:FormatException:The input string '' was not in a correct format."),
        ("+5:+1|-0", "5=1/0"),
        ("5:2147483648|0", "exception:OverflowException:Value was either too large or too small for an Int32."),
        (
            "-2:1|0",
            "exception:index out of bounds:the len is 105 but the index is 18446744073709551614",
        ),
    ];
    for (input, expected) in cases {
        assert_eq!(outcome(|| parse(&g, input)), *expected, "input {input}");
    }
}
