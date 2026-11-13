//! Messages that refer to a game collection rather than copy it (`LiveList`): the log shows the
//! collection's contents when it is rendered, not when the message was logged. The skin and the
//! views render a live list by reading it again from the game, as `current` does here.

use treachery_engine::messages::expression::{ExpressionElement, LiveList};
use treachery_engine::prelude::*;

/// What a rendering of `e` with `game` shows: a live list's current contents, otherwise the
/// items taken when the message was logged.
fn current(e: &ExpressionElement, game: Option<&Game>) -> Vec<ExpressionElement> {
    match e {
        ExpressionElement::List { items, live, .. } => live.and_then(|l| game.and_then(|g| l.read(g))).unwrap_or_else(|| items.clone()),
        other => panic!("not a list: {other:?}"),
    }
}

/// `DealAccepted` logs the player's discarded traitors itself: a traitor discarded after the
/// message shows up when the message is rendered.
#[test]
fn discarded_traitors_in_a_message_show_the_current_set() {
    let mut g = Game::new();
    g.players.push(Player::new(Faction::Harkonnen));
    let harkonnen = g.get_player(Some(Faction::Harkonnen)).unwrap();
    let mut leaders = LeaderManager::get_leaders(Faction::Emperor).map(Hero::Leader);
    let (first, second) = (leaders.next().unwrap(), leaders.next().unwrap());
    g.player_mut(harkonnen).discarded_traitors.insert(first);

    let live = ExpressionElement::live(&g, LiveList::PlayerDiscardedTraitors(harkonnen));
    assert_eq!(live.to_cs_string(), "System.Collections.Generic.HashSet`1[Treachery.Shared.IHero]");
    assert_eq!(current(&live, Some(&g)), vec![ExpressionElement::Hero(first)]);

    g.player_mut(harkonnen).discarded_traitors.insert(second);
    assert_eq!(current(&live, Some(&g)), vec![ExpressionElement::Hero(first), ExpressionElement::Hero(second)]);
    // Without a game the snapshot taken at log time is shown.
    assert_eq!(current(&live, None), vec![ExpressionElement::Hero(first)]);
}
