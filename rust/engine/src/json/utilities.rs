use crate::json::JsonError;
use crate::prelude::*;
use crate::json::{self, FromJson, JsonValue, ToJson};

pub struct Utilities;

/// The parent of a node in `ReplaceTypeMetadataAndNestedArrays` (`JsonNode.Parent`).
#[derive(Clone, Copy, PartialEq, Eq)]
enum ParentKind {
    None,
    Object,
    Array,
}

impl Utilities {
    /// `ICloneable.Clone` is Rust `Clone` here.
    pub fn clone_list<T: Clone>(to_clone: &[T]) -> Vec<T> {
        to_clone.iter().cloned().collect()
    }

    /// `ToDictionary`: a compact copy in enumeration order (keys are unique).
    pub fn clone_object_dictionary<TX: Clone + Eq + std::hash::Hash + 'static, TY: Clone>(to_clone: &IndexMap<TX, TY>) -> IndexMap<TX, TY> {
        let mut result = IndexMap::new();
        for (k, v) in to_clone.iter() {
            result.insert(k.clone(), v.clone());
        }
        result
    }

    pub fn clone_enum_dictionary<TX: Clone + Eq + std::hash::Hash + 'static, TY: Copy>(to_clone: &IndexMap<TX, TY>) -> IndexMap<TX, TY> {
        let mut result = IndexMap::new();
        for (k, v) in to_clone.iter() {
            result.insert(k.clone(), *v);
        }
        result
    }

    /// The options are built into `json.rs`.
    pub fn options() -> () {}

    pub fn newtonsoft_indicator() -> String {
        "Treachery.Shared.GameState".to_string()
    }

    /// JSON `null` gives `Ok(None)`; malformed input gives the reader's error.
    pub fn deserialize<T: FromJson>(serialized_value: &str) -> Result<Option<T>, JsonError> {
        // serializedValue.Length < 128 || !serializedValue[..128].Contains(NewtonsoftIndicator)
        // (lengths in UTF-16 code units)
        let utf16_len: usize = serialized_value.chars().map(|c| c.len_utf16()).sum();
        let is_legacy = utf16_len >= 128 && {
            let mut units = 0;
            let prefix: String = serialized_value
                .chars()
                .take_while(|c| {
                    units += c.len_utf16();
                    units <= 128
                })
                .collect();
            prefix.contains(&Utilities::newtonsoft_indicator())
        };

        if !is_legacy {
            return Utilities::json_deserialize(serialized_value);
        }

        let converted_value = Utilities::convert_newtonsoft_json_to_plain_json(serialized_value)?;
        let res = Utilities::json_deserialize(&converted_value);
        res
    }

    /// `None` for a JSON null.
    fn json_deserialize<T: FromJson>(text: &str) -> Result<Option<T>, JsonError> {
        let v: JsonValue = serde_json::from_str(text).map_err(|e| JsonError::new("JsonException", e.to_string()))?;
        if v.is_null() {
            return Ok(None);
        }
        T::from_json(&v).map(Some).map_err(|e| json::locate(text, e))
    }

    /// Known difference: `JsonNode.Parse` throws ArgumentException on a duplicate key
    /// when the object is first accessed; serde_json keeps the last value.
    pub fn convert_newtonsoft_json_to_plain_json(value: &str) -> Result<String, JsonError> {
        let json_node: JsonValue = serde_json::from_str(value).map_err(|e| JsonError::new("JsonException", e.to_string()))?;
        if json_node.is_null() {
            return Ok(String::new());
        }

        let mut json_node = json_node;
        Utilities::replace_type_metadata_and_nested_arrays(&mut json_node)?;
        Ok(serde_json::to_string(&json_node).unwrap_or_default())
    }

    /// Takes the root node (which has no parent).
    pub fn replace_type_metadata_and_nested_arrays(node: &mut JsonValue) -> Result<(), JsonError> {
        // A root wrapper's ReplaceWith has no parent and does nothing.
        Utilities::replace_in(node, ParentKind::None).map(|_| ())
    }

    /// One call of `ReplaceTypeMetadataAndNestedArrays`. Returns the array the node
    /// replaced itself with (`obj.ReplaceWith(newArray)`); the caller, as the parent,
    /// performs the replacement. With no parent `ReplaceWith` does nothing.
    fn replace_in(node: &mut JsonValue, parent: ParentKind) -> Result<Option<JsonValue>, JsonError> {
        match node {
            JsonValue::Object(obj) => {
                if obj.contains_key("$type") {
                    // obj["$type"]!.GetValue<string>()
                    let type_name = match obj.get("$type") {
                        Some(JsonValue::String(s)) => s.clone(),
                        Some(JsonValue::Null) | None => return Err(JsonError::new("JsonException", "$type is null")),
                        Some(_) => return Err(JsonError::new("JsonException", "$type is not a string")),
                    };
                    let event_type = Utilities::determine_game_event(&type_name);
                    match event_type {
                        // Replaced in place: the key keeps its position.
                        Some(t) => {
                            obj.insert("$type".to_string(), JsonValue::String(t.name().to_string()));
                        }
                        None => {
                            obj.shift_remove("$type");
                        }
                    }
                }

                if obj.len() == 1 && matches!(obj.get("$values"), Some(JsonValue::Array(_))) {
                    let Some(JsonValue::Array(values_array)) = obj.get("$values") else { unreachable!() };
                    let mut new_array: Vec<JsonValue> = Vec::new();
                    // OfType<JsonNode>() skips null items
                    for item in values_array.iter().filter(|i| !i.is_null()) {
                        let mut clone = item.clone();
                        // The clone has no parent yet, so a wrapper inside it is not replaced.
                        Utilities::replace_in(&mut clone, ParentKind::None)?;
                        new_array.push(clone);
                    }
                    if parent == ParentKind::None {
                        return Ok(None);
                    }
                    return Ok(Some(JsonValue::Array(new_array)));
                } else {
                    let keys: Vec<String> = obj.keys().cloned().collect();
                    for key in keys {
                        let child_node = obj.get_mut(&key).expect("a key of this object");
                        if !child_node.is_null() {
                            // Replacing the value of an existing key does not invalidate the key enumeration.
                            if let Some(replacement) = Utilities::replace_in(child_node, ParentKind::Object)? {
                                *child_node = replacement;
                            }
                        }
                    }
                }
                Ok(None)
            }
            JsonValue::Array(array) => {
                for i in 0..array.len() {
                    if !array[i].is_null() {
                        if let Some(replacement) = Utilities::replace_in(&mut array[i], ParentKind::Array)? {
                            // The old Newtonsoft conversion failed on an array holding a wrapped array.
                            array[i] = replacement;
                            return Err(JsonError::new("JsonException", "An array holds a wrapped array"));
                        }
                    }
                }
                Ok(None)
            }
            _ => Ok(None),
        }
    }

    /// `Type.GetType(name)`, as probed on .NET 10: leading whitespace before the type
    /// name is skipped, trailing whitespace is not (`"X "` and `"X , A"` are not found);
    /// the type name is case-sensitive; without an assembly name the calling assembly
    /// (Shared) is searched; an assembly name matches on its simple name,
    /// case-insensitively and ignoring surrounding whitespace and the other parts.
    pub fn determine_game_event(full_type_name: &str) -> Option<GameEventType> {
        let (type_part, assembly_part) = match full_type_name.find(',') {
            Some(i) => (&full_type_name[..i], Some(&full_type_name[i + 1..])),
            None => (full_type_name, None),
        };
        if let Some(assembly) = assembly_part {
            let simple_name = assembly.split(',').next().unwrap_or("").trim();
            if !simple_name.eq_ignore_ascii_case("Treachery.Shared") {
                return None;
            }
        }
        let class_name = type_part.trim_start().strip_prefix("Treachery.Shared.")?;
        // Only concrete GameEvent subclasses are in the discriminator list (the abstract
        // PassableGameEvent and PlacementEvent are not).
        GameEventType::from_json_discriminator(class_name)
    }

    pub fn serialize<T: ToJson + ?Sized>(value: &T) -> String {
        json::serialize(value)
    }

    /// UI code (font scaling), ported for completeness.
    pub fn scale_font(font: &str, scale: f32) -> String {
        let fontsize_lower = font.to_lowercase();
        let mut result = String::new();

        for font_definition_part in fontsize_lower.split(' ') {
            if font_definition_part.contains("px") {
                let number_part = &font_definition_part[..font_definition_part.find("px").nre()];
                if let Ok(fontsize_number) = dotnet_compat::parse::try_float_parse_invariant(Some(number_part)) {
                    result += " ";
                    result += &Utilities::px((scale * fontsize_number) as f64);
                } else {
                    result += " ";
                    result += font_definition_part;
                }
            } else {
                result += " ";
                result += font_definition_part;
            }
        }

        result
    }

    pub fn px(x: f64) -> String {
        if x > -0.001 && x < 0.001 {
            "0".to_string()
        } else {
            dotnet_compat::fmt::f64_to_string(x) + "px"
        }
    }

    pub fn round(x: f64) -> String {
        dotnet_compat::fmt::f64_to_string(x)
    }
}
