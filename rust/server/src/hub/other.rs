//! Port of `Server/GameHub_Other.cs`: connect, persistence, restore, cleanup and admin. Package H4.
//!
//! C# iterates the `ConcurrentDictionary` caches while it awaits database calls. Calls here run
//! one at a time, so nothing else changes a cache during a loop; the loops work on a snapshot of
//! the keys where the body needs `&mut self`. The C# enumeration order of those dictionaries is
//! not defined; the `BTreeMap` order is one of the possible ones (server-plan.md, "Order").

use std::cell::RefCell;
use std::rc::Rc;

use dotnet_compat::NetDictionary;
use treachery_engine::cs::{self, Shared};
use treachery_engine::date_time_offset::DateTimeOffset;
use treachery_engine::enumerations::{ErrorType, SubscriptionType};
use treachery_engine::game::Game;
use treachery_engine::game_state::GameState;
use treachery_engine::json::{JsonError, JsonValue};

use super::{GameHub, HubError, HubResult, ManagedGameRef, CLEANUP_FREQUENCY_HOURS, GAME_PERSIST_FREQUENCY_MINUTES, MAXIMUM_LOGIN_TIME_DAYS, PERSIST_FREQUENCY_MINUTES};
use crate::clients::Callback;
use crate::db::persisted_game::PersistedGame;
use crate::db::persisted_scheduled_game::PersistedScheduledGame;
use crate::json;
use crate::logged_in_user::LoggedInUser;
use crate::model::admin_info::AdminInfo;
use crate::model::managed_game::ManagedGame;
use crate::model::result::{CallResult, VoidResult};
use crate::model::scheduled_game::ScheduledGame;
use crate::model::server_info::ServerInfo;
use crate::model::user_info::UserInfo;
use crate::net_time::{DateTimeOffsetExt, TimeSpanExt};

impl GameHub {
    /// C#: `public async Task<Result<ServerInfo>> Connect()` (GameHub_Other.cs:5)
    pub async fn connect(&mut self) -> HubResult<CallResult<ServerInfo>> {
        self.restore_games_if_server_just_started().await?;

        let result = ServerInfo {
            admin_name: self.services.config.get("GameAdminUsername").unwrap_or("admin").to_string(),
            scheduled_maintenance: self.maintenance_date,
        };

        Ok(Self::success_with(result))
    }

    /// C#: `private async Task CleanupUserTokensIfNeeded()` (GameHub_Other.cs:18)
    pub async fn cleanup_user_tokens_if_needed(&mut self) -> HubResult<()> {
        if self.last_cleaned_up_user_tokens.add_hours(f64::from(CLEANUP_FREQUENCY_HOURS)) > self.now() {
            return Ok(());
        }

        self.cleanup_user_tokens().await
    }

    /// C#: `private async Task CleanupUserTokens()` (GameHub_Other.cs:31)
    pub async fn cleanup_user_tokens(&mut self) -> HubResult<()> {
        let now = self.now();
        if self.last_cleaned_up_user_tokens.add_hours(f64::from(CLEANUP_FREQUENCY_HOURS)) > now {
            return Ok(());
        }

        self.last_cleaned_up_user_tokens = now;

        Self::log(&format!("CleanupUserTokensIfNeeded {} {}", self.users_by_user_token.len(), self.connection_info_by_user_id.len()));

        self.users_by_user_token
            .retain(|_, user| now.subtract(user.logged_in_date_time).total_days() < f64::from(MAXIMUM_LOGIN_TIME_DAYS));

        // `db.UserTokens.AsEnumerable().Where(...)`: the filter runs in .NET, on every row.
        let expired: Vec<String> = self
            .services
            .db
            .user_tokens_all()
            .await?
            .into_iter()
            .filter(|t| now.subtract(t.created).total_days() >= f64::from(MAXIMUM_LOGIN_TIME_DAYS))
            .map(|t| t.token)
            .collect();
        self.services.db.user_tokens_delete(&expired).await?;

        let user_ids: Vec<i32> = self.connection_info_by_user_id.keys().copied().collect();
        for user_id in user_ids {
            let Some(connections) = self.connection_info_by_user_id.get(&user_id) else { continue };
            for game_id in connections.get_game_ids_with_old_connections(MAXIMUM_LOGIN_TIME_DAYS, self.now()) {
                self.remove_from_group(&game_id, user_id);
            }
        }

        Ok(())
    }

