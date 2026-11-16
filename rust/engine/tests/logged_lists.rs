//! Lists of game objects in log messages are copies taken when the message is logged
//! (behavior-changes.md, "Logged lists no longer change after the fact"). Each test logs a
//! message holding one of the game's lists, changes the list, and checks that the message still
//! holds what the list held when it was logged.

use treachery_engine::messages::expression::ExpressionElement;
use treachery_engine::prelude::*;

fn game_with(factions: &[Faction]) -> Game {
    let mut g = Game::new();
    for f in factions {
        g.players.push(Player::new(Some(*f)));
    }
    g
}

fn idx(g: &Game, f: Faction) -> PlayerIndex {
    g.get_player(Some(f)).unwrap()
}

fn card_of(t: TreacheryCardType, nth: usize) -> TreacheryCard {
    TreacheryCardManager::items().iter().copied().filter(|c| c.type_() == t).nth(nth).unwrap()
}

fn cards(cs: &[TreacheryCard]) -> Vec<ExpressionElement> {
    cs.iter().map(|c| ExpressionElement::Hero(Hero::TreacheryCard(*c))).collect()
}

/// The items of the list in the last message logged.
fn last_logged_list(g: &Game) -> Vec<ExpressionElement> {
    let m = g.current_report().messages.last().unwrap();
    m.expression
        .elements
        .iter()
        .find_map(|e| match e {
            ExpressionElement::List { items, .. } => Some(items.clone()),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no list in {m:?}"))
}

/// CHOAM's audit shows the audited cards. The next audit clears and refills
/// `battle.audited_cards`.
#[test]
fn an_audit_message_keeps_the_cards_of_its_audit() {
    let mut g = game_with(&[Faction::Choam, Faction::Emperor]);
    let first = card_of(TreacheryCardType::Laser, 0);
    g.battle.audited_cards = vec![first];

    let mut ev = AuditCancelled::new(&g, Faction::Emperor);
    ev.cancelled = false;
    Rc::new(ev).execute_concrete_event(&mut g).unwrap();
    assert_eq!(g.current_report().messages.last().unwrap().target, Some(Faction::Emperor));
    assert_eq!(last_logged_list(&g), cards(&[first]));

    g.battle.audited_cards = vec![card_of(TreacheryCardType::Shield, 0), card_of(TreacheryCardType::Shield, 1)];
    assert_eq!(last_logged_list(&g), cards(&[first]));
}

/// The Atreides ambassador shows the visitor's hand, which changes as the game goes on.
#[test]
fn an_ambassador_message_keeps_the_hand_it_showed() {
    let mut g = game_with(&[Faction::Ecaz, Faction::Harkonnen]);
    let harkonnen = idx(&g, Faction::Harkonnen);
    let first = card_of(TreacheryCardType::Karma, 0);
    g.player_mut(harkonnen).treachery_cards.push(first);

    let ev = Rc::new(AmbassadorActivated::new(&g, Faction::Ecaz));
    ev.handle_ambassador(&mut g, Ambassador::Atreides, Faction::Harkonnen).unwrap();
    assert_eq!(last_logged_list(&g), cards(&[first]));

    g.player_mut(harkonnen).treachery_cards.push(card_of(TreacheryCardType::Useless, 0));
    assert_eq!(last_logged_list(&g), cards(&[first]));
}

/// A bot that accepts a "tell which traitors were discarded" deal logs its discarded traitors.
/// The set grows when it discards another one.
#[test]
fn a_deal_message_keeps_the_traitors_discarded_so_far() {
    let mut g = game_with(&[Faction::Harkonnen, Faction::Atreides]);
    let harkonnen = idx(&g, Faction::Harkonnen);
    let mut leaders = LeaderManager::get_leaders(Faction::Emperor).map(Hero::Leader);
    let (first, second) = (leaders.next().unwrap(), leaders.next().unwrap());
    g.player_mut(harkonnen).discarded_traitors.insert(first);

    let mut offer = DealOffered::new(&g, Faction::Harkonnen);
    offer.type_ = DealType::TellDiscardedTraitors;
    DealAccepted::new(&g, Faction::Atreides).handle_accepted_bot_deal(&mut g, &Rc::new(offer));
    // The list is in the message to Atreides, before the public "gave the agreed information".
    let reports = g.current_report().messages.clone();
    let to_atreides = reports.iter().rev().find(|m| m.target == Some(Faction::Atreides)).unwrap();
    let logged = |m: &Message| match &m.expression.elements[2] {
        ExpressionElement::List { items, .. } => items.clone(),
        other => panic!("not a list: {other:?}"),
    };
    assert_eq!(logged(to_atreides), vec![ExpressionElement::Hero(first)]);

    g.player_mut(harkonnen).discarded_traitors.insert(second);
    let to_atreides = g.current_report().messages.iter().rev().find(|m| m.target == Some(Faction::Atreides)).unwrap();
    assert_eq!(logged(to_atreides), vec![ExpressionElement::Hero(first)]);
}
