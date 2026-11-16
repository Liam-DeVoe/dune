use crate::prelude::*;

#[derive(Clone, Debug, PartialEq)]
pub struct CharityClaim {
    pub faction: Faction,
    pub amount: i32,
}

impl CharityClaim {
    pub fn new(faction: Faction, amount: i32) -> Self {
        CharityClaim { faction, amount }
    }

}
