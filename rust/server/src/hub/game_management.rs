//! Port of `Server/GameHub_GameManagement.cs`. Package H2.
//!
//! Notes on the port:
//!
//! - C# holds `ManagedGame` objects by reference; here they are [`ManagedGameRef`]s. No
//!   `RefCell` borrow of a game is held across a call to another hub method or across a
//!   callback, because callbacks serialize the game's `Participation` right away.
//! - `GameInfo.SeatedPlayers` references the live seat dictionary (server-plan.md, "Hazards"):
//!   see [`GameHub::extract_game_info`] and [`SeatedPlayersRef`].

use dotnet_compat::NetDictionary;
use treachery_engine::cs::{NullRef, Shared};
use treachery_engine::date_time_offset::DateTimeOffset;
use treachery_engine::enumerations::{ErrorType, Faction, GameListScope, Phase, Ruleset, SubscriptionType};
use treachery_engine::game::Game;
use treachery_engine::game_settings::GameSettings;
use treachery_engine::game_state::GameState;
use treachery_engine::participation::Participation;

use super::{CallContext, GameHub, HubResult, ManagedGameRef, ACTIVE_GAME_THRESHOLD_MINUTES, MAXIMUM_NUMBER_OF_GAMES_PER_PLAYER, SERVER_STATUS_FREQUENCY_MS};
use crate::clients::Callback;
use crate::mail::{Attachment, MailAddress, MailMessage, MailPriority};
use crate::model::available_seat_info::AvailableSeatInfo;
use crate::model::game_info::{GameInfo, SeatedPlayersRef};
use crate::model::game_init_info::GameInitInfo;
use crate::model::phase_control_info::PhaseControlInfo;
use crate::model::logged_in_user_info::LoggedInUserInfo;
use crate::model::managed_game::ManagedGame;
use crate::model::result::{CallResult, VoidResult};
use crate::model::scheduled_game::ScheduledGame;
use crate::model::scheduled_game_info::ScheduledGameInfo;
use crate::model::server_status::ServerStatus;
use crate::net_time::DateTimeOffsetExt;
use crate::render;

/// `string.IsNullOrEmpty(s)`.
fn is_null_or_empty(s: Option<&str>) -> bool {
    s.is_none_or(str::is_empty)
}

/// The `(Id, PlayerName)` of the user object `AreValid` returned.
fn id_and_player_name(user: &crate::logged_in_user::UserRef) -> (i32, String) {
    let u = user.borrow();
    (u.id, u.player_name.clone())
}

impl GameHub {
    /// The `new GameInitInfo { ... }` of the game management methods: the state, the game's own
    /// `Participation` object, and the name unless `with_name` is false (`RequestReconnectGame`).
    fn game_init_info(game_id: &str, game: &ManagedGameRef, with_name: bool, phase_control: Option<PhaseControlInfo>) -> GameInitInfo {
        let mg = game.borrow();
        GameInitInfo {
            game_id: Some(game_id.to_string()),
            game_state: Some(GameState::get_state_as_string(&mg.game)),
            game_name: if with_name { mg.name.clone() } else { None },
            participation: Some(mg.game.participation.clone()),
            phase_control,
        }
    }

    /// C#: `public async Task<Result<GameInitInfo>> RequestCreateGame(string name, string userToken, string hashedPassword, string stateData, string skin)` (GameHub_GameManagement.cs:7)
    pub async fn request_create_game(
        &mut self,
        cx: &CallContext,
        name: Option<&str>,
        user_token: &str,
        hashed_password: Option<&str>,
        state_data: Option<&str>,
        skin: Option<&str>,
    ) -> HubResult<CallResult<GameInitInfo>> {
        let Some(user) = self.users_by_user_token.get(user_token) else {
            return Ok(Self::error(ErrorType::UserNotFound, ""));
        };
        let (user_id, player_name) = id_and_player_name(&user.user);

        if self.running_games_by_game_id.values().filter(|g| g.borrow().creator_user_id == user_id).count() as i32 >= MAXIMUM_NUMBER_OF_GAMES_PER_PLAYER {
            return Ok(Self::error(ErrorType::TooManyGames, ""));
        }

        let mut game = Game::new();
        let load_game = !is_null_or_empty(state_data);
        if load_game {
            // C#: GameState.Load throws on bad data; the exception escapes the call.
            let state = GameState::load(state_data.unwrap_or_default());
            let mut loaded_game = None;
            let error_message = Game::try_load(&state, &Shared::new(Participation::new()), false, false, &mut loaded_game);

            match (error_message, loaded_game) {
                (None, Some(loaded_game)) => game = loaded_game,
                (error_message, _) => {
                    let details = match &error_message {
                        Some(m) => render::message_to_string_with_default_describer(m),
                        None => "Unknown error".to_string(),
                    };
                    return Ok(Self::error(ErrorType::InvalidGameEvent, &details));
                }
            }
        }

        let game_id = self.services.ids.new_guid();
        let managed_game = ManagedGame {
            creation_date: self.now(),
            last_activity: self.now(),
            last_persisted: DateTimeOffset::default(),
            creator_user_id: user_id,
            game_id: game_id.clone(),
            game,
            name: Some(match name {
                Some(name) if !name.is_empty() => name.to_string(),
                _ => format!("{player_name}'s Game"),
            }),
            hashed_password: hashed_password.map(str::to_string),
            observers_require_password: false,
            statistics_sent: false,
            last_async_play_message_sent: DateTimeOffset::default(),
            seated_players_ref: None,
            bots: Vec::new(),
        };
        let managed_game: ManagedGameRef = std::rc::Rc::new(std::cell::RefCell::new(managed_game));

        self.running_games_by_game_id.insert(game_id.clone(), managed_game.clone());

        managed_game.borrow_mut().game.add_player(user_id, &player_name, -1);
        managed_game.borrow_mut().game.set_or_unset_host(user_id);
        self.add_to_group(cx, &game_id, user_id);

        if let Some(skin) = skin.filter(|s| !s.is_empty()) {
            self.clients.group(&game_id, Callback::HandleSetSkin(skin.to_string()));
        }

        self.persist_game_if_needed(&managed_game).await?;

        Ok(Self::success_with(Self::game_init_info(&game_id, &managed_game, true, None)))
    }

