//! LINQ operators whose .NET semantics differ from the obvious Rust idiom.
//!
//! Operators that are plain iterator adapters with identical semantics
//! (`Where` = `filter`, `Select` = `map`, `Any`, `All`, `Count`, `Concat` =
//! `chain`, `SelectMany` = `flat_map`, `Contains`) have no helper here.
//!
//! The main traps:
//! - `MaxBy`/`MinBy` return the FIRST extreme element; Rust's
//!   `Iterator::max_by_key` returns the LAST maximum (`min_by_key` returns the
//!   first minimum). Use [`max_by_key_first`] / [`min_by_key_first`].
//! - `Max()`/`Min()` throw on an empty sequence of a value type; on nullable or
//!   reference types they skip nulls and return null when nothing is left.
//! - `Max`/`Min` over doubles have their own NaN rules, and comparisons in
//!   OrderBy/Sort use `CompareTo`, where NaN is the smallest value.
//! - `OrderByDescending` is a stable sort with the comparison reversed. It is
//!   NOT `OrderBy(...).Reverse()`: equal keys keep their source order.
//! - `SingleOrDefault` throws when there is more than one element.
//! - `Sum` over ints is checked: overflow throws. `Sum`/`Average` over floats
//!   accumulate in double and convert back to float at the end.

use crate::exception::{
    MORE_THAN_ONE_ELEMENT, MORE_THAN_ONE_MATCH, NO_ELEMENTS, NO_MATCH, invalid_operation, overflow, throw,
};
use crate::math::cmp_f64;
use crate::{NetDictionary, NetHashSet};
use std::cmp::Ordering;
use std::collections::HashSet;
use std::hash::Hash;

#[track_caller]
fn no_elements() -> ! {
    invalid_operation(NO_ELEMENTS)
}

// ---------------------------------------------------------------------------
// MaxBy / MinBy / Max / Min
// ---------------------------------------------------------------------------

/// Turns the `None` of an empty `MaxBy`/`MinBy` into .NET's throw, for
/// value-type sources: `or_no_elements(max_by_key_first(xs, key))`.
#[track_caller]
pub fn or_no_elements<T>(value: Option<T>) -> T {
    value.unwrap_or_else(|| no_elements())
}

/// `source.MaxBy(key)`: the first element with the largest key. `None` for an
/// empty source (.NET returns null for reference types and throws for value
/// types; call `.expect()` for the latter).
pub fn max_by_key_first<T, K: Ord>(source: impl IntoIterator<Item = T>, mut key: impl FnMut(&T) -> K) -> Option<T> {
    let mut best: Option<(K, T)> = None;
    for item in source {
        let k = key(&item);
        match &best {
            Some((bk, _)) if k <= *bk => {}
            _ => best = Some((k, item)),
        }
    }
    best.map(|(_, t)| t)
}

/// `source.MinBy(key)`: the first element with the smallest key.
pub fn min_by_key_first<T, K: Ord>(source: impl IntoIterator<Item = T>, mut key: impl FnMut(&T) -> K) -> Option<T> {
    let mut best: Option<(K, T)> = None;
    for item in source {
        let k = key(&item);
        match &best {
            Some((bk, _)) if k >= *bk => {}
            _ => best = Some((k, item)),
        }
    }
    best.map(|(_, t)| t)
}

/// `source.MaxBy(key, comparer)` / first maximum under an arbitrary comparison
/// (e.g. [`crate::math::cmp_f64`] for double keys).
pub fn max_by_first<T>(source: impl IntoIterator<Item = T>, mut cmp: impl FnMut(&T, &T) -> Ordering) -> Option<T> {
    let mut best: Option<T> = None;
    for item in source {
        match &best {
            Some(b) if cmp(&item, b) != Ordering::Greater => {}
            _ => best = Some(item),
        }
    }
    best
}

/// `source.MinBy(key, comparer)` / first minimum under an arbitrary comparison.
pub fn min_by_first<T>(source: impl IntoIterator<Item = T>, mut cmp: impl FnMut(&T, &T) -> Ordering) -> Option<T> {
    let mut best: Option<T> = None;
    for item in source {
        match &best {
            Some(b) if cmp(&item, b) != Ordering::Less => {}
            _ => best = Some(item),
        }
    }
    best
}

