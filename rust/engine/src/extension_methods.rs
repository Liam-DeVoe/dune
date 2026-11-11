//! Port of ExtensionMethods.cs.
//!
//! The engine only uses `RandomOrDefault(LoggedRandom)`. The bots also use
//! the selector helpers. C# has two versions of each: one taking a
//! `LoggedRandom` (ported here) and one using a private, unseeded static
//! `System.Random`, which cannot be reproduced. Those are ported against
//! [`set_static_random_seed`], which needs a matching C# test hook (see
//! `rust/docs/csharp-hooks.md`) before bot traces can be compared.
//!
//! C# returns `default` when nothing can be picked: null for reference
//! types, but 0/false for value types. These return `Option`; at value-type
//! call sites write `.unwrap_or_default()`.
//!
//! Selectors return a [`CsComparable`] key: C# boxes the key as
//! `IComparable`, orders keys with `Comparer.Default` and tests ties with
//! `Equals`. For strings the two disagree: ordering is culture-sensitive
//! (`"a" < "B"`), equality is ordinal. Floats order NaN lowest, and NaN
//! equals NaN.
//!
//! Selector calls are observable when a selector has side effects, so each
//! helper calls it exactly as often, and in the same order, as the C# LINQ
//! query does. tests/random_helpers.rs checks the results, selector call
//! counts and random draws against .NET.

use std::cell::RefCell;
use std::cmp::Ordering;

use dotnet_compat::NetRandom;

use crate::cs::CsComparable;
use crate::logged_random::SharedRandom;

/// `sourceArray.Max(selector)` for a reference-type result: the first
/// maximal key, one selector call per element.
fn max_key<T, K: CsComparable>(source: &[T], selector: &impl Fn(&T) -> K) -> Option<K> {
    extreme_key(source, selector, Ordering::Greater)
}

/// `sourceArray.Min(selector)`.
fn min_key<T, K: CsComparable>(source: &[T], selector: &impl Fn(&T) -> K) -> Option<K> {
    extreme_key(source, selector, Ordering::Less)
}

fn extreme_key<T, K: CsComparable>(source: &[T], selector: &impl Fn(&T) -> K, better: Ordering) -> Option<K> {
    let mut best: Option<K> = None;
    for v in source {
        let k = selector(v);
        match &best {
            None => best = Some(k),
            Some(b) => {
                if k.cs_compare(b) == better {
                    best = Some(k);
                }
            }
        }
    }
    best
}

/// `source.OrderBy(selector).Take(n).ToList()` (or `OrderByDescending`) for
/// `n > 0`: each key is computed once, in element order, then the items are
/// sorted stably (ties keep their order, also when descending).
fn order_take<T, K: CsComparable>(source: impl Iterator<Item = T>, selector: &impl Fn(&T) -> K, descending: bool, n: i32) -> Vec<T> {
    let mut keyed: Vec<(K, T)> = source.map(|v| (selector(&v), v)).collect();
    keyed.sort_by(|a, b| {
        let c = a.0.cs_compare(&b.0);
        if descending { c.reverse() } else { c }
    });
    keyed.into_iter().take(n as usize).map(|(_, v)| v).collect()
}

/// Extension methods on `IEnumerable<T>` that take a `LoggedRandom`.
pub trait RandomExt: Iterator + Sized {
    /// `RandomOrDefault(random)`: draws only when there are 2 or more items.
    fn random_or_default(self, random: &SharedRandom) -> Option<Self::Item> {
        let mut source: Vec<Self::Item> = self.collect();
        // A null element is returned as None: both are C# null.
        crate::cs::null_to_none(match source.len() {
            0 => None,
            1 => source.pop(),
            n => {
                let i = random.next(n as i32) as usize;
                Some(source.swap_remove(i))
            }
        })
    }

    /// `HighestOrDefault(random, selector)`: `Max(selector)`, then
    /// `Where(v => selector(v).Equals(best))`, so the selector runs twice per element.
    fn highest_or_default<K: CsComparable>(self, random: &SharedRandom, selector: impl Fn(&Self::Item) -> K) -> Option<Self::Item> {
        let source: Vec<Self::Item> = self.collect();
        let best = max_key(&source, &selector)?;
        source.into_iter().filter(|v| selector(v).cs_equals(&best)).random_or_default(random)
    }

    /// `LowestOrDefault(random, selector)`
    fn lowest_or_default<K: CsComparable>(self, random: &SharedRandom, selector: impl Fn(&Self::Item) -> K) -> Option<Self::Item> {
        let source: Vec<Self::Item> = self.collect();
        let best = min_key(&source, &selector)?;
        source.into_iter().filter(|v| selector(v).cs_equals(&best)).random_or_default(random)
    }

    /// `OneOfHighestNOrDefault(random, selector, n)`: stable descending sort, take n, pick one.
    fn one_of_highest_n_or_default<K: CsComparable>(
        self,
        random: &SharedRandom,
        selector: impl Fn(&Self::Item) -> K,
        n: i32,
    ) -> Option<Self::Item> {
        if n <= 0 {
            return None;
        }
        order_take(self, &selector, true, n).into_iter().random_or_default(random)
    }

