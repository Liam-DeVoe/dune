//! Pathfinding works from the game as it is at the moment of the call. The map used to cache
//! `find_neighbours` until the history grew, so a change within one event, or by an event that
//! failed halfway, could give an answer for the state before it (escapes.md, "Paths found for
//! an earlier state"). These tests change the state between two calls with the same arguments.

use std::cell::{Cell, RefCell};

use treachery_engine::enumerations::*;
use treachery_engine::map::location::Location;
use treachery_engine::map::territory::Territory;
use treachery_engine::map::{Map, MapGame};
use treachery_engine::prelude::*;

#[derive(Default)]
struct StubGame {
    sector_in_storm: Cell<i32>,
    forces: RefCell<Vec<(Location, Vec<Faction>)>>,
    blocked: RefCell<Vec<Territory>>,
}

impl MapGame for StubGame {
    fn sector_in_storm(&self) -> i32 {
        self.sector_in_storm.get()
    }
    fn applicable(&self, _rule: Rule) -> bool {
        false
    }
    fn forces_battalion_factions(&self) -> Vec<(Location, Vec<Faction>)> {
        self.forces.borrow().clone()
    }
    fn player_ally(&self, _f: Faction) -> Option<Faction> {
        None
    }
    fn nr_of_occupants_excluding_faction(&self, l: Location, f: Option<Faction>) -> i32 {
        self.forces.borrow().iter().filter(|(at, _)| *at == l).flat_map(|(_, fs)| fs.iter()).filter(|x| Some(**x) != f).count() as i32
    }
    fn current_blocked_territories(&self) -> Vec<Territory> {
        self.blocked.borrow().clone()
    }
}

fn reachable(m: &Map, g: &StubGame, from: Location) -> Vec<Location> {
    m.find_neighbours(from, 2, false, Faction::Atreides, g, true)
}

#[test]
fn the_storm_moving_changes_the_paths_found() {
    let m = Map::new();
    let g = StubGame { sector_in_storm: Cell::new(-1), ..Default::default() };
    let before = reachable(&m, &g, m.arrakeen());
    let sector = before.iter().map(|l| l.sector(&m)).find(|s| *s != m.arrakeen().sector(&m)).expect("a sector to put the storm on");
    g.sector_in_storm.set(sector);
    let after = reachable(&m, &g, m.arrakeen());
    assert!(after.iter().all(|l| l.sector(&m) != sector), "a location in the storm is reachable");
    assert_ne!(before, after);
}

#[test]
fn forces_arriving_change_the_paths_found() {
    let m = Map::new();
    let g = StubGame { sector_in_storm: Cell::new(-1), ..Default::default() };
    assert!(reachable(&m, &g, m.arrakeen()).contains(&m.carthag()));
    // Two other factions in Carthag make it an obstacle for Atreides.
    *g.forces.borrow_mut() = vec![(m.carthag(), vec![Faction::Harkonnen, Faction::Emperor])];
    assert!(!reachable(&m, &g, m.arrakeen()).contains(&m.carthag()));
}

#[test]
fn a_territory_becoming_blocked_changes_the_paths_found() {
    let m = Map::new();
    let g = StubGame { sector_in_storm: Cell::new(-1), ..Default::default() };
    assert!(reachable(&m, &g, m.arrakeen()).contains(&m.carthag()));
    *g.blocked.borrow_mut() = vec![m.carthag().territory()];
    assert!(!reachable(&m, &g, m.arrakeen()).contains(&m.carthag()));
}

/// The same through a whole game: the move fixture, with the storm moved onto a sector a
/// player's forces could reach, without an event in between.
#[test]
fn a_game_finds_move_targets_for_its_current_storm() {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scripts/fixtures/move.json");
    let state = treachery_engine::game::GameState::load(&std::fs::read_to_string(path).unwrap()).expect("savegame");
    let mut g = Game::of_version(state.version);
    for e in state.events() {
        e.initialize_game(&g);
        assert!(e.execute(&mut g, false, false).is_none());
    }

    let (p, from, battalion) = g
        .player_indices()
        .filter(|p| g.player(*p).faction != Some(Faction::Fremen))
        .flat_map(|p| g.player(p).forces_on_planet().into_iter().map(move |(l, b)| (p, l, b)))
        .find(|(p, l, b)| !PlacementEvent::valid_targets(&g, *p, *l, b).is_empty())
        .expect("forces that can move");
    let before = PlacementEvent::valid_targets(&g, p, from, &battalion);
    let sector = before
        .iter()
        .map(|l| l.sector(&g.board.map))
        .find(|s| *s >= 0 && *s != from.sector(&g.board.map))
        .expect("a sector to put the storm on");
    g.board.sector_in_storm = sector;
    let after = PlacementEvent::valid_targets(&g, p, from, &battalion);
    assert!(after.iter().all(|l| l.sector(&g.board.map) != sector), "a target in the storm");
    assert_ne!(before, after);
}
