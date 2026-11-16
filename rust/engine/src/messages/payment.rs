use crate::enumerations::Faction;
use crate::events::Event;

/// `Payment`. Messages use it to print an amount of spice (`Payment.Of(3)`).
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Payment {
    pub amount: i32,
    pub by: Option<Faction>,
    pub to: Option<Faction>,
    /// `Event? Reason`, set by `Game.SetRecentPayment`. Compared by reference.
    pub reason: Option<Event>,
}

impl Payment {
    pub fn new(amount: i32) -> Self {
        Payment { amount, ..Default::default() }
    }

    pub fn of(amount: i32) -> Self {
        Payment { amount, ..Default::default() }
    }

    pub fn of_by(amount: i32, by: Option<Faction>) -> Self {
        Payment { amount, by, ..Default::default() }
    }

    pub fn new_full(amount: i32, by: Option<Faction>, to: Option<Faction>, reason: Option<Event>) -> Self {
        Payment { amount, by, to, reason }
    }
}
