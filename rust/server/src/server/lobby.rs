//! The lobby: who is around, the running and scheduled games, and chat.

use treachery_engine::enumerations::{Faction, Phase, Ruleset};
use treachery_engine::enum_type::name_or_absent;
use treachery_engine::game::Game;

use super::{refuse, Answer, CallError, Presence, RunningGame, Server, MAX_SCHEDULED_GAMES_PER_USER};
use crate::db::ScheduledGameRow;
use crate::ids;
use crate::time::Time;
use crate::protocol::{enum_name, enum_value, ErrorCode, Notification};
use treachery_protocol::{
    AvailableSeat, ChatMessage, GameInfo, GameListScope, GlobalChatMessage, Lobby, ScheduledGameInfo, SeenUser, ServerInfo, Subscriber, SubscriptionType, UserStatus,
};

/// Users seen this recently are listed as around.
const SEEN_SECONDS: i64 = 60;
/// Scheduled games stay listed this long after their start.
const SCHEDULED_GAME_HOURS: i64 = 4;

impl Server {
    pub(super) fn server_info(&self) -> ServerInfo {
        ServerInfo {
            scheduled_maintenance: self.maintenance.unwrap_or_default(),
            admin_name: self.services.settings.admin.clone().unwrap_or_else(|| "admin".into()),
        }
    }

    pub(super) fn heartbeat(&mut self, user: i32, scope: GameListScope) -> Lobby {
        let now = self.now();
        self.presence.entry(user).or_insert(Presence { status: UserStatus::Online, last_seen: None }).last_seen = Some(now);
        self.lobby_status(scope, user)
    }