    /// C#: `private static void CleanupScheduledGamesIfNeeded()` (GameHub_Other.cs:68)
    pub fn cleanup_scheduled_games_if_needed(&mut self) {
        if self.last_cleaned_up_scheduled_games.add_hours(f64::from(CLEANUP_FREQUENCY_HOURS)) > self.now() {
            return;
        }

        Self::log(&format!("CleanupScheduledGamesIfNeeded {}", self.scheduled_games_by_game_id.len()));

        let threshold_date_time = self.now().add_hours(-4.0);
        self.scheduled_games_by_game_id.retain(|_, g| g.date_time >= threshold_date_time);

        self.last_cleaned_up_scheduled_games = self.now();
    }

    /// C#: `public async Task<Result<string>> AdminUpdateMaintenance(string userToken, DateTimeOffset maintenanceDate)` (GameHub_Other.cs:84)
    pub async fn admin_update_maintenance(&mut self, user_token: &str, maintenance_date: DateTimeOffset) -> HubResult<CallResult<String>> {
        if !self.is_admin_token(user_token) {
            return Ok(Self::error(ErrorType::InvalidUserNameOrPassword, ""));
        }

        self.maintenance_date = maintenance_date;
        Ok(Self::success_with("Maintenance window updated".to_string()))
    }

    /// C#: `public async Task<Result<string>> AdminPersistState(string userToken)` (GameHub_Other.cs:93)
    pub async fn admin_persist_state(&mut self, user_token: &str) -> HubResult<CallResult<String>> {
        if !self.is_admin_token(user_token) {
            return Ok(Self::error(ErrorType::InvalidUserNameOrPassword, ""));
        }

        let (amount_of_new_games, amount_of_updated_games, amount_of_unchanged, amount_of_deleted_games) = self.persist_running_games().await?;
        let amount_of_scheduled_games = self.persist_scheduled_games().await?;

        Ok(Self::success_with(format!(
            "Games: {amount_of_new_games} new, {amount_of_updated_games} updated, {amount_of_unchanged} unchanged, {amount_of_deleted_games} deleted. Scheduled games: {amount_of_scheduled_games}."
        )))
    }

    /// C#: `private async Task PersistScheduledGamesIfNeeded()` (GameHub_Other.cs:104)
    pub async fn persist_scheduled_games_if_needed(&mut self) -> HubResult<()> {
        if self.last_persisted_scheduled_games.add_minutes(f64::from(PERSIST_FREQUENCY_MINUTES)) > self.now() {
            return Ok(());
        }

        self.persist_scheduled_games().await?;
        Ok(())
    }

    /// C#: `private async Task<(int, int, int, int)> PersistRunningGames()` (GameHub_Other.cs:112):
    /// new, updated, unchanged, deleted.
    pub async fn persist_running_games(&mut self) -> HubResult<(i32, i32, i32, i32)> {
        if self.last_restored == DateTimeOffset::default() {
            return Ok((0, 0, 0, 0));
        }

        let now = self.now();

        Self::log("PersistRunningGames started...");

        let mut amount_of_new_games = 0;
        let mut amount_of_updated_games = 0;
        let mut amount_of_deleted_games = 0;
        let mut amount_of_unchanged = 0;

        let changed: Vec<(String, ManagedGameRef)> = self
            .running_games_by_game_id
            .iter()
            .filter(|(_, game)| game.borrow().last_activity > self.last_persisted_running_games)
            .map(|(key, game)| (key.clone(), game.clone()))
            .collect();

        for (key, game) in changed {
            let game = game.borrow();
            let persisted_game = self.services.db.persisted_games_first_by_game_id(&game.game_id).await?;
            if let Some(mut persisted_game) = persisted_game {
                if persisted_game.last_action == game.last_activity {
                    amount_of_unchanged += 1;
                    continue;
                }

                persisted_game.game_state = GameState::get_state_as_string(&game.game);
                persisted_game.game_participation = json::serialize_participation(&game.game.participation.borrow());
                persisted_game.statistics_sent = game.statistics_sent;
                persisted_game.last_async_play_message_sent = game.last_async_play_message_sent;
                persisted_game.last_action = game.last_activity;
                self.services.db.persisted_games_update_all(&persisted_game).await?;
                amount_of_updated_games += 1;
            } else {
                // C# parity: `GameId` is the dictionary key here, and `LastAction` is the time of
                // the last event rather than `LastActivity`.
                self.services.db.persisted_games_insert(&new_persisted_game(&game, key)).await?;
                amount_of_new_games += 1;
            }
        }

        for persisted_game in self.services.db.persisted_games_all().await? {
            let Some(game_id) = &persisted_game.game_id else { continue };
            if self.running_games_by_game_id.contains_key(game_id) {
                continue;
            }
            self.services.db.persisted_games_delete(persisted_game.id).await?;
            amount_of_deleted_games += 1;
        }

        self.last_persisted_running_games = now;

        Self::log(&format!(
            "PersistRunningGames: {amount_of_new_games} new, {amount_of_updated_games} updated, {amount_of_unchanged} unchanged, {amount_of_deleted_games} deleted."
        ));

        Ok((amount_of_new_games, amount_of_updated_games, amount_of_unchanged, amount_of_deleted_games))
    }

