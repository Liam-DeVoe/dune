//! `observe refit`: carries a recorded game over to an engine whose random draws differ.
//!
//! The savegames in `scripts/fixtures` and the recorded games the tests replay are event logs.
//! After a change to the random number generator the same events no longer fit: the traitors,
//! cards and storm moves come out differently, so the first choice that names one of them is
//! refused and the rest of the log goes with it. Refitting replays the log and keeps every event
//! that still passes validation. An event that doesn't is replaced by one of the same type from the
//! same faction, first as its bot would decide it, then from the choices the client's form offers
//! (the fuzzer's `build`). When no such event is valid yet, the bots play on for a few rounds until
//! one is; if none becomes valid the event is dropped. The result reaches the same kind of
//! situation through the same sequence of event types, with different cards and positions.
//!
//! An event that was already invalid where the old log executed it (savegames are loaded without
//! validation) is kept only if it passes validation now, and never replaced.

use treachery_engine::enum_type::GameEnum;
use std::panic::{self, AssertUnwindSafe};

use serde_json::{Map, Value};
use treachery_bots::{BotParameters, ClassicBot, Priority};
use treachery_engine::events::{AlliancePassed, CardsDetermined, EndPhase, Event, EventType, PerformHmsPlacement, PerformSetup};
use treachery_engine::game::Game;
use treachery_legacy as json;
use treachery_engine::prelude::*;

use crate::fuzz::Fuzzer;

/// Bot rounds to play, at most, to catch up with the old log's position.
const CATCH_UP_ROUNDS: usize = 300;
/// Bot rounds without a change of phase after which the bots are taken to be stuck.
const STALL_ROUNDS: usize = 25;
/// Events the bots may add to one log, at most.
const MAX_ADDED: usize = 5000;
/// Bot rounds to play, at most, within the event's turn and main phase before it is dropped.
const ADVANCE_ROUNDS: usize = 8;
/// Candidates the form-based builder tries per repair.
const BUILD_TRIES: usize = 6;

/// One line of the log to refit.
#[derive(Clone, Debug)]
pub enum Item {
    /// An event: its JSON object, whether it was executed with validation and as the host,
    /// whether it was valid where the old log executed it, and the turn and main phase it was
    /// executed in there.
    Event { event: Map<String, Value>, validate: bool, host: bool, legit: bool, at: Option<Position> },
    Undo(i32),
    Restore,
    Dismiss,
}

/// A turn and main phase, ordered as the game goes.
pub type Position = (i32, i32);

/// Where a refit should end: the old log's last turn, main phase and phase, and whether a battle
/// was in progress.
#[derive(Clone, Debug, Default)]
pub struct End {
    pub position: Option<Position>,
    pub phase: Option<String>,
    pub battle: Option<bool>,
}

impl End {
    /// Whether the game is at the phase (and battle) the old log ended at.
    pub fn reached(&self, game: &Game) -> bool {
        self.phase.as_ref().is_none_or(|p| *p == game.phase().to_string()) && self.battle.is_none_or(|b| b == game.current_battle().is_some())
    }
}

/// How an event came through.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fate {
    /// Executed unchanged.
    Kept,
    /// Replaced by the bot's or the form's event of the same type.
    Replaced,
    /// Added by the bots playing on before a replaced event.
    Added,
}

/// One line of the refitted log. `from` is the index of the input item it stands for (`None`
/// for events the bots added).
#[derive(Clone, Debug)]
pub struct Out {
    pub from: Option<usize>,
    pub item: Item,
    pub fate: Fate,
}

#[derive(Clone, Debug, Default)]
pub struct Stats {
    pub kept: usize,
    pub replaced: usize,
    pub added: usize,
    pub dropped: usize,
}

pub struct Refit {
    pub game: Game,
    /// Keyed by the faction, `None` for a player who has none yet.
    bots: Vec<(Option<Faction>, ClassicBot)>,
    fuzz: Fuzzer,
    pub out: Vec<Out>,
    pub stats: Stats,
    /// For each event of the old log's current history, the length of the new history after it,
    /// to carry undo positions over.
    old_history: Vec<usize>,
    /// `old_history` before the last undo, for a restore.
    old_history_before_undo: Option<Vec<usize>>,
}

fn quiet<R>(f: impl FnOnce() -> R) -> Option<R> {
    let hook = panic::take_hook();
    panic::set_hook(Box::new(|_| {}));
    let r = panic::catch_unwind(AssertUnwindSafe(f)).ok();
    panic::set_hook(hook);
    r
}

fn type_of(event: &Map<String, Value>) -> Option<EventType> {
    let name = event.get("$type")?.as_str()?;
    EventType::ALL.iter().copied().find(|t| t.name() == name)
}

