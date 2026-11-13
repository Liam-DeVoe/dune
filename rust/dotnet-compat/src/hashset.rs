//! `System.Collections.Generic.HashSet<T>` with .NET's exact enumeration order
//! (same slot/free-list rules as Dictionary).
//!
//! Unlike Dictionary, HashSet's copy constructor `new HashSet<T>(otherSet)`
//! sometimes clones the slot array verbatim (holes and free list included) and
//! sometimes compacts it, depending on the source's internal capacity. To get
//! that right this type also tracks .NET's capacity (`_entries.Length`).

use std::collections::HashMap;
use std::hash::Hash;

/// `HashHelpers.Primes`
const PRIMES: [usize; 72] = [
    3, 7, 11, 17, 23, 29, 37, 47, 59, 71, 89, 107, 131, 163, 197, 239, 293, 353, 431, 521, 631, 761, 919, 1103, 1327,
    1597, 1931, 2333, 2801, 3371, 4049, 4861, 5839, 7013, 8419, 10103, 12143, 14591, 17519, 21023, 25229, 30293, 36353,
    43627, 52361, 62851, 75431, 90523, 108631, 130363, 156437, 187751, 225307, 270371, 324449, 389357, 467237, 560689,
    672827, 807403, 968897, 1162687, 1395263, 1674319, 2009191, 2411033, 2893249, 3471899, 4166287, 4999559, 5999471,
    7199369,
];
const HASH_PRIME: usize = 101;
const MAX_PRIME_ARRAY_LENGTH: usize = 0x7FFF_FFC3;

fn is_prime(candidate: usize) -> bool {
    if candidate & 1 != 0 {
        let limit = (candidate as f64).sqrt() as usize;
        let mut divisor = 3;
        while divisor <= limit {
            if candidate % divisor == 0 {
                return false;
            }
            divisor += 2;
        }
        return true;
    }
    candidate == 2
}

/// `HashHelpers.GetPrime`
pub(crate) fn get_prime(min: usize) -> usize {
    for &p in PRIMES.iter() {
        if p >= min {
            return p;
        }
    }
    let mut i = min | 1;
    while i < i32::MAX as usize {
        if is_prime(i) && (i - 1) % HASH_PRIME != 0 {
            return i;
        }
        i += 2;
    }
    min
}

/// `HashHelpers.ExpandPrime`
pub(crate) fn expand_prime(old_size: usize) -> usize {
    let new_size = 2 * old_size;
    if new_size > MAX_PRIME_ARRAY_LENGTH && MAX_PRIME_ARRAY_LENGTH > old_size {
        return MAX_PRIME_ARRAY_LENGTH;
    }
    get_prime(new_size)
}

#[derive(Clone, Debug)]
pub struct NetHashSet<T> {
    /// Slot array (`_entries[0.._count]`). `None` is a freed slot.
    entries: Vec<Option<T>>,
    /// Free slots, top of stack = head of .NET's `_freeList`.
    free: Vec<usize>,
    index: HashMap<T, usize>,
    /// .NET `_entries.Length` (0 while `_buckets` is null).
    capacity: usize,
}

impl<T: Eq + Hash + Clone> Default for NetHashSet<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Eq + Hash + Clone> NetHashSet<T> {
    /// `new HashSet<T>()`
    pub fn new() -> Self {
        NetHashSet { entries: Vec::new(), free: Vec::new(), index: HashMap::new(), capacity: 0 }
    }

    /// `new HashSet<T>(capacity)`
    pub fn with_capacity(capacity: usize) -> Self {
        let mut s = Self::new();
        if capacity > 0 {
            s.initialize(capacity);
        }
        s
    }

    /// `Initialize(capacity)`: allocates a fresh, empty slot array.
    fn initialize(&mut self, capacity: usize) {
        self.capacity = get_prime(capacity);
        self.entries.clear();
        self.free.clear();
        self.index.clear();
    }

    /// `AddIfNotPresent`; returns the slot and whether it was added.
    fn add_or_get_location(&mut self, item: T) -> (usize, bool) {
        if self.capacity == 0 {
            self.initialize(0);
        }
        if let Some(&slot) = self.index.get(&item) {
            return (slot, false);
        }
        let slot = match self.free.pop() {
            Some(slot) => {
                self.entries[slot] = Some(item.clone());
                slot
            }
            None => {
                if self.entries.len() == self.capacity {
                    self.capacity = expand_prime(self.entries.len());
                }
                self.entries.push(Some(item.clone()));
                self.entries.len() - 1
            }
        };
        self.index.insert(item, slot);
        (slot, true)
    }

    /// `set.Add(x)`
    /// The same as [`add`](Self::add), under the std name.
    pub fn insert(&mut self, item: T) -> bool {
        self.add(item)
    }

    pub fn add(&mut self, item: T) -> bool {
        self.add_or_get_location(item).1
    }

