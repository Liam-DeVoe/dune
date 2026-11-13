//! Creating, loading and closing games; joining, observing and leaving them; seats, hosts and
//! the host's other controls.

use treachery_engine::shared::Shared;
use treachery_engine::enumerations::{ErrorType, GameListScope};
use treachery_engine::game::Game;
use treachery_engine::game::GameSettings;
use treachery_engine::game::GameState;
use treachery_engine::game::Participation;

use super::{contents, done, refuse, CallError, Reply, RunningGame, Server, MAX_BOT_SPEED, MAX_GAMES_PER_USER};
use crate::connections::{ConnectionId, Notification};
use crate::ids;
use crate::participation;
use crate::wire::GameInitInfo;

/// Loads a savegame into a game with the given participation, or says why it can't.
pub fn load_savegame(savegame: &str, participation: &Shared<Participation>, restoring: bool) -> Result<Game, String> {
    let state = GameState::load(savegame).map_err(|e| e.message)?;
    Game::load(&state, participation, false, restoring).map_err(|m| crate::render::message_to_string(&m))
}

fn refuse_savegame(reason: String) -> CallError {
    CallError::Refused(ErrorType::InvalidGameEvent, reason)
}

impl Server {
    fn init_info(&self, game: &str) -> GameInitInfo {
        let g = &self.games[game];
        GameInitInfo {
            game_id: g.id.clone(),
            game_name: g.name.clone(),
            participation: participation::to_wire(&g.game.participation.borrow()),
            phase_control: Some(g.pause.last_sent.clone().unwrap_or_default()),
        }
    }

    /// The connection follows the game, and gets its view after this call.
    fn follow(&mut self, game: &str, connection: ConnectionId, user: i32) {
        self.connections.follow(game, connection, user);
        self.connections.view_for(game, connection, user);
    }

    pub(super) async fn create_game(
        &mut self,
        connection: ConnectionId,
        user: i32,
        name: Option<&str>,
        password_hash: Option<&str>,
        savegame: Option<&str>,
        skin: Option<&str>,
    ) -> Reply {
        if self.games.values().filter(|g| g.creator == user).count() >= MAX_GAMES_PER_USER {
            return Err(refuse(ErrorType::TooManyGames));
        }
        let game = match savegame.filter(|s| !s.is_empty()) {
            Some(savegame) => load_savegame(savegame, &Shared::new(Participation::new()), false).map_err(refuse_savegame)?,
            None => Game::new(),
        };
        let player_name = self.player_name(user);
        let name = match name.filter(|n| !n.is_empty()) {
            Some(n) => n.to_string(),
            None => format!("{player_name}'s Game"),
        };
        let id = ids::new_id();
        let mut running = RunningGame::new(id.clone(), name, user, game, self.now());
        running.password_hash = password_hash.filter(|p| !p.is_empty()).map(str::to_string);
        running.skin = skin.filter(|s| !s.is_empty()).map(str::to_string);
        running.generation = self.new_generation();
        running.game.add_player(user, &player_name, -1);
        running.game.set_or_unset_host(user);
        self.games.insert(id.clone(), running);
        self.follow(&id, connection, user);
        if let Some(skin) = skin.filter(|s| !s.is_empty()) {
            self.connections.notify(&id, Notification::SetSkin(skin.to_string()));
        }
        self.changed(&id);
        let mut info = self.init_info(&id);
        info.phase_control = None;
        contents(&info)
    }

    pub(super) async fn load_game(&mut self, user: i32, game: &str, savegame: &str, skin: Option<&str>) -> Reply {
        let game = self.host_of(user, game)?;
        let generation = self.new_generation();
        let g = self.games.get_mut(&game).expect("checked");
        // The loaded game keeps this game's participation.
        let mut loaded = load_savegame(savegame, &g.game.participation, false).map_err(refuse_savegame)?;
        if loaded.seed != g.game.seed {
            loaded.reset_seats();
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
            self.connections.notify(&game, Notification::SetSkin(skin.to_string()));
        }
        self.changed(&game);
        self.schedule_bots(&game);
        done()
    }