    /// `OneOfLowestNOrDefault(random, selector, n)`: stable ascending sort, take n, pick one.
    fn one_of_lowest_n_or_default<K: CsComparable>(
        self,
        random: &SharedRandom,
        selector: impl Fn(&Self::Item) -> K,
        n: i32,
    ) -> Option<Self::Item> {
        if n <= 0 {
            return None;
        }
        order_take(self, &selector, false, n).into_iter().random_or_default(random)
    }
}

impl<I: Iterator> RandomExt for I {}

thread_local! {
    /// `ExtensionMethods.Random` (private static, unseeded in C#).
    static STATIC_RANDOM: RefCell<Option<NetRandom>> = const { RefCell::new(None) };
}

/// Seeds the stand-in for C#'s unseeded `ExtensionMethods.Random`. Only
/// meaningful once C# has a hook that seeds it the same way.
pub fn set_static_random_seed(seed: i32) {
    STATIC_RANDOM.with(|r| *r.borrow_mut() = Some(NetRandom::new(seed)));
}

/// `ExtensionMethods.Random.Next(max)`.
pub fn static_random_next(max: i32) -> i32 {
    STATIC_RANDOM.with(|r| {
        r.borrow_mut()
            .as_mut()
            .expect("ExtensionMethods static Random used without set_static_random_seed")
            .next_max(max)
    })
}

fn static_random_next_full() -> i32 {
    STATIC_RANDOM.with(|r| {
        r.borrow_mut()
            .as_mut()
            .expect("ExtensionMethods static Random used without set_static_random_seed")
            .next()
    })
}

/// The overloads that use the static unseeded `Random` (bots only).
pub trait UnseededRandomExt: Iterator + Sized {
    /// `RandomOrDefault()`
    fn random_or_default_unseeded(self) -> Option<Self::Item> {
        let mut source: Vec<Self::Item> = self.collect();
        // A null element is returned as None: both are C# null.
        crate::cs::null_to_none(match source.len() {
            0 => None,
            1 => source.pop(),
            n => {
                let i = static_random_next(n as i32) as usize;
                Some(source.swap_remove(i))
            }
        })
    }

    /// `HighestOrDefault(selector)`
    fn highest_or_default_unseeded<K: CsComparable>(self, selector: impl Fn(&Self::Item) -> K) -> Option<Self::Item> {
        let source: Vec<Self::Item> = self.collect();
        let best = max_key(&source, &selector)?;
        source.into_iter().filter(|v| selector(v).cs_equals(&best)).random_or_default_unseeded()
    }

    /// `LowestOrDefault(selector)`
    fn lowest_or_default_unseeded<K: CsComparable>(self, selector: impl Fn(&Self::Item) -> K) -> Option<Self::Item> {
        let source: Vec<Self::Item> = self.collect();
        let best = min_key(&source, &selector)?;
        source.into_iter().filter(|v| selector(v).cs_equals(&best)).random_or_default_unseeded()
    }

    /// `OneOfHighestNOrDefault(selector, n)`
    fn one_of_highest_n_or_default_unseeded<K: CsComparable>(self, selector: impl Fn(&Self::Item) -> K, n: i32) -> Option<Self::Item> {
        if n <= 0 {
            return None;
        }
        order_take(self, &selector, true, n).into_iter().random_or_default_unseeded()
    }

    /// `OneOfLowestNOrDefault(selector, n)`
    fn one_of_lowest_n_or_default_unseeded<K: CsComparable>(self, selector: impl Fn(&Self::Item) -> K, n: i32) -> Option<Self::Item> {
        if n <= 0 {
            return None;
        }
        order_take(self, &selector, false, n).into_iter().random_or_default_unseeded()
    }

    /// `TakeRandomN(n)`: `OrderBy(_ => Random.Next()).Take(n)`, a lazy query.
    /// Nothing is drawn until the result is iterated. Each iteration draws
    /// one key per element, then sorts stably by key; iterate a clone to
    /// enumerate the C# query again (with fresh draws). `Take(n)` with
    /// `n <= 0` never enumerates the source, so it draws nothing.
    fn take_random_n(self, n: i32) -> TakeRandomN<Self> {
        TakeRandomN { source: Some(self), n, sorted: None }
    }
}

impl<I: Iterator> UnseededRandomExt for I {}

/// The lazy query returned by [`UnseededRandomExt::take_random_n`].
#[derive(Clone)]
pub struct TakeRandomN<I: Iterator> {
    source: Option<I>,
    n: i32,
    sorted: Option<std::vec::IntoIter<I::Item>>,
}

impl<I: Iterator> Iterator for TakeRandomN<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> {
        if self.sorted.is_none() {
            let source = self.source.take()?;
            if self.n <= 0 {
                self.sorted = Some(Vec::new().into_iter());
            } else {
                let mut keyed: Vec<(i32, I::Item)> = source.map(|v| (static_random_next_full(), v)).collect();
                keyed.sort_by_key(|(k, _)| *k);
                let taken: Vec<I::Item> = keyed.into_iter().take(self.n as usize).map(|(_, v)| v).collect();
                self.sorted = Some(taken.into_iter());
            }
        }
        self.sorted.as_mut()?.next()
    }
}
