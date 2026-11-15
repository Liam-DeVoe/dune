//! The message format of rust/docs/protocol.md: decoding a client's frame into a [`Request`],
//! and encoding replies and notifications.
//!
//! Decoding runs on the game thread, because a request can hold engine values (events), which
//! are not `Send`. The socket tasks only move text.

use serde_json::{json, Value};
use treachery_engine::enumerations::{ErrorType, GameListScope, SubscriptionType, UserStatus};
use treachery_engine::game::GameSettings;
use treachery_engine::json::FromJson;

use super::time::{self, Time};
use super::wire::{GameChatMessage, GlobalChatMessage};

/// The largest frame a client may send.
pub const MAX_MESSAGE_BYTES: usize = 4 * 1024 * 1024;

/// A call, by its method in protocol.md. The caller is the user the connection is logged in as
/// (`Server::caller`); no call names its user.
pub enum Request {
    // Server and accounts
    Connect,
    CreateUser { name: String, password_hash: String, email: Option<String>, player_name: String },
    Login { version: i32, name: String, password_hash: String },
    ResumeSession { token: String },
    LinkLogin { token: String },
    Logout,
    PasswordReset { name_or_email: String },
    SetPassword { name: String, reset_token: String, password_hash: String },
    UpdateUserInfo { password_hash: Option<String>, player_name: String, email: Option<String> },
    SetUserStatus { status: UserStatus },
    Heartbeat { scope: GameListScope },

    // Lobby and game management
    CreateGame { name: Option<String>, password_hash: Option<String>, savegame: Option<String>, skin: Option<String> },
    LoadGame { game: String, savegame: String, skin: Option<String> },
    UpdateSettings { game: String, settings: GameSettings },
    CloseGame { game: String },
    GameState { game: String },
    ScheduleGame { starts: Time, ruleset: Option<i32>, players: i32, max_turns: i32, factions: Vec<i32>, async_play: bool },
    CancelScheduledGame { scheduled_game: String },
    Subscribe { scheduled_game: Option<String>, subscription: SubscriptionType },

    // Seating and participation
    JoinGame { game: String, password_hash: Option<String>, seat: i32 },
    ObserveGame { game: String, password_hash: Option<String> },
    ReconnectGame { game: String },
    LeaveGame { game: String },
    SetOrUnsetHost { game: String, user: i32 },
    OpenOrCloseSeat { game: String, seat: i32 },
    Kick { game: String, user: i32 },
    AssignSeats { game: String, seats: Vec<(i32, i32)> },
    SetBotSpeed { game: String, speed: i32 },
    NudgeBots { game: String },

    // Playing
    /// The event as the client sent it, a JSON object: the server sets the members it
    /// computes itself before decoding it (`Server::submit_event`).
    SubmitEvent { game: String, event: serde_json::Map<String, Value> },
    Undo { game: String, until: i32 },
    RestoreUndone { game: String },
    DismissUndone { game: String },
    SetTimer { game: String, value: i32 },
    SetSkin { game: String, skin: String },
    PhaseDone { game: String },
    PhaseHold { game: String },
    PhaseRelease { game: String },
    PhaseContinue { game: String, event_number: i32 },

    // Chat
    Chat { game: String, message: GameChatMessage },
    GlobalChat { message: GlobalChatMessage },

    // Admin
    SetMaintenance { starts: Time },
    SaveAll,
    ReloadAll,
    AdminCloseGame { game: String },
    DownloadGame { game: String },
    AdminCancelScheduledGame { scheduled_game: String },
    DeleteUser { user: i32 },
    AdminInfo,

    // Game view
    QueryDraft { game: String, event_number: i32, draft: String },
    Report { game: String, index: i32 },
    UndoList { game: String },
    UndoPreview { game: String, gap: i32 },
    SaveGame { game: String },
}

/// A decoded frame.
pub enum Message {
    Call { id: u64, request: Request },
    /// A call whose method or arguments are wrong: answered with an `error` reply.
    BadCall { id: u64, error: String },
    Ping,
}

struct Args<'a> {
    method: &'a str,
    args: &'a [Value],
}