    /// C#: `public async Task<Result<ServerStatus>> RequestScheduleGame(string userToken, DateTimeOffset dateTime, Ruleset? ruleset, int numberOfPlayers, int maximumTurns, List<Faction> allowedFactionsInPlay, bool asyncPlay)` (GameHub_GameManagement.cs:61)
    #[allow(clippy::too_many_arguments)]
    pub async fn request_schedule_game(
        &mut self,
        user_token: &str,
        date_time: DateTimeOffset,
        ruleset: Option<Ruleset>,
        number_of_players: i32,
        maximum_turns: i32,
        allowed_factions_in_play: Vec<Faction>,
        async_play: bool,
    ) -> HubResult<CallResult<ServerStatus>> {
        let Some(user) = self.users_by_user_token.get(user_token) else {
            return Ok(Self::error(ErrorType::UserNotFound, ""));
        };
        let (user_id, player_name) = id_and_player_name(&user.user);

        if self.scheduled_games_by_game_id.values().filter(|g| g.creator_user_id == user_id).count() >= 3 {
            return Ok(Self::error(ErrorType::TooManyScheduledGames, ""));
        }

        let game_id = self.services.ids.new_guid();
        let mut subscribed_users = NetDictionary::new();
        subscribed_users.insert(user_id, SubscriptionType::CertainAsHost);
        let scheduled_game = ScheduledGame {
            scheduled_game_id: game_id.clone(),
            date_time,
            creator_user_id: user_id,
            creator_player_name: player_name,
            ruleset,
            number_of_players,
            maximum_turns,
            allowed_factions_in_play,
            async_play,
            subscribed_users,
        };

        self.scheduled_games_by_game_id.insert(game_id, scheduled_game);

        self.update_server_status_if_needed(true);

        Ok(Self::success_with(self.filtered_server_status(GameListScope::Active, user_id)))
    }

    /// C#: `public async Task<Result<ServerStatus>> RequestCancelGame(string userToken, string scheduledGameId)` (GameHub_GameManagement.cs:93)
    pub async fn request_cancel_game(&mut self, user_token: &str, scheduled_game_id: &str) -> HubResult<CallResult<ServerStatus>> {
        let Some(user) = self.users_by_user_token.get(user_token) else {
            return Ok(Self::error(ErrorType::UserNotFound, ""));
        };
        let user_id = user.id();

        let Some(scheduled_game) = self.scheduled_games_by_game_id.get(scheduled_game_id) else {
            return Ok(Self::error(ErrorType::ScheduledGameNotFound, ""));
        };

        if user_id != scheduled_game.creator_user_id {
            return Ok(Self::error(ErrorType::NoHost, ""));
        }

        self.scheduled_games_by_game_id.remove(scheduled_game_id);

        self.update_server_status_if_needed(true);

        Ok(Self::success_with(self.filtered_server_status(GameListScope::Active, user_id)))
    }

    /// C#: `public async Task<VoidResult> RequestLoadGame(string userToken, string gameId, string stateData, string skin)` (GameHub_GameManagement.cs:112)
    pub async fn request_load_game(&mut self, user_token: &str, game_id: &str, state_data: &str, skin: Option<&str>) -> HubResult<VoidResult> {
        let (user, game) = match self.are_valid(Some(user_token), Some(game_id)) {
            Ok(x) => x,
            Err(error) => return Ok(error),
        };
        let user_id = user.borrow().id;

        if !game.borrow().game.is_host(Some(user_id)) {
            return Ok(Self::error(ErrorType::NoHost, ""));
        }

        let state = GameState::load(state_data);
        // The game's own Participation object: the loaded game shares it.
        let participation = game.borrow().game.participation.clone();
        let mut loaded_game = None;
        let error_message = Game::try_load(&state, &participation, false, false, &mut loaded_game);

        let mut loaded_game = match (error_message, loaded_game) {
            (None, Some(loaded_game)) => loaded_game,
            (error_message, _) => {
                let details = match &error_message {
                    Some(m) => render::message_to_string_with_default_describer(m),
                    None => "Unknown error".to_string(),
                };
                return Ok(Self::error(ErrorType::InvalidGameEvent, &details));
            }
        };

        if loaded_game.seed != game.borrow().game.seed {
            loaded_game.reset_seats();
        }

        game.borrow_mut().game = loaded_game;
        let info = GameInitInfo {
            game_id: Some(game_id.to_string()),
            game_state: Some(state_data.to_string()),
            game_name: game.borrow().name.clone(),
            participation: Some(game.borrow().game.participation.clone()),
            phase_control: None,
        };
        self.clients.group(game_id, Callback::HandleLoadGame(info));
        self.update_pause(&game, None, None, true).await?;

        if let Some(skin) = skin.filter(|s| !s.is_empty()) {
            self.clients.group(game_id, Callback::HandleSetSkin(skin.to_string()));
        }

        self.persist_game_if_needed(&game).await?;

        self.schedule_bot_event(&game, false).await?;
        Ok(Self::success())
    }

