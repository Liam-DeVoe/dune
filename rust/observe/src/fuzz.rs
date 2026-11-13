//! Fuzz event generation for the recorder, in the spirit of the C# oracle's `client` profile
//! (treachery-online/Oracle/Fuzzer*.cs): events of an applicable type, for the player's own
//! faction, starting from the draft the client's form would send (`treachery_view::actions::
//! default_draft`), with some fields changed to real values of the game: the player's cards and
//! leaders, locations and territories of the map, factions in play, small amounts. Candidates
//! that pass validation are preferred, but invalid ones are submitted now and then.
//!
//! The generator reads a shadow copy of the game ([`Shadow`]), never the session's game, so it
//! can't change state the replayer wouldn't change: drafts and validation fill caches (hazards.md
//! 6.1, 4.3). Only the recorder runs it; the replayer reads its events from the trace.

use std::collections::BTreeMap;
use std::panic::{self, AssertUnwindSafe};

use serde_json::{Map, Value};
use treachery_engine::cs;
use treachery_engine::events::{GameEvent, GameEventType};
use treachery_engine::game::Game;
use treachery_engine::json;
use treachery_engine::participation::Participation;
use treachery_engine::prelude::*;
use treachery_view::build::ResolvedViewer;

use crate::rng::Rng;

/// A copy of the session's game, kept in step by executing its history without validation.
pub struct Shadow {
    pub game: Game,
    participation: Shared<Participation>,
}

impl Shadow {
    pub fn new(version: i32, participation: &Shared<Participation>) -> Shadow {
        Shadow { game: Game::new_int32_participation(version, participation), participation: participation.clone() }
    }

    /// Catches up with `game`; starts over when `rebuild` or when it is ahead.
    pub fn sync(&mut self, game: &Game, rebuild: bool) {
        if rebuild || self.game.event_count() > game.event_count() {
            self.game = Game::new_int32_participation(game.version, &self.participation);
        }
        let done = self.game.event_count() as usize;
        for e in game.history.iter().skip(done) {
            let text = json::serialize(e);
            let shadow = &mut self.game;
            let ok = quiet(|| {
                let copy: GameEvent = json::deserialize(&text).ok()?;
                copy.initialize_game(shadow);
                copy.execute(shadow, false, false);
                Some(())
            });
            if ok.flatten().is_none() {
                // A shadow that can't follow is rebuilt next time; generation just gets worse.
                self.game = Game::new_int32_participation(game.version, &self.participation);
                return;
            }
        }
    }
}

fn quiet<R>(f: impl FnOnce() -> R) -> Option<R> {
    panic::catch_unwind(AssertUnwindSafe(|| cs::catch(AssertUnwindSafe(f)).ok())).ok().flatten()
}

/// Values of the game a client could put into an event.
struct Domains {
    own_cards: Vec<i32>,
    held_cards: Vec<i32>,
    all_cards: Vec<i32>,
    own_heroes: Vec<i32>,
    all_heroes: Vec<i32>,
    own_locations: Vec<i32>,
    locations: Vec<i32>,
    strongholds: Vec<i32>,
    territories: Vec<i32>,
    factions: Vec<i32>,
}

impl Domains {
    fn of(game: &Game, player: Option<usize>) -> Domains {
        let me = player.filter(|i| *i < game.players.len()).map(|i| &game.players[i]);
        let own_cards = me.map(|p| p.treachery_cards.iter().map(|c| c.id()).collect()).unwrap_or_default();
        let held_cards = game.players.iter().flat_map(|p| p.treachery_cards.iter().map(|c| c.id())).collect();
        let all_cards = TreacheryCardManager::get_cards_in_and_outside_play().iter().map(|c| c.id()).collect();
        let mut own_heroes: Vec<i32> = me.map(|p| p.leaders.iter().map(|l| l.id()).collect()).unwrap_or_default();
        if let Some(p) = me {
            own_heroes.extend(p.traitors.iter().map(|h| h.id()));
        }
        let all_heroes = game.players.iter().flat_map(|p| p.leaders.iter().map(|l| l.id())).collect();
        let own_locations = me.map(|p| p.forces_in_locations.keys().map(|l| l.id()).collect()).unwrap_or_default();
        let locations = game.map.locations(false).map(|l| l.id()).collect();
        let strongholds = game.map.strongholds().map(|l| l.id()).collect();
        let territories = game.map.territories(false).map(|t| t.id()).collect();
        let factions = game.players.iter().map(|p| p.faction.0).filter(|f| *f != 0).collect();
        Domains { own_cards, held_cards, all_cards, own_heroes, all_heroes, own_locations, locations, strongholds, territories, factions }
    }
}

