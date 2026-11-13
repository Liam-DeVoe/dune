use crate::prelude::*;

#[derive(Clone, Debug, PartialEq, Canon)]
pub struct RevivalRow {
    pub faction: Faction,
    pub forces: i32,
    pub special_forces: i32,
    pub cost: i32,
    pub hero: Option<Hero>,
    pub as_ghola: bool,
    pub paid_by_other: i32,
    pub payer: Faction,
    pub card: Option<TreacheryCard>,
    pub tleilaxu_income: i32,
}

impl Default for RevivalRow {
    fn default() -> Self {
        RevivalRow {
            faction: Default::default(),
            forces: Default::default(),
            special_forces: Default::default(),
            cost: Default::default(),
            hero: Default::default(),
            as_ghola: Default::default(),
            paid_by_other: Default::default(),
            payer: Default::default(),
            card: Default::default(),
            tleilaxu_income: Default::default(),
        }
    }
}

impl RevivalRow {
    pub fn new(faction: Faction, forces: i32, special_forces: i32, cost: i32) -> Self {
        RevivalRow { faction, forces, special_forces, cost, hero: None, as_ghola: false, paid_by_other: 0, payer: Faction::None, card: None, tleilaxu_income: 0 }
    }

}
