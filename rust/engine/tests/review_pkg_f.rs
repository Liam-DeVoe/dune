//! Package F review (revival and leaders): tests that show where the Rust port
//! behaves differently from C#. See docs/reviews/pkg-f.md.
//!
//! Expected values come from the C# probe:
//!
//!     cd rust/engine/csharp-ref && dotnet run -c Release -- review-f
//!
//! Each test names the probe line it checks. These tests are meant to FAIL until
//! the difference is fixed.

use treachery_engine::canon;
use treachery_engine::events::GameEvent;
use treachery_engine::game::Game;
use treachery_engine::json;
use treachery_engine::participation::Participation;
use treachery_engine::player::PlayerIndex;

// Step 1 of rust/traces/sample/fuzz-2026-00018.jsonl.gz (same text as ReviewA.cs).
const ESTABLISH: &str = r#"{"$type":"EstablishPlayers","Settings":{"NumberOfPlayers":3,"MaximumTurns":8,"InitialRules":[301,300,65,60,20,1013,15,700,105,80,85,604,90,998,30,701,40,107,10,70,50,25,209,999,112],"AllowedFactionsInPlay":[50,10,120,60,40,100,20,90,70,80,30,110],"AutoOpenEmptySeats":true,"AsyncPlay":false,"AsyncPlayMessageIntervalMinutes":0},"Seed":1073548723,"MaximumNumberOfPlayers":3,"MaximumTurns":8,"ApplicableRules":[301,300,65,60,20,1013,15,700,105,80,85,604,90,998,30,701,40,107,10,70,50,25,209,999,112],"_players":"","_factionsInPlay":"Guild,Fremen,Moritani,BeneGesserit,Emperor,Richese,Atreides,Choam,Ixian,Tleilaxu,Harkonnen,Ecaz","Initiator":0,"Time":"2024-01-01T12:02:17+00:00"}"#;

fn trace_participation() -> Participation {
    let mut p = Participation::new();
    p.player_names.insert(100, "Human100".to_string());
    p.seated_players.insert(100, 2);
    p
}

fn run(events: &[&str]) -> Game {
    let mut g = Game::new_int32_participation(Game::LATEST_VERSION, &treachery_engine::cs::Shared::new(trace_participation()));
    for text in events {
        let e: GameEvent = json::deserialize(text).expect("event json");
        e.initialize_game(&g);
        let r = e.execute(&mut g, true, true);
        assert!(r.is_none(), "event rejected: {text}");
    }
    g
}

/// `SetIncreasedRevivalLimits.ExecuteConcreteEvent` stores `Factions` in the game
/// before `Log()` throws on it. With a null array and no validation (`Game.TryLoad`
/// with `performValidation: false`, or any `Execute(false, ...)`), C# leaves
/// `Game.FactionsWithIncreasedRevivalLimits` null. Rust stores an empty list
/// (`unwrap_or_default`), so the state dump has `[]` where C# has `null`.
///
/// Probe: `sirl_null.result_null = False`, `sirl_null.history = 1`,
/// `sirl_null.limits_is_null = True`, `sirl_null.max_revivals = 3`.
#[test]
fn set_increased_revival_limits_null_factions_without_validation_stores_null() {
    let mut g = run(&[ESTABLISH]);
    let faction = g.players[0].faction;
    let text = format!(
        r#"{{"$type":"SetIncreasedRevivalLimits","Factions":null,"Initiator":{},"Time":"2024-01-01T12:02:40+00:00"}}"#,
        faction.0
    );
    let e: GameEvent = json::deserialize(&text).expect("event json");
    e.initialize_game(&g);
    let r = e.execute(&mut g, false, true);
    assert!(r.is_some(), "C#: Log() throws, so Execute returns a Game Error message");
    assert_eq!(g.history.len(), 1);
    // C# reads the null array through a span, so GetMaxRevivals still returns 3.
    assert_eq!(g.get_max_revivals(&g, g.player_at(1), false), 3);

    let dump = String::from_utf8(canon::dump_game(&g)).unwrap();
    assert!(
        dump.contains(r#""FactionsWithIncreasedRevivalLimits":null"#),
        "C# dumps the null array as null; Rust wrote: {}",
        &dump[dump.find("\"FactionsWithIncreasedRevivalLimits\"").unwrap()..][..60]
    );
}
