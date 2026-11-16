//! Reading and writing the logs `observe refit` carries over: savegames (`scripts/fixtures`),
//! recorded traces (`kind` lines, `rust/testdata/games` and `rust/view/tests/data`) and the bot
//! tests' knowledge games (`k` lines, with the probes made at each step).

use std::path::Path;

use serde_json::{Map, Value};
use treachery_engine::enum_type::value_or_absent;
use treachery_engine::game::Game;

use crate::refit::{refit, End, Item, Refit};

/// `[userId, seat, name]` of each seated user, from a header's participation (.NET
/// `Participation` JSON). A user without a name is "?".
pub fn seated(o: &Value) -> Vec<(i32, i32, String)> {
    let int = |v: &Value| v.as_i64().unwrap_or(0) as i32;
    let list = |k: &str| o[k].as_array().cloned().unwrap_or_default();
    let names: Vec<(i32, String)> = list("PlayerNames").iter().map(|kv| (int(&kv[0]), kv[1].as_str().unwrap_or("").to_string())).collect();
    let mut seated: Vec<(i32, i32, String)> = Vec::new();
    for kv in list("SeatedPlayers") {
        let (user, seat) = (int(&kv[0]), int(&kv[1]));
        let name = names.iter().rev().find(|(u, _)| *u == user).map_or("?".to_string(), |(_, n)| n.clone());
        match seated.iter_mut().find(|(u, _, _)| *u == user) {
            Some(entry) => entry.1 = seat,
            None => seated.push((user, seat, name)),
        }
    }
    seated
}

/// What the old engine saw of a log (`--old`): per event whether it was valid and the turn and
/// main phase it was executed in, and where the game ended.
#[derive(Clone, Debug, Default)]
pub struct Old {
    pub legit: Vec<bool>,
    pub at: Vec<(i32, i32)>,
    pub end: End,
    pub turn: Option<i32>,
    /// The faction of each player, in the order they joined.
    pub factions: Option<Vec<i32>>,
    /// Each player's ally at the end.
    pub allies: Option<Vec<i32>>,
    /// Actions the refitted game should offer at the end, as (faction, event type name). A
    /// design fixture is made for the actions it ends on (`karma-monster` for Fremen's
    /// `KarmaMonster`), and a refit that ends in the right phase may still not offer them.
    pub offers: Vec<(i32, String)>,
}

impl Old {
    pub fn from_json(v: &Value) -> Old {
        let legit = v["legit"].as_array().map(|a| a.iter().map(|b| b.as_bool().unwrap_or(true)).collect()).unwrap_or_default();
        let at = v["at"].as_array().map(|a| a.iter().map(|p| (p[0].as_i64().unwrap_or(0) as i32, p[1].as_i64().unwrap_or(0) as i32)).collect()).unwrap_or_default();
        let turn = v["turn"].as_i64().map(|t| t as i32);
        let position = turn.zip(v["main"].as_i64().map(|m| m as i32));
        let factions = v["factions"].as_array().map(|a| a.iter().map(|f| f.as_i64().unwrap_or(0) as i32).collect());
        let allies = v["allies"].as_array().map(|a| a.iter().map(|f| f.as_i64().unwrap_or(0) as i32).collect());
        let offers = v["offers"]
            .as_array()
            .map(|a| a.iter().map(|o| (o[0].as_i64().unwrap_or(0) as i32, o[1].as_str().unwrap_or("").to_string())).collect())
            .unwrap_or_default();
        Old { factions, allies, offers, legit, at, end: End { position, phase: v["phase"].as_str().map(str::to_string), battle: v["battle"].as_bool() }, turn }
    }

    /// How many of `offers` the refitted game doesn't offer at its end.
    fn offers_missing(&self, r: &mut Refit) -> usize {
        let game = &mut r.game;
        self.offers
            .iter()
            .filter(|(f, t)| match game.get_player(treachery_engine::enum_type::from_value_or_absent::<treachery_engine::prelude::Faction>(*f).ok().flatten()) {
                Some(p) => !game.get_applicable_events(Some(p), false).iter().any(|e| e.name() == t),
                None => true,
            })
            .count()
    }

