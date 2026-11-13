//! Exact reimplementations of .NET behavior that the game engine depends on.
//!
//! Every type here must match .NET 10 bit for bit, including iteration order
//! and the sequence of random numbers. APIs that throw in .NET panic with the
//! .NET type name and message (see `exception`). See ../../docs/compat.md for
//! what each item maps to in .NET and why the game still needs it.

pub mod exception;
pub mod random;
pub mod dictionary;
pub mod hashset;
pub mod math;
pub mod parse;
pub mod fmt;

pub use dictionary::NetDictionary;
pub use exception::{CsException, CsToString};
pub use hashset::NetHashSet;
pub use random::NetRandom;