/// `source.Max()` / `source.Max(selector)` for a non-nullable value type
/// (int, enum, ...): throws on an empty sequence.
pub fn max<T: Ord>(source: impl IntoIterator<Item = T>) -> T {
    max_by_first(source, |a, b| a.cmp(b)).unwrap_or_else(|| no_elements())
}

/// `source.Min()` for a non-nullable value type: throws on empty.
pub fn min<T: Ord>(source: impl IntoIterator<Item = T>) -> T {
    min_by_first(source, |a, b| a.cmp(b)).unwrap_or_else(|| no_elements())
}

/// `source.Max()` over `int?` (or `Max(selector)` where the selector returns a
/// nullable or reference type, like `IComparable`): nulls are skipped, `None`
/// when no non-null value exists. Never panics. Returns the first maximum.
pub fn max_nullable<T: Ord>(source: impl IntoIterator<Item = Option<T>>) -> Option<T> {
    max_by_first(source.into_iter().flatten(), |a, b| a.cmp(b))
}

/// `source.Min()` over a nullable / reference type; see [`max_nullable`].
pub fn min_nullable<T: Ord>(source: impl IntoIterator<Item = Option<T>>) -> Option<T> {
    min_by_first(source.into_iter().flatten(), |a, b| a.cmp(b))
}

/// `IEnumerable<double>.Max()`: throws on empty. Leading NaNs are skipped and
/// NaN is only returned if every element is NaN. Uses `>`, so of -0.0 and 0.0
/// the first one seen wins.
pub fn max_f64(source: impl IntoIterator<Item = f64>) -> f64 {
    let mut it = source.into_iter();
    let mut value = it.next().unwrap_or_else(|| no_elements());
    while value.is_nan() {
        match it.next() {
            Some(x) => value = x,
            None => return value,
        }
    }
    for x in it {
        if x > value {
            value = x;
        }
    }
    value
}

/// `IEnumerable<double>.Min()`: throws on empty; returns NaN as soon as any
/// element is NaN.
pub fn min_f64(source: impl IntoIterator<Item = f64>) -> f64 {
    let mut it = source.into_iter();
    let mut value = it.next().unwrap_or_else(|| no_elements());
    if value.is_nan() {
        return value;
    }
    for x in it {
        if x < value {
            value = x;
        } else if x.is_nan() {
            return x;
        }
    }
    value
}

/// `IEnumerable<float>.Max()`; see [`max_f64`].
pub fn max_f32(source: impl IntoIterator<Item = f32>) -> f32 {
    max_f64(source.into_iter().map(|x| x as f64)) as f32
}

/// `IEnumerable<float>.Min()`; see [`min_f64`].
pub fn min_f32(source: impl IntoIterator<Item = f32>) -> f32 {
    min_f64(source.into_iter().map(|x| x as f64)) as f32
}

// ---------------------------------------------------------------------------
// Sum / Average / Aggregate
// ---------------------------------------------------------------------------

/// `Sum()` over ints: checked, throws OverflowException.
pub fn sum_i32(source: impl IntoIterator<Item = i32>) -> i32 {
    let mut sum: i32 = 0;
    for x in source {
        sum = sum.checked_add(x).unwrap_or_else(|| overflow());
    }
    sum
}

/// `Sum()` over longs: checked.
pub fn sum_i64(source: impl IntoIterator<Item = i64>) -> i64 {
    let mut sum: i64 = 0;
    for x in source {
        sum = sum.checked_add(x).unwrap_or_else(|| overflow());
    }
    sum
}

/// `Sum()` over floats: accumulates in double, converts to float at the end.
pub fn sum_f32(source: impl IntoIterator<Item = f32>) -> f32 {
    let mut sum = 0f64;
    for x in source {
        sum += x as f64;
    }
    sum as f32
}

/// `Sum()` over doubles.
pub fn sum_f64(source: impl IntoIterator<Item = f64>) -> f64 {
    let mut sum = 0f64;
    for x in source {
        sum += x;
    }
    sum
}

