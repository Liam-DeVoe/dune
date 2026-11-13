use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct Battalion {
    pub faction: Faction,
    pub location: Location,
    pub amount_of_forces: i32,
    pub amount_of_special_forces: i32,
}

impl Battalion {
    pub fn new(faction: Faction, amount_of_forces: i32, amount_of_special_forces: i32, location: Location) -> Self {
        Battalion { faction, amount_of_forces, amount_of_special_forces, location }
    }

    pub fn is(&self, f: Faction) -> bool {
        self.faction == f
    }

    pub fn can_occupy(&self) -> bool {
        self.amount_of_forces > 0 || (self.faction != Faction::BeneGesserit && self.amount_of_special_forces > 0)
    }

    pub fn total_amount_of_forces(&self) -> i32 {
        self.amount_of_forces.wrapping_add(self.amount_of_special_forces)
    }

    pub fn change_forces(&mut self, amount: i32) {
        if self.amount_of_forces.wrapping_add(amount) >= 0 {
            self.amount_of_forces = self.amount_of_forces.wrapping_add(amount);
        }
    }

    pub fn change_special_forces(&mut self, amount: i32) {
        if self.amount_of_special_forces.wrapping_add(amount) >= 0 {
            self.amount_of_special_forces = self.amount_of_special_forces.wrapping_add(amount);
        }
    }

    pub fn clear(&mut self) {
        self.amount_of_forces = 0;
        self.amount_of_special_forces = 0;
    }

    pub fn take_half(&self) -> Battalion {
        Battalion::new(
            self.faction,
            (0.5f64 * self.amount_of_forces as f64).ceil() as i32,
            (0.5f64 * self.amount_of_special_forces as f64).ceil() as i32,
            self.location,
        )
    }

    pub fn take(&self, amount: i32, prefer_special: bool) -> Battalion {
        // C# returns `this` here. Rust returns a copy; no Shared caller mutates the result
        // (only bots call Take), so the identity difference is unobservable.
        if amount >= self.total_amount_of_forces() {
            return self.clone();
        }

        if prefer_special {
            let mut to_take = amount;
            let special_amount_to_take = std::cmp::min(self.amount_of_special_forces, to_take);
            to_take = to_take.wrapping_sub(special_amount_to_take);
            let normal_amount_to_take = std::cmp::min(self.amount_of_forces, to_take);
            Battalion::new(self.faction, normal_amount_to_take, special_amount_to_take, self.location)
        } else {
            let mut to_take = amount;
            let normal_amount_to_take = std::cmp::min(self.amount_of_forces, to_take);
            to_take = to_take.wrapping_sub(normal_amount_to_take);
            let special_amount_to_take = std::cmp::min(self.amount_of_special_forces, to_take);
            Battalion::new(self.faction, normal_amount_to_take, special_amount_to_take, self.location)
        }
    }

    /// Returns the memberwise clone as a `Battalion` rather than `object`; only `Game.Clone`
    /// (via `Utilities.CloneObjectDictionary`) calls it, and that is not ported.
    pub fn cs_clone(&self) -> Battalion {
        self.clone()
    }

}
