//! The game's random number generator, and the selection helpers the engine and the bots use.
//!
//! A game draws every random number from one ChaCha8 generator seeded with the seed of its
//! `EstablishPlayers` event, so a game replays the same way from its events. Each bot has its own
//! generator, seeded when the bot is created. ChaCha8 (`rand_chacha`) promises the same stream for
//! a seed in every version and on every platform, and so does rand's sampling (`random_range`,
//! `shuffle`, `choose`) within one minor version of rand. Upgrading `rand` or `rand_chacha` to a
//! new minor version may change the draws; if it does, that is a change of game version.
//!
//! `Game.random` is shared with every `Deck` built from it: [`SharedRandom`] is that shared
//! reference, so draws through any deck advance the same stream.

use std::cell::RefMut;
use std::cmp::Ordering;

use rand::SeedableRng;
use rand::seq::IteratorRandom;

/// rand's sampling traits, for drawing from [`SharedRandom::lock`].
pub use rand::Rng;
pub use rand::seq::{IndexedRandom, SliceRandom};

use crate::shared::Shared;

/// The generator behind every random draw of a game or a bot.
pub type GameRng = rand_chacha::ChaCha8Rng;

/// A new generator for a game or bot seed.
pub fn new_rng(seed: i32) -> GameRng {
    GameRng::seed_from_u64(seed as i64 as u64)
}

/// A shared reference to a generator: clones share the same stream.
#[derive(Clone, Debug, PartialEq)]
pub struct SharedRandom(Shared<GameRng>);

impl SharedRandom {
    pub fn from_seed(seed: i32) -> Self {
        SharedRandom(Shared::new(new_rng(seed)))
    }

    /// The generator, to draw from with rand's methods (`random_range`, `shuffle`, ...). Don't
    /// hold it across a call that may draw from the same generator.
    pub fn lock(&self) -> RefMut<'_, GameRng> {
        self.0.borrow_mut()
    }

    /// A copy of the generator's current state, for tests that check how far it has drawn.
    pub fn state(&self) -> GameRng {
        self.0.borrow().clone()
    }

    /// Whether both refer to the same generator.
    pub fn ptr_eq(&self, other: &SharedRandom) -> bool {
        self.0.ptr_eq(&other.0)
    }
}

/// A key to rank items by in [`RandomPick`]. Floats rank by `total_cmp`, with -0 equal to 0.
pub trait RankKey {
    fn rank(&self, other: &Self) -> Ordering;
}

macro_rules! rank_key_ord {
    ($($t:ty),*) => {$(
        impl RankKey for $t {
            fn rank(&self, other: &Self) -> Ordering { self.cmp(other) }
        }
    )*};
}
rank_key_ord!(u16, i32, i64, u32, usize, bool);

impl RankKey for f32 {
    fn rank(&self, other: &Self) -> Ordering {
        (self + 0.0).total_cmp(&(other + 0.0))
    }
}

impl RankKey for f64 {
    fn rank(&self, other: &Self) -> Ordering {
        (self + 0.0).total_cmp(&(other + 0.0))
    }
}

/// Picking a random item from an iterator. Each helper collects the items (and computes each key
/// once) before it draws, so the iterator and the key function may draw from the same generator.
pub trait RandomPick: Iterator + Sized {
    /// A random item, or `None` if there are none. Draws only when there are two or more.
    fn random_item(self, random: &SharedRandom) -> Option<Self::Item> {
        let items: Vec<Self::Item> = self.collect();
        items.into_iter().choose(&mut *random.lock())
    }

    /// A random one of the items with the highest key.
    fn random_highest<K: RankKey>(self, random: &SharedRandom, key: impl FnMut(&Self::Item) -> K) -> Option<Self::Item> {
        random_extreme(self, random, key, Ordering::Greater)
    }

    /// A random one of the items with the lowest key.
    fn random_lowest<K: RankKey>(self, random: &SharedRandom, key: impl FnMut(&Self::Item) -> K) -> Option<Self::Item> {
        random_extreme(self, random, key, Ordering::Less)
    }

    /// A random one of the `n` items with the lowest keys (ties broken by position).
    fn random_of_lowest_n<K: RankKey>(self, random: &SharedRandom, mut key: impl FnMut(&Self::Item) -> K, n: usize) -> Option<Self::Item> {
        let mut keyed: Vec<(K, Self::Item)> = self.map(|v| (key(&v), v)).collect();
        keyed.sort_by(|a, b| a.0.rank(&b.0));
        keyed.into_iter().take(n).map(|(_, v)| v).random_item(random)
    }
}

impl<I: Iterator> RandomPick for I {}

fn random_extreme<I: Iterator, K: RankKey>(source: I, random: &SharedRandom, mut key: impl FnMut(&I::Item) -> K, better: Ordering) -> Option<I::Item> {
    let keyed: Vec<(K, I::Item)> = source.map(|v| (key(&v), v)).collect();
    let mut best = 0;
    for i in 1..keyed.len() {
        if keyed[i].0.rank(&keyed[best].0) == better {
            best = i;
        }
    }
    let best_key = &keyed.get(best)?.0;
    let ties: Vec<usize> = (0..keyed.len()).filter(|&i| keyed[i].0.rank(best_key) == Ordering::Equal).collect();
    let chosen = ties.into_iter().random_item(random)?;
    keyed.into_iter().nth(chosen).map(|(_, v)| v)
}
