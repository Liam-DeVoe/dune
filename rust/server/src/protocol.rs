//! The message format of rust/docs/protocol.md: decoding a client's frame into a [`Request`],
//! and encoding replies and notifications.
//!
//! Decoding runs on the game thread, because a request can hold engine values (events), which
//! are not `Send`. The socket tasks only move text.

use serde_json::{json, Value};
use treachery_engine::enumerations::{ErrorType, GameListScope, SubscriptionType, UserStatus};
use treachery_engine::events::GameEvent;
use treachery_engine::game_settings::GameSettings;
use treachery_engine::json::FromJson;

use super::time::{self, Time};
use super::wire::{GameChatMessage, GlobalChatMessage};

/// The largest frame a client may send.
pub const MAX_MESSAGE_BYTES: usize = 4 * 1024 * 1024;

/// A call, by its method in protocol.md. `token` is the caller's session token.
pub enum Request {
    // Server and accounts
    Connect,
    CreateUser { name: String, password_hash: String, email: Option<String>, player_name: String },
    Login { version: i32, name: String, password_hash: String },
    LoginInfo { token: String },
    PasswordReset { name_or_email: String },
    SetPassword { name: String, reset_token: String, password_hash: String },
    UpdateUserInfo { token: String, password_hash: Option<String>, player_name: String, email: Option<String> },
    SetUserStatus { token: String, status: UserStatus },
    Heartbeat { token: String, scope: GameListScope },

    // Lobby and game management
    CreateGame { name: Option<String>, token: String, password_hash: Option<String>, savegame: Option<String>, skin: Option<String> },
    LoadGame { token: String, game: String, savegame: String, skin: Option<String> },
    UpdateSettings { token: String, game: String, settings: GameSettings },
    CloseGame { token: String, game: String },
    GameState { token: String, game: String },
    ScheduleGame { token: String, starts: Time, ruleset: Option<i32>, players: i32, max_turns: i32, factions: Vec<i32>, async_play: bool },
    CancelScheduledGame { token: String, scheduled_game: String },
    Subscribe { token: String, scheduled_game: Option<String>, subscription: SubscriptionType },

    // Seating and participation
    JoinGame { token: String, game: String, password_hash: Option<String>, seat: i32 },
    ObserveGame { token: String, game: String, password_hash: Option<String> },
    ReconnectGame { token: String, game: String },
    LeaveGame { token: String, game: String },
    SetOrUnsetHost { token: String, game: String, user: i32 },
    OpenOrCloseSeat { token: String, game: String, seat: i32 },
    Kick { token: String, game: String, user: i32 },
    AssignSeats { token: String, game: String, seats: Vec<(i32, i32)> },
    SetBotSpeed { token: String, game: String, speed: i32 },
    NudgeBots { token: String, game: String },

    // Playing
    SubmitEvent { token: String, game: String, event: GameEvent },
    Undo { token: String, game: String, until: i32 },
    RestoreUndone { token: String, game: String },
    DismissUndone { token: String, game: String },
    SetTimer { token: String, game: String, value: i32 },
    SetSkin { token: String, game: String, skin: String },
    PhaseDone { token: String, game: String },
    PhaseHold { token: String, game: String },
    PhaseRelease { token: String, game: String },
    PhaseContinue { token: String, game: String, event_number: i32 },

    // Chat
    Chat { token: String, game: String, message: GameChatMessage },
    GlobalChat { token: String, message: GlobalChatMessage },

    // Admin
    SetMaintenance { token: String, starts: Time },
    SaveAll { token: String },
    ReloadAll { token: String },
    AdminCloseGame { token: String, game: String },
    DownloadGame { token: String, game: String },
    AdminCancelScheduledGame { token: String, scheduled_game: String },
    DeleteUser { token: String, user: i32 },
    AdminInfo { token: String },

