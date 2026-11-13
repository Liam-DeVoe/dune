//! Structured records attached to log messages (`with_log`), one module per game phase.

pub mod aside;
pub use aside::*;
pub mod bidding;
pub use bidding::*;
pub mod blow;
pub use blow::*;
pub mod charity;
pub use charity::*;
pub mod collection;
pub use collection::*;
pub mod mentat;
pub use mentat::*;
pub mod revival;
pub use revival::*;
pub mod ship_move;
pub use ship_move::*;
pub mod storm;
pub use storm::*;