impl Args<'_> {
    fn get(&self, i: usize) -> Result<&Value, String> {
        self.args.get(i).ok_or_else(|| format!("{}: missing argument {i}", self.method))
    }

    fn string(&self, i: usize) -> Result<String, String> {
        match self.get(i)? {
            Value::String(s) => Ok(s.clone()),
            v => Err(format!("{}: argument {i} must be a string, not {v}", self.method)),
        }
    }

    fn optional_string(&self, i: usize) -> Result<Option<String>, String> {
        match self.get(i)? {
            Value::Null => Ok(None),
            _ => self.string(i).map(Some),
        }
    }

    fn int(&self, i: usize) -> Result<i32, String> {
        self.get(i)?.as_i64().and_then(|n| i32::try_from(n).ok()).ok_or_else(|| format!("{}: argument {i} must be an int", self.method))
    }

    fn optional_int(&self, i: usize) -> Result<Option<i32>, String> {
        match self.get(i)? {
            Value::Null => Ok(None),
            _ => self.int(i).map(Some),
        }
    }

    fn bool(&self, i: usize) -> Result<bool, String> {
        self.get(i)?.as_bool().ok_or_else(|| format!("{}: argument {i} must be true or false", self.method))
    }

    fn time(&self, i: usize) -> Result<Time, String> {
        time::parse(&self.string(i)?).ok_or_else(|| format!("{}: argument {i} must be a time", self.method))
    }

    fn engine<T: FromJson>(&self, i: usize) -> Result<T, String> {
        T::from_json(self.get(i)?).map_err(|e| format!("{}: argument {i}: {}", self.method, e.message))
    }

    fn json<T: serde::de::DeserializeOwned>(&self, i: usize) -> Result<T, String> {
        serde_json::from_value(self.get(i)?.clone()).map_err(|e| format!("{}: argument {i}: {e}", self.method))
    }

    /// `[[key, value], ...]`, or an object with integer keys.
    fn int_pairs(&self, i: usize) -> Result<Vec<(i32, i32)>, String> {
        let bad = || format!("{}: argument {i} must be [key, value] pairs", self.method);
        let int = |v: &Value| v.as_i64().and_then(|n| i32::try_from(n).ok()).ok_or_else(bad);
        match self.get(i)? {
            Value::Array(pairs) => pairs
                .iter()
                .map(|p| match p.as_array().map(Vec::as_slice) {
                    Some([k, v]) => Ok((int(k)?, int(v)?)),
                    _ => Err(bad()),
                })
                .collect(),
            Value::Object(map) => map.iter().map(|(k, v)| Ok((k.parse().map_err(|_| bad())?, int(v)?))).collect(),
            _ => Err(bad()),
        }
    }
}

/// Every method, with the number of arguments it takes.
const METHODS: &[(&str, usize)] = &[
    ("Connect", 0),
    ("RequestCreateUser", 4),
    ("RequestLogin", 3),
    ("RequestResumeSession", 1),
    ("RequestLinkLogin", 1),
    ("RequestLogout", 0),
    ("RequestPasswordReset", 1),
    ("RequestSetPassword", 3),
    ("RequestUpdateUserInfo", 3),
    ("RequestSetUserStatus", 1),
    ("RequestHeartbeat", 1),
    ("RequestCreateGame", 4),
    ("RequestLoadGame", 3),
    ("RequestUpdateSettings", 2),
    ("RequestCloseGame", 1),
    ("RequestGameState", 1),
    ("RequestScheduleGame", 6),
    ("RequestCancelGame", 1),
    ("RequestSubscribeGame", 2),
    ("RequestJoinGame", 3),
    ("RequestObserveGame", 2),
    ("RequestReconnectGame", 1),
    ("RequestLeaveGame", 1),
    ("RequestSetOrUnsetHost", 2),
    ("RequestOpenOrCloseSeat", 2),
    ("RequestKick", 2),
    ("RequestAssignSeats", 2),
    ("RequestSetBotSpeed", 2),
    ("RequestNudgeBots", 1),
    ("SubmitEvent", 2),
    ("RequestUndo", 2),
    ("RequestRestoreRecentlyUndone", 1),
    ("RequestDismissRecentlyUndone", 1),
    ("SetTimer", 2),
    ("RequestSetSkin", 2),
    ("RequestPhaseDone", 1),
    ("RequestPhaseHold", 1),
    ("RequestPhaseRelease", 1),
    ("RequestPhaseContinue", 2),
    ("SendChatMessage", 2),
    ("SendGlobalChatMessage", 1),
    ("AdminUpdateMaintenance", 1),
    ("AdminPersistState", 0),
    ("AdminRestoreState", 0),
    ("AdminCloseGame", 1),
    ("AdminDownloadGame", 1),
    ("AdminCancelGame", 1),
    ("DeleteUser", 1),
    ("GetAdminInfo", 0),
    ("QueryDraft", 3),
    ("RequestReport", 2),
    ("RequestUndoList", 1),
    ("RequestUndoPreview", 2),
    ("RequestSaveGame", 1),
];

