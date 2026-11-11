//! Port of Model/IFetcher.cs.

/// `IFetcher<T>`: turns JSON ids into objects and back. `-1` stands for null.
pub trait IFetcher<T> {
    /// `Find(id)`; `None` is C# null. May throw like the C# implementation.
    fn find(&self, id: i32) -> Option<T>;
    /// `GetId(obj)`; `None` gives -1.
    fn get_id(&self, obj: Option<T>) -> i32;
}