    /// C#: `private async Task<int> PersistScheduledGames()` (GameHub_Other.cs:188)
    pub async fn persist_scheduled_games(&mut self) -> HubResult<i32> {
        if self.last_restored == DateTimeOffset::default() {
            return Ok(0);
        }

        self.last_persisted_scheduled_games = self.now();

        Self::log("PersistScheduledGames started...");

        let mut amount_of_scheduled_games = 0;

        self.services.db.scheduled_games_delete_all().await?;

        for (key, game) in &self.scheduled_games_by_game_id {
            let persisted = PersistedScheduledGame {
                id: 0,
                date_time: game.date_time,
                creator_user_id: game.creator_user_id,
                creator_player_name: game.creator_player_name.clone(),
                game_id: key.clone(),
                ruleset: game.ruleset,
                maximum_turns: game.maximum_turns,
                number_of_players: game.number_of_players,
                allowed_factions_in_play: game.allowed_factions_in_play.clone(),
                subscribed_users: serialize_subscriptions(&game.subscribed_users),
                async_play: game.async_play,
            };

            // One SaveChanges per game: a failure leaves the games before it saved.
            self.services.db.scheduled_games_insert(&persisted).await?;
            amount_of_scheduled_games += 1;
        }

        Self::log(&format!("PersistScheduledGames: {amount_of_scheduled_games}."));

        Ok(amount_of_scheduled_games)
    }

    /// C#: `private async Task EraseGame(ManagedGame game)` (GameHub_Other.cs:230)
    pub async fn erase_game(&mut self, game: &ManagedGameRef) -> HubResult<()> {
        let game_id = game.borrow().game_id.clone();
        if let Some(persisted_game) = self.services.db.persisted_games_first_by_game_id(&game_id).await? {
            self.services.db.persisted_games_delete(persisted_game.id).await?;
        }
        Ok(())
    }

    /// C#: `private async Task PersistGameIfNeeded(ManagedGame game)` (GameHub_Other.cs:241)
    pub async fn persist_game_if_needed(&mut self, game: &ManagedGameRef) -> HubResult<()> {
        if game.borrow().last_persisted.add_minutes(f64::from(GAME_PERSIST_FREQUENCY_MINUTES)) > self.now() {
            return Ok(());
        }

        self.persist_game(game).await
    }

    /// C#: `private async Task PersistGame(ManagedGame game)` (GameHub_Other.cs:249)
    pub async fn persist_game(&mut self, game: &ManagedGameRef) -> HubResult<()> {
        game.borrow_mut().last_persisted = self.now();

        let game = game.borrow();
        let persisted_game = self.services.db.persisted_games_first_by_game_id(&game.game_id).await?;
        if let Some(mut persisted_game) = persisted_game {
            if persisted_game.last_action == game.last_activity {
                return Ok(());
            }

            persisted_game.game_state = GameState::get_state_as_string(&game.game);
            persisted_game.game_participation = json::serialize_participation(&game.game.participation.borrow());
            persisted_game.statistics_sent = game.statistics_sent;
            persisted_game.last_async_play_message_sent = game.last_async_play_message_sent;
            persisted_game.last_action = game.last_activity;
            self.services.db.persisted_games_update_all(&persisted_game).await?;
        } else {
            self.services.db.persisted_games_insert(&new_persisted_game(&game, game.game_id.clone())).await?;
        }

        Ok(())
    }

    /// C#: `public async Task<Result<string>> AdminRestoreState(string userToken)` (GameHub_Other.cs:289)
    pub async fn admin_restore_state(&mut self, user_token: &str) -> HubResult<CallResult<String>> {
        if !self.is_admin_token(user_token) {
            return Ok(Self::error(ErrorType::InvalidUserNameOrPassword, ""));
        }

        let (amount_running, amount_scheduled) = self.restore_games_and_user_cache().await?;

        Ok(Self::success_with(format!("Restored: {amount_running} running games, {amount_scheduled} scheduled games")))
    }

