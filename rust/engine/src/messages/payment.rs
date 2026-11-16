use crate::enumerations::Faction;
use crate::events::EventType;

/// `Payment`. Messages use it to print an amount of spice (`Payment.Of(3)`).
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Payment {
    pub amount: i32,
    pub by: Option<Faction>,
    pub to: Option<Faction>,
    /// The type of event the payment was for (`Game::set_recent_payment`).
    pub reason: Option<EventType>,
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

    pub fn new_full(amount: i32, by: Option<Faction>, to: Option<Faction>, reason: Option<EventType>) -> Self {
        Payment { amount, by, to, reason }
    }
}