/// `Average()` over ints: long accumulator, double result. Throws on empty.
pub fn average_i32(source: impl IntoIterator<Item = i32>) -> f64 {
    let mut sum: i64 = 0;
    let mut count: i64 = 0;
    for x in source {
        sum = sum.checked_add(x as i64).unwrap_or_else(|| overflow());
        count += 1;
    }
    if count == 0 {
        no_elements();
    }
    sum as f64 / count as f64
}

/// `Average()` over floats: double accumulator, float result. Throws on empty.
pub fn average_f32(source: impl IntoIterator<Item = f32>) -> f32 {
    // .NET seeds the accumulator with the first element (so -0.0 survives).
    let mut it = source.into_iter();
    let mut sum = it.next().unwrap_or_else(|| no_elements()) as f64;
    let mut count: i64 = 1;
    for x in it {
        sum += x as f64;
        count += 1;
    }
    (sum / count as f64) as f32
}

/// `Average()` over doubles. Throws on empty.
pub fn average_f64(source: impl IntoIterator<Item = f64>) -> f64 {
    let mut it = source.into_iter();
    let mut sum = it.next().unwrap_or_else(|| no_elements());
    let mut count: i64 = 1;
    for x in it {
        sum += x;
        count += 1;
    }
    sum / count as f64
}

/// `Aggregate(func)` without a seed: throws on empty.
pub fn aggregate<T>(source: impl IntoIterator<Item = T>, func: impl FnMut(T, T) -> T) -> T {
    source.into_iter().reduce(func).unwrap_or_else(|| no_elements())
}

/// `Aggregate(seed, func)`: a left fold.
pub fn aggregate_seed<T, A>(source: impl IntoIterator<Item = T>, seed: A, func: impl FnMut(A, T) -> A) -> A {
    source.into_iter().fold(seed, func)
}

// ---------------------------------------------------------------------------
// Ordering
// ---------------------------------------------------------------------------

/// `source.OrderBy(key)`: stable ascending sort. The key selector is called
/// exactly once per element, in source order, before sorting (matters for
/// side-effecting selectors like `_ => random.Next()`).
pub fn order_by<T, K: Ord>(source: impl IntoIterator<Item = T>, mut key: impl FnMut(&T) -> K) -> Vec<T> {
    let mut keyed: Vec<(K, T)> = source.into_iter().map(|t| (key(&t), t)).collect();
    keyed.sort_by(|a, b| a.0.cmp(&b.0));
    keyed.into_iter().map(|(_, t)| t).collect()
}

/// `source.OrderByDescending(key)`: stable descending sort. Elements with equal
/// keys stay in source order (this is not `order_by(...).reverse()`).
pub fn order_by_descending<T, K: Ord>(source: impl IntoIterator<Item = T>, mut key: impl FnMut(&T) -> K) -> Vec<T> {
    let mut keyed: Vec<(K, T)> = source.into_iter().map(|t| (key(&t), t)).collect();
    keyed.sort_by(|a, b| b.0.cmp(&a.0));
    keyed.into_iter().map(|(_, t)| t).collect()
}

/// `OrderBy`/`ThenBy` chains with an arbitrary comparison, e.g.
/// `order_by_cmp(xs, |a, b| a.x.cmp(&b.x).then_with(|| b.y.cmp(&a.y)))` for
/// `OrderBy(x).ThenByDescending(y)`. Stable.
pub fn order_by_cmp<T>(source: impl IntoIterator<Item = T>, cmp: impl FnMut(&T, &T) -> Ordering) -> Vec<T> {
    let mut v: Vec<T> = source.into_iter().collect();
    v.sort_by(cmp);
    v
}

/// `OrderBy` with a double key (`Comparer<double>.Default`: NaN first).
pub fn order_by_f64<T>(source: impl IntoIterator<Item = T>, mut key: impl FnMut(&T) -> f64) -> Vec<T> {
    let mut keyed: Vec<(f64, T)> = source.into_iter().map(|t| (key(&t), t)).collect();
    keyed.sort_by(|a, b| cmp_f64(a.0, b.0));
    keyed.into_iter().map(|(_, t)| t).collect()
}

/// `OrderByDescending` with a double key (NaN last).
pub fn order_by_descending_f64<T>(source: impl IntoIterator<Item = T>, mut key: impl FnMut(&T) -> f64) -> Vec<T> {
    let mut keyed: Vec<(f64, T)> = source.into_iter().map(|t| (key(&t), t)).collect();
    keyed.sort_by(|a, b| cmp_f64(b.0, a.0));
    keyed.into_iter().map(|(_, t)| t).collect()
}

