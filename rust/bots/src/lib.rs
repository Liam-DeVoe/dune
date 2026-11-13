//! The classic bots: `ClassicBot` decides, for one player, which event to submit next.

pub mod bot;
pub mod parameters;
pub mod prelude;
pub mod support;

pub use bot::{ClassicBot, Priority};
pub use parameters::BotParameters;
