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
