//! Focused tests for package C (setup). Expected values follow the C# code in Game_Setup.cs and
//! the setup events; the comments say which C# behaviour each test pins down.

use std::rc::Rc;
use treachery_engine::cs;
use treachery_engine::prelude::*;

fn game_with(factions: &[Faction]) -> Game {
    let mut g = Game::new();
    for (seat, f) in factions.iter().enumerate() {
        let mut p = Player::new(&g, *f);
        p.seat = seat as i32;
        g.players.push(p);
    }
    g
}

fn idx(g: &Game, f: Faction) -> PlayerIndex {
    g.get_player(Some(f)).unwrap()
}

fn exception_text<R>(f: impl FnOnce() -> R) -> String {
    match cs::catch(f) {
        Ok(_) => "no exception".to_string(),
        Err(e) => format!("{}: {}", e.type_name, e.message),
    }
}

fn settings(n: i32, factions: &[Faction], rules: &[Rule]) -> Shared<GameSettings> {
    let mut s = GameSettings::new();
    s.number_of_players = n;
    s.allowed_factions_in_play = Some(Shared::new(factions.to_vec()));
    s.initial_rules = Some(rules.to_vec());
    Shared::new(s)
}

// ---------------------------------------------------------------- Game_Setup

/// `Players.Select(p => p.Faction).FirstOrDefault(...)` on a value type gives Faction.None.
#[test]
fn next_faction_to_perform_custom_setup_defaults_to_none() {
    let mut g = game_with(&[Faction::Atreides, Faction::Emperor]);
    assert_eq!(g.next_faction_to_perform_custom_setup(), Faction::Atreides);
    g.has_acted_or_passed.push(Faction::Atreides);
    assert_eq!(g.next_faction_to_perform_custom_setup(), Faction::Emperor);
    g.has_acted_or_passed.push(Faction::Emperor);
    assert_eq!(g.next_faction_to_perform_custom_setup(), Faction::None);
}

/// `TreacheryCardsBeforeTraitors` needs version 121 and the LeaderSkills rule.
#[test]
fn treachery_cards_before_traitors_is_version_gated() {
    let mut g = game_with(&[Faction::Atreides]);
    g.rules.push(Rule::LeaderSkills);
    assert!(g.treachery_cards_before_traitors());
    g.version = 120;
    assert!(!g.treachery_cards_before_traitors());
}

/// An empty starting deck throws the Deck.Draw message, after earlier players got their card.
#[test]
fn deal_remaining_starting_cards_throws_on_empty_deck_after_partial_deal() {
    let mut g = game_with(&[Faction::Atreides, Faction::Ixian, Faction::Emperor]);
    let card = TreacheryCardManager::items()[0];
    g.starting_treachery_cards = Some(Deck::new(vec![card], None));
    let text = exception_text(|| g.deal_remaining_starting_treachery_cards_to_non_ixian());
    assert_eq!(text, "InvalidOperationException: Cannot draw from an empty deck");
}

// ---------------------------------------------------------------- MulliganPerformed

/// MayMulligan counts the player's own leaders among their traitors.
#[test]
fn may_mulligan_needs_two_own_leaders() {
    let mut g = game_with(&[Faction::Harkonnen]);
    let me = idx(&g, Faction::Harkonnen);
    let own: Vec<Hero> = LeaderManager::get_leaders(Faction::Harkonnen).take(2).map(Hero::Leader).collect();
    let other = Hero::Leader(LeaderManager::get_leaders(Faction::Emperor).next().unwrap());
    g.players[me.index].traitors = vec![own[0], other];
    assert!(!MulliganPerformed::may_mulligan(&g, me));
    g.players[me.index].traitors.push(own[1]);
    assert!(MulliganPerformed::may_mulligan(&g, me));
}

// ---------------------------------------------------------------- FactionTradeOffered

/// RemoveInvalidTradeOffers removes offers by either trading party, by identity.
#[test]
fn remove_invalid_trade_offers_by_identity() {
    let mut g = game_with(&[Faction::Atreides, Faction::Emperor, Faction::BeneGesserit]);
    let mk = |g: &Game, from: Faction, to: Faction| {
        let mut e = FactionTradeOffered::new(g, from);
        e.target = to;
        Rc::new(e)
    };
    let a = mk(&g, Faction::Atreides, Faction::Emperor);
    let b = mk(&g, Faction::BeneGesserit, Faction::Atreides);
    let c = mk(&g, Faction::Emperor, Faction::BeneGesserit);
    g.current_trade_offers = vec![a.clone(), b.clone(), c.clone()];
    let trade = mk(&g, Faction::Atreides, Faction::Emperor);
    trade.remove_invalid_trade_offers(&mut g);
    // Initiator Atreides or Emperor: a and c go, b (by BeneGesserit) stays.
    assert_eq!(g.current_trade_offers.len(), 1);
    assert!(Rc::ptr_eq(&g.current_trade_offers[0], &b));
}