    // Game view
    QueryDraft { token: String, game: String, event_number: i32, draft: String },
    Report { token: String, game: String, index: i32 },
    UndoList { token: String, game: String },
    UndoPreview { token: String, game: String, gap: i32 },
    SaveGame { token: String, game: String },
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
    ("GetLoginInfo", 1),
    ("RequestPasswordReset", 1),
    ("RequestSetPassword", 3),
    ("RequestUpdateUserInfo", 4),
    ("RequestSetUserStatus", 2),
    ("RequestHeartbeat", 2),
    ("RequestCreateGame", 5),
    ("RequestLoadGame", 4),
    ("RequestUpdateSettings", 3),
    ("RequestCloseGame", 2),
    ("RequestGameState", 2),
    ("RequestScheduleGame", 7),
    ("RequestCancelGame", 2),
    ("RequestSubscribeGame", 3),
    ("RequestJoinGame", 4),
    ("RequestObserveGame", 3),
    ("RequestReconnectGame", 2),
    ("RequestLeaveGame", 2),
    ("RequestSetOrUnsetHost", 3),
    ("RequestOpenOrCloseSeat", 3),
    ("RequestKick", 3),
    ("RequestAssignSeats", 3),
    ("RequestSetBotSpeed", 3),
    ("RequestNudgeBots", 2),
    ("SubmitEvent", 3),
    ("RequestUndo", 3),
    ("RequestRestoreRecentlyUndone", 2),
    ("RequestDismissRecentlyUndone", 2),
    ("SetTimer", 3),
    ("RequestSetSkin", 3),
    ("RequestPhaseDone", 2),
    ("RequestPhaseHold", 2),
    ("RequestPhaseRelease", 2),
    ("RequestPhaseContinue", 3),
    ("SendChatMessage", 3),
    ("SendGlobalChatMessage", 2),
    ("AdminUpdateMaintenance", 2),
    ("AdminPersistState", 1),
    ("AdminRestoreState", 1),
    ("AdminCloseGame", 2),
    ("AdminDownloadGame", 2),
    ("AdminCancelGame", 2),
    ("DeleteUser", 2),
    ("GetAdminInfo", 1),
    ("QueryDraft", 4),
    ("RequestReport", 3),
    ("RequestUndoList", 2),
    ("RequestUndoPreview", 3),
    ("RequestSaveGame", 2),
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
    let token = || a.string(0);
    let game = || a.string(1);
    use Request::*;
    Ok(match method {
        "Connect" => Connect,
        "RequestCreateUser" => CreateUser { name: a.string(0)?, password_hash: a.string(1)?, email: a.optional_string(2)?, player_name: a.string(3)? },
        "RequestLogin" => Login { version: a.int(0)?, name: a.string(1)?, password_hash: a.string(2)? },
        "GetLoginInfo" => LoginInfo { token: token()? },
        "RequestPasswordReset" => PasswordReset { name_or_email: a.string(0)? },
        "RequestSetPassword" => SetPassword { name: a.string(0)?, reset_token: a.string(1)?, password_hash: a.string(2)? },
        "RequestUpdateUserInfo" => {
            UpdateUserInfo { token: token()?, password_hash: a.optional_string(1)?, player_name: a.string(2)?, email: a.optional_string(3)? }
        }
        "RequestSetUserStatus" => SetUserStatus { token: token()?, status: UserStatus(a.int(1)?) },
        "RequestHeartbeat" => Heartbeat { token: token()?, scope: GameListScope(a.int(1)?) },
        "RequestCreateGame" => CreateGame {
            name: a.optional_string(0)?,
            token: a.string(1)?,
            password_hash: a.optional_string(2)?,
            savegame: a.optional_string(3)?,
            skin: a.optional_string(4)?,
        },
        "RequestLoadGame" => LoadGame { token: token()?, game: game()?, savegame: a.string(2)?, skin: a.optional_string(3)? },
        "RequestUpdateSettings" => UpdateSettings { token: token()?, game: game()?, settings: a.engine(2)? },
        "RequestCloseGame" => CloseGame { token: token()?, game: game()? },
        "RequestGameState" => GameState { token: token()?, game: game()? },
        "RequestScheduleGame" => ScheduleGame {
            token: token()?,
            starts: a.time(1)?,
            ruleset: a.optional_int(2)?,
            players: a.int(3)?,
            max_turns: a.int(4)?,
            factions: a.json(5)?,
            async_play: a.bool(6)?,
        },
        "RequestCancelGame" => CancelScheduledGame { token: token()?, scheduled_game: a.string(1)? },
        "RequestSubscribeGame" => Subscribe { token: token()?, scheduled_game: a.optional_string(1)?, subscription: SubscriptionType(a.int(2)?) },
        "RequestJoinGame" => JoinGame { token: token()?, game: game()?, password_hash: a.optional_string(2)?, seat: a.int(3)? },
        "RequestObserveGame" => ObserveGame { token: token()?, game: game()?, password_hash: a.optional_string(2)? },
        "RequestReconnectGame" => ReconnectGame { token: token()?, game: game()? },
        "RequestLeaveGame" => LeaveGame { token: token()?, game: game()? },
        "RequestSetOrUnsetHost" => SetOrUnsetHost { token: token()?, game: game()?, user: a.int(2)? },
        "RequestOpenOrCloseSeat" => OpenOrCloseSeat { token: token()?, game: game()?, seat: a.int(2)? },
        "RequestKick" => Kick { token: token()?, game: game()?, user: a.int(2)? },
        "RequestAssignSeats" => AssignSeats { token: token()?, game: game()?, seats: a.int_pairs(2)? },
        "RequestSetBotSpeed" => SetBotSpeed { token: token()?, game: game()?, speed: a.int(2)? },
        "RequestNudgeBots" => NudgeBots { token: token()?, game: game()? },
        "SubmitEvent" => SubmitEvent { token: token()?, game: game()?, event: a.engine(2)? },
        "RequestUndo" => Undo { token: token()?, game: game()?, until: a.int(2)? },
        "RequestRestoreRecentlyUndone" => RestoreUndone { token: token()?, game: game()? },
        "RequestDismissRecentlyUndone" => DismissUndone { token: token()?, game: game()? },
        "SetTimer" => SetTimer { token: token()?, game: game()?, value: a.int(2)? },
        "RequestSetSkin" => SetSkin { token: token()?, game: game()?, skin: a.string(2)? },
        "RequestPhaseDone" => PhaseDone { token: token()?, game: game()? },
        "RequestPhaseHold" => PhaseHold { token: token()?, game: game()? },
        "RequestPhaseRelease" => PhaseRelease { token: token()?, game: game()? },
        "RequestPhaseContinue" => PhaseContinue { token: token()?, game: game()?, event_number: a.int(2)? },
        "SendChatMessage" => Chat { token: token()?, game: game()?, message: a.json(2)? },
        "SendGlobalChatMessage" => GlobalChat { token: token()?, message: a.json(1)? },
        "AdminUpdateMaintenance" => SetMaintenance { token: token()?, starts: a.time(1)? },
        "AdminPersistState" => SaveAll { token: token()? },
        "AdminRestoreState" => ReloadAll { token: token()? },
        "AdminCloseGame" => AdminCloseGame { token: token()?, game: game()? },
        "AdminDownloadGame" => DownloadGame { token: token()?, game: game()? },
        "AdminCancelGame" => AdminCancelScheduledGame { token: token()?, scheduled_game: a.string(1)? },
        "DeleteUser" => DeleteUser { token: token()?, user: a.int(1)? },
        "GetAdminInfo" => AdminInfo { token: token()? },
        "QueryDraft" => {
            let draft = match a.get(3)? {
                d @ Value::Object(_) => d.to_string(),
                _ => return Err("QueryDraft: the draft must be an event object".into()),
            };
            QueryDraft { token: token()?, game: game()?, event_number: a.int(2)?, draft }
        }
        "RequestReport" => Report { token: token()?, game: game()?, index: a.int(2)? },
        "RequestUndoList" => UndoList { token: token()?, game: game()? },
        "RequestUndoPreview" => UndoPreview { token: token()?, game: game()?, gap: a.int(2)? },
        "RequestSaveGame" => SaveGame { token: token()?, game: game()? },
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
        assert!(matches!(decode(r#"{"type":"call","id":5,"method":"GetLoginInfo","args":[null]}"#), Ok(Message::BadCall { id: 5, .. })));
        match decode(r#"{"type":"call","id":6,"method":"QueryDraft","args":["t","g",12,{"$type":"StormDialled","Amount":3}]}"#) {
            Ok(Message::Call { request: Request::QueryDraft { event_number: 12, draft, .. }, .. }) => {
                assert_eq!(draft, r#"{"$type":"StormDialled","Amount":3}"#)
            }
            _ => panic!(),
        }
        for (args, expected) in [(r#"[[5,1],[2,0]]"#, vec![(5, 1), (2, 0)]), (r#"{"2":0}"#, vec![(2, 0)])] {
            let text = format!(r#"{{"type":"call","id":8,"method":"RequestAssignSeats","args":["t","g",{args}]}}"#);
            match decode(&text) {
                Ok(Message::Call { request: Request::AssignSeats { seats, .. }, .. }) => assert_eq!(seats, expected),
                _ => panic!("{args}"),
            }
        }
        assert_eq!(reply_error(7, "x"), r#"{"type":"reply","id":7,"error":"x"}"#);
        assert_eq!(notify("JoinGame", vec![json!(2), json!("Bob"), json!(-1)]), r#"{"type":"notify","name":"JoinGame","args":[2,"Bob",-1]}"#);
    }
}
