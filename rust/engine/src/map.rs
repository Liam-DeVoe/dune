//! Port of Map.cs.
//!
//! Every C# `Game` creates its own `Map`. Most of it never changes: the static
//! tables in `map_data.rs` hold the territories and locations, and
//! [`Location`]/[`Territory`] are handles into them. The parts that do change
//! during a game are stored here:
//!
//! - the `Neighbours` list of every location (`AttachedLocation.PointAt` adds
//!   and removes the attached location from its host's list),
//! - `AttachedToLocation` and `Game` of the six attached locations,
//! - the `FindNeighbours` cache (keyed on `game.History.Count`; it can return a
//!   stale result when the storm or forces change within one event, and that
//!   is reproduced).
//!
//! C# `Game.Clone()` copies the `Map` reference, so a clone shares its map with
//! the original. Only the C# tests call `Game.Clone()`; deriving `Clone` here
//! gives an independent copy instead.

use std::cell::{Cell, RefCell};

use crate::cs::{index, CsIterExt};
use crate::discovery_stronghold::DiscoveredLocation;
use crate::enumerations::*;
use crate::homeworld::Homeworld;
use crate::i_fetcher::IFetcher;
use crate::location::Location;
use crate::map_data::{INITIAL_NEIGHBOURS, LOCATION_COUNT};
use crate::resource_card::ResourceCard;
use crate::territory::Territory;
use crate::throw;

/// The static properties of a territory (`new Territory(id) { ... }` in
/// `InitializeLocations`).
#[derive(Debug)]
pub struct TerritoryData {
    pub id: i32,
    pub is_homeworld: bool,
    pub has_reduced_shipping_cost: bool,
    pub is_discovery: bool,
    pub is_stronghold: bool,
    pub is_protected_from_storm: bool,
    pub is_protected_from_worm: bool,
    pub advantage: StrongholdAdvantage,
    /// `Territory.Locations` is the run `first_location..first_location+location_count` of `_locations`.
    pub first_location: u8,
    pub location_count: u8,
}

/// The static properties of a location. `sector` is the C# `Sector` for
/// normal locations and homeworlds (-1); attached locations compute theirs.
#[derive(Debug)]
pub struct LocationData {
    pub id: i32,
    pub territory: Territory,
    pub orientation: &'static str,
    pub sector: i32,
    pub spice_blow_amount: i32,
    pub discovery_token_type: DiscoveryTokenType,
    pub kind: LocationKind,
}

/// The C# runtime type of a location.
#[derive(Debug)]
pub enum LocationKind {
    Normal,
    Homeworld(HomeworldData),
    Discovered { discovery: DiscoveryToken },
    HiddenMobileStronghold,
}

/// The fields of `Homeworld`.
#[derive(Debug)]
pub struct HomeworldData {
    pub world: World,
    pub faction: Faction,
    pub is_home_of_normal_forces: bool,
    pub is_home_of_special_forces: bool,
    pub threshold: i32,
    pub battle_bonus_and_lasgun_shield_limit_at_high_threshold: i32,
    pub battle_bonus_and_lasgun_shield_limit_at_low_threshold: i32,
    pub resource_amount: i32,
}

/// What `Map` reads from `Game`. `Game` implements this; tests use a stub.
///
/// Map code in C# takes a `Game` and calls into it. Rust cannot pass `&Game`
/// while `game.map` is borrowed through the same `Game`, and `Map` must not
/// depend on the whole `Game` type, so the calls it makes are listed here.
pub trait MapGame {
    /// `game.Version`
    fn version(&self) -> i32;
    /// `game.SectorInStorm`
    fn sector_in_storm(&self) -> i32;
    /// `game.History.Count`
    fn history_count(&self) -> i32;
    /// `game.Applicable(rule)`
    fn applicable(&self, rule: Rule) -> bool;
    /// `game.Forces()` (homeworlds excluded): every location with the
    /// `Faction` of each of its battalions, in dictionary enumeration order.
    fn forces_battalion_factions(&self) -> Vec<(Location, Vec<Faction>)>;
    /// `game.GetPlayer(f)?.Ally`
    fn player_ally(&self, f: Faction) -> Option<Faction>;
    /// `game.NrOfOccupantsExcludingFaction(l, f)`
    fn nr_of_occupants_excluding_faction(&self, l: Location, f: Faction) -> i32;
    /// `game.CurrentBlockedTerritories`
    fn current_blocked_territories(&self) -> Vec<Territory>;
}

