//! Port of `Model/Result.cs`: `Result<T>`, `VoidResult`, `VoidContents`. `ErrorType` is the
//! engine's `enumerations::ErrorType` (generated from the same file).

use treachery_engine::enumerations::ErrorType;
use treachery_engine::json::{JsonWriter, ToJson};

/// C# `Result<T>`. Named `CallResult` so it does not shadow Rust's `Result`.
#[derive(Clone, Debug, PartialEq)]
pub struct CallResult<T> {
    pub success: bool,
    /// Initializer: `ErrorType.None`.
    pub error: ErrorType,
    /// Initializer: `string.Empty`.
    pub error_details: String,
    pub contents: Option<T>,
}

/// C# `VoidContents`: an empty class, serialized as `{}`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VoidContents;

/// C# `VoidResult : Result<VoidContents>`.
pub type VoidResult = CallResult<VoidContents>;

impl ToJson for VoidContents {
    fn write_json(&self, w: &mut JsonWriter) {
        w.object_value();
        w.end_object();
    }
}

impl<T: ToJson> ToJson for CallResult<T> {
    fn write_json(&self, w: &mut JsonWriter) {
        w.object_value();
        w.member("Success", &self.success);
        w.member("Error", &self.error);
        w.member("ErrorDetails", &self.error_details);
        w.member("Contents", &self.contents);
        w.end_object();
    }
}
