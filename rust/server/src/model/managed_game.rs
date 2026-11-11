//! Port of `Model/ManagedGame.cs`: a running game and the server's data about it.

use treachery_bots::ClassicBot;
use treachery_engine::date_time_offset::DateTimeOffset;
use treachery_engine::enumerations::Faction;
use treachery_engine::game::Game;

use super::game_info::SeatedPlayersRef;

pub struct ManagedGame {
    pub creation_date: DateTimeOffset,
    pub last_activity: DateTimeOffset,
    pub last_persisted: DateTimeOffset,
    pub creator_user_id: i32,
    pub game_id: String,
    /// Replaced by undo, restore, dismiss and load (`game.Game = ...`).
    pub game: Game,
    /// C# `string Name { get; init; } = string.Empty`, but the restore path assigns the
    /// nullable `PersistedGame.GameName` to it, so it can be null.
    pub name: Option<String>,
    /// C# `= string.Empty`, but `RequestCreateGame` and the restore path can store null.
    pub hashed_password: Option<String>,
    pub observers_require_password: bool,
    pub statistics_sent: bool,
    pub last_async_play_message_sent: DateTimeOffset,
    /// Rust only: the reference that `GameInfo`s extracted from this game share for its current
    /// `Participation.SeatedPlayers` dictionary object (C# shares the object itself). `None`
    /// until `ExtractGameInfo` first needs it; `RequestAssignSeats` detaches it and clears it.
    pub seated_players_ref: Option<SeatedPlayersRef>,
    /// `Dictionary<Faction, IBot> Bots` (ManagedGame.cs): one bot per faction, created when a bot
    /// of that faction is first asked (`GetOrInitializeBot`) and kept across events and undo. Only
    /// `TryGetValue` and `Add` are used, so a list in insertion order serves.
    pub bots: Vec<(Faction, ClassicBot)>,
}

// C# parity: `ProcessEventAsync` (ManagedGame.cs:43) serializes event processing per game with
// a semaphore. The Rust hub runs every call to completion before the next one starts
// (server-plan.md, "Concurrency"), which already gives that guarantee, so there is no lock here.
