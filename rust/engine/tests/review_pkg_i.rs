//! Package I review (battle actions around the fight). See docs/reviews/pkg-i.md.
//!
//! Each probe replays the first steps of `traces/sample/bots-2026-00042` (stored in
//! `tests/review_pkg_i/bots42.jsonl`, with the oracle's queries and the after-step
//! `GetApplicableEvents` calls), then executes events the bots never played in that
//! game, and compares the validation result and the SHA-256 of the state dump with C#:
//!
//!     cd rust/engine/csharp-ref && dotnet run -c Release -- review-i
//!
//! (`ReviewI.cs`; set `REVIEW_I_DUMPS=DIR` on both sides to write the dumps for a diff.)
//! The game is played at `Game::LATEST_VERSION`; the two prefix probes check the harness.

use serde_json::Value;
use treachery_engine::canon::{self, Canon};
use treachery_engine::compat::cs::{self, Shared};
use treachery_engine::enumerations::Faction;
use treachery_engine::events::GameEvent;
use treachery_engine::game::Game;
use treachery_engine::json;
use treachery_engine::game::Participation;
use treachery_engine::cards::treachery_card_manager::TreacheryCardManager;

fn steps() -> Vec<Value> {
    let path = format!("{}/tests/review_pkg_i/bots42.jsonl", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(path).unwrap().lines().map(|l| serde_json::from_str(l).unwrap()).collect()
}

fn applicable(g: &mut Game) {
    let count = g.players.len();
    for i in 0..=count {
        let p = if i < count { Some(g.player_at(i)) } else { None };
        for host in [false, true] {
            let _ = catch(|| g.get_applicable_events(p, host));
        }
    }
}

/// Runs `f`, catching any panic: the result, or the panic message.
fn catch<R>(f: impl FnOnce() -> R) -> Result<R, String> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).map_err(|p| cs::panic_message(p.as_ref()))
}

fn exec(g: &mut Game, text: &str, validate: bool, host: bool) -> String {
    if let Some(rest) = text.strip_prefix("HOOK:") {
        hook(g, rest);
        return "null".into();
    }
    let ev: GameEvent = json::deserialize(text).expect("event json");
    ev.initialize_game(g);
    g.map.mark_neighbour_cache_stale();
    match catch(|| ev.execute(g, validate, host)) {
        Ok(None) => "null".into(),
        Ok(Some(m)) => String::from_utf8(canon::dump_detached(&m as &dyn Canon, Some(g))).unwrap(),
        Err(e) => format!("ESCAPED {e}"),
    }
}

/// The state changes `ReviewI.Exec` makes for `HOOK:` lines.
fn hook(g: &mut Game, what: &str) {
    if what == "fd_winner" {
        let tleilaxu = g.get_player(Some(Faction::Tleilaxu)).unwrap();
        let hero = g.winner_hero().unwrap();
        g.player_mut(tleilaxu).face_dancers[0] = hero;
    } else if what == "fd_others_revealed" {
        let tleilaxu = g.get_player(Some(Faction::Tleilaxu)).unwrap();
        let (a, b) = (g.player(tleilaxu).face_dancers[1], g.player(tleilaxu).face_dancers[2]);
        g.player_mut(tleilaxu).revealed_face_dancers.add(a);
        g.player_mut(tleilaxu).revealed_face_dancers.add(b);
    } else if let Some(rest) = what.strip_prefix("givenull:") {
        let f = Faction(rest.parse().unwrap());
        let p = g.get_player(Some(f)).unwrap();
        g.player_mut(p).treachery_cards.push(treachery_engine::cards::treachery_card::TreacheryCard::NULL);
    } else if let Some(rest) = what.strip_prefix("rule:") {
        g.rules.push(treachery_engine::enumerations::Rule(rest.parse().unwrap()));
    } else if let Some(rest) = what.strip_prefix("faction:") {
        let mut parts = rest.split(':');
        let from = Faction(parts.next().unwrap().parse().unwrap());
        let to = Faction(parts.next().unwrap().parse().unwrap());
        let p = g.get_player(Some(from)).unwrap();
        g.player_mut(p).faction = to;
    } else if let Some(rest) = what.strip_prefix("give:") {
        let mut parts = rest.split(':');
        let f = Faction(parts.next().unwrap().parse().unwrap());
        let card = TreacheryCardManager::get(parts.next().unwrap().parse().unwrap()).unwrap();
        let p = g.get_player(Some(f)).unwrap();
        g.player_mut(p).treachery_cards.push(card);
    } else {
        panic!("unknown hook {what}");
    }
}