/// Decodes a call's method and arguments.
pub fn decode_call(method: &str, args: &[Value]) -> Result<Request, String> {
    let Some(&(method, arity)) = METHODS.iter().find(|(m, _)| m.eq_ignore_ascii_case(method)) else {
        return Err(format!("Unknown method '{method}'"));
    };
    if args.len() != arity {
        return Err(format!("{method} takes {arity} arguments, not {}", args.len()));
    }
    let a = Args { method, args };
    let game = || a.string(0);
    use Request::*;
    Ok(match method {
        "Connect" => Connect,
        "RequestCreateUser" => CreateUser { name: a.string(0)?, password_hash: a.string(1)?, email: a.optional_string(2)?, player_name: a.string(3)? },
        "RequestLogin" => Login { version: a.int(0)?, name: a.string(1)?, password_hash: a.string(2)? },
        "RequestResumeSession" => ResumeSession { token: a.string(0)? },
        "RequestLinkLogin" => LinkLogin { token: a.string(0)? },
        "RequestLogout" => Logout,
        "RequestPasswordReset" => PasswordReset { name_or_email: a.string(0)? },
        "RequestSetPassword" => SetPassword { name: a.string(0)?, reset_token: a.string(1)?, password_hash: a.string(2)? },
        "RequestUpdateUserInfo" => UpdateUserInfo { password_hash: a.optional_string(0)?, player_name: a.string(1)?, email: a.optional_string(2)? },
        "RequestSetUserStatus" => SetUserStatus { status: UserStatus(a.int(0)?) },
        "RequestHeartbeat" => Heartbeat { scope: GameListScope(a.int(0)?) },
        "RequestCreateGame" => {
            CreateGame { name: a.optional_string(0)?, password_hash: a.optional_string(1)?, savegame: a.optional_string(2)?, skin: a.optional_string(3)? }
        }
        "RequestLoadGame" => LoadGame { game: game()?, savegame: a.string(1)?, skin: a.optional_string(2)? },
        "RequestUpdateSettings" => UpdateSettings { game: game()?, settings: a.engine(1)? },
        "RequestCloseGame" => CloseGame { game: game()? },
        "RequestGameState" => GameState { game: game()? },
        "RequestScheduleGame" => ScheduleGame {
            starts: a.time(0)?,
            ruleset: a.optional_int(1)?,
            players: a.int(2)?,
            max_turns: a.int(3)?,
            factions: a.json(4)?,
            async_play: a.bool(5)?,
        },
        "RequestCancelGame" => CancelScheduledGame { scheduled_game: a.string(0)? },
        "RequestSubscribeGame" => Subscribe { scheduled_game: a.optional_string(0)?, subscription: SubscriptionType(a.int(1)?) },
        "RequestJoinGame" => JoinGame { game: game()?, password_hash: a.optional_string(1)?, seat: a.int(2)? },
        "RequestObserveGame" => ObserveGame { game: game()?, password_hash: a.optional_string(1)? },
        "RequestReconnectGame" => ReconnectGame { game: game()? },
        "RequestLeaveGame" => LeaveGame { game: game()? },
        "RequestSetOrUnsetHost" => SetOrUnsetHost { game: game()?, user: a.int(1)? },
        "RequestOpenOrCloseSeat" => OpenOrCloseSeat { game: game()?, seat: a.int(1)? },
        "RequestKick" => Kick { game: game()?, user: a.int(1)? },
        "RequestAssignSeats" => AssignSeats { game: game()?, seats: a.int_pairs(1)? },
        "RequestSetBotSpeed" => SetBotSpeed { game: game()?, speed: a.int(1)? },
        "RequestNudgeBots" => NudgeBots { game: game()? },
        "SubmitEvent" => match a.get(1)? {
            Value::Object(event) => SubmitEvent { game: game()?, event: event.clone() },
            _ => return Err("SubmitEvent: the event must be an object".into()),
        },
        "RequestUndo" => Undo { game: game()?, until: a.int(1)? },
        "RequestRestoreRecentlyUndone" => RestoreUndone { game: game()? },
        "RequestDismissRecentlyUndone" => DismissUndone { game: game()? },
        "SetTimer" => SetTimer { game: game()?, value: a.int(1)? },
        "RequestSetSkin" => SetSkin { game: game()?, skin: a.string(1)? },
        "RequestPhaseDone" => PhaseDone { game: game()? },
        "RequestPhaseHold" => PhaseHold { game: game()? },
        "RequestPhaseRelease" => PhaseRelease { game: game()? },
        "RequestPhaseContinue" => PhaseContinue { game: game()?, event_number: a.int(1)? },
        "SendChatMessage" => Chat { game: game()?, message: a.json(1)? },
        "SendGlobalChatMessage" => GlobalChat { message: a.json(0)? },
        "AdminUpdateMaintenance" => SetMaintenance { starts: a.time(0)? },
        "AdminPersistState" => SaveAll,
        "AdminRestoreState" => ReloadAll,
        "AdminCloseGame" => AdminCloseGame { game: game()? },
        "AdminDownloadGame" => DownloadGame { game: game()? },
        "AdminCancelGame" => AdminCancelScheduledGame { scheduled_game: a.string(0)? },
        "DeleteUser" => DeleteUser { user: a.int(0)? },
        "GetAdminInfo" => AdminInfo,
        "QueryDraft" => {
            let draft = match a.get(2)? {
                d @ Value::Object(_) => d.to_string(),
                _ => return Err("QueryDraft: the draft must be an event object".into()),
            };
            QueryDraft { game: game()?, event_number: a.int(1)?, draft }
        }
        "RequestReport" => Report { game: game()?, index: a.int(1)? },
        "RequestUndoList" => UndoList { game: game()? },
        "RequestUndoPreview" => UndoPreview { game: game()?, gap: a.int(1)? },
        "RequestSaveGame" => SaveGame { game: game()? },
        _ => unreachable!("every method in METHODS is decoded"),
    })
}

