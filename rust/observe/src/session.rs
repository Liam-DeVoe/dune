//! A game as the server runs it, and what players can observe of each call.
//!
//! The recorder and the replayer both drive a [`Session`] with the same calls in the same order:
//! the recorder decides the calls, the replayer reads them from the trace. Everything a call
//! returns that a player could notice is returned as trace values, which the replayer compares.
//!
//! As on the server (`server/src/views.rs`), views are built from a separate view replica that
//! executes every accepted event without validation, so building views never touches the
//! authoritative game.

use std::panic::{self, AssertUnwindSafe};

use serde_json::Value;
use treachery_bots::{BotParameters, ClassicBot, Priority};
use treachery_engine::events::{EventType, HistoryEntry};
use treachery_engine::messages::part::Part;
use treachery_engine::game::Game;
use treachery_engine::json;
use treachery_engine::messages::message::Message;
use treachery_engine::prelude::*;
use treachery_engine::game::PlayerId;
use treachery_view::build::{ResolvedViewer, Seating};
use treachery_view::replica::ViewReplica;

use crate::trace::{BotChoice, BotsResult, EventOutcome, Obs, QueryResult, SpecialOutcome};
use crate::view_hash::ViewHash;

/// The bot priority levels, in the order the server asks them.
pub const LEVELS: [&str; 5] = ["Highest", "High", "Middle", "Low", "End"];

pub struct Session {
    pub game: Game,
    /// What the views show of the seated users. Their seats are in `game.human_seats`.
    pub seating: Seating,
    replica: Option<ViewReplica>,
    /// The bots, one per faction (`None` for a player who has none yet), in creation order. They
    /// outlive undo, as on the server.
    pub bots: Vec<(Option<Faction>, ClassicBot)>,
    /// The applicable types from the latest query of each player, for the bots.
    last_query: Vec<Option<Vec<EventType>>>,
    /// Number of log messages already reported.
    log_seen: usize,
    pub full_views: bool,
    /// How views are hashed: [`ViewHash::LATEST`] when recording, what the trace says when
    /// replaying.
    pub view_hash: ViewHash,
}

/// A call that failed in a way the trace can't describe further: the game ends there. The
/// reference records one when its engine panics outside a C#-style exception.
pub struct Panicked(pub String);

/// Runs `f`. A panic is an error of the call, with the panic's message: the server catches a
/// call that panics, reports it, and the game goes on.
fn guarded<R>(f: impl FnOnce() -> R) -> Result<Result<R, String>, Panicked> {
    match panic::catch_unwind(AssertUnwindSafe(f)) {
        Ok(r) => Ok(Ok(r)),
        Err(p) => Ok(Err(treachery_engine::error::panic_message(p.as_ref()))),
    }
}

/// Silences the default panic hook: engine panics are expected and reported in the trace.
pub fn quiet_panics() {
    if std::env::var_os("OBSERVE_PRINT_PANICS").is_none() {
        panic::set_hook(Box::new(|_| {}));
    }
}

pub fn faction_name(f: Option<Faction>) -> String {
    name_or_absent(f)
}

pub fn parse_faction(name: &str) -> Option<Faction> {
    Faction::ALL.iter().copied().find(|f| f.to_string() == name)
}

fn find_exception(m: &Message) -> bool {
    m.parts.iter().any(|e| matches!(e, Part::Error(_)))
}

/// The human seats and what the views show of them, for `[userId, seat, name]` of each seated
/// user, as the server derives them from its participation. A seat of -1 is none.
pub fn seats(seated: &[(i32, i32, String)]) -> (IndexSet<Seat>, Seating) {
    let mut seating = Seating::default();
    let mut human_seats = IndexSet::new();
    for (_, seat, name) in seated {
        if let Some(seat) = Seat::from_number(*seat) {
            seating.names.entry(seat).or_insert_with(|| name.clone());
            human_seats.insert(seat);
        }
    }
    seating.seated_users = seated.len() as i32;
    (human_seats, seating)
}

impl Session {
    pub fn new(version: i32, seated: &[(i32, i32, String)], full_views: bool) -> Session {
        let (human_seats, seating) = seats(seated);
        let mut game = Game::of_version(version);
        game.human_seats = human_seats;
        Session { game, seating, replica: None, bots: Vec::new(), last_query: Vec::new(), log_seen: 0, full_views, view_hash: ViewHash::LATEST }
    }

    pub fn player_count(&self) -> usize {
        self.game.players.len()
    }

    fn player(&self, i: usize) -> PlayerId {
        self.game.player_at(i)
    }