    /// Seats the game's players. Only users who play in the game can be seated, each on a seat
    /// of the game's players or on none (-1), and no two on the same seat.
    pub(super) fn assign_seats(&mut self, user: i32, game: &str, seats: Vec<(i32, i32)>) -> Reply {
        let game = self.host_of(user, game)?;
        {
            let g = &self.games[&game].game;
            let bad = |reason: String| Err(CallError::Refused(ErrorType::InvalidGameEvent, reason));
            for (i, (user, seat)) in seats.iter().enumerate() {
                if !g.is_player(*user) {
                    return Err(refuse(ErrorType::UserNotInGame));
                }
                if *seat != -1 && !g.players.iter().any(|p| p.seat == *seat) {
                    return bad(format!("There is no seat {seat}"));
                }
                if seats[..i].iter().any(|(u, s)| u == user || (*s == *seat && *seat != -1)) {
                    return bad("Each user and each seat may be assigned once".into());
                }
            }
        }
        {
            let g = &self.games[&game].game;
            let mut participation = g.participation.borrow_mut();
            participation.seated_players = Default::default();
            for (user, seat) in &seats {
                participation.seated_players.insert(*user, *seat);
            }
        }
        self.connections.notify(&game, Notification::AssignSeats(seats));
        self.changed(&game);
        self.schedule_bots(&game);
        done()
    }

    /// A user who already plays in the game only follows it again. Others need the password, an
    /// open seat (or none in particular before the game starts) and not to have been kicked.
    /// Taking a seat that another user holds removes that user.
    pub(super) fn join_game(&mut self, connection: ConnectionId, user: i32, game: &str, password_hash: Option<&str>, seat: i32) -> Reply {
        let game = self.running(game)?;
        let player_name = self.player_name(user);
        let g = self.games.get_mut(&game).expect("checked");
        if !g.game.is_player(user) {
            if let Some(password) = g.password_hash.as_deref() {
                if password_hash != Some(password) {
                    return Err(refuse(ErrorType::IncorrectGamePassword));
                }
            }
            // A particular seat must be one the host opened; before the game starts, users join
            // without one.
            if !g.game.is_open(seat) || (seat >= 0 && !g.game.seat_is_available(seat)) {
                return Err(refuse(ErrorType::SeatNotAvailableNotOpen));
            }
            if g.game.was_kicked(user) {
                return Err(refuse(ErrorType::SeatNotAvailableKicked));
            }
            if seat >= 0 {
                let previous = g.game.user_id_in_seat(seat);
                if previous >= 0 {
                    g.game.remove_user(previous, false);
                    self.connections.notify(&game, Notification::RemoveUser { user: previous, kick: false });
                }
            }
            let g = self.games.get_mut(&game).expect("checked");
            g.game.add_player(user, &player_name, seat);
            let make_host = g.game.number_of_seated_players() == 1 || g.creator == user;
            if make_host && !g.game.is_host(Some(user)) {
                g.game.set_or_unset_host(user);
            }
            self.connections.notify(&game, Notification::JoinGame { user, name: player_name, seat });
            if make_host {
                self.connections.notify(&game, Notification::SetOrUnsetHost(user));
            }
        }
        self.follow(&game, connection, user);
        self.changed(&game);
        if self.games[&game].game.number_of_seated_players() == 1 {
            self.schedule_bots(&game);
        }
        contents(&self.init_info(&game))
    }