    /// C#: `public async Task<VoidResult> RequestAssignSeats(string userToken, string gameId, Dictionary<int, int> assignment)` (GameHub_GameManagement.cs:147)
    pub async fn request_assign_seats(&mut self, user_token: &str, game_id: &str, assignment: NetDictionary<i32, i32>) -> HubResult<VoidResult> {
        let (user, game) = match self.are_valid(Some(user_token), Some(game_id)) {
            Ok(x) => x,
            Err(error) => return Ok(error),
        };
        let user_id = user.borrow().id;

        if !game.borrow().game.is_host(Some(user_id)) {
            return Ok(Self::error(ErrorType::NoHost, ""));
        }

        // `Participation.SeatedPlayers = assignment` replaces the dictionary object: lobby
        // entries that reference the old one keep its contents from now on.
        if let Some(old) = game.borrow_mut().seated_players_ref.take() {
            old.detach();
        }
        game.borrow().game.participation.borrow_mut().seated_players = assignment.clone();
        self.clients.group(game_id, Callback::HandleAssignSeats(assignment));

        self.persist_game_if_needed(&game).await?;

        self.schedule_bot_event(&game, false).await?;
        Ok(Self::success())
    }

    /// C#: `public async Task<Result<GameInitInfo>> RequestJoinGame(string userToken, string gameId, string hashedPassword, int seat)` (GameHub_GameManagement.cs:164)
    ///
    /// C# parity: a user who is already a player is added to the group again without the
    /// password and seat checks. Joining an occupied open seat removes its user from the game
    /// (but not from the group).
    pub async fn request_join_game(&mut self, cx: &CallContext, user_token: &str, game_id: &str, hashed_password: Option<&str>, seat: i32) -> HubResult<CallResult<GameInitInfo>> {
        let (user, game) = match self.are_valid(Some(user_token), Some(game_id)) {
            Ok(x) => x,
            Err(error) => return Ok(error),
        };
        let (user_id, player_name) = id_and_player_name(&user);

        if !game.borrow().game.is_player(user_id) {
            {
                let mg = game.borrow();
                if let Some(game_password) = mg.hashed_password.as_deref() {
                    if !game_password.is_empty() && Some(game_password) != hashed_password {
                        return Ok(Self::error(ErrorType::IncorrectGamePassword, ""));
                    }
                }

                if !mg.game.is_open(seat) {
                    return Ok(Self::error(ErrorType::SeatNotAvailableNotOpen, ""));
                }

                if mg.game.was_kicked(user_id) {
                    return Ok(Self::error(ErrorType::SeatNotAvailableKicked, ""));
                }
            }

            if seat >= 0 {
                let current_user_id = game.borrow().game.user_id_in_seat(seat);
                if current_user_id >= 0 {
                    game.borrow_mut().game.remove_user(current_user_id, false);
                    self.clients.group(game_id, Callback::HandleRemoveUser { user_id: current_user_id, kick: false });
                }
            }

            game.borrow_mut().game.add_player(user_id, &player_name, seat);
            self.clients.group(game_id, Callback::HandleJoinGame { user_id, user_name: player_name.clone(), seat });

            let make_host = {
                let mg = game.borrow();
                mg.game.number_of_seated_players() == 1 || mg.creator_user_id == user_id
            };
            if make_host {
                game.borrow_mut().game.set_or_unset_host(user_id);
                self.clients.group(game_id, Callback::HandleSetOrUnsetHost(user_id));
            }
        }

        self.add_to_group(cx, game_id, user_id);

        self.persist_game_if_needed(&game).await?;

        if game.borrow().game.number_of_seated_players() == 1 {
            self.schedule_bot_event(&game, false).await?;
        }

        Ok(Self::success_with(Self::game_init_info(game_id, &game, true, Some(self.current_phase_control(game_id)))))
    }

    /// C#: `public async Task<Result<ServerStatus>> RequestSubscribeGame(string userToken, string? gameId, SubscriptionType subscription)` (GameHub_GameManagement.cs:216)
    pub async fn request_subscribe_game(&mut self, user_token: &str, game_id: Option<&str>, subscription: SubscriptionType) -> HubResult<CallResult<ServerStatus>> {
        let Some(game_id) = game_id else {
            return Ok(Self::error(ErrorType::GameNotFound, ""));
        };

        let Some(user) = self.users_by_user_token.get(user_token) else {
            return Ok(Self::error(ErrorType::UserNotFound, ""));
        };
        let user_id = user.id();

        let Some(game) = self.scheduled_games_by_game_id.get_mut(game_id) else {
            return Ok(Self::error(ErrorType::GameNotFound, ""));
        };

        game.subscribed_users.insert(user_id, subscription);

        self.update_server_status_if_needed(true);

        Ok(Self::success_with(self.filtered_server_status(GameListScope::Active, user_id)))
    }

    /// C#: `public async Task<VoidResult> RequestOpenOrCloseSeat(string userToken, string gameId, int seat)` (GameHub_GameManagement.cs:240)
    pub async fn request_open_or_close_seat(&mut self, user_token: &str, game_id: &str, seat: i32) -> HubResult<VoidResult> {
        let (user, game) = match self.are_valid(Some(user_token), Some(game_id)) {
            Ok(x) => x,
            Err(error) => return Ok(error),
        };
        let user_id = user.borrow().id;

        if !game.borrow().game.is_host(Some(user_id)) {
            return Ok(Self::error(ErrorType::NoHost, ""));
        }

        game.borrow_mut().game.open_or_close_seat(seat);
        self.clients.group(game_id, Callback::HandleOpenOrCloseSeats(vec![seat]));

        self.persist_game_if_needed(&game).await?;

        Ok(Self::success())
    }

