//! The recorder: plays one bot or fuzz game and writes its trace. The driver follows the C#
//! oracle's `GameRunner.Play` (treachery-online/Oracle/GameRunner.cs) at the latest version:
//! the host establishes the game, then each round the bots are queried and asked, and when no
//! bot acts a human player passes or the host acts. Fuzz games add random events, changed bot
//! events, events of rarely executed types, and undo.

use std::io::Write;
use std::sync::mpsc::Sender;

use serde_json::{Map, Value};
use treachery_engine::events::{AlliancePassed, CardsDetermined, EndPhase, FactionSelected, GameEvent, GameEventType, PerformHmsPlacement, PerformSetup};
use treachery_engine::game::Game;
use treachery_engine::json;
use treachery_engine::prelude::*;

use crate::config::{Config, HostStyle};
use crate::fuzz::{self, Fuzzer, Shadow};
use crate::rng::{self, Rng};
use crate::session::{Panicked, Session};
use crate::trace::{self, Header, Line};
use crate::watch::{self, Budgets, Ending, Msg};

/// Why a game stopped early.
struct Stop(String, Option<String>);

impl From<Panicked> for Stop {
    fn from(p: Panicked) -> Stop {
        Stop("panic".into(), Some(p.0))
    }
}

struct Recorder<'a> {
    tx: &'a Sender<Msg>,
    sess: Session,
    rng: Rng,
    game_seed: u64,
    cfg: Config,
    fuzz: Option<Fuzzer>,
    shadow: Option<Shadow>,
    /// Seconds after 2024-01-01T12:00:00Z of the latest event.
    clock: i64,
    /// Events submitted so far.
    events: usize,
    /// Report reach (`RecordOptions::reach`).
    reach: bool,
}

/// `2024-01-01T12:00:00+00:00` plus `secs`, as .NET writes a `DateTimeOffset`.
fn time_text(secs: i64) -> String {
    let t = 19723 * 86400 + 12 * 3600 + secs;
    let (days, rem) = (t.div_euclid(86400), t.rem_euclid(86400));
    // Howard Hinnant's civil_from_days.
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    format!("{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}+00:00", rem / 3600, rem / 60 % 60, rem % 60)
}

fn to_object(e: &GameEvent) -> Map<String, Value> {
    match serde_json::from_str(&json::serialize(e)) {
        Ok(Value::Object(m)) => m,
        _ => Map::new(),
    }
}

impl<'a> Recorder<'a> {
    fn emit(&mut self, line: Line) {
        let _ = self.tx.send(Msg::Line(line));
    }

    fn begin(&self, line: &Line) {
        let _ = self.tx.send(Msg::Begin(line.clone()));
    }

    fn query(&mut self, p: Option<usize>, host: bool) -> Result<Vec<GameEventType>, Stop> {
        let line = Line::Query { p, host, result: Default::default() };
        self.begin(&line);
        match self.sess.query(p, host) {
            Ok((result, types)) => {
                if self.reach && p.is_some() {
                    let phase = self.sess.game.current_phase;
                    for t in &types {
                        let _ = self.tx.send(Msg::Text(format!("offered\t{}\t{phase}", t.name())));
                    }
                }
                self.emit(Line::Query { p, host, result });
                Ok(types)
            }
            Err(e) => {
                self.emit(Line::Query { p, host, result: trace::QueryResult { panic: true, ..Default::default() } });
                Err(e.into())
            }
        }
    }