    /// C#: `private async Task RestoreGamesIfServerJustStarted()` (GameHub_Other.cs:304). The C#
    /// semaphore is not needed: calls never overlap here.
    pub async fn restore_games_if_server_just_started(&mut self) -> HubResult<()> {
        if self.last_restored != DateTimeOffset::default() {
            return Ok(());
        }

        self.restore_games_and_user_cache().await?;
        Ok(())
    }

    /// C#: `private async Task<(int amountRunning, int amountScheduled)> RestoreGamesAndUserCache()` (GameHub_Other.cs:320)
    ///
    /// C# parity, kept on purpose:
    /// - `UsersById` gets new `User` objects, but the `LoggedInUser`s already in
    ///   `UsersByUserToken` keep the objects they had (a token is only added when absent).
    /// - Every running game is replaced by the database copy, discarding unsaved changes.
    ///   Group memberships stay.
    /// - The counts include games whose id was already added (`TryAdd` fails, the count still
    ///   goes up). A game that fails to load is skipped, logged only if it threw.
    /// - Bad `SubscribedUsers` JSON throws out of the method, with `Restoring` left set and
    ///   `LastRestored` unchanged.
    pub async fn restore_games_and_user_cache(&mut self) -> HubResult<(i32, i32)> {
        self.restoring = true;

        Self::log("RestoreGamesAndUserCache started...");

        let mut amount_running = 0;
        let mut amount_scheduled = 0;

        self.users_by_id.clear();
        for user in self.services.db.users_all().await? {
            self.users_by_id.entry(user.id).or_insert_with(|| Rc::new(RefCell::new(user)));
        }

        self.restore_user_tokens().await?;

        self.running_games_by_game_id.clear();

        for persisted_game in self.services.db.persisted_games_all().await? {
            let Some(id) = persisted_game.game_id.clone() else { continue };

            match load_persisted_game(&persisted_game) {
                Ok(Some(game)) => {
                    let managed_game = ManagedGame {
                        creation_date: persisted_game.creation_date,
                        last_activity: persisted_game.last_action,
                        last_persisted: DateTimeOffset::default(),
                        creator_user_id: persisted_game.creator_user_id,
                        game_id: id.clone(),
                        game,
                        name: persisted_game.game_name.clone(),
                        hashed_password: persisted_game.hashed_password.clone(),
                        observers_require_password: persisted_game.observers_require_password,
                        statistics_sent: persisted_game.statistics_sent,
                        last_async_play_message_sent: persisted_game.last_async_play_message_sent,
                        seated_players_ref: None,
                        bots: Vec::new(),
                    };

                    self.running_games_by_game_id.entry(id).or_insert_with(|| Rc::new(RefCell::new(managed_game)));
                    amount_running += 1;
                }
                Ok(None) => {}
                Err(ex) => Self::log(&format!("Unable to restore game {id}: {ex}")),
            }
        }

        self.scheduled_games_by_game_id.clear();

        for scheduled_game in self.services.db.scheduled_games_all().await? {
            let id = scheduled_game.game_id.clone();

            let subscriptions = deserialize_subscriptions(&scheduled_game.subscribed_users)
                .map_err(|e| HubError::Exception { type_name: e.kind, message: e.message })?;
            let game = ScheduledGame {
                scheduled_game_id: scheduled_game.game_id,
                date_time: scheduled_game.date_time,
                creator_user_id: scheduled_game.creator_user_id,
                creator_player_name: scheduled_game.creator_player_name,
                ruleset: scheduled_game.ruleset,
                maximum_turns: scheduled_game.maximum_turns,
                number_of_players: scheduled_game.number_of_players,
                allowed_factions_in_play: scheduled_game.allowed_factions_in_play,
                subscribed_users: subscriptions,
                async_play: scheduled_game.async_play,
            };

            self.scheduled_games_by_game_id.entry(id).or_insert(game);
            amount_scheduled += 1;
        }

        self.last_restored = self.now();
        self.last_persisted_running_games = self.last_restored;

        Self::log(&format!("RestoreGamesAndUserCache {amount_running} {amount_scheduled}"));

        self.restoring = false;

        Ok((amount_running, amount_scheduled))
    }

