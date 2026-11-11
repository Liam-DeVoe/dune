//! Prints the engine's static data the web client needs beside the skin (`src/static_data.rs`):
//!
//!     cargo run -p treachery-view --example static_data > web/public/static/data.json
//!
//! `tests/static_data.rs` fails when the checked-in file is stale.

fn main() {
    println!("{}", serde_json::to_string(&treachery_view::static_data::static_data()).unwrap());
}