    /// `set.Remove(x)`. Frees the slot.
    pub fn remove(&mut self, item: &T) -> bool {
        match self.index.remove(item) {
            Some(slot) => {
                self.entries[slot] = None;
                self.free.push(slot);
                true
            }
            None => false,
        }
    }

    pub fn contains(&self, item: &T) -> bool {
        self.index.contains_key(item)
    }

    /// `set.Clear()`: resets the slot array (capacity is kept).
    pub fn clear(&mut self) {
        self.entries.clear();
        self.free.clear();
        self.index.clear();
    }

    /// `set.Count`
    pub fn len(&self) -> usize {
        self.index.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// `foreach (var x in set)`
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.entries.iter().filter_map(|e| e.as_ref())
    }

    /// `new HashSet<T>(otherHashSet)` (same comparer). If the source is not
    /// oversized this clones the slot array exactly, holes and free list
    /// included, so later adds fill the source's freed slots. Otherwise it
    /// re-adds the live items compactly in enumeration order.
    pub fn from_set(source: &NetHashSet<T>) -> Self {
        let mut s = Self::new();
        if source.len() == 0 {
            return s;
        }
        let capacity = source.capacity;
        let threshold = expand_prime(source.len() + 1);
        if threshold >= capacity {
            s = source.clone();
        } else {
            s.initialize(source.len());
            for item in source.iter() {
                s.add(item.clone());
            }
        }
        s
    }

    /// `new HashSet<T>(collection)` where `collection` is an `ICollection<T>`
    /// that is not a HashSet (List, array, Dictionary.Keys, ...): presizes to
    /// the collection's Count, adds in order, then trims if oversized.
    pub fn from_collection<'a>(collection: impl IntoIterator<Item = &'a T, IntoIter: ExactSizeIterator>) -> Self
    where
        T: 'a,
    {
        let it = collection.into_iter();
        let mut s = Self::new();
        let count = it.len();
        if count > 0 {
            s.initialize(count);
        }
        for item in it {
            s.add(item.clone());
        }
        s.shrink_after_construct();
        s
    }

    /// `new HashSet<T>(enumerable)` / `enumerable.ToHashSet()` where the source
    /// is a lazy IEnumerable (a LINQ query, an iterator method): no presizing.
    pub fn from_enumerable(items: impl IntoIterator<Item = T>) -> Self {
        let mut s = Self::new();
        for item in items {
            s.add(item);
        }
        s.shrink_after_construct();
        s
    }

    fn shrink_after_construct(&mut self) {
        // if (_count > 0 && _entries!.Length / _count > ShrinkThreshold) TrimExcess();
        if !self.entries.is_empty() && self.capacity / self.entries.len() > 3 {
            self.trim_excess();
        }
    }

    /// `set.TrimExcess()`: compacts the slot array, keeping enumeration order.
    pub fn trim_excess(&mut self) {
        let new_size = get_prime(self.len());
        if new_size >= self.capacity {
            return;
        }
        let items: Vec<T> = self.entries.drain(..).flatten().collect();
        self.initialize(new_size);
        for (i, item) in items.into_iter().enumerate() {
            self.entries.push(Some(item.clone()));
            self.index.insert(item, i);
        }
    }

    /// `set.UnionWith(other)`: adds each item of `other` in order.
    pub fn union_with(&mut self, other: impl IntoIterator<Item = T>) {
        for item in other {
            self.add(item);
        }
    }

    /// `set.ExceptWith(other)`: removes each item of `other` in `other`'s order
    /// (so the free list order follows `other`). No-op when this set is empty.
    pub fn except_with<'a>(&mut self, other: impl IntoIterator<Item = &'a T>)
    where
        T: 'a,
    {
        if self.len() == 0 {
            return;
        }
        for item in other {
            self.remove(item);
        }
    }

    fn remove_slots_not_marked(&mut self, original_count: usize, keep: &[bool]) {
        for i in 0..original_count {
            if !keep[i] {
                if let Some(item) = self.entries[i].clone() {
                    self.remove(&item);
                }
            }
        }
    }

    /// `set.IntersectWith(other)` where `other` is a lazy IEnumerable (not an
    /// ICollection): removes non-members in slot order, even if `other` is empty.
    pub fn intersect_with_enumerable<'a>(&mut self, other: impl IntoIterator<Item = &'a T>)
    where
        T: 'a,
    {
        if self.len() == 0 {
            return;
        }
        let original_count = self.entries.len();
        let mut keep = vec![false; original_count];
        for item in other {
            if let Some(&slot) = self.index.get(item) {
                keep[slot] = true;
            }
        }
        self.remove_slots_not_marked(original_count, &keep);
    }

    /// `set.IntersectWith(other)` where `other` is an ICollection (List, array,
    /// HashSet, Keys, ...). An empty `other` calls `Clear()` (resetting the slot
    /// array); otherwise removes non-members in slot order.
    pub fn intersect_with_collection<'a>(&mut self, other: impl IntoIterator<Item = &'a T, IntoIter: ExactSizeIterator>)
    where
        T: 'a,
    {
        if self.len() == 0 {
            return;
        }
        let it = other.into_iter();
        if it.len() == 0 {
            self.clear();
            return;
        }
        self.intersect_with_enumerable(it);
    }

    /// `set.IntersectWith(otherHashSet)`.
    pub fn intersect_with_set(&mut self, other: &NetHashSet<T>) {
        if self.len() == 0 {
            return;
        }
        if other.len() == 0 {
            self.clear();
            return;
        }
        let original_count = self.entries.len();
        let keep: Vec<bool> =
            self.entries.iter().map(|e| e.as_ref().map(|x| other.contains(x)).unwrap_or(true)).collect();
        self.remove_slots_not_marked(original_count, &keep);
    }

    /// `set.SymmetricExceptWith(otherHashSet)`: for each item of `other` in its
    /// order, remove it if present, else add it.
    pub fn symmetric_except_with_set(&mut self, other: &NetHashSet<T>) {
        if self.len() == 0 {
            self.union_with(other.iter().cloned());
            return;
        }
        for item in other.iter() {
            if !self.remove(item) {
                self.add(item.clone());
            }
        }
    }

    /// `set.SymmetricExceptWith(other)` where `other` is not a HashSet (it may
    /// contain duplicates). Adds new items in `other`'s order, then removes the
    /// original members that appeared in `other`, in slot order.
    pub fn symmetric_except_with(&mut self, other: impl IntoIterator<Item = T>) {
        if self.len() == 0 {
            self.union_with(other);
            return;
        }
        let original_count = self.entries.len();
        let mut to_remove = vec![false; original_count];
        let mut added_from_other = vec![false; original_count];
        for item in other {
            let (location, added) = self.add_or_get_location(item);
            if added {
                if location < original_count {
                    added_from_other[location] = true;
                }
            } else if location < original_count && !added_from_other[location] {
                to_remove[location] = true;
            }
        }
        for i in 0..original_count {
            if to_remove[i] {
                if let Some(item) = self.entries[i].clone() {
                    self.remove(&item);
                }
            }
        }
    }

    /// `set.RemoveWhere(pred)`: tests and removes in slot order; returns the
    /// number removed.
    pub fn remove_where(&mut self, mut pred: impl FnMut(&T) -> bool) -> usize {
        let mut removed = 0;
        for i in 0..self.entries.len() {
            let hit = match &self.entries[i] {
                Some(item) => pred(item),
                None => false,
            };
            if hit {
                let item = self.entries[i].clone().unwrap();
                self.remove(&item);
                removed += 1;
            }
        }
        removed
    }
}

