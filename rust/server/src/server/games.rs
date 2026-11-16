//! Creating, loading and closing games; joining, observing and leaving them; seats, hosts and
//! the host's other controls.

use treachery_protocol::UserId;
use treachery_engine::game::{Game, Seat};
use treachery_engine::game::GameSettings;

use super::{refuse, Answer, CallError, RunningGame, Server, MAX_BOT_SPEED, MAX_GAMES_PER_USER};
use crate::connections::ConnectionId;
use crate::protocol::{ErrorCode, Notification};
use crate::ids;
use crate::participation::{self, Participation};
use treachery_protocol::{GameInitInfo, GameListScope, Lobby};

/// Loads a savegame into a game whose seats the participation gives, or says why it can't. A
/// savegame of version 190 or before is converted (`treachery_legacy::load_any`).
pub fn load_savegame(savegame: &str, participation: &Participation, restoring: bool) -> Result<Game, String> {
    let state = treachery_legacy::load_any(savegame)?;
    Game::load(&state, &participation.human_seats(), false, restoring).map_err(|m| crate::render::message_to_string(&m))
}

fn refuse_savegame(reason: String) -> CallError {
    CallError::Refused(ErrorCode::InvalidGameEvent, reason)
}

impl Server {
    fn init_info(&self, game: &str) -> GameInitInfo {
        let g = &self.games[game];
        GameInitInfo {
            game_id: g.id.clone(),
            game_name: g.name.clone(),
            participation: participation::to_wire(&g.participation),
            phase_control: Some(g.pause.last_sent.clone().unwrap_or_default()),
        }
    }

    /// The connection follows the game, and gets its view after this call.
    fn follow(&mut self, game: &str, connection: ConnectionId, user: UserId) {
        self.connections.follow(game, connection, user);
        self.connections.view_for(game, connection, user);
    }

    pub(super) async fn create_game(
        &mut self,
        connection: ConnectionId,
        user: UserId,
        name: Option<&str>,
        password_hash: Option<&str>,
        savegame: Option<&str>,
        skin: Option<&str>,
    ) -> Answer<GameInitInfo> {
        if self.games.values().filter(|g| g.creator == user).count() >= MAX_GAMES_PER_USER {
            let message = format!("You cannot have more than {MAX_GAMES_PER_USER} active games, delete a game first");
            return Err(CallError::Refused(ErrorCode::TooManyGames, message));
        }
        let game = match savegame.filter(|s| !s.is_empty()) {
            Some(savegame) => load_savegame(savegame, &Participation::new(), false).map_err(refuse_savegame)?,
            None => Game::new(),
        };
        let player_name = self.player_name(user);
        let name = match name.filter(|n| !n.is_empty()) {
            Some(n) => n.to_string(),
            None => format!("{player_name}'s Game"),
        };
        let id = ids::new_id();
        let mut running = RunningGame::new(id.clone(), name, user, game, Participation::new(), self.now());
        running.password_hash = password_hash.filter(|p| !p.is_empty()).map(str::to_string);
        running.skin = skin.filter(|s| !s.is_empty()).map(str::to_string);
        running.generation = self.new_generation();
        running.participation.add_player(&mut running.game, user, &player_name, None);
        running.participation.set_or_unset_host(user);
        self.games.insert(id.clone(), running);
        self.follow(&id, connection, user);
        if let Some(skin) = skin.filter(|s| !s.is_empty()) {
            self.connections.notify(&id, Notification::SetSkin { skin: skin.to_string() });
        }
        self.changed(&id);
        let mut info = self.init_info(&id);
        info.phase_control = None;
        Ok(info)
    }

    pub(super) async fn load_game(&mut self, user: UserId, game: &str, savegame: &str, skin: Option<&str>) -> Answer<()> {
        let game = self.host_of(user, game)?;
        let generation = self.new_generation();
        let g = self.games.get_mut(&game).expect("checked");
        // The loaded game keeps this game's participation.
        let mut loaded = load_savegame(savegame, &g.participation, false).map_err(refuse_savegame)?;
        if loaded.seed != g.game.seed {
            g.participation.reset_seats(&mut loaded);
        }
        g.game = loaded;
        g.generation = generation;
        if let Some(skin) = skin.filter(|s| !s.is_empty()) {
            g.skin = Some(skin.to_string());
        }
        let mut info = self.init_info(&game);
        info.phase_control = None;
        self.connections.notify(&game, Notification::LoadGame(info));
        self.suspend_pause(&game);
        if let Some(skin) = skin.filter(|s| !s.is_empty()) {
            self.connections.notify(&game, Notification::SetSkin { skin: skin.to_string() });
        }
        self.changed(&game);
        self.schedule_bots(&game);
        Ok(())
    }

