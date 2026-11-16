//! Every message between the web client and the server (rust/docs/protocol.md).
//!
//! A client sends calls, `{"id":7,"call":"joinGame","args":{...}}` ([`Request`]). The server
//! answers each with `{"id":7,"ok":...}` or `{"id":7,"err":{"code","message"}}`
//! ([`ServerMessage`]), and sends notifications, `{"notify":"joinGame","data":{...}}`
//! ([`Notification`]). Each call's reply type is in [`replies`], and `tests/ts_bindings.rs`
//! generates the TypeScript types of all of them in `web/src/protocol/`.
//!
//! Events and game settings are still in the savegame format inside `submitEvent`,
//! `queryDraft`, `updateSettings` and the `updateSettings` notification.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use treachery_view::cache::UndoEntry;
use treachery_view::schema::{DraftResult, GameView, ReportLines};
use ts_rs::TS;

mod error;
mod ids;
mod lobby;
pub mod time;

pub use error::{CallError, ErrorCode};
pub use ids::{seated_users, UserId};
pub use treachery_engine::game::Seat;
pub use lobby::*;

/// The largest frame a client may send.
pub const MAX_MESSAGE_BYTES: usize = 4 * 1024 * 1024;

/// An event or draft in the savegame format: `{"$type":"Bid", ...}`.
pub type SavegameEvent = Map<String, Value>;

