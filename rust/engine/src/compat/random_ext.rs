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

use crate::random::SharedRandom;

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
        match source.len() {
            0 => None,
            1 => source.pop(),
            n => {
                let i = random.next(n as i32) as usize;
                Some(source.swap_remove(i))
            }
        }
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
        match source.len() {
            0 => None,
            1 => source.pop(),
            n => {
                let i = static_random_next(n as i32) as usize;
                Some(source.swap_remove(i))
            }
        }
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

/// `string.Compare(a, b)` / `a.CompareTo(b)` under the invariant culture
/// (and under en-US, which ICU does not tailor): the comparison
/// `Comparer.Default` uses for string keys in `OrderBy`, `Max` and the
/// `HighestOrDefault` family.
///
/// .NET delegates to ICU's root collation. For ASCII text that is:
/// - control characters other than tab, line feed, vertical tab, form feed and
///   carriage return are ignored;
/// - every other character has a primary weight in the order of
///   [`COLLATION_ORDER`]: whitespace, punctuation, symbols, digits, then
///   letters, with each upper-case letter sharing the primary weight of its
///   lower-case form;
/// - strings are compared by their primary weights first and, only when those
///   are equal, by case, with lower case first at the first difference.
///
/// So `"a" < "B" < "b"`, `"a-b" < "ab"` and `"ab" < "Ab" < "ac"`. Distinct
/// strings can compare equal ("a\u{1}b" and "ab").
///
/// Limit: only ASCII is implemented. Enum names and the leader and card
/// names in the skin are ASCII. A non-ASCII character panics (a porting bug,
/// not a C# exception) rather than giving a wrong order silently. Checked
/// against .NET in tests/random_helpers.rs and tests/linq_table.rs.
pub fn compare_invariant(a: &str, b: &str) -> std::cmp::Ordering {
    fn weights(s: &str) -> Vec<(u8, bool)> {
        s.chars()
            .filter_map(|c| {
                assert!(c.is_ascii(), "cs::compare_invariant: culture comparison of non-ASCII text is not implemented ({s:?})");
                let lower = c.to_ascii_lowercase() as u8;
                let primary = COLLATION_ORDER.iter().position(|&x| x == lower)?;
                Some((primary as u8, c.is_ascii_uppercase()))
            })
            .collect()
    }
    let (wa, wb) = (weights(a), weights(b));
    let primary = wa.iter().map(|w| w.0).cmp(wb.iter().map(|w| w.0));
    primary.then_with(|| wa.iter().map(|w| w.1).cmp(wb.iter().map(|w| w.1)))
}

/// ASCII characters (lower-case letters only) in ICU root collation order,
/// as measured with .NET 10. Characters missing here are ignorable.
pub const COLLATION_ORDER: &[u8] =
    b"\t\n\x0b\x0c\r _-,;:!?.'\"()[]{}@*/\\&#%`^+<=>|~$0123456789abcdefghijklmnopqrstuvwxyz";

/// A key that a C# selector returns as `IComparable` (`Func<T, IComparable>`
/// in the `HighestOrDefault` family, or an `OrderBy` key). C# compares the
/// boxed keys with `Comparer.Default` and tests them with `object.Equals`;
/// these two can disagree (strings compare by culture but are equal only
/// ordinally), so a key provides both.
pub trait CsComparable {
    /// `Comparer.Default.Compare(self, other)`.
    fn cs_compare(&self, other: &Self) -> std::cmp::Ordering;
    /// `self.Equals(other)` on the boxed values.
    fn cs_equals(&self, other: &Self) -> bool;
}

macro_rules! cs_comparable_ord {
    ($($t:ty),*) => {$(
        impl CsComparable for $t {
            fn cs_compare(&self, other: &Self) -> std::cmp::Ordering { self.cmp(other) }
            fn cs_equals(&self, other: &Self) -> bool { self == other }
        }
    )*};
}
cs_comparable_ord!(i32, i64, bool);

/// `float.CompareTo`: NaN is smaller than everything and equal to itself;
/// `Equals` treats NaN as equal to NaN and -0 as equal to 0.
impl CsComparable for f32 {
    fn cs_compare(&self, other: &Self) -> std::cmp::Ordering {
        dotnet_compat::math::cmp_f32(*self, *other)
    }
    fn cs_equals(&self, other: &Self) -> bool {
        dotnet_compat::math::equals_f32(*self, *other)
    }
}

impl CsComparable for f64 {
    fn cs_compare(&self, other: &Self) -> std::cmp::Ordering {
        dotnet_compat::math::cmp_f64(*self, *other)
    }
    fn cs_equals(&self, other: &Self) -> bool {
        dotnet_compat::math::equals_f64(*self, *other)
    }
}

/// Culture comparison, ordinal equality. See [`compare_invariant`].
impl CsComparable for String {
    fn cs_compare(&self, other: &Self) -> std::cmp::Ordering {
        compare_invariant(self, other)
    }
    fn cs_equals(&self, other: &Self) -> bool {
        self == other
    }
}

impl CsComparable for &str {
    fn cs_compare(&self, other: &Self) -> std::cmp::Ordering {
        compare_invariant(self, other)
    }
    fn cs_equals(&self, other: &Self) -> bool {
        self == other
    }
}