/// `Map.NeighbourCacheKey`. Equality as in its `IEquatable` implementation
/// (which is what `Dictionary` uses).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NeighbourCacheKey {
    pub start: Location,
    pub distance: i32,
    pub faction: Faction,
    pub ignore_storm: bool,
}

#[derive(Clone, Debug)]
pub struct Map {
    /// `Neighbours` of every location, by index. For attached locations this
    /// is the one-element list `[AttachedToLocation]` (or empty), matching the
    /// list the C# getter builds.
    neighbours: Vec<Vec<Location>>,
    /// `AttachedLocation.AttachedToLocation`, by location index.
    attached_to: Vec<Option<Location>>,
    /// `AttachedLocation.Game?.Version`, by location index. Set by `PointAt`
    /// only when the game version is >= 163.
    attached_game_version: Vec<Option<i32>>,
    /// `_neighbourCacheTimestamp`
    neighbour_cache_timestamp: Cell<i32>,
    /// `_neighbourCache`. A `Dictionary` in C#; it is only cleared, added to
    /// and looked up, so a list in insertion order has the same contents and
    /// enumeration order.
    neighbour_cache: RefCell<Vec<(NeighbourCacheKey, Vec<Location>)>>,
}

impl Default for Map {
    fn default() -> Self {
        Map::new()
    }
}

impl Map {
    pub const NUMBER_OF_SECTORS: i32 = 18;

    pub fn new() -> Map {
        crate::cs::install_null_key_check();
        let mut map = Map {
            neighbours: vec![Vec::new(); LOCATION_COUNT],
            attached_to: vec![None; LOCATION_COUNT],
            attached_game_version: vec![None; LOCATION_COUNT],
            neighbour_cache_timestamp: Cell::new(-1),
            neighbour_cache: RefCell::new(Vec::new()),
        };
        map.initialize_location_neighbours();
        map
    }

    fn initialize_location_neighbours(&mut self) {
        for &(from, to) in INITIAL_NEIGHBOURS.iter() {
            let from = Location::from_index(from as usize);
            // Adding to an attached location's Neighbours adds to a temporary
            // list in C#, so it has no effect. (The table has no such entry.)
            if from.as_attached_location().is_none() {
                self.neighbours[from.index()].push(Location::from_index(to as usize));
            }
        }
    }

    // ---- state of locations, used by Location/AttachedLocation ----

    pub(crate) fn neighbours_of(&self, l: Location) -> &[Location] {
        &self.neighbours[l.index()]
    }

    pub(crate) fn attached_to_location(&self, l: Location) -> Option<Location> {
        self.attached_to[l.index()]
    }

    /// `AttachedLocation.Sector => Game?.Version >= 159 && AttachedToLocation != null ? AttachedToLocation.Sector : -1`
    pub(crate) fn attached_sector(&self, l: Location) -> i32 {
        match (self.attached_game_version[l.index()], self.attached_to[l.index()]) {
            (Some(v), Some(host)) if v >= 159 => host.sector(self),
            _ => -1,
        }
    }

    /// `AttachedLocation.PointAt`
    pub(crate) fn point_at(&mut self, attached: Location, game_version: i32, new_location: Location) {
        // AttachedToLocation?.Neighbours.Remove(this): List.Remove removes the
        // first element that Equals `this`. If the host is itself attached its
        // Neighbours getter returns a temporary list, so nothing happens.
        if let Some(old) = self.attached_to[attached.index()] {
            if old.as_attached_location().is_none() {
                let list = &mut self.neighbours[old.index()];
                if let Some(pos) = list.iter().position(|l| *l == attached) {
                    list.remove(pos);
                }
            }
        }

        if new_location.as_attached_location().is_none() {
            self.neighbours[new_location.index()].push(attached);
        }
        self.attached_to[attached.index()] = Some(new_location);
        self.neighbours[attached.index()] = vec![new_location];

        if game_version >= 163 {
            self.attached_game_version[attached.index()] = Some(game_version);
        }
    }

