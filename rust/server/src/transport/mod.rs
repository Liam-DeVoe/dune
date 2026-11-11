//! The wire format of rust/docs/protocol.md, kept apart from the hub so it can change freely.
//!
//! Everything here runs on the hub thread: decoding builds engine values (events), which are
//! not `Send`, and encoding reads them. The socket tasks in [`ws`] only move text.
//!
//! - [`decode`]: one client message to a [`ClientMessage`].
//! - [`encode_reply`] / [`encode_call_error`]: the reply to one call.
//! - [`encode_callback`]: a hub callback as a notification. Callbacks that carried game state
//!   (`HandleGameEvent`, `HandleUndo`, ...) are not sent as such; the game-view layer
//!   (`crate::views`) answers them with a `GameView` for each member of the game's group.
//!
//! Replies and notifications write `Dictionary<int, V>` as `[key, value]` pairs in the
//! dictionary's order ([`crate::json::wire`]); the hub's types keep the C# encoding everywhere
//! else (database, equivalence transcripts).

pub mod decode;
pub mod static_files;
pub mod ws;

use serde_json::Value;
use treachery_engine::json::{JsonWriter, ToJson};

use crate::clients::Callback;
use crate::hub::call::{HubCall, Reply};
use crate::model::game_init_info::GameInitInfo;
use crate::model::result::CallResult;

/// Maximum incoming message, the C# `MaximumReceiveMessageSize`.
pub const MAX_MESSAGE_BYTES: usize = 4_194_304;

/// The calls of protocol.md's "Game view" section. They are not C# hub methods: the game-view
/// layer answers them from the game's view replica (`crate::views`).
#[derive(Clone, Debug, PartialEq)]
pub enum ViewCall {
    /// `QueryDraft(userToken, gameId, eventNumber, draft)`. `draft` is the event's JSON text.
    QueryDraft { user_token: String, game_id: String, event_number: i32, draft: String },
    /// `RequestReport(userToken, gameId, index)`.
    RequestReport { user_token: String, game_id: String, index: i32 },
    /// `RequestUndoList(userToken, gameId)`.
    RequestUndoList { user_token: String, game_id: String },
    /// `RequestUndoPreview(userToken, gameId, gap)`.
    RequestUndoPreview { user_token: String, game_id: String, gap: i32 },
    /// `RequestSaveGame(userToken, gameId)`.
    RequestSaveGame { user_token: String, game_id: String },
}

/// A decoded client message.
pub enum ClientMessage {
    Call { id: u64, call: HubCall },
    View { id: u64, call: ViewCall },
    /// A call whose method or arguments did not decode. Answered with an `error` reply.
    BadCall { id: u64, error: String },
    Ping,
}

const VIEW_METHODS: &[&str] = &["QueryDraft", "RequestReport", "RequestUndoList", "RequestUndoPreview", "RequestSaveGame"];

/// The method's canonical name: names are matched ASCII case-insensitively (protocol.md).
fn canonical_method(name: &str) -> Option<&'static str> {
    decode::METHODS.iter().chain(VIEW_METHODS).copied().find(|m| m.eq_ignore_ascii_case(name))
}

/// Parses one text frame. `Err` is a malformed envelope, which closes the connection.
pub fn decode(text: &str) -> Result<ClientMessage, String> {
    let v: Value = serde_json::from_str(text).map_err(|e| format!("not JSON: {e}"))?;
    let Value::Object(o) = &v else { return Err("a message must be a JSON object".into()) };
    match o.get("type").and_then(Value::as_str) {
        Some("ping") => Ok(ClientMessage::Ping),
        Some("call") => {
            let id = o.get("id").and_then(Value::as_u64).ok_or("a call needs a non-negative integer id")?;
            let Some(method) = o.get("method").and_then(Value::as_str) else {
                return Ok(ClientMessage::BadCall { id, error: "a call needs a method".into() });
            };
            let args: &[Value] = match o.get("args") {
                Some(Value::Array(a)) => a,
                None => &[],
                Some(_) => return Ok(ClientMessage::BadCall { id, error: "args must be an array".into() }),
            };
            let Some(method) = canonical_method(method) else {
                return Ok(ClientMessage::BadCall { id, error: format!("Unknown method '{method}'") });
            };
            let decoded = if VIEW_METHODS.contains(&method) {
                decode_view_call(method, args).map(|call| ClientMessage::View { id, call })
            } else {
                decode::decode(method, args).map(|call| ClientMessage::Call { id, call })
            };
            Ok(decoded.unwrap_or_else(|error| ClientMessage::BadCall { id, error }))
        }
        _ => Err("unknown message type".into()),
    }
}

