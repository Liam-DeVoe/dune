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

/// Extension methods on maps.
pub trait MapExt<K, V> {
    /// Inserts a key that must not be present yet. Panics, leaving the map unchanged, if it is.
    fn insert_new(&mut self, key: K, value: V);
}

impl<K: std::hash::Hash + Eq + std::fmt::Debug, V> MapExt<K, V> for indexmap::IndexMap<K, V> {
    #[track_caller]
    fn insert_new(&mut self, key: K, value: V) {
        assert!(!self.contains_key(&key), "duplicate key {key:?}");
        self.insert(key, value);
    }
}

impl<K: std::hash::Hash + Eq + Clone + std::fmt::Debug + 'static, V> MapExt<K, V> for dotnet_compat::NetDictionary<K, V> {
    #[track_caller]
    fn insert_new(&mut self, key: K, value: V) {
        assert!(!self.contains_key(&key), "duplicate key {key:?}");
        self.insert(key, value);
    }
}
