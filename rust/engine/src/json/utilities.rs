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
    pub fn clone_object_dictionary<TX: Clone + Eq + std::hash::Hash + 'static, TY: Clone>(to_clone: &NetDictionary<TX, TY>) -> NetDictionary<TX, TY> {
        let mut result = NetDictionary::new();
        for (k, v) in to_clone.iter() {
            result.insert(k.clone(), v.clone());
        }
        result
    }

    pub fn clone_enum_dictionary<TX: Clone + Eq + std::hash::Hash + 'static, TY: Copy>(to_clone: &NetDictionary<TX, TY>) -> NetDictionary<TX, TY> {
        let mut result = NetDictionary::new();
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

    /// JSON `null` gives `None`; malformed input throws the reader's exception
    /// (`JsonException` and friends; most messages are not .NET's, see json.rs).
    pub fn deserialize<T: FromJson>(serialized_value: &str) -> Option<T> {
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

        let converted_value = Utilities::convert_newtonsoft_json_to_plain_json(serialized_value);
        let res = Utilities::json_deserialize(&converted_value);
        res
    }

    /// `JsonSerializer.Deserialize<T>(text, Options)`: `null` for a JSON null, throws on bad input.
    fn json_deserialize<T: FromJson>(text: &str) -> Option<T> {
        let v: JsonValue = match serde_json::from_str(text) {
            Ok(v) => v,
            Err(e) => cs::throw("JsonException", e.to_string()),
        };
        if v.is_null() {
            return None;
        }
        match T::from_json(&v) {
            Ok(t) => Some(t),
            Err(e) => {
                let e = json::locate(text, e);
                cs::throw(e.kind, e.message)
            }
        }
    }

    /// Known difference: `JsonNode.Parse` throws ArgumentException on a duplicate key
    /// when the object is first accessed; serde_json keeps the last value.
    pub fn convert_newtonsoft_json_to_plain_json(value: &str) -> String {
        let json_node: JsonValue = match serde_json::from_str(value) {
            Ok(v) => v,
            Err(e) => cs::throw("JsonException", e.to_string()),
        };
        if json_node.is_null() {
            return String::new();
        }

        let mut json_node = json_node;
        Utilities::replace_type_metadata_and_nested_arrays(&mut json_node);
        serde_json::to_string(&json_node).unwrap_or_default()
    }

    /// Takes the root node (which has no parent).
    pub fn replace_type_metadata_and_nested_arrays(node: &mut JsonValue) {
        // A root wrapper's ReplaceWith has no parent and does nothing.
        let _ = Utilities::replace_in(node, ParentKind::None);
    }

    /// One call of `ReplaceTypeMetadataAndNestedArrays`. Returns the array the node
    /// replaced itself with (`obj.ReplaceWith(newArray)`); the caller, as the parent,
    /// performs the replacement. With no parent `ReplaceWith` does nothing.
    fn replace_in(node: &mut JsonValue, parent: ParentKind) -> Option<JsonValue> {
        match node {
            JsonValue::Object(obj) => {
                if obj.contains_key("$type") {
                    // obj["$type"]!.GetValue<string>()
                    let type_name = match obj.get("$type") {
                        Some(JsonValue::String(s)) => s.clone(),
                        Some(JsonValue::Null) | None => cs::throw("NullReferenceException", cs::NULL_REFERENCE_MESSAGE.to_string()),
                        Some(JsonValue::Number(_)) => cs::throw(
                            "InvalidOperationException",
                            "An element of type 'Number' cannot be converted to a 'System.String'.".to_string(),
                        ),
                        Some(JsonValue::Bool(b)) => cs::throw(
                            "InvalidOperationException",
                            format!("An element of type '{}' cannot be converted to a 'System.String'.", if *b { "True" } else { "False" }),
                        ),
                        Some(_) => cs::throw("InvalidOperationException", "The node must be of type 'JsonValue'.".to_string()),
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
                        let _ = Utilities::replace_in(&mut clone, ParentKind::None);
                        new_array.push(clone);
                    }
                    if parent == ParentKind::None {
                        return None;
                    }
                    return Some(JsonValue::Array(new_array));
                } else {
                    let keys: Vec<String> = obj.keys().cloned().collect();
                    for key in keys {
                        let child_node = obj.get_mut(&key).nre();
                        if !child_node.is_null() {
                            // Replacing the value of an existing key does not invalidate the key enumeration.
                            if let Some(replacement) = Utilities::replace_in(child_node, ParentKind::Object) {
                                *obj.get_mut(&key).nre() = replacement;
                            }
                        }
                    }
                }
                None
            }
            JsonValue::Array(array) => {
                for i in 0..array.len() {
                    if !array[i].is_null() {
                        if let Some(replacement) = Utilities::replace_in(&mut array[i], ParentKind::Array) {
                            // JsonArray.SetItem changes the list's version: the foreach throws at its next MoveNext.
                            array[i] = replacement;
                            cs::throw(
                                "InvalidOperationException",
                                "Collection was modified; enumeration operation may not execute.".to_string(),
                            );
                        }
                    }
                }
                None
            }
            _ => None,
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
