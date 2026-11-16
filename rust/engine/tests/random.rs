//! The game's random number generator and selection helpers (`random.rs`): each helper picks
//! every allowed item about equally often, and a game replays the same way from its seed.
//!
//! The frequency tests use a fixed seed, so they are deterministic; their bounds are loose
//! (about six standard deviations), so they only fail for a real bias, such as an item that is
//! never picked or a shuffle that favours one position.

use std::collections::HashMap;

use treachery_engine::cards::deck::Deck;
use treachery_engine::game::{Game, GameState};
use treachery_engine::random::{RandomPick, SharedRandom};

/// Asserts that every count is within `tolerance` (a fraction) of `expected`.
fn assert_even<K: std::fmt::Debug>(what: &str, counts: &HashMap<K, usize>, cells: usize, expected: f64, tolerance: f64) {
    assert_eq!(counts.len(), cells, "{what}: some outcomes never came up: {counts:?}");
    for (k, &c) in counts {
        let off = (c as f64 - expected).abs() / expected;
        assert!(off <= tolerance, "{what}: {k:?} came up {c} times, expected about {expected}");
    }
}

#[test]
fn shuffle_gives_every_order_about_equally_often() {
    let random = SharedRandom::from_seed(20261113);
    let mut counts: HashMap<Vec<i32>, usize> = HashMap::new();
    let n = 24_000;
    for _ in 0..n {
        let mut deck = Deck::new([1, 2, 3, 4], Some(random.clone()));
        deck.shuffle();
        *counts.entry(deck.items).or_default() += 1;
    }
    // 24 orders, 1000 each; one standard deviation is about 31.
    assert_even("shuffle of 4", &counts, 24, n as f64 / 24.0, 0.2);
}

#[test]
fn shuffle_puts_every_card_on_top_about_equally_often() {
    let random = SharedRandom::from_seed(7);
    let mut counts: HashMap<i32, usize> = HashMap::new();
    let n = 33_000;
    for _ in 0..n {
        let mut deck = Deck::new(0..33, Some(random.clone()));
        deck.shuffle();
        *counts.entry(deck.top().unwrap()).or_default() += 1;
    }
    // 1000 each; one standard deviation is about 31.
    assert_even("top of 33", &counts, 33, 1000.0, 0.2);
}

#[test]
fn random_item_picks_every_item_about_equally_often() {
    let random = SharedRandom::from_seed(1);
    let mut counts: HashMap<char, usize> = HashMap::new();
    let n = 50_000;
    for _ in 0..n {
        let c = "abcde".chars().random_item(&random).unwrap();
        *counts.entry(c).or_default() += 1;
    }
    assert_even("random_item of 5", &counts, 5, 10_000.0, 0.1);
    assert_eq!(std::iter::empty::<i32>().random_item(&random), None);
}

#[test]
fn random_item_draws_only_when_there_is_a_choice() {
    let random = SharedRandom::from_seed(3);
    let before = random.state();
    assert_eq!(std::iter::once(9).random_item(&random), Some(9));
    assert_eq!(std::iter::empty::<i32>().random_item(&random), None);
    assert_eq!(random.state(), before);
}

#[test]
fn random_highest_and_lowest_pick_among_the_ties() {
    let random = SharedRandom::from_seed(5);
    let items = [(0, 1), (1, 3), (2, 3), (3, 2), (4, 3), (5, 1)];
    let mut highest: HashMap<usize, usize> = HashMap::new();
    let mut lowest: HashMap<usize, usize> = HashMap::new();
    let n = 30_000;
    for _ in 0..n {
        *highest.entry(items.iter().random_highest(&random, |x| x.1).unwrap().0).or_default() += 1;
        *lowest.entry(items.iter().random_lowest(&random, |x| x.1).unwrap().0).or_default() += 1;
    }
    assert_even("highest", &highest, 3, n as f64 / 3.0, 0.1);
    assert!(highest.contains_key(&1) && highest.contains_key(&2) && highest.contains_key(&4));
    assert_even("lowest", &lowest, 2, n as f64 / 2.0, 0.1);
    assert!(lowest.contains_key(&0) && lowest.contains_key(&5));
}

