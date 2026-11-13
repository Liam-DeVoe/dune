//! Package C review (setup events): the static helpers and obsolete getters that the
//! differential traces never call. See docs/reviews/pkg-c.md.
//!
//! Expected values come from the C# probe, saved as tests/golden/review_c.txt:
//!
//!     cd rust/engine/csharp-ref && dotnet run -c Release -- review-c > ../tests/golden/review_c.txt
//!
//! The test rebuilds every "name = value" line from Rust and compares them all.

use treachery_engine::compat::cs::{self, Shared};
use treachery_engine::events::*;
use treachery_engine::game::Game;
use treachery_engine::json;
use treachery_engine::game::Participation;
use treachery_engine::prelude::*;

const EXPECTED: &str = include_str!("golden/review_c.txt");

const T: &str = r#""Time":"2024-01-01T12:00:00+00:00""#;

fn establish() -> String {
    format!(
        r#"{{"$type":"EstablishPlayers","Settings":{{"NumberOfPlayers":4,"MaximumTurns":5,"InitialRules":[998,400,1013],"AllowedFactionsInPlay":[20,50,60,70,110],"AutoOpenEmptySeats":true}},"Seed":31337,"Initiator":0,{T}}}"#
    )
}

fn run(events: &[String]) -> Game {
    let mut g = Game::new_int32_participation(Game::LATEST_VERSION, &Shared::new(Participation::new()));
    for text in events {
        let e: GameEvent = json::deserialize(text).expect("event json");
        e.initialize_game(&g);
        let r = e.execute(&mut g, true, true);
        assert!(r.is_none(), "event rejected: {text}");
    }
    g
}

fn try_str(f: impl FnOnce() -> Option<String>) -> String {
    match cs::catch_panic(f) {
        Ok(Some(s)) => s,
        Ok(None) => "null".to_string(),
        Err(e) => format!("exception:{}:{}", e.type_name, e.message.replace('\n', "\\n")),
    }
}

fn join<X: ToString>(xs: impl IntoIterator<Item = X>) -> String {
    xs.into_iter().map(|x| x.to_string()).collect::<Vec<_>>().join(",")
}

fn locs(xs: Vec<Location>) -> String {
    join(xs.into_iter().map(|l| l.id()))
}

#[test]
fn static_helpers_and_obsolete_getters_match_csharp() {
    let mut out: Vec<String> = Vec::new();
    let mut p = |name: String, value: String| out.push(format!("{name} = {value}"));

    let g = run(&[establish()]);
    p("factions".into(), join(g.players.iter().map(|p| p.faction)));
    p("phase".into(), g.current_phase.to_string());

    for i in 0..g.players.len() {
        let pi = g.player_at(i);
        let f = g.player(pi).faction;
        p(format!("tradeTargets.{f}"), join(FactionTradeOffered::valid_targets(&g, pi).collect::<Vec<_>>()));
        p(format!("ruleTypes.{f}"), join(AutomationConfigured::get_available_rule_types(&g, pi)));
        p(format!("biddingFactions.{f}"), join(AutomationConfigured::get_valid_bidding_factions(&g, pi)));
        p(format!("hmsLocations.{f}"), locs(PerformHmsPlacement::valid_locations(&g, pi).collect::<Vec<_>>()));
        p(
            format!("validLeaders.{f}"),
            join(SkillAssigned::valid_leaders(&g, pi).into_iter().map(|l| l.map_or("null".to_string(), |l| l.id().to_string()))),
        );
    }

    p("validFactions".into(), join(FactionSelected::valid_factions(&g).collect::<Vec<_>>()));
    p("moritaniLocations".into(), locs(PerformMoritaniSetup::valid_locations(&g).collect::<Vec<_>>()));
    p("beneGesseritLocations".into(), locs(PerformBeneGesseritPlacement::valid_locations(&g).collect::<Vec<_>>()));
    p("replaceTargets".into(), join(PlayerReplaced::valid_targets(&g)));
    p("ruleGroups".into(), join(EstablishPlayers::available_rule_groups()));
    p("maxPlayers".into(), EstablishPlayers::get_maximum_number_of_players().to_string());
    p("mustChooseSkills".into(), if SkillAssigned::players_must_choose_leader_skills(&g) { "True" } else { "False" }.into());
    p("nextCustomSetup".into(), g.next_faction_to_perform_custom_setup().to_string());

    let mut rule_types: Vec<AutomationRuleType> = AutomationRuleType::ALL.to_vec();
    rule_types.push(AutomationRuleType(999));
    for faction in [Faction::Atreides, Faction::Ixian] {
        for rt in &rule_types {
            let mut e = AutomationConfigured::new(&g, faction);
            e.rule_type = *rt;
            e.bidding_above_amount = 7;
            e.bidding_winning_faction = Faction::BeneGesserit;
            p(format!("description.{faction}.{}", rt.0), try_str(|| e.get_description(&g).map(|m| m.to_cs_string())));
        }
    }

    for s in ["Atreides, Harkonnen", " Atreides ", "Atreides,", "10", "-5", "+20", "0x10", "Atreides;Harkonnen", "Fremen,fremen", "1e1", " ", "Moritani,Ecaz,None"] {
        let mut ep = EstablishPlayers::default();
        ep.factions_in_play = Some(s.to_string());
        p(format!("factionsInPlay[{s}]"), try_str(|| Some(join(ep.factions_in_play()))));
    }

    for s in ["", ">", "a>>b", "a"] {
        let mut ep = EstablishPlayers::default();
        ep.players = Some(s.to_string());
        p(format!("players[{s}]"), ep.players().iter().map(|x| format!("<{x}>")).collect::<Vec<_>>().join("|"));
    }

    let mut gi = GameInfo::default();
    gi.phase = Phase::AwaitingPlayers;
    gi.max_players = 2;
    gi.seated_players.add(1, 0);
    gi.seated_players.add(2, 1);
    p("gameinfo.canBeJoined.full".into(), if gi.can_be_joined() { "True" } else { "False" }.into());
    p("gameinfo.seat".into(), format!("{},{}", gi.your_current_seat(2), gi.your_current_seat(9)));

    let expected: Vec<&str> = EXPECTED.lines().collect();
    let mut failures = Vec::new();
    for (i, line) in out.iter().enumerate() {
        match expected.get(i) {
            Some(e) if e == line => {}
            e => failures.push(format!("C#:   {}\nRust: {line}", e.unwrap_or(&"<missing>"))),
        }
    }
    if expected.len() != out.len() {
        failures.push(format!("C# wrote {} lines, Rust {}", expected.len(), out.len()));
    }
    assert!(failures.is_empty(), "{} differences:\n{}", failures.len(), failures.join("\n"));
}

