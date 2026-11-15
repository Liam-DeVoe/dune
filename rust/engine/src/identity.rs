/// `Identifiable`: objects that JSON refers to by an integer id.
pub trait Identifiable {
    fn id(&self) -> i32;
}

/// `Fetcher<T>`: turns JSON ids into objects and back. `-1` stands for null.
pub trait Fetcher<T> {
    /// `Find(id)`; `None` means null. May throw.
    fn find(&self, id: i32) -> Option<T>;
    /// `GetId(obj)`; `None` gives -1.
    fn get_id(&self, obj: Option<T>) -> i32;
}
