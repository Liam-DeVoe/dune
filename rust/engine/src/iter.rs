//! Iterator helpers that std doesn't have.

/// `contains` for a value that may be absent. An absent value is in no list.
pub trait ContainsOpt<T> {
    fn contains_opt(&self, v: Option<T>) -> bool;
}

impl<T: PartialEq> ContainsOpt<T> for [T] {
    fn contains_opt(&self, v: Option<T>) -> bool {
        v.is_some_and(|v| self.contains(&v))
    }
}

impl<T: std::hash::Hash + Eq> ContainsOpt<T> for indexmap::IndexSet<T> {
    fn contains_opt(&self, v: Option<T>) -> bool {
        v.is_some_and(|v| self.contains(&v))
    }
}

/// Map lookups by a key that may be absent, which no entry has.
pub trait MapOpt<K, V> {
    fn get_opt(&self, k: Option<K>) -> Option<&V>;
    fn contains_key_opt(&self, k: Option<K>) -> bool;
    fn shift_remove_opt(&mut self, k: Option<K>) -> Option<V>;
}

impl<K: std::hash::Hash + Eq, V> MapOpt<K, V> for indexmap::IndexMap<K, V> {
    fn get_opt(&self, k: Option<K>) -> Option<&V> {
        k.and_then(|k| self.get(&k))
    }

    fn contains_key_opt(&self, k: Option<K>) -> bool {
        k.is_some_and(|k| self.contains_key(&k))
    }

    fn shift_remove_opt(&mut self, k: Option<K>) -> Option<V> {
        k.and_then(|k| self.shift_remove(&k))
    }
}

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

/// Removes the first element equal to `item`. Returns whether there was one.
pub fn remove_first<T: PartialEq>(list: &mut Vec<T>, item: &T) -> bool {
    match list.iter().position(|x| x == item) {
        Some(i) => {
            list.remove(i);
            true
        }
        None => false,
    }
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

impl<K: Ord + std::fmt::Debug, V> MapExt<K, V> for std::collections::BTreeMap<K, V> {
    #[track_caller]
    fn insert_new(&mut self, key: K, value: V) {
        assert!(!self.contains_key(&key), "duplicate key {key:?}");
        self.insert(key, value);
    }
}
