//! Every message of real games, rendered with the default skin, against C#.
//!
//! The C# probe (`csharp-ref/Traces.cs`) replays oracle traces and writes, for each trace
//! `X.jsonl.gz`, a file `X.skin.jsonl.gz` with every text it rendered: validation results,
//! `GetMessage`/`GetShortMessage` of each event, every new report message, the `GameStatus`
//! descriptions and flash messages of status queries, and at the end every faction's rules
//! sheet and leader popup. This test replays the same trace with the Rust engine, renders the
//! same texts in the same order, and compares.
//!
//! `trace_corpus_matches_csharp` checks the goldens of `traces/sample` in `tests/golden/traces`.
//! `fresh_trace_corpus_matches_csharp` checks any corpus: set `SKIN_TRACES` to a trace directory
//! and `SKIN_RENDERED` to the probe's output directory for it (see rust/docs/skin.md).

mod common;

use std::path::Path;

use common::*;
use serde_json::Value;
use treachery_engine::compat::cs::{self, Shared};
use treachery_engine::enumerations::*;
use treachery_engine::events::GameEvent;
use treachery_engine::events::GameEventType;
use treachery_engine::game::Game;
use treachery_engine::game::GameStatus;
use treachery_engine::heroes::hero::Hero;
use treachery_engine::heroes::leader_manager::LeaderManager;
use treachery_engine::messages::report::Report;
use treachery_engine::messages::expression::ExpressionElement;
use treachery_skin::{default_skin, Skin};

/// The marker milestone of the oracle's `StepExecutor.Run`.
const SENTINEL: Milestone = Milestone(i32::MIN);

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
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).map_err(|p| cs::panic_message(p.as_ref()))
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

