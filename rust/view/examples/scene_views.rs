//! Writes the view of every comparison scene, and a manifest of them, for the Playwright harness
//! that compares the web client with the C# client (`scripts/ui/compare.py`,
//! client-plan.md "Comparing with the C# client").
//!
//!     cargo run --release -p treachery-view --example scene_views -- OUT_DIR [FIXTURE...]
//!
//! For each fixture in `scripts/fixtures` (or the ones named), and for each faction in that game
//! plus an observer, it writes `OUT_DIR/<fixture>-<Faction>.json` (`-Observer.json` for the
//! observer): the view the web client's `/scene/:fixture/:faction` route renders. The scene is
//! the C# `/scene` page's: the savegame replayed, user 1 named "Scene" seated as that faction
//! and made host ([`ViewReplica::scene`]); the observer is user 1 named "Scene" as a non-host
//! observer, which is what `/scene?faction=observer` does in C#.
//!
//! `OUT_DIR/manifest.json` lists the scenes with the facts the harness needs to decide which
//! expected differences (protocol.md, "Where the C# client shows more than the rules allow")
//! apply to a scene. A fixture the engine cannot replay is listed with its error and no scenes.
//!
//! When the Rust server's dev scene endpoint exists, the harness can ask the server for the
//! scene instead and this example is only needed for the manifest.

use std::path::{Path, PathBuf};

use serde_json::{json, Value};
use treachery_engine::compat::cs::Shared;
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
    // because building a view writes state (client-plan.md, The view replica).
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
        scenes.push(json!({"faction": label, "view": file, "leaks": leaks(game, viewer)}));
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

/// What the C# client submits by itself once the scene is on screen, which the C# scene page then
/// executes: `TreacheryCalledComponent` passes for a player who cannot call treachery. With saved
/// views the web client's own pass (the TreacheryCalled form) can't reach a server, so the scene
/// is written after it. Returns whether an event was applied.
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

/// The C# `/scene` page with `faction=observer`: user 1, named "Scene", observing, not host.
fn observer_scene(savegame: &str) -> Result<(ViewReplica, Viewer), String> {
    let participation = Shared::new(Participation::default());
    let replica = ViewReplica::load(savegame, &participation)?;
    {
        let mut p = participation.borrow_mut();
        p.observers.add(SCENE_USER_ID);
        p.player_names.add(SCENE_USER_ID, "Scene".to_string());
    }
    Ok((replica, Viewer { user_id: SCENE_USER_ID }))
}

/// Which expected differences (protocol.md, Visibility) the C# client shows to this viewer in
/// this state, so the harness masks them: three hidden-information leaks and one C# crash.
fn leaks(game: &Game, viewer: Option<Faction>) -> Vec<&'static str> {
    let mut leaks = Vec::new();

    // 1. BattlePlans.razor:19-25 draws what prescience revealed of a plan for every viewer. The
    //    view gives it to the prescience user's side only.
    if game.current_main_phase == MainPhase::Battle && game.current_phase != Phase::BattleReport {
        let users = game.current_prescience.as_ref().map(|p| p.initiator()).into_iter().chain(game.current_nexus_prescience.as_ref().map(|p| p.initiator()));
        let sees = |user: Faction| Some(user) == viewer || game.get_player(Some(user)).is_some_and(|p| Some(game.player(p).ally) == viewer);
        let users: Vec<Faction> = users.collect();
        if !users.is_empty() && !users.iter().any(|&u| sees(u)) {
            leaks.push("partial-prescience");
        }
    }

    // 2. The story at game end ignores HideSecrets. The view leaves those factions' secrets out.
    if game.current_main_phase == MainPhase::Ended && !game.secrets_remain_hidden.is_empty() {
        leaks.push("story-hide-secrets");
    }

    // 3. Bids.razor:25 colours a silent-auction bid by Passed. The view sends "ready" only.
    if matches!(game.current_phase, Phase::Bidding | Phase::BlackMarketBidding)
        && matches!(game.current_auction_type, AuctionType::BlackMarketSilent | AuctionType::RicheseSilent)
    {
        leaks.push("silent-auction-pass");
    }

    // 4. Not a leak: for an observer, the C# bidding panel throws (BiddingPanel.razor reads
    //    Player, which is null) and Blazor shows its error bar. The view gives observers the
    //    panel of a player without prescience (protocol.md, deviation 5).
    if viewer.is_none() && matches!(game.current_phase, Phase::Bidding | Phase::BlackMarketBidding) {
        leaks.push("observer-bidding-error");
    }
    leaks
}
