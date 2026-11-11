//! Sorting with .NET semantics.
//!
//! [`list_sort_by`] is a port of `ArraySortHelper<T>` (the `Comparison<T>`
//! path) from `src/libraries/System.Private.CoreLib/src/System/Collections/
//! Generic/ArraySortHelper.cs`. It is an unstable introsort, so the final order
//! of elements that compare equal depends on the exact algorithm; this port
//! reproduces it step for step:
//!
//! - depth limit `2 * (floor(log2(n)) + 1)`
//! - partitions of size <= 16 use insertion sort (sizes 2 and 3 use fixed
//!   compare-and-swap sequences)
//! - median-of-three pivot (`lo`, `hi >> 1`, `hi`), pivot parked at `hi - 1`
//! - heapsort when the depth limit runs out
//! - the right partition is recursed into, the left one is looped on
//!
//! Where the C# code holds a copy of an element (`pivot`, insertion-sort `t`,
//! heapsort `d`), this port keeps the element in place and swaps instead; the
//! sequence of comparisons and resulting permutation are identical.

use crate::exception::{CsException, throw};
use std::cmp::Ordering;

const INTROSORT_SIZE_THRESHOLD: usize = 16;

/// Number of times the heapsort fallback has run in this process. Lets tests
/// check that their adversarial inputs really reach it.
#[doc(hidden)]
pub static HEAPSORT_CALLS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// `List<T>.Sort(Comparison<T>)`, `List<T>.Sort(IComparer<T>)`,
/// `Array.Sort(T[], Comparison<T>)` and `Array.Sort(T[], IComparer<T>)`:
/// .NET's unstable introsort (ArraySortHelper). Order of equal elements
/// matches .NET exactly.
///
/// Exceptions, as in .NET (the slice is left partially sorted in both cases):
/// - An inconsistent comparer can make the partition loop run off the slice.
///   That throws ArgumentException "Unable to sort because the
///   IComparer.Compare() method returns inconsistent results. ...". The
///   message ends with the comparer's type name, which C# prints as
///   ``System.Comparison`1[T]``; this uses the last segment of the Rust type
///   name for `T`, so it only matches when the names agree.
/// - A C# exception thrown by the comparer is wrapped in
///   InvalidOperationException "Failed to compare two elements in the array."
///   (the inner exception is not kept).
pub fn list_sort_by<T>(items: &mut [T], mut cmp: impl FnMut(&T, &T) -> Ordering) {
    let n = items.len();
    if n > 1 {
        let depth_limit = 2 * ((usize::BITS - 1 - n.leading_zeros()) as i32 + 1);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            intro_sort(items, depth_limit, &mut cmp);
        }));
        if let Err(payload) = result {
            if payload.downcast_ref::<RanOffTheEnd>().is_some() {
                let full = std::any::type_name::<T>();
                let short = full.rsplit("::").next().unwrap_or(full);
                throw(
                    "ArgumentException",
                    format!(
                        "Unable to sort because the IComparer.Compare() method returns inconsistent results. \
                         Either a value does not compare equal to itself, or one value repeatedly compared to another \
                         value yields different results. IComparer: 'System.Comparison`1[{short}]'."
                    ),
                );
            }
            if payload.downcast_ref::<CsException>().is_some() {
                throw("InvalidOperationException", "Failed to compare two elements in the array.".to_string());
            }
            std::panic::resume_unwind(payload);
        }
    }
}

/// Internal panic payload: the partition loop left the slice (.NET's
/// IndexOutOfRangeException inside ArraySortHelper).
struct RanOffTheEnd;

fn ran_off_the_end() -> ! {
    std::panic::resume_unwind(Box::new(RanOffTheEnd))
}

/// `List<T>.Sort()` / `Array.Sort(T[])` with the default comparer, for `T`
/// implementing `IComparable<T>` (`GenericArraySortHelper<T>`). That helper
/// is the same algorithm as the comparison path (it calls `x.CompareTo(y)`
/// with operands in the same roles), so for a consistent `Ord` it produces the
/// same permutation as [`list_sort_by`].
///
/// Not exact for `float`/`double` keys: .NET first moves NaNs to the front
/// with swaps, which can reorder `-0.0` and `0.0`. For integer and other
/// primitive keys, equal elements are indistinguishable, so any order is fine.
pub fn list_sort<T: Ord>(items: &mut [T]) {
    list_sort_by(items, |a, b| a.cmp(b));
}

/// `List<T>.Sort(Comparison<T>)` with the comparison written as a C#-style
/// function returning a signed int.
pub fn list_sort_by_int<T>(items: &mut [T], mut cmp: impl FnMut(&T, &T) -> i32) {
    list_sort_by(items, |a, b| cmp(a, b).cmp(&0));
}

