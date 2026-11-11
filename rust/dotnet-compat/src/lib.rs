//! Exact reimplementations of .NET behavior that the game engine depends on.
//!
//! Every type here must match .NET 10 bit for bit, including iteration order
//! and the sequence of random numbers. APIs that throw in .NET panic with a
//! [`CsException`] payload carrying the exact .NET type name and message. See ../../docs/hazards.md.
//! Existing signatures are fixed (other crates code against them). See
//! ../../docs/compat.md for what each item maps to in C#.

pub mod exception;
pub mod random;
pub mod dictionary;
pub mod hashset;
pub mod sort;
pub mod math;
pub mod linq;
pub mod parse;
pub mod fmt;

pub use dictionary::NetDictionary;
pub use exception::{CsException, CsToString};
pub use hashset::NetHashSet;
pub use random::NetRandom;