    /// Seats the game's players. Only users who play in the game can be seated, each on a seat
    /// of the game's players or on none, and no two on the same seat.
    pub(super) fn assign_seats(&mut self, user: UserId, game: &str, seats: Vec<(UserId, Option<Seat>)>) -> Answer<()> {
        let game = self.host_of(user, game)?;
        {
            let g = &self.games[&game];
            let bad = |reason: String| Err(CallError::Refused(ErrorCode::InvalidGameEvent, reason));
            for (i, (user, seat)) in seats.iter().enumerate() {
                if !g.participation.is_player(*user) {
                    return Err(refuse(ErrorCode::UserNotInGame));
                }
                if let Some(seat) = seat.filter(|s| g.game.get_player_by_seat(*s).is_none()) {
                    return bad(format!("There is no seat {seat}"));
                }
                if seats[..i].iter().any(|(u, s)| u == user || (s.is_some() && s == seat)) {
                    return bad("Each user and each seat may be assigned once".into());
                }
            }
        }
        {
            let g = self.games.get_mut(&game).expect("checked");
            g.participation.assign_seats(&mut g.game, &seats);
        }
        self.connections.notify(&game, Notification::AssignSeats { seats });
        self.changed(&game);
        self.schedule_bots(&game);
        Ok(())
    }

    /// A user who already plays in the game only follows it again. Others need the password, an
    /// open seat (or none in particular before the game starts) and not to have been kicked.
    /// Taking a seat that another user holds removes that user.
    pub(super) fn join_game(&mut self, connection: ConnectionId, user: UserId, game: &str, password_hash: Option<&str>, seat: Option<Seat>) -> Answer<GameInitInfo> {
        let game = self.running(game)?;
        let player_name = self.player_name(user);
        let g = self.games.get_mut(&game).expect("checked");
        if !g.participation.is_player(user) {
            if let Some(password) = g.password_hash.as_deref() {
                if password_hash != Some(password) {
                    return Err(refuse(ErrorCode::IncorrectGamePassword));
                }
            }
            // A particular seat must be one the host opened; before the game starts, users join
            // without one.
            if !g.participation.is_open(&g.game, seat) || seat.is_some_and(|s| !g.participation.seat_is_available(s)) {
                return Err(refuse(ErrorCode::SeatNotAvailableNotOpen));
            }
            if g.participation.was_kicked(user) {
                return Err(refuse(ErrorCode::SeatNotAvailableKicked));
            }
            if let Some(seat) = seat {
                if let Some(previous) = g.participation.user_in_seat(seat) {
                    g.participation.remove_user(&mut g.game, previous, false);
                    self.connections.notify(&game, Notification::RemoveUser { user: previous, kick: false });
                }
            }
            let g = self.games.get_mut(&game).expect("checked");
            g.participation.add_player(&mut g.game, user, &player_name, seat);
            let make_host = g.participation.number_of_seated_players() == 1 || g.creator == user;
            if make_host && !g.participation.is_host(user) {
                g.participation.set_or_unset_host(user);
            }
            self.connections.notify(&game, Notification::JoinGame { user, name: player_name, seat });
            if make_host {
                self.connections.notify(&game, Notification::SetOrUnsetHost { user });
            }
        }
        self.follow(&game, connection, user);
        self.changed(&game);
        if self.games[&game].participation.number_of_seated_players() == 1 {
            self.schedule_bots(&game);
        }
        Ok(self.init_info(&game))
    }

    pub(super) fn observe_game(&mut self, connection: ConnectionId, user: UserId, game: &str, _password_hash: Option<&str>) -> Answer<GameInitInfo> {
        let game = self.running(game)?;
        let player_name = self.player_name(user);
        let g = self.games.get_mut(&game).expect("checked");
        if !g.participation.is_observer(user) {
            if g.participation.is_player(user) {
                return Err(refuse(ErrorCode::AlreadyPlayer));
            }
            // A kicked user doesn't come back to watch either.
            if g.participation.was_kicked(user) {
                return Err(refuse(ErrorCode::SeatNotAvailableKicked));
            }
            g.participation.add_observer(&mut g.game, user, &player_name);
            self.connections.notify(&game, Notification::ObserveGame { user, name: player_name });
        }
        self.follow(&game, connection, user);
        self.changed(&game);
        self.schedule_bots(&game);
        Ok(self.init_info(&game))
    }

