//! `Some(Handle::NULL)` must mean the same as `None` everywhere (engine-conventions.md,
//! "Null handles"). In C# both are just null, but Rust code that tests `is_some()` or
//! matches `Some(x)` would treat the null handle as a value.
//!
//! Two checks:
//! - `every_option_handle_parameter_is_normalized` scans the engine sources. Every
//!   function with an `Option<Handle>` parameter must start by mapping a null handle
//!   to `None` (`let x = x.and_then(crate::cs::CsNull::opt);`). New functions are
//!   caught here, and the failure message gives the line to add.
//! - `game_methods_treat_null_like_none` calls `Game` and `Player` methods with
//!   `Some(NULL)` and with `None` on identical games and requires the same result
//!   and the same state dump.

use std::path::Path;

use treachery_engine::canon;
use treachery_engine::cs::{self, CsNull};
use treachery_engine::enumerations::*;
use treachery_engine::events::GameEvent;
use treachery_engine::game::Game;
use treachery_engine::i_hero::Hero;
use treachery_engine::json;
use treachery_engine::leader::Leader;
use treachery_engine::location::Location;
use treachery_engine::participation::Participation;
use treachery_engine::player::PlayerIndex;
use treachery_engine::territory::Territory;
use treachery_engine::treachery_card::TreacheryCard;
use treachery_engine::treachery_card_manager::TreacheryCardManager;

const HANDLES: &[&str] = &["TreacheryCard", "Leader", "Hero", "Location", "Territory", "ResourceCard", "PlayerIndex"];
const NORMALIZE: &str = "and_then(crate::cs::CsNull::opt)";

/// The parameter list of each `fn` with a body, as (name, params, body prefix).
fn functions(text: &str) -> Vec<(String, String, String)> {
    let b = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while let Some(off) = text[i..].find("fn ") {
        let at = i + off;
        i = at + 3;
        if at > 0 && (b[at - 1].is_ascii_alphanumeric() || b[at - 1] == b'_') {
            continue;
        }
        let name: String = text[at + 3..].chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
        if name.is_empty() {
            continue;
        }
        let Some(open) = text[at..].find('(').map(|o| at + o) else { break };
        let mut depth = 0;
        let mut close = open;
        for (k, c) in text[open..].char_indices() {
            match c {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        close = open + k;
                        break;
                    }
                }
                _ => {}
            }
        }
        let rest = &text[close..];
        let semi = rest.find(';').unwrap_or(usize::MAX);
        let brace = rest.find('{').unwrap_or(usize::MAX);
        if brace == usize::MAX || semi < brace {
            continue;
        }
        let body_start = close + brace + 1;
        let body = text[body_start..(body_start + 400).min(text.len())].to_string();
        out.push((name, text[open + 1..close].to_string(), body));
    }
    out
}

fn option_handle_params(params: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0;
    let mut cur = String::new();
    let mut parts = Vec::new();
    for c in params.chars() {
        match c {
            '<' | '(' | '[' => depth += 1,
            '>' | ')' | ']' => depth -= 1,
            _ => {}
        }
        if c == ',' && depth == 0 {
            parts.push(std::mem::take(&mut cur));
        } else {
            cur.push(c);
        }
    }
    parts.push(cur);
    for p in parts {
        let Some((name, ty)) = p.split_once(':') else { continue };
        let name = name.trim().trim_start_matches("mut ").trim();
        let ty: String = ty.chars().filter(|c| !c.is_whitespace()).collect();
        let Some(inner) = ty.strip_prefix("Option<").and_then(|t| t.strip_suffix('>')) else { continue };
        let last = inner.rsplit("::").next().unwrap();
        if HANDLES.contains(&last) {
            out.push(name.to_string());
        }
    }
    out
}

