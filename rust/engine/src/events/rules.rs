//! What every event has in common: who submitted it ([`By`]), the rules it follows
//! ([`EventRules`]), its place in the history ([`HistoryEntry`]), and why a submission failed
//! ([`SubmitError`]).

use crate::prelude::*;

/// Who submitted an event: the faction (`None` for the host, or a player before faction
/// selection), and that faction's player when the event was submitted.
///
/// The player is looked up once, before validation, so it stays the submitter even if the event
/// changes who plays which faction (`FactionTradeOffered`, `PlayerReplaced`). The faction can name
/// nobody: a fuzzed event can be submitted for a faction that isn't playing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct By {
    pub faction: Option<Faction>,
    pub player: Option<PlayerId>,
}

impl By {
    /// The submitter `faction` of `game`, with its player as of now.
    pub fn new(game: &Game, faction: impl Into<Option<Faction>>) -> By {
        let faction = faction.into();
        By { faction, player: game.get_player(faction) }
    }

    /// The host, or anyone without a faction.
    pub const NOBODY: By = By { faction: None, player: None };

    /// The submitting player, for an event only players can submit: panics for the host.
    #[track_caller]
    pub fn player(self) -> PlayerId {
        self.player.expect("the event has a player")
    }

    /// The faction, for an event that has passed validation: a Game Error if there is none.
    pub fn required_faction(self) -> GameResult<Faction> {
        self.faction.required("faction")
    }

    /// Whether `f` submitted the event (`Event.By(f)` in C#).
    pub fn is(self, f: impl Into<Option<Faction>>) -> bool {
        self.faction == f.into()
    }
}

/// The rules of one kind of event. Converted events implement this; see engine-conventions.md,
/// "Events".
pub trait EventRules {
    /// Whether the rules allow the event now. `Err` holds the message the player sees.
    fn validate(&self, game: &Game, by: By) -> Result<(), Rejection>;

    /// The validation `Game::submit` runs: also fails when the event can't even be validated,
    /// because the client left out a value validation needs. Such events override this, and their
    /// `validate` reports the failure as a rejection for bots and views.
    fn try_validate(&self, game: &Game, by: By) -> GameResult<Result<(), Rejection>> {
        Ok(self.validate(game, by))
    }

    /// Changes the game. Only called after validation passed (or when replaying the history).
    fn apply(&self, game: &mut Game, by: By) -> GameResult;

    /// The log line of the event.
    fn describe(&self, game: &Game, by: By) -> Message;

    /// [`describe`](EventRules::describe), or why it can't be built for an event that is about
    /// to be refused (a faction that isn't playing, no card).
    fn try_describe(&self, game: &Game, by: By) -> GameResult<Message> {
        Ok(self.describe(game, by))
    }

    /// The short form of the log line, for lists of events.
    fn describe_short(&self, game: &Game, by: By) -> Message {
        self.describe(game, by)
    }
}

/// One played event, as the history keeps it. Savegames hold it as
/// `{"by":"Atreides","at":"2026-11-13T04:14:35.622405Z","event":{"type":"Move",..}}`.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct HistoryEntry {
    /// The submitter's faction, `None` for the host.
    pub by: Option<Faction>,
    /// When the event was played. The server sets it after the event is applied, so the event
    /// itself sees the time it was submitted with (none, for a live submission).
    pub at: DateTimeOffset,
    #[ts(type = "Event")]
    pub event: Event,
    /// Which submission this is. A clone keeps it, so an undo, which replays the kept entries,
    /// keeps it too, while an event submitted after the undo gets a new one.
    #[serde(skip, default = "EntryId::fresh")]
    pub id: EntryId,
}

/// Tells history entries apart, as C# told its event objects apart by reference: an undo
/// replayed the same objects, so a battle kept its identity, and a battle initiated again after
/// the undo was a different one even at the same place in the history. Unique within the
/// process; never saved or shown.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct EntryId(u64);

impl EntryId {
    pub fn fresh() -> EntryId {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        EntryId(NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
    }
}

impl HistoryEntry {
    /// The entry of `event` by `by`, with no time.
    pub fn new(by: impl Into<Option<Faction>>, event: impl Into<Event>) -> HistoryEntry {
        HistoryEntry { by: by.into(), at: DateTimeOffset::default(), event: event.into(), id: EntryId::fresh() }
    }

    pub fn event_type(&self) -> EventType {
        self.event.event_type()
    }

    /// A copy to play in another game. It is a new submission, with its own id.
    pub fn detached(&self) -> HistoryEntry {
        HistoryEntry { by: self.by, at: self.at, event: self.event.clone(), id: EntryId::fresh() }
    }

    /// The submitter in `game`: its faction and that faction's player now.
    pub fn by_in(&self, game: &Game) -> By {
        By::new(game, self.by)
    }

    /// Whether `f` submitted the event.
    pub fn is_by(&self, f: impl Into<Option<Faction>>) -> bool {
        self.by == f.into()
    }

    /// The log line of the event in `game`.
    pub fn describe(&self, game: &Game) -> Message {
        self.event.describe(game, self.by_in(game))
    }

    /// Whether the event is valid in `game`, for its submitter.
    pub fn validate(&self, game: &Game) -> Result<(), Rejection> {
        self.event.validate(game, self.by_in(game))
    }
}

/// Why `Game::submit` didn't play an event.
#[derive(Clone, Debug)]
pub enum SubmitError {
    /// The submitter can't play this kind of event now. Holds "Event '...' is not applicable".
    NotApplicable(Message),
    /// The rules refuse the event.
    Rejected(Rejection),
    /// The event was accepted but failed while it was applied. The changes it made before the
    /// failure stay, and it isn't added to the history.
    Failed(GameError),
}

impl SubmitError {
    /// The message the player sees, as the C# `Execute` returned it.
    pub fn message(&self) -> Message {
        match self {
            SubmitError::NotApplicable(m) => m.clone(),
            SubmitError::Rejected(r) => r.0.clone(),
            SubmitError::Failed(e) => express!("Game Error: ", e.clone(), "."),
        }
    }
}

impl From<GameError> for SubmitError {
    fn from(e: GameError) -> SubmitError {
        SubmitError::Failed(e)
    }
}
