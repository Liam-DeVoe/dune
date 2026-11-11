//! A hub call as data, independent of the wire format: one variant per hub method, with the
//! C# parameters. The transport decodes messages into [`HubCall`]s and encodes [`Reply`]s; the
//! equivalence harness builds them from its scenarios. [`GameHub::invoke`] runs one.
//!
//! The C# hub's 140 `Request<Event>` methods all forward to `ProcessGameEvent`, except
//! `RequestEstablishPlayers`. They are one variant, [`HubCall::SubmitEvent`].

use dotnet_compat::NetDictionary;
use treachery_engine::date_time_offset::DateTimeOffset;
use treachery_engine::enumerations::{Faction, GameListScope, Ruleset, SubscriptionType, UserStatus};
use treachery_engine::events::GameEvent;
use treachery_engine::game_settings::GameSettings;

use super::{CallContext, GameHub, HubResult};
use crate::model::admin_info::AdminInfo;
use crate::model::chat_message::{GameChatMessage, GlobalChatMessage};
use crate::model::game_init_info::GameInitInfo;
use crate::model::login_info::LoginInfo;
use crate::model::result::{CallResult, VoidResult};
use crate::model::server_info::ServerInfo;
use crate::model::server_status::ServerStatus;

/// String parameters are `Option` where the C# code tests for null or the client may send null.
pub enum HubCall {
    // Authentication
    RequestCreateUser { user_name: String, hashed_password: String, email: Option<String>, player_name: String },
    RequestLogin { version: i32, user_name: String, hashed_password: String },
    RequestPasswordReset { username_or_email: String },
    RequestSetPassword { user_name: String, password_reset_token: String, new_hashed_password: String },
    GetLoginInfo { user_token: String },
    RequestUpdateUserInfo { user_token: String, hashed_password: Option<String>, player_name: String, email: Option<String> },
    RequestSetUserStatus { user_token: String, status: UserStatus },

    // Game management
    RequestCreateGame { name: Option<String>, user_token: String, hashed_password: Option<String>, state_data: Option<String>, skin: Option<String> },
    RequestUpdateSettings { user_token: String, game_id: String, settings: GameSettings },
    RequestCloseGame { user_token: String, game_id: String },
    RequestJoinGame { user_token: String, game_id: String, hashed_password: Option<String>, seat: i32 },
    RequestObserveGame { user_token: String, game_id: String, hashed_password: Option<String> },
    RequestReconnectGame { user_token: String, game_id: String },
    RequestSetOrUnsetHost { user_token: String, game_id: String, user_id: i32 },
    RequestOpenOrCloseSeat { user_token: String, game_id: String, seat: i32 },
    RequestLeaveGame { user_token: String, game_id: String },
    RequestKick { user_token: String, game_id: String, user_id: i32 },
    RequestLoadGame { user_token: String, game_id: String, state_data: String, skin: Option<String> },
    RequestSetSkin { user_token: String, game_id: String, skin: String },
    RequestUndo { user_token: String, game_id: String, until_event_nr: i32 },
    RequestRestoreRecentlyUndone { user_token: String, game_id: String },
    RequestDismissRecentlyUndone { user_token: String, game_id: String },
    RequestGameState { user_token: String, game_id: String },
    RequestSetBotSpeed { user_token: String, game_id: String, speed: i32 },
    RequestHeartbeat { user_token: String, scope: GameListScope },
    RequestAssignSeats { user_token: String, game_id: String, assignment: NetDictionary<i32, i32> },
    RequestScheduleGame {
        user_token: String,
        date_time: DateTimeOffset,
        ruleset: Option<Ruleset>,
        number_of_players: i32,
        maximum_turns: i32,
        allowed_factions_in_play: Vec<Faction>,
        async_play: bool,
    },
    RequestCancelGame { user_token: String, scheduled_game_id: String },
    RequestSubscribeGame { user_token: String, game_id: Option<String>, subscription: SubscriptionType },

    // Game events
    SetTimer { user_token: String, game_id: String, value: i32 },
    RequestPhaseDone { user_token: String, game_id: String },
    RequestPhaseHold { user_token: String, game_id: String },
    RequestPhaseRelease { user_token: String, game_id: String },
    RequestPhaseContinue { user_token: String, game_id: String, event_number: i32 },
    /// Every `Request<Event>` method, `RequestEstablishPlayers` included.
    SubmitEvent { user_token: String, game_id: String, event: GameEvent },

