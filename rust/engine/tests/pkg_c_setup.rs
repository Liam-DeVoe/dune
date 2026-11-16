//! Focused tests for setup. The comments say which behavior each test pins down.

use treachery_engine::error::catch_panic;
use treachery_engine::prelude::*;

fn game_with(factions: &[Faction]) -> Game {
    let mut g = Game::new();
    for (seat, f) in factions.iter().enumerate() {
        let mut p = Player::new(Some(*f));
        p.seat = Seat::from_number(seat);
        g.players.push(p);
    }
    g
}

fn idx(g: &Game, f: Faction) -> PlayerId {
    g.get_player(Some(f)).unwrap()
}

fn exception_text<R>(f: impl FnOnce() -> R) -> String {
    match catch_panic(f) {
        Ok(_) => "no exception".to_string(),
        Err(e) => e,
    }
}

fn settings(n: i32, factions: &[Faction], rules: &[Rule]) -> GameSettings {
    let mut s = GameSettings::new();
    s.number_of_players = n;
    s.allowed_factions_in_play = Some(factions.to_vec());
    s.initial_rules = Some(rules.to_vec());
    s
}

// ---------------------------------------------------------------- Game_Setup

/// `Players.Select(p => p.Faction).FirstOrDefault(...)` on a value type gives Faction.None.
#[test]
fn next_faction_to_perform_custom_setup_defaults_to_none() {
    let mut g = game_with(&[Faction::Atreides, Faction::Emperor]);
    assert_eq!(g.next_faction_to_perform_custom_setup(), Some(Faction::Atreides));
    g.turn.has_acted_or_passed.push(Faction::Atreides);
    assert_eq!(g.next_faction_to_perform_custom_setup(), Some(Faction::Emperor));
    g.turn.has_acted_or_passed.push(Faction::Emperor);
    assert_eq!(g.next_faction_to_perform_custom_setup(), None);
}

/// `TreacheryCardsBeforeTraitors` needs the LeaderSkills rule.
#[test]
fn treachery_cards_before_traitors_needs_leader_skills() {
    let mut g = game_with(&[Faction::Atreides]);
    assert!(!g.treachery_cards_before_traitors());
    g.rules.active.push(Rule::LeaderSkills);
    assert!(g.treachery_cards_before_traitors());
}

/// Running out of starting cards panics (the deck always holds enough in a valid game), after
/// earlier players got their card.
#[test]
fn deal_remaining_starting_cards_panics_on_empty_deck_after_partial_deal() {
    let mut g = game_with(&[Faction::Atreides, Faction::Ixian, Faction::Emperor]);
    let card = TreacheryCardManager::items()[0];
    g.enter_setup(SetupStep::IxianSelectingCard);
    g.setup_mut().unwrap().starting_treachery_cards = Some(Deck::new(vec![card]));
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| g.deal_remaining_starting_treachery_cards_to_non_ixian()));
    assert!(r.is_err());
    assert_eq!(g.players[0].treachery_cards, vec![card]);
    assert!(g.players[2].treachery_cards.is_empty());
}

// ---------------------------------------------------------------- MulliganPerformed

/// MayMulligan counts the player's own leaders among their traitors.
#[test]
fn may_mulligan_needs_two_own_leaders() {
    let mut g = game_with(&[Faction::Harkonnen]);
    let me = idx(&g, Faction::Harkonnen);
    let own: Vec<Hero> = LeaderManager::get_leaders(Faction::Harkonnen).take(2).map(Hero::Leader).collect();
    let other = Hero::Leader(LeaderManager::get_leaders(Faction::Emperor).next().unwrap());
    g.players[me.index()].traitors = vec![own[0], other];
    assert!(!MulliganPerformed::may_mulligan(&g, me));
    g.players[me.index()].traitors.push(own[1]);
    assert!(MulliganPerformed::may_mulligan(&g, me));
}

// ---------------------------------------------------------------- FactionTradeOffered

/// RemoveInvalidTradeOffers removes the offers by either trading party.
#[test]
fn remove_invalid_trade_offers_by_either_party() {
    let mut g = game_with(&[Faction::Atreides, Faction::Emperor, Faction::BeneGesserit]);
    let mk = |from: Faction, to: Faction| TradeOffer { initiator: Some(from), target: Some(to) };
    let b = mk(Faction::BeneGesserit, Faction::Atreides);
    g.enter_setup(SetupStep::TradingFactions);
    g.setup_mut().unwrap().current_trade_offers = vec![mk(Faction::Atreides, Faction::Emperor), b, mk(Faction::Emperor, Faction::BeneGesserit)];
    let trade = FactionTradeOffered { target: Some(Faction::Emperor) };
    let by = By::new(&g, Faction::Atreides);
    trade.remove_invalid_trade_offers(&mut g, by);
    // Initiator Atreides or Emperor: the first and last go, b (by BeneGesserit) stays.
    assert_eq!(g.setup().unwrap().current_trade_offers, vec![b]);
}

