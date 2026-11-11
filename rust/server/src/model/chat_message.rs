//! Ports of `Model/ChatMessage.cs`, `GameChatMessage.cs` and `Model/GlobalChatMessage.cs`.
//!
//! The server only relays these. `GetBodyIncludingPlayerInfo` is client rendering and is not
//! ported here.

use treachery_engine::json::{FromJson, JsonError, JsonValue, JsonWriter, ToJson};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GameChatMessage {
    pub target_user_id: i32,
    pub source_user_id: i32,
    /// C# `string Body` without an initializer: null unless set.
    pub body: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GlobalChatMessage {
    pub source_user_id: i32,
    pub body: Option<String>,
}

impl ToJson for GameChatMessage {
    fn write_json(&self, w: &mut JsonWriter) {
        w.object_value();
        w.member("TargetUserId", &self.target_user_id);
        w.member("SourceUserId", &self.source_user_id);
        w.member("Body", &self.body);
        w.end_object();
    }
}

impl ToJson for GlobalChatMessage {
    fn write_json(&self, w: &mut JsonWriter) {
        w.object_value();
        w.member("SourceUserId", &self.source_user_id);
        w.member("Body", &self.body);
        w.end_object();
    }
}

fn members(v: &JsonValue) -> Result<&serde_json::Map<String, JsonValue>, JsonError> {
    v.as_object().ok_or_else(|| JsonError::new("JsonException", "expected an object"))
}

impl FromJson for GameChatMessage {
    fn from_json(v: &JsonValue) -> Result<Self, JsonError> {
        let mut m = GameChatMessage::default();
        for (k, v) in members(v)? {
            match k.as_str() {
                "TargetUserId" => m.target_user_id = FromJson::from_json(v)?,
                "SourceUserId" => m.source_user_id = FromJson::from_json(v)?,
                "Body" => m.body = FromJson::from_json(v)?,
                _ => {}
            }
        }
        Ok(m)
    }
}

impl FromJson for GlobalChatMessage {
    fn from_json(v: &JsonValue) -> Result<Self, JsonError> {
        let mut m = GlobalChatMessage::default();
        for (k, v) in members(v)? {
            match k.as_str() {
                "SourceUserId" => m.source_user_id = FromJson::from_json(v)?,
                "Body" => m.body = FromJson::from_json(v)?,
                _ => {}
            }
        }
        Ok(m)
    }
}
