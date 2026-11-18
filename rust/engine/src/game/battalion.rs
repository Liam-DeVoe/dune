use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct Battalion {
    pub faction: Faction,
    pub location: Location,
    pub amount_of_forces: u16,
    pub amount_of_special_forces: u16,
}

impl Battalion {
    pub fn new(faction: Faction, amount_of_forces: u16, amount_of_special_forces: u16, location: Location) -> Self {
        Battalion { faction, amount_of_forces, amount_of_special_forces, location }
    }

    pub fn is(&self, f: impl Into<Option<Faction>>) -> bool {
        let f: Option<Faction> = f.into();
        Some(self.faction) == f
    }

    pub fn can_occupy(&self) -> bool {
        self.amount_of_forces > 0 || (self.faction != Faction::BeneGesserit && self.amount_of_special_forces > 0)
    }

    /// The forces as `i32`, the type the rules compute with.
    pub fn forces(&self) -> i32 {
        i32::from(self.amount_of_forces)
    }

    /// The special forces as `i32`, the type the rules compute with.
    pub fn special_forces(&self) -> i32 {
        i32::from(self.amount_of_special_forces)
    }

    pub fn total_amount_of_forces(&self) -> i32 {
        self.forces() + self.special_forces()
    }

    /// A Game Error if the battalion would have fewer than zero forces.
    pub fn change_forces(&mut self, amount: i32) -> GameResult {
        crate::game::amounts::change(&mut self.amount_of_forces, amount, "forces")
    }

    /// A Game Error if the battalion would have fewer than zero special forces.
    pub fn change_special_forces(&mut self, amount: i32) -> GameResult {
        crate::game::amounts::change(&mut self.amount_of_special_forces, amount, "special forces")
    }

    pub fn clear(&mut self) {
        self.amount_of_forces = 0;
        self.amount_of_special_forces = 0;
    }

    pub fn take_half(&self) -> Battalion {
        Battalion::new(
            self.faction,
            self.amount_of_forces.div_ceil(2),
            self.amount_of_special_forces.div_ceil(2),
            self.location,
        )
    }

    /// At most `amount` forces of the battalion. A negative `amount` takes none.
    pub fn take(&self, amount: i32, prefer_special: bool) -> Battalion {
        if amount >= self.total_amount_of_forces() {
            return self.clone();
        }
        // Each kind is a u16, so taking u16::MAX of a kind takes all of it.
        let amount = u16::try_from(amount.max(0)).unwrap_or(u16::MAX);

        if prefer_special {
            let mut to_take = amount;
            let special_amount_to_take = std::cmp::min(self.amount_of_special_forces, to_take);
            to_take -= special_amount_to_take;
            let normal_amount_to_take = std::cmp::min(self.amount_of_forces, to_take);
            Battalion::new(self.faction, normal_amount_to_take, special_amount_to_take, self.location)
        } else {
            let mut to_take = amount;
            let normal_amount_to_take = std::cmp::min(self.amount_of_forces, to_take);
            to_take -= normal_amount_to_take;
            let special_amount_to_take = std::cmp::min(self.amount_of_special_forces, to_take);
            Battalion::new(self.faction, normal_amount_to_take, special_amount_to_take, self.location)
        }
    }

}