fn prefix(last: i64) -> Game {
    let mut p = Participation::default();
    p.player_names.add(100, "Human100".to_string());
    p.seated_players.add(100, 0);
    let mut g = Game::new_int32_participation(Game::LATEST_VERSION, &Shared::new(p));
    for s in steps() {
        if s["n"].as_i64().unwrap() > last {
            break;
        }
        for q in s["queries"].as_array().unwrap() {
            let p = q[0].as_u64().map(|i| g.player_at(i as usize));
            let host = q[1].as_bool().unwrap();
            let _ = catch(|| g.get_applicable_events(p, host));
        }
        let text = serde_json::to_string(&s["event"]).unwrap();
        exec(&mut g, &text, s["validate"].as_bool().unwrap(), s["isHost"].as_bool().unwrap());
        applicable(&mut g);
    }
    g
}

/// Replays the trace steps after `after` on a game that has left the trace (`ReviewI.Continue`).
fn continue_trace(g: &mut Game, after: i64) -> String {
    let mut out = String::from("continued:");
    for s in steps() {
        if s["n"].as_i64().unwrap() <= after {
            continue;
        }
        for q in s["queries"].as_array().unwrap() {
            let p = q[0].as_u64().map(|i| g.player_at(i as usize));
            let host = q[1].as_bool().unwrap();
            let _ = catch(|| g.get_applicable_events(p, host));
        }
        let before = g.history.len();
        let text = serde_json::to_string(&s["event"]).unwrap();
        let r = exec(g, &text, s["validate"].as_bool().unwrap(), s["isHost"].as_bool().unwrap());
        out.push(if g.history.len() > before { 'a' } else if r == "null" { '?' } else { 'r' });
        applicable(g);
    }
    out
}

/// Runs a probe and returns (state hash, results joined with " | ").
fn probe(after: i64, events: &[(&str, bool, bool)], name: &str) -> (String, String) {
    let mut g = prefix(after);
    let mut results = Vec::new();
    for (text, validate, host) in events {
        if *text == "CONTINUE" {
            results.push(continue_trace(&mut g, after));
            continue;
        }
        if let Some(n) = text.strip_prefix("CONTINUE:") {
            results.push(continue_trace(&mut g, n.parse().unwrap()));
            continue;
        }
        if *text == "CALL:audit" {
            results.push(match catch(|| treachery_engine::events::AuditCancelled::get_number_of_cards_that_may_be_audited(&g)) {
                Ok(n) => format!("audit {n}"),
                Err(_) => "THROWS".to_string(),
            });
            continue;
        }
        if let Some(f) = text.strip_prefix("CALL:applicable:") {
            let who = g.get_player(Some(Faction(f.parse().unwrap())));
            results.push(match catch(|| g.get_applicable_events(who, false)) {
                Ok(v) => format!("applicable {}", v.iter().map(|t| t.name()).collect::<Vec<_>>().join(",")),
                Err(_) => "THROWS".to_string(),
            });
            continue;
        }
        if let Some(json_text) = text.strip_prefix("VALIDATE:") {
            let ev: GameEvent = json::deserialize(json_text).expect("event json");
            ev.initialize_game(&g);
            results.push(match catch(|| ev.validate(&g)) {
                Ok(None) => "null".into(),
                Ok(Some(m)) => String::from_utf8(canon::dump_detached(&m as &dyn Canon, Some(&g))).unwrap(),
                Err(_) => "THROWS".to_string(),
            });
            continue;
        }
        results.push(exec(&mut g, text, *validate, *host));
        applicable(&mut g);
    }
    let dump = canon::dump_game(&g);
    if let Ok(dir) = std::env::var("REVIEW_I_DUMPS") {
        std::fs::write(format!("{dir}/{name}.rs.json"), &dump).unwrap();
    }
    (sha256_hex(&dump), results.join(" | "))
}

