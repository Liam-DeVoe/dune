//! Every text the recorded games of `rust/testdata/games` render with the default skin, as a
//! snapshot: validation results, `get_message`/`get_short_message` of each event, every new report
//! message, the `GameStatus` descriptions and flash messages of the status queries in the trace,
//! and at the end every faction's rules sheet and leader popup. The snapshot of game `X.jsonl.gz`
//! is `tests/golden/traces/X.skin.jsonl.gz`, one line per rendered text: the trace line `n` it
//! belongs to, `what` was rendered, and the text `out`.
//!
//! After a deliberate change to these texts, rewrite the snapshots and review the diff:
//!
//!     UPDATE_GOLDENS=1 cargo test --release -p treachery-skin --test trace_corpus

mod common;

use std::path::Path;

use common::*;
use serde_json::Value;
use treachery_engine::shared::Shared;
use treachery_engine::enumerations::*;
use treachery_engine::events::GameEvent;
use treachery_engine::game::{Game, Participation};
use treachery_engine::game::GameStatus;
use treachery_engine::heroes::hero::Hero;
use treachery_engine::heroes::leader_manager::LeaderManager;
use treachery_engine::messages::report::Report;
use treachery_engine::messages::expression::ExpressionElement;
use treachery_skin::{default_skin, Skin};

/// The game's participation from a trace header.
fn participation(o: &Value) -> Participation {
    let int = |v: &Value| v.as_i64().unwrap() as i32;
    let list = |k: &str| o[k].as_array().cloned().unwrap_or_default();
    let mut p = Participation { bots_speed: o["BotsSpeed"].as_i64().unwrap_or(1) as i32, ..Default::default() };
    for kv in list("PlayerNames") {
        p.player_names.insert(int(&kv[0]), kv[1].as_str().unwrap().to_string());
    }
    for kv in list("SeatedPlayers") {
        p.seated_players.insert(int(&kv[0]), int(&kv[1]));
    }
    for (key, set) in [
        ("Observers", &mut p.observers),
        ("Hosts", &mut p.hosts),
        ("AvailableSeats", &mut p.available_seats),
        ("Kicked", &mut p.kicked),
    ] {
        for v in list(key) {
            set.insert(int(&v));
        }
    }
    p
}

struct Renderer<'a> {
    skin: &'a Skin,
    out: Vec<(i64, String, String)>,
    n: i64,
    seen: Vec<usize>,
}

impl Renderer<'_> {
    fn emit(&mut self, what: impl Into<String>, f: impl FnOnce() -> String) {
        let text = outcome(f);
        self.out.push((self.n, what.into(), text));
    }

    fn new_report_messages(&mut self, game: &Game) {
        let reports: Vec<&Report> = std::iter::once(&game.pre_game_report).chain(game.report_list.iter()).collect();
        for (i, r) in reports.iter().enumerate() {
            if self.seen.len() <= i {
                self.seen.push(0);
            }
            for m in r.messages.iter().skip(self.seen[i]) {
                let skin = self.skin;
                self.emit(format!("report {i}"), || skin.message_to_string_in(m, Some(game)));
            }
            self.seen[i] = r.messages.len();
        }
        self.seen.truncate(reports.len());
    }
}

/// Runs `f`, catching any panic: the result, or the panic message.
fn catch_any<R>(f: impl FnOnce() -> R) -> Result<R, String> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).map_err(|p| treachery_engine::error::panic_message(p.as_ref()))
}

fn applicable_calls(game: &mut Game) {
    for i in 0..game.players.len() {
        let p = game.player_at(i);
        let _ = catch_any(|| game.get_applicable_events(Some(p), false));
        let _ = catch_any(|| game.get_applicable_events(Some(p), true));
    }
    let _ = catch_any(|| game.get_applicable_events(None, false));
    let _ = catch_any(|| game.get_applicable_events(None, true));
}