    /// C#: `private static void RestoreUserTokens(TreacheryContext context)` (GameHub_Other.cs:418).
    /// Tokens already in memory are kept, so an admin-triggered restore does not reset anyone's
    /// status.
    pub async fn restore_user_tokens(&mut self) -> HubResult<()> {
        let now = self.now();
        for token in self.services.db.user_tokens_all().await? {
            if now.subtract(token.created).total_days() >= f64::from(MAXIMUM_LOGIN_TIME_DAYS) {
                continue;
            }
            let Some(user) = self.users_by_id.get(&token.user_id) else { continue };
            let user = user.clone();
            self.users_by_user_token.entry(token.token).or_insert_with(|| LoggedInUser::new(user, Some(token.created), now));
        }
        Ok(())
    }

    /// C#: `public async Task<Result<string>> AdminCloseGame(string userToken, string gameId)` (GameHub_Other.cs:429)
    pub async fn admin_close_game(&mut self, user_token: &str, game_id: &str) -> HubResult<CallResult<String>> {
        if !self.is_admin_token(user_token) {
            return Ok(Self::error(ErrorType::InvalidUserNameOrPassword, ""));
        }

        self.pauses_by_game_id.remove(game_id);
        if let Some(game) = self.running_games_by_game_id.remove(game_id) {
            self.remove_all_players(&game, game_id);
        }

        Ok(Self::success_with("Game removed".to_string()))
    }

    /// The loop of `AdminCloseGame` (GameHub_Other.cs:438) and `DeleteUser` (:480): for each user
    /// in `PlayerNames`, `RemoveUser(userId, true)` (an engine call that logs), `HandleRemoveUser`
    /// to the group, then `RemoveFromGroup`.
    ///
    /// C# enumerates `PlayerNames.Keys` while `RemoveUser` removes the current key, which .NET
    /// allows and which visits every key once, in order. A copy of the keys does the same.
    fn remove_all_players(&mut self, game: &ManagedGameRef, game_id: &str) {
        let user_ids: Vec<i32> = game.borrow().game.participation.borrow().player_names.keys().copied().collect();
        for user_id in user_ids {
            game.borrow_mut().game.remove_user(user_id, true);
            self.clients.group(game_id, Callback::HandleRemoveUser { user_id, kick: true });
            self.remove_from_group(game_id, user_id);
        }
    }

    /// C#: `public async Task<Result<string>> AdminDownloadGame(string userToken, string gameId)` (GameHub_Other.cs:449)
    pub async fn admin_download_game(&mut self, user_token: &str, game_id: &str) -> HubResult<CallResult<String>> {
        if !self.is_admin_token(user_token) {
            return Ok(Self::error(ErrorType::InvalidUserNameOrPassword, ""));
        }

        let Some(game) = self.running_games_by_game_id.get(game_id) else {
            return Ok(Self::error(ErrorType::GameNotFound, ""));
        };

        Ok(Self::success_with(GameState::get_state_as_string(&game.borrow().game)))
    }

    /// C#: `public async Task<Result<string>> AdminCancelGame(string userToken, string scheduledGameId)` (GameHub_Other.cs:461)
    pub async fn admin_cancel_game(&mut self, user_token: &str, scheduled_game_id: &str) -> HubResult<CallResult<String>> {
        if !self.is_admin_token(user_token) {
            return Ok(Self::error(ErrorType::InvalidUserNameOrPassword, ""));
        }

        self.scheduled_games_by_game_id.remove(scheduled_game_id);
        Ok(Self::success_with("Scheduled game cancelled".to_string()))
    }

    /// C#: `public async Task<Result<string>> DeleteUser(string userToken, int userId)` (GameHub_Other.cs:470)
    ///
    /// C# parity: the user stays in `UsersById`, their tokens stay in `UsersByUserToken` and in
    /// the `UserTokens` table, and games they only joined keep them.
    pub async fn delete_user(&mut self, user_token: &str, user_id: i32) -> HubResult<CallResult<String>> {
        let Some(user) = self.users_by_user_token.get(user_token) else {
            return Ok(Self::error(ErrorType::InvalidUserNameOrPassword, ""));
        };

        if user_id != user.id() && self.services.config.get("GameAdminUsername") != Some(user.username().as_str()) {
            return Ok(Self::error(ErrorType::NotAdmin, ""));
        }

        let games: Vec<ManagedGameRef> =
            self.running_games_by_game_id.values().filter(|x| x.borrow().creator_user_id == user_id).cloned().collect();
        for game in games {
            let game_id = game.borrow().game_id.clone();
            self.remove_all_players(&game, &game_id);

            self.running_games_by_game_id.remove(&game_id);
            self.pauses_by_game_id.remove(&game_id);
            self.erase_game(&game).await?;
        }

        self.services.db.users_delete(user_id).await?;

        Ok(Self::success_with("User deleted".to_string()))
    }

