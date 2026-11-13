use crate::prelude::*;

#[derive(Clone, Debug, PartialEq, Canon)]
pub struct CharityClaim {
    pub faction: Faction,
    pub amount: i32,
}

impl Default for CharityClaim {
    fn default() -> Self {
        CharityClaim {
            faction: Default::default(),
            amount: Default::default(),
        }
    }
}

impl CharityClaim {
    pub fn new(faction: Faction, amount: i32) -> Self {
        CharityClaim { faction, amount }
    }

}