/// Replaces each Game Error message in `results` (" | "-separated) by "GAME ERROR" and each
/// "THROWS <type>: <message>" by "THROWS". The C# results name the .NET exception; the engine
/// reports a GameError with its own message, or panics, so only the kind of failure is compared.
fn without_error_text(results: &str) -> String {
    const GAME_ERROR: &str = r#"{"$type":"Message","Expression":{"$type":"Expression","Elements":["Game Error: ""#;
    results
        .split(" | ")
        .map(|r| if r.starts_with(GAME_ERROR) { "GAME ERROR" } else if r.starts_with("THROWS") { "THROWS" } else { r })
        .collect::<Vec<_>>()
        .join(" | ")
}

fn check(name: &str, after: i64, events: &[(&str, bool, bool)], hash: &str, results: &str) {
    let (h, r) = probe(after, events, name);
    let (r, results) = (without_error_text(&r), without_error_text(results));
    let mut errors = Vec::new();
    if r != results {
        errors.push(format!("results differ\n  C#:   {results}\n  Rust: {r}"));
    }
    if h != hash {
        errors.push(format!("state hash differs (C# {hash}, Rust {h})"));
    }
    assert!(errors.is_empty(), "{name}: {}", errors.join("\n"));
}

const NO_MATCH: &str = r#"{"$type":"Message","Expression":{"$type":"Expression","Elements":["Game Error: ","Sequence contains no matching element",". Technical description: ",{"$type":"$Exception","message":"Sequence contains no matching element","type":"System.InvalidOperationException"},"."]},"Fact":null,"Heading":null,"Log":null,"Plans":null,"Route":null,"Target":"None(0)"}"#;
const NRE: &str = r#"{"$type":"Message","Expression":{"$type":"Expression","Elements":["Game Error: ","Object reference not set to an instance of an object.",". Technical description: ",{"$type":"$Exception","message":"Object reference not set to an instance of an object.","type":"System.NullReferenceException"},"."]},"Fact":null,"Heading":null,"Log":null,"Plans":null,"Route":null,"Target":"None(0)"}"#;

const ECAZ_DIAL: &str = r#"{"$type":"KarmaEcazDial","Initiator":80,"Time":"2024-01-01T21:56:00+00:00"}"#;
const JUICE_AGGRESSOR: &str = r#"{"$type":"JuicePlayed","Type":30,"Initiator":100,"Time":"2024-01-01T21:56:00+00:00"}"#;
const RICHESE_PASSES: &str = r#"{"$type":"TreacheryCalled","TraitorCalled":false,"Initiator":100,"Time":"2024-01-01T21:57:06+00:00"}"#;
const TLEILAXU_PASSES: &str = r#"{"$type":"TreacheryCalled","TraitorCalled":false,"Initiator":80,"Time":"2024-01-01T21:57:07+00:00"}"#;
const FD_REVEAL: &str = r#"{"$type":"FaceDancerRevealed","Passed":false,"Initiator":80,"Time":"2024-01-01T21:59:02+00:00"}"#;
const FD_DANCED: &str = r#"{"$type":"FaceDanced","_targetForceLocations":"8:2|0","ForcesFromReserve":2,"FaceDancerCalled":false,"_toId":-1,"_forceLocations":"","Passed":false,"Initiator":80,"Time":"2024-01-01T21:59:30+00:00"}"#;
const FD_DANCED_SOURCES: &str = r#"{"$type":"FaceDanced","_targetForceLocations":"8:2|0","ForcesFromReserve":1,"FaceDancerCalled":false,"_toId":-1,"_forceLocations":"95:1|0,5:0|0","Passed":false,"Initiator":80,"Time":"2024-01-01T21:59:30+00:00"}"#;

#[test]
fn harness_reproduces_the_trace() {
    check("prefix127", 127, &[], "a990e48d84dcc88bc7c1d69f5da0cd9629a4994ec48411487f025025e1ef51bf", "");
    check("prefix129", 129, &[], "987abf827ba64327b1b21abebfa8d92916c6624518584da95d76cfefabbb78d7", "");
}