    /// C#: `public async Task<VoidResult> RequestSetOrUnsetHost(string userToken, string gameId, int userId)` (GameHub_GameManagement.cs:256)
    pub async fn request_set_or_unset_host(&mut self, user_token: &str, game_id: &str, user_id: i32) -> HubResult<VoidResult> {
        let (host, game) = match self.are_valid(Some(user_token), Some(game_id)) {
            Ok(x) => x,
            Err(error) => return Ok(error),
        };
        let host_id = host.borrow().id;

        if !game.borrow().game.is_host(Some(host_id)) {
            return Ok(Self::error(ErrorType::NoHost, ""));
        }

        if user_id == host_id && game.borrow().game.number_of_hosts() <= 1 {
            return Ok(Self::error(ErrorType::CannotRemoveLastHost, ""));
        }

        game.borrow_mut().game.set_or_unset_host(user_id);
        self.clients.group(game_id, Callback::HandleSetOrUnsetHost(user_id));

        self.persist_game_if_needed(&game).await?;

        Ok(Self::success())
    }

    /// C#: `public async Task<Result<ServerStatus>> RequestLeaveGame(string userToken, string gameId)` (GameHub_GameManagement.cs:275)
    ///
    /// C# parity: when the last host leaves, every other *seated* player becomes host (a
    /// player who already was a host is unset instead, but then the leaver was not the only
    /// host). Observers are never promoted.
    pub async fn request_leave_game(&mut self, user_token: &str, game_id: &str) -> HubResult<CallResult<ServerStatus>> {
        let (user, game) = match self.are_valid(Some(user_token), Some(game_id)) {
            Ok(x) => x,
            Err(error) => return Ok(error),
        };
        let user_id = user.borrow().id;

        let last_host = {
            let mg = game.borrow();
            mg.game.is_host(Some(user_id)) && mg.game.number_of_hosts() == 1
        };
        if last_host {
            // C# enumerates SeatedPlayers.Keys lazily; SetOrUnsetHost only changes Hosts.
            let ids: Vec<i32> = game.borrow().game.participation.borrow().seated_players.keys().copied().filter(|id| *id != user_id).collect();
            for id in ids {
                game.borrow_mut().game.set_or_unset_host(id);
                self.clients.group(game_id, Callback::HandleSetOrUnsetHost(id));
            }
        }

        game.borrow_mut().game.remove_user(user_id, false);
        self.clients.group(game_id, Callback::HandleRemoveUser { user_id, kick: false });
        self.remove_from_group(game_id, user_id);

        self.persist_game_if_needed(&game).await?;

        self.schedule_bot_event(&game, false).await?;
        self.update_server_status_if_needed(true);

        Ok(Self::success_with(self.filtered_server_status(GameListScope::Active, user_id)))
    }

    /// C#: `public async Task<VoidResult> RequestKick(string userToken, string gameId, int userId)` (GameHub_GameManagement.cs:300)
    pub async fn request_kick(&mut self, user_token: &str, game_id: &str, user_id: i32) -> HubResult<VoidResult> {
        let (user, game) = match self.are_valid(Some(user_token), Some(game_id)) {
            Ok(x) => x,
            Err(error) => return Ok(error),
        };
        let host_id = user.borrow().id;

        if !game.borrow().game.is_host(Some(host_id)) {
            return Ok(Self::error(ErrorType::NoHost, ""));
        }

        game.borrow_mut().game.remove_user(user_id, true);
        self.clients.group(game_id, Callback::HandleRemoveUser { user_id, kick: true });
        self.remove_from_group(game_id, user_id);

        self.persist_game_if_needed(&game).await?;

        self.schedule_bot_event(&game, false).await?;
        Ok(Self::success())
    }

    /// C#: `public async Task<VoidResult> RequestUpdateSettings(string userToken, string gameId, GameSettings settings)` (GameHub_GameManagement.cs:318)
    pub async fn request_update_settings(&mut self, user_token: &str, game_id: &str, settings: GameSettings) -> HubResult<VoidResult> {
        let (user, game) = match self.are_valid(Some(user_token), Some(game_id)) {
            Ok(x) => x,
            Err(error) => return Ok(error),
        };
        let user_id = user.borrow().id;

        if game.borrow().creator_user_id != user_id {
            return Ok(Self::error(ErrorType::NoCreator, ""));
        }

        game.borrow_mut().game.settings = Some(Shared::new(settings.clone()));

        self.clients.group(game_id, Callback::HandleUpdateSettings(settings));

        self.persist_game_if_needed(&game).await?;

        Ok(Self::success())
    }

    /// C#: `public async Task<Result<ServerStatus>> RequestCloseGame(string userToken, string gameId)` (GameHub_GameManagement.cs:335)
    pub async fn request_close_game(&mut self, user_token: &str, game_id: &str) -> HubResult<CallResult<ServerStatus>> {
        let (user, game) = match self.are_valid(Some(user_token), Some(game_id)) {
            Ok(x) => x,
            Err(error) => return Ok(error),
        };
        let user_id = user.borrow().id;

        if game.borrow().creator_user_id != user_id {
            return Ok(Self::error(ErrorType::NoCreator, ""));
        }

        // C# enumerates PlayerNames.Keys while RemoveUser removes the current key, which .NET
        // allows and which visits every key once, in order. A copy of the keys does the same.
        let player_ids: Vec<i32> = game.borrow().game.participation.borrow().player_names.keys().copied().collect();
        for player_id in player_ids {
            game.borrow_mut().game.remove_user(player_id, true);
            self.clients.group(game_id, Callback::HandleRemoveUser { user_id: player_id, kick: true });
            self.remove_from_group(game_id, player_id);
        }

        self.running_games_by_game_id.remove(game_id);
        self.pauses_by_game_id.remove(game_id);

        self.erase_game(&game).await?;

        self.update_server_status_if_needed(true);
        Ok(Self::success_with(self.filtered_server_status(GameListScope::Active, user_id)))
    }