// ---------------------------------------------------------------------------------------
// Events executed with performValidation = false (Game.Undo, the client). Expected values
// from `dotnet run -c Release -- review-c validate-off`, saved as
// tests/golden/review_c_validate_off.txt. See docs/reviews/pkg-c.md.

const EXPECTED_VALIDATE_OFF: &str = include_str!("golden/review_c_validate_off.txt");

/// "accepted", "rejected:<message>" or "error", like ReviewC.Exec. The C# golden writes a
/// Game Error as "error:<type>:<message>"; the engine's GameError is compared as "error" (see
/// `validate_off_matches_csharp`). A panic out of `execute` is "escaped:<type>:<message>", which
/// never matches the golden.
fn exec(g: &mut Game, text: &str, validate: bool) -> String {
    let e: GameEvent = json::deserialize(text).expect("event json");
    e.initialize_game(g);
    match cs::catch_panic(|| e.execute(g, validate, true)) {
        Ok(None) => "accepted".to_string(),
        Ok(Some(m)) => {
            for el in &m.expression.elements {
                if let ExpressionElement::Error(_) = el {
                    // The C# golden names the exception; the engine reports a GameError.
                    return "error".to_string();
                }
            }
            format!("rejected:{}", m.to_cs_string())
        }
        Err(x) => format!("escaped:{}:{}", x.type_name, x.message),
    }
}

#[test]
fn validate_off_matches_csharp() {
    let mut out: Vec<String> = Vec::new();

    // PerformHmsPlacement twice: a real target, then none.
    {
        let mut g = run(&[establish()]);
        let first = g.players[0].faction.0;
        let r = exec(&mut g, &format!(r#"{{"$type":"PerformHmsPlacement","_targetId":5,"Initiator":{first},{T}}}"#), false);
        out.push(format!("hms.first = {r}"));
        let target = g.map.location_lookup().find(5).unwrap();
        let hms: Location = g.map.hidden_mobile_stronghold().into();
        out.push(format!(
            "hms.neighbourOfTarget.before = {}",
            if target.neighbours(&g.map).contains(&hms) { "True" } else { "False" }
        ));
        let r = exec(&mut g, &format!(r#"{{"$type":"PerformHmsPlacement","_targetId":-1,"Initiator":{first},{T}}}"#), false);
        out.push(format!("hms.second = {r}"));
        out.push(format!(
            "hms.neighbourOfTarget.after = {}",
            if target.neighbours(&g.map).contains(&hms) { "True" } else { "False" }
        ));
        out.push(format!(
            "hms.attachedTo = {}",
            g.map.hidden_mobile_stronghold().attached_to_location(&g.map).map_or(String::new(), |l| l.id().to_string())
        ));
    }

    // The exception type and message of a Game Error aren't compared.
    let expected: Vec<String> = EXPECTED_VALIDATE_OFF
        .lines()
        .map(|l| match l.split_once(" = error:") {
            Some((name, _)) => format!("{name} = error"),
            None => l.to_string(),
        })
        .collect();
    let mut failures = Vec::new();
    for (i, line) in out.iter().enumerate() {
        match expected.get(i) {
            Some(e) if e == line => {}
            e => failures.push(format!("C#:   {}\nRust: {line}", e.map_or("<missing>", String::as_str))),
        }
    }
    assert!(failures.is_empty(), "{} differences:\n{}", failures.len(), failures.join("\n"));
}
