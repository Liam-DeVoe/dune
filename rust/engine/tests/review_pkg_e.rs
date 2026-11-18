//! Bidding: guard tests for message texts that games only show in the log.

use treachery_engine::prelude::*;

#[test]
fn emperor_bid_support_message_lists_the_support() {
    let g = Game::new();
    let mut amounts = IndexMap::new();
    amounts.insert(Faction::Atreides, 3);
    amounts.insert(Faction::Harkonnen, 0);
    amounts.insert(Faction::Fremen, 2);
    let mut e = EmperorBidSupport::default();
    e.amounts = Some(amounts);
    let by = By::new(&g, Faction::Emperor);
    assert_eq!(
        e.describe(&g, by).plain_text(),
        "Emperor supports [Atreides:Treachery.Shared.Payment , Fremen:Treachery.Shared.Payment ]"
    );

    let mut none = IndexMap::new();
    none.insert(Faction::Atreides, 0);
    let mut e = EmperorBidSupport::default();
    e.amounts = Some(none);
    assert_eq!(e.describe(&g, by).plain_text(), "Emperor doesn't support bids by other factions");
}

/// The silent-auction log of a Richese bid and a black market bid.
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

/// `Bureaucracy` logs its message as the only element of a new message.
#[test]
fn bureaucracy_dynamic_message_nested() {
    let inner = express!(Faction::Atreides, " apply Bureaucracy → ", Faction::Emperor, " lose ", Payment::of(2));
    let outer = Message::express(vec![inner.into()]);
    assert_eq!(outer.plain_text(), "Atreides apply Bureaucracy → Emperor lose Treachery.Shared.Payment");
}
