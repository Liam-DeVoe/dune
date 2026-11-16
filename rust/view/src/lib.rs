//! Per-player game views for the Rust server and the React client (`web/`).
//!
//! Views are built from the server's own game: every query they make takes `&Game`. What a
//! game's views keep between builds (the skin and the faction sheets) is a [`cache::ViewCache`],
//! which the server keeps per game.
//!
//! Design: `rust/docs/client.md`. Wire format: the Game view section of
//! `rust/docs/protocol.md`. Types: [`schema`].

pub mod actions;
pub mod board;
pub mod build;
pub mod cache;
pub mod describe;
pub mod ending;
pub mod expr;
pub mod lobby;
pub mod log;
pub mod schema;
pub mod shield;
pub mod side;
pub mod skin_texts;
pub mod static_data;
pub mod wire;