    pub(super) fn observe_game(&mut self, connection: ConnectionId, user: i32, game: &str, _password_hash: Option<&str>) -> Reply {
        let game = self.running(game)?;
        let player_name = self.player_name(user);
        let g = self.games.get_mut(&game).expect("checked");
        if !g.game.is_observer(Some(user)) {
            if g.game.is_player(user) {
                return Err(refuse(ErrorType::AlreadyPlayer));
            }
            // A kicked user doesn't come back to watch either.
            if g.game.was_kicked(user) {
                return Err(refuse(ErrorType::SeatNotAvailableKicked));
            }
            g.game.add_observer(user, &player_name);
            self.connections.notify(&game, Notification::ObserveGame { user, name: player_name });
        }
        self.follow(&game, connection, user);
        self.changed(&game);
        self.schedule_bots(&game);
        contents(&self.init_info(&game))
    }

    /// A new connection of a player or observer follows the game again.
    pub(super) fn reconnect_game(&mut self, connection: ConnectionId, user: i32, game: &str) -> Reply {
        let game = self.participant_of(user, game)?;
        self.follow(&game, connection, user);
        contents(&self.init_info(&game))
    }

    pub(super) fn game_state(&mut self, connection: ConnectionId, user: i32, game: &str) -> Reply {
        let game = self.participant_of(user, game)?;
        self.connections.view_for(&game, connection, user);
        contents(&self.init_info(&game))
    }

    pub(super) fn leave_game(&mut self, user: i32, game: &str) -> Reply {
        let game = self.participant_of(user, game)?;
        self.remove_from_game(&game, user);
        contents(&self.lobby_status(GameListScope::Active, user))
    }

    /// Takes a user out of a running game. When the last host leaves, every other seated
    /// player becomes a host.
    pub(super) fn remove_from_game(&mut self, game: &str, user: i32) {
        let game = game.to_string();
        let g = self.games.get_mut(&game).expect("running game");
        if g.game.is_host(Some(user)) && g.game.number_of_hosts() == 1 {
            let others: Vec<i32> = g.game.participation.borrow().seated_players.keys().copied().filter(|u| *u != user).collect();
            for other in others {
                let g = self.games.get_mut(&game).expect("checked");
                g.game.set_or_unset_host(other);
                self.connections.notify(&game, Notification::SetOrUnsetHost(other));
            }
        }
        self.games.get_mut(&game).expect("checked").game.remove_user(user, false);
        self.connections.notify(&game, Notification::RemoveUser { user, kick: false });
        self.connections.unfollow_user(&game, user);
        self.changed(&game);
        self.schedule_bots(&game);
    }

    /// Removes another user of the game for good.
    pub(super) fn kick(&mut self, host: i32, game: &str, user: i32) -> Reply {
        let game = self.host_of(host, game)?;
        if user == host || !self.games[&game].game.is_participant(user) {
            return Err(refuse(ErrorType::UserNotInGame));
        }
        self.games.get_mut(&game).expect("checked").game.remove_user(user, true);
        self.connections.notify(&game, Notification::RemoveUser { user, kick: true });
        self.connections.unfollow_user(&game, user);
        self.changed(&game);
        self.schedule_bots(&game);
        done()
    }

    /// Makes a player of the game a host, or no longer one.
    pub(super) fn set_or_unset_host(&mut self, host: i32, game: &str, user: i32) -> Reply {
        let game = self.host_of(host, game)?;
        let g = self.games.get_mut(&game).expect("checked");
        if !g.game.is_player(user) {
            return Err(refuse(ErrorType::UserNotInGame));
        }
        if user == host && g.game.number_of_hosts() <= 1 {
            return Err(refuse(ErrorType::CannotRemoveLastHost));
        }
        g.game.set_or_unset_host(user);
        self.connections.notify(&game, Notification::SetOrUnsetHost(user));
        self.changed(&game);
        done()
    }

    /// Opens or closes the seat of one of the game's players.
    pub(super) fn open_or_close_seat(&mut self, user: i32, game: &str, seat: i32) -> Reply {
        let game = self.host_of(user, game)?;
        if !self.games[&game].game.players.iter().any(|p| p.seat == seat) {
            return Err(CallError::Refused(ErrorType::InvalidGameEvent, format!("There is no seat {seat}")));
        }
        self.games.get_mut(&game).expect("checked").game.open_or_close_seat(seat);
        self.connections.notify(&game, Notification::OpenOrCloseSeats(vec![seat]));
        self.changed(&game);
        done()
    }

