//! Edge cases of alliances, deals, discards and the game status.

use treachery_engine::events::HistoryEntry;
use treachery_engine::game::{Game, Seat};

// ReviewA.Establish with AllowedFactionsInPlay [Choam, Atreides, Richese] (same text as ReviewJ.cs).
const ESTABLISH: &str = r#"{"by":null,"at":"2024-01-01T12:02:17Z","event":{"type":"EstablishPlayers","settings":{"numberOfPlayers":3,"maximumTurns":8,"initialRules":["TleilaxuGholas","IxianSwappingCardOnBid","BeneGesseritWorthlessAsKarma","BeneGesseritAutoCharity","AdvancedKarama","BattleWithoutLeader","IncreasedResourceFlow","MoritaniAssassinate","Ssw","GuildDetermineShipment","EmperorSpecialForces","Expansion3TreacheryCards","BribesAreImmediate","FillWithBots","FremenStormLosses","EcazLoyalty","FremenSpecialForces","FullPhaseKarma","AdvancedCombat","BeneGesseritAdvisors","HarkonnenCapturesOrKillsLeaders","FremenDeterminesStorm","ExpansionTreacheryCards","ExtraKaramaCards","AssistedNotekeeping"],"allowedFactionsInPlay":["Choam","Atreides","Richese"],"autoOpenEmptySeats":true,"asyncPlay":false,"asyncPlayMessageIntervalMinutes":0},"seed":1073548723,"maximumNumberOfPlayers":3,"maximumTurns":8,"applicableRules":["TleilaxuGholas","IxianSwappingCardOnBid","BeneGesseritWorthlessAsKarma","BeneGesseritAutoCharity","AdvancedKarama","BattleWithoutLeader","IncreasedResourceFlow","MoritaniAssassinate","Ssw","GuildDetermineShipment","EmperorSpecialForces","Expansion3TreacheryCards","BribesAreImmediate","FillWithBots","FremenStormLosses","EcazLoyalty","FremenSpecialForces","FullPhaseKarma","AdvancedCombat","BeneGesseritAdvisors","HarkonnenCapturesOrKillsLeaders","FremenDeterminesStorm","ExpansionTreacheryCards","ExtraKaramaCards","AssistedNotekeeping"],"players":[],"factionsInPlay":["Choam","Atreides","Richese"]}}"#;


fn new_game() -> Game {
    // User 100 ("Human100") sits in seat 2.
    let mut g = Game::of_version(Game::LATEST_VERSION);
    g.human_seats.insert(Seat::new(2));
    let e = ev(&g, ESTABLISH);
    assert!(g.play_for_message(e, true, true).is_none(), "establish rejected");
    g
}

fn ev(_g: &Game, text: &str) -> HistoryEntry {
    serde_json::from_str(text).expect("event json")
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
        r#"{"by":"Choam","at":"2024-01-01T12:02:40Z","event":{"type":"DealOffered","to":[],"dealType":"None","dealParameter1":null,"dealParameter2":null,"text":"x","endPhase":null,"price":0,"benefit":0,"cancel":false}}"#,
    );
    assert!(g.play_for_message(offer, false, true).is_none());
    assert_eq!(g.deals.offers.len(), 1);

    let cancel = ev(
        &g,
        r#"{"by":"Choam","at":"2024-01-01T12:02:41Z","event":{"type":"DealOffered","to":null,"dealType":"None","dealParameter1":null,"dealParameter2":null,"text":"x","endPhase":null,"price":0,"benefit":0,"cancel":true}}"#,
    );
    let r = g.play_for_message(cancel, false, true);
    assert!(r.is_none(), "C# accepts the cancellation; Rust: {r:?}");
    assert_eq!(g.deals.offers.len(), 0, "C# removes the offer");
    assert_eq!(g.history.entries.len(), 3);
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
        r#"{"by":"Choam","at":"2024-01-01T12:02:40Z","event":{"type":"DealOffered","to":null,"dealType":"None","dealParameter1":null,"dealParameter2":null,"text":"x","endPhase":null,"price":0,"benefit":0,"cancel":false}}"#,
    );
    assert!(g.play_for_message(offer, false, true).is_none());
    assert_eq!(g.deals.offers.len(), 1);

    let cancel = ev(
        &g,
        r#"{"by":"Choam","at":"2024-01-01T12:02:41Z","event":{"type":"DealOffered","to":[],"dealType":"None","dealParameter1":null,"dealParameter2":null,"text":"x","endPhase":null,"price":0,"benefit":0,"cancel":true}}"#,
    );
    let r = g.play_for_message(cancel, false, true);
    assert!(r.is_none(), "C# accepts the cancellation; Rust: {r:?}");
    assert_eq!(g.deals.offers.len(), 0, "C# removes the offer");
}