    /// C#: `private async Task AddToGroup(string gameId, int userId, string connectionId)` (GameHub_GameManagement.cs:358)
    ///
    /// C# parity: the parameter `connectionId` is used for the group, but `Context.ConnectionId`
    /// is what gets recorded in `UserConnections` (line 367). Both are the caller's connection
    /// at every call site.
    pub fn add_to_group(&mut self, cx: &CallContext, game_id: &str, user_id: i32) {
        let connection_id = cx.connection_id.expect("AddToGroup is only called from a client call, which has a connection");
        self.clients.add_to_group(connection_id, game_id, user_id);

        // `new ConnectionInfo()` reads DateTimeOffset.Now.
        let now = self.now();
        self.connection_info_by_user_id.entry(user_id).or_default().set_connection_id(game_id, connection_id, now);
    }

    /// C#: `private async Task RemoveFromGroup(string gameId, int userId)` (GameHub_GameManagement.cs:370)
    ///
    /// Removes only the connection recorded last for the user and game; another connection of
    /// the same user stays in the group. The user's `UserConnections` entry stays, even empty.
    pub fn remove_from_group(&mut self, game_id: &str, user_id: i32) {
        if let Some(connection_info) = self.connection_info_by_user_id.get_mut(&user_id) {
            if let Some(connection_id) = connection_info.try_get_connection_id(game_id) {
                connection_info.remove_connection_id(game_id);
                self.clients.remove_from_group(connection_id, game_id);
            }
        }
    }

    /// C#: `public async Task<Result<GameInitInfo>> RequestObserveGame(string userToken, string gameId, string hashedPassword)` (GameHub_GameManagement.cs:380)
    pub async fn request_observe_game(&mut self, cx: &CallContext, user_token: &str, game_id: &str, hashed_password: Option<&str>) -> HubResult<CallResult<GameInitInfo>> {
        let (user, game) = match self.are_valid(Some(user_token), Some(game_id)) {
            Ok(x) => x,
            Err(error) => return Ok(error),
        };
        let (user_id, player_name) = id_and_player_name(&user);

        if !game.borrow().game.is_observer(Some(user_id)) {
            {
                let mg = game.borrow();
                if mg.observers_require_password {
                    if let Some(game_password) = mg.hashed_password.as_deref() {
                        if !game_password.is_empty() && Some(game_password) != hashed_password {
                            return Ok(Self::error(ErrorType::IncorrectGamePassword, ""));
                        }
                    }
                }

                if mg.game.is_player(user_id) {
                    return Ok(Self::error(ErrorType::AlreadyPlayer, ""));
                }
            }

            game.borrow_mut().game.add_observer(user_id, &player_name);
            self.clients.group(game_id, Callback::HandleObserveGame { user_id, user_name: player_name.clone() });
        }

        self.add_to_group(cx, game_id, user_id);

        self.persist_game_if_needed(&game).await?;

        self.schedule_bot_event(&game, false).await?;

        Ok(Self::success_with(Self::game_init_info(game_id, &game, true, Some(self.current_phase_control(game_id)))))
    }

    /// C#: `public async Task<Result<GameInitInfo>> RequestReconnectGame(string userToken, string gameId)` (GameHub_GameManagement.cs:413)
    ///
    /// C# parity: the result has no `GameName`.
    pub async fn request_reconnect_game(&mut self, cx: &CallContext, user_token: &str, game_id: &str) -> HubResult<CallResult<GameInitInfo>> {
        let (user, game) = match self.are_valid(Some(user_token), Some(game_id)) {
            Ok(x) => x,
            Err(error) => return Ok(error),
        };
        let user_id = user.borrow().id;

        if !game.borrow().game.is_participant(user_id) {
            return Ok(Self::error(ErrorType::UserNotInGame, ""));
        }

        self.add_to_group(cx, game_id, user_id);

        self.persist_game_if_needed(&game).await?;

        Ok(Self::success_with(Self::game_init_info(game_id, &game, false, Some(self.current_phase_control(game_id)))))
    }

    /// C#: `public async Task<Result<GameInitInfo>> RequestGameState(string userToken, string gameId)` (GameHub_GameManagement.cs:433)
    pub async fn request_game_state(&mut self, user_token: &str, game_id: &str) -> HubResult<CallResult<GameInitInfo>> {
        let (user, game) = match self.are_valid(Some(user_token), Some(game_id)) {
            Ok(x) => x,
            Err(error) => return Ok(error),
        };
        let user_id = user.borrow().id;

        if !game.borrow().game.is_participant(user_id) {
            return Ok(Self::error(ErrorType::UserNotInGame, ""));
        }

        Ok(Self::success_with(Self::game_init_info(game_id, &game, true, Some(self.current_phase_control(game_id)))))
    }

    /// C#: `public async Task<VoidResult> RequestSetSkin(string userToken, string gameId, string skin)` (GameHub_GameManagement.cs:450)
    pub async fn request_set_skin(&mut self, user_token: &str, game_id: &str, skin: &str) -> HubResult<VoidResult> {
        let (user, game) = match self.are_valid(Some(user_token), Some(game_id)) {
            Ok(x) => x,
            Err(error) => return Ok(error),
        };
        let user_id = user.borrow().id;

        if !game.borrow().game.is_host(Some(user_id)) {
            return Ok(Self::error(ErrorType::NoHost, ""));
        }

        self.clients.group(game_id, Callback::HandleSetSkin(skin.to_string()));

        Ok(Self::success())
    }