#[test]
fn karma_ecaz_dial() {
    check("ecazdial", 127, &[(ECAZ_DIAL, false, true)], "0c4dd5fbbf95b4d2e3c9f20c2c7f1c791374538248bfc333ca4fe883daaca266", "null");
    check(
        "ecazdial_karma",
        127,
        &[("HOOK:give:80:23", false, false), (ECAZ_DIAL, false, true)],
        "14dcc89e273e620db7e8e9276775864ad87387b6715e322358f25c2fcb10a6d5",
        "null | null",
    );
}

#[test]
fn juice_played() {
    check("juice_aggressor", 127, &[(JUICE_AGGRESSOR, false, true)], "a40ba2debfee930b8c16a70e4f431a9af9f40520a953d9863fe522ecd78c2843", NO_MATCH);
    let not_applicable_tleilaxu = r#"{"$type":"Message","Expression":{"$type":"Expression","Elements":["Event '",{"$type":"Message","Expression":{"$type":"Expression","Elements":["Tleilaxu(80)"," don't call treachery"]},"Fact":null,"Heading":null,"Log":null,"Plans":null,"Route":null,"Target":"None(0)"},"' is not applicable"]},"Fact":null,"Heading":null,"Log":null,"Plans":null,"Route":null,"Target":"None(0)"}"#;
    check(
        "juice_swap",
        127,
        &[("HOOK:give:100:48", false, false), (JUICE_AGGRESSOR, false, true), (RICHESE_PASSES, true, false), (TLEILAXU_PASSES, true, false)],
        "29d5fa100de203ff7686a82966b772550be4874984f420396b1a5f962b6e64bd",
        &format!("null | null | null | {not_applicable_tleilaxu}"),
    );
    check(
        "juice_gofirst_at_battle",
        124,
        &[("HOOK:give:50:48", false, false), (r#"{"$type":"JuicePlayed","Type":10,"Initiator":50,"Time":"2024-01-01T21:54:00+00:00"}"#, false, true)],
        "c862112aafee817236e8f9994aaab5898941f0d8be154b139e106fe6cebb25cb",
        "null | null",
    );
}

#[test]
fn residual_played() {
    check("residual", 127, &[(r#"{"$type":"ResidualPlayed","Initiator":100,"Time":"2024-01-01T21:56:00+00:00"}"#, false, true)], "a990e48d84dcc88bc7c1d69f5da0cd9629a4994ec48411487f025025e1ef51bf", NO_MATCH);
    check(
        "residual_richese",
        127,
        &[("HOOK:give:100:54", false, false), (r#"{"$type":"ResidualPlayed","Initiator":100,"Time":"2024-01-01T21:56:00+00:00"}"#, false, true)],
        "b13741f3219fd51895daff2475ece92f8bf0a6df76ca6ad49ba28782d3f37783",
        "null | null",
    );
    check(
        "residual_tleilaxu_card",
        127,
        &[("HOOK:give:80:54", false, false), (r#"{"$type":"ResidualPlayed","Initiator":80,"Time":"2024-01-01T21:56:00+00:00"}"#, false, true)],
        "2527605b3fbf6e598950f96c3fa7e6e7ff2288ed682032796872787d9df74233",
        "null | null",
    );
}

#[test]
fn plan_modifiers() {
    let one = |name: &str, ev: &str, hash: &str| check(name, 127, &[(ev, false, true)], hash, "null");
    one("rock_kill", r#"{"$type":"RockWasMelted","Kill":true,"Initiator":80,"Time":"2024-01-01T21:56:00+00:00"}"#, "170b752d498c21f6a229f0a6a9354d5e5de7e3f9649961033433b7e28b60921a");
    one("antidote", r#"{"$type":"PortableAntidoteUsed","Initiator":80,"Time":"2024-01-01T21:56:00+00:00"}"#, "fb797af958a82cb7040893311e4fce3e168857a31c2d00ec5c64bd8965e6c39b");
    one("diplomacy", r#"{"$type":"Diplomacy","_cardId":32,"Initiator":80,"Time":"2024-01-01T21:56:00+00:00"}"#, "a6c5530d2ad724cb54ad85ad29397cfc77d00df8926fa6ab425888a8040670a7");
    one("karma_prescience", r#"{"$type":"KarmaPrescience","Initiator":100,"Time":"2024-01-01T21:56:00+00:00"}"#, "4e42293ad8ee7b5d4882e5b19acfedbecd9559c47a5e97ffa05438185f4795d2");
    one("voice", r#"{"$type":"Voice","Must":true,"Type":30,"Initiator":60,"Time":"2024-01-01T21:56:00+00:00"}"#, "dc354633de938008ddd28add3d2fe4531b940ca9ceb5684c10685f5f1c7794d4");
    one("poison_tooth", r#"{"$type":"PoisonToothCancelled","Initiator":80,"Time":"2024-01-01T21:56:00+00:00"}"#, "3a4d0794c2b2acabf8cdeccb172375263487aedf21444c1b5dba46ff1381073b");
    one("bene_gesserit_announce", r#"{"$type":"BeneGesseritBattleAnnouncement","_territoryId":6,"Initiator":60,"Time":"2024-01-01T21:56:00+00:00"}"#, "272c9d93aa911141d538f0ce971a2c74ccd42fc36785d884875cc26e21c23ff2");
    one("traitor_discard", r#"{"$type":"TraitorDiscarded","_traitorId":1016,"Initiator":100,"Time":"2024-01-01T21:56:00+00:00"}"#, "70768c6e146ad77c4fe52c33164925bb5acf14078d75f419b0e9ac3f39295234");
}

#[test]
fn thought() {
    check(
        "thought",
        127,
        &[
            (r#"{"$type":"Thought","_cardId":6,"Initiator":80,"Time":"2024-01-01T21:56:00+00:00"}"#, false, true),
            (r#"{"$type":"ThoughtAnswered","_cardId":6,"Initiator":100,"Time":"2024-01-01T21:56:30+00:00"}"#, true, true),
        ],
        "50553d3c523029bcfbebdffa9f5a1954132a97469f9dad933a2b755436e11fae",
        "null | null",
    );
    check("thought_answer_none", 127, &[(r#"{"$type":"ThoughtAnswered","_cardId":6,"Initiator":100,"Time":"2024-01-01T21:56:30+00:00"}"#, false, true)], "a990e48d84dcc88bc7c1d69f5da0cd9629a4994ec48411487f025025e1ef51bf", NRE);
}

#[test]
fn audit() {
    let defender_traitor_null = r#"{"$type":"Message","Expression":{"$type":"Expression","Elements":["Game Error: ","Defender traitor action is null",". Technical description: ",{"$type":"$Exception","message":"Defender traitor action is null","type":"System.NullReferenceException"},"."]},"Fact":null,"Heading":null,"Log":null,"Plans":null,"Route":null,"Target":"None(0)"}"#;
    check("audit_cancel", 127, &[(r#"{"$type":"AuditCancelled","Cancelled":true,"Initiator":100,"Time":"2024-01-01T21:56:00+00:00"}"#, false, true)], "e91fd879ea565e3489b5893cf089afd38e881740ea65900f29eeb7cb742bf6ca", defender_traitor_null);
    check("audit_cancel_after", 129, &[(r#"{"$type":"AuditCancelled","Cancelled":true,"Initiator":80,"Time":"2024-01-01T21:58:50+00:00"}"#, false, true)], "ead1d6bc8a7b62c283ca7808573206d1d4b9e3dd45b836c4f5ee8f3f8f0a5030", "null");
    check("audit_keep_after", 129, &[(r#"{"$type":"AuditCancelled","Cancelled":false,"Initiator":80,"Time":"2024-01-01T21:58:50+00:00"}"#, false, true)], "06d0d6389bcba8eec821d9d30d47a0f4330d05770a49abaa750ae3c012ecc309", "null");
    check("audited_after", 129, &[(r#"{"$type":"Audited","Initiator":90,"Time":"2024-01-01T21:58:50+00:00"}"#, false, true)], "0a284de9ca9fc2dd2ec7077c4cdb1ec450776cec2a7719f6754cf3cea6a39714", "null");
}

#[test]
fn face_dancers() {
    let cant = r#"{"$type":"Message","Expression":{"$type":"Expression","Elements":["You can't reveal a face dancer"]},"Fact":null,"Heading":null,"Log":null,"Plans":null,"Route":null,"Target":"None(0)"}"#;
    check("fd_reveal", 129, &[(FD_REVEAL, true, false)], "987abf827ba64327b1b21abebfa8d92916c6624518584da95d76cfefabbb78d7", cant);
    check("fd_replace", 129, &[(r#"{"$type":"FaceDancerReplaced","Passed":false,"dancerId":-1,"Initiator":80,"Time":"2024-01-01T21:59:02+00:00"}"#, false, true)], "e720e893771a72bc30e2753347d6399001df95f8c3b2b6babb5010341a39a237", "null");
    check("fd_replace_one", 129, &[(r#"{"$type":"FaceDancerReplaced","Passed":false,"dancerId":1042,"Initiator":80,"Time":"2024-01-01T21:59:02+00:00"}"#, false, true)], "ddd92e06f69a46af629c02bb983f0996ec3a6b16d1cc13414569289de2c3a298", "null");
    check("fd_danced", 129, &[("HOOK:fd_winner", false, false), (FD_REVEAL, true, false), (FD_DANCED, true, false)], "58b814a263e24c4160492ba30524dc881523a4647e7f88ce8cc548e4bbb3ea0c", "null | null | null");
    check(
        "fd_danced_sources",
        129,
        &[("HOOK:fd_winner", false, false), (FD_REVEAL, false, true), (FD_DANCED_SOURCES, false, true)],
        "6773014563e1dd86d90e0c983995f20840026fd6306a77747b1efb94612da2e1",
        "null | null | null",
    );
    check(
        "fd_danced_last",
        129,
        &[("HOOK:fd_winner", false, false), ("HOOK:fd_others_revealed", false, false), (FD_REVEAL, true, false), (FD_DANCED, true, false)],
        "a293356c376ec4b6f1c1db3a953b10d1b5616f52e7f09e3b7bf8eb37a6521daf",
        "null | null | null | null",
    );
    check("fd_danced_no_match", 129, &[(FD_DANCED, false, true)], "987abf827ba64327b1b21abebfa8d92916c6624518584da95d76cfefabbb78d7", NO_MATCH);
}

fn msg(elements: &str) -> String {
    format!(r#"{{"$type":"Message","Expression":{{"$type":"Expression","Elements":[{elements}]}},"Fact":null,"Heading":null,"Log":null,"Plans":null,"Route":null,"Target":"None(0)"}}"#)
}

#[test]
fn exceptions_keep_partial_state() {
    check("ecazdial_no_battle", 124, &[(r#"{"$type":"KarmaEcazDial","Initiator":80,"Time":"2024-01-01T21:54:00+00:00"}"#, false, true)], "e7671a3b0fa1eb4738c16a6c29f834178499040f55aa6f4f851f82e6ee5cdd65", NRE);
    check(
        "residual_no_battle",
        124,
        &[("HOOK:give:100:54", false, false), (r#"{"$type":"ResidualPlayed","Initiator":100,"Time":"2024-01-01T21:54:00+00:00"}"#, false, true)],
        "6e47be221fdc17ff593471d4242d6a8954ed95f0d506680e6221bda4c584b9dc",
        &format!("null | {NRE}"),
    );
}

#[test]
fn juice_then_the_rest_of_the_trace() {
    check(
        "juice_golast_cont",
        124,
        &[("HOOK:give:50:48", false, false), (r#"{"$type":"JuicePlayed","Type":20,"Initiator":50,"Time":"2024-01-01T21:54:00+00:00"}"#, false, true), ("CONTINUE", false, false)],
        "ee7d7c85b6afff93fcf1c617fcc71441d2142ea6e618fc3c3f2fe1c64edfec9d",
        "null | null | continued:aaaaaaarrrrrrrrrrrrrrrrrr",
    );
    check(
        "juice_gofirst_cont",
        124,
        &[("HOOK:give:80:48", false, false), (r#"{"$type":"JuicePlayed","Type":10,"Initiator":80,"Time":"2024-01-01T21:54:00+00:00"}"#, false, true), ("CONTINUE", false, false)],
        "d3619c2753975fecab2d214e54c7ebba357b5e0ecc8ad0ce083461ca5b6e2521",
        "null | null | continued:aaaaaaaaaaaaaaaaaaaaaaaaa",
    );
    check(
        "juice_swap_cont",
        127,
        &[("HOOK:give:100:48", false, false), (JUICE_AGGRESSOR, false, true), ("CONTINUE", false, false)],
        "55072d3f20b7a084a7dd582d32b326ee05bff4e37daf5b80c8cb46fec1756afd",
        "null | null | continued:aaaaaaaaaaaaaaaaaaaaaa",
    );
    check(
        "juice_aggr_before_battle_cont",
        125,
        &[("HOOK:give:100:48", false, false), (r#"{"$type":"JuicePlayed","Type":30,"Initiator":100,"Time":"2024-01-01T21:54:30+00:00"}"#, false, true), ("CONTINUE", false, false)],
        "34005ec3b860b5bbd17da8983849cc6c005b72dade2514a0f40a9b4398fbd618",
        "null | null | continued:aaaaaaaaaaaaaaaaaaaaaaaa",
    );
}

#[test]
fn face_danced_then_the_rest_of_the_trace() {
    check(
        "fd_danced_cont",
        129,
        &[("HOOK:fd_winner", false, false), (FD_REVEAL, true, false), (FD_DANCED, true, false), ("CONTINUE", false, false)],
        "cb8e5ab5de0262b342f661719008e60cb1814805db43c95e0573d1d244c56d41",
        "null | null | null | continued:raaaaaaaaaaaaaaaaaaa",
    );
}

#[test]
fn validation_messages() {
    let v = |j: &str| format!("VALIDATE:{j}");
    let evs = [
        v(r#"{"$type":"FaceDanced","_targetForceLocations":"8:3|0","ForcesFromReserve":3,"_forceLocations":"","Passed":false,"Initiator":80,"Time":"2024-01-01T21:59:30+00:00"}"#),
        v(r#"{"$type":"FaceDanced","_targetForceLocations":"8:1|0","ForcesFromReserve":2,"_forceLocations":"","Passed":false,"Initiator":80,"Time":"2024-01-01T21:59:30+00:00"}"#),
        v(r#"{"$type":"FaceDanced","_targetForceLocations":"8:x|0","ForcesFromReserve":2,"_forceLocations":"","Passed":false,"Initiator":80,"Time":"2024-01-01T21:59:30+00:00"}"#),
        v(r#"{"$type":"FaceDanced","_targetForceLocations":null,"ForcesFromReserve":2,"_forceLocations":"","Passed":false,"Initiator":80,"Time":"2024-01-01T21:59:30+00:00"}"#),
        v(r#"{"$type":"FaceDanced","_targetForceLocations":"8:2|0","ForcesFromReserve":2147483647,"_forceLocations":"95:1|0","Passed":false,"Initiator":80,"Time":"2024-01-01T21:59:30+00:00"}"#),
        v(r#"{"$type":"FaceDanced","_targetForceLocations":"8:2|0","ForcesFromReserve":2,"_forceLocations":"","Passed":false,"Initiator":100,"Time":"2024-01-01T21:59:30+00:00"}"#),
        v(r#"{"$type":"FaceDancerReplaced","Passed":false,"dancerId":1042,"Initiator":100,"Time":"2024-01-01T21:59:30+00:00"}"#),
        v(r#"{"$type":"FaceDancerReplaced","Passed":false,"dancerId":1001,"Initiator":80,"Time":"2024-01-01T21:59:30+00:00"}"#),
        v(r#"{"$type":"FaceDancerReplaced","Passed":false,"dancerId":9999,"Initiator":80,"Time":"2024-01-01T21:59:30+00:00"}"#),
        v(r#"{"$type":"TraitorDiscarded","_traitorId":1016,"Initiator":80,"Time":"2024-01-01T21:59:30+00:00"}"#),
        v(r#"{"$type":"Voice","Must":true,"Type":30,"Initiator":60,"Time":"2024-01-01T21:59:30+00:00"}"#),
        v(r#"{"$type":"AuditCancelled","Cancelled":true,"Initiator":80,"Time":"2024-01-01T21:59:30+00:00"}"#),
        v(r#"{"$type":"BeneGesseritBattleAnnouncement","_territoryId":4,"Initiator":60,"Time":"2024-01-01T21:59:30+00:00"}"#),
        v(r#"{"$type":"BeneGesseritBattleAnnouncement","_territoryId":4,"Initiator":20,"Time":"2024-01-01T21:59:30+00:00"}"#),
        v(r#"{"$type":"BeneGesseritBattleAnnouncement","_territoryId":999,"Initiator":60,"Time":"2024-01-01T21:59:30+00:00"}"#),
        v(r#"{"$type":"ThoughtAnswered","_cardId":2,"Initiator":100,"Time":"2024-01-01T21:59:30+00:00"}"#),
        v(r#"{"$type":"Thought","_cardId":2,"Initiator":80,"Time":"2024-01-01T21:59:30+00:00"}"#),
        v(r#"{"$type":"Thought","_cardId":41,"Initiator":80,"Time":"2024-01-01T21:59:30+00:00"}"#),
    ];
    let mut events: Vec<(&str, bool, bool)> = vec![("HOOK:fd_winner", false, false), (FD_REVEAL, true, false)];
    events.extend(evs.iter().map(|e| (e.as_str(), false, false)));
    let nre = "THROWS NullReferenceException: Object reference not set to an instance of an object.";
    let expected = [
        "null".to_string(),
        "null".to_string(),
        msg(r#""Place ",2," or less forces""#),
        msg(r#""The amount of forces you selected from the planet and from reserves (",2,") should equal the amount you wish to put in ",{"$ref":"Territory","id":6}," (",1,")""#),
        "THROWS FormatException: The input string 'x' was not in a correct format.".to_string(),
        nre.to_string(),
        msg(r#""The amount of forces you selected from the planet and from reserves (",-2147483648,") should equal the amount you wish to put in ",{"$ref":"Territory","id":6}," (",2,")""#),
        msg(r#""You can't reveal a Face Dancer""#),
        msg(r#""Invalid Face Dancer""#),
        msg(r#""Invalid Face Dancer""#),
        msg(r#""Select a leader""#),
        msg(r#""Invalid traitor""#),
        msg(r#""You cannot use Voice""#),
        "null".to_string(),
        msg(r#""You don't have advisors there""#),
        msg(r#""Your faction can't announce battle""#),
        msg(r#""You don't have advisors there""#),
        nre.to_string(),
        "null".to_string(),
        msg(r#""Invalid card""#),
    ];
    check("fd_validate", 129, &events, "5fe27798d9a9432b3ae283174bd3c7a394adf2a4b5652d7425854d1657370e6b", &expected.join(" | "));
}

// ---- SHA-256 (the engine crate has no hashing dependency) ----

fn sha256_hex(data: &[u8]) -> String {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3,
        0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
        0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13,
        0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
        0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3, 0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208,
        0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
    ];
    let mut h: [u32; 8] = [0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19];
    let mut msg = data.to_vec();
    let bits = (data.len() as u64).wrapping_mul(8);
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bits.to_be_bytes());
    for chunk in msg.chunks(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([chunk[4 * i], chunk[4 * i + 1], chunk[4 * i + 2], chunk[4 * i + 3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16].wrapping_add(s0).wrapping_add(w[i - 7]).wrapping_add(s1);
        }
        let mut v = h;
        for i in 0..64 {
            let s1 = v[4].rotate_right(6) ^ v[4].rotate_right(11) ^ v[4].rotate_right(25);
            let ch = (v[4] & v[5]) ^ (!v[4] & v[6]);
            let t1 = v[7].wrapping_add(s1).wrapping_add(ch).wrapping_add(K[i]).wrapping_add(w[i]);
            let s0 = v[0].rotate_right(2) ^ v[0].rotate_right(13) ^ v[0].rotate_right(22);
            let maj = (v[0] & v[1]) ^ (v[0] & v[2]) ^ (v[1] & v[2]);
            let t2 = s0.wrapping_add(maj);
            v = [t1.wrapping_add(t2), v[0], v[1], v[2], v[3].wrapping_add(t1), v[4], v[5], v[6]];
        }
        for i in 0..8 {
            h[i] = h[i].wrapping_add(v[i]);
        }
    }
    h.iter().map(|x| format!("{x:08x}")).collect()
}