/// Collecting uses `new HashSet<T>(IEnumerable)` / `ToHashSet()` semantics for
/// a lazy source. Use [`NetHashSet::from_collection`] for a List/array source
/// and [`NetHashSet::from_set`] for a HashSet source; they only differ in
/// internal capacity, which matters if the result is later copied with
/// `from_set`.
impl<T: Eq + Hash + Clone> FromIterator<T> for NetHashSet<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self::from_enumerable(iter)
    }
}

/// Equal when both enumerate the same items in the same order.
impl<T: Eq + Hash + Clone> PartialEq for NetHashSet<T> {
    fn eq(&self, other: &Self) -> bool {
        self.len() == other.len() && self.iter().zip(other.iter()).all(|(a, b)| a == b)
    }
}
impl<T: Eq + Hash + Clone> Eq for NetHashSet<T> {}

impl<'a, T: Eq + Hash + Clone> IntoIterator for &'a NetHashSet<T> {
    type Item = &'a T;
    type IntoIter = Box<dyn Iterator<Item = &'a T> + 'a>;
    fn into_iter(self) -> Self::IntoIter {
        Box::new(self.iter())
    }
}

impl<T: serde::Serialize + Eq + Hash + Clone> serde::Serialize for NetHashSet<T> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_seq(self.iter())
    }
}

impl<'de, T: serde::Deserialize<'de> + Eq + Hash + Clone> serde::Deserialize<'de> for NetHashSet<T> {
    /// Adds in document order, ignoring duplicates (like the JSON libraries'
    /// `HashSet.Add`).
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Visitor<T>(std::marker::PhantomData<T>);
        impl<'de, T: serde::Deserialize<'de> + Eq + Hash + Clone> serde::de::Visitor<'de> for Visitor<T> {
            type Value = NetHashSet<T>;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a sequence")
            }
            fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut s = NetHashSet::new();
                while let Some(x) = seq.next_element()? {
                    s.add(x);
                }
                Ok(s)
            }
        }
        deserializer.deserialize_seq(Visitor(std::marker::PhantomData))
    }
}