// ---------------------------------------------------------------- AutomationConfigured

/// Create numbers rules max+1; Delete with an unknown id fails the event.
#[test]
fn automation_rules_create_and_delete() {
    let mut g = game_with(&[Faction::Atreides]);
    let mk = |action: ItemAction, id: i32| AutomationConfigured { action, automation_rule_id: id, ..Default::default() };
    let by = By::new(&g, Faction::Atreides);
    let ids = |g: &Game| g.rules.automation_rules.iter().map(|r| r.id).collect::<Vec<_>>();
    mk(ItemAction::Create, -1).apply(&mut g, by).unwrap();
    mk(ItemAction::Create, -1).apply(&mut g, by).unwrap();
    assert_eq!(ids(&g), vec![0, 1]);

    mk(ItemAction::Delete, 0).apply(&mut g, by).unwrap();
    assert_eq!(g.rules.automation_rules.len(), 1);
    mk(ItemAction::Create, -1).apply(&mut g, by).unwrap();
    assert_eq!(ids(&g), vec![1, 2]);

    assert!(mk(ItemAction::Delete, 7).apply(&mut g, by).is_err());
    assert_eq!(g.rules.automation_rules.len(), 2);
}

// ---------------------------------------------------------------- EstablishPlayers

/// The game takes a copy of the event's settings and factions list: a removal through the game
/// leaves the event as it was, so the event is saved as it was submitted.
#[test]
fn establish_players_copies_factions_list() {
    let mut g = Game::new();
    let s = settings(
        3,
        &[Faction::Atreides, Faction::Emperor, Faction::BeneGesserit, Faction::Fremen],
        &[Rule::PlayersChooseFactions],
    );
    let mut ev = EstablishPlayers::default();
    ev.seed = 1234;
    ev.settings = Some(s);
    let r = g.play_for_message(HistoryEntry::new(None, ev.clone()), false, true);
    assert!(r.is_none());
    assert_eq!(g.players.len(), 3);
    assert_eq!(g.phase(), Phase::SelectingFactions);
    assert_eq!(g.rules.factions_in_play.len(), 4);
    g.rules.factions_in_play.retain(|f| Some(*f) != Some(Faction::Atreides));
    assert_eq!(ev.settings.as_ref().unwrap().allowed_factions_in_play.as_ref().unwrap().len(), 4);
}

/// A null InitialRules throws ArgumentNullException from Where, not NullReferenceException.
#[test]
fn establish_players_null_initial_rules() {
    let mut g = Game::new();
    let mut s = settings(2, &[Faction::Atreides, Faction::Emperor], &[]);
    s.initial_rules = None;
    let mut ev = EstablishPlayers::default();
    ev.settings = Some(s);
    let text = exception_text(|| g.play_for_message(HistoryEntry::new(None, ev), false, true));
    assert_eq!(text, "rules");
}

/// Same seed, same setup: the whole setup is deterministic and every player gets a seat.
#[test]
fn establish_players_is_deterministic() {
    let run = || {
        let mut g = Game::new();
        let s = settings(4, &[Faction::Atreides, Faction::Emperor, Faction::BeneGesserit, Faction::Fremen, Faction::Harkonnen], &[]);
        let mut ev = EstablishPlayers::default();
        ev.seed = 42;
        ev.settings = Some(s);
        g.play_for_message(HistoryEntry::new(None, ev), false, true);
        let factions: Vec<Option<Faction>> = g.players.iter().map(|p| p.faction).collect();
        let seats: Vec<Option<Seat>> = g.players.iter().map(|p| p.seat).collect();
        (factions, seats, g.phase())
    };
    let (f1, s1, p1) = run();
    let (f2, s2, p2) = run();
    assert_eq!((f1.clone(), s1.clone(), p1), (f2, s2, p2));
    let mut sorted = s1.clone();
    sorted.sort();
    assert_eq!(sorted, (0..4).map(Seat::from_number).collect::<Vec<_>>());
    assert!(f1.iter().all(|f| *f != None));
    assert_eq!(p1, Phase::TradingFactions);
}