fn intro_sort<T, F: FnMut(&T, &T) -> Ordering>(keys: &mut [T], mut depth_limit: i32, cmp: &mut F) {
    let mut partition_size = keys.len();
    while partition_size > 1 {
        if partition_size <= INTROSORT_SIZE_THRESHOLD {
            if partition_size == 2 {
                swap_if_greater(keys, cmp, 0, 1);
                return;
            }
            if partition_size == 3 {
                swap_if_greater(keys, cmp, 0, 1);
                swap_if_greater(keys, cmp, 0, 2);
                swap_if_greater(keys, cmp, 1, 2);
                return;
            }
            insertion_sort(&mut keys[..partition_size], cmp);
            return;
        }
        if depth_limit == 0 {
            heap_sort(&mut keys[..partition_size], cmp);
            return;
        }
        depth_limit -= 1;
        let p = pick_pivot_and_partition(&mut keys[..partition_size], cmp);
        intro_sort(&mut keys[p + 1..partition_size], depth_limit, cmp);
        partition_size = p;
    }
}

fn swap_if_greater<T, F: FnMut(&T, &T) -> Ordering>(keys: &mut [T], cmp: &mut F, i: usize, j: usize) {
    if cmp(&keys[i], &keys[j]) == Ordering::Greater {
        keys.swap(i, j);
    }
}

fn pick_pivot_and_partition<T, F: FnMut(&T, &T) -> Ordering>(keys: &mut [T], cmp: &mut F) -> usize {
    let hi = keys.len() - 1;
    let middle = hi >> 1;
    swap_if_greater(keys, cmp, 0, middle);
    swap_if_greater(keys, cmp, 0, hi);
    swap_if_greater(keys, cmp, middle, hi);
    // T pivot = keys[middle]; Swap(keys, middle, hi - 1);
    // The pivot stays at hi - 1 for the whole loop: swaps only touch
    // left < right <= hi - 2.
    keys.swap(middle, hi - 1);
    let pivot = hi - 1;
    let mut left = 0usize;
    let mut right = hi - 1;
    while left < right {
        loop {
            left += 1;
            if left >= keys.len() {
                ran_off_the_end();
            }
            if cmp(&keys[left], &keys[pivot]) != Ordering::Less {
                break;
            }
        }
        loop {
            if right == 0 {
                ran_off_the_end();
            }
            right -= 1;
            if cmp(&keys[pivot], &keys[right]) != Ordering::Less {
                break;
            }
        }
        if left >= right {
            break;
        }
        keys.swap(left, right);
    }
    if left != hi - 1 {
        keys.swap(left, hi - 1);
    }
    left
}

fn heap_sort<T, F: FnMut(&T, &T) -> Ordering>(keys: &mut [T], cmp: &mut F) {
    HEAPSORT_CALLS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let n = keys.len();
    let mut i = n >> 1;
    while i >= 1 {
        down_heap(keys, i, n, cmp);
        i -= 1;
    }
    let mut i = n;
    while i > 1 {
        keys.swap(0, i - 1);
        down_heap(keys, 1, i - 1, cmp);
        i -= 1;
    }
}

fn down_heap<T, F: FnMut(&T, &T) -> Ordering>(keys: &mut [T], mut i: usize, n: usize, cmp: &mut F) {
    // `d` is the element currently at keys[i - 1]; it moves down by swaps.
    while i <= n >> 1 {
        let mut child = 2 * i;
        if child < n && cmp(&keys[child - 1], &keys[child]) == Ordering::Less {
            child += 1;
        }
        if cmp(&keys[i - 1], &keys[child - 1]) != Ordering::Less {
            break;
        }
        keys.swap(i - 1, child - 1);
        i = child;
    }
}

fn insertion_sort<T, F: FnMut(&T, &T) -> Ordering>(keys: &mut [T], cmp: &mut F) {
    for i in 0..keys.len() - 1 {
        // t = keys[i + 1], kept at position j + 1 while shifting.
        let mut j = i as isize;
        while j >= 0 && cmp(&keys[j as usize + 1], &keys[j as usize]) == Ordering::Less {
            keys.swap(j as usize, j as usize + 1);
            j -= 1;
        }
    }
}

/// LINQ `OrderBy`/`ThenBy`: stable sort. Rust's `sort_by` is stable, so this is
/// a thin wrapper kept for greppability. See also `linq::order_by`.
pub fn order_by<T>(items: &mut Vec<T>, cmp: impl FnMut(&T, &T) -> Ordering) {
    items.sort_by(cmp)
}