fn decode_view_call(method: &str, args: &[Value]) -> Result<ViewCall, String> {
    let s = |i: usize| -> Result<String, String> {
        match args.get(i) {
            Some(Value::String(s)) => Ok(s.clone()),
            _ => Err(format!("{method}: argument {i} must be a string")),
        }
    };
    let int = |i: usize| -> Result<i32, String> {
        args.get(i).and_then(Value::as_i64).and_then(|n| i32::try_from(n).ok()).ok_or_else(|| format!("{method}: argument {i} must be an int"))
    };
    let expected = match method {
        "RequestUndoList" | "RequestSaveGame" => 2,
        "RequestReport" | "RequestUndoPreview" => 3,
        _ => 4,
    };
    if args.len() != expected {
        return Err(format!("{method} takes {expected} arguments, got {}", args.len()));
    }
    Ok(match method {
        "QueryDraft" => {
            let draft = match &args[3] {
                d @ Value::Object(_) => d.to_string(),
                _ => return Err(format!("{method}: the draft must be an event object")),
            };
            ViewCall::QueryDraft { user_token: s(0)?, game_id: s(1)?, event_number: int(2)?, draft }
        }
        "RequestReport" => ViewCall::RequestReport { user_token: s(0)?, game_id: s(1)?, index: int(2)? },
        "RequestUndoPreview" => ViewCall::RequestUndoPreview { user_token: s(0)?, game_id: s(1)?, gap: int(2)? },
        "RequestSaveGame" => ViewCall::RequestSaveGame { user_token: s(0)?, game_id: s(1)? },
        _ => ViewCall::RequestUndoList { user_token: s(0)?, game_id: s(1)? },
    })
}

/// `GameInitInfo` as clients get it: without `GameState` (protocol.md, Payload objects).
struct ClientGameInitInfo<'a>(&'a GameInitInfo);

impl ToJson for ClientGameInitInfo<'_> {
    fn write_json(&self, w: &mut JsonWriter) {
        let info = self.0;
        w.object_value();
        w.member("GameId", &info.game_id);
        w.member("GameName", &info.game_name);
        w.key("Participation");
        match &info.participation {
            Some(p) => crate::json::write_participation(w, &p.borrow()),
            None => w.null(),
        }
        w.member("PhaseControl", &info.phase_control);
        w.end_object();
    }
}

fn result_json<T: ToJson>(r: &CallResult<T>) -> String {
    treachery_engine::json::serialize(r)
}

/// The `Result` JSON of a reply, with integer-keyed dictionaries as `[key, value]` pairs
/// (protocol.md, Encoding).
pub fn reply_result_json(reply: &Reply) -> String {
    crate::json::wire(|| reply_result_json_inner(reply))
}

fn reply_result_json_inner(reply: &Reply) -> String {
    match reply {
        Reply::Void(r) => result_json(r),
        Reply::LoginInfo(r) => result_json(r),
        Reply::ServerStatus(r) => result_json(r),
        Reply::ServerInfo(r) => result_json(r),
        Reply::String(r) => result_json(r),
        Reply::AdminInfo(r) => result_json(r),
        Reply::GameInitInfo(r) => result_json(&CallResult {
            success: r.success,
            error: r.error,
            error_details: r.error_details.clone(),
            contents: r.contents.as_ref().map(ClientGameInitInfo),
        }),
    }
}

/// `{"type":"reply","id":..,"result":..}`, with `result` already JSON.
pub fn encode_reply_json(id: u64, result_json: &str) -> String {
    format!("{{\"type\":\"reply\",\"id\":{id},\"result\":{result_json}}}")
}

/// `{"type":"reply","id":..,"result":..}`. `GameInitInfo.GameState` is left out (protocol.md).
pub fn encode_reply(id: u64, reply: &Reply) -> String {
    encode_reply_json(id, &reply_result_json(reply))
}

/// `{"type":"reply","id":..,"error":".."}`.
pub fn encode_call_error(id: u64, error: &str) -> String {
    let mut w = JsonWriter::new();
    w.object_value();
    w.member("type", "reply");
    w.key("id");
    w.int(id as i64);
    w.member("error", error);
    w.end_object();
    w.finish()
}

/// `{"type":"notify","name":..,"args":[..]}`, with each argument already JSON.
pub fn encode_notify(name: &str, args: &[String]) -> String {
    format!("{{\"type\":\"notify\",\"name\":\"{name}\",\"args\":[{}]}}", args.join(","))
}

fn j<T: ToJson + ?Sized>(v: &T) -> String {
    treachery_engine::json::serialize(v)
}

/// A callback as a `notify` message, or `None` for the callbacks that carried game state, which
/// the game-view layer turns into `GameView` notifications (protocol.md, Notifications).
pub fn encode_callback(callback: &Callback) -> Option<String> {
    crate::json::wire(|| encode_callback_inner(callback))
}

