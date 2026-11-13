//! `System.Collections.Generic.Dictionary<TKey, TValue>` with .NET's exact
//! enumeration order.
//!
//! .NET stores entries in an array and enumerates it in index ("slot") order,
//! skipping freed slots. New keys go into the most recently freed slot (the
//! free list is LIFO since .NET Core 3.0), otherwise at the end. `Clear` resets
//! the array. Hash codes only affect bucket chains, never enumeration order, so
//! this port keeps the slot array and uses a std `HashMap` purely as a
//! key -> slot index.
//!
//! `Clone` copies the exact internal state (holes and free list included), like
//! a deep copy of the .NET object. The C# copy constructor
//! `new Dictionary<K,V>(other)` and `ToDictionary` compact the entries instead;
//! use [`NetDictionary::from_dictionary`] for those.

use crate::exception::{CsToString, throw};
use std::any::Any;
use std::collections::HashMap;
use std::hash::Hash;
use std::sync::OnceLock;

static NULL_KEY: OnceLock<fn(&dyn Any) -> bool> = OnceLock::new();

/// Registers the test for C# null keys. The engine's static-data handles have
/// a null value that this crate doesn't know about; every key-taking method
/// asks this function and throws ArgumentNullException "Value cannot be null.
/// (Parameter 'key')" for a null key, as .NET does. The first registration
/// wins.
pub fn set_null_key_check(f: fn(&dyn Any) -> bool) {
    let _ = NULL_KEY.set(f);
}

#[track_caller]
fn check_key<K: 'static>(key: &K) {
    if let Some(f) = NULL_KEY.get() {
        if f(key) {
            throw("ArgumentNullException", "Value cannot be null. (Parameter 'key')".to_string());
        }
    }
}

#[derive(Clone, Debug)]
pub struct NetDictionary<K, V> {
    /// Slot array (`_entries[0.._count]`). `None` is a freed slot.
    entries: Vec<Option<(K, V)>>,
    /// Free slots, top of stack = head of .NET's `_freeList`.
    free: Vec<usize>,
    index: HashMap<K, usize>,
}

impl<K: Eq + Hash + Clone + 'static, V> Default for NetDictionary<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: Eq + Hash + Clone + 'static, V> NetDictionary<K, V> {
    pub fn new() -> Self {
        NetDictionary { entries: Vec::new(), free: Vec::new(), index: HashMap::new() }
    }

    /// `new Dictionary<K,V>(capacity)`. Capacity never affects order.
    pub fn with_capacity(capacity: usize) -> Self {
        NetDictionary { entries: Vec::with_capacity(capacity), free: Vec::new(), index: HashMap::with_capacity(capacity) }
    }

    fn insert_new_slot(&mut self, key: K, value: V) {
        let slot = match self.free.pop() {
            Some(slot) => {
                self.entries[slot] = Some((key.clone(), value));
                slot
            }
            None => {
                self.entries.push(Some((key.clone(), value)));
                self.entries.len() - 1
            }
        };
        self.index.insert(key, slot);
    }

    /// `dict[key] = value` (overwrites in place, keeps slot)
    #[track_caller]
    pub fn insert(&mut self, key: K, value: V) {
        check_key(&key);
        match self.index.get(&key) {
            Some(&slot) => self.entries[slot].as_mut().unwrap().1 = value,
            None => self.insert_new_slot(key, value),
        }
    }


    /// `dict.TryAdd(key, value)`
    #[track_caller]
    pub fn try_add(&mut self, key: K, value: V) -> bool {
        check_key(&key);
        if self.index.contains_key(&key) {
            return false;
        }
        self.insert_new_slot(key, value);
        true
    }

    /// `dict.TryGetValue(key, out v)`
    #[track_caller]
    pub fn get(&self, key: &K) -> Option<&V> {
        check_key(key);
        self.index.get(key).map(|&slot| &self.entries[slot].as_ref().unwrap().1)
    }