fn rust_files(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    for e in std::fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.is_dir() {
            rust_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

#[test]
fn every_option_handle_parameter_is_normalized() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    rust_files(&src, &mut files);
    let mut missing = Vec::new();
    let mut checked = 0;
    for f in files {
        let rel = f.strip_prefix(&src).unwrap().to_string_lossy().to_string();
        // cs.rs defines the helpers themselves; i_fetcher.rs has no bodies.
        if rel == "cs.rs" || rel == "i_fetcher.rs" {
            continue;
        }
        let text = std::fs::read_to_string(&f).unwrap();
        for (name, params, body) in functions(&text) {
            for p in option_handle_params(&params) {
                checked += 1;
                let ok = body.contains(&format!("let {p} = {p}.{NORMALIZE};")) || body.contains(&format!("let mut {p} = {p}.{NORMALIZE};"));
                if !ok {
                    missing.push(format!("{rel}: fn {name}: add `let {p} = {p}.{NORMALIZE};` at the top"));
                }
            }
        }
    }
    assert!(checked > 100, "only {checked} parameters found: the scanner is broken");
    assert!(missing.is_empty(), "Option<Handle> parameters that don't map a null handle to None:\n{}", missing.join("\n"));
}

// ---- behaviour ----

const ESTABLISH: &str = r#"{"$type":"EstablishPlayers","Settings":{"NumberOfPlayers":3,"MaximumTurns":8,"InitialRules":[301,300,65,60,20,1013,15,700,105,80,85,604,90,998,30,701,40,107,10,70,50,25,209,999,112],"AllowedFactionsInPlay":[90,20,100],"AutoOpenEmptySeats":true,"AsyncPlay":false,"AsyncPlayMessageIntervalMinutes":0},"Seed":1073548723,"MaximumNumberOfPlayers":3,"MaximumTurns":8,"ApplicableRules":[301,300,65,60,20,1013,15,700,105,80,85,604,90,998,30,701,40,107,10,70,50,25,209,999,112],"_players":"","_factionsInPlay":"Choam,Atreides,Richese","Initiator":0,"Time":"2024-01-01T12:02:17+00:00"}"#;

fn new_game() -> Game {
    let mut p = Participation::new();
    p.player_names.insert(100, "Human100".to_string());
    p.seated_players.insert(100, 2);
    let mut g = Game::new_int32_participation(187, &cs::Shared::new(p));
    let e: GameEvent = json::deserialize(ESTABLISH).expect("event json");
    e.initialize_game(&g);
    assert!(e.execute(&mut g, true, true).is_none(), "establish rejected");
    // A null card in a hand, as RicheseGaveCard with an unknown id leaves it.
    let card = TreacheryCardManager::get(28).unwrap();
    g.players[0].treachery_cards = vec![card, TreacheryCard::NULL];
    g
}

type Call = fn(&mut Game, bool) -> String;

fn opt<T: CsNull>(null: bool) -> Option<T> {
    if null { Some(T::NULL) } else { None }
}

#[rustfmt::skip]
fn calls() -> Vec<(&'static str, Call)> {
    vec![
        ("Game.Discard(TreacheryCard?)", |g, n| { g.discard_treachery_card_boolean(opt(n), true); String::new() }),
        ("Game.Discard(Player?, TreacheryCard?)", |g, n| { let p = g.player_at(0); g.discard_player_treachery_card_boolean(Some(p), opt(n), true); String::new() }),
        ("Game.Discard(null Player, card)", |g, n| { let c = TreacheryCardManager::get(28); g.discard_player_treachery_card_boolean(opt(n), c, true); String::new() }),
        ("Game.RegisterKnown(Player, TreacheryCard?)", |g, n| { let p = g.player_at(1); Game::register_known_player_treachery_card(g, p, opt(n)); String::new() }),
        ("Game.PlayKarma", |g, n| { let p = g.player_at(0); g.play_karma(p, opt(n), vec![]); String::new() }),
        ("Game.CollectTueksSietchBonus", |g, n| { let p = g.player_at(0); g.collect_tueks_sietch_bonus(p, opt(n)); String::new() }),
        ("Game.PrepareSkillAssignmentToRevivedLeader", |g, n| { let p = g.player_at(0); g.prepare_skill_assignment_to_revived_leader(p, opt::<Leader>(n)); String::new() }),
        ("Game.LetMonsterAppear", |g, n| { g.let_monster_appear(opt::<Territory>(n), false); String::new() }),
        ("Game.DetermineOccupationLocation", |g, n| { g.determine_occupation_location(opt::<Location>(n)); String::new() }),
        ("Game.RevealCurrentNoField", |g, n| { let p = g.player_at(0); g.reveal_current_no_field(p, opt::<Location>(n)); String::new() }),
        ("Game.IsSkilled", |g, n| g.is_skilled(opt::<Hero>(n)).to_string()),
        ("Game.Skill", |g, n| g.skill(opt::<Hero>(n)).to_string()),
        ("Game.IsInFrontOfShield", |g, n| g.is_in_front_of_shield(opt::<Hero>(n)).to_string()),
        ("Game.ActivateSmugglerIfApplicable", |g, n| { let p = g.player_at(0); let t = g.map.arrakeen().territory(); g.activate_smuggler_if_applicable(p, opt::<Hero>(n), opt::<Hero>(n), t); String::new() }),
        ("Game.GainCollectedResources", |g, n| { let t = g.map.arrakeen().territory(); g.gain_collected_resources(Faction::Atreides, t, 2, opt::<Location>(n), true); String::new() }),
        ("Game.HasStormPrescience", |g, n| g.has_storm_prescience(opt::<PlayerIndex>(n)).to_string()),
        ("Game.HasResourceDeckPrescience", |g, n| g.has_resource_deck_prescience(opt::<PlayerIndex>(n)).to_string()),
        ("Game.AssistedNoteKeepingEnabled", |g, n| g.assisted_note_keeping_enabled(opt::<PlayerIndex>(n)).to_string()),
        ("Game.GetApplicableEvents", |g, n| format!("{:?}", g.get_applicable_events(opt::<PlayerIndex>(n), false).iter().map(|t| t.name()).collect::<Vec<_>>())),
        ("Player.Has(TreacheryCard?)", |g, n| g.players[1].has(opt(n)).to_string()),
        ("Player.ForcesIn(Location?)", |g, n| g.players[0].forces_in(opt(n)).to_string()),
        ("Player.SpecialForcesIn(Location?)", |g, n| g.players[0].special_forces_in(opt(n)).to_string()),
        ("Player.AnyForcesIn(Location?)", |g, n| g.players[0].any_forces_in(opt(n)).to_string()),
        ("Player.OccupyingForcesIn(Territory?)", |g, n| g.players[0].occupying_forces_in(opt(n)).to_string()),
        ("Player.ForcesIn(Territory?)", |g, n| g.players[0].forces_in_territory(opt(n)).to_string()),
        ("Player.SpecialForcesIn(Territory?)", |g, n| g.players[0].special_forces_in_territory(opt(n)).to_string()),
        ("Player.AnyForcesIn(Territory?)", |g, n| g.players[0].any_forces_in_territory(opt(n)).to_string()),
        ("Player.Occupies(Location?)", |g, n| { let g2 = &*g; g2.players[0].occupies(g2, opt(n)).to_string() }),
        ("Player.Occupies(Territory?)", |g, n| { let g2 = &*g; g2.players[0].occupies_territory(g2, opt(n)).to_string() }),
        ("Hero.ValueInCombatAgainst(IHero?)", |g, n| { let _ = g; Hero::Messiah.value_in_combat_against(opt(n)).to_string() }),
    ]
}

fn run(call: Call, null: bool) -> (String, Vec<u8>) {
    let mut g = new_game();
    let r = match cs::catch(std::panic::AssertUnwindSafe(|| call(&mut g, null))) {
        Ok(v) => format!("ok:{v}"),
        Err(e) => format!("{}: {}", e.type_name, e.message),
    };
    (r, canon::dump_game(&g))
}

#[test]
fn game_methods_treat_null_like_none() {
    let mut failures = Vec::new();
    for (name, call) in calls() {
        let (r_none, d_none) = run(call, false);
        let (r_null, d_null) = run(call, true);
        if r_none != r_null {
            failures.push(format!("{name}: None gives {r_none}, Some(NULL) gives {r_null}"));
        } else if d_none != d_null {
            failures.push(format!("{name}: same result ({r_none}) but different state"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// The OrDefault helpers return `None` for a null element, as for an empty source.
#[test]
fn or_default_helpers_map_null_to_none() {
    use treachery_engine::cs::CsIterExt;
    use treachery_engine::extension_methods::RandomExt;
    use treachery_engine::logged_random::SharedRandom;
    let hand = vec![TreacheryCard::NULL];
    let r = SharedRandom::from_seed(1);
    assert_eq!(hand.iter().copied().random_or_default(&r), None);
    assert_eq!(hand.iter().random_or_default(&r), None);
    assert_eq!(hand.iter().copied().cs_first_or_default_where(|_| true), None);
    assert_eq!(hand.iter().copied().cs_single_or_default(), None);
    assert_eq!(hand.iter().copied().cs_single_or_default_where(|_| true), None);
    assert_eq!(hand.iter().copied().cs_last_or_default_where(|_| true), None);
    assert_eq!(cs::last_or_default_where_list(&hand, |_| true), None);
    // The throwing forms still return the null element.
    assert_eq!(hand.iter().copied().cs_single_where(|_| true), TreacheryCard::NULL);
    assert_eq!(hand.iter().copied().cs_last_where(|_| true), TreacheryCard::NULL);
    let heroes = vec![Hero::Leader(Leader::NULL)];
    assert_eq!(heroes.iter().copied().random_or_default(&r), None);
}
