//! Per-player game views for the Rust server and the React client (`web/`).
//!
//! The server holds the authoritative game. Views are never built from it: building one runs
//! `GameStatus::determine_status` and `get_applicable_events`, which write game state, and draft queries fill the neighbour cache. So the server keeps a
//! [`replica::ViewReplica`] per game, a second copy that executes every accepted event and
//! answers every view and query.
//!
//! Design: `rust/docs/client.md`. Wire format: the Game view section of
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
