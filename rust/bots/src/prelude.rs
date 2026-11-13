//! Everything a bot module needs: `use crate::prelude::*;`. Support module (no C# counterpart).

pub use treachery_engine::prelude::*;
pub use treachery_engine::events::GameEventType;
pub use treachery_engine::game::Game;
pub use treachery_engine::game::{Player, PlayerIndex};
pub use treachery_engine::game::ObjectCounter;
pub use treachery_engine::compat::random_ext::RandomExt;

pub use crate::bot::ClassicBot;
pub use crate::parameters::BotParameters;
pub use crate::bot::VoicePlan;
pub use crate::bot::BattalionInLocation;
pub use crate::bot::MovedBattalion;
pub use crate::bot::ShipmentDecision;
pub use crate::bot::Attack;
