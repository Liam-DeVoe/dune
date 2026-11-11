//! Port of `treachery-online/Server`, the C# SignalR `GameHub`, to axum + tokio + sqlx.
//!
//! Plan, inventory and package split: `rust/docs/server-plan.md`. Wire format:
//! `rust/docs/protocol.md`.
//!
//! Layers, from the inside out:
//!
//! - [`hub`]: the port of the `GameHub` partial classes. Transport-free: every hub method
//!   takes typed arguments and returns the C# result type, and sends [`clients::Callback`]s
//!   (the C# `IGameClient` calls) through [`clients::Clients`]. This is the layer that must
//!   be equivalent to C#, and the layer the equivalence harness drives.
//! - [`db`]: the EF Core model (`Server/Data`) on sqlx, same SQLite schema.
//! - [`model`]: the server types the C# keeps in `Shared/Model` (`ManagedGame`, `GameInfo`, ...).
//! - [`transport`]: the wire format, the WebSocket and the web client's files. Thin and
//!   replaceable.
//! - [`views`]: the game-view layer: a view replica per game, `GameView` notifications, and the
//!   view calls (`QueryDraft`, `RequestReport`, `RequestUndoList`). Never touches the hub's games
//!   beyond reading their history.
//! - [`dev`]: the development-only scene endpoint.
//! - [`runtime`]: the hub thread that runs every hub call one at a time (see the plan,
//!   "Concurrency"), and the axum app.

pub mod clients;
pub mod clock;
pub mod config;
pub mod db;
pub mod dev;
pub mod dotnet_string;
pub mod hub;
pub mod json;
pub mod logged_in_user;
pub mod mail;
pub mod model;
pub mod net_time;
pub mod phase_control;
pub mod render;
pub mod runtime;
pub mod timers;
pub mod transport;
pub mod views;
