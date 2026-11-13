use treachery_engine::events::GameEventMethods;
use treachery_engine::game::{Game, GameState, Participation};
use treachery_engine::shared::Shared;
#[test]
fn probe() {
    let name = std::env::var("PROBE").unwrap_or("alliance-allied".into());
    let p = format!("{}/../../scripts/fixtures/{name}.json", env!("CARGO_MANIFEST_DIR"));
    let state = GameState::load(&std::fs::read_to_string(&p).unwrap()).unwrap();
    let mut g = Game::new_int32_participation(state.version, &Shared::new(Participation::default()));
    let mut neg = vec![false; 12];
    for (i, e) in state.events.as_ref().unwrap().iter().enumerate() {
        e.initialize_game(&g);
        e.execute(&mut g, false, true);
        for (k, pl) in g.players.iter().enumerate() {
            if (pl.resources < 0) != neg[k] {
                neg[k] = pl.resources < 0;
                println!("event {i} {}: {} now has {} spice: {}", e.event_type().name(), pl.faction, pl.resources, treachery_engine::json::serialize(e));
            }
        }
    }
}
