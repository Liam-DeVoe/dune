//! Package K review (alliances, deals, discards, status): tests that show where the
//! Rust port behaves differently from C#. See docs/reviews/pkg-k.md.
//!
//! Expected values come from the C# probe:
//!
//!     cd rust/engine/csharp-ref && dotnet run -c Release -- review-k
//!
//! (`ReviewK.cs`). Each test names the probe lines it checks. These tests are meant
//! to FAIL until the difference is fixed.

use treachery_engine::events::GameEvent;
use treachery_engine::game::Game;
use treachery_engine::json;
use treachery_engine::game::Participation;

// ReviewA.Establish with AllowedFactionsInPlay [Choam, Atreides, Richese] (same text as ReviewJ.cs).
const ESTABLISH: &str = r#"{"$type":"EstablishPlayers","Settings":{"NumberOfPlayers":3,"MaximumTurns":8,"InitialRules":[301,300,65,60,20,1013,15,700,105,80,85,604,90,998,30,701,40,107,10,70,50,25,209,999,112],"AllowedFactionsInPlay":[90,20,100],"AutoOpenEmptySeats":true,"AsyncPlay":false,"AsyncPlayMessageIntervalMinutes":0},"Seed":1073548723,"MaximumNumberOfPlayers":3,"MaximumTurns":8,"ApplicableRules":[301,300,65,60,20,1013,15,700,105,80,85,604,90,998,30,701,40,107,10,70,50,25,209,999,112],"_players":"","_factionsInPlay":"Choam,Atreides,Richese","Initiator":0,"Time":"2024-01-01T12:02:17+00:00"}"#;


fn new_game() -> Game {
    let mut p = Participation::new();
    p.player_names.insert(100, "Human100".to_string());
    p.seated_players.insert(100, 2);
    let mut g = Game::new_int32_participation(Game::LATEST_VERSION, &treachery_engine::shared::Shared::new(p));
    let e = ev(&g, ESTABLISH);
    assert!(e.execute(&mut g, true, true).is_none(), "establish rejected");
    g
}

fn ev(g: &Game, text: &str) -> GameEvent {
    let e: GameEvent = json::deserialize(text).expect("event json");
    e.initialize_game(g);
    e
}

/// `DealOffered.Same` ends with `offeredDeal.To.SequenceEqual(To)`. Both are
/// `Faction[]`, and C# 14 binds that call to `MemoryExtensions.SequenceEqual` on spans,
/// where a null array is an empty span. So a cancellation with `"To":null` matches an
/// offer with `"To":[]` and removes it. Rust ports `Enumerable.SequenceEqual` and throws
/// ArgumentNullException (Parameter 'first'), so the event fails with "Game Error".
///
/// `DealOffered.Validate` always returns null, and deals are applicable in every phase
/// after setup from version 110, so the test executes without validation only to skip
/// the setup phase.
///
/// C#: `Events/DealOffered.cs:87`. Rust: `events/deal_offered.rs` (`same`).
///
/// Probe: `deal_cancel_null_to.offers_before = 1`, `.same = True`,
/// `.cancel_result = null`, `.offers_after = 0`, `.history = 3`.
#[test]
fn deal_cancel_with_null_to_matches_empty_to() {
    let mut g = new_game();
    let offer = ev(
        &g,
        r#"{"$type":"DealOffered","To":[],"Type":0,"Text":"x","EndPhase":0,"Price":0,"Benefit":0,"Cancel":false,"Initiator":90,"Time":"2024-01-01T12:02:40+00:00"}"#,
    );
    assert!(offer.execute(&mut g, false, true).is_none());
    assert_eq!(g.deal_offers.len(), 1);

    let cancel = ev(
        &g,
        r#"{"$type":"DealOffered","To":null,"Type":0,"Text":"x","EndPhase":0,"Price":0,"Benefit":0,"Cancel":true,"Initiator":90,"Time":"2024-01-01T12:02:41+00:00"}"#,
    );
    let r = cancel.execute(&mut g, false, true);
    assert!(r.is_none(), "C# accepts the cancellation; Rust: {r:?}");
    assert_eq!(g.deal_offers.len(), 0, "C# removes the offer");
    assert_eq!(g.history.len(), 3);
}

/// The same `Same` call with the null on the other side: the stored offer has `"To":null`,
/// the cancellation `"To":[]`. C# again treats null as an empty span and removes the offer.
/// Rust throws ArgumentNullException (Parameter 'second').
///
/// C#: `Events/DealOffered.cs:87`. Rust: `events/deal_offered.rs` (`same`).
///
/// Probe: `deal_offer_null_to.offer_result = null`, `.cancel_result = null`,
/// `.offers_after = 0`; `span_binding.contains_on_null_array = False`,
/// `span_binding.sequence_equal_null_null = True`.
#[test]
fn deal_offer_with_null_to_cancelled_by_empty_to() {
    let mut g = new_game();
    let offer = ev(
        &g,
        r#"{"$type":"DealOffered","To":null,"Type":0,"Text":"x","EndPhase":0,"Price":0,"Benefit":0,"Cancel":false,"Initiator":90,"Time":"2024-01-01T12:02:40+00:00"}"#,
    );
    assert!(offer.execute(&mut g, false, true).is_none());
    assert_eq!(g.deal_offers.len(), 1);

    let cancel = ev(
        &g,
        r#"{"$type":"DealOffered","To":[],"Type":0,"Text":"x","EndPhase":0,"Price":0,"Benefit":0,"Cancel":true,"Initiator":90,"Time":"2024-01-01T12:02:41+00:00"}"#,
    );
    let r = cancel.execute(&mut g, false, true);
    assert!(r.is_none(), "C# accepts the cancellation; Rust: {r:?}");
    assert_eq!(g.deal_offers.len(), 0, "C# removes the offer");
}