    /// `GetApplicableEvents` for player `p` (an index) or for nobody.
    pub fn query(&mut self, p: Option<usize>, host: bool) -> Result<(QueryResult, Vec<EventType>), Panicked> {
        let player = p.map(|i| self.player(i));
        let game = &mut self.game;
        let r = guarded(|| game.get_applicable_events(player, host))?;
        let types = r.as_ref().map(|t| t.clone()).unwrap_or_default();
        if let Some(i) = p {
            if self.last_query.len() <= i {
                self.last_query.resize(i + 1, None);
            }
            self.last_query[i] = Some(types.clone());
        }
        Ok((
            match r {
                Ok(t) => QueryResult { types: Some(t.iter().map(|t| t.name().to_string()).collect()), ..Default::default() },
                Err(e) => QueryResult { error: Some(e), ..Default::default() },
            },
            types,
        ))
    }

    /// One round of asking the bots (`GameHub::perform_bot_event`). `seed_for` gives the seed of a
    /// bot that doesn't exist yet, or None to refuse creating it. Returns the result and the bots
    /// created, `[faction, seed]`.
    pub fn deliberate(&mut self, order: &[usize], end: bool, seed_for: &mut dyn FnMut(Option<Faction>) -> Option<i32>) -> Result<(BotsResult, Vec<(String, i32)>), Panicked> {
        let mut created = Vec::new();
        let levels = if end { 5 } else { 4 };
        let outcome = guarded(|| -> Result<Option<(usize, usize, HistoryEntry)>, String> {
            for level in 0..levels {
                for &i in order {
                    let p = self.game.player_at(i);
                    let faction = self.game.player(p).faction;
                    let index = match self.bots.iter().position(|(f, _)| *f == faction) {
                        Some(index) => {
                            self.bots[index].1.set_player(p);
                            index
                        }
                        None => {
                            let seed = seed_for(faction).ok_or_else(|| format!("harness: no seed for a new {} bot", name_or_absent(faction)))?;
                            let bot = ClassicBot::new(p, BotParameters::for_faction(faction), SharedRandom::from_seed(seed));
                            self.bots.push((faction, bot));
                            created.push((faction_name(faction), seed));
                            self.bots.len() - 1
                        }
                    };
                    let events = self.last_query.get(i).cloned().flatten().ok_or_else(|| format!("harness: player {i} was not queried before the bots"))?;
                    let bot = &mut self.bots[index].1;
                    let g = &mut self.game;
                    let evt = bot.decide(g, Priority::ALL[level], &events);
                    if let Some(e) = evt {
                        return Ok(Some((i, level, HistoryEntry::decided(faction, e))));
                    }
                }
            }
            Ok(None)
        });
        let result = match outcome {
            Err(p) => return Err(p),
            Ok(Err(e)) | Ok(Ok(Err(e))) => BotsResult { error: Some(e), ..Default::default() },
            Ok(Ok(Ok(None))) => BotsResult::default(),
            Ok(Ok(Ok(Some((p, level, e))))) => {
                let text = json::serialize(&e);
                let event: Value = serde_json::from_str(&text).unwrap_or(Value::String(text));
                BotsResult { choice: Some(BotChoice { p, level: LEVELS[level].to_string(), event }), ..Default::default() }
            }
        };
        Ok((result, created))
    }

    /// Executes an event with validation, as the server does for a submission.
    pub fn submit(&mut self, event_json: &str, host: bool) -> Result<EventOutcome, Panicked> {
        match json::deserialize(event_json) {
            Ok(e) => self.submit_event(e, host),
            Err(e) => Ok(EventOutcome { result: "unreadable".into(), message: Some(format!("{e:?}")), obs: Obs::default() }),
        }
    }

    /// The same for an event that is already decoded.
    pub fn submit_event(&mut self, ev: HistoryEntry, host: bool) -> Result<EventOutcome, Panicked> {
        let before = self.game.history.entries.len();
        let game = &mut self.game;
        let r = guarded(|| game.play(ev, true, host).err().map(|e| e.message()))?;
        let accepted = self.game.history.entries.len() > before;
        let (result, message) = match r {
            Err(e) => ("error", Some(e)),
            Ok(None) => (if accepted { "accepted" } else { "rejected" }, None),
            Ok(Some(m)) => {
                let text = treachery_skin::default_skin().render(&m);
                let kind = if accepted { "accepted" } else if find_exception(&m) { "error" } else { "rejected" };
                (kind, Some(text))
            }
        };
        let obs = self.observe(self.game.history.entries.len() != before, false)?;
        Ok(EventOutcome { result: result.into(), message, obs })
    }

    /// `Undo`, `RestoreRecentlyUndone` or `DismissRecentlyUndone` (`kind` is the trace op).
    pub fn special(&mut self, kind: &str, until: i32) -> Result<SpecialOutcome, Panicked> {
        let game = &mut self.game;
        let r = guarded(|| match kind {
            "undo" => game.undo(until),
            "restore" => game.restore_recently_undone(),
            _ => game.dismiss_recently_undone(),
        })?;
        let obs = self.observe(true, true)?;
        Ok(SpecialOutcome { error: r.err(), panic: false, obs })
    }