    /// The lobby as `user` sees it. `Active` lists games with recent activity and the user's
    /// own; `All` every game; `None` no games.
    pub(super) fn lobby_status(&self, scope: GameListScope, user: i32) -> Lobby {
        let now = self.now();
        let recently_seen_users = self
            .presence
            .iter()
            .filter_map(|(id, p)| {
                let last_seen = p.last_seen.filter(|t| now - *t < chrono::Duration::seconds(SEEN_SECONDS))?;
                Some(SeenUser { user_id: *id, name: self.player_name(*id), status: p.status, last_seen })
            })
            .collect();
        if scope == GameListScope::None {
            return Lobby { recently_seen_users, ..Lobby::default() };
        }
        let infos: Vec<GameInfo> = self.games.values().map(|g| self.game_info(g)).collect();
        let threshold = now - chrono::Duration::minutes(super::ACTIVE_GAME_MINUTES);
        let running_games = infos
            .iter()
            .filter(|g| scope == GameListScope::All || g.last_activity >= threshold || g.seated_players.iter().any(|(u, _)| *u == user))
            .cloned()
            .collect();
        Lobby {
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
                starts: g.starts,
                creator_id: g.creator_id,
                creator_name: g.creator_player_name.clone(),
                players: g.players,
                max_turns: g.max_turns,
                ruleset: g.ruleset.map(|r| Ruleset::INFO.format(r)),
                async_play: g.async_play,
                allowed_factions: g.factions.iter().map(|f| name_or_absent(treachery_engine::enum_type::from_value_or_absent::<Faction>(*f).ok().flatten())).collect(),
                subscribers: g
                    .subscriptions
                    .iter()
                    .filter_map(|(u, s)| {
                        Some(Subscriber {
                            user_id: *u,
                            user_name: self.users.get(u).map(|u| u.name.clone()).unwrap_or_else(|| "?".into()),
                            subscription: SubscriptionType::from_stored(*s)?,
                        })
                    })
                    .collect(),
            })
            .collect()
    }

    /// A game as the lobby lists it. Before the game starts, the factions and ruleset are those
    /// of its settings; after, those of its players.
    pub(super) fn game_info(&self, g: &RunningGame) -> GameInfo {
        let game = &g.game;
        let before_start = game.turn.current_phase <= Phase::AwaitingPlayers;
        let (factions_in_play, ruleset, max_players, max_turns) = match &game.rules.settings {
            Some(settings) => {
                let allowed: Vec<Faction> = settings.allowed_factions_in_play.clone().unwrap_or_default();
                let (factions, ruleset) = if before_start {
                    let rules = settings.initial_rules.as_ref().expect("initial rules");
                    (allowed.clone(), Game::determine_approximate_ruleset(&allowed, rules, Game::EXPANSION_LEVEL))
                } else {
                    let all: Vec<Faction> = game.players.iter().filter_map(|p| p.faction).collect();
                    (all.clone(), Game::determine_approximate_ruleset(&all, &game.rules.active, Game::EXPANSION_LEVEL))
                };
                (factions, Some(ruleset), settings.number_of_players, settings.maximum_turns)
            }
            None => (vec![], None, 0, 0),
        };
        let participation = &g.participation;
        GameInfo {
            game_id: g.id.clone(),
            name: g.name.clone(),
            creator_id: g.creator,
            creator_user_name: self.users.get(&g.creator).map(|u| u.name.clone()).unwrap_or_else(|| "?".into()),
            created: g.created,
            last_activity: g.last_activity,
            has_password: g.password_hash.as_deref().is_some_and(|p| !p.is_empty()),
            max_players,
            max_turns,
            main_phase: enum_name(game.turn.current_main_phase),
            phase: enum_name(game.turn.current_phase),
            turn: game.turn.current_turn,
            bots: game.number_of_bots(),
            players: participation.seated_players.len() as i32,
            factions_in_play: factions_in_play.iter().map(|f| enum_name(*f)).collect(),
            ruleset: treachery_engine::enum_type::name_or_absent(ruleset),
            seated_players: participation.seated_players.iter().map(|(u, s)| (*u, *s)).collect(),
            available_seats: game
                .players
                .iter()
                .filter(|p| participation.seat_is_available(p.seat))
                .map(|p| AvailableSeat { seat: p.seat, faction: name_or_absent(p.faction), is_bot: p.is_bot(game) })
                .collect(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) async fn schedule_game(
        &mut self,
        user: i32,
        starts: Time,
        ruleset: Option<&str>,
        players: i32,
        max_turns: i32,
        factions: &[String],
        async_play: bool,
    ) -> Answer<Lobby> {
        if self.scheduled.values().filter(|g| g.creator_id == user).count() >= MAX_SCHEDULED_GAMES_PER_USER {
            let message = format!("You cannot schedule more than {MAX_SCHEDULED_GAMES_PER_USER} games, delete a game first");
            return Err(CallError::Refused(ErrorCode::TooManyScheduledGames, message));
        }
        let unknown = |name: &str| CallError::Refused(ErrorCode::BadCall, format!("unknown faction or ruleset {name}"));
        let ruleset = match ruleset {
            Some(name) => Some(Ruleset::INFO.declared.iter().find(|(n, _)| *n == name).map(|(_, v)| *v).ok_or_else(|| unknown(name))?),
            None => None,
        };
        let factions = factions.iter().map(|name| enum_value::<Faction>(name).map(|f| f as i32).ok_or_else(|| unknown(name))).collect::<Result<Vec<i32>, _>>()?;
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
            subscriptions: vec![(user, SubscriptionType::CertainAsHost.stored())],
        };
        self.services.db.save_scheduled_game(&game).await?;
        self.scheduled.insert(game.id.clone(), game);
        Ok(self.lobby_status(GameListScope::Active, user))
    }

    pub(super) async fn cancel_scheduled_game(&mut self, user: i32, id: &str) -> Answer<Lobby> {
        let Some(game) = self.scheduled.get(id) else {
            return Err(refuse(ErrorCode::ScheduledGameNotFound));
        };
        if game.creator_id != user {
            return Err(refuse(ErrorCode::NoHost));
        }
        self.services.db.delete_scheduled_game(id).await?;
        self.scheduled.remove(id);
        Ok(self.lobby_status(GameListScope::Active, user))
    }

    pub(super) async fn subscribe(&mut self, user: i32, id: &str, subscription: SubscriptionType) -> Answer<Lobby> {
        let Some(game) = self.scheduled.get_mut(id) else {
            return Err(refuse(ErrorCode::GameNotFound));
        };
        match game.subscriptions.iter_mut().find(|(u, _)| *u == user) {
            Some(s) => s.1 = subscription.stored(),
            None => game.subscriptions.push((user, subscription.stored())),
        }
        let game = game.clone();
        self.services.db.save_scheduled_game(&game).await?;
        Ok(self.lobby_status(GameListScope::Active, user))
    }

    /// A player's message to everyone in the game, or to one other user in it. A message to
    /// one user goes only to that user and the sender.
    pub(super) fn chat(&mut self, user: i32, game: &str, to: Option<i32>, body: String) -> Answer<()> {
        let game = self.player_of(user, game)?;
        let message = ChatMessage { from: user, to, body };
        match to {
            None => self.connections.notify(&game, Notification::ChatMessage(message)),
            Some(target) => {
                if !self.games[&game].participation.is_participant(target) {
                    return Err(refuse(ErrorCode::UserNotInGame));
                }
                self.connections.notify_users(&game, &[user, target], Notification::ChatMessage(message));
            }
        }
        Ok(())
    }

    pub(super) fn global_chat(&mut self, user: i32, body: String) {
        self.connections.notify_all(Notification::GlobalChatMessage(GlobalChatMessage { from: user, body }));
    }
}