    /// C#: `public async Task<VoidResult> RequestUndo(string userToken, string gameId, int untilEventNr)` (GameHub_GameManagement.cs:463)
    ///
    /// An exception in `Undo` (a negative `untilEventNr`) escapes the call and leaves the old
    /// game in place, as the C# assignment never happens.
    pub async fn request_undo(&mut self, user_token: &str, game_id: &str, until_event_nr: i32) -> HubResult<VoidResult> {
        let (user, game) = match self.are_valid(Some(user_token), Some(game_id)) {
            Ok(x) => x,
            Err(error) => return Ok(error),
        };
        let user_id = user.borrow().id;

        if !game.borrow().game.is_host(Some(user_id)) {
            return Ok(Self::error(ErrorType::NoHost, ""));
        }

        game.borrow_mut().game.undo(until_event_nr);
        self.clients.group(game_id, Callback::HandleUndo(until_event_nr));
        self.update_pause(&game, None, None, true).await?;

        game.borrow_mut().last_activity = self.now();
        self.persist_game_if_needed(&game).await?;
        self.schedule_bot_event(&game, false).await?;
        Ok(Self::success())
    }

    /// C#: `public async Task<VoidResult> RequestRestoreRecentlyUndone(string userToken, string gameId)` (GameHub_GameManagement.cs:480)
    pub async fn request_restore_recently_undone(&mut self, user_token: &str, game_id: &str) -> HubResult<VoidResult> {
        let (user, game) = match self.are_valid(Some(user_token), Some(game_id)) {
            Ok(x) => x,
            Err(error) => return Ok(error),
        };
        let user_id = user.borrow().id;

        if !game.borrow().game.is_host(Some(user_id)) {
            return Ok(Self::error(ErrorType::NoHost, ""));
        }

        game.borrow_mut().game.restore_recently_undone();
        self.clients.group(game_id, Callback::HandleRestoreRecentlyUndone);
        self.update_pause(&game, None, None, true).await?;

        game.borrow_mut().last_activity = self.now();
        self.persist_game_if_needed(&game).await?;
        self.schedule_bot_event(&game, false).await?;
        Ok(Self::success())
    }

    /// C#: `public async Task<VoidResult> RequestDismissRecentlyUndone(string userToken, string gameId)` (GameHub_GameManagement.cs:497)
    pub async fn request_dismiss_recently_undone(&mut self, user_token: &str, game_id: &str) -> HubResult<VoidResult> {
        let (user, game) = match self.are_valid(Some(user_token), Some(game_id)) {
            Ok(x) => x,
            Err(error) => return Ok(error),
        };
        let user_id = user.borrow().id;

        if !game.borrow().game.is_host(Some(user_id)) {
            return Ok(Self::error(ErrorType::NoHost, ""));
        }

        game.borrow_mut().game.dismiss_recently_undone();
        self.clients.group(game_id, Callback::HandleDismissRecentlyUndone);
        self.update_pause(&game, None, None, true).await?;

        game.borrow_mut().last_activity = self.now();
        self.persist_game_if_needed(&game).await?;
        self.schedule_bot_event(&game, false).await?;
        Ok(Self::success())
    }

    /// C#: `public async Task<VoidResult> RequestSetBotSpeed(string userToken, string gameId, int speed)` (GameHub_GameManagement.cs:514)
    ///
    /// C# parity: `HandleBotSpeed` goes to every connection, not the game's group.
    pub async fn request_set_bot_speed(&mut self, user_token: &str, game_id: &str, speed: i32) -> HubResult<VoidResult> {
        let (user, game) = match self.are_valid(Some(user_token), Some(game_id)) {
            Ok(x) => x,
            Err(error) => return Ok(error),
        };
        let user_id = user.borrow().id;

        if !game.borrow().game.is_host(Some(user_id)) {
            return Ok(Self::error(ErrorType::NoHost, ""));
        }

        let bots_speed = {
            let mg = game.borrow();
            let mut participation = mg.game.participation.borrow_mut();
            participation.bots_speed = speed;
            participation.bots_speed
        };
        self.clients.all(Callback::HandleBotSpeed(bots_speed));

        self.persist_game_if_needed(&game).await?;

        self.schedule_bot_event(&game, false).await?;
        Ok(Self::success())
    }

    /// C#: `public async Task<Result<ServerStatus>> RequestHeartbeat(string userToken, GameListScope scope)` (GameHub_GameManagement.cs:531)
    ///
    /// C# keeps the `LoggedInUser` it looked up. If `RunTasksIfNeeded` removes its token
    /// (token cleanup), C# sets `LastSeenDateTime` on the removed object, which nothing reads
    /// again, and still answers with the status for its id. Here the id is kept and the time is
    /// set only on an entry that is still there.
    pub async fn request_heartbeat(&mut self, user_token: &str, scope: GameListScope) -> HubResult<CallResult<ServerStatus>> {
        self.restore_games_if_server_just_started().await?;

        let Some(logged_in_user) = self.users_by_user_token.get(user_token) else {
            return Ok(Self::error(ErrorType::UserNotFound, ""));
        };
        let user_id = logged_in_user.id();
        let user_object = logged_in_user.user.clone();

        self.run_tasks_if_needed().await?;

        let now = self.now();
        if let Some(logged_in_user) = self.users_by_user_token.get_mut(user_token) {
            // The same LoggedInUser object only if it still holds the same User (cleanup and
            // restore replace the entry).
            if std::rc::Rc::ptr_eq(&logged_in_user.user, &user_object) {
                logged_in_user.last_seen_date_time = now;
            }
        }

        Ok(Self::success_with(self.filtered_server_status(scope, user_id)))
    }