fn encode_callback_inner(callback: &Callback) -> Option<String> {
    use Callback::*;
    Some(match callback {
        HandleGameEvent { .. } | HandleUndo(_) | HandleRestoreRecentlyUndone | HandleDismissRecentlyUndone => return None,
        HandleChatMessage(m) => encode_notify("ChatMessage", &[j(m)]),
        HandleGlobalChatMessage(m) => encode_notify("GlobalChatMessage", &[j(m)]),
        HandleSetTimer(v) => encode_notify("SetTimer", &[j(v)]),
        HandlePhaseControl(info) => encode_notify("PhaseControl", &[j(info)]),
        HandleSetSkin(skin) => encode_notify("SetSkin", &[j(skin)]),
        HandleUpdateSettings(settings) => encode_notify("UpdateSettings", &[j(settings)]),
        HandleJoinGame { user_id, user_name, seat } => encode_notify("JoinGame", &[j(user_id), j(user_name), j(seat)]),
        HandleSetOrUnsetHost(user_id) => encode_notify("SetOrUnsetHost", &[j(user_id)]),
        HandleObserveGame { user_id, user_name } => encode_notify("ObserveGame", &[j(user_id), j(user_name)]),
        HandleOpenOrCloseSeats(seats) => encode_notify("OpenOrCloseSeats", &[j(seats)]),
        HandleRemoveUser { user_id, kick } => encode_notify("RemoveUser", &[j(user_id), j(kick)]),
        HandleBotSpeed(speed) => encode_notify("BotSpeed", &[j(speed)]),
        HandleLoadGame(info) => encode_notify("LoadGame", &[j(&ClientGameInitInfo(info))]),
        HandleAssignSeats(assignment) => {
            let mut w = JsonWriter::new();
            crate::json::write_int_dict(&mut w, assignment);
            encode_notify("AssignSeats", &[w.finish()])
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelopes() {
        assert!(matches!(decode(r#"{"type":"ping"}"#), Ok(ClientMessage::Ping)));
        assert!(decode("[1]").is_err());
        assert!(decode(r#"{"type":"call","method":"Connect"}"#).is_err(), "a call without an id is malformed");
        match decode(r#"{"type":"call","id":3,"method":"connect","args":[]}"#) {
            Ok(ClientMessage::Call { id: 3, call: HubCall::Connect }) => {}
            _ => panic!("method names are case-insensitive"),
        }
        match decode(r#"{"type":"call","id":4,"method":"RequestFoo","args":[]}"#) {
            Ok(ClientMessage::BadCall { id: 4, error }) => assert_eq!(error, "Unknown method 'RequestFoo'"),
            _ => panic!(),
        }
        match decode(r#"{"type":"call","id":5,"method":"GetLoginInfo","args":[null]}"#) {
            Ok(ClientMessage::BadCall { id: 5, .. }) => {}
            _ => panic!("a null token is a failed call"),
        }
        match decode(r#"{"type":"call","id":6,"method":"QueryDraft","args":["t","g",12,{"$type":"StormDialled","Amount":3}]}"#) {
            Ok(ClientMessage::View { id: 6, call: ViewCall::QueryDraft { event_number: 12, draft, .. } }) => {
                assert_eq!(draft, r#"{"$type":"StormDialled","Amount":3}"#, "member order is kept")
            }
            _ => panic!(),
        }
        assert_eq!(encode_call_error(7, "x"), r#"{"type":"reply","id":7,"error":"x"}"#);
        // An assignment comes as [userId, seat] pairs, in the client's order; the C# object
        // form is still read.
        for (args, expected) in [(r#"[[5,1],[2,0]]"#, vec![(5, 1), (2, 0)]), (r#"{"2":0}"#, vec![(2, 0)])] {
            let text = format!(r#"{{"type":"call","id":8,"method":"RequestAssignSeats","args":["t","g",{args}]}}"#);
            match decode(&text) {
                Ok(ClientMessage::Call { call: HubCall::RequestAssignSeats { assignment, .. }, .. }) => {
                    assert_eq!(assignment.iter().map(|(k, v)| (*k, *v)).collect::<Vec<_>>(), expected)
                }
                _ => panic!("{args}"),
            }
        }
    }

    #[test]
    fn callbacks() {
        let mut d = dotnet_compat::NetDictionary::new();
        d.add(12, 3);
        assert_eq!(encode_callback(&Callback::HandleAssignSeats(d)).unwrap(), r#"{"type":"notify","name":"AssignSeats","args":[[[12,3]]]}"#);
        assert_eq!(
            encode_callback(&Callback::HandleJoinGame { user_id: 2, user_name: "Bob".into(), seat: -1 }).unwrap(),
            r#"{"type":"notify","name":"JoinGame","args":[2,"Bob",-1]}"#
        );
        assert!(encode_callback(&Callback::HandleUndo(3)).is_none());
        let info = GameInitInfo { game_id: Some("g".into()), game_state: Some("{}".into()), game_name: None, participation: None, phase_control: None };
        assert_eq!(
            encode_callback(&Callback::HandleLoadGame(info)).unwrap(),
            r#"{"type":"notify","name":"LoadGame","args":[{"GameId":"g","GameName":null,"Participation":null,"PhaseControl":null}]}"#
        );
    }
}
