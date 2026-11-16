//! What every event has in common: who submitted it ([`By`]), the rules it follows
//! ([`EventRules`]), its place in the history ([`HistoryEntry`]), and why a submission failed
//! ([`SubmitError`]).
//!
//! Events are being converted from the C# shape (an `Rc` holding `GameEventBase` with the
//! initiator, implementing `GameEventMethods`) to plain data implementing [`EventRules`]. The
//! `events!` list in `events/mod.rs` marks the ones not yet converted with `@legacy`.
//! `events/dispatch.rs` lets the dispatch treat both alike.

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

/// One played event, as the history keeps it.
#[derive(Clone, Debug)]
pub struct HistoryEntry {
    /// The submitter's faction, `None` for the host.
    pub by: Option<Faction>,
    /// When the event was played. The server sets it after the event is applied, so the event
    /// itself sees the time it was submitted with (none, for a live submission).
    pub at: DateTimeOffset,
    pub event: Event,
}

impl HistoryEntry {
    /// The entry of `event` by `by`. Its time is none, except for an `@legacy` event copied from
    /// a played one (a deal's cancellation copies the offer), which keeps the original's time.
    pub fn new(by: impl Into<Option<Faction>>, event: impl Into<Event>) -> HistoryEntry {
        let event = event.into();
        let at = event.legacy_base().map(|b| b.time.get()).unwrap_or_default();
        HistoryEntry { by: by.into(), at, event }
    }

    pub fn event_type(&self) -> EventType {
        self.event.event_type()
    }

    /// The entry of an `@legacy` event built for its initiator (`X::new(game, faction)`): the
    /// submitter is the initiator in its base. Panics for a converted event; use
    /// [`HistoryEntry::new`] with the submitter instead.
    #[track_caller]
    pub fn legacy(event: impl Into<Event>) -> HistoryEntry {
        let event = event.into();
        HistoryEntry::new(event.legacy_initiator(), event)
    }

    /// The entry of a bot's decision for a bot of `faction`. An `@legacy` event keeps the initiator
    /// it was built with, which isn't always the bot's: a bot cancels a deal offer with a copy of
    /// the offer, initiator included.
    pub fn decided(faction: Option<Faction>, event: impl Into<Event>) -> HistoryEntry {
        let event = event.into();
        let by = event.legacy_base().map_or(faction, |b| b.initiator.get());
        HistoryEntry::new(by, event)
    }

    /// A copy whose event shares nothing with this one's (see [`Event::detached`]), to play in
    /// another game.
    pub fn detached(&self) -> HistoryEntry {
        HistoryEntry { by: self.by, at: self.at, event: self.event.detached() }
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
        self.event.prepare(game, self.by);
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
