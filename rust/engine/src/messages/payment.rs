use crate::enumerations::Faction;
use crate::events::GameEvent;

/// `Payment`. Messages use it to print an amount of spice (`Payment.Of(3)`).
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Payment {
    pub amount: i32,
    pub by: Faction,
    pub to: Faction,
    /// `GameEvent? Reason`, set by `Game.SetRecentPayment`. Compared by reference.
    pub reason: Option<GameEvent>,
}

impl Payment {
    pub fn new(amount: i32) -> Self {
        Payment { amount, ..Default::default() }
    }

    pub fn of(amount: i32) -> Self {
        Payment { amount, ..Default::default() }
    }

    pub fn of_by(amount: i32, by: Faction) -> Self {
        Payment { amount, by, ..Default::default() }
    }

    pub fn new_full(amount: i32, by: Faction, to: Faction, reason: Option<GameEvent>) -> Self {
        Payment { amount, by, to, reason }
    }
}