fn to_object(e: &HistoryEntry) -> Map<String, Value> {
    match serde_json::from_str(&json::serialize(e)) {
        Ok(Value::Object(m)) => m,
        _ => Map::new(),
    }
}

impl Refit {
    /// `seated` holds `[userId, seat, name]` of each seated user.
    pub fn new(version: i32, seated: &[(i32, i32, String)], seed: u64) -> Refit {
        let (human_seats, seating) = crate::session::seats(seated);
        let mut game = Game::of_version(version);
        game.human_seats = human_seats;
        Refit {
            game,
            bots: Vec::new(),
            fuzz: Fuzzer::new(seed).with_seating(seating),
            out: Vec::new(),
            stats: Stats::default(),
            old_history: Vec::new(),
            old_history_before_undo: None,
        }
    }

    /// Executes an event; true if it was accepted (it joined the history).
    fn execute(&mut self, event: &Map<String, Value>, validate: bool, host: bool) -> bool {
        let Ok(e) = crate::events::decode(&Value::Object(event.clone())) else { return false };
        let before = self.game.history.entries.len();
        let game = &mut self.game;
        let r = quiet(|| game.play(e, validate, host));
        match r {
            Some(_) => self.game.history.entries.len() > before,
            None => {
                // A panic may have left the game half changed: rebuild it from its history.
                self.rebuild(before);
                false
            }
        }
    }

    /// Replays the first `n` events of the history into a new game.
    fn rebuild(&mut self, n: usize) {
        let events: Vec<HistoryEntry> = self.game.history.entries.iter().take(n).cloned().collect();
        let mut game = self.game.new_like();
        for e in events {
            let text = json::serialize(&e);
            let e: HistoryEntry = json::deserialize(&text).expect("a history event reads back");
            quiet(|| game.play(e, false, true));
        }
        self.game = game;
    }

    /// The index of `faction`'s bot in `bots`, created on first use, pointed at its player.
    fn bot(bots: &mut Vec<(Option<Faction>, ClassicBot)>, game: &Game, faction: Option<Faction>) -> Option<usize> {
        let p = game.get_player(faction)?;
        let index = match bots.iter().position(|(f, _)| *f == faction) {
            Some(i) => i,
            None => {
                let bot = ClassicBot::new(p, BotParameters::for_faction(faction), treachery_engine::random::new_rng(7919 * value_or_absent(faction) + 1));
                bots.push((faction, bot));
                bots.len() - 1
            }
        };
        bots[index].1.set_player(p);
        Some(index)
    }

    /// An event of type `t` for `faction` that passes validation now: the bot's, else one built
    /// from the form's choices.
    fn replacement(&mut self, t: EventType, faction: Option<Faction>) -> Option<Map<String, Value>> {
        if let Some(b) = Self::bot(&mut self.bots, &self.game, faction) {
            let (bot, game) = (&mut self.bots[b].1, &self.game);
            for level in Priority::ALL {
                if let Some(Some(e)) = quiet(|| bot.decide(game, level, &[t])) {
                    if e.event_type() == t {
                        return Some(to_object(&e));
                    }
                }
            }
        }
        let player = self.game.players.iter().position(|p| p.faction == faction);
        for _ in 0..BUILD_TRIES {
            let candidate = quiet(|| self.fuzz.build(&self.game, player, t)).flatten()?;
            let text = Value::Object(candidate.clone()).to_string();
            let game = &self.game;
            let valid = quiet(|| {
                let e: HistoryEntry = json::deserialize(&text).ok()?;
                Some(e.validate(game).is_ok())
            })
            .flatten()
            .unwrap_or(false);
            if valid {
                return Some(candidate);
            }
        }
        None
    }