    /// A new connection of a player or observer follows the game again.
    pub(super) fn reconnect_game(&mut self, connection: ConnectionId, user: UserId, game: &str) -> Answer<GameInitInfo> {
        let game = self.participant_of(user, game)?;
        self.follow(&game, connection, user);
        Ok(self.init_info(&game))
    }

    pub(super) fn game_state(&mut self, connection: ConnectionId, user: UserId, game: &str) -> Answer<GameInitInfo> {
        let game = self.participant_of(user, game)?;
        self.connections.view_for(&game, connection, user);
        Ok(self.init_info(&game))
    }

    pub(super) fn leave_game(&mut self, user: UserId, game: &str) -> Answer<Lobby> {
        let game = self.participant_of(user, game)?;
        self.remove_from_game(&game, user);
        Ok(self.lobby_status(GameListScope::Active, user))
    }

    /// Takes a user out of a running game. When the last host leaves, every other seated
    /// player becomes a host.
    pub(super) fn remove_from_game(&mut self, game: &str, user: UserId) {
        let game = game.to_string();
        let g = self.games.get_mut(&game).expect("running game");
        if g.participation.is_host(user) && g.participation.number_of_hosts() == 1 {
            let others: Vec<UserId> = g.participation.seated_players.keys().copied().filter(|u| *u != user).collect();
            for other in others {
                let g = self.games.get_mut(&game).expect("checked");
                g.participation.set_or_unset_host(other);
                self.connections.notify(&game, Notification::SetOrUnsetHost { user: other });
            }
        }
        let g = self.games.get_mut(&game).expect("checked");
        g.participation.remove_user(&mut g.game, user, false);
        self.connections.notify(&game, Notification::RemoveUser { user, kick: false });
        self.connections.unfollow_user(&game, user);
        self.changed(&game);
        self.schedule_bots(&game);
    }

    /// Removes another user of the game for good.
    pub(super) fn kick(&mut self, host: UserId, game: &str, user: UserId) -> Answer<()> {
        let game = self.host_of(host, game)?;
        if user == host || !self.games[&game].participation.is_participant(user) {
            return Err(refuse(ErrorCode::UserNotInGame));
        }
        let g = self.games.get_mut(&game).expect("checked");
        g.participation.remove_user(&mut g.game, user, true);
        self.connections.notify(&game, Notification::RemoveUser { user, kick: true });
        self.connections.unfollow_user(&game, user);
        self.changed(&game);
        self.schedule_bots(&game);
        Ok(())
    }

    /// Makes a player of the game a host, or no longer one.
    pub(super) fn set_or_unset_host(&mut self, host: UserId, game: &str, user: UserId) -> Answer<()> {
        let game = self.host_of(host, game)?;
        let g = self.games.get_mut(&game).expect("checked");
        if !g.participation.is_player(user) {
            return Err(refuse(ErrorCode::UserNotInGame));
        }
        if user == host && g.participation.number_of_hosts() <= 1 {
            return Err(refuse(ErrorCode::CannotRemoveLastHost));
        }
        g.participation.set_or_unset_host(user);
        self.connections.notify(&game, Notification::SetOrUnsetHost { user });
        self.changed(&game);
        Ok(())
    }

    /// Opens or closes the seat of one of the game's players.
    pub(super) fn open_or_close_seat(&mut self, user: UserId, game: &str, seat: Seat) -> Answer<()> {
        let game = self.host_of(user, game)?;
        if self.games[&game].game.get_player_by_seat(seat).is_none() {
            return Err(CallError::Refused(ErrorCode::InvalidGameEvent, format!("There is no seat {seat}")));
        }
        self.games.get_mut(&game).expect("checked").participation.open_or_close_seat(seat);
        self.connections.notify(&game, Notification::OpenOrCloseSeats { seats: vec![seat] });
        self.changed(&game);
        Ok(())
    }

