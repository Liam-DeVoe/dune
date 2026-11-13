//! Per-replay memory budgets.
//!
//! The differ's global allocator counts the bytes each thread holds. A replay
//! thread sets a budget with [`set_budget`]. When an allocation would take it
//! over the budget, the allocator raises the thread's flag and then puts the
//! thread to sleep for good, inside the allocation. It can't unwind there, and
//! sleeping stops the growth: the thread keeps what it already has (at most the
//! budget) until the process exits, like a hung replay. The thread that waits
//! for the replay sees the flag and reports a memory limit failure.
//!
//! Memory freed by another thread is subtracted from that thread's count, not
//! the allocating thread's, so a count is an estimate. A replay keeps its game
//! on its own thread, so the estimate is close for the case that matters, a
//! game that keeps growing.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub struct CountingAllocator;

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

thread_local! {
    /// Bytes allocated minus bytes freed on this thread.
    static USED: Cell<isize> = const { Cell::new(0) };
    /// Budget in bytes; 0 means none.
    static BUDGET: Cell<usize> = const { Cell::new(0) };
    /// Set when the budget was exceeded. Points into an `Arc` the thread keeps alive.
    static FLAG: Cell<*const AtomicBool> = const { Cell::new(std::ptr::null()) };
}

/// Counts `size` new bytes on this thread. Over the budget, raises the flag and never returns.
#[inline]
fn charge(size: usize) {
    let _ = USED.try_with(|used| {
        let now = used.get() + size as isize;
        used.set(now);
        let budget = BUDGET.try_with(|b| b.get()).unwrap_or(0);
        if budget != 0 && now > budget as isize {
            let flag = FLAG.try_with(|f| f.get()).unwrap_or(std::ptr::null());
            if !flag.is_null() {
                // SAFETY: the flag's Arc lives in this thread's `Budget` guard, and this
                // thread never returns from here, so the guard is never dropped.
                unsafe { (*flag).store(true, Ordering::SeqCst) };
                loop {
                    std::thread::sleep(std::time::Duration::from_secs(3600));
                }
            }
        }
    });
}

#[inline]
fn credit(size: usize) {
    let _ = USED.try_with(|used| used.set(used.get() - size as isize));
}

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        charge(layout.size());
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        charge(layout.size());
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        credit(layout.size());
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        if new_size > layout.size() {
            charge(new_size - layout.size());
        } else {
            credit(layout.size() - new_size);
        }
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

/// Keeps a thread's budget in force; the flag tells another thread that the budget was exceeded.
pub struct Budget {
    flag: Arc<AtomicBool>,
}

impl Drop for Budget {
    fn drop(&mut self) {
        let _ = BUDGET.try_with(|b| b.set(0));
        let _ = FLAG.try_with(|f| f.set(std::ptr::null()));
    }
}

/// Gives the current thread a budget of `bytes` (0 = none), counted from now. Returns the flag
/// that is raised when the thread exceeds it. Keep the returned guard alive on this thread.
pub fn set_budget(bytes: usize, flag: Arc<AtomicBool>) -> Budget {
    USED.with(|u| u.set(0));
    let budget = Budget { flag };
    FLAG.with(|f| f.set(Arc::as_ptr(&budget.flag)));
    BUDGET.with(|b| b.set(bytes));
    budget
}