    /// C#: `public async Task<Result<AdminInfo>> GetAdminInfo(string userToken)` (GameHub_Other.cs:497)
    pub async fn get_admin_info(&mut self, user_token: &str) -> HubResult<CallResult<AdminInfo>> {
        if !self.is_admin_token(user_token) {
            return Ok(Self::error(ErrorType::InvalidUserNameOrPassword, ""));
        }

        let users = self
            .services
            .db
            .users_all()
            .await?
            .into_iter()
            .map(|u| UserInfo { id: u.id, username: u.name, player_name: u.player_name, email: u.email, last_login: u.last_login })
            .collect();

        let result = AdminInfo {
            users: Some(users),
            users_by_user_token_count: self.users_by_user_token.len() as i32,
            connection_info_by_user_id_count: self.connection_info_by_user_id.len() as i32,
            games_by_game_id_count: self.running_games_by_game_id.len() as i32,
        };

        Ok(Self::success_with(result))
    }

    /// C#: `public async Task<VoidResult> RequestNudgeBots(string userToken, string gameId)` (GameHub_Other.cs:514)
    pub async fn request_nudge_bots(&mut self, user_token: &str, game_id: &str) -> HubResult<VoidResult> {
        let game = match self.are_valid(Some(user_token), Some(game_id)) {
            Ok((_, game)) => game,
            Err(error) => return Ok(error),
        };

        self.schedule_bot_event(&game, true).await?;

        Ok(Self::success())
    }
}

/// The `new PersistedGame { ... }` of `PersistRunningGames` (GameHub_Other.cs:149) and
/// `PersistGame` (:270), which differ only in where `GameId` comes from.
fn new_persisted_game(game: &ManagedGame, game_id: String) -> PersistedGame {
    PersistedGame {
        id: 0,
        creation_date: game.creation_date,
        creator_user_id: game.creator_user_id,
        game_id: Some(game_id),
        game_state: GameState::get_state_as_string(&game.game),
        game_name: game.name.clone(),
        game_participation: json::serialize_participation(&game.game.participation.borrow()),
        hashed_password: game.hashed_password.clone(),
        observers_require_password: game.observers_require_password,
        statistics_sent: game.statistics_sent,
        last_async_play_message_sent: game.last_async_play_message_sent,
        last_action: game.game.history.last().map(|e| e.time()).unwrap_or(game.creation_date),
    }
}

/// The `try` block of `RestoreGamesAndUserCache` (GameHub_Other.cs:347-372): `Ok(Some)` for a
/// loaded game, `Ok(None)` when `TryLoad` returned a message, `Err` for what the `catch` logs.
fn load_persisted_game(persisted_game: &PersistedGame) -> Result<Option<Game>, String> {
    let outcome = cs::catch(|| -> Result<Option<Game>, JsonError> {
        let game_state = GameState::load(&persisted_game.game_state);
        let participation = Shared::new(json::deserialize_participation(&persisted_game.game_participation)?);
        let mut game = None;
        let load_message = Game::try_load(&game_state, &participation, false, true, &mut game);
        Ok(if load_message.is_none() { game } else { None })
    });
    match outcome {
        Ok(Ok(game)) => Ok(game),
        Ok(Err(e)) => Err(format!("{}: {}", e.kind, e.message)),
        Err(e) => Err(e.to_string()),
    }
}

/// `Utilities.Serialize(game.SubscribedUsers)` (GameHub_Other.cs:215).
fn serialize_subscriptions(subscriptions: &NetDictionary<i32, SubscriptionType>) -> String {
    let mut w = treachery_engine::json::JsonWriter::new();
    json::write_int_dict(&mut w, subscriptions);
    w.finish()
}

/// `Utilities.Deserialize<Dictionary<int,SubscriptionType>>(scheduledGame.SubscribedUsers)`
/// (GameHub_Other.cs:387).
fn deserialize_subscriptions(text: &str) -> Result<NetDictionary<i32, SubscriptionType>, JsonError> {
    let v: JsonValue = serde_json::from_str(text).map_err(|e| JsonError::new("JsonException", e.to_string()))?;
    json::read_int_dict(&v)
}
