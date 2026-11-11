//! Port of Model/IIdentifiable.cs.

/// `IIdentifiable`: objects that JSON refers to by an integer id.
pub trait IIdentifiable {
    fn id(&self) -> i32;
}