    /// C#: `private static ServerStatus FilteredServerStatus(GameListScope scope, int userId)` (GameHub_GameManagement.cs:545)
    ///
    /// The lists share the cached `RunningGames`' entries, whose `SeatedPlayers` are live.
    pub fn filtered_server_status(&self, scope: GameListScope, user_id: i32) -> ServerStatus {
        if scope == GameListScope::Active {
            let last_active_game_threshold = self.now().add_minutes(-f64::from(ACTIVE_GAME_THRESHOLD_MINUTES));
            ServerStatus {
                // `mg.LastActivity >= threshold` on a `DateTimeOffset?`: false when null.
                running_games: self
                    .running_games
                    .iter()
                    .filter(|mg| mg.last_activity.is_some_and(|a| a >= last_active_game_threshold) || mg.you_are_in(user_id))
                    .cloned()
                    .collect(),
                own_games: self.running_games.iter().filter(|mg| mg.creator_id == user_id).cloned().collect(),
                scheduled_games: self.scheduled_games.clone(),
                recently_seen_users: self.recently_seen_users.clone(),
            }
        } else if scope == GameListScope::All {
            ServerStatus {
                running_games: self.running_games.clone(),
                own_games: self.running_games.iter().filter(|mg| mg.creator_id == user_id).cloned().collect(),
                scheduled_games: self.scheduled_games.clone(),
                recently_seen_users: self.recently_seen_users.clone(),
            }
        } else {
            // GameListScope.None and any other value.
            ServerStatus {
                running_games: Vec::new(),
                own_games: Vec::new(),
                scheduled_games: Vec::new(),
                recently_seen_users: self.recently_seen_users.clone(),
            }
        }
    }

    /// C#: `private async Task RunTasksIfNeeded()` (GameHub_GameManagement.cs:590)
    pub async fn run_tasks_if_needed(&mut self) -> HubResult<()> {
        if self.restoring {
            return Ok(());
        }

        self.restore_games_if_server_just_started().await?;
        self.cleanup_scheduled_games_if_needed();
        self.persist_scheduled_games_if_needed().await?;
        self.cleanup_user_tokens_if_needed().await?;
        self.update_server_status_if_needed(false);
        Ok(())
    }

    /// C#: `private static void UpdateServerStatusIfNeeded(bool forceUpdate = false)` (GameHub_GameManagement.cs:602)
    pub fn update_server_status_if_needed(&mut self, force_update: bool) {
        let now = self.now();

        if !force_update && self.last_updated_server_status.add_milliseconds(f64::from(SERVER_STATUS_FREQUENCY_MS)) > now {
            return;
        }

        let running_games: Vec<GameInfo> = self.running_games_by_game_id.values().map(|g| self.extract_game_info(g)).collect();
        self.running_games = running_games;

        let scheduled_games: Vec<ScheduledGameInfo> = self.scheduled_games_by_game_id.values().map(|g| self.extract_scheduled_game_info(g)).collect();
        self.scheduled_games = scheduled_games;

        self.recently_seen_users = self
            .users_by_user_token
            .values()
            .filter(|u| u.last_seen_date_time.add_minutes(1.0) > now)
            .map(|u| LoggedInUserInfo { id: u.id(), status: u.status, name: Some(u.player_name()), last_seen: Some(u.last_seen_date_time) })
            .collect();

        self.last_updated_server_status = self.now();
    }

    /// C#: `private static ScheduledGameInfo ExtractScheduledGameInfo(ScheduledGame g)` (GameHub_GameManagement.cs:627)
    ///
    /// C# shares `SubscribedUsers` and `AllowedFactionsInPlay` with the scheduled game. Copies
    /// are equivalent: the list is never changed, and every change to the subscriptions
    /// (`RequestSubscribeGame`) refreshes the cached infos right away.
    pub fn extract_scheduled_game_info(&self, g: &ScheduledGame) -> ScheduledGameInfo {
        let mut subscriber_names = NetDictionary::new();
        for user_id in g.subscribed_users.keys() {
            let name = match self.users_by_id.get(user_id) {
                Some(user) => user.borrow().name.clone(),
                None => "?".to_string(),
            };
            subscriber_names.insert(*user_id, name);
        }

        ScheduledGameInfo {
            scheduled_game_id: Some(g.scheduled_game_id.clone()),
            ruleset: g.ruleset,
            async_play: g.async_play,
            date_time: g.date_time,
            creator_user_id: g.creator_user_id,
            creator_name: Some(g.creator_player_name.clone()),
            number_of_players: Some(g.number_of_players),
            maximum_turns: Some(g.maximum_turns),
            subscribers: g.subscribed_users.clone(),
            subscriber_names,
            allowed_factions_in_play: g.allowed_factions_in_play.clone(),
        }
    }