    // ---- queries ----

    pub fn get_discovery_stronghold(&self, discovery: DiscoveryToken) -> DiscoveredLocation {
        match self
            .locations(false)
            .find_map(|l| l.as_discovered_location().filter(|ds| ds.discovery() == discovery))
        {
            Some(ds) => ds,
            None => throw!(Exception, "Discovery stronghold for {} not found", discovery),
        }
    }

    /// `Locations(includeHomeworlds)`
    pub fn locations(&self, include_homeworlds: bool) -> impl Iterator<Item = Location> + Clone + 'static {
        Location::all()
            .iter()
            .copied()
            .filter(move |l| include_homeworlds || l.as_homeworld().is_none())
    }

    /// `Homeworlds`
    pub fn homeworlds(&self) -> impl Iterator<Item = Homeworld> + Clone + 'static {
        Location::all().iter().filter_map(|l| l.as_homeworld())
    }

    /// `Territories(includeHomeworlds)`: `Locations(..).Select(l => l.Territory).Distinct()`
    pub fn territories(&self, include_homeworlds: bool) -> impl Iterator<Item = Territory> + Clone + 'static {
        Self::territories_static(include_homeworlds)
    }

    /// `Territories` does not read map state, so code without a map can use this.
    pub fn territories_static(include_homeworlds: bool) -> impl Iterator<Item = Territory> + Clone + 'static {
        let mut last: Option<Territory> = None;
        // Locations of a territory are contiguous, so dropping repeats of the
        // previous element is the same as Distinct (checked by the golden test).
        Location::all()
            .iter()
            .copied()
            .filter(move |l| include_homeworlds || l.as_homeworld().is_none())
            .map(|l| l.territory())
            .filter(move |t| {
                let new = last != Some(*t);
                last = Some(*t);
                new
            })
    }

    /// `Strongholds => _locations.Where(l => l.Territory.IsStronghold)`
    pub fn strongholds(&self) -> impl Iterator<Item = Location> + Clone + 'static {
        Location::all().iter().copied().filter(|l| l.territory().is_stronghold())
    }

    /// `GetHomeWorld(w)`
    pub fn get_home_world(&self, w: World) -> Option<Homeworld> {
        self.homeworlds().find(|hw| hw.world() == w)
    }

    /// `Map.GetResourceCardsInPlay(g)`. C# creates new card objects on every
    /// call; here the same handles come back each time (see `resource_card.rs`).
    /// Lazy like the C# query: callers stop early (docs/lazy-queries.md).
    pub fn get_resource_cards_in_play(g: &impl MapGame) -> impl Iterator<Item = ResourceCard> {
        ResourceCard::get_resource_cards_in_and_outside_play()
            .iter()
            .copied()
            .filter(move |c| Self::is_in_play(g, *c))
    }

    fn is_in_play(g: &impl MapGame, c: ResourceCard) -> bool {
        c.is_shai_hulud()
            || (c.is_spice_blow() && !c.is_discovery())
            || (c.is_sand_trout() && g.applicable(Rule::SandTrout))
            || (c.is_great_maker() && g.applicable(Rule::GreatMaker))
            || (c.is_discovery() && g.applicable(Rule::DiscoveryTokens))
    }

    /// `Map.LocationLookup`
    pub fn location_lookup(&self) -> LocationFetcher {
        LocationFetcher
    }

    /// `Map.TerritoryLookup`
    pub fn territory_lookup(&self) -> TerritoryFetcher {
        TerritoryFetcher
    }

    // ---- pathfinding ----

    /// `FindNeighbours(start, distance, ignoreStorm, f, game, checkForceObstacles)`.
    /// C# returns the cached list object itself; callers only read it, so a
    /// copy is returned.
    pub fn find_neighbours(
        &self,
        start: Location,
        distance: i32,
        ignore_storm: bool,
        f: Faction,
        game: &impl MapGame,
        check_force_obstacles: bool,
    ) -> Vec<Location> {
        let cache_key = NeighbourCacheKey { start, distance, faction: f, ignore_storm };

        if check_force_obstacles {
            if self.neighbour_cache_timestamp.get() != game.history_count() {
                self.neighbour_cache.borrow_mut().clear();
                self.neighbour_cache_timestamp.set(game.history_count());
            } else if let Some((_, value)) = self.neighbour_cache.borrow().iter().find(|(k, _)| *k == cache_key) {
                return value.clone();
            }
        }

        let mut force_obstacles = Vec::new();
        if check_force_obstacles {
            force_obstacles = Self::determine_force_obstacles(f, game);
        }

        let mut neighbours = Vec::new();
        self.find_neighbours_rec(
            &mut neighbours,
            start,
            None,
            0,
            distance,
            if ignore_storm { 99 } else { game.sector_in_storm() },
            &force_obstacles,
        );

        if let Some(pos) = neighbours.iter().position(|l| *l == start) {
            neighbours.remove(pos);
        }

        if check_force_obstacles {
            self.neighbour_cache.borrow_mut().push((cache_key, neighbours.clone()));
        }

        neighbours
    }

    fn belongs_to(game: &impl MapGame, battalion_faction: Faction, f: Faction) -> bool {
        battalion_faction == f || (game.player_ally(f) == Some(Faction::Ecaz) && battalion_faction == Faction::Ecaz)
    }

    fn determine_force_obstacles(f: Faction, game: &impl MapGame) -> Vec<Location> {
        // Forces().Where(...).Select(kvp => kvp.Key).Distinct()
        //   .Union(CurrentBlockedTerritories.SelectMany(t => t.Locations)).ToList()
        let first = game
            .forces_battalion_factions()
            .into_iter()
            .filter(|(l, battalions)| {
                l.is_stronghold()
                    && !battalions.iter().any(|b| Self::belongs_to(game, *b, f))
                    && game.nr_of_occupants_excluding_faction(*l, f) >= 2
            })
            .map(|(l, _)| l)
            .cs_distinct();
        first
            .into_iter()
            .chain(game.current_blocked_territories().into_iter().flat_map(|t| t.locations().iter().copied()))
            .cs_distinct()
    }

    #[allow(clippy::too_many_arguments)]
    fn find_neighbours_rec(
        &self,
        found: &mut Vec<Location>,
        current: Location,
        previous: Option<Location>,
        current_distance: i32,
        max_distance: i32,
        sector_in_storm: i32,
        force_obstacles: &[Location],
    ) {
        let previous = previous.and_then(crate::cs::CsNull::opt);
        if !found.contains(&current) {
            found.push(current);
        }

        for &neighbour in current.neighbours(self).iter().filter(|n| {
            Some(n.id()) != previous.map(|p| p.id())
                && n.sector(self) != sector_in_storm
                && !force_obstacles.contains(n)
        }) {
            let distance = if current.territory() == neighbour.territory() { 0 } else { 1 };

            if current_distance + distance <= max_distance {
                self.find_neighbours_rec(
                    found,
                    neighbour,
                    Some(current),
                    current_distance + distance,
                    max_distance,
                    sector_in_storm,
                    force_obstacles,
                );
            }
        }
    }

    /// `Map.FindPaths` (static in C#; the neighbour lists come from this map).
    /// Each path is listed from `destination` back to `start` (C# `Stack.ToList()`).
    pub fn find_paths(
        &self,
        start: Location,
        destination: Location,
        distance: i32,
        ignore_storm: bool,
        f: Faction,
        game: &impl MapGame,
    ) -> Vec<Vec<Location>> {
        let mut paths = Vec::new();
        let mut route = Vec::new();
        let obstacles = Self::determine_force_obstacles(f, game);
        self.find_paths_rec(
            &mut paths,
            &mut route,
            start,
            destination,
            None,
            0,
            distance,
            if ignore_storm { 99 } else { game.sector_in_storm() },
            &obstacles,
        );
        paths
    }

    /// `current_path` is the C# `Stack`, bottom first.
    #[allow(clippy::too_many_arguments)]
    fn find_paths_rec(
        &self,
        found_paths: &mut Vec<Vec<Location>>,
        current_path: &mut Vec<Location>,
        current: Location,
        destination: Location,
        previous: Option<Location>,
        current_distance: i32,
        max_distance: i32,
        sector_in_storm: i32,
        obstacles: &[Location],
    ) {
        let previous = previous.and_then(crate::cs::CsNull::opt);
        current_path.push(current);

        if current == destination {
            found_paths.push(current_path.iter().rev().copied().collect());
        } else {
            for &neighbour in current.neighbours(self).iter() {
                // The Where is evaluated lazily, so currentPath is checked as it is now.
                if !(Some(neighbour.id()) != previous.map(|p| p.id())
                    && neighbour.sector(self) != sector_in_storm
                    && !current_path.contains(&neighbour)
                    && !obstacles.contains(&neighbour))
                {
                    continue;
                }
                let distance = if current.territory() == neighbour.territory() { 0 } else { 1 };
                if current_distance + distance <= max_distance {
                    self.find_paths_rec(
                        found_paths,
                        current_path,
                        neighbour,
                        destination,
                        Some(current),
                        current_distance + distance,
                        max_distance,
                        sector_in_storm,
                        obstacles,
                    );
                }
            }
        }

        current_path.pop();
    }

    /// `Map.FindFirstShortestPath`; the path runs from `start` to `destination`.
    pub fn find_first_shortest_path(
        &self,
        start: Location,
        destination: Location,
        ignore_storm: bool,
        f: Faction,
        game: &impl MapGame,
    ) -> Option<Vec<Location>> {
        let mut route = Vec::new();
        let obstacles = Self::determine_force_obstacles(f, game);
        for i in 0..=4 {
            let path = self.find_path(
                &mut route,
                start,
                destination,
                None,
                0,
                i,
                if ignore_storm { 99 } else { game.sector_in_storm() },
                &obstacles,
            );
            if path.is_some() {
                return path;
            }
        }
        None
    }

    #[allow(clippy::too_many_arguments)]
    fn find_path(
        &self,
        current_route: &mut Vec<Location>,
        current: Location,
        destination: Location,
        previous: Option<Location>,
        current_distance: i32,
        max_distance: i32,
        sector_in_storm: i32,
        obstacles: &[Location],
    ) -> Option<Vec<Location>> {
        let previous = previous.and_then(crate::cs::CsNull::opt);
        current_route.push(current);

        if current == destination {
            // Stack.Reverse() enumerates bottom first.
            return Some(current_route.clone());
        }

        for &neighbour in current.neighbours(self).iter() {
            if !(Some(neighbour.id()) != previous.map(|p| p.id())
                && neighbour.sector(self) != sector_in_storm
                && !current_route.contains(&neighbour)
                && !obstacles.contains(&neighbour))
            {
                continue;
            }
            let distance = if current.territory() == neighbour.territory() { 0 } else { 1 };

            if current_distance + distance > max_distance {
                continue;
            }

            let found = self.find_path(
                current_route,
                neighbour,
                destination,
                Some(current),
                current_distance + distance,
                max_distance,
                sector_in_storm,
                obstacles,
            );
            if found.is_some() {
                return found;
            }
        }

        current_route.pop();
        None
    }

    /// `Map.FindNeighboursForHmsMovement`
    pub fn find_neighbours_for_hms_movement(
        &self,
        start: Location,
        distance: i32,
        ignore_storm: bool,
        sector_in_storm: i32,
    ) -> Vec<Location> {
        let mut neighbours = Vec::new();
        self.find_neighbours_for_hms_movement_rec(&mut neighbours, start, None, 0, distance, ignore_storm, sector_in_storm);
        if let Some(pos) = neighbours.iter().position(|l| *l == start) {
            neighbours.remove(pos);
        }
        neighbours
    }

    #[allow(clippy::too_many_arguments)]
    fn find_neighbours_for_hms_movement_rec(
        &self,
        found: &mut Vec<Location>,
        current: Location,
        previous: Option<Location>,
        current_distance: i32,
        max_distance: i32,
        ignore_storm: bool,
        sector_in_storm: i32,
    ) {
        let previous = previous.and_then(crate::cs::CsNull::opt);
        if !found.contains(&current) {
            found.push(current);
        }

        for &neighbour in current
            .neighbours(self)
            .iter()
            .filter(|n| Some(n.id()) != previous.map(|p| p.id()) && (ignore_storm || n.sector(self) != sector_in_storm))
        {
            let distance = if current.territory() == neighbour.territory() { 0 } else { 1 };

            if current_distance + distance <= max_distance {
                self.find_neighbours_for_hms_movement_rec(
                    found,
                    neighbour,
                    Some(current),
                    current_distance + distance,
                    max_distance,
                    ignore_storm,
                    sector_in_storm,
                );
            }
        }
    }

    /// `Map.FindNeighboursWithinTerritory`
    pub fn find_neighbours_within_territory(&self, start: Location, ignore_storm: bool, sector_in_storm: i32) -> Vec<Location> {
        let mut neighbours = Vec::new();
        self.find_neighbours_within_territory_rec(&mut neighbours, start, None, ignore_storm, sector_in_storm);
        neighbours
    }

    fn find_neighbours_within_territory_rec(
        &self,
        found: &mut Vec<Location>,
        current: Location,
        previous: Option<Location>,
        ignore_storm: bool,
        sector_in_storm: i32,
    ) {
        let previous = previous.and_then(crate::cs::CsNull::opt);
        if !found.contains(&current) {
            found.push(current);
        }

        for &neighbour in current.neighbours(self).iter().filter(|l| {
            l.territory() == current.territory()
                && Some(l.id()) != previous.map(|p| p.id())
                && (ignore_storm || l.sector(self) != sector_in_storm)
        }) {
            self.find_neighbours_within_territory_rec(found, neighbour, Some(current), ignore_storm, sector_in_storm);
        }
    }

    /// Read access to the neighbour cache, for state dumps.
    /// Sets `_neighbourCacheTimestamp` to -1, so the next `FindNeighbours`
    /// clears the cache. The replayer calls this before every event, as the
    /// oracle does (trace-format.md section 6).
    pub fn mark_neighbour_cache_stale(&self) {
        self.neighbour_cache_timestamp.set(-1);
    }

    pub fn neighbour_cache(&self) -> (i32, Vec<(NeighbourCacheKey, Vec<Location>)>) {
        (self.neighbour_cache_timestamp.get(), self.neighbour_cache.borrow().clone())
    }

    /// `AttachedLocation.Game?.Version` of an attached location, for state dumps.
    pub fn attached_game_version(&self, l: Location) -> Option<i32> {
        self.attached_game_version[l.index()]
    }
}

