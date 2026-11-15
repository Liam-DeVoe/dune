use crate::json::JsonError;
use crate::json::{self, FromJson, JsonValue, ToJson};

pub struct Utilities;

impl Utilities {
    /// JSON `null` gives `Ok(None)`; malformed input gives the reader's error.
    pub fn deserialize<T: FromJson>(text: &str) -> Result<Option<T>, JsonError> {
        let v: JsonValue = serde_json::from_str(text).map_err(|e| JsonError::new("JsonException", e.to_string()))?;
        if v.is_null() {
            return Ok(None);
        }
        T::from_json(&v).map(Some).map_err(|e| json::locate(text, e))
    }

    pub fn serialize<T: ToJson + ?Sized>(value: &T) -> String {
        json::serialize(value)
    }
}