    #[track_caller]
    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        check_key(key);
        match self.index.get(key) {
            Some(&slot) => Some(&mut self.entries[slot].as_mut().unwrap().1),
            None => None,
        }
    }


    #[track_caller]
    pub fn contains_key(&self, key: &K) -> bool {
        check_key(key);
        self.index.contains_key(key)
    }

    /// `dict.ContainsValue(value)`
    pub fn contains_value(&self, value: &V) -> bool
    where
        V: PartialEq,
    {
        self.values().any(|v| v == value)
    }

    /// `dict.Remove(key)` / `dict.Remove(key, out value)`. Frees the slot.
    #[track_caller]
    pub fn remove(&mut self, key: &K) -> Option<V> {
        check_key(key);
        let slot = self.index.remove(key)?;
        let (_, v) = self.entries[slot].take().unwrap();
        self.free.push(slot);
        Some(v)
    }

    /// `dict.Clear()`: resets the slot array, so later adds start at slot 0.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.free.clear();
        self.index.clear();
    }

    /// `dict.Count`
    pub fn len(&self) -> usize {
        self.index.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// `foreach (var kvp in dict)`
    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.entries.iter().filter_map(|e| e.as_ref().map(|(k, v)| (k, v)))
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&K, &mut V)> {
        self.entries.iter_mut().filter_map(|e| e.as_mut().map(|(k, v)| (&*k, v)))
    }

    /// The stored keys, for mutation that keeps each key's `Eq` and `Hash` (a field
    /// the comparison ignores, such as which C# object a key is). Not a .NET operation.
    pub fn keys_mut_same_identity(&mut self) -> impl Iterator<Item = &mut K> {
        self.entries.iter_mut().filter_map(|e| e.as_mut().map(|(k, _)| k))
    }

    /// `dict.Keys` (same order as enumeration)
    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.iter().map(|(k, _)| k)
    }

    /// `dict.Values` (same order as enumeration)
    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.iter().map(|(_, v)| v)
    }

    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut V> {
        self.iter_mut().map(|(_, v)| v)
    }

    /// `dict.GetValueOrDefault(key)` for value types with a C# default.
    pub fn get_value_or_default(&self, key: &K) -> V
    where
        V: Clone + Default,
    {
        self.get(key).cloned().unwrap_or_default()
    }

    /// `dict.GetValueOrDefault(key, defaultValue)`
    pub fn get_value_or(&self, key: &K, default: V) -> V
    where
        V: Clone,
    {
        self.get(key).cloned().unwrap_or(default)
    }

    /// `new Dictionary<K,V>(other)`: copies live entries in enumeration order
    /// into a fresh, compact dictionary (no holes, empty free list).
    pub fn from_dictionary(other: &NetDictionary<K, V>) -> Self
    where
        V: Clone,
    {
        let mut d = Self::with_capacity(other.len());
        for (k, v) in other.iter() {
            d.insert_new_slot(k.clone(), v.clone());
        }
        d
    }


    /// `dict.Add(key, value)` with the key's C# `ToString()` supplied by the
    /// caller, for key types without a [`CsToString`] impl.
    #[track_caller]
    pub fn add_with(&mut self, key: K, value: V, key_text: impl FnOnce(&K) -> String) {
        check_key(&key);
        if self.index.contains_key(&key) {
            throw(
                "ArgumentException",
                format!("An item with the same key has already been added. Key: {}", key_text(&key)),
            );
        }
        self.insert_new_slot(key, value);
    }

    /// `dict[key]` read with the key's C# `ToString()` supplied by the caller.
    #[track_caller]
    pub fn index_with(&self, key: &K, key_text: impl FnOnce(&K) -> String) -> &V {
        match self.get(key) {
            Some(v) => v,
            None => key_not_found(key_text(key)),
        }
    }

    /// Number of slots in use including freed ones (.NET `_count`). Test aid.
    #[doc(hidden)]
    pub fn debug_slot_count(&self) -> usize {
        self.entries.len()
    }
}

#[track_caller]
fn key_not_found(key_text: String) -> ! {
    throw("KeyNotFoundException", format!("The given key '{key_text}' was not present in the dictionary."))
}

