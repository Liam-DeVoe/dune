//! The objects of the lobby: accounts, game lists, scheduled games, a game's participation.
//!
//! The engine's enums (`Faction`, `Ruleset`, `Phase`, `MainPhase`) are sent by member name, the
//! names the game view and the skin use. They are strings here, typed as the enum's union in
//! TypeScript (`web/src/generated/enums.ts`).

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use treachery_engine::game::Seat;

use crate::ids::UserId;
use crate::time::{optional_time, serde_time, Time};

/// A user's own account.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub user_id: UserId,
    pub user_name: String,
    pub player_name: String,
    pub email: String,
}

/// The reply to the calls that start a session: the session's token, which the server sends
/// only this once, and the account. The connection is now authenticated with it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct NewSession {
    pub token: String,
    pub account: Account,
}

/// The reply to `linkLogin`: the new session, and the game the link opens if the user is
/// still in it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LinkLoginInfo {
    pub session: NewSession,
    pub game_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ServerInfo {
    /// When the admin announced maintenance; the start of 0001-01-01 when they didn't.
    #[serde(with = "serde_time")]
    #[ts(type = "string")]
    pub scheduled_maintenance: Time,
    pub admin_name: String,
}

/// A game in the lobby.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct GameInfo {
    pub game_id: String,
    pub name: String,
    pub creator_id: UserId,
    pub creator_user_name: String,
    #[serde(with = "serde_time")]
    #[ts(type = "string")]
    pub created: Time,
    #[serde(with = "serde_time")]
    #[ts(type = "string")]
    pub last_activity: Time,
    pub has_password: bool,
    pub max_players: i32,
    pub max_turns: i32,
    #[ts(type = "MainPhase")]
    pub main_phase: String,
    #[ts(type = "Phase")]
    pub phase: String,
    pub turn: i32,
    pub bots: i32,
    /// Users seated in the game.
    pub players: i32,
    /// Before the game starts, the factions its settings allow; after, those in play.
    #[ts(type = "Array<Faction>")]
    pub factions_in_play: Vec<String>,
    #[ts(type = "Ruleset")]
    pub ruleset: String,
    /// `[userId, seat]` of each user seated, -1 for a user without a seat yet.
    #[serde(with = "crate::ids::seated_users")]
    #[ts(type = "Array<[number, number]>")]
    pub seated_players: Vec<(UserId, Option<Seat>)>,
    /// The seats a host opened for others to join.
    pub available_seats: Vec<AvailableSeat>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AvailableSeat {
    pub seat: Seat,
    #[ts(type = "Faction")]
    pub faction: String,
    pub is_bot: bool,
}

/// A game announced for a later time.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ScheduledGameInfo {
    pub scheduled_game_id: String,
    #[serde(with = "serde_time")]
    #[ts(type = "string")]
    pub starts: Time,
    pub creator_id: UserId,
    pub creator_name: String,
    pub players: i32,
    pub max_turns: i32,
    #[ts(type = "Ruleset | null")]
    pub ruleset: Option<String>,
    pub async_play: bool,
    #[ts(type = "Array<Faction>")]
    pub allowed_factions: Vec<String>,
    pub subscribers: Vec<Subscriber>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Subscriber {
    pub user_id: UserId,
    pub user_name: String,
    pub subscription: SubscriptionType,
}

/// Whether and how a user means to take part in a scheduled game.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub enum SubscriptionType {
    DontParticipate,
    MaybeAsPlayer,
    CertainAsPlayer,
    MaybeAsHost,
    CertainAsHost,
}

impl SubscriptionType {
    const STORED: [(SubscriptionType, i32); 5] = [
        (SubscriptionType::DontParticipate, 10),
        (SubscriptionType::MaybeAsPlayer, 20),
        (SubscriptionType::CertainAsPlayer, 30),
        (SubscriptionType::MaybeAsHost, 40),
        (SubscriptionType::CertainAsHost, 50),
    ];

    /// The number the database keeps. Existing databases hold these numbers, so they can't change.
    pub fn stored(self) -> i32 {
        Self::STORED.iter().find(|(s, _)| *s == self).map(|(_, n)| *n).expect("every type is stored")
    }

    /// The type a stored number stands for. None for 0, which databases hold for "none".
    pub fn from_stored(n: i32) -> Option<SubscriptionType> {
        Self::STORED.iter().find(|(_, v)| *v == n).map(|(s, _)| *s)
    }
}

/// What a user tells the lobby about themselves.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
pub enum UserStatus {
    #[default]
    Online,
    Away,
    /// Looking for a game.
    Lfg,
    /// Looking for players.
    Lfm,
    InGame,
}