fn pick(rng: &mut Rng, lists: &[(&[i32], f64)]) -> Option<i32> {
    for (list, p) in lists {
        if !list.is_empty() && rng.chance(*p) {
            return rng.pick(list).copied();
        }
    }
    lists.iter().rev().find(|(l, _)| !l.is_empty()).and_then(|(l, _)| rng.pick(l).copied())
}

fn amount(rng: &mut Rng) -> i32 {
    if rng.chance(0.75) { rng.range(0, 6) } else { rng.range(6, 21) }
}

/// Fields whose values are other enums or free text: left as the draft has them, because the
/// client only offers defined values there and this generator doesn't know which enum a field is.
fn is_amount(k: &str) -> bool {
    ["amount", "forces", "resources", "price", "bid", "contribution", "bonus", "valueadded", "towin", "toreplace", "turn", "seat", "direction", "numberof", "permitted", "losses", "halfstrength", "advisor"]
        .iter()
        .any(|s| k.contains(s))
}

/// A value for field `key` (current value `cur`), or None when the field isn't fuzzed.
fn value_for(key: &str, cur: &Value, d: &Domains, rng: &mut Rng) -> Option<Value> {
    let k = key.to_ascii_lowercase();
    if matches!(k.as_str(), "initiator" | "time" | "$type" | "_players" | "_factionsinplay" | "settings" | "seed" | "initiatorplayername") {
        return None;
    }
    if cur.is_boolean() {
        return Some(Value::Bool(!cur.as_bool().unwrap()));
    }
    if cur.is_string() || cur.is_null() {
        if k.ends_with("forcelocations") {
            let mut parts = Vec::new();
            for _ in 0..rng.range(1, 4) {
                let loc = pick(rng, &[(&d.own_locations, 0.6), (&d.locations, 1.0)])?;
                parts.push(format!("{loc}:{}|{}", rng.range(0, 6), if rng.chance(0.7) { 0 } else { rng.range(0, 4) }));
            }
            return Some(Value::String(parts.join(",")));
        }
        if k.ends_with("cardids") {
            let mut ids = Vec::new();
            for _ in 0..rng.range(0, 4) {
                ids.extend(pick(rng, &[(&d.own_cards, 0.8), (&d.held_cards, 1.0)]));
            }
            return Some(Value::String(ids.iter().map(|i| i.to_string()).collect::<Vec<_>>().join(";")));
        }
        return None;
    }
    if !cur.is_i64() {
        return None;
    }
    let v = if k.contains("card") {
        pick(rng, &[(&d.own_cards, 0.6), (&d.held_cards, 0.6), (&d.all_cards, 1.0)])?
    } else if k.contains("hero") || k.contains("traitor") || k.contains("leader") || k.contains("dancer") {
        pick(rng, &[(&d.own_heroes, 0.6), (&d.all_heroes, 1.0)])?
    } else if k.contains("territory") || k.contains("stronghold") {
        if k.contains("stronghold") { pick(rng, &[(&d.strongholds, 0.5), (&d.territories, 1.0)])? } else { pick(rng, &[(&d.territories, 1.0)])? }
    } else if k.starts_with('_') && ["to", "from", "location", "target", "sneak"].iter().any(|s| k.contains(s)) {
        pick(rng, &[(&d.own_locations, 0.4), (&d.strongholds, 0.3), (&d.locations, 1.0)])?
    } else if k == "target" || k.contains("faction") {
        pick(rng, &[(&d.factions, 1.0)])?
    } else if is_amount(&k) {
        amount(rng)
    } else {
        return None;
    };
    Some(Value::from(v))
}

/// Changes up to `count` fuzzable fields of `event`. Returns the names of the fields changed.
pub fn mutate(event: &mut Map<String, Value>, game: &Game, player: Option<usize>, count: usize, rng: &mut Rng) -> Vec<String> {
    // A game in a broken state (a null card in a hand) can make even these reads throw.
    let Some(d) = quiet(|| Domains::of(game, player)) else { return vec![] };
    let keys: Vec<String> = event.keys().cloned().collect();
    let mut changed = Vec::new();
    for _ in 0..count * 3 {
        if changed.len() >= count {
            break;
        }
        let Some(k) = rng.pick(&keys).cloned() else { break };
        if let Some(v) = value_for(&k, &event[&k], &d, rng) {
            event.insert(k.clone(), v);
            changed.push(k);
        }
    }
    changed
}

