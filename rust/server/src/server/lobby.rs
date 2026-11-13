//! The lobby: who is around, the running and scheduled games, and chat.

use treachery_engine::compat::cs::NullRef;
use treachery_engine::enumerations::{ErrorType, Faction, GameListScope, Phase, SubscriptionType, UserStatus};
use treachery_engine::game::Game;

use super::{contents, done, refuse, Presence, Reply, RunningGame, Server, MAX_SCHEDULED_GAMES_PER_USER};
use crate::connections::Notification;
use crate::db::ScheduledGameRow;
use crate::ids;
use crate::time::Time;
use crate::wire::{AvailableSeat, GameChatMessage, GameInfo, GlobalChatMessage, ScheduledGameInfo, SeenUser, ServerInfo, ServerStatus};

/// Users seen this recently are listed as around.
const SEEN_SECONDS: i64 = 60;
/// Scheduled games stay listed this long after their start.
const SCHEDULED_GAME_HOURS: i64 = 4;

impl Server {
    pub(super) fn server_info(&self) -> Reply {
        contents(&ServerInfo {
            scheduled_maintenance: self.maintenance.unwrap_or_default(),
            admin_name: self.services.settings.admin.clone().unwrap_or_else(|| "admin".into()),
        })
    }

    pub(super) fn heartbeat(&mut self, token: &str, scope: GameListScope) -> Reply {
        let user = self.session_user(token)?;
        let now = self.now();
        self.presence.entry(user).or_insert(Presence { status: UserStatus::Online, last_seen: None }).last_seen = Some(now);
        contents(&self.lobby_status(scope, user))
    }

    /// The lobby as `user` sees it. `Active` lists games with recent activity and the user's
    /// own; `All` every game; `None` no games.
    pub(super) fn lobby_status(&self, scope: GameListScope, user: i32) -> ServerStatus {
        let now = self.now();
        let recently_seen_users = self
            .presence
            .iter()
            .filter_map(|(id, p)| {
                let last_seen = p.last_seen.filter(|t| now - *t < chrono::Duration::seconds(SEEN_SECONDS))?;
                Some(SeenUser { id: *id, name: self.player_name(*id), status: p.status.0, last_seen })
            })
            .collect();
        if scope != GameListScope::Active && scope != GameListScope::All {
            return ServerStatus { recently_seen_users, ..ServerStatus::default() };
        }
        let infos: Vec<GameInfo> = self.games.values().map(|g| self.game_info(g)).collect();
        let threshold = now - chrono::Duration::minutes(super::ACTIVE_GAME_MINUTES);
        let running_games = infos
            .iter()
            .filter(|g| scope == GameListScope::All || g.last_activity >= threshold || g.seated_players.iter().any(|(u, _)| *u == user))
            .cloned()
            .collect();
        ServerStatus {
            running_games,
            own_games: infos.into_iter().filter(|g| g.creator_id == user).collect(),
            scheduled_games: self.scheduled_games(now),
            recently_seen_users,
        }
    }

    fn scheduled_games(&self, now: Time) -> Vec<ScheduledGameInfo> {
        let mut games: Vec<&ScheduledGameRow> =
            self.scheduled.values().filter(|g| g.starts + chrono::Duration::hours(SCHEDULED_GAME_HOURS) >= now).collect();
        games.sort_by_key(|g| g.starts);
        games
            .into_iter()
            .map(|g| ScheduledGameInfo {
                scheduled_game_id: g.id.clone(),
                date_time: g.starts,
                creator_user_id: g.creator_id,
                creator_name: g.creator_player_name.clone(),
                number_of_players: g.players,
                maximum_turns: g.max_turns,
                ruleset: g.ruleset,
                async_play: g.async_play,
                allowed_factions_in_play: g.factions.clone(),
                subscribers: g.subscriptions.clone(),
                subscriber_names: g
                    .subscriptions
                    .iter()
                    .map(|(u, _)| (*u, self.users.get(u).map(|u| u.name.clone()).unwrap_or_else(|| "?".into())))
                    .collect(),
            })
            .collect()
    }