/// Replays one trace and renders what the C# probe renders.
fn render_trace(path: &Path, skin: &Skin) -> Vec<(i64, String, String)> {
    let mut r = Renderer { skin, out: Vec::new(), n: 0, seen: Vec::new() };
    let mut game: Option<Game> = None;
    for text in read_lines(path) {
        r.n += 1;
        let line: Value = serde_json::from_str(&text).unwrap();
        let kind = line["kind"].as_str().unwrap();
        if kind == "header" {
            let participation = differ::replay::participation(line.get("participation")).expect("participation");
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
                let overrides = ev.event_type() == GameEventType::EstablishPlayers;
                if !overrides {
                    g.recent_milestones.push(SENTINEL);
                }
                let result = catch_any(|| {
                    ev.execute(g, line["validate"].as_bool().unwrap(), line["isHost"].as_bool().unwrap())
                });
                if !overrides {
                    if let Some(i) = g.recent_milestones.iter().rposition(|m| *m == SENTINEL) {
                        g.recent_milestones.remove(i);
                    }
                }
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

/// The text of a rendering, without the details of a failure. C# renders a failed event as
/// "Game Error: <message>. Technical description: <exception>." and a call that throws as
/// "!<Type>: <message>". The engine's Game Errors and panics have their own messages, so only
/// the kind of failure is compared.
fn without_error_text(out: &str) -> &str {
    if out.starts_with("Game Error: ") {
        return "Game Error";
    }
    if let Some((name, _)) = out.strip_prefix('!').and_then(|rest| rest.split_once(": ")) {
        if name == "panic" || name.ends_with("Exception") {
            return "!exception";
        }
    }
    out
}

/// Traces that differ from C# on purpose, with the first line (`n`) that differs. Each is a C# bug
/// fixed in Rust (rust/docs/escapes.md, "Automatic bid passes that never stop" for 00009 and "A
/// card trade offered to a faction that isn't the ally" for 00045).
const DELIBERATE: &[(&str, i64)] = &[("fuzz-2026-00009", 602), ("fuzz-2026-00045", 857)];

/// The C# text with the text fixes of rust/docs/behavior-changes.md applied ("Missing spaces",
/// "Setting an automation rule", "Other small text quirks"), so the rest of each text is still
/// compared. `actual` is needed for the automation rule descriptions, which C# doesn't have.
fn with_text_fixes(expected: &str, actual: &str) -> String {
    if expected.contains(" change ally permissions") && actual.contains(" an automation rule") {
        return actual.to_string();
    }
    let mut e = expected.replace("from receiving", "from receiving ").replace("CHOAMUsing Karama", "CHOAM using Karama");
    // Nexus card plays named the card type twice ("Nexus Cunning to Cunning prevent ...") or
    // started their text with a space.
    for t in ["Secret Ally", "Cunning", "Betrayal"] {
        let fixed = format!("Nexus {t} to ");
        for prefix in ["Secret Ally to ", "Secret Allyto ", "Secret Ally", "Cunningto ", "Cunning ", "Cunning", "cunningto ", " "] {
            e = e.replace(&format!("{fixed}{prefix}"), &fixed);
        }
    }
    e
}

/// Compares the probe's renderings for every trace in `traces` that has a file in `rendered`.
fn check_corpus(traces: &Path, rendered: &Path) {
    let skin = default_skin();
    let mut diffs = Diffs::new();
    let mut files = 0;
    let mut entries: Vec<_> = std::fs::read_dir(rendered).unwrap().map(|e| e.unwrap().path()).collect();
    entries.sort();
    for golden_file in entries {
        let name = golden_file.file_name().unwrap().to_str().unwrap().to_string();
        let Some(stem) = name.strip_suffix(".skin.jsonl.gz") else { continue };
        let trace = [format!("{stem}.jsonl.gz"), format!("{stem}.jsonl")].into_iter().map(|t| traces.join(t)).find(|p| p.exists());
        let Some(trace) = trace else { panic!("no trace for {name} in {}", traces.display()) };
        files += 1;
        // A trace that differs from C# on purpose is compared up to the line where it starts to differ.
        let stop = DELIBERATE.iter().find(|(s, _)| *s == stem).map_or(i64::MAX, |(_, n)| *n);
        let expected: Vec<(i64, String, String)> = read_lines(&golden_file)
            .iter()
            .map(|l| {
                let v: Value = serde_json::from_str(l).unwrap();
                (v["n"].as_i64().unwrap(), v["what"].as_str().unwrap().to_string(), v["out"].as_str().unwrap().to_string())
            })
            .filter(|(n, _, _)| *n < stop)
            .collect();
        let mut actual = render_trace(&trace, skin);
        actual.retain(|(n, _, _)| *n < stop);
        let len = expected.len().max(actual.len());
        for i in 0..len {
            let key = |x: Option<&(i64, String, String)>| x.map(|(n, w, _)| format!("line {n} {w}")).unwrap_or("(missing)".into());
            let (e, a) = (expected.get(i), actual.get(i));
            if key(e) != key(a) {
                diffs.check(&format!("{stem} entry {i}"), &key(e), &key(a));
                break;
            }
            let actual = without_error_text(&a.unwrap().2);
            diffs.check(&format!("{stem} {}", key(e)), &with_text_fixes(without_error_text(&e.unwrap().2), actual), actual);
        }
    }
    assert!(files > 0, "no rendered traces in {}", rendered.display());
    diffs.finish(&format!("{files} traces"));
}

#[test]
fn trace_corpus_matches_csharp() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    check_corpus(&root.join("../traces/sample"), &golden("traces"));
}

#[test]
#[ignore = "needs SKIN_TRACES and SKIN_RENDERED; see rust/docs/skin.md"]
fn fresh_trace_corpus_matches_csharp() {
    let traces = std::env::var("SKIN_TRACES").expect("SKIN_TRACES");
    let rendered = std::env::var("SKIN_RENDERED").expect("SKIN_RENDERED");
    check_corpus(Path::new(&traces), Path::new(&rendered));
}
