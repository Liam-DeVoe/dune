//! .NET runtime behavior the engine still imitates: exceptions and throwing LINQ operators (`cs`),
//! integer-backed enums, `DateTimeOffset`, and random selection helpers.
//! Later cleanup steps replace these with plain Rust.

pub mod cs;
pub mod date_time_offset;
pub mod net_enum;
pub mod random_ext;