    /// The settings the host is choosing before the game starts.
    pub(super) fn update_settings(&mut self, user: i32, game: &str, settings: GameSettings) -> Reply {
        let game = self.host_of(user, game)?;
        let g = self.games.get_mut(&game).expect("checked");
        let json = serde_json::from_str(&treachery_engine::json::serialize(&settings)).expect("settings serialize to JSON");
        g.game.settings = Some(Shared::new(settings));
        self.connections.notify(&game, Notification::UpdateSettings(json));
        self.changed(&game);
        done()
    }

    /// Removes everyone from a game and forgets it.
    pub(super) async fn end_game(&mut self, game: &str) -> Result<(), CallError> {
        let Some(g) = self.games.get_mut(game) else { return Ok(()) };
        let users: Vec<i32> = g.game.participation.borrow().player_names.keys().copied().collect();
        for user in users {
            self.games.get_mut(game).expect("present").game.remove_user(user, true);
            self.connections.notify(game, Notification::RemoveUser { user, kick: true });
        }
        self.games.remove(game);
        self.connections.drop_game(game);
        self.services.db.delete_game(game).await?;
        Ok(())
    }

    pub(super) async fn close_game(&mut self, user: i32, game: &str) -> Reply {
        let game = self.running(game)?;
        if self.games[&game].creator != user {
            return Err(refuse(ErrorType::NoCreator));
        }
        self.end_game(&game).await?;
        contents(&self.lobby_status(GameListScope::Active, user))
    }

    pub(super) fn set_skin(&mut self, user: i32, game: &str, skin: String) -> Reply {
        let game = self.host_of(user, game)?;
        self.games.get_mut(&game).expect("checked").skin = Some(skin.clone());
        self.connections.notify(&game, Notification::SetSkin(skin));
        done()
    }

    pub(super) fn set_timer(&mut self, user: i32, game: &str, value: i32) -> Reply {
        let game = self.host_of(user, game)?;
        self.games.get_mut(&game).expect("checked").timer = value;
        self.connections.notify(&game, Notification::SetTimer(value));
        done()
    }

    /// The game's members hear of the new speed. A speed is 0 (paused) to [`MAX_BOT_SPEED`]: a
    /// faster one would have the bots take the game thread from every other game.
    pub(super) fn set_bot_speed(&mut self, user: i32, game: &str, speed: i32) -> Reply {
        let game = self.host_of(user, game)?;
        if !(0..=MAX_BOT_SPEED).contains(&speed) {
            return Err(CallError::Refused(ErrorType::InvalidGameEvent, format!("The bot speed must be 0 to {MAX_BOT_SPEED}")));
        }
        self.games[&game].game.participation.borrow_mut().bots_speed = speed;
        self.connections.notify(&game, Notification::BotSpeed(speed));
        self.changed(&game);
        self.schedule_bots(&game);
        done()
    }

    pub(super) async fn undo(&mut self, user: i32, game: &str, until: i32) -> Reply {
        let game = self.host_of(user, game)?;
        if until < 0 {
            return Err(CallError::Refused(ErrorType::InvalidGameEvent, format!("can't undo to event {until}")));
        }
        self.games.get_mut(&game).expect("checked").game.undo(until);
        self.after_undo(&game);
        done()
    }

    pub(super) async fn restore_undone(&mut self, user: i32, game: &str) -> Reply {
        let game = self.host_of(user, game)?;
        self.games.get_mut(&game).expect("checked").game.restore_recently_undone();
        self.after_undo(&game);
        done()
    }

    pub(super) async fn dismiss_undone(&mut self, user: i32, game: &str) -> Reply {
        let game = self.host_of(user, game)?;
        self.games.get_mut(&game).expect("checked").game.dismiss_recently_undone();
        self.after_undo(&game);
        done()
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
