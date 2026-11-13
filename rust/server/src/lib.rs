//! The treachery.online server: accounts, the lobby, running games and their bots, over one
//! WebSocket per client (rust/docs/protocol.md), with SQLite for what must survive a restart.
//! rust/docs/server.md describes how it fits together.
//!
//! - [`http`]: the process: the game thread and the HTTP server; [`socket`] is one WebSocket.
//! - [`server`]: the state and every call's handling, on the game thread.
//! - [`protocol`] and [`wire`]: the messages and the objects in them.
//! - [`connections`]: which connection follows which game; notifications.
//! - [`views`]: each participant's view of a game, built on a replica of it.
//! - [`db`]: the database, and the conversion of the old C# server's database.
//! - [`pause_rules`]: what each pause between phases waits for.
//! - [`passwords`]: account password hashes; [`ids`]: tokens and their digests.
//! - [`settings`], [`time`], [`timers`], [`mail`], [`participation`],
//!   [`render`], [`static_files`].

pub mod connections;
pub mod db;
pub mod http;
pub mod ids;
pub mod mail;
pub mod participation;
pub mod passwords;
pub mod pause_rules;
pub mod protocol;
pub mod render;
pub mod server;
pub mod settings;
pub mod socket;
pub mod static_files;
pub mod time;
pub mod timers;
pub mod views;
pub mod wire;