    /// C#: `private async Task SendEndOfGameMail(ManagedGame game)` (GameHub_GameManagement.cs:643)
    ///
    /// The faction names are `DefaultSkin.Default.Describe(faction)`, through
    /// `render::describe_faction`. The subject's `{info.Ruleset}` is the enum's `ToString()`,
    /// not the skin's name.
    pub async fn send_end_of_game_mail(&mut self, game: &ManagedGameRef) -> HubResult<()> {
        let info = self.extract_game_info(game);
        let state = GameState::get_state_as_string(&game.borrow().game);

        let from = self.services.config.get("GameEndEmailFrom").map(str::to_string);
        let to = self.services.config.get("GameEndEmailTo").map(str::to_string);
        let (Some(from), Some(to)) = (from, to) else {
            return Ok(());
        };

        let subject = format!("{} ({} Players, {} Bots, Turn {} - {})", info.name, info.nr_of_players, info.nr_of_bots, info.turn, info.ruleset);
        let save_game_to_attach = Attachment { name: format!("savegame{}.json", format_yyyymmdd_hhmm(self.now())), content: state };

        // StringBuilder.AppendLine: Environment.NewLine, "\n" on Linux and macOS.
        let mut sb = String::new();
        let mut append_line = |line: &str| {
            sb.push_str(line);
            sb.push('\n');
        };
        append_line("<h1>Player table in HTML format</h1>");
        append_line("<table>");
        append_line("<thead>");
        append_line("<tr><th>Faction</th><th>Seat</th><th>UserId</th>");
        append_line("</thead>");
        append_line("<tbody>");
        let players: Vec<(Faction, i32, i32)> = {
            let mg = game.borrow();
            mg.game.players.iter().map(|p| (p.faction, p.seat, mg.game.user_id_in_seat(p.seat))).collect()
        };
        for (faction, seat, user_id) in &players {
            append_line(&format!("<tr><td>{}</td><td>{seat}</td><td>{user_id}</td></tr>", render::describe_faction(*faction)));
        }
        append_line("</tbody>");
        append_line("</table>");
        append_line("<h1>Player table in CSV format</h1>");
        append_line("<p>");
        append_line("Faction;Seat;UserId");
        for (faction, seat, user_id) in &players {
            append_line(&format!("{};{seat};{user_id}", render::describe_faction(*faction)));
        }
        append_line("</p>");

        // `new MailAddress(...)` throws FormatException (or ArgumentException for "") outside
        // SendMail's try/catch: the exception escapes.
        let mail_message = MailMessage {
            from: MailAddress::parse(&from)?,
            to: vec![MailAddress::parse(&to)?],
            subject,
            is_body_html: true,
            body: sb,
            // `NrOfBots < 0.5f * NrOfPlayers`, compared as float.
            priority: if (info.nr_of_bots as f32) < 0.5f32 * info.nr_of_players as f32 { MailPriority::Normal } else { MailPriority::Low },
            attachments: vec![save_game_to_attach],
        };

        self.send_mail(mail_message);
        Ok(())
    }

    /// C#: `private static GameInfo ExtractGameInfo(ManagedGame managedGame)` (GameHub_GameManagement.cs:693)
    ///
    /// Every member is a snapshot except `SeatedPlayers`, which C# sets to the game's live
    /// `Participation.SeatedPlayers` dictionary. All infos extracted while the same dictionary
    /// object is current share one [`SeatedPlayersRef`], kept on the `ManagedGame`.
    ///
    /// `Settings` can be null only after an `EstablishPlayers` without settings (a forged
    /// event); C# then throws `NullReferenceException` here, and so does this.
    pub fn extract_game_info(&self, managed_game: &ManagedGameRef) -> GameInfo {
        let seated_players = {
            let mut mg = managed_game.borrow_mut();
            let participation = mg.game.participation.clone();
            match &mg.seated_players_ref {
                Some(r) if r.is_live_for(&participation) => r.clone(),
                _ => {
                    let r = SeatedPlayersRef::live(&participation);
                    mg.seated_players_ref = Some(r.clone());
                    r
                }
            }
        };

        let mg = managed_game.borrow();
        let game = &mg.game;
        let settings = game.settings.as_ref().nre().borrow();
        let before_start = game.current_phase <= Phase::AwaitingPlayers;

        let factions_in_play: Vec<Faction> = if before_start {
            settings.allowed_factions_in_play.as_ref().nre().borrow().clone()
        } else {
            game.players.iter().filter(|p| p.faction != Faction::None).map(|p| p.faction).collect()
        };

        let ruleset = if before_start {
            Game::determine_approximate_ruleset(
                &settings.allowed_factions_in_play.as_ref().nre().borrow(),
                settings.initial_rules.as_ref().nre(),
                Game::EXPANSION_LEVEL,
            )
        } else {
            let factions: Vec<Faction> = game.players.iter().map(|p| p.faction).collect();
            Game::determine_approximate_ruleset(&factions, &game.rules, Game::EXPANSION_LEVEL)
        };

        GameInfo {
            game_id: mg.game_id.clone(),
            creator_id: mg.creator_user_id,
            creator_username: match self.users_by_id.get(&mg.creator_user_id) {
                Some(creator) => creator.borrow().name.clone(),
                None => "?".to_string(),
            },
            creation_date: mg.creation_date,
            factions_in_play,
            nr_of_bots: game.number_of_bots(),
            ruleset,
            last_activity: Some(mg.last_activity),
            main_phase: game.current_main_phase,
            phase: game.current_phase,
            turn: game.current_turn,
            // C# parity: `Name` is null only for a restored row without a name, which no
            // server writes.
            name: mg.name.clone().unwrap_or_default(),
            has_password: !is_null_or_empty(mg.hashed_password.as_deref()),
            max_players: settings.number_of_players,
            max_turns: settings.maximum_turns,
            nr_of_players: game.participation.borrow().seated_players.len() as i32,
            seated_players,
            available_seats: game
                .players
                .iter()
                .filter(|p| game.seat_is_available(p.seat))
                .map(|p| AvailableSeatInfo { seat: p.seat, faction: p.faction, is_bot: p.is_bot(game) })
                .collect(),
        }
    }
}

/// `DateTime.ToString("yyyyMMdd.HHmm")` of the local clock time.
fn format_yyyymmdd_hhmm(now: DateTimeOffset) -> String {
    // "yyyy-MM-ddTHH:mm:ss..." for years 1 to 9999.
    let s = now.to_json_string();
    format!("{}{}{}.{}{}", &s[0..4], &s[5..7], &s[8..10], &s[11..13], &s[14..16])
}

#[cfg(test)]
mod tests {
    use super::*;
    use treachery_engine::date_time_offset::TICKS_PER_MINUTE;

    #[test]
    fn attachment_time_format() {
        let d = DateTimeOffset::parse_json("2026-03-02T09:05:59.9+02:00").unwrap();
        assert_eq!(format_yyyymmdd_hhmm(d), "20260302.0905");
        let d = DateTimeOffset::new(d.ticks + 14 * 60 * TICKS_PER_MINUTE, 120);
        assert_eq!(format_yyyymmdd_hhmm(d), "20260302.2305");
    }
}