    /// One round of the bots, as the server asks them: every player's bot at the highest priority
    /// first. If no bot acts, the host ends the phase when it may. False if nothing happened.
    fn bot_round(&mut self, time: &Value) -> bool {
        let players: Vec<(usize, Faction)> = self.game.players.iter().enumerate().filter_map(|(i, p)| p.faction.map(|f| (i, f))).collect();
        for level in Priority::ALL {
            for &(i, faction) in &players {
                let p = self.game.player_at(i);
                let Some(allowed) = quiet(|| self.game.get_applicable_events(Some(p), false)) else { continue };
                let Some(b) = Self::bot(&mut self.bots, &self.game, Some(faction)) else { continue };
                let (bot, game) = (&mut self.bots[b].1, &self.game);
                if let Some(Some(e)) = quiet(|| bot.decide(game, level, &allowed)) {
                    let mut event = to_object(&e);
                    event.insert("Time".into(), time.clone());
                    if self.execute(&event, true, false) {
                        self.push(None, Item::Event { event, validate: true, host: false, legit: true, at: None }, Fate::Added);
                        self.stats.added += 1;
                        return true;
                    }
                }
            }
        }
        // A seated human who hasn't passed in the alliance phase passes (no bot does that for them).
        if matches!(self.game.phase(), Phase::AllianceA | Phase::AllianceB) {
            for &(i, faction) in &players {
                let p = self.game.player_at(i);
                if self.game.turn.has_acted_or_passed.contains(&faction) {
                    continue;
                }
                let allowed = quiet(|| self.game.get_applicable_events(Some(p), false)).unwrap_or_default();
                if allowed.contains(&EventType::AlliancePassed) {
                    let e = Event::from(AlliancePassed::default());
                    if self.add(to_object(&HistoryEntry::new(Some(faction), e)), false, time) {
                        return true;
                    }
                }
            }
        }
        // What the host does when no bot acts.
        let host = quiet(|| self.game.get_applicable_events(None, true)).unwrap_or_default();
        let game = &self.game;
        let mut candidates: Vec<HistoryEntry> = Vec::new();
        if host.contains(&EventType::CardsDetermined) {
            let cards: Vec<TreacheryCard> = game.decks.treachery_deck.as_ref().map(|d| d.items.clone()).unwrap_or_default();
            let richese = if game.is_playing(Faction::Richese) { TreacheryCardManager::get_richese_cards() } else { vec![] };
            let mut e = CardsDetermined::default();
            e.set_treachery_cards(&cards);
            e.set_richese_cards(&richese);
            candidates.push(HistoryEntry::new(None, e));
        }
        if host.contains(&EventType::PerformSetup) {
            if let Some(by) = quiet(|| game.next_faction_to_perform_custom_setup()) {
                let mut e = PerformSetup::default();
                e.resources = 10;
                e.force_locations = Some(String::new());
                candidates.push(HistoryEntry::new(by, e));
            }
        }
        if host.contains(&EventType::PerformHmsPlacement) && !game.players.is_empty() {
            let p = game.player_at(0);
            if let Some(l) = PerformHmsPlacement::valid_locations(game, p).next() {
                let mut e = PerformHmsPlacement::default();
                e.target = Some(l);
                candidates.push(HistoryEntry::new(game.player(p).faction, e));
            }
        }
        if host.contains(&EventType::EndPhase) {
            candidates.push(HistoryEntry::new(None, EndPhase::default()));
        }
        for e in candidates {
            if self.add(to_object(&e), true, time) {
                return true;
            }
        }
        false
    }

    /// Executes an event the refit adds, and records it. True if it was accepted.
    fn add(&mut self, mut event: Map<String, Value>, host: bool, time: &Value) -> bool {
        event.insert("Time".into(), time.clone());
        if self.execute(&event, true, host) {
            self.push(None, Item::Event { event, validate: true, host, legit: true, at: None }, Fate::Added);
            self.stats.added += 1;
            return true;
        }
        false
    }

    fn push(&mut self, from: Option<usize>, item: Item, fate: Fate) {
        self.out.push(Out { from, item, fate });
    }

    /// Tries the event as it is, then a replacement. True if one was executed.
    fn place(&mut self, index: usize, event: &Map<String, Value>, validate: bool, host: bool, legit: bool) -> bool {
        if self.execute(event, validate || legit, host) {
            self.push(Some(index), Item::Event { event: event.clone(), validate: validate || legit, host, legit, at: None }, Fate::Kept);
            self.stats.kept += 1;
            return true;
        }
        if !legit {
            return false;
        }
        let (Some(t), Some(initiator)) = (type_of(event), event.get("Initiator").and_then(|v| v.as_i64())) else { return false };
        let Some(mut replacement) = self.replacement(t, treachery_engine::enum_type::from_value_or_absent::<Faction>(initiator as i32).ok().flatten()) else { return false };
        if let Some(time) = event.get("Time") {
            replacement.insert("Time".into(), time.clone());
        }
        if self.execute(&replacement, true, host) {
            self.push(Some(index), Item::Event { event: replacement, validate: true, host, legit, at: None }, Fate::Replaced);
            self.stats.replaced += 1;
            return true;
        }
        false
    }

    fn position(&self) -> Position {
        (i32::from(self.game.turn.current_turn), self.game.turn.current_main_phase.value())
    }

    /// Takes back what was added since `history` and `out` had these lengths.
    fn take_back(&mut self, history: usize, out: usize) {
        let added = self.out[out..].iter().filter(|o| o.fate == Fate::Added).count();
        self.stats.added -= added;
        self.out.truncate(out);
        self.rebuild(history);
    }