/// Decodes a frame. `Err` is a malformed message, which closes the connection.
pub fn decode(text: &str) -> Result<Message, String> {
    let v: Value = serde_json::from_str(text).map_err(|e| format!("not JSON: {e}"))?;
    let Value::Object(o) = &v else { return Err("a message must be a JSON object".into()) };
    match o.get("type").and_then(Value::as_str) {
        Some("ping") => Ok(Message::Ping),
        Some("call") => {
            let id = o.get("id").and_then(Value::as_u64).ok_or("a call needs a non-negative integer id")?;
            let method = o.get("method").and_then(Value::as_str);
            let args = match o.get("args") {
                Some(Value::Array(a)) => Some(a.as_slice()),
                None => Some(&[][..]),
                Some(_) => None,
            };
            Ok(match (method, args) {
                (None, _) => Message::BadCall { id, error: "a call needs a method".into() },
                (_, None) => Message::BadCall { id, error: "args must be an array".into() },
                (Some(m), Some(args)) => match decode_call(m, args) {
                    Ok(request) => Message::Call { id, request },
                    Err(error) => Message::BadCall { id, error },
                },
            })
        }
        _ => Err("unknown message type".into()),
    }
}

/// The `Result` object of a call (protocol.md, Result): success with contents.
pub fn success(contents: Value) -> Value {
    json!({"Success": true, "Error": 0, "ErrorDetails": "", "Contents": contents})
}

/// The `Result` object of a call the server refused.
pub fn refusal(error: ErrorType, details: &str) -> Value {
    json!({"Success": false, "Error": error.0, "ErrorDetails": details, "Contents": null})
}

pub fn reply(id: u64, result: &Value) -> String {
    json!({"type": "reply", "id": id, "result": result}).to_string()
}

pub fn reply_error(id: u64, error: &str) -> String {
    json!({"type": "reply", "id": id, "error": error}).to_string()
}

pub fn notify(name: &str, args: Vec<Value>) -> String {
    json!({"type": "notify", "name": name, "args": args}).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelopes() {
        assert!(matches!(decode(r#"{"type":"ping"}"#), Ok(Message::Ping)));
        assert!(decode("[1]").is_err());
        assert!(decode(r#"{"type":"call","method":"Connect"}"#).is_err(), "a call without an id is malformed");
        assert!(matches!(decode(r#"{"type":"call","id":3,"method":"connect","args":[]}"#), Ok(Message::Call { id: 3, request: Request::Connect })));
        match decode(r#"{"type":"call","id":4,"method":"RequestFoo","args":[]}"#) {
            Ok(Message::BadCall { id: 4, error }) => assert_eq!(error, "Unknown method 'RequestFoo'"),
            _ => panic!(),
        }
        assert!(matches!(decode(r#"{"type":"call","id":5,"method":"RequestResumeSession","args":[null]}"#), Ok(Message::BadCall { id: 5, .. })));
        match decode(r#"{"type":"call","id":6,"method":"QueryDraft","args":["g",12,{"$type":"StormDialled","Amount":3}]}"#) {
            Ok(Message::Call { request: Request::QueryDraft { event_number: 12, draft, .. }, .. }) => {
                assert_eq!(draft, r#"{"$type":"StormDialled","Amount":3}"#)
            }
            _ => panic!(),
        }
        for (args, expected) in [(r#"[[5,1],[2,0]]"#, vec![(5, 1), (2, 0)]), (r#"{"2":0}"#, vec![(2, 0)])] {
            let text = format!(r#"{{"type":"call","id":8,"method":"RequestAssignSeats","args":["g",{args}]}}"#);
            match decode(&text) {
                Ok(Message::Call { request: Request::AssignSeats { seats, .. }, .. }) => assert_eq!(seats, expected),
                _ => panic!("{args}"),
            }
        }
        assert_eq!(reply_error(7, "x"), r#"{"type":"reply","id":7,"error":"x"}"#);
        assert_eq!(notify("JoinGame", vec![json!(2), json!("Bob"), json!(-1)]), r#"{"type":"notify","name":"JoinGame","args":[2,"Bob",-1]}"#);
    }
}
