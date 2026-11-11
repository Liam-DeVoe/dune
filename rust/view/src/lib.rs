//! Per-player game views for the Rust server and the React client (`web/`).
//!
//! The server holds the authoritative game. Views are never built from it: building one runs
//! `GameStatus.DetermineStatus` and `GetApplicableEvents`, which write game state (hazards 4.3),
//! and draft queries fill the neighbour cache (hazards 6.1). C# ran those calls only in each
//! client's own copy of the game. So the server keeps a [`replica::ViewReplica`] per game, which
//! executes every event the way `Client.cs` does and answers every view and query.
//!
//! Design: `rust/docs/client-plan.md`. Wire format: the Game view section of
//! `rust/docs/protocol.md`. Types: [`schema`].

pub mod actions;
pub mod board;
pub mod build;
pub mod describe;
pub mod ending;
pub mod expr;
pub mod lobby;
pub mod log;
pub mod replica;
pub mod schema;
pub mod shield;
pub mod side;
pub mod skin_texts;
pub mod static_data;