/// The throwing operations. Their messages embed the key's C# `ToString()`.
impl<K: Eq + Hash + Clone + CsToString + 'static, V> NetDictionary<K, V> {
    /// `dict.Add(key, value)`. Throws ArgumentException "An item with the same
    /// key has already been added. Key: {key}" on a duplicate.
    #[track_caller]
    pub fn add(&mut self, key: K, value: V) {
        self.add_with(key, value, |k| k.cs_to_string())
    }

    /// `dict[key]` read. Throws KeyNotFoundException "The given key '{key}'
    /// was not present in the dictionary."
    #[track_caller]
    pub fn index(&self, key: &K) -> &V {
        self.index_with(key, |k| k.cs_to_string())
    }

    /// `dict[key]` for in-place mutation of the value; throws like [`Self::index`].
    #[track_caller]
    pub fn index_mut(&mut self, key: &K) -> &mut V {
        if !self.contains_key(key) {
            key_not_found(key.cs_to_string());
        }
        self.get_mut(key).unwrap()
    }

    /// `source.ToDictionary(keySelector, valueSelector)`: `Add` in source
    /// order, so a duplicate key throws like [`Self::add`].
    #[track_caller]
    pub fn to_dictionary<T>(
        source: impl IntoIterator<Item = T>,
        mut key: impl FnMut(&T) -> K,
        mut value: impl FnMut(T) -> V,
    ) -> Self {
        let mut d = Self::new();
        for item in source {
            let k = key(&item);
            d.add(k, value(item));
        }
        d
    }
}

impl<K: Eq + Hash + Clone + CsToString + 'static, V> std::ops::Index<&K> for NetDictionary<K, V> {
    type Output = V;
    fn index(&self, key: &K) -> &V {
        NetDictionary::index(self, key)
    }
}

impl<K: Eq + Hash + Clone + CsToString + 'static, V> std::ops::IndexMut<&K> for NetDictionary<K, V> {
    #[track_caller]
    fn index_mut(&mut self, key: &K) -> &mut V {
        self.get_mut(key).expect("key not in the dictionary")
    }
}

/// Collecting uses `Add` semantics (throws on duplicate keys), matching
/// `ToDictionary` and `new Dictionary<K,V>(IEnumerable<KeyValuePair>)`.
impl<K: Eq + Hash + Clone + CsToString + 'static, V> FromIterator<(K, V)> for NetDictionary<K, V> {
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        let mut d = Self::new();
        for (k, v) in iter {
            d.add(k, v);
        }
        d
    }
}

/// Equal when both enumerate the same (key, value) pairs in the same order.
/// (C# `==` on dictionaries is reference equality; this is a Rust convenience.)
impl<K: Eq + Hash + Clone + 'static, V: PartialEq> PartialEq for NetDictionary<K, V> {
    fn eq(&self, other: &Self) -> bool {
        self.len() == other.len() && self.iter().zip(other.iter()).all(|(a, b)| a == b)
    }
}
impl<K: Eq + Hash + Clone + 'static, V: Eq> Eq for NetDictionary<K, V> {}

impl<'a, K: Eq + Hash + Clone + 'static, V> IntoIterator for &'a NetDictionary<K, V> {
    type Item = (&'a K, &'a V);
    type IntoIter = Box<dyn Iterator<Item = (&'a K, &'a V)> + 'a>;
    fn into_iter(self) -> Self::IntoIter {
        Box::new(self.iter())
    }
}

impl<K: serde::Serialize + Eq + Hash + Clone + 'static, V: serde::Serialize> serde::Serialize for NetDictionary<K, V> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_map(self.iter())
    }
}

impl<'de, K, V> serde::Deserialize<'de> for NetDictionary<K, V>
where
    K: serde::Deserialize<'de> + Eq + Hash + Clone + 'static,
    V: serde::Deserialize<'de>,
{
    /// Inserts with indexer semantics in document order (what System.Text.Json
    /// and Newtonsoft do), so a duplicate key keeps its first slot and takes
    /// the last value.
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Visitor<K, V>(std::marker::PhantomData<(K, V)>);
        impl<'de, K, V> serde::de::Visitor<'de> for Visitor<K, V>
        where
            K: serde::Deserialize<'de> + Eq + Hash + Clone + 'static,
            V: serde::Deserialize<'de>,
        {
            type Value = NetDictionary<K, V>;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a map")
            }
            fn visit_map<A: serde::de::MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut d = NetDictionary::new();
                while let Some((k, v)) = map.next_entry()? {
                    d.insert(k, v);
                }
                Ok(d)
            }
        }
        deserializer.deserialize_map(Visitor(std::marker::PhantomData))
    }
}
