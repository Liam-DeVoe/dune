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
//! The two prefix probes check the harness: their hashes are the trace's own.

use serde_json::Value;
use treachery_engine::canon::{self, Canon};
use treachery_engine::cs::{self, Shared};
use treachery_engine::enumerations::Faction;
use treachery_engine::events::game_event::GameEventMethods;
use treachery_engine::events::GameEvent;
use treachery_engine::game::Game;
use treachery_engine::json;
use treachery_engine::participation::Participation;
use treachery_engine::treachery_card_manager::TreacheryCardManager;

fn steps() -> Vec<Value> {
    let path = format!("{}/tests/review_pkg_i/bots42.jsonl", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(path).unwrap().lines().map(|l| serde_json::from_str(l).unwrap()).collect()
}

fn applicable(g: &mut Game) {
    let count = g.players.len();
    for i in 0..=count {
        let p = if i < count { Some(g.player_at(i)) } else { None };
        for host in [false, true] {
            let _ = cs::catch(|| g.get_applicable_events(p, host));
        }
    }
}

fn exec(g: &mut Game, text: &str, validate: bool, host: bool) -> String {
    if let Some(rest) = text.strip_prefix("HOOK:") {
        hook(g, rest);
        return "null".into();
    }
    let ev: GameEvent = json::deserialize(text).expect("event json");
    ev.initialize_game(g);
    g.map.mark_neighbour_cache_stale();
    match cs::catch(|| ev.execute(g, validate, host)) {
        Ok(None) => "null".into(),
        Ok(Some(m)) => String::from_utf8(canon::dump_detached(&m as &dyn Canon, Some(g))).unwrap(),
        Err(e) => format!("ESCAPED {}: {}", e.type_name, e.message),
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
        g.player_mut(p).treachery_cards.push(treachery_engine::treachery_card::TreacheryCard::NULL);
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
    let mut g = Game::new_int32_participation(187, &Shared::new(p));
    for s in steps() {
        if s["n"].as_i64().unwrap() > last {
            break;
        }
        for q in s["queries"].as_array().unwrap() {
            let p = q[0].as_u64().map(|i| g.player_at(i as usize));
            let host = q[1].as_bool().unwrap();
            let _ = cs::catch(|| g.get_applicable_events(p, host));
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
            let _ = cs::catch(|| g.get_applicable_events(p, host));
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
            results.push(match cs::catch(|| treachery_engine::events::AuditCancelled::get_number_of_cards_that_may_be_audited(&g)) {
                Ok(n) => format!("audit {n}"),
                Err(e) => format!("THROWS {}: {}", e.type_name, e.message),
            });
            continue;
        }
        if let Some(f) = text.strip_prefix("CALL:applicable:") {
            let who = g.get_player(Some(Faction(f.parse().unwrap())));
            results.push(match cs::catch(|| g.get_applicable_events(who, false)) {
                Ok(v) => format!("applicable {}", v.iter().map(|t| t.name()).collect::<Vec<_>>().join(",")),
                Err(e) => format!("THROWS {}: {}", e.type_name, e.message),
            });
            continue;
        }
        if let Some(json_text) = text.strip_prefix("VALIDATE:") {
            let ev: GameEvent = json::deserialize(json_text).expect("event json");
            ev.initialize_game(&g);
            results.push(match cs::catch(|| ev.validate(&g)) {
                Ok(None) => "null".into(),
                Ok(Some(m)) => String::from_utf8(canon::dump_detached(&m as &dyn Canon, Some(&g))).unwrap(),
                Err(e) => format!("THROWS {}: {}", e.type_name, e.message),
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

fn check(name: &str, after: i64, events: &[(&str, bool, bool)], hash: &str, results: &str) {
    let (h, r) = probe(after, events, name);
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
    check("prefix127", 127, &[], "4f6ba0ca7e9b068bb1c2dbf08a0864d4227dba4c50c07277eaeee37c86c344e2", "");
    check("prefix129", 129, &[], "da4c5b2a774a7ae47159bab31bcc8370d551def80e5163796d6293dfc615ff58", "");
}

#[test]
fn karma_ecaz_dial() {
    check("ecazdial", 127, &[(ECAZ_DIAL, false, true)], "3c6a26a0dcafb070b382fe9986468cdaaaf895f26861ed06b0cdbdda7eda48d7", "null");
    check(
        "ecazdial_karma",
        127,
        &[("HOOK:give:80:23", false, false), (ECAZ_DIAL, false, true)],
        "6ee0f3370368121279673e2bece70129c365595572761b0d804296af332714a6",
        "null | null",
    );
}

#[test]
fn juice_played() {
    check("juice_aggressor", 127, &[(JUICE_AGGRESSOR, false, true)], "162edc0a27feb08ce1f11a2fe7bfdd8919b93237bae38c49632ec4b979af7ad1", NO_MATCH);
    let not_applicable_tleilaxu = r#"{"$type":"Message","Expression":{"$type":"Expression","Elements":["Event '",{"$type":"Message","Expression":{"$type":"Expression","Elements":["Tleilaxu(80)"," don't call treachery"]},"Fact":null,"Heading":null,"Log":null,"Plans":null,"Route":null,"Target":"None(0)"},"' is not applicable"]},"Fact":null,"Heading":null,"Log":null,"Plans":null,"Route":null,"Target":"None(0)"}"#;
    check(
        "juice_swap",
        127,
        &[("HOOK:give:100:48", false, false), (JUICE_AGGRESSOR, false, true), (RICHESE_PASSES, true, false), (TLEILAXU_PASSES, true, false)],
        "14d7ef4f184bb2c57cb7d685a8609c221773374572a74652aad90a81457d397e",
        &format!("null | null | null | {not_applicable_tleilaxu}"),
    );
    check(
        "juice_gofirst_at_battle",
        124,
        &[("HOOK:give:50:48", false, false), (r#"{"$type":"JuicePlayed","Type":10,"Initiator":50,"Time":"2024-01-01T21:54:00+00:00"}"#, false, true)],
        "0d6913373f9a3d6866014bdfa6b8e591f5d8a0099dd3f416a4025e8fdbc62c3a",
        "null | null",
    );
}

#[test]
fn residual_played() {
    check("residual", 127, &[(r#"{"$type":"ResidualPlayed","Initiator":100,"Time":"2024-01-01T21:56:00+00:00"}"#, false, true)], "4f6ba0ca7e9b068bb1c2dbf08a0864d4227dba4c50c07277eaeee37c86c344e2", NO_MATCH);
    check(
        "residual_richese",
        127,
        &[("HOOK:give:100:54", false, false), (r#"{"$type":"ResidualPlayed","Initiator":100,"Time":"2024-01-01T21:56:00+00:00"}"#, false, true)],
        "f37b646519d2bcb72558b4467a40bdd7c5dd2c9d8da0b661ebc7f41462433455",
        "null | null",
    );
    check(
        "residual_tleilaxu_card",
        127,
        &[("HOOK:give:80:54", false, false), (r#"{"$type":"ResidualPlayed","Initiator":80,"Time":"2024-01-01T21:56:00+00:00"}"#, false, true)],
        "0c7345e5291ddcf65802c29c25af37d59e385fc7bfc8f793200fc465665a2f01",
        "null | null",
    );
}

#[test]
fn plan_modifiers() {
    let one = |name: &str, ev: &str, hash: &str| check(name, 127, &[(ev, false, true)], hash, "null");
    one("rock_kill", r#"{"$type":"RockWasMelted","Kill":true,"Initiator":80,"Time":"2024-01-01T21:56:00+00:00"}"#, "5ed90969c16349e2d3e7b2d68fbe3a138d730700e2947289e9868431afb41dc5");
    one("antidote", r#"{"$type":"PortableAntidoteUsed","Initiator":80,"Time":"2024-01-01T21:56:00+00:00"}"#, "623da6d644018680657496443fff53613dee3dc1a3dc4489e46f3e6aa49d713f");
    one("diplomacy", r#"{"$type":"Diplomacy","_cardId":32,"Initiator":80,"Time":"2024-01-01T21:56:00+00:00"}"#, "8df77c04459f8c87e71c6d79adff7ffc216c3342fe4767ed59476a8414251f65");
    one("karma_prescience", r#"{"$type":"KarmaPrescience","Initiator":100,"Time":"2024-01-01T21:56:00+00:00"}"#, "576aabdfdf43161f0c1760f57fcfa9ad8398ee158bff8caa7c2626c6dbe03b8a");
    one("voice", r#"{"$type":"Voice","Must":true,"Type":30,"Initiator":60,"Time":"2024-01-01T21:56:00+00:00"}"#, "c49422471278f1cbcf94ddca22f87e2a222dd7911dd03ea079fd1ff9e71089e7");
    one("poison_tooth", r#"{"$type":"PoisonToothCancelled","Initiator":80,"Time":"2024-01-01T21:56:00+00:00"}"#, "881465a7146c681562d6ca9409e1ecb45925153ffdf5208fee13d4b97feccda4");
    one("bene_gesserit_announce", r#"{"$type":"BeneGesseritBattleAnnouncement","_territoryId":6,"Initiator":60,"Time":"2024-01-01T21:56:00+00:00"}"#, "220914171e09d577741e39dca71fe8a11d87cc209f523b2a87ad9eac5e970d4a");
    one("traitor_discard", r#"{"$type":"TraitorDiscarded","_traitorId":1016,"Initiator":100,"Time":"2024-01-01T21:56:00+00:00"}"#, "73d396f272185962058dbb11ded5f93ab19d13803622ca26e12954b899414943");
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
        "b0073b2afda3974474a4d3dd38852da2dc9d8713875711dcf5ebed4008c41503",
        "null | null",
    );
    check("thought_answer_none", 127, &[(r#"{"$type":"ThoughtAnswered","_cardId":6,"Initiator":100,"Time":"2024-01-01T21:56:30+00:00"}"#, false, true)], "4f6ba0ca7e9b068bb1c2dbf08a0864d4227dba4c50c07277eaeee37c86c344e2", NRE);
}

#[test]
fn audit() {
    let defender_traitor_null = r#"{"$type":"Message","Expression":{"$type":"Expression","Elements":["Game Error: ","Defender traitor action is null",". Technical description: ",{"$type":"$Exception","message":"Defender traitor action is null","type":"System.NullReferenceException"},"."]},"Fact":null,"Heading":null,"Log":null,"Plans":null,"Route":null,"Target":"None(0)"}"#;
    check("audit_cancel", 127, &[(r#"{"$type":"AuditCancelled","Cancelled":true,"Initiator":100,"Time":"2024-01-01T21:56:00+00:00"}"#, false, true)], "28e04b882af32feb8bce521dd87c15019aa566635218f71653a1bf203cc774b3", defender_traitor_null);
    check("audit_cancel_after", 129, &[(r#"{"$type":"AuditCancelled","Cancelled":true,"Initiator":80,"Time":"2024-01-01T21:58:50+00:00"}"#, false, true)], "dc89bb4ab6c5c875bc07784b1d8e794f728302a880b60a5522714aa2cd33a105", "null");
    check("audit_keep_after", 129, &[(r#"{"$type":"AuditCancelled","Cancelled":false,"Initiator":80,"Time":"2024-01-01T21:58:50+00:00"}"#, false, true)], "3c57a3abde1eb3791b078333938c6b0ab1f5eea27be1388332c7f4d90c173f50", "null");
    check("audited_after", 129, &[(r#"{"$type":"Audited","Initiator":90,"Time":"2024-01-01T21:58:50+00:00"}"#, false, true)], "f9dfdbb19497bfa7eb0b75f08c916e8c58fd02422734daafbcbd1f28e3d514d0", "null");
}

#[test]
fn face_dancers() {
    let cant = r#"{"$type":"Message","Expression":{"$type":"Expression","Elements":["You can't reveal a face dancer"]},"Fact":null,"Heading":null,"Log":null,"Plans":null,"Route":null,"Target":"None(0)"}"#;
    check("fd_reveal", 129, &[(FD_REVEAL, true, false)], "da4c5b2a774a7ae47159bab31bcc8370d551def80e5163796d6293dfc615ff58", cant);
    check("fd_replace", 129, &[(r#"{"$type":"FaceDancerReplaced","Passed":false,"dancerId":-1,"Initiator":80,"Time":"2024-01-01T21:59:02+00:00"}"#, false, true)], "42785c942899195f217c456abbcc0288be2c2baa402db14555b5781acf3a5b14", "null");
    check("fd_replace_one", 129, &[(r#"{"$type":"FaceDancerReplaced","Passed":false,"dancerId":1042,"Initiator":80,"Time":"2024-01-01T21:59:02+00:00"}"#, false, true)], "c357a93f5dd692f270e0e2a1727d97cbca07575a0542dac963e2282f04a56871", "null");
    check("fd_danced", 129, &[("HOOK:fd_winner", false, false), (FD_REVEAL, true, false), (FD_DANCED, true, false)], "dff16da180d0fd0fc1fbc8c7259280795d814aa66fa147fa6ab2a9e5f0f8f9c1", "null | null | null");
    check(
        "fd_danced_sources",
        129,
        &[("HOOK:fd_winner", false, false), (FD_REVEAL, false, true), (FD_DANCED_SOURCES, false, true)],
        "aae99f44bd1e1a779358c4e329de2d2b5b3016a075fa5431dabad462a3f2e78c",
        "null | null | null",
    );
    check(
        "fd_danced_last",
        129,
        &[("HOOK:fd_winner", false, false), ("HOOK:fd_others_revealed", false, false), (FD_REVEAL, true, false), (FD_DANCED, true, false)],
        "3e9685cc84ddeae9031e22d47045bd0404b9343dd19756538dcfe8a91197743a",
        "null | null | null | null",
    );
    check("fd_danced_no_match", 129, &[(FD_DANCED, false, true)], "da4c5b2a774a7ae47159bab31bcc8370d551def80e5163796d6293dfc615ff58", NO_MATCH);
}

fn msg(elements: &str) -> String {
    format!(r#"{{"$type":"Message","Expression":{{"$type":"Expression","Elements":[{elements}]}},"Fact":null,"Heading":null,"Log":null,"Plans":null,"Route":null,"Target":"None(0)"}}"#)
}

#[test]
fn exceptions_keep_partial_state() {
    check("ecazdial_no_battle", 124, &[(r#"{"$type":"KarmaEcazDial","Initiator":80,"Time":"2024-01-01T21:54:00+00:00"}"#, false, true)], "a7b64a95ad2658c7434d8989cf2c858f3a5ce07d5ff22b2d7ce9d961eb3f77b4", NRE);
    check(
        "residual_no_battle",
        124,
        &[("HOOK:give:100:54", false, false), (r#"{"$type":"ResidualPlayed","Initiator":100,"Time":"2024-01-01T21:54:00+00:00"}"#, false, true)],
        "171055f2a73ccdf3501e2cbfab88bb73b4fb916e78f57702a2fd6358b5bc7bd6",
        &format!("null | {NRE}"),
    );
}

#[test]
fn juice_then_the_rest_of_the_trace() {
    check(
        "juice_golast_cont",
        124,
        &[("HOOK:give:50:48", false, false), (r#"{"$type":"JuicePlayed","Type":20,"Initiator":50,"Time":"2024-01-01T21:54:00+00:00"}"#, false, true), ("CONTINUE", false, false)],
        "3c135bb5da836b86dc7f63fe840eb971158a7c439e84491398d6aed3b3baaa6d",
        "null | null | continued:aaaaaaarrrrrrrrrrrrrrrrrr",
    );
    check(
        "juice_gofirst_cont",
        124,
        &[("HOOK:give:80:48", false, false), (r#"{"$type":"JuicePlayed","Type":10,"Initiator":80,"Time":"2024-01-01T21:54:00+00:00"}"#, false, true), ("CONTINUE", false, false)],
        "32e772dbfc892281130735d78d9c105b336ded5fdca382414b60bd0d0e4ea6a4",
        "null | null | continued:aaaaaaaaaaaaaaaaaaaaaaaaa",
    );
    check(
        "juice_swap_cont",
        127,
        &[("HOOK:give:100:48", false, false), (JUICE_AGGRESSOR, false, true), ("CONTINUE", false, false)],
        "a8fd1f4798860de7ccae3a03ae65a12e4e277a7d5183a817ba8968c68051187c",
        "null | null | continued:aaaaaaaaaaaaaaaaaaaaaa",
    );
    check(
        "juice_aggr_before_battle_cont",
        125,
        &[("HOOK:give:100:48", false, false), (r#"{"$type":"JuicePlayed","Type":30,"Initiator":100,"Time":"2024-01-01T21:54:30+00:00"}"#, false, true), ("CONTINUE", false, false)],
        "908ddcb3359764300e8274ce89dfe186c5af8a79fc8ac95820e23d631ee13c92",
        "null | null | continued:aaaaaaaaaaaaaaaaaaaaaaaa",
    );
}

#[test]
fn face_danced_then_the_rest_of_the_trace() {
    check(
        "fd_danced_cont",
        129,
        &[("HOOK:fd_winner", false, false), (FD_REVEAL, true, false), (FD_DANCED, true, false), ("CONTINUE", false, false)],
        "04f8543bd31341cab645ebd32953501274d85c4c3bb34eb0ae771ecd34b327ac",
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
    check("fd_validate", 129, &events, "69199ae8f83b994f364da1a0bc52a4d690329af488fb3afea11008b36e207308", &expected.join(" | "));
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