    // Chat
    SendChatMessage { user_token: String, game_id: String, message: GameChatMessage },
    SendGlobalChatMessage { user_token: String, message: GlobalChatMessage },

    // Other
    Connect,
    RequestNudgeBots { user_token: String, game_id: String },

    // Admin
    AdminUpdateMaintenance { user_token: String, maintenance_date: DateTimeOffset },
    AdminPersistState { user_token: String },
    AdminRestoreState { user_token: String },
    AdminCloseGame { user_token: String, game_id: String },
    AdminDownloadGame { user_token: String, game_id: String },
    AdminCancelGame { user_token: String, scheduled_game_id: String },
    DeleteUser { user_token: String, user_id: i32 },
    GetAdminInfo { user_token: String },
}

/// The C# return value of a hub method.
pub enum Reply {
    Void(VoidResult),
    LoginInfo(CallResult<LoginInfo>),
    ServerStatus(CallResult<ServerStatus>),
    GameInitInfo(CallResult<GameInitInfo>),
    ServerInfo(CallResult<ServerInfo>),
    String(CallResult<String>),
    AdminInfo(CallResult<AdminInfo>),
}

impl GameHub {
    /// Runs one hub call to completion.
    pub async fn invoke(&mut self, cx: &CallContext, call: HubCall) -> HubResult<Reply> {
        use HubCall::*;
        Ok(match call {
            RequestCreateUser { user_name, hashed_password, email, player_name } => {
                Reply::LoginInfo(self.request_create_user(&user_name, &hashed_password, email.as_deref(), &player_name).await?)
            }
            RequestLogin { version, user_name, hashed_password } => Reply::LoginInfo(self.request_login(version, &user_name, &hashed_password).await?),
            RequestPasswordReset { username_or_email } => Reply::Void(self.request_password_reset(&username_or_email).await?),
            RequestSetPassword { user_name, password_reset_token, new_hashed_password } => {
                Reply::LoginInfo(self.request_set_password(&user_name, &password_reset_token, &new_hashed_password).await?)
            }
            GetLoginInfo { user_token } => Reply::LoginInfo(self.get_login_info(&user_token).await?),
            RequestUpdateUserInfo { user_token, hashed_password, player_name, email } => Reply::LoginInfo(
                self.request_update_user_info(&user_token, hashed_password.as_deref(), &player_name, email.as_deref()).await?,
            ),
            RequestSetUserStatus { user_token, status } => Reply::ServerStatus(self.request_set_user_status(&user_token, status).await?),

            RequestCreateGame { name, user_token, hashed_password, state_data, skin } => Reply::GameInitInfo(
                self.request_create_game(cx, name.as_deref(), &user_token, hashed_password.as_deref(), state_data.as_deref(), skin.as_deref())
                    .await?,
            ),
            RequestUpdateSettings { user_token, game_id, settings } => Reply::Void(self.request_update_settings(&user_token, &game_id, settings).await?),
            RequestCloseGame { user_token, game_id } => Reply::ServerStatus(self.request_close_game(&user_token, &game_id).await?),
            RequestJoinGame { user_token, game_id, hashed_password, seat } => {
                Reply::GameInitInfo(self.request_join_game(cx, &user_token, &game_id, hashed_password.as_deref(), seat).await?)
            }
            RequestObserveGame { user_token, game_id, hashed_password } => {
                Reply::GameInitInfo(self.request_observe_game(cx, &user_token, &game_id, hashed_password.as_deref()).await?)
            }
            RequestReconnectGame { user_token, game_id } => Reply::GameInitInfo(self.request_reconnect_game(cx, &user_token, &game_id).await?),
            RequestSetOrUnsetHost { user_token, game_id, user_id } => Reply::Void(self.request_set_or_unset_host(&user_token, &game_id, user_id).await?),
            RequestOpenOrCloseSeat { user_token, game_id, seat } => Reply::Void(self.request_open_or_close_seat(&user_token, &game_id, seat).await?),
            RequestLeaveGame { user_token, game_id } => Reply::ServerStatus(self.request_leave_game(&user_token, &game_id).await?),
            RequestKick { user_token, game_id, user_id } => Reply::Void(self.request_kick(&user_token, &game_id, user_id).await?),
            RequestLoadGame { user_token, game_id, state_data, skin } => {
                Reply::Void(self.request_load_game(&user_token, &game_id, &state_data, skin.as_deref()).await?)
            }
            RequestSetSkin { user_token, game_id, skin } => Reply::Void(self.request_set_skin(&user_token, &game_id, &skin).await?),
            RequestUndo { user_token, game_id, until_event_nr } => Reply::Void(self.request_undo(&user_token, &game_id, until_event_nr).await?),
            RequestRestoreRecentlyUndone { user_token, game_id } => Reply::Void(self.request_restore_recently_undone(&user_token, &game_id).await?),
            RequestDismissRecentlyUndone { user_token, game_id } => Reply::Void(self.request_dismiss_recently_undone(&user_token, &game_id).await?),
            RequestGameState { user_token, game_id } => Reply::GameInitInfo(self.request_game_state(&user_token, &game_id).await?),
            RequestSetBotSpeed { user_token, game_id, speed } => Reply::Void(self.request_set_bot_speed(&user_token, &game_id, speed).await?),
            RequestHeartbeat { user_token, scope } => Reply::ServerStatus(self.request_heartbeat(&user_token, scope).await?),
            RequestAssignSeats { user_token, game_id, assignment } => Reply::Void(self.request_assign_seats(&user_token, &game_id, assignment).await?),
            RequestScheduleGame { user_token, date_time, ruleset, number_of_players, maximum_turns, allowed_factions_in_play, async_play } => {
                Reply::ServerStatus(
                    self.request_schedule_game(&user_token, date_time, ruleset, number_of_players, maximum_turns, allowed_factions_in_play, async_play)
                        .await?,
                )
            }
            RequestCancelGame { user_token, scheduled_game_id } => Reply::ServerStatus(self.request_cancel_game(&user_token, &scheduled_game_id).await?),
            RequestSubscribeGame { user_token, game_id, subscription } => {
                Reply::ServerStatus(self.request_subscribe_game(&user_token, game_id.as_deref(), subscription).await?)
            }

            SetTimer { user_token, game_id, value } => Reply::Void(self.set_timer(&user_token, &game_id, value).await?),
            RequestPhaseDone { user_token, game_id } => Reply::Void(self.request_phase_done(&user_token, &game_id).await?),
            RequestPhaseHold { user_token, game_id } => Reply::Void(self.request_phase_hold(&user_token, &game_id).await?),
            RequestPhaseRelease { user_token, game_id } => Reply::Void(self.request_phase_release(&user_token, &game_id).await?),
            RequestPhaseContinue { user_token, game_id, event_number } => {
                Reply::Void(self.request_phase_continue(&user_token, &game_id, event_number).await?)
            }
            SubmitEvent { user_token, game_id, event } => Reply::Void(self.submit_event(&user_token, &game_id, event).await?),

            SendChatMessage { user_token, game_id, message } => Reply::Void(self.send_chat_message(&user_token, &game_id, message).await?),
            SendGlobalChatMessage { user_token, message } => Reply::Void(self.send_global_chat_message(&user_token, message).await?),

            Connect => Reply::ServerInfo(self.connect().await?),
            RequestNudgeBots { user_token, game_id } => Reply::Void(self.request_nudge_bots(&user_token, &game_id).await?),

            AdminUpdateMaintenance { user_token, maintenance_date } => Reply::String(self.admin_update_maintenance(&user_token, maintenance_date).await?),
            AdminPersistState { user_token } => Reply::String(self.admin_persist_state(&user_token).await?),
            AdminRestoreState { user_token } => Reply::String(self.admin_restore_state(&user_token).await?),
            AdminCloseGame { user_token, game_id } => Reply::String(self.admin_close_game(&user_token, &game_id).await?),
            AdminDownloadGame { user_token, game_id } => Reply::String(self.admin_download_game(&user_token, &game_id).await?),
            AdminCancelGame { user_token, scheduled_game_id } => Reply::String(self.admin_cancel_game(&user_token, &scheduled_game_id).await?),
            DeleteUser { user_token, user_id } => Reply::String(self.delete_user(&user_token, user_id).await?),
            GetAdminInfo { user_token } => Reply::AdminInfo(self.get_admin_info(&user_token).await?),
        })
    }
}