    /// The bots play until the game reaches `at`. Rounds that stop changing the phase (bots that
    /// repeat themselves) are taken back, and so is everything once the log has grown too much.
    fn catch_up(&mut self, at: Position, time: &Value) {
        let mut checkpoint = (self.game.history.entries.len(), self.out.len(), self.game.phase());
        for _ in 0..CATCH_UP_ROUNDS {
            if self.position() >= at || self.stats.added > MAX_ADDED || !self.bot_round(time) {
                return;
            }
            if self.game.phase() != checkpoint.2 {
                checkpoint = (self.game.history.entries.len(), self.out.len(), self.game.phase());
            } else if self.out.len() - checkpoint.1 > STALL_ROUNDS {
                break;
            }
        }
        if self.position() < at {
            self.take_back(checkpoint.0, checkpoint.1);
        }
    }

    /// Places one event: the bots first catch up with the turn and main phase the old log
    /// executed it in; an event whose moment has passed is dropped. Then the event as it is, or a
    /// replacement, possibly after a few more bot rounds in the same turn and main phase.
    fn feed_event(&mut self, index: usize, event: &Map<String, Value>, validate: bool, host: bool, legit: bool, at: Option<Position>) -> bool {
        let time = event.get("Time").cloned().unwrap_or(Value::Null);
        if legit {
            if let Some(at) = at {
                self.catch_up(at, &time);
                if self.position() > at {
                    return false;
                }
            }
        }
        if self.place(index, event, validate, host, legit) {
            return true;
        }
        if !legit {
            return false;
        }
        let (history, out, here) = (self.game.history.entries.len(), self.out.len(), self.position());
        for _ in 0..ADVANCE_ROUNDS {
            if !self.bot_round(&time) || self.position() != here {
                break;
            }
            if self.place(index, event, validate, host, legit) {
                return true;
            }
        }
        self.take_back(history, out);
        false
    }

    /// After the last event: the bots play on until the game is where the old log ended, and stop
    /// there (taking back rounds that went past it).
    pub fn finish(&mut self, end: &End, time: &Value) {
        let Some(target) = end.position else { return };
        self.catch_up(target, time);
        if self.position() != target {
            return;
        }
        let (mut history, mut out) = (self.game.history.entries.len(), self.out.len());
        for _ in 0..ADVANCE_ROUNDS * 4 {
            if end.reached(&self.game) {
                return;
            }
            if !self.bot_round(time) {
                return;
            }
            if self.position() != target {
                self.take_back(history, out);
                return;
            }
            (history, out) = (self.game.history.entries.len(), self.out.len());
        }
    }

    /// Handles one input item.
    pub fn feed(&mut self, index: usize, item: &Item) {
        match item {
            Item::Event { event, validate, host, legit, at } => {
                if !self.feed_event(index, event, *validate, *host, *legit, *at) {
                    self.stats.dropped += 1;
                }
            }
            Item::Undo(until) => {
                // `until` counts events of the old history; map it to the new one.
                let keep = (*until).clamp(0, self.old_history.len() as i32) as usize;
                let n = if keep == 0 { 0 } else { self.old_history[keep - 1] };
                let n = n.min(self.game.history.entries.len()) as i32;
                if quiet(|| self.game.undo(n)).is_some() {
                    self.old_history_before_undo = Some(self.old_history.clone());
                    self.old_history.truncate(keep);
                    self.push(Some(index), Item::Undo(n), Fate::Kept);
                }
            }
            Item::Restore => {
                if quiet(|| self.game.restore_recently_undone()).is_some() {
                    if let Some(h) = self.old_history_before_undo.take() {
                        self.old_history = h;
                    }
                    self.push(Some(index), Item::Restore, Fate::Kept);
                }
            }
            Item::Dismiss => {
                if quiet(|| self.game.dismiss_recently_undone()).is_some() {
                    self.old_history_before_undo = None;
                    self.push(Some(index), Item::Dismiss, Fate::Kept);
                }
            }
        }
        if matches!(item, Item::Event { .. }) {
            self.old_history.push(self.game.history.entries.len());
        }
    }
}

/// Refits a whole log. `seed` replaces the seed of `EstablishPlayers`, if given.
pub fn refit(version: i32, seated: &[(i32, i32, String)], items: &[Item], end: &End, seed: Option<i32>) -> Refit {
    let mut r = Refit::new(version, seated, 1);
    let mut time = Value::Null;
    for (i, item) in items.iter().enumerate() {
        if let Item::Event { event, .. } = item {
            time = event.get("Time").cloned().unwrap_or(Value::Null);
        }
        match (item, seed) {
            (Item::Event { event, validate, host, legit, at }, Some(seed)) if event.get("$type").and_then(|v| v.as_str()) == Some("EstablishPlayers") => {
                let mut event = event.clone();
                event.insert("Seed".into(), Value::from(seed));
                r.feed(i, &Item::Event { event, validate: *validate, host: *host, legit: *legit, at: *at });
            }
            _ => r.feed(i, item),
        }
    }
    r.finish(end, &time);
    r
}
