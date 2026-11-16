//! Everything a bot module needs: `use crate::prelude::*;`.

pub use treachery_engine::prelude::*;
pub use treachery_engine::events::EventType;
pub use treachery_engine::game::Game;
pub use treachery_engine::game::{Player, PlayerId};
pub use treachery_engine::game::ObjectCounter;
pub use treachery_engine::random::RandomPick;

pub use crate::bot::{ClassicBot, Decision, Priority};
pub use crate::support::{f32_of, FloatKey};
pub use crate::parameters::BotParameters;
pub use crate::bot::VoicePlan;
pub use crate::bot::BattalionInLocation;
pub use crate::bot::MovedBattalion;
pub use crate::bot::ShipmentDecision;
pub use crate::bot::Attack;
