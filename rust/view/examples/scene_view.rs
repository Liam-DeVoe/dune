//! Prints the view of a scene as JSON: a fixture from `scripts/fixtures`, seen by one faction
//! (see `scene_views.rs`). The web client's sample views come from this:
//!
//!     cargo run -p treachery-view --example scene_view -- alliance Emperor > web/public/views/alliance-Emperor.json

use treachery_engine::enumerations::Faction;
use treachery_view::cache::ViewedGame;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (fixture, faction) = (&args[1], &args[2]);
    let path = format!("{}/../../scripts/fixtures/{fixture}.json", env!("CARGO_MANIFEST_DIR"));
    let savegame = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
    let faction = Faction::ALL.iter().copied().find(|f| f.to_string() == *faction).unwrap_or_else(|| panic!("unknown faction {faction}"));
    // Fixtures are savegames of version 190.
    let savegame = serde_json::to_string(&treachery_legacy::load_any(&savegame).unwrap()).unwrap();
    let (mut viewed, viewer) = ViewedGame::scene(&savegame, faction).unwrap();
    println!("{}", serde_json::to_string_pretty(&viewed.view(viewer)).unwrap());
}
