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
    /// `new Payment(amount)`
    pub fn new(amount: i32) -> Self {
        Payment { amount, ..Default::default() }
    }

    /// `new Payment(amount, by)`
    pub fn new_by(amount: i32, by: Faction) -> Self {
        Payment { amount, by, ..Default::default() }
    }

    /// `Payment.Of(amount)`
    pub fn of(amount: i32) -> Self {
        Payment { amount, ..Default::default() }
    }

    /// `Payment.Of(amount, by)`
    pub fn of_by(amount: i32, by: Faction) -> Self {
        Payment { amount, by, ..Default::default() }
    }

    /// `new Payment(amount, by, to, reason)`
    pub fn new_full(amount: i32, by: Faction, to: Faction, reason: Option<GameEvent>) -> Self {
        Payment { amount, by, to, reason }
    }
}
