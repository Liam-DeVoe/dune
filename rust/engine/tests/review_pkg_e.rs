//! Package E review (bidding): guard tests for message texts that bot and fuzz
//! traces only see as part of the state hash. See docs/reviews/pkg-e.md.
//!
//! Expected values come from the C# probe:
//!
//!     cd rust/engine/csharp-ref && dotnet run -c Release -- review-e
//!
//! Each test names the probe line it checks.

use treachery_engine::prelude::*;

fn as_player<E: GameEventMethods>(e: E, g: &Game, f: Faction) -> E {
    e.base().initiator.set(f);
    e.base().player.set(g.get_player(Some(f)));
    e
}

/// Probe: `emperor_bid_support.message`, `emperor_bid_support.message_none`.
#[test]
fn emperor_bid_support_message_renders_lazy_query_type() {
    let g = Game::new();
    let mut amounts = IndexMap::new();
    amounts.insert(Faction::Atreides, 3);
    amounts.insert(Faction::Harkonnen, 0);
    amounts.insert(Faction::Fremen, 2);
    let mut e = EmperorBidSupport::default();
    e.amounts = Some(amounts);
    let e = as_player(e, &g, Faction::Emperor);
    assert_eq!(
        e.get_message(&g).to_cs_string(),
        "Emperor supports System.Linq.Enumerable+IEnumerableWhereSelectIterator`2[System.Collections.Generic.KeyValuePair`2[Treachery.Shared.Faction,System.Int32],Treachery.Shared.MessagePart]"
    );

    let mut none = IndexMap::new();
    none.insert(Faction::Atreides, 0);
    let mut e = EmperorBidSupport::default();
    e.amounts = Some(none);
    let e = as_player(e, &g, Faction::Emperor);
    assert_eq!(e.get_message(&g).to_cs_string(), "Emperor doesn't support bids by other factions");
}

/// The silent-auction log of `Bid.HandleRicheseBid` (Bid.cs:296) and
/// `BlackMarketBid.HandleBlackMarketSpecialBid` (BlackMarketBid.cs:117).
///
/// Probe: `silent_bids.richese`, `silent_bids.black_market`.
#[test]
fn silent_bids_log_renders_list_type() {
    let parts: Vec<MessagePart> = vec![
        express_part!(Faction::Atreides, Payment::of(3), " "),
        express_part!(Faction::Harkonnen, Payment::of(0), " "),
    ];
    let richese = Message::express(vec!["Bids: ".into(), parts.clone().into()]);
    assert_eq!(richese.to_cs_string(), "Bids: System.Collections.Generic.List`1[Treachery.Shared.MessagePart]");
    let black_market = Message::express(vec![parts.into()]);
    assert_eq!(black_market.to_cs_string(), "System.Collections.Generic.List`1[Treachery.Shared.MessagePart]");
}

/// `Bureaucracy` logs `GetDynamicMessage()` as the only element of a new message
/// (Bureaucracy.cs:39). Probe: `bureaucracy.nested`.
#[test]
fn bureaucracy_dynamic_message_nested() {
    let inner = express!(Faction::Atreides, " apply Bureaucracy → ", Faction::Emperor, " lose ", Payment::of(2));
    let outer = Message::express(vec![inner.into()]);
    assert_eq!(outer.to_cs_string(), "Atreides apply Bureaucracy → Emperor lose Treachery.Shared.Payment");
}

/// `EmperorBidSupport.Amounts` read from JSON when two keys name the same faction:
/// .NET keeps the first key's position and the last value.
///
/// Probe: `emperor_bid_support.json.dup_case`, `.dup_number`, `.dup_exact` (all `Atreides=2,Harkonnen=5`).
#[test]
fn emperor_bid_support_amounts_duplicate_keys() {
    for json in [
        r#"{"$type":"EmperorBidSupport","Amounts":{"Atreides":1,"Harkonnen":5,"atreides":2},"Initiator":10}"#,
        r#"{"$type":"EmperorBidSupport","Amounts":{"Atreides":1,"Harkonnen":5,"20":2},"Initiator":10}"#,
        r#"{"$type":"EmperorBidSupport","Amounts":{"Atreides":1,"Harkonnen":5,"Atreides":2},"Initiator":10}"#,
    ] {
        let e: GameEvent = treachery_engine::json::deserialize(json).expect("event json");
        let GameEvent::EmperorBidSupport(e) = e else { panic!("not a EmperorBidSupport") };
        let got: Vec<String> = e.amounts.as_ref().unwrap().iter().map(|(k, v)| format!("{}={}", k, v)).collect();
        assert_eq!(got.join(","), "Atreides=2,Harkonnen=5", "{json}");
    }
}