/// Replays one trace and renders its texts.
fn render_trace(path: &Path, skin: &Skin) -> Vec<(i64, String, String)> {
    let mut r = Renderer { skin, out: Vec::new(), n: 0, seen: Vec::new() };
    let mut game: Option<Game> = None;
    for text in read_lines(path) {
        r.n += 1;
        let line: Value = serde_json::from_str(&text).unwrap();
        let kind = line["kind"].as_str().unwrap();
        if kind == "header" {
            let participation = participation(&line["participation"]);
            game = Some(Game::new_int32_participation(line["gameVersion"].as_i64().unwrap() as i32, &Shared::new(participation)));
            continue;
        }
        let Some(g) = game.as_mut() else { break };

        for q in line["queries"].as_array().into_iter().flatten() {
            // A bot deliberation (trace format 2): this test doesn't run bots.
            if q[0].as_str() == Some("bots") {
                continue;
            }
            if q[0].as_str() == Some("status") {
                let p = q[1].as_u64().map(|i| g.player_at(i as usize));
                let is_player = q[2].as_bool().unwrap();
                match catch_any(|| GameStatus::determine_status(g, p, is_player)) {
                    Err(e) => r.emit("status", || format!("!panic: {e}")),
                    Ok(status) => {
                        let gr: &Game = g;
                        r.emit("status awaited", || skin.message_to_string_in(&status.description_when_awaited, Some(gr)));
                        r.emit("status waiting", || skin.message_to_string_in(&status.description_when_waiting, Some(gr)));
                        for fi in &status.flash_info {
                            r.emit("flash", || skin.message_to_string_in(&fi.message, Some(gr)));
                        }
                    }
                }
                continue;
            }
            let p = q[0].as_u64().map(|i| g.player_at(i as usize));
            let is_host = q[1].as_bool().unwrap();
            let _ = catch_any(|| g.get_applicable_events(p, is_host));
        }

        match kind {
            "step" => {
                let ev: GameEvent = treachery_engine::json::deserialize(&line["event"].to_string()).expect("event JSON");
                ev.initialize_game(g);
                g.map.mark_neighbour_cache_stale();
                let result = catch_any(|| {
                    ev.execute(g, line["validate"].as_bool().unwrap(), line["isHost"].as_bool().unwrap())
                });
                let gr: &Game = g;
                match result {
                    Ok(Some(m)) => r.emit("result", || skin.message_to_string_in(&m, Some(gr))),
                    Ok(None) => {}
                    Err(e) => r.emit("escaped", || e),
                }
                r.emit("GetMessage", || skin.message_to_string_in(&ev.get_message(gr), Some(gr)));
                r.emit("GetShortMessage", || skin.message_to_string_in(&ev.get_short_message(gr), Some(gr)));
                r.new_report_messages(gr);
            }
            "undo" => {
                g.undo(line["untilEventNr"].as_i64().unwrap() as i32);
                r.new_report_messages(g);
            }
            "restoreRecentlyUndone" => {
                g.restore_recently_undone();
                r.new_report_messages(g);
            }
            "dismissRecentlyUndone" => {
                g.dismiss_recently_undone();
                r.new_report_messages(g);
            }
            "end" => {
                let reason = line["reason"].as_str().unwrap_or("");
                if !matches!(reason, "hung" | "botHung" | "undoHung" | "memoryLimit" | "botMemoryLimit" | "undoMemoryLimit") {
                    end(&mut r, g);
                }
                return r.out;
            }
            other => panic!("unknown line kind {other}"),
        }
        applicable_calls(g);
    }
    r.out
}

fn end(r: &mut Renderer, game: &Game) {
    let skin = r.skin;
    for &f in Faction::ALL.iter() {
        r.emit(format!("sheet {f}"), || skin.get_faction_info_html(game, f, None));
        r.emit(format!("sheet {f} styled"), || skin.get_faction_info_html(game, f, Some("x")));
        r.emit(format!("sheet parts {f}"), || match skin.get_faction_sheet(game, f) {
            None => "null".to_string(),
            Some(s) => [
                s.at_start.clone(),
                s.resources.to_string(),
                s.free_revival.to_string(),
                s.theme.clone(),
                s.advantages.join(";"),
                s.advanced_advantages.join(";"),
                s.parameters.iter().map(|p: &ExpressionElement| skin.describe(p)).collect::<Vec<_>>().join(";"),
            ]
            .join("|"),
        });
    }
    for l in LeaderManager::leaders() {
        r.emit(format!("popup {}", l.id()), || skin.get_popup_hero_game(Some(Hero::Leader(*l)), game));
    }
    r.emit("popup messiah", || skin.get_popup_hero_game(Some(Hero::Messiah), game));
}

fn line(n: i64, what: &str, out: &str) -> Value {
    serde_json::json!({"n": n, "what": what, "out": out})
}

#[test]
fn trace_corpus_renders_like_the_snapshots() {
    let games = Path::new(env!("CARGO_MANIFEST_DIR")).join("../testdata/games");
    let skin = default_skin();
    let mut diffs = Diffs::new();
    let mut traces: Vec<_> = std::fs::read_dir(&games).unwrap().map(|e| e.unwrap().path()).collect();
    traces.retain(|p| p.to_string_lossy().ends_with(".jsonl.gz"));
    traces.sort();
    assert!(!traces.is_empty(), "no games in {}", games.display());
    for trace in &traces {
        let stem = trace.file_name().unwrap().to_str().unwrap().strip_suffix(".jsonl.gz").unwrap().to_string();
        let snapshot = golden(&format!("traces/{stem}.skin.jsonl.gz"));
        let actual: Vec<Value> = render_trace(trace, skin).iter().map(|(n, w, o)| line(*n, w, o)).collect();
        if updating() {
            write_lines(&snapshot, &actual);
            continue;
        }
        let expected: Vec<Value> = read_lines(&snapshot).iter().map(|l| serde_json::from_str(l).unwrap()).collect();
        let key = |x: Option<&Value>| x.map(|v| format!("line {} {}", v["n"], v["what"].as_str().unwrap())).unwrap_or("(missing)".into());
        for i in 0..expected.len().max(actual.len()) {
            let (e, a) = (expected.get(i), actual.get(i));
            if key(e) != key(a) {
                diffs.check(&format!("{stem} entry {i}"), &key(e), &key(a));
                break;
            }
            diffs.check(&format!("{stem} {}", key(e)), e.unwrap()["out"].as_str().unwrap(), a.unwrap()["out"].as_str().unwrap());
        }
    }
    diffs.finish(&format!("{} traces", traces.len()));
}
