use std::ops::Deref;

use crate::enumerations::{Faction, World};
use crate::map::location::Location;
use crate::map::{HomeworldData, LocationKind};

/// Handle to a homeworld (`l is Homeworld hw`). Derefs to [`Location`].
/// Its `Sector` is always -1 (stored in the static table).
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct Homeworld(Location);

impl Homeworld {
    pub(crate) fn from_location(l: Location) -> Self {
        Homeworld(l)
    }

    pub fn location(self) -> Location {
        self.0
    }

    fn hw(self) -> &'static HomeworldData {
        match &self.0.data().kind {
            LocationKind::Homeworld(hw) => hw,
            _ => unreachable!("Homeworld handle on a non-homeworld location"),
        }
    }

    pub fn world(self) -> World {
        self.hw().world
    }

    pub fn faction(self) -> Faction {
        self.hw().faction
    }

    pub fn threshold(self) -> i32 {
        self.hw().threshold
    }

    pub fn resource_amount(self) -> i32 {
        self.hw().resource_amount
    }

    pub fn is_home_of_normal_forces(self) -> bool {
        self.hw().is_home_of_normal_forces
    }

    pub fn is_home_of_special_forces(self) -> bool {
        self.hw().is_home_of_special_forces
    }

    pub fn battle_bonus_and_lasgun_shield_limit_at_high_threshold(self) -> i32 {
        self.hw().battle_bonus_and_lasgun_shield_limit_at_high_threshold
    }

    pub fn battle_bonus_and_lasgun_shield_limit_at_low_threshold(self) -> i32 {
        self.hw().battle_bonus_and_lasgun_shield_limit_at_low_threshold
    }
}

impl Deref for Homeworld {
    type Target = Location;
    fn deref(&self) -> &Location {
        &self.0
    }
}

impl From<Homeworld> for Location {
    fn from(h: Homeworld) -> Location {
        h.0
    }
}

/// `HomeworldStatus`
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HomeworldStatus {
    is_high: bool,
    occupant: Faction,
}

impl HomeworldStatus {
    pub fn new(is_high: bool, occupant: Faction) -> Self {
        HomeworldStatus { is_high, occupant }
    }

    pub fn is_high(&self) -> bool {
        self.is_high
    }

    pub fn is_low(&self) -> bool {
        !self.is_high
    }

    pub fn occupant(&self) -> Faction {
        self.occupant
    }
}