    fn submit(&mut self, mut event: Map<String, Value>, src: &str, p: Option<usize>, host: bool) -> Result<bool, Stop> {
        let r = self.rng.f64();
        self.clock += if r < 0.02 { 7200 + self.rng.range(0, 3600) as i64 } else { self.rng.range(1, 120) as i64 };
        event.insert("Time".into(), Value::String(time_text(self.clock)));
        let event = Value::Object(event);
        self.events += 1;
        let phase = self.sess.game.current_phase;
        let type_name = event.get("$type").and_then(|t| t.as_str()).unwrap_or("").to_string();
        let mut line = Line::Event { src: src.into(), p, host, event: event.clone(), outcome: Default::default() };
        self.begin(&line);
        let outcome = self.sess.submit(&event.to_string(), host);
        let (outcome, stop) = match outcome {
            Ok(o) => (o, None),
            Err(e) => (trace::EventOutcome { result: "panic".into(), ..Default::default() }, Some(Stop::from(e))),
        };
        let accepted = outcome.result == "accepted";
        let history_changed = outcome.obs.views.is_some();
        if self.reach {
            let _ = self.tx.send(Msg::Text(format!("event\t{type_name}\t{phase}\t{src}\t{}", outcome.result)));
        }
        if let Line::Event { outcome: o, .. } = &mut line {
            *o = outcome;
        }
        self.emit(line);
        if let Some(s) = stop {
            return Err(s);
        }
        if accepted {
            if let Some(f) = &mut self.fuzz {
                if let Some(t) = GameEventType::ALL.iter().find(|t| t.name() == type_name) {
                    f.note_accepted(t.name(), phase);
                }
                match type_name.as_str() {
                    "Battle" => {
                        if let (Value::Object(m), Some(i)) = (&event, event.get("Initiator").and_then(|v| v.as_i64())) {
                            f.plans.insert(i as i32, m.clone());
                        }
                    }
                    "BattleInitiated" | "BattleConcluded" => f.plans.clear(),
                    _ => {}
                }
            }
        }
        if history_changed {
            if let Some(s) = &mut self.shadow {
                s.sync(&self.sess.game, false);
            }
        }
        Ok(accepted)
    }

    fn special(&mut self, kind: &str, until: i32) -> Result<(), Stop> {
        let make = |outcome: trace::SpecialOutcome| match kind {
            "undo" => Line::Undo { until, outcome },
            "restore" => Line::Restore { outcome },
            _ => Line::Dismiss { outcome },
        };
        self.begin(&make(Default::default()));
        match self.sess.special(kind, until) {
            Ok(o) => self.emit(make(o)),
            Err(e) => {
                self.emit(make(trace::SpecialOutcome { panic: true, ..Default::default() }));
                return Err(e.into());
            }
        }
        if let Some(s) = &mut self.shadow {
            s.sync(&self.sess.game, true);
        }
        Ok(())
    }

