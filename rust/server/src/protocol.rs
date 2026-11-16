//! The server's side of the message format (`treachery-protocol`, rust/docs/protocol.md):
//! decoding frames, encoding replies and notifications, and the engine's enums by name.
//!
//! Decoding runs on the game thread, with everything else a call does. The socket tasks only
//! move text.

use serde::Serialize;
use serde_json::Value;
use treachery_engine::enum_type::GameEnum;
pub use treachery_protocol::{decode, replies, Call, Decoded, ErrorCode, Notification, ServerMessage, MAX_MESSAGE_BYTES};

/// A successful reply.
pub fn ok(id: u64, value: Value) -> String {
    ServerMessage::Ok { id, ok: value }.encode()
}

/// A failed call's reply. An empty `message` gives the code's usual text.
pub fn err(id: u64, code: ErrorCode, message: &str) -> String {
    ServerMessage::Err { id, err: treachery_protocol::CallError::with_message(code, message) }.encode()
}

pub fn notify(notification: Notification) -> String {
    ServerMessage::Notify(notification).encode()
}

pub fn to_value<T: Serialize>(value: &T) -> Value {
    serde_json::to_value(value).expect("replies serialize")
}

/// An engine enum's member name, as the protocol sends it.
pub fn enum_name<T: GameEnum>(value: T) -> String {
    T::INFO.name_of(value.value()).map_or_else(|| value.value().to_string(), str::to_string)
}

/// The member with this name.
pub fn enum_value<T: GameEnum>(name: &str) -> Option<T> {
    T::INFO.declared.iter().find(|(n, _)| *n == name).map(|(_, v)| T::from_value(*v))
}

#[cfg(test)]
mod tests {
    use super::*;
    use treachery_engine::enumerations::Faction;

    #[test]
    fn enums_by_name() {
        assert_eq!(enum_name(Faction::Atreides), "Atreides");
        assert_eq!(enum_value::<Faction>("Atreides"), Some(Faction::Atreides));
        assert_eq!(enum_value::<Faction>("Arrakis"), None);
    }
}
