//! Lazy `IEnumerable` returns (support module, no C# counterpart).
//!
//! A C# member that returns a deferred LINQ query runs its predicates only
//! when, and as far as, the caller enumerates. Callers often stop early
//! (`Any`, `First`, `Contains`), so the predicate never sees later elements.
//! The Rust port returns `impl Iterator` for every such member; see
//! docs/lazy-queries.md for the list and engine-conventions.md for the rule.
//!
//! `Either` lets one function return different lazy (or empty) queries on
//! different branches, as C# does with `return cond ? query : [];`.

/// One of two iterators with the same item type.
#[derive(Clone)]
pub enum Either<A, B> {
    A(A),
    B(B),
}

impl<T, A: Iterator<Item = T>, B: Iterator<Item = T>> Iterator for Either<A, B> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        match self {
            Either::A(a) => a.next(),
            Either::B(b) => b.next(),
        }
    }
}

/// `Array.Empty<T>()` / `[]` on a branch of a lazy member.
pub fn empty<T>() -> std::iter::Empty<T> {
    std::iter::empty()
}

/// `Enumerable.Contains(source, value)` on a lazy source: stops at the first equal element.
pub fn contains<T: PartialEq>(mut source: impl Iterator<Item = T>, value: &T) -> bool {
    source.any(|x| x == *value)
}

/// `source.Distinct()` as a lazy query: yields the first occurrence of each
/// element as it is reached, so a consumer that stops early pulls no further.
pub fn distinct<T: PartialEq + Clone>(source: impl Iterator<Item = T>) -> impl Iterator<Item = T> {
    let mut seen: Vec<T> = Vec::new();
    source.filter(move |x| {
        if seen.contains(x) {
            false
        } else {
            seen.push(x.clone());
            true
        }
    })
}

/// `first.Union(second)` as a lazy query: `Distinct` over `first` followed by `second`.
pub fn union<T: PartialEq + Clone>(first: impl Iterator<Item = T>, second: impl Iterator<Item = T>) -> impl Iterator<Item = T> {
    distinct(first.chain(second))
}

/// `Contains` on a lazy query, as a method: `ValidTargets(g, p).Contains(x)` is
/// `valid_targets(g, p).cs_contains(&x)`. Stops at the first equal element.
pub trait LazyQuery: Iterator + Sized {
    fn cs_contains(mut self, value: &Self::Item) -> bool
    where
        Self::Item: PartialEq,
    {
        self.any(|x| x == *value)
    }
}

impl<I: Iterator> LazyQuery for I {}
