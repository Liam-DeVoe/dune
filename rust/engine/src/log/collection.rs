use crate::prelude::*;

#[derive(Clone, Debug, PartialEq)]
pub struct SpiceCollected {
    pub faction: Faction,
    pub territory: Territory,
    pub amount: i32,
    pub field: Option<Location>,
    pub income: bool,
}

impl SpiceCollected {
    pub fn new(faction: Faction, territory: Territory, amount: i32, field: Option<Location>, income: bool) -> Self {
        SpiceCollected { faction, territory, amount, field, income }
    }

}
