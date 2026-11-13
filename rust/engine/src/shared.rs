//! Values shared between owners, compared by identity.

use std::cell::{Ref, RefCell, RefMut};
use std::rc::Rc;

/// A mutable object with more than one owner (the settings an event shares with the game).
/// Cloning shares the object. Equality is reference equality.
pub struct Shared<T>(Rc<RefCell<T>>);

impl<T> Shared<T> {
    pub fn new(value: T) -> Self {
        Shared(Rc::new(RefCell::new(value)))
    }

    #[track_caller]
    pub fn borrow(&self) -> Ref<'_, T> {
        self.0.borrow()
    }

    #[track_caller]
    pub fn borrow_mut(&self) -> RefMut<'_, T> {
        self.0.borrow_mut()
    }

    /// Whether both are the same object.
    pub fn ptr_eq(&self, other: &Shared<T>) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl<T> Clone for Shared<T> {
    fn clone(&self) -> Self {
        Shared(Rc::clone(&self.0))
    }
}

/// Reference equality.
impl<T> PartialEq for Shared<T> {
    fn eq(&self, other: &Self) -> bool {
        self.ptr_eq(other)
    }
}

impl<T> Eq for Shared<T> {}

impl<T: std::fmt::Debug> std::fmt::Debug for Shared<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.borrow().fmt(f)
    }
}

impl<T: Default> Default for Shared<T> {
    fn default() -> Self {
        Shared::new(T::default())
    }
}

/// An `Rc` used as a map key or set element by identity: equality and hashing by
/// address, not by contents.
pub struct RcKey<T>(pub Rc<T>);

impl<T> Clone for RcKey<T> {
    fn clone(&self) -> Self {
        RcKey(Rc::clone(&self.0))
    }
}

impl<T> PartialEq for RcKey<T> {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl<T> Eq for RcKey<T> {}

impl<T> std::hash::Hash for RcKey<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        (Rc::as_ptr(&self.0) as *const () as usize).hash(state)
    }
}

impl<T> std::fmt::Debug for RcKey<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RcKey({:p})", Rc::as_ptr(&self.0))
    }
}
