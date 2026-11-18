//! Edge cases of storm, spice blow and charity.

use treachery_engine::error::catch_panic;
use treachery_engine::prelude::*;

fn game_with(factions: &[Faction]) -> Game {
    let mut g = Game::new();
    for f in factions {
        let p = Player::new(Some(*f));
        g.players.push(p);
    }
    g
}

/// A Fremen player whose `Ally` names a faction with no player, storm in sector 5.
fn fremen_with_dangling_ally() -> (Game, PlayerId) {
    let mut g = game_with(&[Faction::Fremen]);
    let y = g.get_player(Some(Faction::Fremen)).unwrap();
    g.player_mut(y).ally = Some(Faction::Atreides);
    g.board.sector_in_storm = Some(Sector::new(5));
    (g, y)
}

/// `FremenRidesMonster::valid_targets` is lazy: looking for the Polar Sink, which comes first,
/// stops before the dangling ally of a later location is read.
#[test]
fn rides_valid_targets_contains_is_lazy() {
    let (g, y) = fremen_with_dangling_ally();
    let polar_sink = g.board.map.polar_sink();
    let r = catch_panic(|| FremenRidesMonster::valid_targets(&g, y).any(|v| v == polar_sink));
    assert_eq!(r, Ok(true));
}

/// Guard: looking for a location after the first one the predicate rejects reads the dangling
/// ally, which panics.
#[test]
fn rides_valid_targets_later_location_panics() {
    let (g, y) = fremen_with_dangling_ally();
    let arrakeen = g.board.map.arrakeen();
    let r = catch_panic(|| FremenRidesMonster::valid_targets(&g, y).any(|v| v == arrakeen));
    assert!(r.is_err());
}

/// Guard: without the dangling ally, 82 targets.
#[test]
fn rides_valid_targets_count_without_ally() {
    let (mut g, y) = fremen_with_dangling_ally();
    g.player_mut(y).ally = None;
    assert_eq!(FremenRidesMonster::valid_targets(&g, y).count(), 82);
}
