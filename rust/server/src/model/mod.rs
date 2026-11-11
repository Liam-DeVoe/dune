//! Ports of the server types that the C# keeps in `Shared` (they were shared with the Blazor
//! client): `Model/ManagedGame.cs`, `Model/ScheduledGame.cs`, `Model/UserConnections.cs`, the
//! hub's result and payload classes, and the chat messages.
//!
//! Each payload class implements `ToJson` with the members in the order System.Text.Json
//! writes them (own members in declaration order, then the base class's). The equivalence
//! harness compares these encodings with the C# ones byte for byte.

/// Implements `ToJson` for a class whose members all implement `ToJson`.
macro_rules! to_json_members {
    ($ty:ty { $($name:literal => $field:ident),* $(,)? }) => {
        impl treachery_engine::json::ToJson for $ty {
            fn write_json(&self, w: &mut treachery_engine::json::JsonWriter) {
                w.object_value();
                $( w.member($name, &self.$field); )*
                w.end_object();
            }
        }
    };
}
pub(crate) use to_json_members;

pub mod admin_info;
pub mod available_seat_info;
pub mod chat_message;
pub mod game_info;
pub mod game_init_info;
pub mod logged_in_user_info;
pub mod login_info;
pub mod managed_game;
pub mod phase_control_info;
pub mod result;
pub mod scheduled_game;
pub mod scheduled_game_info;
pub mod server_info;
pub mod server_status;
pub mod user_connections;
pub mod user_info;
