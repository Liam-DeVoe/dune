//! Prints the lobby data the web client loads (`src/lobby.rs`):
//!
//!     cargo run -p treachery-view --example lobby_data > web/public/static/lobby.json
//!
//! `tests/lobby_data.rs` fails when the checked-in file is stale.

fn main() {
    println!("{}", serde_json::to_string(&treachery_view::lobby::lobby_data()).unwrap());
}