// ---------------------------------------------------------------- AutomationConfigured

/// Create numbers rules max+1; Delete with an unknown id throws First's message.
#[test]
fn automation_rules_create_and_delete() {
    let mut g = game_with(&[Faction::Atreides]);
    let mk = |g: &Game, action: ItemAction, id: i32| {
        let mut e = AutomationConfigured::new(g, Faction::Atreides);
        e.action = action;
        e.automation_rule_id.set(id);
        Rc::new(e)
    };
    let r1 = mk(&g, ItemAction::Create, -1);
    r1.execute_concrete_event(&mut g);
    let r2 = mk(&g, ItemAction::Create, -1);
    r2.execute_concrete_event(&mut g);
    assert_eq!(r1.automation_rule_id.get(), 0);
    assert_eq!(r2.automation_rule_id.get(), 1);

    mk(&g, ItemAction::Delete, 0).execute_concrete_event(&mut g);
    assert_eq!(g.automation_rules.len(), 1);
    let r3 = mk(&g, ItemAction::Create, -1);
    r3.execute_concrete_event(&mut g);
    assert_eq!(r3.automation_rule_id.get(), 2);

    let text = exception_text(|| mk(&g, ItemAction::Delete, 7).execute_concrete_event(&mut g));
    assert_eq!(text, "InvalidOperationException: Sequence contains no matching element");
}

// ---------------------------------------------------------------- EstablishPlayers

/// Version >= 170 shares the event's settings and factions list with the game (hazards 5.1):
/// FactionSelected before v172 removes through the game and the event sees it.
#[test]
fn establish_players_shares_factions_list() {
    let mut g = Game::new_int32_participation(171, &treachery_engine::cs::Shared::new(Participation::new()));
    let s = settings(
        3,
        &[Faction::Atreides, Faction::Emperor, Faction::BeneGesserit, Faction::Fremen],
        &[Rule::PlayersChooseFactions],
    );
    let mut ev = EstablishPlayers::new(&g, Faction::None);
    ev.seed = 1234;
    ev.settings = Some(s.clone());
    let ev = Rc::new(ev);
    let r = ev.execute(&mut g, false, true);
    assert!(r.is_none());
    assert_eq!(g.players.len(), 3);
    assert_eq!(g.current_phase, Phase::SelectingFactions);
    assert!(g.settings.as_ref().unwrap().ptr_eq(&s));
    g.factions_in_play.borrow_mut().retain(|f| *f != Faction::Atreides);
    assert_eq!(s.borrow().allowed_factions_in_play.as_ref().unwrap().borrow().len(), 3);
}

/// A null InitialRules throws ArgumentNullException from Where, not NullReferenceException.
#[test]
fn establish_players_null_initial_rules() {
    let mut g = Game::new();
    let s = settings(2, &[Faction::Atreides, Faction::Emperor], &[]);
    s.borrow_mut().initial_rules = None;
    let mut ev = EstablishPlayers::new(&g, Faction::None);
    ev.settings = Some(s);
    let ev = Rc::new(ev);
    let text = exception_text(|| ev.execute(&mut g, false, true));
    assert_eq!(text, "ArgumentNullException: Value cannot be null. (Parameter 'source')");
}

/// Same seed, same setup: the whole setup is deterministic and every player gets a seat.
#[test]
fn establish_players_is_deterministic() {
    let run = || {
        let mut g = Game::new();
        let s = settings(4, &[Faction::Atreides, Faction::Emperor, Faction::BeneGesserit, Faction::Fremen, Faction::Harkonnen], &[]);
        let mut ev = EstablishPlayers::new(&g, Faction::None);
        ev.seed = 42;
        ev.settings = Some(s);
        Rc::new(ev).execute(&mut g, false, true);
        let factions: Vec<Faction> = g.players.iter().map(|p| p.faction).collect();
        let seats: Vec<i32> = g.players.iter().map(|p| p.seat).collect();
        (factions, seats, g.current_phase)
    };
    let (f1, s1, p1) = run();
    let (f2, s2, p2) = run();
    assert_eq!((f1.clone(), s1.clone(), p1), (f2, s2, p2));
    let mut sorted = s1.clone();
    sorted.sort();
    assert_eq!(sorted, vec![0, 1, 2, 3]);
    assert!(f1.iter().all(|f| *f != Faction::None));
    assert_eq!(p1, Phase::TradingFactions);
}
