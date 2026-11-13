//! Port of Model/Deck.cs.

use crate::random::SharedRandom;
use crate::throw;

/// `Deck<T>`. `random` is `None` where C# would hold null (a deck created
/// from `Game.Random` before `EstablishPlayers` sets it); shuffling such a
/// deck throws NullReferenceException, as in C#.
#[derive(Clone, Debug, canon_derive::Canon)]
pub struct Deck<T> {
    pub items: Vec<T>,
    pub random: Option<SharedRandom>,
}

impl<T: Clone> Deck<T> {
    /// `new Deck<T>(items, random)`
    pub fn new(items: impl IntoIterator<Item = T>, random: Option<SharedRandom>) -> Self {
        Deck { items: items.into_iter().collect(), random }
    }

    /// `new Deck<T>(random)`
    pub fn empty(random: Option<SharedRandom>) -> Self {
        Deck { items: Vec::new(), random }
    }

    /// `PutOnTop(item)`
    pub fn put_on_top(&mut self, item: T) {
        self.items.insert(0, item);
    }

    /// `PutOnTop(items)`: each item goes on top in turn, so the last one ends up on top.
    pub fn put_on_top_all(&mut self, items: impl IntoIterator<Item = T>) {
        for item in items {
            self.put_on_top(item);
        }
    }

    /// `PutOnBottom(item)`
    pub fn put_on_bottom(&mut self, item: T) {
        self.items.push(item);
    }

    /// `Draw()`; throws InvalidOperationException on an empty deck.
    pub fn draw(&mut self) -> T {
        if !self.items.is_empty() {
            return self.items.remove(0);
        }
        throw!(InvalidOperationException, "Cannot draw from an empty deck")
    }

    /// `Shuffle()`
    pub fn shuffle(&mut self) {
        match &self.random {
            Some(random) => Self::shuffle_list(&mut self.items, random),
            None => {
                // C# parity: with no items or one item, Shuffle never touches Random.
                if self.items.len() > 1 {
                    throw!(NullReferenceException, "{}", crate::compat::cs::NULL_REFERENCE_MESSAGE)
                }
            }
        }
    }

    /// `Deck<T>.Shuffle(items, random)` (static)
    pub fn shuffle_list(items: &mut [T], random: &SharedRandom) {
        let mut n = items.len();
        while n > 1 {
            n -= 1;
            let k = random.next(n as i32 + 1) as usize;
            items.swap(n, k);
        }
    }

    // `Deck<T>.Randomize` uses an unseeded `new LoggedRandom()` and nothing
    // calls it, so it is not ported.

    /// `IsEmpty`
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// `Top`: `None` where C# returns `default` (null for reference types; for
    /// value types C# returns e.g. 0, so use `.unwrap_or_default()` there).
    pub fn top(&self) -> Option<T> {
        self.items.first().cloned()
    }

    /// `Clear()`
    pub fn clear(&mut self) {
        self.items.clear();
    }
}
