use crate::prelude::*;

/// What is on the board besides the players' forces: the map, the storm, spice, terror tokens,
/// ambassadors, discoveries, stronghold ownership and homeworld occupation.
pub struct Board {
    pub map: Map,
    pub sector_in_storm: Option<Sector>,
    pub next_storm_moves: i32,
    pub shield_wall_destroyed: bool,
    // Spice, discoveries and stronghold owners are in map order (by location id), so the board lists
    // them the same way every time. Terror tokens, ambassadors and occupied homeworlds are in the
    // order they were placed or taken, which a sort by token, territory or world wouldn't improve.
    pub resources_on_planet: BTreeMap<Location, u16>,
    pub terror_on_planet: IndexMap<TerrorType, Territory>,
    pub unplaced_terror_tokens: Vec<TerrorType>,
    pub atomics_aftermath: Option<Territory>,
    pub discoveries_on_planet: BTreeMap<Location, Discovery>,
    pub ambassadors_on_planet: IndexMap<Territory, Ambassador>,
    pub homeworld_occupation: IndexMap<Homeworld, Faction>,
    pub stronghold_ownership: BTreeMap<Location, Faction>,
    /// The discovery tokens that may be revealed in this turn's collection phase: those where
    /// forces were when it began. Kept until the next collection phase.
    pub pending_discoveries: Vec<DiscoveryToken>,
    /// Who holds the Flight discovery. `Some(None)`: decided, and nobody.
    pub owner_of_flight_discovery: Option<Option<Faction>>,
    /// Strongholds revealed as discoveries that forces may still enter, until the next storm.
    pub just_revealed_discovery_strongholds: Vec<DiscoveredLocation>,
    /// The factions whose battle wheels dial the storm: first and last at the table, then those
    /// of the latest battle.
    pub has_battle_wheel: Vec<Faction>,
    /// Sectors the Ixian may still move the Hidden Mobile Stronghold with Karama: 2 until they
    /// use it, then 0 from the next battle phase on.
    pub karma_hms_moves_left: i32,
    /// The latest card drawn from spice deck A and from deck B, shown until the next one.
    pub latest_spice_card_a: Option<ResourceCard>,
    pub latest_spice_card_b: Option<ResourceCard>,
    /// The worms of the latest spice blow (of deck A or B), until the next blow or the charity
    /// phase.
    pub monsters: Vec<MonsterAppearance>,
    /// Worms that appeared in the game (for the Shield Wall stronghold rule).
    pub number_of_monsters: i32,
    /// A sandtrout waiting for the next worm, which it cancels.
    pub sand_trout: Option<ResourceCard>,
    /// A sandtrout cancelled a worm, so the next spice blow is doubled.
    pub sand_trout_doubles_resources: bool,
}

impl Board {
    pub fn new() -> Self {
        Board {
            map: Map::new(),
            sector_in_storm: None,
            next_storm_moves: -1,
            shield_wall_destroyed: Default::default(),
            resources_on_planet: Default::default(),
            terror_on_planet: Default::default(),
            unplaced_terror_tokens: Vec::new(),
            atomics_aftermath: None,
            discoveries_on_planet: Default::default(),
            ambassadors_on_planet: Default::default(),
            homeworld_occupation: Default::default(),
            stronghold_ownership: Default::default(),
            pending_discoveries: Vec::new(),
            owner_of_flight_discovery: None,
            just_revealed_discovery_strongholds: Vec::new(),
            has_battle_wheel: Vec::new(),
            karma_hms_moves_left: 2,
            latest_spice_card_a: None,
            latest_spice_card_b: None,
            monsters: Vec::new(),
            number_of_monsters: 0,
            sand_trout: None,
            sand_trout_doubles_resources: false,
        }
    }
}

impl Game {
    pub fn resources_in(&self, l: Location) -> i32 {
        self.board.resources_on_planet.get(&l).copied().map_or(0, i32::from)
    }

    /// Adds `amount` spice (negative to take some) at `location`, and drops the location when
    /// none is left. A Game Error if less than zero would be left.
    pub fn change_resources_on_planet(&mut self, location: Location, amount: i32) -> GameResult {
        let mut current = self.board.resources_on_planet.get(&location).copied().unwrap_or(0);
        crate::game::amounts::change(&mut current, amount, "spice")?;
        if current == 0 {
            self.board.resources_on_planet.remove(&location);
        } else {
            self.board.resources_on_planet.insert(location, current);
        }
        Ok(())
    }

    pub fn remove_resources(&mut self, location: Location) -> i32 {
        let mut result = 0;
        if self.board.resources_on_planet.contains_key(&location) {
            result = i32::from(self.board.resources_on_planet[&location]);
            self.board.resources_on_planet.remove(&location);
        }

        result
    }

    pub fn remove_resources_territory(&mut self, territory: Territory) -> i32 {
        let mut result: i32 = 0;
        for l in territory.locations() {
            result += self.remove_resources(*l);
        }

        result
    }

    pub fn is_in_storm(&self, l: Location) -> bool {
        Sector::in_storm(l.sector(&self.board.map), self.board.sector_in_storm)
    }

    pub fn is_in_storm_territory(&self, t: Territory) -> bool {
        t.locations().iter().any(|l| self.is_in_storm(*l))
    }

    pub fn occupies(&self, f: impl Into<Option<Faction>>, w: World) -> bool {
        let f: Option<Faction> = f.into();
        if f != None {
            let hw_occupation = self.board.homeworld_occupation.keys().find(|hw| hw.world() == w).copied();
            if let Some(hw) = hw_occupation {
                return self.board.homeworld_occupation.get(&hw).copied() == f;
            }
        }

        false
    }

    pub fn occupier_of(&self, w: World) -> Option<PlayerId> {
        let hw_occupation = self.board.homeworld_occupation.keys().find(|hw| hw.world() == w).copied();
        match hw_occupation {
            Some(hw) => self.get_player(self.board.homeworld_occupation.get(&hw).copied()),
            None => None,
        }
    }

    pub fn owner_of_location(&self, stronghold: Location) -> Option<PlayerId> {
        match self.board.stronghold_ownership.get(&stronghold) {
            Some(value) => self.get_player(Some(*value)),
            None => None,
        }
    }

    pub fn determine_occupation_at_start_or_end_of_turn(&mut self) {
        let current_occupier_of_ecaz_homeworld = self.occupier_of(World::Ecaz);
        let mut updated_occupation: IndexMap<Homeworld, Faction> = IndexMap::new();

        for hw in self.board.map.homeworlds() {
            for i in 0..self.players.len() {
                let player = self.player(self.player_at(i));
                let faction = player.faction;
                if player.controls(self, hw.location(), false) && !player.is_native_homeworld(hw) {
                    updated_occupation.insert_new(hw, faction.expect("faction"));

                    if !self.occupies(faction, hw.world()) {
                        self.log(vec![faction.into(), " now occupy ".into(), hw.location().into()]);
                    }
                }
            }
        }

        let previous: Vec<(Homeworld, Faction)> = self.board.homeworld_occupation.iter().map(|(k, v)| (*k, *v)).collect();
        for (key, value) in previous {
            // ICollection<KeyValuePair>.Contains: the key is present with an equal value
            if updated_occupation.get(&key) != Some(&value) {
                self.log(vec![value.into(), " no longer occupies ".into(), key.location().into()]);
            }
        }

        self.board.homeworld_occupation = updated_occupation;

        self.check_if_shipment_permissions_should_be_revoked();
        self.check_if_occupier_takes_vidal(current_occupier_of_ecaz_homeworld);
        self.let_factions_discard_surplus_cards();
    }
}