    /// Lower is better: a different phase, battle, faction in a seat or alliance counts most, then a different turn, then the changes.
    pub fn score(&self, r: &mut Refit) -> (u8, usize, u8, usize) {
        let seats = self.factions.as_ref().is_some_and(|f| *f != r.game.players.iter().map(|p| value_or_absent(p.faction)).collect::<Vec<_>>());
        let allies = self.allies.as_ref().is_some_and(|a| *a != r.game.players.iter().map(|p| value_or_absent(p.ally)).collect::<Vec<_>>());
        let phase = (!self.end.reached(&r.game) || seats || allies) as u8;
        let turn = self.turn.is_some_and(|t| t != i32::from(r.game.turn.current_turn)) as u8;
        (phase, self.offers_missing(r), turn, r.stats.dropped * 4 + r.stats.replaced + r.stats.added)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Savegame,
    Trace,
    Knowledge,
}

/// A log read from a file.
pub struct Log {
    pub kind: Kind,
    /// `[userId, seat, name]` of each seated user.
    pub seated: Vec<(i32, i32, String)>,
    pub items: Vec<Item>,
    /// For traces and knowledge games, the input line of each item.
    lines: Vec<Value>,
    /// The other lines (header, end, refused steps, probes), each with the index of the item it
    /// follows (-1: before all).
    other: Vec<(isize, Value)>,
    /// A savegame's top-level object.
    savegame: Option<Map<String, Value>>,
}

pub fn read_lines(path: &Path) -> Result<Vec<Value>, String> {
    use std::io::{BufRead, BufReader, Read};
    let file = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let reader: Box<dyn Read> = if path.extension().is_some_and(|e| e == "gz") { Box::new(flate2::read::GzDecoder::new(file)) } else { Box::new(file) };
    BufReader::new(reader).lines().map(|l| l.map_err(|e| e.to_string()).and_then(|l| serde_json::from_str(&l).map_err(|e| e.to_string()))).collect()
}

pub fn write_lines(path: &Path, lines: &[Value]) -> Result<(), String> {
    use std::io::Write;
    let file = std::fs::File::create(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut w: Box<dyn Write> = if path.extension().is_some_and(|e| e == "gz") {
        Box::new(flate2::write::GzEncoder::new(file, flate2::Compression::best()))
    } else {
        Box::new(std::io::BufWriter::new(file))
    };
    for l in lines {
        writeln!(w, "{}", serde_json::to_string(l).unwrap()).map_err(|e| e.to_string())?;
    }
    w.flush().map_err(|e| e.to_string())
}

fn event_object(v: &Value) -> Map<String, Value> {
    match v {
        Value::Object(m) => m.clone(),
        Value::String(s) => match serde_json::from_str(s) {
            Ok(Value::Object(m)) => m,
            _ => Map::new(),
        },
        _ => Map::new(),
    }
}

/// Reads a savegame (`.json`), a recorded trace or a knowledge game. Without `old`, every event
/// counts as valid, except a trace step that wasn't accepted.
pub fn read(path: &Path, old: &Old) -> Result<Log, String> {
    let legit_at = |i: usize| old.legit.get(i).copied().unwrap_or(true);
    let old_at = |i: usize| old.at.get(i).copied();
    if path.extension().is_some_and(|e| e == "json") {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let Value::Object(top) = serde_json::from_str(&text).map_err(|e| e.to_string())? else { return Err("not a savegame".into()) };
        let events = top.get("_events").and_then(|v| v.as_array()).cloned().unwrap_or_default();
        let items = events.iter().enumerate().map(|(i, e)| Item::Event { event: event_object(e), validate: true, host: true, legit: legit_at(i), at: old_at(i) }).collect();
        return Ok(Log { kind: Kind::Savegame, seated: vec![], items, lines: vec![], other: vec![], savegame: Some(top) });
    }
    let all = read_lines(path)?;
    let knowledge = all.first().is_some_and(|l| l.get("k").is_some());
    let kind_key = if knowledge { "k" } else { "kind" };
    let mut log = Log { kind: if knowledge { Kind::Knowledge } else { Kind::Trace }, seated: vec![], items: vec![], lines: vec![], other: vec![], savegame: None };
    let mut accepted_events = 0;
    for l in all {
        let at = log.items.len() as isize - 1;
        let item = match l[kind_key].as_str().unwrap_or("") {
            "header" => {
                if let Some(p) = l.get("participation") {
                    log.seated = seated(p);
                }
                None
            }
            // A refused step stays where it was, refused (`write` drops it if it would pass now).
            "step" if !knowledge && l["result"] != "accepted" => None,
            "step" => {
                let item = Item::Event {
                    event: event_object(&l["event"]),
                    validate: l["validate"].as_bool().unwrap_or(true),
                    host: l["isHost"].as_bool().unwrap_or(false),
                    legit: legit_at(accepted_events),
                    at: old_at(accepted_events),
                };
                accepted_events += 1;
                Some(item)
            }
            "undo" => Some(Item::Undo(l["untilEventNr"].as_i64().unwrap_or(0) as i32)),
            "restoreRecentlyUndone" => Some(Item::Restore),
            "dismissRecentlyUndone" => Some(Item::Dismiss),
            _ => None,
        };
        match item {
            Some(item) => {
                log.items.push(item);
                log.lines.push(l);
            }
            None => log.other.push((at, l)),
        }
    }
    Ok(log)
}

fn own_seed(log: &Log) -> Option<i64> {
    log.items.iter().find_map(|i| match i {
        Item::Event { event, .. } if event.get("$type").and_then(|v| v.as_str()) == Some("EstablishPlayers") => event.get("Seed").and_then(|v| v.as_i64()),
        _ => None,
    })
}

/// Refits a log, trying up to `seeds` seeds for `EstablishPlayers` (the log's own first), and
/// keeps the result closest to `target`. Returns the seed used if it isn't the log's own.
pub fn refit_best(log: &Log, target: &Old, seeds: usize) -> (Refit, Option<i32>) {
    let version = Game::LATEST_VERSION;
    let Some(own) = own_seed(log) else { return (refit(version, &log.seated, &log.items, &target.end, None), None) };
    let candidates = seats_kept(log, target, own as i32, seeds);
    let mut best: Option<(_, Refit, Option<i32>)> = None;
    for seed in candidates {
        let mut r = refit(version, &log.seated, &log.items, &target.end, seed);
        let score = target.score(&mut r);
        if best.as_ref().is_none_or(|(b, _, _)| score < *b) {
            best = Some((score, r, seed));
        }
    }
    let (_, r, seed) = best.expect("at least the log's own seed");
    (r, seed)
}

/// Up to `n` seeds to try: the log's own (`None`) and the next ones. When the old log's factions
/// are known, only seeds that seat the same factions in the same seats during setup, found by
/// refitting just the setup events (up to 20,000 seeds, so the result may be empty: then the own
/// seed and the next ones).
fn seats_kept(log: &Log, target: &Old, own: i32, n: usize) -> Vec<Option<i32>> {
    let plain: Vec<Option<i32>> = (0..n).map(|k| if k == 0 { None } else { Some(own.wrapping_add(k as i32)) }).collect();
    let Some(factions) = &target.factions else { return plain };
    let setup = target.at.iter().take_while(|p| p.0 == 0).count().min(log.items.len());
    let seats = |r: &Refit| r.game.players.iter().map(|p| value_or_absent(p.faction)).collect::<Vec<_>>() == *factions;
    let none = End::default();
    let mut found = Vec::new();
    for k in 0..20_000 {
        let seed = if k == 0 { None } else { Some(own.wrapping_add(k)) };
        if seats(&refit(Game::LATEST_VERSION, &log.seated, &log.items[..setup], &none, seed)) {
            found.push(seed);
            if found.len() >= n {
                break;
            }
        }
    }
    if found.is_empty() { plain } else { found }
}

/// Writes the refitted log in the input's format, at the latest game version. The other lines
/// (probes, refused steps, the end) follow the item they followed before; steps are renumbered,
/// and a probe takes the number of the step it follows.
pub fn write(path: &Path, log: &Log, r: &Refit) -> Result<(), String> {
    let version = Game::LATEST_VERSION;
    if let Some(top) = &log.savegame {
        let mut top = top.clone();
        top.insert("Version".into(), Value::from(version));
        let events: Vec<Value> = r
            .out
            .iter()
            .filter_map(|o| match &o.item {
                Item::Event { event, .. } => Some(Value::Object(event.clone())),
                _ => None,
            })
            .collect();
        top.insert("_events".into(), Value::Array(events));
        return std::fs::write(path, serde_json::to_string(&Value::Object(top)).unwrap()).map_err(|e| e.to_string());
    }
    let knowledge = log.kind == Kind::Knowledge;
    let mut by_item: Vec<Vec<Value>> = vec![Vec::new(); log.items.len()];
    let mut added: Vec<Value> = Vec::new();
    for o in &r.out {
        let line = match (&o.item, o.from) {
            (Item::Event { event, validate, host, .. }, Some(i)) => {
                let mut l = log.lines[i].clone();
                l["event"] = if knowledge { Value::String(Value::Object(event.clone()).to_string()) } else { Value::Object(event.clone()) };
                if l.get("validate").is_some() || !*validate {
                    l["validate"] = Value::from(*validate);
                }
                l["isHost"] = Value::from(*host);
                l
            }
            (Item::Event { event, validate, host, .. }, None) => {
                if knowledge {
                    serde_json::json!({"k": "step", "n": 0, "validate": validate, "isHost": host, "event": Value::Object(event.clone()).to_string()})
                } else {
                    serde_json::json!({"kind": "step", "n": 0, "source": "bot", "validate": validate, "isHost": host, "queries": [], "event": Value::Object(event.clone()), "result": "accepted"})
                }
            }
            (Item::Undo(n), Some(i)) => {
                let mut l = log.lines[i].clone();
                l["untilEventNr"] = Value::from(*n);
                l
            }
            (_, Some(i)) => log.lines[i].clone(),
            (_, None) => continue,
        };
        match o.from {
            Some(i) => {
                by_item[i].append(&mut added);
                by_item[i].push(line);
            }
            None => added.push(line),
        }
    }

    // The game the refused steps are checked against: the refitted log replayed so far.
    let mut out: Vec<Value> = Vec::new();
    let mut other = log.other.iter().peekable();
    let mut checker = Checker::new(&log.seated);
    let mut place_other = |out: &mut Vec<Value>, at: isize, checker: &mut Checker| {
        while let Some((a, l)) = other.peek() {
            if *a != at {
                break;
            }
            let mut l = l.clone();
            other.next();
            let key = if knowledge { "k" } else { "kind" };
            match l[key].as_str() {
                Some("header") => {
                    if knowledge {
                        l["version"] = Value::from(version);
                    } else {
                        l["gameVersion"] = Value::from(version);
                    }
                }
                Some("step") if !checker.refused(&l) => continue,
                _ => {}
            }
            out.push(l);
        }
    };
    place_other(&mut out, -1, &mut checker);
    for (i, lines) in by_item.into_iter().enumerate() {
        for l in lines {
            checker.apply(&l, knowledge);
            out.push(l);
        }
        place_other(&mut out, i as isize, &mut checker);
    }
    for l in added {
        out.push(l);
    }

    let mut n: i64 = -1;
    let mut lines_n: i64 = 0;
    for l in &mut out {
        if knowledge {
            if l["k"] == "step" {
                n += 1;
            }
            if l.get("n").is_some() {
                l["n"] = Value::from(n.max(0));
            }
        } else if l.get("n").is_some() {
            l["n"] = Value::from(lines_n);
            lines_n += 1;
        }
    }
    write_lines(path, &out)
}

/// For each input event, the number of the step it became in the refitted log (a dropped event
/// maps to the step before it). Probes made "after step n" move with it.
pub fn step_map(log: &Log, r: &Refit) -> Vec<i64> {
    let mut map: Vec<Option<i64>> = vec![None; log.items.len()];
    let mut n: i64 = -1;
    for o in &r.out {
        if matches!(o.item, Item::Event { .. }) {
            n += 1;
            if let Some(i) = o.from {
                map[i] = Some(n);
            }
        }
    }
    let mut last = 0;
    map.into_iter()
        .map(|m| {
            last = m.unwrap_or(last);
            last
        })
        .collect()
}

/// Rewrites the step numbers of a probe file (`n`) with `step_map`'s numbers.
pub fn remap_probes(path: &Path, out: &Path, map: &[i64]) -> Result<(), String> {
    let mut lines = read_lines(path)?;
    for l in &mut lines {
        if let Some(n) = l.get("n").and_then(|v| v.as_u64()) {
            l["n"] = Value::from(map.get(n as usize).copied().unwrap_or(*map.last().unwrap_or(&0)));
        }
    }
    write_lines(out, &lines)
}

/// Replays written lines, to tell whether a refused step would still be refused.
struct Checker {
    game: Game,
}

impl Checker {
    fn new(seated: &[(i32, i32, String)]) -> Checker {
        let mut game = Game::new();
        game.human_seats = crate::session::seats(seated).0;
        Checker { game }
    }

    fn event(l: &Value) -> Option<treachery_engine::events::HistoryEntry> {
        match &l["event"] {
            Value::String(s) => crate::events::decode(&serde_json::from_str(s).ok()?).ok(),
            v => crate::events::decode(v).ok(),
        }
    }

    fn apply(&mut self, l: &Value, knowledge: bool) {
        let key = if knowledge { "k" } else { "kind" };
        let game = &mut self.game;
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match l[key].as_str() {
            Some("step") => {
                if let Some(e) = Self::event(l) {
                    let _ = game.play(e, l["validate"].as_bool().unwrap_or(true), l["isHost"].as_bool().unwrap_or(false));
                }
            }
            Some("undo") => game.undo(l["untilEventNr"].as_i64().unwrap_or(0) as i32),
            Some("restoreRecentlyUndone") => game.restore_recently_undone(),
            Some("dismissRecentlyUndone") => game.dismiss_recently_undone(),
            _ => {}
        }));
    }

    /// Whether the step would be refused now (it doesn't pass validation, or it throws).
    fn refused(&self, l: &Value) -> bool {
        let Some(e) = Self::event(l) else { return true };
        let game = &self.game;
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            e.validate(game).is_err()
        }))
        .unwrap_or(true)
    }
}
