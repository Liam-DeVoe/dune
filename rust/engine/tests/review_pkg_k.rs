//! Package K review (alliances, deals, discards, status): tests that show where the
//! Rust port behaves differently from C#. See docs/reviews/pkg-k.md.
//!
//! Expected values come from the C# probe:
//!
//!     cd rust/engine/csharp-ref && dotnet run -c Release -- review-k
//!
//! (`ReviewK.cs`). Each test names the probe lines it checks. These tests are meant
//! to FAIL until the difference is fixed.

use treachery_engine::cs;
use treachery_engine::events::GameEvent;
use treachery_engine::game::Game;
use treachery_engine::json;
use treachery_engine::participation::Participation;
use treachery_engine::prelude::*;
use treachery_engine::treachery_card::TreacheryCard;
use treachery_engine::treachery_card_manager::TreacheryCardManager;

// ReviewA.Establish with AllowedFactionsInPlay [Choam, Atreides, Richese] (same text as ReviewJ.cs).
const ESTABLISH: &str = r#"{"$type":"EstablishPlayers","Settings":{"NumberOfPlayers":3,"MaximumTurns":8,"InitialRules":[301,300,65,60,20,1013,15,700,105,80,85,604,90,998,30,701,40,107,10,70,50,25,209,999,112],"AllowedFactionsInPlay":[90,20,100],"AutoOpenEmptySeats":true,"AsyncPlay":false,"AsyncPlayMessageIntervalMinutes":0},"Seed":1073548723,"MaximumNumberOfPlayers":3,"MaximumTurns":8,"ApplicableRules":[301,300,65,60,20,1013,15,700,105,80,85,604,90,998,30,701,40,107,10,70,50,25,209,999,112],"_players":"","_factionsInPlay":"Choam,Atreides,Richese","Initiator":0,"Time":"2024-01-01T12:02:17+00:00"}"#;

const USELESS: i32 = 28; // probe: useless.id = 28

fn new_game() -> Game {
    let mut p = Participation::new();
    p.player_names.insert(100, "Human100".to_string());
    p.seated_players.insert(100, 2);
    let mut g = Game::new_int32_participation(Game::LATEST_VERSION, &cs::Shared::new(p));
    let e = ev(&g, ESTABLISH);
    assert!(e.execute(&mut g, true, true).is_none(), "establish rejected");
    g
}

fn ev(g: &Game, text: &str) -> GameEvent {
    let e: GameEvent = json::deserialize(text).expect("event json");
    e.initialize_game(g);
    e
}

fn faction_index(g: &Game, f: Faction) -> usize {
    g.players.iter().position(|p| p.faction == f).unwrap()
}

/// `CardTraded.Validate` checks `ValidCards(Player).Contains(Card)`. `ValidCards`
/// returns the hand itself, so `Enumerable.Contains` calls `List<T>.Contains`, which
/// finds a null `Card` (unknown `_cardId`) when the hand holds a null. Rust treats a
/// null card as never contained and returns "Invalid card".
///
/// C#: `Events/CardTraded.cs:55`. Rust: `events/card_traded.rs` (`validate`,
/// `is_some_and`).
///
/// Probe: `card_traded_null.validate = valid`.
#[test]
fn card_traded_null_card_in_hand_is_valid() {
    let mut g = new_game();
    let choam = faction_index(&g, Faction::Choam);
    let atreides = faction_index(&g, Faction::Atreides);
    g.players[choam].ally = Faction::Atreides;
    g.players[atreides].ally = Faction::Choam;
    let useless = TreacheryCardManager::get(USELESS).unwrap();
    g.players[choam].treachery_cards = vec![useless, TreacheryCard::NULL];
    g.players[atreides].treachery_cards = vec![useless];

    let e = ev(
        &g,
        r#"{"$type":"CardTraded","Target":20,"_cardId":-1,"_requestedCardId":-1,"Initiator":90,"Time":"2024-01-01T12:02:40+00:00"}"#,
    );
    let r = cs::catch(|| e.validate(&g).map(|m| format!("{m:?}")));
    assert_eq!(r.as_ref().ok(), Some(&None), "C# Validate returns null (valid); Rust: {r:?}");
}

/// `DiscardedSearched.Validate`: the same `List<T>.Contains(null)` on
/// `TreacheryDiscardPile.Items`. A null gets into the pile through
/// `BattleConcluded.DiscardedCards` with an unknown id (`PutOnTop(c)`).
///
/// C#: `Events/DiscardedSearched.cs:44`. Rust: `events/discarded_searched.rs` (`validate`).
///
/// Probe: `discarded_searched_null.validate = valid`.
#[test]
fn discarded_searched_null_card_in_pile_is_valid() {
    let mut g = new_game();
    g.treachery_discard_pile.as_mut().unwrap().items = vec![TreacheryCard::NULL];

    let e = ev(&g, r#"{"$type":"DiscardedSearched","_cardId":-1,"Initiator":90,"Time":"2024-01-01T12:02:40+00:00"}"#);
    let r = cs::catch(|| e.validate(&g).map(|m| format!("{m:?}")));
    assert_eq!(r.as_ref().ok(), Some(&None), "C# Validate returns null (valid); Rust: {r:?}");
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

/// `MakeAlliance` discards random cards while a hand is over the limit
/// (`Discard(playerA, playerA.TreacheryCards.RandomOrDefault(Random!))`). When the pick is a
/// null card, C# `Discard(Player?, TreacheryCard?)` does nothing and the loop draws again.
/// Rust's `random_or_default` returns `Some(TreacheryCard::NULL)`, which
/// `discard_player_treachery_card_boolean` treats as a card, and
/// `RecentlyDiscarded.Add(null, ...)` throws ArgumentNullException. The package J review
/// listed this site as untested; this is the test.
///
/// Atreides holds cards 0..3 plus four nulls (8 > 4). C# discards exactly the four real cards.
///
/// C#: `Game_Alliances.cs:61,63`. Rust: `game_alliances.rs:32-40`.
///
/// Probe: `make_alliance_null.accept_result = null`, `.atreides_hand = -1,-1,-1,-1`,
/// `.atreides_ally = Choam`, `.discard_pile = 4`.
#[test]
fn make_alliance_hand_limit_skips_null_cards() {
    let mut g = new_game();
    let atreides = faction_index(&g, Faction::Atreides);
    let mut hand: Vec<TreacheryCard> = (0..4).map(|id| TreacheryCardManager::get(id).unwrap()).collect();
    hand.extend(std::iter::repeat(TreacheryCard::NULL).take(4));
    g.players[atreides].treachery_cards = hand;

    let o1 = ev(&g, r#"{"$type":"AllianceOffered","Target":20,"Initiator":90,"Time":"2024-01-01T12:02:40+00:00"}"#);
    assert!(o1.execute(&mut g, false, true).is_none());
    let o2 = ev(&g, r#"{"$type":"AllianceOffered","Target":90,"Initiator":20,"Time":"2024-01-01T12:02:41+00:00"}"#);
    let r = o2.execute(&mut g, false, true);
    assert!(r.is_none(), "C# accepts the alliance; Rust: {r:?}");
    assert!(g.players[atreides].treachery_cards.iter().all(|c| c.is_null()) && g.players[atreides].treachery_cards.len() == 4);
    assert_eq!(g.players[atreides].ally, Faction::Choam);
    assert_eq!(g.treachery_discard_pile.as_ref().unwrap().items.len(), 4);
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
