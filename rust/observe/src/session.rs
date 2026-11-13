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
use sha2::{Digest, Sha256};
use treachery_bots::{BotParameters, ClassicBot, Priority};
use treachery_engine::compat::cs::{self, CsException};
use treachery_engine::events::{GameEvent, GameEventType};
use treachery_engine::messages::expression::ExpressionElement;
use treachery_engine::game::Game;
use treachery_engine::json;
use treachery_engine::messages::message::Message;
use treachery_engine::game::Participation;
use treachery_engine::prelude::*;
use treachery_engine::game::PlayerIndex;
use treachery_view::build::ResolvedViewer;
use treachery_view::replica::ViewReplica;

use crate::trace::{BotChoice, BotsResult, EventOutcome, Obs, QueryResult, SpecialOutcome};

/// The bot priority levels, in the order the server asks them.
pub const LEVELS: [&str; 5] = ["Highest", "High", "Middle", "Low", "End"];

pub struct Session {
    pub game: Game,
    pub participation: Shared<Participation>,
    replica: Option<ViewReplica>,
    /// The bots, one per faction, in creation order. They outlive undo, as on the server.
    pub bots: Vec<(Faction, ClassicBot)>,
    /// The applicable types from the latest query of each player, for the bots.
    last_query: Vec<Option<Vec<GameEventType>>>,
    /// Number of log messages already reported.
    log_seen: usize,
    pub full_views: bool,
}

/// A panic that is not a C# exception: the engine or a bot failed in a way the trace can't
/// describe further. The game ends there.
pub struct Panicked(pub String);

fn panic_text(p: Box<dyn std::any::Any + Send>) -> String {
    if let Some(e) = p.downcast_ref::<CsException>() {
        e.to_string()
    } else if let Some(s) = p.downcast_ref::<String>() {
        s.clone()
    } else if let Some(s) = p.downcast_ref::<&str>() {
        (*s).to_string()
    } else {
        "panic".to_string()
    }
}

/// Runs `f`, turning a C# exception into `Ok(Err(..))` and any other panic into `Err(Panicked)`.
fn guarded<R>(f: impl FnOnce() -> R) -> Result<Result<R, String>, Panicked> {
    match panic::catch_unwind(AssertUnwindSafe(|| cs::catch(AssertUnwindSafe(f)))) {
        Ok(Ok(r)) => Ok(Ok(r)),
        Ok(Err(e)) => Ok(Err(e.to_string())),
        Err(p) => Err(Panicked(panic_text(p))),
    }
}

/// Silences the default panic hook: engine panics are expected and reported in the trace.
pub fn quiet_panics() {
    if std::env::var_os("OBSERVE_PRINT_PANICS").is_none() {
        panic::set_hook(Box::new(|_| {}));
    }
}

pub fn faction_name(f: Faction) -> String {
    f.to_string()
}

pub fn parse_faction(name: &str) -> Option<Faction> {
    Faction::ALL.iter().copied().find(|f| f.to_string() == name)
}

fn short_hash(bytes: &[u8]) -> String {
    let d = Sha256::digest(bytes);
    d.iter().take(8).map(|b| format!("{b:02x}")).collect()
}

fn find_exception(m: &Message) -> bool {
    m.expression.elements.iter().any(|e| matches!(e, ExpressionElement::Exception(_)))
}

impl Session {
    pub fn new(version: i32, seated: &[(i32, i32, String)], full_views: bool) -> Session {
        let participation = Shared::new(Participation::default());
        {
            let mut p = participation.borrow_mut();
            for (user, seat, name) in seated {
                p.player_names.add(*user, name.clone());
                p.seated_players.add(*user, *seat);
            }
        }
        let game = Game::new_int32_participation(version, &participation);
        Session { game, participation, replica: None, bots: Vec::new(), last_query: Vec::new(), log_seen: 0, full_views }
    }

    pub fn player_count(&self) -> usize {
        self.game.players.len()
    }

    fn player(&self, i: usize) -> PlayerIndex {
        self.game.player_at(i)
    }

    /// `GetApplicableEvents` for player `p` (an index) or for nobody.
    pub fn query(&mut self, p: Option<usize>, host: bool) -> Result<(QueryResult, Vec<GameEventType>), Panicked> {
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
    pub fn deliberate(&mut self, order: &[usize], end: bool, seed_for: &mut dyn FnMut(Faction) -> Option<i32>) -> Result<(BotsResult, Vec<(String, i32)>), Panicked> {
        let mut created = Vec::new();
        let levels = if end { 5 } else { 4 };
        let outcome = guarded(|| -> Result<Option<(usize, usize, GameEvent)>, String> {
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
                            let seed = seed_for(faction).ok_or_else(|| format!("harness: no seed for a new {faction} bot"))?;
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
                        return Ok(Some((i, level, e)));
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
        let ev: GameEvent = match json::deserialize(event_json) {
            Ok(e) => e,
            Err(e) => return Ok(EventOutcome { result: "unreadable".into(), message: Some(format!("{e:?}")), obs: Obs::default() }),
        };
        let before = self.game.history.len();
        let game = &mut self.game;
        let r = guarded(|| {
            ev.initialize_game(game);
            ev.execute(game, true, host)
        })?;
        let accepted = self.game.history.len() > before;
        let (result, message) = match r {
            Err(e) => ("error", Some(e)),
            Ok(None) => (if accepted { "accepted" } else { "rejected" }, None),
            Ok(Some(m)) => {
                let text = treachery_skin::default_skin().message_to_string_in(&m, Some(&self.game));
                let kind = if accepted { "accepted" } else if find_exception(&m) { "error" } else { "rejected" };
                (kind, Some(text))
            }
        };
        let obs = self.observe(self.game.history.len() != before, false)?;
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
                        let target = if m.target == Faction::None { String::new() } else { faction_name(m.target) };
                        let text = match guarded(|| skin.message_to_string_in(m, Some(&self.game)))? {
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
        let replica = self.replica.get_or_insert_with(|| ViewReplica::new(self.game.version, &self.participation));
        let done = replica.game.event_count() as usize;
        for (nr, e) in self.game.history.iter().enumerate().skip(done) {
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
                (format!("player {} ({})", p.index, faction_name(f)), ResolvedViewer { player: Some(p), faction: Some(f), is_host: false, is_observer: false })
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
            let rv = ResolvedViewer { player: rv.player.map(|p| replica.game.player_at(p.index)), ..*rv };
            match guarded(|| serde_json::to_vec(&replica.view_as(&rv)).expect("views serialize"))? {
                Ok(bytes) => {
                    hashes.push(short_hash(&bytes));
                    if self.full_views {
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
