//! Tests for the package D review (rust/docs/reviews/pkg-d.md). The expected values
//! come from the "review-d" mode of the C# reference program:
//!
//!   cd rust/engine/csharp-ref && dotnet run -c Release -- review-d

use treachery_engine::compat::cs;
use treachery_engine::prelude::*;
use treachery_engine::compat::lazy::LazyQuery;

fn game_with(factions: &[Faction]) -> Game {
    let mut g = Game::new();
    for f in factions {
        let p = Player::new(&g, *f);
        g.players.push(p);
    }
    g
}

/// A Fremen player whose `Ally` names a faction with no player, storm in sector 5.
fn fremen_with_dangling_ally() -> (Game, PlayerIndex) {
    let mut g = game_with(&[Faction::Fremen]);
    let y = g.get_player(Some(Faction::Fremen)).unwrap();
    g.player_mut(y).ally = Faction::Atreides;
    g.sector_in_storm = 5;
    (g, y)
}

/// C#: `FremenRidesMonster.ValidTargets(g, p).Contains(Map.PolarSink)` is `True`
/// (review-d: `ridesValidTargets.containsPolarSink = True`). The lazy `Where` stops at
/// the Polar Sink, which comes first and short-circuits before `p.AlliedPlayer.Occupies`.
/// Rust collects every target first, so the null `AlliedPlayer` of a later location throws.
#[test]
fn rides_valid_targets_contains_is_lazy() {
    let (g, y) = fremen_with_dangling_ally();
    let polar_sink = g.map.polar_sink();
    let r = cs::catch_panic(|| FremenRidesMonster::valid_targets(&g, y).cs_contains(&polar_sink));
    assert_eq!(r.map_err(|e| format!("{}:{}", e.type_name, e.message)), Ok(true));
}

/// Guard: a location after the first one the predicate rejects throws in both engines
/// (review-d: `ridesValidTargets.containsArrakeen = exception:NullReferenceException`).
#[test]
fn rides_valid_targets_later_location_throws_in_both() {
    let (g, y) = fremen_with_dangling_ally();
    let arrakeen = g.map.arrakeen();
    let r = cs::catch_panic(|| FremenRidesMonster::valid_targets(&g, y).cs_contains(&arrakeen));
    assert_eq!(r.err().map(|e| e.type_name), Some("NullReferenceException"));
}

/// Guard: without the dangling ally, 82 targets (review-d: `ridesValidTargets.noAlly.count = 82`).
#[test]
fn rides_valid_targets_count_without_ally() {
    let (mut g, y) = fremen_with_dangling_ally();
    g.player_mut(y).ally = Faction::None;
    assert_eq!(FremenRidesMonster::valid_targets(&g, y).count(), 82);
}
