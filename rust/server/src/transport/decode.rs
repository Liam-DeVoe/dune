//! Hub call arguments (C# `Utilities.Serialize` form) to [`HubCall`]s: the `args` of a `call`
//! message (protocol.md), and the scenarios of the equivalence harness (`server-equiv`).
//!
//! The C# harness binds the same JSON to the hub method's parameters with
//! `Utilities.Deserialize<T>`. A `null` where the Rust `HubCall` has no `Option` is a failed call
//! (server-plan.md, coordinator answer 6), reported as bad arguments.

use serde_json::Value;
use treachery_engine::json::FromJson;
use crate::hub::call::HubCall;

/// Every hub method a scenario can call, in the order of `HubCall`. `SubmitEvent` stands for the
/// C# hub's 139 `Request<Event>` methods.
pub const METHODS: &[&str] = &[
    "RequestCreateUser",
    "RequestLogin",
    "RequestPasswordReset",
    "RequestSetPassword",
    "GetLoginInfo",
    "RequestUpdateUserInfo",
    "RequestSetUserStatus",
    "RequestCreateGame",
    "RequestUpdateSettings",
    "RequestCloseGame",
    "RequestJoinGame",
    "RequestObserveGame",
    "RequestReconnectGame",
    "RequestSetOrUnsetHost",
    "RequestOpenOrCloseSeat",
    "RequestLeaveGame",
    "RequestKick",
    "RequestLoadGame",
    "RequestSetSkin",
    "RequestUndo",
    "RequestRestoreRecentlyUndone",
    "RequestDismissRecentlyUndone",
    "RequestGameState",
    "RequestSetBotSpeed",
    "RequestHeartbeat",
    "RequestAssignSeats",
    "RequestScheduleGame",
    "RequestCancelGame",
    "RequestSubscribeGame",
    "SetTimer",
    "RequestPhaseDone",
    "RequestPhaseHold",
    "RequestPhaseRelease",
    "RequestPhaseContinue",
    "SubmitEvent",
    "SendChatMessage",
    "SendGlobalChatMessage",
    "Connect",
    "RequestNudgeBots",
    "AdminUpdateMaintenance",
    "AdminPersistState",
    "AdminRestoreState",
    "AdminCloseGame",
    "AdminDownloadGame",
    "AdminCancelGame",
    "DeleteUser",
    "GetAdminInfo",
];

struct Args<'a> {
    method: &'a str,
    args: &'a [Value],
}

impl Args<'_> {
    fn get(&self, i: usize) -> Result<&Value, String> {
        self.args.get(i).ok_or_else(|| format!("{}: missing argument {i}", self.method))
    }

    fn s(&self, i: usize) -> Result<String, String> {
        match self.get(i)? {
            Value::String(s) => Ok(s.clone()),
            Value::Null => Err(format!("{}: argument {i} is null", self.method)),
            v => Err(format!("{}: argument {i} is not a string: {v}", self.method)),
        }
    }

    fn os(&self, i: usize) -> Result<Option<String>, String> {
        match self.get(i)? {
            Value::Null => Ok(None),
            _ => self.s(i).map(Some),
        }
    }

    fn t<T: FromJson>(&self, i: usize) -> Result<T, String> {
        T::from_json(self.get(i)?).map_err(|e| format!("{}: argument {i}: {e:?}", self.method))
    }
}

