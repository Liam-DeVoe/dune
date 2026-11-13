//! Writes the view of every scene, and a manifest of them, for the web client's
//! `/scene/:fixture/:faction` route (design work and screenshots without a server).
//!
//!     cargo run --release -p treachery-view --example scene_views -- OUT_DIR [FIXTURE...]
//!
//! For each fixture in `scripts/fixtures` (or the ones named), and for each faction in that game
//! plus an observer, it writes `OUT_DIR/<fixture>-<Faction>.json` (`-Observer.json` for the
//! observer). A scene is the savegame replayed with user 1, named "Scene", seated as that
//! faction and made host ([`ViewReplica::scene`]); the observer is user 1 as a non-host
//! observer. `<fixture>-<Faction>.reports.json` holds the lines of every report, for the log's
//! history.
//!
//! `OUT_DIR/manifest.json` lists the scenes. A fixture the engine cannot replay is listed with
//! its error and no scenes.

use std::path::{Path, PathBuf};

use serde_json::{json, Value};
use treachery_engine::shared::Shared;
use treachery_engine::enumerations::Faction;
use treachery_engine::game::Participation;
use treachery_engine::prelude::*;
use treachery_view::build::Viewer;
use treachery_view::replica::ViewReplica;

const SCENE_USER_ID: i32 = 1;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some((out, wanted)) = args.split_first() else {
        eprintln!("usage: scene_views OUT_DIR [FIXTURE...]");
        std::process::exit(2);
    };
    let out = PathBuf::from(out);
    std::fs::create_dir_all(&out).unwrap_or_else(|e| panic!("{}: {e}", out.display()));
    let fixtures_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/fixtures");

    let mut names: Vec<String> = if wanted.is_empty() {
        std::fs::read_dir(&fixtures_dir)
            .unwrap_or_else(|e| panic!("{}: {e}", fixtures_dir.display()))
            .filter_map(|e| e.ok()?.file_name().to_str()?.strip_suffix(".json").map(str::to_string))
            .collect()
    } else {
        wanted.to_vec()
    };
    names.sort();

    let mut fixtures = Vec::new();
    for name in &names {
        let path = fixtures_dir.join(format!("{name}.json"));
        let savegame = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        // A fixture that panics the engine is reported, not fatal: the other scenes still matter.
        let result = std::panic::catch_unwind(|| fixture(name, &savegame, &out));
        let entry = match result {
            Ok(Ok(entry)) => entry,
            Ok(Err(e)) => json!({"fixture": name, "error": e, "scenes": []}),
            Err(p) => {
                let msg = p.downcast_ref::<String>().cloned().or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string()));
                json!({"fixture": name, "error": format!("panic: {}", msg.unwrap_or_default()), "scenes": []})
            }
        };
        if let Some(e) = entry["error"].as_str() {
            eprintln!("{name}: {e}");
        }
        fixtures.push(entry);
    }
    let manifest = json!({"fixtures": fixtures});
    let path = out.join("manifest.json");
    std::fs::write(&path, serde_json::to_string_pretty(&manifest).unwrap()).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let scenes: usize = fixtures.iter().map(|f| f["scenes"].as_array().map_or(0, Vec::len)).sum();
    eprintln!("wrote {scenes} views of {} fixtures to {}", fixtures.len(), out.display());
}

/// Writes the views of one fixture and returns its manifest entry.
fn fixture(name: &str, savegame: &str, out: &Path) -> Result<Value, String> {
    // One replay to read the game's factions and facts; each scene then replays its own copy,
    // because building a view writes state (client.md, The view replica).
    let probe = ViewReplica::load(savegame, &Shared::new(Participation::default()))?;
    let game = &probe.game;
    let factions: Vec<Faction> = game.players.iter().map(|p| p.faction).collect();

    let mut scenes = Vec::new();
    for viewer in factions.iter().copied().map(Some).chain([None]) {
        let (mut replica, v) = match viewer {
            Some(f) => ViewReplica::scene(savegame, f)?,
            None => observer_scene(savegame)?,
        };
        let label = viewer.map_or("Observer".to_string(), |f| f.to_string());
        let mut view = replica.view(v);
        if client_auto_pass(&mut replica, viewer, &view)? {
            view = replica.view(v);
        }
        let file = format!("{name}-{label}.json");
        std::fs::write(out.join(&file), serde_json::to_string(&view).unwrap()).map_err(|e| format!("{file}: {e}"))?;
        // Every report's lines, as `RequestReport` answers them, for the log's history
        // (web/src/state/source.ts, StaticSource.report).
        let reports: Vec<_> = (0..replica.game.reports().len()).filter_map(|i| replica.report(v, i)).collect();
        let reports_file = format!("{name}-{label}.reports.json");
        std::fs::write(out.join(&reports_file), serde_json::to_string(&reports).unwrap()).map_err(|e| format!("{reports_file}: {e}"))?;
        scenes.push(json!({"faction": label, "view": file}));
    }
    Ok(json!({
        "fixture": name,
        "version": game.version,
        "turn": game.current_turn,
        "phase": game.current_phase.to_string(),
        "factions": factions.iter().map(|f| f.to_string()).collect::<Vec<_>>(),
        "scenes": scenes,
    }))
}

/// What the web client submits by itself once the scene is on screen: the TreacheryCalled form
/// passes for a player who cannot call treachery. With saved views that pass can't reach a
/// server, so the scene is written after it. Returns whether an event was applied.
fn client_auto_pass(replica: &mut ViewReplica, viewer: Option<Faction>, view: &treachery_view::schema::GameView) -> Result<bool, String> {
    let Some(f) = viewer else { return Ok(false) };
    if !view.actions.iter().any(|a| a.event_type == "TreacheryCalled") {
        return Ok(false);
    }
    let game = &replica.game;
    let Some(p) = game.get_player(Some(f)) else { return Ok(false) };
    if game.current_battle.is_none() || TreacheryCalled::may_call_treachery(game, p) {
        return Ok(false);
    }
    let mut pass = TreacheryCalled::new(game, f);
    pass.traitor_called = false;
    let json = treachery_engine::json::serialize(&GameEvent::from(std::rc::Rc::new(pass)));
    replica.apply(&json)?;
    Ok(true)
}

/// The observer scene: user 1, named "Scene", observing, not host.
fn observer_scene(savegame: &str) -> Result<(ViewReplica, Viewer), String> {
    let participation = Shared::new(Participation::default());
    let replica = ViewReplica::load(savegame, &participation)?;
    {
        let mut p = participation.borrow_mut();
        p.observers.insert(SCENE_USER_ID);
        p.player_names.insert(SCENE_USER_ID, "Scene".to_string());
    }
    Ok((replica, Viewer { user_id: SCENE_USER_ID }))
}
