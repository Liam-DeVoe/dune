//! Bidding: guard tests for message texts that games only show in the log.

use treachery_engine::prelude::*;

fn as_player<E: GameEventMethods>(e: E, g: &Game, f: Faction) -> E {
    e.base().initiator.set(Some(f));
    e.base().player.set(g.get_player(Some(f)));
    e
}

/// Probe: `emperor_bid_support.message`, `emperor_bid_support.message_none`.
#[test]
fn emperor_bid_support_message_lists_the_support() {
    let g = Game::new();
    let mut amounts = IndexMap::new();
    amounts.insert(Faction::Atreides, 3);
    amounts.insert(Faction::Harkonnen, 0);
    amounts.insert(Faction::Fremen, 2);
    let mut e = EmperorBidSupport::default();
    e.amounts = Some(amounts);
    let e = as_player(e, &g, Faction::Emperor);
    assert_eq!(
        e.get_message(&g).plain_text(),
        "Emperor supports [Atreides:Treachery.Shared.Payment , Fremen:Treachery.Shared.Payment ]"
    );

    let mut none = IndexMap::new();
    none.insert(Faction::Atreides, 0);
    let mut e = EmperorBidSupport::default();
    e.amounts = Some(none);
    let e = as_player(e, &g, Faction::Emperor);
    assert_eq!(e.get_message(&g).plain_text(), "Emperor doesn't support bids by other factions");
}

/// The silent-auction log of `Bid.HandleRicheseBid` (Bid.cs:296) and
/// `BlackMarketBid.HandleBlackMarketSpecialBid` (BlackMarketBid.cs:117).
///
/// Probe: `silent_bids.richese`, `silent_bids.black_market`.
#[test]
fn silent_bids_log_lists_the_bids() {
    let parts: Vec<MessagePart> = vec![
        express_part!(Faction::Atreides, Payment::of(3), " "),
        express_part!(Faction::Harkonnen, Payment::of(0), " "),
    ];
    let richese = Message::express(vec!["Bids: ".into(), parts.clone().into()]);
    assert_eq!(richese.plain_text(), "Bids: [AtreidesTreachery.Shared.Payment , HarkonnenTreachery.Shared.Payment ]");
    let black_market = Message::express(vec![parts.into()]);
    assert_eq!(black_market.plain_text(), "[AtreidesTreachery.Shared.Payment , HarkonnenTreachery.Shared.Payment ]");
}

/// `Bureaucracy` logs `GetDynamicMessage()` as the only element of a new message
/// (Bureaucracy.cs:39). Probe: `bureaucracy.nested`.
#[test]
fn bureaucracy_dynamic_message_nested() {
    let inner = express!(Faction::Atreides, " apply Bureaucracy → ", Faction::Emperor, " lose ", Payment::of(2));
    let outer = Message::express(vec![inner.into()]);
    assert_eq!(outer.plain_text(), "Atreides apply Bureaucracy → Emperor lose Treachery.Shared.Payment");
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
        let e: HistoryEntry = treachery_engine::json::deserialize(json).expect("event json");
        let Event::EmperorBidSupport(e) = e.event else { panic!("not a EmperorBidSupport") };
        let got: Vec<String> = e.amounts.as_ref().unwrap().iter().map(|(k, v)| format!("{}={}", k, v)).collect();
        assert_eq!(got.join(","), "Atreides=2,Harkonnen=5", "{json}");
    }
}