/// A user seen in the last minute.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SeenUser {
    pub user_id: UserId,
    pub name: String,
    pub status: UserStatus,
    #[serde(with = "serde_time")]
    #[ts(type = "string")]
    pub last_seen: Time,
}

/// Which games a lobby reply lists.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub enum GameListScope {
    /// No games: a client in a game only keeps itself on the user list.
    None,
    /// Games with activity in the last two hours, and the caller's own.
    Active,
    /// Every game, for the admin.
    All,
}

/// The lobby.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Lobby {
    pub running_games: Vec<GameInfo>,
    /// The games the caller created.
    pub own_games: Vec<GameInfo>,
    pub scheduled_games: Vec<ScheduledGameInfo>,
    pub recently_seen_users: Vec<SeenUser>,
}

/// Who plays which seat, observes or hosts, in the order they came.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ParticipationInfo {
    /// `[userId, name]` of every player and observer.
    pub player_names: Vec<(UserId, String)>,
    /// `[userId, seat]` of every player, -1 for one without a seat yet.
    #[serde(with = "crate::ids::seated_users")]
    #[ts(type = "Array<[number, number]>")]
    pub seated_players: Vec<(UserId, Option<Seat>)>,
    pub observers: Vec<UserId>,
    pub hosts: Vec<UserId>,
    /// The seats a host opened for others to join.
    pub available_seats: Vec<Seat>,
    pub kicked: Vec<UserId>,
    /// 0 pauses the bots; 1, 2 and 4 divide their delays.
    pub bots_speed: i32,
}

/// What the current pause between phases waits for (protocol.md, Playing).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PhaseControlInfo {
    /// False: the host alone continues this pause.
    pub active: bool,
    /// Seats.
    pub waiting_for: Vec<Seat>,
    pub countdown: bool,
    /// Set while the countdown runs and nobody holds it.
    #[serde(with = "optional_time")]
    #[ts(type = "string | null")]
    pub countdown_ends: Option<Time>,
    /// Seats.
    pub holding: Vec<Seat>,
    /// Why the pause waits for each seat in `waiting_for`, in the same order. Every reason is
    /// something everyone can already see, so it goes to everyone.
    pub reasons: Vec<PauseWait>,
}

/// What the pause waits for one seat to do.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PauseWait {
    pub seat: Seat,
    /// One or more things the seat may do here; the client joins them with "or".
    pub reasons: Vec<PauseReason>,
}

/// Something a player may do in a pause between phases (`rust/server/src/pause_rules.rs`). Some
/// need a secret card or spice; the pause waits regardless, so the reason says what the player may
/// do, never what they hold.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum PauseReason {
    ChooseFaction,
    TradeFactions,
    EnterDiscovery,
    UseTestingStation,
    /// Fremen's once-per-game Karama sandworm.
    KaramaSandworm,
    /// Swap the card just won, as Ixian's ally or with the Ixian Nexus.
    ReplaceCardWon,
    /// Atreides looks at the next card before its auction.
    LookAtNextCard,
    SetRevivalLimits,
    /// Kaitain at high threshold: discard cards for 2 spice each.
    EmperorDiscard,
    /// Tleilaxu's once-per-game Karama, which stops a revival.
    KaramaStopRevival,
    Revive,
    PlaceAmbassador,
    RevealNoField,
    RemoveTerror,
    PlantTerror,
    /// CHOAM's Inflation token.
    Inflation,
    TripToGamont,
    ChoamDiscard,
    /// CHOAM and its ally trading a card.
    TradeCard,
}

/// The reply to creating, joining, observing or reconnecting to a game, and the `loadGame`
/// notification. The game itself arrives as a `gameView` notification.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct GameInitInfo {
    pub game_id: String,
    pub game_name: String,
    pub participation: ParticipationInfo,
    /// Set in the replies to join, observe, reconnect and game state.
    pub phase_control: Option<PhaseControlInfo>,
}

/// What the admin page shows.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AdminInfo {
    pub users: Vec<AdminUser>,
    pub sessions: i32,
    /// Users with a connection that follows a game.
    pub users_in_games: i32,
    pub running_games: i32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AdminUser {
    pub user_id: UserId,
    pub user_name: String,
    pub email: String,
    #[serde(with = "serde_time")]
    #[ts(type = "string")]
    pub last_login: Time,
    pub player_name: String,
}

/// A chat message in a game, as the members it is for receive it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessage {
    pub from: UserId,
    /// The one user the message is for; null for everyone in the game.
    pub to: Option<UserId>,
    pub body: String,
}

/// A chat message to everyone connected.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct GlobalChatMessage {
    pub from: UserId,
    pub body: String,
}
