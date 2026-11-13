//! The Treachery game engine: rules, events, validation and log messages.
//! See ../../docs/engine.md for the module layout.

pub mod prelude;

// Game state and rules
pub mod events;
pub mod game;
pub mod log;

// Static game data
pub mod cards;
pub mod enumerations;
pub mod heroes;
pub mod identity;
pub mod map;

// Messages
pub mod messages;

// Randomness, serialization and the state dump
pub mod canon;
pub mod json;
pub mod random;

// .NET runtime behavior the engine still relies on
pub mod compat;