/// Declares the calls: each one's wire name, arguments and reply type. Makes the [`Call`]
/// enum, a type alias per call in [`replies`] for the server to check its replies against, and
/// [`CallReplies`], which only exists to give TypeScript the reply type of each call.
macro_rules! calls {
    ($(
        $(#[doc = $doc:literal])*
        $name:literal $variant:ident $({ $( $(#[$field_meta:meta])* $field:ident : $ty:ty ),* $(,)? })? => $reply:ty;
    )*) => {
        /// A call, by its method. The caller is the user the connection is authenticated as; no
        /// call names its user.
        #[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
        #[serde(tag = "call", content = "args", rename_all_fields = "camelCase")]
        pub enum Call {
            $(
                $(#[doc = $doc])*
                #[serde(rename = $name)]
                $variant $({ $( $(#[$field_meta])* $field: $ty ),* })?,
            )*
        }

        /// The type of each call's `ok`.
        pub mod replies {
            #[allow(unused_imports)]
            use super::*;
            $( pub type $variant = $reply; )*
        }

        /// The reply type of each call, for TypeScript: `CallReplies["joinGame"]` is
        /// `GameInitInfo`.
        #[derive(TS)]
        #[allow(non_snake_case, dead_code)]
        pub struct CallReplies {
            $(
                #[ts(rename = $name)]
                $variant: $reply,
            )*
        }

        impl Call {
            /// The call's wire name.
            pub fn name(&self) -> &'static str {
                match self {
                    $( Call::$variant { .. } => $name, )*
                }
            }
        }
    };
}

calls! {
    // Server and accounts

    /// The server's details. Needs no login.
    "serverInfo" GetServerInfo => ServerInfo;
    /// Creates an account and authenticates the connection with a new session.
    "createUser" CreateUser { user_name: String, password_hash: String, email: Option<String>, player_name: String } => NewSession;
    /// Authenticates the connection with a new session. `version` is the client's engine
    /// version, which must be the server's.
    "login" Login { version: i32, user_name: String, password_hash: String } => NewSession;
    /// Authenticates the connection with the token of a session started earlier. A new
    /// WebSocket sends this first.
    "authenticate" Authenticate { token: String } => Account;
    /// Authenticates the connection with a new session, from the single-use link of an
    /// async-play mail.
    "linkLogin" LinkLogin { token: String } => LinkLoginInfo;
    /// Ends the connection's session, which logs out every connection that uses it.
    "logout" Logout => ();
    /// Mails a password reset token.
    "requestPasswordReset" RequestPasswordReset { user_name_or_email: String } => ();
    /// A new password with a mailed reset token. Ends the user's other sessions and
    /// authenticates the connection with a new one.
    "setPassword" SetPassword { user_name: String, reset_token: String, password_hash: String } => NewSession;
    /// A null or empty `password_hash` keeps the password. A new password ends the user's other
    /// sessions.
    "updateUserInfo" UpdateUserInfo { password_hash: Option<String>, player_name: String, email: Option<String> } => Account;
    "setUserStatus" SetUserStatus { status: UserStatus } => Lobby;
    /// Keeps the user on the lobby's user list, which shows users seen in the last minute.
    "heartbeat" Heartbeat { scope: GameListScope } => Lobby;

    // Lobby and game management

    /// A new game, or one from a savegame. Empty or null `name` names it after the creator.
    "createGame" CreateGame { name: Option<String>, password_hash: Option<String>, savegame: Option<String>, skin: Option<String> } => GameInitInfo;
    /// Hosts: replaces the game with a savegame.
    "loadGame" LoadGame { game: String, savegame: String, skin: Option<String> } => ();
    /// Hosts: the settings for the game's start, in the savegame format.
    "updateSettings" UpdateSettings { game: String, #[ts(type = "unknown")] settings: Value } => ();
    /// The creator: removes everyone and deletes the game.
    "closeGame" CloseGame { game: String } => Lobby;
    /// Members: the game's participation, and the caller's view again.
    "gameState" GameState { game: String } => GameInitInfo;
    "scheduleGame" ScheduleGame {
        #[serde(with = "time::serde_time")]
        #[ts(type = "string")]
        starts: time::Time,
        #[ts(type = "Ruleset | null")]
        ruleset: Option<String>,
        players: i32,
        max_turns: i32,
        #[ts(type = "Array<Faction>")]
        factions: Vec<String>,
        async_play: bool,
    } => Lobby;
    /// The scheduled game's creator.
    "cancelScheduledGame" CancelScheduledGame { scheduled_game: String } => Lobby;
    "subscribe" Subscribe { scheduled_game: String, subscription: SubscriptionType } => Lobby;

    // Seating and participation

    /// A null `seat` joins without a particular seat, which works only while the game awaits
    /// players. A particular seat must be one a host opened.
    "joinGame" JoinGame { game: String, password_hash: Option<String>, seat: Option<Seat> } => GameInitInfo;
    "observeGame" ObserveGame { game: String, password_hash: Option<String> } => GameInitInfo;
    /// Members: a new connection follows the game again.
    "reconnectGame" ReconnectGame { game: String } => GameInitInfo;
    /// Members.
    "leaveGame" LeaveGame { game: String } => Lobby;
    /// Hosts: `user` must be a player of the game.
    "setOrUnsetHost" SetOrUnsetHost { game: String, user: UserId } => ();
    /// Hosts: `seat` must be a player's seat in the game.
    "openOrCloseSeat" OpenOrCloseSeat { game: String, seat: Seat } => ();
    /// Hosts: `user` must be another member of the game.
    "kick" Kick { game: String, user: UserId } => ();
    /// Hosts: `[userId, seat]` for each player, -1 for no seat. Each user a player of the game,
    /// each seat a player's seat, no user or seat twice.
    "assignSeats" AssignSeats {
        game: String,
        #[serde(with = "ids::seated_users")]
        #[ts(type = "Array<[number, number]>")]
        seats: Vec<(UserId, Option<Seat>)>,
    } => ();
    /// Hosts: 0 to 4.
    "setBotSpeed" SetBotSpeed { game: String, speed: i32 } => ();
    /// Players: runs the bots' turn at once.
    "nudgeBots" NudgeBots { game: String } => ();

    // Playing

    /// Players: an event in the savegame format. The server sets who made it.
    "submitEvent" SubmitEvent { game: String, #[ts(type = "Record<string, unknown>")] event: SavegameEvent } => ();
    /// Hosts: back to the moment before event `until`.
    "undo" Undo { game: String, until: i32 } => ();
    "restoreUndone" RestoreUndone { game: String } => ();
    "dismissUndone" DismissUndone { game: String } => ();
    /// Hosts: the bidding time limit in seconds, or -1 for none.
    "setTimer" SetTimer { game: String, value: i32 } => ();
    /// Hosts: a skin's JSON.
    "setSkin" SetSkin { game: String, skin: String } => ();
    /// Players: the caller has acted or passes in the current pause.
    "phaseDone" PhaseDone { game: String } => ();
    /// Players: holds the running countdown.
    "phaseHold" PhaseHold { game: String } => ();
    /// Players: stops holding.
    "phaseRelease" PhaseRelease { game: String } => ();
    /// Hosts: ends the current pause, but only while the game has `event_number` events.
    "phaseContinue" PhaseContinue { game: String, event_number: i32 } => ();

    // Chat

    /// Players: to everyone in the game, or with `to` to one member and the sender.
    "chat" Chat { game: String, to: Option<UserId>, body: String } => ();
    "globalChat" GlobalChat { body: String } => ();

    // Admin: the user named in TREACHERY_ADMIN. Each answers with a text for the admin page.

    "adminSetMaintenance" AdminSetMaintenance {
        #[serde(with = "time::serde_time")]
        #[ts(type = "string")]
        starts: time::Time,
    } => String;
    /// Saves every game with unsaved changes.
    "adminSaveAll" AdminSaveAll => String;
    /// Loads the saved games that aren't running.
    "adminReloadAll" AdminReloadAll => String;
    "adminCloseGame" AdminCloseGame { game: String } => String;
    /// The savegame.
    "adminDownloadGame" AdminDownloadGame { game: String } => String;
    "adminCancelScheduledGame" AdminCancelScheduledGame { scheduled_game: String } => String;
    /// The admin, or a user deleting their own account.
    "deleteUser" DeleteUser { user: UserId } => String;
    "adminInfo" GetAdminInfo => AdminInfo;

    // The game view

    /// Players: the options and validation of a draft of one of the caller's actions.
    "queryDraft" QueryDraft { game: String, event_number: i32, #[ts(type = "Record<string, unknown>")] draft: SavegameEvent } => DraftResult;
    /// Members: the lines of a report of `log.reports`.
    "report" Report { game: String, index: i32 } => ReportLines;
    /// Hosts: every event, for the undo panel.
    "undoList" UndoList { game: String } => Vec<UndoEntry>;
    /// Hosts: the caller's view at gap `gap`, just before event `gap`, without actions.
    "undoPreview" UndoPreview { game: String, gap: i32 } => GameView;
    /// Hosts and the admin: the live game as a savegame.
    "saveGame" SaveGame { game: String } => String;
}

/// A call with the id the client chose. The server echoes the id in the reply.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
pub struct Request {
    #[ts(type = "number")]
    pub id: u64,
    #[serde(flatten)]
    pub call: Call,
}

/// How a frame from a client decodes.
#[derive(Debug)]
pub enum Decoded {
    Call { id: u64, call: Call },
    /// An object with an id that isn't a call the server knows: answered with `badCall`.
    BadCall { id: u64, error: String },
}

/// Decodes a frame. `Err` is a frame that isn't a JSON object with a non-negative integer `id`,
/// which closes the connection.
pub fn decode(text: &str) -> Result<Decoded, String> {
    let value: Value = serde_json::from_str(text).map_err(|e| format!("not JSON: {e}"))?;
    let Value::Object(mut object) = value else { return Err("a message must be a JSON object".into()) };
    let id = object.remove("id").and_then(|id| id.as_u64()).ok_or("a call needs a non-negative integer id")?;
    Ok(match serde_json::from_value::<Call>(Value::Object(object)) {
        Ok(call) => Decoded::Call { id, call },
        Err(e) => Decoded::BadCall { id, error: e.to_string() },
    })
}

/// A notification: something happened that the client didn't ask about, or not only it.
#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(tag = "notify", content = "data", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Notification {
    /// The receiver's own view of a game it follows.
    GameView(Box<GameView>),
    /// A host loaded a savegame. A `gameView` follows.
    LoadGame(GameInitInfo),
    /// The settings a host chose for the game's start, in the savegame format.
    UpdateSettings {
        #[ts(type = "unknown")]
        settings: Value,
    },
    /// A user joined, on `seat` or -1 for none yet.
    JoinGame {
        user: UserId,
        name: String,
        #[serde(with = "treachery_engine::game::seat_or_minus_one")]
        #[ts(type = "number")]
        seat: Option<Seat>,
    },
    ObserveGame { user: UserId, name: String },
    /// The user became a host, or is no longer one.
    SetOrUnsetHost { user: UserId },
    /// Each seat was opened if it was closed, and closed if it was open.
    OpenOrCloseSeats { seats: Vec<Seat> },
    /// The user left or, with `kick`, was removed for good.
    RemoveUser { user: UserId, kick: bool },
    /// `[userId, seat]`: the seats every player now has.
    AssignSeats {
        #[serde(with = "ids::seated_users")]
        #[ts(type = "Array<[number, number]>")]
        seats: Vec<(UserId, Option<Seat>)>,
    },
    BotSpeed { speed: i32 },
    SetTimer { value: i32 },
    SetSkin { skin: String },
    /// What the current pause waits for changed.
    PhaseControl(PhaseControlInfo),
    ChatMessage(ChatMessage),
    GlobalChatMessage(GlobalChatMessage),
    /// The session the connection was authenticated with ended. The connection is now nobody,
    /// and follows no game.
    SessionEnded,
}

/// What the server sends: a reply to a call, or a notification.
#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(untagged)]
pub enum ServerMessage {
    /// The call succeeded. `ok` is the call's reply type ([`CallReplies`]).
    Ok {
        #[ts(type = "number")]
        id: u64,
        #[ts(type = "unknown")]
        ok: Value,
    },
    Err {
        #[ts(type = "number")]
        id: u64,
        err: CallError,
    },
    Notify(Notification),
}

impl ServerMessage {
    pub fn encode(&self) -> String {
        serde_json::to_string(self).expect("messages serialize")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn call(text: &str) -> Call {
        match decode(text) {
            Ok(Decoded::Call { call, .. }) => call,
            other => panic!("{text}: {other:?}"),
        }
    }

    #[test]
    fn calls_decode() {
        assert_eq!(call(r#"{"id":1,"call":"serverInfo"}"#), Call::GetServerInfo);
        assert_eq!(call(r#"{"id":1,"call":"serverInfo","args":null}"#), Call::GetServerInfo);
        assert_eq!(
            call(r#"{"id":7,"call":"joinGame","args":{"game":"g","passwordHash":null,"seat":3}}"#),
            Call::JoinGame { game: "g".into(), password_hash: None, seat: Some(Seat::new(3)) }
        );
        // Optional arguments may be left out.
        assert_eq!(call(r#"{"id":7,"call":"joinGame","args":{"game":"g"}}"#), Call::JoinGame { game: "g".into(), password_hash: None, seat: None });
        assert_eq!(
            call(r#"{"id":8,"call":"assignSeats","args":{"game":"g","seats":[[5,1],[2,-1]]}}"#),
            Call::AssignSeats { game: "g".into(), seats: vec![(UserId(5), Some(Seat::new(1))), (UserId(2), None)] }
        );
        match call(r#"{"id":6,"call":"queryDraft","args":{"game":"g","eventNumber":12,"draft":{"$type":"StormDialled","Amount":3}}}"#) {
            Call::QueryDraft { event_number: 12, draft, .. } => assert_eq!(Value::Object(draft), json!({"$type": "StormDialled", "Amount": 3})),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn bad_calls_and_malformed_frames() {
        for text in [
            r#"{"id":4,"call":"requestFoo"}"#,
            r#"{"id":4,"call":"authenticate"}"#,
            r#"{"id":4,"call":"authenticate","args":{"token":12}}"#,
            r#"{"id":4,"call":"JoinGame","args":{"game":"g"}}"#,
            // -1 is no seat only where a user may have none; other numbers name no seat.
            r#"{"id":4,"call":"openOrCloseSeat","args":{"game":"g","seat":-1}}"#,
            r#"{"id":4,"call":"assignSeats","args":{"game":"g","seats":[[5,-2]]}}"#,
            r#"{"id":4,"call":"assignSeats","args":{"game":"g","seats":[[5,256]]}}"#,
            r#"{"id":4}"#,
        ] {
            assert!(matches!(decode(text), Ok(Decoded::BadCall { id: 4, .. })), "{text}");
        }
        for text in ["[1]", "nope", r#"{"call":"serverInfo"}"#, r#"{"id":-1,"call":"serverInfo"}"#, r#"{"id":"1","call":"serverInfo"}"#] {
            assert!(decode(text).is_err(), "{text}");
        }
    }

    #[test]
    fn server_messages_encode() {
        assert_eq!(ServerMessage::Ok { id: 7, ok: Value::Null }.encode(), r#"{"id":7,"ok":null}"#);
        assert_eq!(
            ServerMessage::Err { id: 7, err: CallError::new(ErrorCode::SeatNotAvailableNotOpen) }.encode(),
            r#"{"id":7,"err":{"code":"seatNotAvailableNotOpen","message":"Seat is not available"}}"#
        );
        assert_eq!(
            ServerMessage::Notify(Notification::JoinGame { user: UserId(2), name: "Bob".into(), seat: None }).encode(),
            r#"{"notify":"joinGame","data":{"user":2,"name":"Bob","seat":-1}}"#
        );
        assert_eq!(ServerMessage::Notify(Notification::SessionEnded).encode(), r#"{"notify":"sessionEnded"}"#);
        let t = time::parse("2026-11-20T19:00:00Z").unwrap();
        let info = PhaseControlInfo { active: true, waiting_for: vec![Seat::new(1), Seat::new(2)], countdown: true, countdown_ends: Some(t), holding: vec![], reasons: vec![PauseWait { seat: Seat::new(1), reasons: vec![PauseReason::KaramaSandworm] }] };
        assert_eq!(
            ServerMessage::Notify(Notification::PhaseControl(info)).encode(),
            r#"{"notify":"phaseControl","data":{"active":true,"waitingFor":[1,2],"countdown":true,"countdownEnds":"2026-11-20T19:00:00.000000Z","holding":[],"reasons":[{"seat":1,"reasons":["karamaSandworm"]}]}}"#
        );
    }
}