/// `source.Reverse()`
pub fn reverse<T>(source: impl IntoIterator<Item = T>) -> Vec<T> {
    let mut v: Vec<T> = source.into_iter().collect();
    v.reverse();
    v
}

// ---------------------------------------------------------------------------
// Set operators (results are in first-occurrence order)
// ---------------------------------------------------------------------------

/// `source.Distinct()`: first occurrence of each element, in source order.
pub fn distinct<T: Eq + Hash + Clone>(source: impl IntoIterator<Item = T>) -> Vec<T> {
    let mut seen = HashSet::new();
    source.into_iter().filter(|x| seen.insert(x.clone())).collect()
}

/// `source.DistinctBy(key)`: first element for each key.
pub fn distinct_by<T, K: Eq + Hash>(source: impl IntoIterator<Item = T>, mut key: impl FnMut(&T) -> K) -> Vec<T> {
    let mut seen = HashSet::new();
    source.into_iter().filter(|x| seen.insert(key(x))).collect()
}

/// `first.Union(second)`: `Distinct` over `first` followed by `second`.
pub fn union<T: Eq + Hash + Clone>(first: impl IntoIterator<Item = T>, second: impl IntoIterator<Item = T>) -> Vec<T> {
    distinct(first.into_iter().chain(second))
}

/// `first.Intersect(second)`: distinct elements of `first` (in `first`'s
/// order) that occur in `second`.
pub fn intersect<T: Eq + Hash>(first: impl IntoIterator<Item = T>, second: impl IntoIterator<Item = T>) -> Vec<T> {
    let mut set: HashSet<T> = second.into_iter().collect();
    first.into_iter().filter(|x| set.remove(x)).collect()
}

/// `first.Except(second)`: distinct elements of `first` (in `first`'s order)
/// that do not occur in `second`.
pub fn except<T: Eq + Hash + Clone>(first: impl IntoIterator<Item = T>, second: impl IntoIterator<Item = T>) -> Vec<T> {
    let mut set: HashSet<T> = second.into_iter().collect();
    first.into_iter().filter(|x| set.insert(x.clone())).collect()
}

/// `source.GroupBy(key)`: groups in order of each key's first occurrence;
/// elements within a group in source order.
pub fn group_by<T, K: Eq + Hash + Clone>(source: impl IntoIterator<Item = T>, mut key: impl FnMut(&T) -> K) -> Vec<(K, Vec<T>)> {
    let mut index: std::collections::HashMap<K, usize> = std::collections::HashMap::new();
    let mut groups: Vec<(K, Vec<T>)> = Vec::new();
    for item in source {
        let k = key(&item);
        match index.get(&k) {
            Some(&i) => groups[i].1.push(item),
            None => {
                index.insert(k.clone(), groups.len());
                groups.push((k, vec![item]));
            }
        }
    }
    groups
}

/// `first.SequenceEqual(second)`
pub fn sequence_equal<T: PartialEq>(first: impl IntoIterator<Item = T>, second: impl IntoIterator<Item = T>) -> bool {
    first.into_iter().eq(second)
}

// ---------------------------------------------------------------------------
// Element operators
// ---------------------------------------------------------------------------

/// `First()`: throws on empty. (`FirstOrDefault()` is `.next()`.)
pub fn first<T>(source: impl IntoIterator<Item = T>) -> T {
    source.into_iter().next().unwrap_or_else(|| no_elements())
}

/// `Last()`: throws on empty. (`LastOrDefault()` is `.last()`.)
pub fn last<T>(source: impl IntoIterator<Item = T>) -> T {
    source.into_iter().last().unwrap_or_else(|| no_elements())
}

/// `First(predicate)`: throws "Sequence contains no matching element".
#[track_caller]
pub fn first_where<T>(source: impl IntoIterator<Item = T>, predicate: impl FnMut(&T) -> bool) -> T {
    source.into_iter().find(predicate).unwrap_or_else(|| invalid_operation(NO_MATCH))
}

