use crate::prelude::*;

#[derive(Clone, Debug, PartialEq)]
pub struct StormDials {
    pub first: Faction,
    pub first_dial: i32,
    pub second: Faction,
    pub second_dial: i32,
}

impl StormDials {
    pub fn new(first: Faction, first_dial: i32, second: Faction, second_dial: i32) -> Self {
        StormDials { first, first_dial, second, second_dial }
    }

}

#[derive(Clone, Debug, PartialEq)]
pub struct StormPlaced {
    pub sector: i32,
}

impl StormPlaced {
    pub fn new(sector: i32) -> Self {
        StormPlaced { sector }
    }

}

#[derive(Clone, Debug, PartialEq)]
pub struct StormForecast {
    pub distance: i32,
}

impl StormForecast {
    pub fn new(distance: i32) -> Self {
        StormForecast { distance }
    }

}

#[derive(Clone, Debug, PartialEq)]
pub struct StormMoved {
    pub from: i32,
    pub distance: i32,
    pub by: Option<Faction>,
    pub card: Option<TreacheryCard>,
}

impl StormMoved {
    pub fn new(from: i32, distance: i32, by: Option<Faction>, card: Option<TreacheryCard>) -> Self {
        StormMoved { from, distance, by, card }
    }

}

#[derive(Clone, Debug, PartialEq)]
pub struct StormKilled {
    pub faction: Faction,
    pub location: Location,
    pub forces: i32,
    pub special_forces: i32,
}

impl StormKilled {
    pub fn new(faction: Faction, location: Location, forces: i32, special_forces: i32) -> Self {
        StormKilled { faction, location, forces, special_forces }
    }

}

#[derive(Clone, Debug, PartialEq)]
pub struct StormDestroyedSpice {
    pub location: Location,
    pub amount: i32,
}

impl StormDestroyedSpice {
    pub fn new(location: Location, amount: i32) -> Self {
        StormDestroyedSpice { location, amount }
    }

}

#[derive(Clone, Debug, PartialEq)]
pub struct FamilyAtomicsUsed {
    pub faction: Faction,
    pub card: Option<TreacheryCard>,
}

impl FamilyAtomicsUsed {
    pub fn new(faction: Faction, card: Option<TreacheryCard>) -> Self {
        FamilyAtomicsUsed { faction, card }
    }

}

#[derive(Clone, Debug, PartialEq)]
pub struct FamilyAtomicsKilled {
    pub faction: Faction,
    pub forces: i32,
    pub special_forces: i32,
}

impl FamilyAtomicsKilled {
    pub fn new(faction: Faction, forces: i32, special_forces: i32) -> Self {
        FamilyAtomicsKilled { faction, forces, special_forces }
    }

}

#[derive(Clone, Debug, PartialEq)]
pub struct BeneGesseritFlipped {
    pub territory: Territory,
}

impl BeneGesseritFlipped {
    pub fn new(territory: Territory) -> Self {
        BeneGesseritFlipped { territory }
    }

}