    /// The log messages added since the last call, and the views if `history_changed`.
    fn observe(&mut self, history_changed: bool, rebuild: bool) -> Result<Obs, Panicked> {
        let mut obs = Obs::default();
        let reports = self.game.reports();
        let total: usize = reports.iter().map(|r| r.messages.len()).sum();
        if total < self.log_seen {
            obs.log_cut = Some(total);
            self.log_seen = total;
        }
        if total > self.log_seen {
            let skin = treachery_skin::default_skin();
            let mut flat = 0;
            for r in reports {
                for m in &r.messages {
                    if flat >= self.log_seen {
                        let target = match m.to {
                            Audience::Everyone => String::new(),
                            Audience::Faction(f) => faction_name(Some(f)),
                        };
                        let text = match guarded(|| skin.render(m))? {
                            Ok(t) => t,
                            Err(e) => format!("<render error: {e}>"),
                        };
                        obs.log.push((target, text));
                    }
                    flat += 1;
                }
            }
            self.log_seen = total;
        }
        if history_changed {
            if rebuild {
                self.replica = None;
            }
            self.sync_replica(&mut obs)?;
            self.views(&mut obs)?;
        }
        Ok(obs)
    }

    /// Brings the view replica up to date, as `server/src/views.rs` `Entry::sync` does.
    fn sync_replica(&mut self, obs: &mut Obs) -> Result<(), Panicked> {
        if self.replica.as_ref().is_some_and(|r| r.game.event_count() > self.game.event_count()) {
            self.replica = None;
        }
        let replica = self.replica.get_or_insert_with(|| {
            let mut r = ViewReplica::new(self.game.rules.version);
            r.game.human_seats = self.game.human_seats.clone();
            r.seating = self.seating.clone();
            r
        });
        let done = replica.game.event_count() as usize;
        for (nr, e) in self.game.history.entries.iter().enumerate().skip(done) {
            let text = json::serialize(e);
            match guarded(|| replica.apply(&text))? {
                Ok(Ok(())) => {}
                Ok(Err(m)) | Err(m) => {
                    obs.replica = Some(format!("event {nr}: {m}"));
                    self.replica = None;
                    return Ok(());
                }
            }
        }
        Ok(())
    }

    /// The viewers of every view: each player, then a seatless host, then an observer.
    pub fn viewers(&self) -> Vec<(String, ResolvedViewer)> {
        let mut v: Vec<(String, ResolvedViewer)> = self
            .game
            .player_indices()
            .map(|p| {
                let f = self.game.player(p).faction;
                (format!("player {} ({})", p.index(), faction_name(f)), ResolvedViewer { player: Some(p), faction: f, is_host: false, is_observer: false })
            })
            .collect();
        v.push(("host".into(), ResolvedViewer { player: None, faction: None, is_host: true, is_observer: true }));
        v.push(("observer".into(), ResolvedViewer { player: None, faction: None, is_host: false, is_observer: true }));
        v
    }

    fn views(&mut self, obs: &mut Obs) -> Result<(), Panicked> {
        let viewers = self.viewers();
        let Some(replica) = self.replica.as_mut() else { return Ok(()) };
        let mut hashes = Vec::new();
        let mut full = Vec::new();
        for (_, rv) in &viewers {
            // The replica's players are the game's players, by index.
            let rv = ResolvedViewer { player: rv.player.map(|p| replica.game.player_at(p.index())), ..*rv };
            let (kind, full_views) = (self.view_hash, self.full_views);
            // Without full views, the JSON goes straight into the hasher.
            let built = guarded(|| {
                let view = replica.view_as(&rv);
                if full_views {
                    let bytes = serde_json::to_vec(&view).expect("views serialize");
                    (kind.of(&bytes), Some(bytes))
                } else {
                    let mut h = kind.hasher();
                    serde_json::to_writer(&mut h, &view).expect("views serialize");
                    (h.finish(), None)
                }
            })?;
            match built {
                Ok((hash, bytes)) => {
                    hashes.push(hash);
                    if let Some(bytes) = bytes {
                        full.push(serde_json::from_slice(&bytes).unwrap_or(Value::Null));
                    }
                }
                Err(e) => {
                    hashes.push(format!("error: {e}"));
                    if self.full_views {
                        full.push(Value::String(format!("error: {e}")));
                    }
                }
            }
        }
        obs.views = Some(hashes);
        if self.full_views {
            obs.full = Some(full);
        }
        Ok(())
    }
}