/// `Last(predicate)`: throws "Sequence contains no matching element".
#[track_caller]
pub fn last_where<T>(source: impl IntoIterator<Item = T>, predicate: impl FnMut(&T) -> bool) -> T {
    source.into_iter().filter(predicate).last().unwrap_or_else(|| invalid_operation(NO_MATCH))
}

/// `Single()`: throws if there are zero or more than one elements.
#[track_caller]
pub fn single<T>(source: impl IntoIterator<Item = T>) -> T {
    match single_or_default(source) {
        Some(x) => x,
        None => no_elements(),
    }
}

/// `SingleOrDefault()`: `None` if empty, throws if more than one element.
/// For the predicate form use [`single_or_default_where`]: its message differs.
#[track_caller]
pub fn single_or_default<T>(source: impl IntoIterator<Item = T>) -> Option<T> {
    let mut it = source.into_iter();
    let first = it.next()?;
    if it.next().is_some() {
        invalid_operation(MORE_THAN_ONE_ELEMENT);
    }
    Some(first)
}

/// `SingleOrDefault(predicate)`: `None` if nothing matches, throws "Sequence
/// contains more than one matching element" on a second match. Like .NET, the
/// predicate runs on every element up to the second match.
#[track_caller]
pub fn single_or_default_where<T>(source: impl IntoIterator<Item = T>, mut predicate: impl FnMut(&T) -> bool) -> Option<T> {
    let mut found = None;
    for item in source {
        if predicate(&item) {
            if found.is_some() {
                invalid_operation(MORE_THAN_ONE_MATCH);
            }
            found = Some(item);
        }
    }
    found
}

/// `Single(predicate)`: throws on no match or on a second match.
#[track_caller]
pub fn single_where<T>(source: impl IntoIterator<Item = T>, predicate: impl FnMut(&T) -> bool) -> T {
    single_or_default_where(source, predicate).unwrap_or_else(|| invalid_operation(NO_MATCH))
}

/// `ElementAt(index)` on a lazy sequence (not an `IList`): throws
/// ArgumentOutOfRangeException "Specified argument was out of the range of
/// valid values. (Parameter 'index')".
#[track_caller]
pub fn element_at<T>(source: impl IntoIterator<Item = T>, index: i32) -> T {
    let out_of_range = || -> ! {
        throw(
            "ArgumentOutOfRangeException",
            "Specified argument was out of the range of valid values. (Parameter 'index')".to_string(),
        )
    };
    if index < 0 {
        out_of_range();
    }
    source.into_iter().nth(index as usize).unwrap_or_else(|| out_of_range())
}

/// `ElementAt(index)` on a `List<T>` or array (an `IList<T>`): throws with
/// the list indexer's message.
#[track_caller]
pub fn element_at_list<T>(source: &[T], index: i32) -> &T {
    if index < 0 || index as usize >= source.len() {
        throw(
            "ArgumentOutOfRangeException",
            "Index was out of range. Must be non-negative and less than the size of the collection. (Parameter 'index')"
                .to_string(),
        );
    }
    &source[index as usize]
}

/// `Take(count)`: a negative count takes nothing.
pub fn take<T>(source: impl IntoIterator<Item = T>, count: i32) -> impl Iterator<Item = T> {
    source.into_iter().take(count.max(0) as usize)
}

/// `Skip(count)`: a negative count skips nothing.
pub fn skip<T>(source: impl IntoIterator<Item = T>, count: i32) -> impl Iterator<Item = T> {
    source.into_iter().skip(count.max(0) as usize)
}

// ---------------------------------------------------------------------------
// Conversions
// ---------------------------------------------------------------------------

/// `ToDictionary(key, value)`: throws on a duplicate key like `Dictionary.Add`.
#[track_caller]
pub fn to_dictionary<T, K: Eq + Hash + Clone + crate::exception::CsToString + 'static, V>(
    source: impl IntoIterator<Item = T>,
    key: impl FnMut(&T) -> K,
    value: impl FnMut(T) -> V,
) -> NetDictionary<K, V> {
    NetDictionary::to_dictionary(source, key, value)
}

/// `ToHashSet()` over a lazy sequence.
pub fn to_hash_set<T: Eq + Hash + Clone>(source: impl IntoIterator<Item = T>) -> NetHashSet<T> {
    NetHashSet::from_enumerable(source)
}
