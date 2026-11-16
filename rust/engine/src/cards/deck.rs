use crate::error::{GameError, GameResult};
use crate::random::{GameRng, SliceRandom};

/// A deck of cards, tokens or leaders. Shuffling draws from the generator it is given, which is
/// the game's (`game.rng`) for every deck the game owns.
#[derive(Clone, Debug)]
pub struct Deck<T> {
    pub items: Vec<T>,
}

impl<T: Clone> Deck<T> {
    pub fn new(items: impl IntoIterator<Item = T>) -> Self {
        Deck { items: items.into_iter().collect() }
    }

    pub fn empty() -> Self {
        Deck { items: Vec::new() }
    }

    pub fn put_on_top(&mut self, item: T) {
        self.items.insert(0, item);
    }

    /// `PutOnTop(items)`: each item goes on top in turn, so the last one ends up on top.
    pub fn put_on_top_all(&mut self, items: impl IntoIterator<Item = T>) {
        for item in items {
            self.put_on_top(item);
        }
    }

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

    /// Shuffles the deck with draws from `rng`.
    pub fn shuffle(&mut self, rng: &mut GameRng) {
        self.items.shuffle(rng);
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// `Top`: `None` on an empty deck. Where the top of an empty deck should
    /// read as a default value (e.g. 0), use `.unwrap_or_default()`.
    pub fn top(&self) -> Option<T> {
        self.items.first().cloned()
    }

    pub fn clear(&mut self) {
        self.items.clear();
    }
}
