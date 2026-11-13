//! `Game.Random` is one `LoggedRandom` object, and every `Deck` built
//! with it holds a reference to that same object. [`SharedRandom`] is that
//! shared reference: cloning it shares the generator, so draws through any
//! deck advance the same sequence.

use std::cell::RefMut;

use dotnet_compat::NetRandom;

use crate::compat::cs::Shared;

#[derive(Clone, Debug, canon_derive::Canon)]
#[canon(manual)]
pub struct LoggedRandom {
    #[canon(name = "_count")]
    count: i32,
    #[canon(name = "_supply", omit)]
    supply: NetRandom,
}

impl LoggedRandom {
    /// `new LoggedRandom(seed)`
    pub fn new(seed: i32) -> Self {
        LoggedRandom { count: 0, supply: NetRandom::new(seed) }
    }

    /// `Next(i)`
    pub fn next(&mut self, i: i32) -> i32 {
        self.count += 1;
        self.supply.next_max(i)
    }

    /// `GetCount()`
    pub fn get_count(&self) -> i32 {
        self.count
    }

    /// The underlying `System.Random`, for state dumps.
    pub fn supply(&self) -> &NetRandom {
        &self.supply
    }
}

// C# also has `new LoggedRandom()` (unseeded `new Random()`), used only by
// bots without an explicit random. It cannot be reproduced; bots must be given
// a seeded LoggedRandom.

/// A shared reference to a `LoggedRandom`: clones share the same generator.
#[derive(Clone, Debug, PartialEq)]
pub struct SharedRandom(Shared<LoggedRandom>);

impl SharedRandom {
    pub fn new(random: LoggedRandom) -> Self {
        SharedRandom(Shared::new(random))
    }

    /// `new LoggedRandom(seed)`, shared.
    pub fn from_seed(seed: i32) -> Self {
        Self::new(LoggedRandom::new(seed))
    }

    /// `Next(i)`
    pub fn next(&self, i: i32) -> i32 {
        self.lock().next(i)
    }

    /// `GetCount()`
    pub fn get_count(&self) -> i32 {
        self.lock().get_count()
    }

    /// Direct access, e.g. for dumping state.
    pub fn lock(&self) -> RefMut<'_, LoggedRandom> {
        self.0.borrow_mut()
    }

    /// Whether both refer to the same generator.
    pub fn ptr_eq(&self, other: &SharedRandom) -> bool {
        self.0.ptr_eq(&other.0)
    }
}