    /// The settings the host is choosing before the game starts.
    pub(super) fn update_settings(&mut self, user: UserId, game: &str, settings: &serde_json::Value) -> Answer<()> {
        let game = self.host_of(user, game)?;
        let settings: GameSettings = serde_json::from_value(settings.clone()).map_err(|e| CallError::Refused(ErrorCode::BadCall, format!("settings: {e}")))?;
        let g = self.games.get_mut(&game).expect("checked");
        let json = serde_json::to_value(&settings).expect("settings serialize to JSON");
        g.game.rules.settings = Some(settings);
        self.connections.notify(&game, Notification::UpdateSettings { settings: json });
        self.changed(&game);
        Ok(())
    }

    /// Removes everyone from a game and forgets it.
    pub(super) async fn end_game(&mut self, game: &str) -> Result<(), CallError> {
        let Some(g) = self.games.get_mut(game) else { return Ok(()) };
        let users: Vec<UserId> = g.participation.player_names.keys().copied().collect();
        for user in users {
            let g = self.games.get_mut(game).expect("present");
            g.participation.remove_user(&mut g.game, user, true);
            self.connections.notify(game, Notification::RemoveUser { user, kick: true });
        }
        self.games.remove(game);
        self.connections.drop_game(game);
        self.services.db.delete_game(game).await?;
        Ok(())
    }

    pub(super) async fn close_game(&mut self, user: UserId, game: &str) -> Answer<Lobby> {
        let game = self.running(game)?;
        if self.games[&game].creator != user {
            return Err(refuse(ErrorCode::NoCreator));
        }
        self.end_game(&game).await?;
        Ok(self.lobby_status(GameListScope::Active, user))
    }

    pub(super) fn set_skin(&mut self, user: UserId, game: &str, skin: String) -> Answer<()> {
        let game = self.host_of(user, game)?;
        self.games.get_mut(&game).expect("checked").skin = Some(skin.clone());
        self.connections.notify(&game, Notification::SetSkin { skin });
        Ok(())
    }

    pub(super) fn set_timer(&mut self, user: UserId, game: &str, value: i32) -> Answer<()> {
        let game = self.host_of(user, game)?;
        self.games.get_mut(&game).expect("checked").timer = value;
        self.connections.notify(&game, Notification::SetTimer { value });
        Ok(())
    }

    /// The game's members hear of the new speed. A speed is 0 (paused) to [`MAX_BOT_SPEED`]: a
    /// faster one would have the bots take the game thread from every other game.
    pub(super) fn set_bot_speed(&mut self, user: UserId, game: &str, speed: i32) -> Answer<()> {
        let game = self.host_of(user, game)?;
        if !(0..=MAX_BOT_SPEED).contains(&speed) {
            return Err(CallError::Refused(ErrorCode::InvalidGameEvent, format!("The bot speed must be 0 to {MAX_BOT_SPEED}")));
        }
        self.games.get_mut(&game).expect("checked").participation.bots_speed = speed;
        self.connections.notify(&game, Notification::BotSpeed { speed });
        self.changed(&game);
        self.schedule_bots(&game);
        Ok(())
    }

    pub(super) async fn undo(&mut self, user: UserId, game: &str, until: i32) -> Answer<()> {
        let game = self.host_of(user, game)?;
        if until < 0 {
            return Err(CallError::Refused(ErrorCode::InvalidGameEvent, format!("can't undo to event {until}")));
        }
        self.games.get_mut(&game).expect("checked").game.undo(until);
        self.after_undo(&game);
        Ok(())
    }

    pub(super) async fn restore_undone(&mut self, user: UserId, game: &str) -> Answer<()> {
        let game = self.host_of(user, game)?;
        self.games.get_mut(&game).expect("checked").game.restore_recently_undone();
        self.after_undo(&game);
        Ok(())
    }

    pub(super) async fn dismiss_undone(&mut self, user: UserId, game: &str) -> Answer<()> {
        let game = self.host_of(user, game)?;
        self.games.get_mut(&game).expect("checked").game.dismiss_recently_undone();
        self.after_undo(&game);
        Ok(())
    }

    fn after_undo(&mut self, game: &str) {
        self.connections.game_replaced(game);
        self.suspend_pause(game);
        let now = self.now();
        self.games.get_mut(game).expect("checked").last_activity = now;
        self.changed(game);
        self.schedule_bots(game);
    }
}