/// Builds the call for `method` from its JSON arguments.
pub fn decode(method: &str, args: &[Value]) -> Result<HubCall, String> {
    let a = Args { method, args };
    let expected = match method {
        "Connect" => 0,
        "RequestPasswordReset" | "GetLoginInfo" | "AdminPersistState" | "AdminRestoreState" | "GetAdminInfo" => 1,
        "RequestSetUserStatus" | "RequestCloseGame" | "RequestReconnectGame" | "RequestLeaveGame" | "RequestRestoreRecentlyUndone"
        | "RequestDismissRecentlyUndone" | "RequestGameState" | "RequestHeartbeat" | "RequestCancelGame" | "SendGlobalChatMessage"
        | "RequestNudgeBots" | "RequestPhaseDone" | "RequestPhaseHold" | "RequestPhaseRelease" | "AdminUpdateMaintenance" | "AdminCloseGame" | "AdminDownloadGame" | "AdminCancelGame" | "DeleteUser" => 2,
        "RequestLogin" | "RequestSetPassword" | "RequestUpdateSettings" | "RequestObserveGame" | "RequestSetOrUnsetHost"
        | "RequestOpenOrCloseSeat" | "RequestKick" | "RequestSetSkin" | "RequestUndo" | "RequestSetBotSpeed" | "RequestAssignSeats"
        | "RequestSubscribeGame" | "SetTimer" | "SubmitEvent" | "SendChatMessage" | "RequestPhaseContinue" => 3,
        "RequestCreateUser" | "RequestUpdateUserInfo" | "RequestJoinGame" | "RequestLoadGame" => 4,
        "RequestCreateGame" => 5,
        "RequestScheduleGame" => 7,
        _ => return Err(format!("unknown hub method {method}")),
    };
    if args.len() != expected {
        return Err(format!("{method} takes {expected} arguments, the step has {}", args.len()));
    }

    use HubCall::*;
    Ok(match method {
        "RequestCreateUser" => RequestCreateUser { user_name: a.s(0)?, hashed_password: a.s(1)?, email: a.os(2)?, player_name: a.s(3)? },
        "RequestLogin" => RequestLogin { version: a.t(0)?, user_name: a.s(1)?, hashed_password: a.s(2)? },
        "RequestPasswordReset" => RequestPasswordReset { username_or_email: a.s(0)? },
        "RequestSetPassword" => RequestSetPassword { user_name: a.s(0)?, password_reset_token: a.s(1)?, new_hashed_password: a.s(2)? },
        "GetLoginInfo" => GetLoginInfo { user_token: a.s(0)? },
        "RequestUpdateUserInfo" => {
            RequestUpdateUserInfo { user_token: a.s(0)?, hashed_password: a.os(1)?, player_name: a.s(2)?, email: a.os(3)? }
        }
        "RequestSetUserStatus" => RequestSetUserStatus { user_token: a.s(0)?, status: a.t(1)? },
        "RequestCreateGame" => {
            RequestCreateGame { name: a.os(0)?, user_token: a.s(1)?, hashed_password: a.os(2)?, state_data: a.os(3)?, skin: a.os(4)? }
        }
        "RequestUpdateSettings" => RequestUpdateSettings { user_token: a.s(0)?, game_id: a.s(1)?, settings: a.t(2)? },
        "RequestCloseGame" => RequestCloseGame { user_token: a.s(0)?, game_id: a.s(1)? },
        "RequestJoinGame" => RequestJoinGame { user_token: a.s(0)?, game_id: a.s(1)?, hashed_password: a.os(2)?, seat: a.t(3)? },
        "RequestObserveGame" => RequestObserveGame { user_token: a.s(0)?, game_id: a.s(1)?, hashed_password: a.os(2)? },
        "RequestReconnectGame" => RequestReconnectGame { user_token: a.s(0)?, game_id: a.s(1)? },
        "RequestSetOrUnsetHost" => RequestSetOrUnsetHost { user_token: a.s(0)?, game_id: a.s(1)?, user_id: a.t(2)? },
        "RequestOpenOrCloseSeat" => RequestOpenOrCloseSeat { user_token: a.s(0)?, game_id: a.s(1)?, seat: a.t(2)? },
        "RequestLeaveGame" => RequestLeaveGame { user_token: a.s(0)?, game_id: a.s(1)? },
        "RequestKick" => RequestKick { user_token: a.s(0)?, game_id: a.s(1)?, user_id: a.t(2)? },
        "RequestLoadGame" => RequestLoadGame { user_token: a.s(0)?, game_id: a.s(1)?, state_data: a.s(2)?, skin: a.os(3)? },
        "RequestSetSkin" => RequestSetSkin { user_token: a.s(0)?, game_id: a.s(1)?, skin: a.s(2)? },
        "RequestUndo" => RequestUndo { user_token: a.s(0)?, game_id: a.s(1)?, until_event_nr: a.t(2)? },
        "RequestRestoreRecentlyUndone" => RequestRestoreRecentlyUndone { user_token: a.s(0)?, game_id: a.s(1)? },
        "RequestDismissRecentlyUndone" => RequestDismissRecentlyUndone { user_token: a.s(0)?, game_id: a.s(1)? },
        "RequestGameState" => RequestGameState { user_token: a.s(0)?, game_id: a.s(1)? },
        "RequestSetBotSpeed" => RequestSetBotSpeed { user_token: a.s(0)?, game_id: a.s(1)?, speed: a.t(2)? },
        "RequestHeartbeat" => RequestHeartbeat { user_token: a.s(0)?, scope: a.t(1)? },
        "RequestAssignSeats" => RequestAssignSeats {
            user_token: a.s(0)?,
            game_id: a.s(1)?,
            assignment: crate::json::read_wire_int_dict(a.get(2)?).map_err(|e| format!("{method}: assignment: {e:?}"))?,
        },
        "RequestScheduleGame" => RequestScheduleGame {
            user_token: a.s(0)?,
            date_time: a.t(1)?,
            ruleset: a.t(2)?,
            number_of_players: a.t(3)?,
            maximum_turns: a.t(4)?,
            allowed_factions_in_play: a.t(5)?,
            async_play: a.t(6)?,
        },
        "RequestCancelGame" => RequestCancelGame { user_token: a.s(0)?, scheduled_game_id: a.s(1)? },
        "RequestSubscribeGame" => RequestSubscribeGame { user_token: a.s(0)?, game_id: a.os(1)?, subscription: a.t(2)? },
        "SetTimer" => SetTimer { user_token: a.s(0)?, game_id: a.s(1)?, value: a.t(2)? },
        "RequestPhaseDone" => RequestPhaseDone { user_token: a.s(0)?, game_id: a.s(1)? },
        "RequestPhaseHold" => RequestPhaseHold { user_token: a.s(0)?, game_id: a.s(1)? },
        "RequestPhaseRelease" => RequestPhaseRelease { user_token: a.s(0)?, game_id: a.s(1)? },
        "RequestPhaseContinue" => RequestPhaseContinue { user_token: a.s(0)?, game_id: a.s(1)?, event_number: a.t(2)? },
        "SubmitEvent" => SubmitEvent { user_token: a.s(0)?, game_id: a.s(1)?, event: a.t(2)? },
        "SendChatMessage" => SendChatMessage { user_token: a.s(0)?, game_id: a.s(1)?, message: a.t(2)? },
        "SendGlobalChatMessage" => SendGlobalChatMessage { user_token: a.s(0)?, message: a.t(1)? },
        "Connect" => Connect,
        "RequestNudgeBots" => RequestNudgeBots { user_token: a.s(0)?, game_id: a.s(1)? },
        "AdminUpdateMaintenance" => AdminUpdateMaintenance { user_token: a.s(0)?, maintenance_date: a.t(1)? },
        "AdminPersistState" => AdminPersistState { user_token: a.s(0)? },
        "AdminRestoreState" => AdminRestoreState { user_token: a.s(0)? },
        "AdminCloseGame" => AdminCloseGame { user_token: a.s(0)?, game_id: a.s(1)? },
        "AdminDownloadGame" => AdminDownloadGame { user_token: a.s(0)?, game_id: a.s(1)? },
        "AdminCancelGame" => AdminCancelGame { user_token: a.s(0)?, scheduled_game_id: a.s(1)? },
        "DeleteUser" => DeleteUser { user_token: a.s(0)?, user_id: a.t(1)? },
        "GetAdminInfo" => GetAdminInfo { user_token: a.s(0)? },
        _ => unreachable!(),
    })
}
