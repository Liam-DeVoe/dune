//! Port of Model/IIdentifiable.cs.

/// `IIdentifiable`: objects that JSON refers to by an integer id.
pub trait IIdentifiable {
    fn id(&self) -> i32;
}

/// `IFetcher<T>`: turns JSON ids into objects and back. `-1` stands for null.
pub trait IFetcher<T> {
    /// `Find(id)`; `None` is C# null. May throw like the C# implementation.
    fn find(&self, id: i32) -> Option<T>;
    /// `GetId(obj)`; `None` gives -1.
    fn get_id(&self, obj: Option<T>) -> i32;
}