    /// Queries every player and asks the bots. Returns the choice and the queried types.
    fn deliberate(&mut self) -> Result<(Option<trace::BotChoice>, bool, Vec<(usize, Vec<GameEventType>)>), Stop> {
        let n = self.sess.player_count();
        let mut order: Vec<usize> = (0..n).collect();
        if self.cfg.style != HostStyle::Tests {
            self.rng.shuffle(&mut order);
        }
        let list_as_host = matches!(self.cfg.style, HostStyle::Tests | HostStyle::ServerBotsHost);
        let mut queried = Vec::new();
        for &i in &order {
            let types = self.query(Some(i), list_as_host)?;
            queried.push((i, types));
        }
        let end = self.cfg.style != HostStyle::ServerHumanHost;
        let line = Line::Bots { order: order.clone(), end, created: vec![], result: Default::default() };
        self.begin(&line);
        let game_seed = self.game_seed;
        let mut seed_for = |f: Faction| Some(Rng::new(game_seed ^ (f.0 as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0x424F_54).seed());
        match self.sess.deliberate(&order, end, &mut seed_for) {
            Ok((result, created)) => {
                let error = result.error.clone();
                let choice = result.choice.clone();
                self.emit(Line::Bots { order, end, created, result });
                if let Some(e) = error {
                    return Err(Stop("botException".into(), Some(e)));
                }
                Ok((choice, list_as_host, queried))
            }
            Err(e) => {
                self.emit(Line::Bots { order, end, created: vec![], result: trace::BotsResult { panic: true, ..Default::default() } });
                Err(e.into())
            }
        }
    }

    /// Since v188 the alliance phase waits for every faction to pass, also seated humans, whom no
    /// bot plays to the end. Stand in for a human who hasn't passed yet.
    fn human_pass(&mut self) -> Result<Option<(Map<String, Value>, usize)>, Stop> {
        if !matches!(self.sess.game.current_phase, Phase::AllianceA | Phase::AllianceB) {
            return Ok(None);
        }
        for i in 0..self.sess.player_count() {
            let p = self.sess.game.player_at(i);
            let faction = self.sess.game.player(p).faction;
            if self.sess.game.is_bot(p) || self.sess.game.has_acted_or_passed.contains(&faction) {
                continue;
            }
            let types = self.query(Some(i), false)?;
            if types.contains(&GameEventType::AlliancePassed) {
                let e = AlliancePassed::new(&self.sess.game, faction);
                return Ok(Some((to_object(&treachery_view::build::event(e)), i)));
            }
        }
        Ok(None)
    }

    /// What a human host does when no bot acts.
    fn host_action(&mut self) -> Result<Option<Map<String, Value>>, Stop> {
        let types = self.query(None, true)?;
        let game: &Game = &self.sess.game;
        if types.contains(&GameEventType::FactionSelected) {
            let valid: Vec<Faction> = FactionSelected::valid_factions(game).collect();
            if let Some(p) = game.players.iter().find(|p| p.faction == Faction::None) {
                if !valid.is_empty() {
                    let mut e = FactionSelected::new(game);
                    e.base.initiator.set(Faction::None);
                    e.seat = p.seat;
                    e.faction = valid[self.rng.below(valid.len())];
                    e.initiator_player_name = Some(p.name(game));
                    return Ok(Some(to_object(&treachery_view::build::event(e))));
                }
            }
        }
        if types.contains(&GameEventType::CardsDetermined) {
            let cards: Vec<TreacheryCard> = game.treachery_deck.as_ref().map(|d| d.items.clone()).unwrap_or_default();
            let richese = if game.is_playing(Faction::Richese) { TreacheryCardManager::get_richese_cards() } else { vec![] };
            let mut e = CardsDetermined::new(game, Faction::None);
            e.set_treachery_cards(&cards);
            e.set_richese_cards(&richese);
            return Ok(Some(to_object(&treachery_view::build::event(e))));
        }
        if types.contains(&GameEventType::PerformSetup) {
            let mut e = PerformSetup::new(game, game.next_faction_to_perform_custom_setup());
            e.resources = self.rng.range(0, 21);
            e.force_locations = Some(String::new());
            return Ok(Some(to_object(&treachery_view::build::event(e))));
        }
        if types.contains(&GameEventType::PerformHmsPlacement) && !game.players.is_empty() {
            let p = game.player_at(0);
            let valid: Vec<Location> = PerformHmsPlacement::valid_locations(game, p).collect();
            if !valid.is_empty() {
                let mut e = PerformHmsPlacement::new(game, game.player(p).faction);
                e.set_target(game, Some(valid[self.rng.below(valid.len())]));
                return Ok(Some(to_object(&treachery_view::build::event(e))));
            }
        }
        if types.contains(&GameEventType::EndPhase) {
            return Ok(Some(to_object(&treachery_view::build::event(EndPhase::new(game, Faction::None)))));
        }
        Ok(None)
    }

    /// A fuzz event: usually steered (every player is queried and a type rarely executed in this
    /// phase is picked), otherwise of a type applicable to a random player (or nobody).
    fn random_event(&mut self) -> Result<bool, Stop> {
        let n = self.sess.player_count();
        let f = self.fuzz.as_mut().unwrap();
        let host = f.rng.chance(0.5);
        if f.rng.chance(fuzz::STEERED) {
            let mut applicable = Vec::new();
            for i in 0..n {
                applicable.push((Some(i), self.query(Some(i), host)?));
            }
            applicable.push((None, self.query(None, host)?));
            let phase = self.sess.game.current_phase;
            let f = self.fuzz.as_mut().unwrap();
            let Some((actor, t)) = f.steered_pick(&applicable, phase) else { return Ok(false) };
            let shadow = &self.shadow.as_ref().unwrap().game;
            return match f.build(shadow, actor, t) {
                Some(e) => self.submit(e, "fuzz", actor, host),
                None => Ok(false),
            };
        }
        let actor = if n > 0 && f.rng.chance(0.85) { Some(f.rng.below(n)) } else { None };
        let types = self.query(actor, host)?;
        let phase = self.sess.game.current_phase;
        let f = self.fuzz.as_mut().unwrap();
        // A seatless client only sends what is applicable to it (as host).
        let Some(t) = f.choose_type(&types, phase, actor.is_some()) else { return Ok(false) };
        let shadow = &self.shadow.as_ref().unwrap().game;
        match f.build(shadow, actor, t) {
            Some(e) => self.submit(e, "fuzz", actor, host),
            None => Ok(false),
        }
    }

    fn undo(&mut self) -> Result<(), Stop> {
        let count = self.sess.game.history.len() as i32;
        let f = self.fuzz.as_mut().unwrap();
        let until = if f.rng.chance(0.7) { (count - f.rng.range(1, 6)).max(1) } else { f.rng.range(1, count) };
        self.special("undo", until)?;
        if self.fuzz.as_mut().unwrap().rng.chance(fuzz::RESTORE_OR_DISMISS_AFTER_UNDO) {
            self.restore_or_dismiss()?;
        }
        Ok(())
    }

    fn restore_or_dismiss(&mut self) -> Result<(), Stop> {
        let kind = if self.fuzz.as_mut().unwrap().rng.chance(0.5) { "restore" } else { "dismiss" };
        self.special(kind, 0)
    }

    fn play(&mut self) -> Result<String, Stop> {
        let establish = match self.cfg.establish_players() {
            Value::Object(m) => m,
            _ => unreachable!(),
        };
        if let Some(f) = &mut self.fuzz {
            if f.rng.chance(0.3) {
                let mut m = establish.clone();
                let changed = fuzz::mutate(&mut m, &self.sess.game, None, 1, &mut f.rng);
                if !changed.is_empty() {
                    self.submit(m, "mutate", None, true)?;
                }
            }
        }
        if self.sess.game.current_phase == Phase::AwaitingPlayers {
            self.submit(establish, "host", None, true)?;
            if self.sess.game.current_phase == Phase::AwaitingPlayers {
                return Ok("setupFailed".into());
            }
        }

        if self.fuzz.is_some() {
            // Events of any type right after setup: mostly rejected as not applicable.
            let burst = self.fuzz.as_mut().unwrap().rng.range(0, 7);
            for _ in 0..burst {
                let n = self.sess.player_count();
                let f = self.fuzz.as_mut().unwrap();
                let host = f.rng.chance(0.5);
                let actor = if n > 0 && f.rng.chance(0.85) { Some(f.rng.below(n)) } else { None };
                let t = GameEventType::ALL[f.rng.below(GameEventType::ALL.len())];
                let shadow = &self.shadow.as_ref().unwrap().game;
                if let Some(e) = f.build(shadow, actor, t) {
                    self.submit(e, "fuzz", actor, host)?;
                }
            }
        }

        let mut cap = 400 + self.cfg.turns as usize * self.sess.player_count().max(2) * 80;
        // Fuzz games that reach the oracle's cap are mostly going round in circles; past 3000
        // events they cost a lot of time and add little.
        if self.fuzz.is_some() {
            cap = cap.min(3000);
        }
        let mut rejected_in_a_row = 0;
        while self.sess.game.current_phase != Phase::GameEnded {
            if self.events >= cap {
                return Ok("cap".into());
            }
            if rejected_in_a_row >= if self.fuzz.is_some() { 80 } else { 25 } {
                return Err(Stop("stalled".into(), Some("too many consecutive rejected events".into())));
            }
            if self.fuzz.is_some() {
                let r = self.fuzz.as_mut().unwrap().rng.f64();
                if r < fuzz::UNDO && self.sess.game.history.len() > 3 {
                    self.undo()?;
                    continue;
                }
                if r > 1.0 - fuzz::UNDO / 4.0 {
                    self.restore_or_dismiss()?;
                    continue;
                }
                if r < fuzz::UNDO + fuzz::RANDOM_EVENT {
                    self.random_event()?;
                    continue;
                }
            }

            let (choice, as_host, queried) = self.deliberate()?;

            if self.fuzz.is_some() {
                // A battle plan submitted again with a part changed, when the faction may plan again.
                let f = self.fuzz.as_mut().unwrap();
                if !f.plans.is_empty() && f.rng.chance(fuzz::CHANGED_PLAN) {
                    let shadow = &self.shadow.as_ref().unwrap().game;
                    let replanning: Vec<usize> = queried
                        .iter()
                        .filter(|(i, ts)| ts.contains(&GameEventType::Battle) && shadow.players.get(*i).is_some_and(|p| f.plans.contains_key(&p.faction.0)))
                        .map(|(i, _)| *i)
                        .collect();
                    if let Some(&i) = f.rng.pick(&replanning) {
                        if let Some(e) = f.changed_plan(shadow, i) {
                            self.submit(e, "fuzz", Some(i), as_host)?;
                            continue;
                        }
                    }
                }
                // A battle action for a player in the battle.
                let f = self.fuzz.as_mut().unwrap();
                let shadow = &self.shadow.as_ref().unwrap().game;
                if fuzz::battle_in_progress(shadow) && f.rng.chance(fuzz::BATTLE_ACTION) {
                    let phase = self.sess.game.current_phase;
                    if let Some((i, t)) = f.battle_action(&queried, phase) {
                        if let Some(e) = f.build(shadow, Some(i), t) {
                            self.submit(e, "fuzz", Some(i), as_host)?;
                            continue;
                        }
                    }
                }
            }

            if self.fuzz.is_some() && self.fuzz.as_mut().unwrap().rng.chance(fuzz::OPPORTUNISTIC) {
                let phase = self.sess.game.current_phase;
                let f = self.fuzz.as_mut().unwrap();
                if let Some((actor, t)) = f.opportunistic(&queried, phase) {
                    let shadow = &self.shadow.as_ref().unwrap().game;
                    if let Some(e) = f.build(shadow, Some(actor), t) {
                        self.submit(e, "fuzz", Some(actor), as_host)?;
                        continue;
                    }
                }
            }

            let Some(choice) = choice else {
                if let Some((e, i)) = self.human_pass()? {
                    let ok = self.submit(e, "human", Some(i), false)?;
                    rejected_in_a_row = if ok { 0 } else { rejected_in_a_row + 1 };
                    continue;
                }
                match self.host_action()? {
                    Some(e) => {
                        let ok = self.submit(e, "host", None, true)?;
                        rejected_in_a_row = if ok { 0 } else { rejected_in_a_row + 1 };
                    }
                    None if self.fuzz.is_some() => {
                        let ok = self.random_event()?;
                        rejected_in_a_row = if ok { 0 } else { rejected_in_a_row + 1 };
                    }
                    None => return Err(Stop("stalled".into(), Some("no bot or host event".into()))),
                }
                continue;
            };

            let host = self.cfg.style == HostStyle::Tests || choice.level == "End";
            let event = match choice.event {
                Value::Object(m) => m,
                _ => return Err(Stop("botException".into(), Some("bot event did not serialize".into()))),
            };
            if self.fuzz.is_some() {
                // A bot that keeps sending the same accepted event (AllyPermission when its
                // permissions can't settle) would use up the game's events: mostly let the fuzzer
                // act instead.
                let mut key = event.clone();
                key.remove("Time");
                let g = &self.sess.game;
                let key = (choice.p, format!("{} {} {}", g.current_turn, g.current_phase, Value::Object(key)));
                let f = self.fuzz.as_mut().unwrap();
                f.repeats = if f.last_bot_event.as_ref() == Some(&key) { f.repeats + 1 } else { 0 };
                f.last_bot_event = Some(key);
                if f.repeats >= 2 && f.rng.chance(0.8) {
                    let ok = self.random_event()?;
                    rejected_in_a_row = if ok { 0 } else { rejected_in_a_row + 1 };
                    continue;
                }
            }
            if self.fuzz.is_some() && self.fuzz.as_mut().unwrap().rng.chance(fuzz::MUTATE_BOT_EVENT) {
                let mut m = event.clone();
                let f = self.fuzz.as_mut().unwrap();
                let shadow = &self.shadow.as_ref().unwrap().game;
                let t = m.get("$type").and_then(|t| t.as_str()).and_then(|n| GameEventType::ALL.iter().copied().find(|t| t.name() == n));
                let changed = match t {
                    Some(t) => fuzz::mutate_steered(&mut m, shadow, choice.p, t, &mut f.rng),
                    None => fuzz::mutate(&mut m, shadow, Some(choice.p), 1, &mut f.rng),
                };
                if !changed.is_empty() {
                    self.submit(m, "mutate", Some(choice.p), host)?;
                    continue;
                }
            }
            let ok = self.submit(event, "bot", Some(choice.p), host)?;
            rejected_in_a_row = if ok { 0 } else { rejected_in_a_row + 1 };
        }
        Ok("gameEnded".into())
    }
}

pub struct RecordOptions {
    pub mode: String,
    pub seed: u64,
    pub index: u64,
    pub full_views: bool,
    pub budgets: Budgets,
    /// Write reach lines (`event`/`offered`/`end`, tab-separated, for `observe reach`) instead of
    /// the trace.
    pub reach: bool,
}

/// Records one game and writes its trace to `out`. Returns the end reason.
pub fn record_game(o: &RecordOptions, out: &mut dyn Write) -> std::io::Result<String> {
    let (mode, seed, index, full_views, reach) = (o.mode.clone(), o.seed, o.index, o.full_views, o.reach);
    let mut reason = String::from("?");
    let mut error: Option<std::io::Error> = None;
    let mut write = |text: Option<String>, out: &mut dyn Write| {
        let Some(text) = text else { return };
        if error.is_none() {
            if let Err(e) = writeln!(out, "{text}") {
                error = Some(e);
            }
        }
    };
    let line_text = |line: &Line| match line {
        _ if !reach => Some(line.to_json()),
        Line::End { reason, .. } => Some(format!("end\t{reason}")),
        _ => None,
    };
    let ending = watch::run(
        &o.budgets,
        move |tx| {
            crate::session::quiet_panics();
            let game_seed = rng::game_seed(&mode, seed, index);
            let mut rng = Rng::new(game_seed);
            let fuzzing = mode == "fuzz";
            let cfg = Config::randomize(&mut rng, fuzzing);
            let sess = Session::new(Game::LATEST_VERSION, &cfg.seated, full_views);
            let header = Header {
                format: trace::FORMAT,
                mode: mode.clone(),
                seed,
                index,
                version: Game::LATEST_VERSION,
                seated: cfg.seated.clone(),
                config: cfg.to_json(),
                full_views,
            };
            let _ = tx.send(Msg::Line(Line::Header(header)));
            let shadow = fuzzing.then(|| Shadow::new(Game::LATEST_VERSION, &sess.participation));
            let fuzz = fuzzing.then(|| Fuzzer::new(rng.next_u64()));
            let mut r = Recorder { tx, sess, rng, game_seed, cfg, fuzz, shadow, clock: 0, events: 0, reach };
            // A panic in the recorder's own reads (host actions, fuzz values) ends the game; the
            // replayer stops at the end line like for any other reason.
            let played = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| r.play()));
            let (reason, detail) = match played {
                Ok(Ok(reason)) => (reason, None),
                Ok(Err(Stop(reason, detail))) => (reason, detail),
                Err(_) => ("recorderPanic".to_string(), None),
            };
            let _ = tx.send(Msg::Line(Line::End { reason, detail, pending: None }));
        },
        |m| match m {
            Msg::Line(l) => {
                if let Line::End { reason: r, .. } = &l {
                    reason = r.clone();
                }
                write(line_text(&l), out);
            }
            Msg::Text(t) if reach => write(Some(t), out),
            _ => {}
        },
    );
    let end = match ending {
        Ending::Finished => None,
        Ending::Hung(p) => Some(("hung", p)),
        Ending::MemoryLimit(p) => Some(("memoryLimit", p)),
    };
    if let Some((r, pending)) = end {
        reason = r.to_string();
        write(line_text(&Line::End { reason: r.into(), detail: None, pending: pending.map(Box::new) }), out);
    }
    if let Some(e) = error {
        return Err(e);
    }
    Ok(reason)
}