/// `Map.TerritoryFetcher`
#[derive(Clone, Copy, Debug)]
pub struct TerritoryFetcher;

impl IFetcher<Territory> for TerritoryFetcher {
    /// `id == -1 ? null : map.Territories(true).SingleOrDefault(t => t.Id == id)`
    fn find(&self, id: i32) -> Option<Territory> {
        if id == -1 {
            return None;
        }
        Map::territories_static(true).cs_single_or_default_where(|t| t.id() == id)
    }

    fn get_id(&self, obj: Option<Territory>) -> i32 {
        let obj = obj.and_then(crate::cs::CsNull::opt);
        obj.and_then(crate::cs::CsNull::opt).map_or(-1, |t| t.id())
    }
}

/// `Map.LocationFetcher`
#[derive(Clone, Copy, Debug)]
pub struct LocationFetcher;

impl IFetcher<Location> for LocationFetcher {
    /// `id == -1 ? null : map._locations[id]` (ArgumentOutOfRangeException when out of range)
    fn find(&self, id: i32) -> Option<Location> {
        if id == -1 {
            return None;
        }
        Some(*index(Location::all(), id))
    }

    fn get_id(&self, obj: Option<Location>) -> i32 {
        let obj = obj.and_then(crate::cs::CsNull::opt);
        obj.and_then(crate::cs::CsNull::opt).map_or(-1, |l| l.id())
    }
}
