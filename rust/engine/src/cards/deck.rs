use crate::error::{GameError, GameResult};
use crate::random::{SharedRandom, SliceRandom};

/// A deck of cards, tokens or leaders. `random` is `None` for a deck created before
/// `EstablishPlayers` seeds the game; such a deck can't be shuffled.
#[derive(Clone, Debug)]
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

    /// Takes the top item. Panics on an empty deck: use [`Deck::try_draw`] where the deck can
    /// run out.
    pub fn draw(&mut self) -> T {
        self.try_draw().expect("drawing from an empty deck")
    }

    /// Takes the top item, or fails the event on an empty deck.
    pub fn try_draw(&mut self) -> GameResult<T> {
        if self.items.is_empty() {
            return Err(GameError::new("Cannot draw from an empty deck"));
        }
        Ok(self.items.remove(0))
    }

    /// Shuffles the deck. A deck without a generator can only "shuffle" when it has fewer than
    /// two items.
    pub fn shuffle(&mut self) {
        match &self.random {
            Some(random) => self.items.shuffle(&mut *random.lock()),
            None => {
                if self.items.len() > 1 {
                    panic!("shuffling a deck without a random generator")
                }
            }
        }
    }

    /// `IsEmpty`
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// `Top`: `None` on an empty deck. Where the top of an empty deck should
    /// read as a default value (e.g. 0), use `.unwrap_or_default()`.
    pub fn top(&self) -> Option<T> {
        self.items.first().cloned()
    }

    /// `Clear()`
    pub fn clear(&mut self) {
        self.items.clear();
    }
}
