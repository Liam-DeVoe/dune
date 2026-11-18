//! The Treachery game engine: rules, events, validation and log messages.
//! See ../../docs/engine.md for the module layout.

pub mod prelude;

pub use indexmap;

pub mod error;

// Game state and rules
pub mod events;
pub mod game;
pub mod log;

// Static game data
pub mod cards;
pub mod enum_type;
pub mod enumerations;
pub mod heroes;
pub mod identity;
pub mod map;

// Messages
pub mod messages;

// Numbers and enum names in event text
pub mod parse;

// Iterator helpers
pub mod iter;

// Randomness
pub mod random;

// Time stamps of events
pub mod compat;
