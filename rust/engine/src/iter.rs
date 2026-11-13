//! Iterator helpers that std doesn't have.

/// The first occurrence of each element, in order, compared with `==`. Lazy: an element
/// is pulled from `source` only when the consumer asks for the next one, so a consumer
/// that stops early runs no closures of `source` on the elements after it.
pub fn unique<T: PartialEq + Clone>(source: impl Iterator<Item = T>) -> impl Iterator<Item = T> {
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

/// Extension methods on iterators.
pub trait IterExt: Iterator + Sized {
    /// The first occurrence of each element, in order, compared with `==`.
    fn distinct(self) -> Vec<Self::Item>
    where
        Self::Item: PartialEq + Clone,
    {
        unique(self).collect()
    }
}

impl<I: Iterator> IterExt for I {}