    /// A game as the lobby lists it. Before the game starts, the factions and ruleset are those
    /// of its settings; after, those of its players.
    pub(super) fn game_info(&self, g: &RunningGame) -> GameInfo {
        let game = &g.game;
        let before_start = game.current_phase <= Phase::AwaitingPlayers;
        let (factions_in_play, ruleset, max_players, max_turns) = match &game.settings {
            Some(settings) => {
                let settings = settings.borrow();
                let allowed: Vec<Faction> = settings.allowed_factions_in_play.as_ref().map(|f| f.borrow().clone()).unwrap_or_default();
                let (factions, ruleset) = if before_start {
                    let rules = settings.initial_rules.as_ref().nre();
                    (allowed.clone(), Game::determine_approximate_ruleset(&allowed, rules, Game::EXPANSION_LEVEL))
                } else {
                    let all: Vec<Faction> = game.players.iter().map(|p| p.faction).collect();
                    (all.iter().copied().filter(|f| *f != Faction::None).collect(), Game::determine_approximate_ruleset(&all, &game.rules, Game::EXPANSION_LEVEL))
                };
                (factions, ruleset, settings.number_of_players, settings.maximum_turns)
            }
            None => (vec![], Default::default(), 0, 0),
        };
        let participation = game.participation.borrow();
        GameInfo {
            creator_id: g.creator,
            creator_username: self.users.get(&g.creator).map(|u| u.name.clone()).unwrap_or_else(|| "?".into()),
            creation_date: g.created,
            game_id: g.id.clone(),
            has_password: g.password_hash.as_deref().is_some_and(|p| !p.is_empty()),
            name: g.name.clone(),
            max_players,
            max_turns,
            main_phase: game.current_main_phase.0,
            phase: game.current_phase.0,
            turn: game.current_turn,
            nr_of_bots: game.number_of_bots(),
            nr_of_players: participation.seated_players.len() as i32,
            factions_in_play: factions_in_play.iter().map(|f| f.0).collect(),
            ruleset: ruleset.0,
            last_activity: g.last_activity,
            seated_players: participation.seated_players.iter().map(|(u, s)| (*u, *s)).collect(),
            available_seats: game
                .players
                .iter()
                .filter(|p| game.seat_is_available(p.seat))
                .map(|p| AvailableSeat { seat: p.seat, faction: p.faction.0, is_bot: p.is_bot(game) })
                .collect(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) async fn schedule_game(&mut self, token: &str, starts: Time, ruleset: Option<i32>, players: i32, max_turns: i32, factions: Vec<i32>, async_play: bool) -> Reply {
        let user = self.session_user(token)?;
        if self.scheduled.values().filter(|g| g.creator_id == user).count() >= MAX_SCHEDULED_GAMES_PER_USER {
            return Err(refuse(ErrorType::TooManyScheduledGames));
        }
        let game = ScheduledGameRow {
            id: ids::new_id(),
            creator_id: user,
            creator_player_name: self.player_name(user),
            starts,
            ruleset,
            players,
            max_turns,
            factions,
            async_play,
            subscriptions: vec![(user, SubscriptionType::CertainAsHost.0)],
        };
        self.services.db.save_scheduled_game(&game).await?;
        self.scheduled.insert(game.id.clone(), game);
        contents(&self.lobby_status(GameListScope::Active, user))
    }

    pub(super) async fn cancel_scheduled_game(&mut self, token: &str, id: &str) -> Reply {
        let user = self.session_user(token)?;
        let Some(game) = self.scheduled.get(id) else {
            return Err(refuse(ErrorType::ScheduledGameNotFound));
        };
        if game.creator_id != user {
            return Err(refuse(ErrorType::NoHost));
        }
        self.services.db.delete_scheduled_game(id).await?;
        self.scheduled.remove(id);
        contents(&self.lobby_status(GameListScope::Active, user))
    }

    pub(super) async fn subscribe(&mut self, token: &str, id: Option<&str>, subscription: SubscriptionType) -> Reply {
        let id = id.ok_or(refuse(ErrorType::GameNotFound))?;
        let user = self.session_user(token)?;
        let Some(game) = self.scheduled.get_mut(id) else {
            return Err(refuse(ErrorType::GameNotFound));
        };
        match game.subscriptions.iter_mut().find(|(u, _)| *u == user) {
            Some(s) => s.1 = subscription.0,
            None => game.subscriptions.push((user, subscription.0)),
        }
        let game = game.clone();
        self.services.db.save_scheduled_game(&game).await?;
        contents(&self.lobby_status(GameListScope::Active, user))
    }

    /// Relayed as sent to everyone following the game; showing only what is meant for a player
    /// is the client's job.
    pub(super) fn chat(&mut self, token: &str, game: &str, message: GameChatMessage) -> Reply {
        let (_, game) = self.user_in_game(token, game)?;
        self.connections.notify(&game, Notification::ChatMessage(message));
        done()
    }

    pub(super) fn global_chat(&mut self, token: &str, message: GlobalChatMessage) -> Reply {
        self.session_user(token)?;
        self.connections.notify_all(Notification::GlobalChatMessage(message));
        done()
    }
}
