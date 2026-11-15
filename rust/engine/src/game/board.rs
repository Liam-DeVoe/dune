use crate::prelude::*;

/// What is on the board besides the players' forces: the map, the storm, spice, terror tokens,
/// ambassadors, discoveries, stronghold ownership and homeworld occupation.
pub struct Board {
    pub map: Map,
    pub sector_in_storm: i32,
    pub next_storm_moves: i32,
    pub shield_wall_destroyed: bool,
    pub resources_on_planet: NetDictionary<Location, i32>,
    pub terror_on_planet: NetDictionary<TerrorType, Territory>,
    pub unplaced_terror_tokens: Vec<TerrorType>,
    pub atomics_aftermath: Option<Territory>,
    pub discoveries_on_planet: NetDictionary<Location, Discovery>,
    pub ambassadors_on_planet: NetDictionary<Territory, Ambassador>,
    pub homeworld_occupation: NetDictionary<Homeworld, Faction>,
    pub stronghold_ownership: NetDictionary<Location, Faction>,
}

impl Board {
    pub fn new() -> Self {
        Board {
            map: Map::new(),
            sector_in_storm: -1,
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
        }
    }
}

impl Game {
    pub fn resources_in(&self, l: Location) -> i32 {
        self.board.resources_on_planet.get(&l).copied().unwrap_or(0)
    }

    pub fn change_resources_on_planet(&mut self, location: Location, amount: i32) {
        if self.board.resources_on_planet.contains_key(&location) {
            let current = self.board.resources_on_planet[&location];
            if (current + amount) == 0 {
                self.board.resources_on_planet.remove(&location);
            } else {
                self.board.resources_on_planet.insert(location, current + amount);
            }
        } else if amount != 0 {
            self.board.resources_on_planet.insert_new(location, amount);
        }
    }

    pub fn remove_resources(&mut self, location: Location) -> i32 {
        let mut result = 0;
        if self.board.resources_on_planet.contains_key(&location) {
            result = self.board.resources_on_planet[&location];
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
        l.sector(&self.board.map) == self.board.sector_in_storm
    }

    pub fn is_in_storm_territory(&self, t: Territory) -> bool {
        t.locations().iter().any(|l| self.is_in_storm(*l))
    }

    pub fn occupies(&self, f: Faction, w: World) -> bool {
        if f != Faction::None {
            let hw_occupation = self.board.homeworld_occupation.keys().find(|hw| hw.world() == w).copied();
            if let Some(hw) = hw_occupation {
                return self.board.homeworld_occupation.get(&hw) == Some(&f);
            }
        }

        false
    }

    pub fn occupier_of(&self, w: World) -> Option<PlayerIndex> {
        let hw_occupation = self.board.homeworld_occupation.keys().find(|hw| hw.world() == w).copied();
        match hw_occupation {
            Some(hw) => self.get_player(self.board.homeworld_occupation.get(&hw).copied()),
            None => None,
        }
    }

    pub fn owner_of_location(&self, stronghold: Location) -> Option<PlayerIndex> {
        match self.board.stronghold_ownership.get(&stronghold) {
            Some(value) => self.get_player(Some(*value)),
            None => None,
        }
    }

    pub fn determine_occupation_at_start_or_end_of_turn(&mut self) {
        let current_occupier_of_ecaz_homeworld = self.occupier_of(World::Ecaz);
        let mut updated_occupation: NetDictionary<Homeworld, Faction> = NetDictionary::new();

        for hw in self.board.map.homeworlds() {
            for i in 0..self.players.len() {
                let player = self.player(self.player_at(i));
                let faction = player.faction;
                if player.controls(self, hw.location(), false) && !player.is_native_homeworld(hw) {
                    updated_occupation.insert_new(hw, faction);

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
