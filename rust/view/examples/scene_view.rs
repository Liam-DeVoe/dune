//! Prints the view of a scene as JSON: a fixture from `scripts/fixtures`, seen by one faction,
//! as the C# `/scene` page would show it. The web skeleton's sample views come from this:
//!
//!     cargo run -p treachery-view --example scene_view -- alliance Emperor > web/public/views/alliance-Emperor.json

use treachery_engine::enumerations::Faction;
use treachery_view::replica::ViewReplica;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (fixture, faction) = (&args[1], &args[2]);
    let path = format!("{}/../../scripts/fixtures/{fixture}.json", env!("CARGO_MANIFEST_DIR"));
    let savegame = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
    let faction = Faction::ALL.iter().copied().find(|f| f.to_string() == *faction).unwrap_or_else(|| panic!("unknown faction {faction}"));
    let (mut replica, viewer) = ViewReplica::scene(&savegame, faction).unwrap();
    println!("{}", serde_json::to_string_pretty(&replica.view(viewer)).unwrap());
}
