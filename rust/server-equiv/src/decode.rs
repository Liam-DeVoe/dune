//! Scenario arguments (C# `Utilities.Serialize` form) to `HubCall`s. The decoder is the
//! server transport's, so scenarios and WebSocket clients bind arguments the same way.

pub use treachery_server::transport::decode::{decode, METHODS};