#[test]
fn random_of_lowest_n_picks_among_the_n_lowest() {
    let random = SharedRandom::from_seed(11);
    let mut counts: HashMap<i32, usize> = HashMap::new();
    let n = 30_000;
    for _ in 0..n {
        let v = [50, 10, 40, 20, 30].into_iter().random_of_lowest_n(&random, |x| *x, 3).unwrap();
        *counts.entry(v).or_default() += 1;
    }
    assert_even("lowest 3", &counts, 3, n as f64 / 3.0, 0.1);
    assert!(counts.contains_key(&10) && counts.contains_key(&20) && counts.contains_key(&30));
    assert_eq!([1, 2].into_iter().random_of_lowest_n(&random, |x| *x, 0), None);
}

#[test]
fn keys_are_computed_once_per_item() {
    let random = SharedRandom::from_seed(1);
    let calls = std::cell::Cell::new(0);
    let key = |x: &i32| {
        calls.set(calls.get() + 1);
        *x
    };
    let _ = [5, 3, 9, 1, 7].into_iter().random_highest(&random, key);
    assert_eq!(calls.get(), 5);
    let _ = [5, 3, 9, 1, 7].into_iter().random_of_lowest_n(&random, key, 2);
    assert_eq!(calls.get(), 10);
}

/// A key function may draw from the generator the pick uses.
#[test]
fn a_key_may_draw_from_the_same_generator() {
    use treachery_engine::random::Rng;
    let random = SharedRandom::from_seed(1);
    let r = random.clone();
    let picked = (0..10).random_highest(&random, move |_| r.lock().random_range(0..3));
    assert!(picked.is_some());
}

// ---------------------------------------------------------------- whole games

const ESTABLISH: &str = r#"{"$type":"EstablishPlayers","Settings":{"NumberOfPlayers":6,"MaximumTurns":10,"InitialRules":[10000,10001,998,40,85],"AllowedFactionsInPlay":[20,30,40,50,10,60,70,80,90,100,110,120],"AutoOpenEmptySeats":true,"AsyncPlay":false,"AsyncPlayMessageIntervalMinutes":0},"Seed":SEED,"MaximumNumberOfPlayers":6,"MaximumTurns":10,"ApplicableRules":[],"_players":"","_factionsInPlay":"","Initiator":0,"Time":"2026-09-19T20:35:59+00:00"}"#;
const END_PHASE: &str = r#"{"$type":"EndPhase","Initiator":0,"Time":"2026-09-19T20:36:00+00:00"}"#;

/// Establishes a game with `seed` and ends the first phases, then describes what the draws
/// decided: the factions in play and their seats, each player's traitors and the deck order.
fn game_from_seed(seed: i32) -> String {
    let events = format!("[{},{END_PHASE},{END_PHASE}]", ESTABLISH.replace("SEED", &seed.to_string()));
    let savegame = format!(r#"{{"Version":{},"_events":{events}}}"#, Game::LATEST_VERSION);
    let state = GameState::load(&savegame).unwrap();
    let mut game = Game::of_version(state.version);
    for e in state.events.as_ref().unwrap() {
        e.initialize_game(&game);
        assert!(e.execute(&mut game, false, true).is_none());
    }
    let players: Vec<String> = game
        .players
        .iter()
        .map(|p| format!("{} seat {} traitors {:?}", treachery_engine::enum_type::name_or_absent(p.faction), treachery_engine::game::Seat::number_or_minus_one(p.seat), p.traitors.iter().map(|t| t.id()).collect::<Vec<_>>()))
        .collect();
    let deck: Vec<i32> = game.decks.treachery_deck.as_ref().unwrap().items.iter().map(|c| c.id()).collect();
    format!("{}\nphase {}\ndeck {deck:?}\n{}", game.turn.current_phase, game.history.entries.len(), players.join("\n"))
}

#[test]
fn the_same_seed_gives_the_same_game() {
    for seed in [1, 42, -7, i32::MAX] {
        assert_eq!(game_from_seed(seed), game_from_seed(seed), "seed {seed}");
    }
    assert_ne!(game_from_seed(1), game_from_seed(2));
}