/// Generates events and remembers which (type, phase) pairs were executed, to prefer rare ones.
pub struct Fuzzer {
    pub rng: Rng,
    accepted: BTreeMap<(&'static str, i32), u32>,
    opportunistic_tries: BTreeMap<(&'static str, i32), u32>,
}

pub const RANDOM_EVENT: f64 = 0.12;
pub const MUTATE_BOT_EVENT: f64 = 0.10;
pub const UNDO: f64 = 0.004;
pub const RESTORE_OR_DISMISS_AFTER_UNDO: f64 = 0.3;
pub const OPPORTUNISTIC: f64 = 0.3;

impl Fuzzer {
    pub fn new(seed: u64) -> Fuzzer {
        Fuzzer { rng: Rng::new(seed), accepted: BTreeMap::new(), opportunistic_tries: BTreeMap::new() }
    }

    pub fn note_accepted(&mut self, t: &'static str, phase: Phase) {
        *self.accepted.entry((t, phase.0)).or_default() += 1;
    }

    fn rare(&self, t: GameEventType, phase: Phase) -> bool {
        self.accepted.get(&(t.name(), phase.0)).copied().unwrap_or(0) < 2
    }

    /// An applicable type to try, preferring types rarely executed in this phase; any type now and then.
    /// None when nothing is applicable and `any` is false.
    pub fn choose_type(&mut self, applicable: &[GameEventType], phase: Phase, any: bool) -> Option<GameEventType> {
        if any && (applicable.is_empty() || self.rng.chance(0.1)) {
            return Some(GameEventType::ALL[self.rng.below(GameEventType::ALL.len())]);
        }
        if applicable.is_empty() {
            return None;
        }
        let rare: Vec<GameEventType> = applicable.iter().copied().filter(|t| self.rare(*t, phase)).collect();
        if !rare.is_empty() && self.rng.chance(0.6) {
            return Some(rare[self.rng.below(rare.len())]);
        }
        Some(applicable[self.rng.below(applicable.len())])
    }

    /// A (player, type) whose type is applicable and rarely executed in this phase, if any.
    pub fn opportunistic(&mut self, applicable: &[(usize, Vec<GameEventType>)], phase: Phase) -> Option<(usize, GameEventType)> {
        let candidates: Vec<(usize, GameEventType)> = applicable
            .iter()
            .flat_map(|(i, ts)| ts.iter().map(move |t| (*i, *t)))
            .filter(|(_, t)| self.rare(*t, phase) && self.opportunistic_tries.get(&(t.name(), phase.0)).copied().unwrap_or(0) < 15)
            .collect();
        let pick = *self.rng.pick(&candidates)?;
        *self.opportunistic_tries.entry((pick.1.name(), phase.0)).or_default() += 1;
        Some(pick)
    }

    /// Builds an event of type `t` for `player` (None: no faction) from the shadow game: several
    /// candidates, the form's draft with some fields changed. Returns the event JSON object
    /// (without `Time`), preferring one that passes validation.
    pub fn build(&mut self, shadow: &Game, player: Option<usize>, t: GameEventType) -> Option<Map<String, Value>> {
        let faction = player.filter(|i| *i < shadow.players.len()).map(|i| shadow.players[i].faction).unwrap_or(Faction::None);
        let draft = player.filter(|i| *i < shadow.players.len()).and_then(|i| {
            let p = shadow.player_at(i);
            let rv = ResolvedViewer { player: Some(p), faction: Some(faction), is_host: false, is_observer: false };
            quiet(|| treachery_view::actions::default_draft(shadow, &rv, faction, t).map(|e| json::serialize(&e))).flatten()
        });
        let base = draft.unwrap_or_else(|| {
            let minimal = format!("{{\"$type\":\"{}\",\"Initiator\":{}}}", t.name(), faction.0);
            quiet(|| json::deserialize::<GameEvent>(&minimal).ok().map(|e| json::serialize(&e))).flatten().unwrap_or(minimal)
        });
        let Ok(Value::Object(mut base)) = serde_json::from_str::<Value>(&base) else { return None };
        // A form with a faction to pick has one picked: another faction in play.
        if base.get("Target").and_then(|v| v.as_i64()) == Some(0) {
            let others: Vec<i32> = shadow.players.iter().map(|p| p.faction.0).filter(|f| *f != 0 && *f != faction.0).collect();
            if let Some(f) = self.rng.pick(&others) {
                base.insert("Target".into(), Value::from(*f));
            }
        }

        let mut last = None;
        for attempt in 0..6 {
            let mut candidate = base.clone();
            let count = match self.rng.f64() {
                x if x < 0.35 && attempt == 0 => 0,
                x if x < 0.7 => 1,
                x if x < 0.9 => 2,
                _ => 3,
            };
            mutate(&mut candidate, shadow, player, count, &mut self.rng);
            let text = Value::Object(candidate.clone()).to_string();
            let valid = quiet(|| {
                let e: GameEvent = json::deserialize(&text).ok()?;
                e.initialize_game(shadow);
                Some(e.validate(shadow).is_none())
            })
            .flatten()
            .unwrap_or(false);
            if valid && self.rng.chance(0.85) {
                return Some(candidate);
            }
            last = Some(candidate);
        }
        last
    }
}
