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
use treachery_engine::compat::cs;
use treachery_engine::events::{GameEvent, GameEventType};
use treachery_engine::game::Game;
use treachery_engine::json;
use treachery_engine::game::Participation;
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
#[derive(Clone, Default)]
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

/// What the client's form for one event type offers, read from the view's action options
/// (`treachery_view::actions::action_view`): the draft the form sends untouched, lists of
/// choices, and bounds of number pickers. The generator fills event fields from these, so every
/// value is one the form could send.
#[derive(Default)]
pub struct Pools {
    /// The draft the form sends (the default draft after the form adjusted it).
    pub draft: Option<Map<String, Value>>,
    /// Choice lists: (normalized option name, values, whether the values are enum members).
    lists: Vec<(String, Vec<Value>, bool)>,
    /// Number picker bounds: (normalized name after `max`/`maximum`, maximum).
    bounds: Vec<(String, i64)>,
    /// Battle plan force mixes: [forces, at half strength, special, special at half strength, ...].
    mixes: Vec<Vec<i64>>,
    /// Whole events the client offers as buttons (accepting, declining or withdrawing a deal).
    buttons: Vec<Map<String, Value>>,
}

/// Singular, lower case: `heroes` -> `hero`, `karmaCards` -> `karmacard`.
fn norm_option(k: &str) -> String {
    let k = k.to_ascii_lowercase();
    if let Some(s) = k.strip_suffix("ies") {
        format!("{s}y")
    } else if k.ends_with("oes") || k.ends_with("ses") || k.ends_with("xes") {
        k[..k.len() - 2].to_string()
    } else if let Some(s) = k.strip_suffix('s') {
        s.to_string()
    } else {
        k
    }
}

/// Lower case without the raw-id decoration: `_weaponCardId` -> `weaponcard`, `_cardIds` -> `card`.
fn norm_field(k: &str) -> String {
    let k = k.trim_start_matches('_').to_ascii_lowercase();
    for suffix in ["ids", "id"] {
        if k.len() > suffix.len() + 1 {
            if let Some(s) = k.strip_suffix(suffix) {
                return s.to_string();
            }
        }
    }
    k
}

/// How well option list `opt` names event field `field` (both normalized): 3 same, 2 or 1 one
/// contains the other, 0 unrelated.
fn name_score(field: &str, opt: &str) -> u8 {
    let field = match field {
        "to" => "target",
        "from" => "source",
        f => f,
    };
    if field == opt {
        3
    } else if opt.len() >= 3 && field.contains(opt) {
        2
    } else if field.len() >= 3 && opt.contains(field) {
        1
    } else {
        0
    }
}

fn element_value(v: &Value) -> Option<(Value, bool)> {
    match v {
        Value::Number(_) => Some((v.clone(), false)),
        Value::String(s) => Some(match crate::session::parse_faction(s) {
            Some(f) => (Value::from(f.0), true),
            None => (v.clone(), false),
        }),
        Value::Object(o) => {
            for k in ["value", "id", "location"] {
                if let Some(x) = o.get(k) {
                    if x.is_number() || x.is_string() {
                        return Some((x.clone(), o.contains_key("name")));
                    }
                }
            }
            None
        }
        _ => None,
    }
}

impl Pools {
    fn harvest(&mut self, key: &str, v: &Value) {
        match v {
            Value::Object(o) => {
                for (k, x) in o {
                    if k == "draft" {
                        continue;
                    }
                    if let Some(n) = x.as_i64() {
                        let lower = k.to_ascii_lowercase();
                        if let Some(core) = lower.strip_prefix("maximum").or_else(|| lower.strip_prefix("max")) {
                            self.bounds.push((norm_option(core), n));
                        }
                        continue;
                    }
                    self.harvest(k, x);
                }
            }
            Value::Array(items) if key == "mixes" => {
                self.mixes = items.iter().filter_map(|m| m.as_array().map(|a| a.iter().map(|x| x.as_f64().unwrap_or(0.0) as i64).collect())).collect();
            }
            Value::Array(items) => {
                let mut values = Vec::new();
                let mut enumish = false;
                for x in items {
                    if let Some((value, e)) = element_value(x) {
                        enumish |= e;
                        values.push(value);
                    } else {
                        self.harvest(key, x);
                    }
                }
                if !values.is_empty() {
                    self.lists.push((norm_option(key), values, enumish));
                }
            }
            _ => {}
        }
    }

    /// The pools of the form for event type `t` for player `i` of the shadow game.
    pub fn of(shadow: &Game, i: usize, t: GameEventType) -> Pools {
        let mut pools = Pools::default();
        if i >= shadow.players.len() {
            return pools;
        }
        let p = shadow.player_at(i);
        let faction = shadow.players[i].faction;
        let rv = ResolvedViewer { player: Some(p), faction: Some(faction), is_host: false, is_observer: false };
        let options = quiet(|| treachery_view::actions::action_view(shadow, &rv, t, treachery_skin::default_skin()).options.map(|o| serde_json::to_value(o).ok()))
            .flatten()
            .flatten();
        if let Some(o) = options {
            if let Some(Value::Object(d)) = o.get("draft") {
                pools.draft = Some(d.clone());
            }
            pools.harvest("", &o);
        }
        if matches!(t, GameEventType::DealAccepted | GameEventType::DealRejected | GameEventType::DealOffered) {
            let deals = quiet(|| treachery_view::side::deals(shadow, &rv, treachery_skin::default_skin()).map(|d| serde_json::to_value(d).ok())).flatten().flatten();
            for list in ["acceptable", "cancellable"] {
                for offer in deals.as_ref().and_then(|d| d.get(list)).and_then(|l| l.as_array()).into_iter().flatten() {
                    for button in ["accept", "decline", "withdraw"] {
                        if let Some(Value::Object(e)) = offer.get(button) {
                            if e.get("$type").and_then(|v| v.as_str()) == Some(t.name()) {
                                pools.buttons.push(e.clone());
                            }
                        }
                    }
                }
            }
        }
        pools
    }

    /// The choice list for event field `key` whose current value is `cur`: the best-named list,
    /// else a list that holds the current value (enum lists also when it is 0).
    fn list_for(&self, key: &str, cur: &Value, keys: &[String], rng: &mut Rng) -> Option<&[Value]> {
        let field = norm_field(key);
        let best = self.lists.iter().map(|(n, _, _)| name_score(&field, n)).max().unwrap_or(0);
        let named: Vec<&(String, Vec<Value>, bool)> = self.lists.iter().filter(|(n, _, _)| best > 0 && name_score(&field, n) == best).collect();
        if let Some(l) = rng.pick(&named) {
            return Some(&l.1);
        }
        let trivial = matches!(cur.as_i64(), Some(0) | Some(-1)) || cur.is_null() || cur.as_str() == Some("");
        let holding: Vec<&(String, Vec<Value>, bool)> = self.lists.iter().filter(|(_, l, e)| (!trivial || *e) && l.contains(cur)).collect();
        if let Some(l) = rng.pick(&holding) {
            return Some(&l.1);
        }
        // A raw id whose form names its choices differently (`_territoryId` from `targets`): a
        // list no field of the event is named after.
        if key.starts_with('_') && key.ends_with("Id") && cur.is_i64() {
            let unclaimed: Vec<&(String, Vec<Value>, bool)> =
                self.lists.iter().filter(|(n, l, _)| l.iter().all(|v| v.is_i64()) && !keys.iter().any(|k| name_score(&norm_field(k), n) > 0)).collect();
            return rng.pick(&unclaimed).map(|l| &l.1[..]);
        }
        None
    }

    fn bound_for(&self, key: &str) -> Option<i64> {
        let field = norm_field(key);
        self.bounds
            .iter()
            .filter(|(n, _)| !n.is_empty() && name_score(&field, n) >= 2)
            .max_by_key(|(n, _)| (name_score(&field, n), n.len()))
            .or_else(|| self.bounds.iter().find(|(n, _)| n.is_empty()).filter(|_| is_amount(&field)))
            .map(|(_, m)| *m)
    }

    fn string_lists(&self) -> Vec<&[Value]> {
        self.lists.iter().filter(|(_, l, _)| l.iter().any(|v| v.is_string())).map(|(_, l, _)| &l[..]).collect()
    }
}

/// A number from a picker with maximum `max`, as the C# client profile draws it.
fn bounded(max: i64, rng: &mut Rng) -> i64 {
    let max = max.max(0);
    match rng.f64() {
        x if x < 0.6 => rng.range(0, max as i32 + 1) as i64,
        x if x < 0.9 => max,
        _ => 0,
    }
}

/// Forces to move or place, as `loc:forces|special,...`: some of the player's own battalions, or
/// a few forces at locations the form offers.
fn force_locations(game: &Game, player: Option<usize>, pools: &Pools, rng: &mut Rng) -> Option<String> {
    let me = &game.players[player.filter(|i| *i < game.players.len())?];
    let own: Vec<(i32, i32, i32)> = me.forces_in_locations.iter().map(|(l, b)| (l.id(), b.amount_of_forces, b.amount_of_special_forces)).collect();
    let part = |n: i32, rng: &mut Rng| if n <= 0 { 0 } else if rng.chance(0.4) { n } else { rng.range(0, n + 1) };
    let mut parts: Vec<(i32, i32, i32)> = Vec::new();
    if !own.is_empty() && rng.chance(0.6) {
        for _ in 0..rng.range(1, 4) {
            let (l, f, s) = *rng.pick(&own)?;
            if parts.iter().any(|p| p.0 == l) {
                continue;
            }
            let (mut a, b) = (part(f, rng), part(s, rng));
            if a + b == 0 && rng.chance(0.8) {
                a = f.min(1);
            }
            parts.push((l, a, b));
        }
    } else {
        let targets: Vec<i32> = pools.lists.iter().filter(|(n, _, _)| matches!(n.as_str(), "target" | "location" | "source")).flat_map(|(_, l, _)| l.iter().filter_map(|v| v.as_i64()).map(|v| v as i32)).collect();
        for _ in 0..rng.range(1, 4) {
            let Some(l) = rng.pick(&targets).copied() else { break };
            if !parts.iter().any(|p| p.0 == l) {
                parts.push((l, rng.range(0, 6), if rng.chance(0.7) { 0 } else { rng.range(0, 3) }));
            }
        }
    }
    Some(parts.iter().map(|(l, f, s)| format!("{l}:{f}|{s}")).collect::<Vec<_>>().join(","))
}

/// A steered value for field `key` (current value `cur`) from the form's pools, or None to leave it.
fn steered_value(key: &str, cur: &Value, keys: &[String], pools: &Pools, game: &Game, player: Option<usize>, d: &Domains, rng: &mut Rng) -> Option<Value> {
    let k = key.to_ascii_lowercase();
    if matches!(k.as_str(), "initiator" | "time" | "$type" | "_players" | "_factionsinplay" | "settings" | "seed" | "initiatorplayername") {
        return None;
    }
    if cur.is_boolean() {
        return Some(Value::Bool(rng.chance(0.5)));
    }
    if k.ends_with("forcelocations") {
        return force_locations(game, player, pools, rng).map(Value::String);
    }
    if cur.is_array() || ((cur.is_string() || cur.is_null()) && k.ends_with("ids")) {
        // A multi-select: a subset of the offered choices (or of the player's cards).
        let list: Vec<Value> = match pools.list_for(key, cur, keys, rng) {
            Some(l) => l.to_vec(),
            None if k.contains("card") => d.own_cards.iter().map(|c| Value::from(*c)).collect(),
            None => return None,
        };
        let mut subset: Vec<Value> = list.iter().filter(|_| rng.chance(0.5)).cloned().collect();
        if subset.is_empty() && !list.is_empty() && rng.chance(0.7) {
            subset.push(rng.pick(&list).cloned().unwrap());
        }
        return Some(if cur.is_array() {
            Value::Array(subset)
        } else {
            Value::String(subset.iter().map(|v| v.to_string().trim_matches('"').to_string()).collect::<Vec<_>>().join(";"))
        });
    }
    if let Some(list) = pools.list_for(key, cur, keys, rng) {
        let v = rng.pick(list).cloned()?;
        // Keep the field's JSON kind: a string field gets the choice as text.
        return Some(match (&v, cur) {
            (Value::Number(n), Value::String(_) | Value::Null) => Value::String(n.to_string()),
            _ => v,
        });
    }
    if cur.is_string() || cur.is_null() {
        if k.contains("parameter") {
            let lists = pools.string_lists();
            let l = rng.pick(&lists)?;
            return rng.pick(l).cloned();
        }
        return None;
    }
    if !cur.is_i64() {
        return None;
    }
    if let Some(max) = pools.bound_for(key) {
        return Some(Value::from(bounded(max, rng)));
    }
    // No choice offered: the player's own cards, leaders and force locations, which the client
    // lists anyway, and small amounts.
    let own = Domains { all_cards: vec![], all_heroes: vec![], held_cards: vec![], locations: vec![], strongholds: vec![], territories: vec![], ..d.clone() };
    if k.contains("card") || k.contains("hero") || k.contains("leader") || k.starts_with('_') {
        return value_for(key, cur, &own, rng);
    }
    if is_amount(&k) && rng.chance(0.5) {
        return Some(Value::from(amount(rng)));
    }
    None
}

/// Fills fields of `event` from the form's pools: each with probability `p`. Returns the fields changed.
fn steer(event: &mut Map<String, Value>, pools: &Pools, game: &Game, player: Option<usize>, p: f64, rng: &mut Rng) -> Vec<String> {
    let Some(d) = quiet(|| Domains::of(game, player)) else { return vec![] };
    let mut changed = Vec::new();
    if !pools.mixes.is_empty() && event.contains_key("ForcesAtHalfStrength") && rng.chance(p) {
        let m = rng.pick(&pools.mixes).cloned().unwrap_or_default();
        for (i, k) in ["Forces", "ForcesAtHalfStrength", "SpecialForces", "SpecialForcesAtHalfStrength"].iter().enumerate() {
            if let Some(v) = m.get(i) {
                event.insert(k.to_string(), Value::from(*v));
            }
        }
        changed.push("Forces".to_string());
    }
    let keys: Vec<String> = event.keys().cloned().collect();
    for k in keys.clone() {
        if k.contains("Forces") && changed.iter().any(|c| c == "Forces") && !k.contains("Location") {
            continue;
        }
        if !rng.chance(p) {
            continue;
        }
        if let Some(v) = steered_value(&k, &event[&k], &keys, pools, game, player, &d, rng) {
            if event[&k] != v {
                event.insert(k.clone(), v);
                changed.push(k);
            }
        }
    }
    changed
}

/// Changes one field of `event` (an event of type `t` for `player`) to another value its form
/// offers, falling back to [`mutate`]. Returns the fields changed.
pub fn mutate_steered(event: &mut Map<String, Value>, game: &Game, player: usize, t: GameEventType, rng: &mut Rng) -> Vec<String> {
    let pools = Pools::of(game, player, t);
    let Some(d) = quiet(|| Domains::of(game, Some(player))) else { return vec![] };
    let keys: Vec<String> = event.keys().cloned().collect();
    for _ in 0..6 {
        let Some(k) = rng.pick(&keys).cloned() else { break };
        if let Some(v) = steered_value(&k, &event[&k], &keys, &pools, game, Some(player), &d, rng) {
            if event[&k] != v {
                event.insert(k.clone(), v);
                return vec![k];
            }
        }
    }
    mutate(event, game, Some(player), 1, rng)
}

/// The events played around a fight, plus plan revision and traitor calls (the C# oracle's
/// `Fuzzer.BattleActionTypes`).
pub const BATTLE_ACTION_TYPES: [&str; 24] = [
    "Voice", "Prescience", "Thought", "ThoughtAnswered", "Diplomacy", "PoisonToothCancelled", "PortableAntidoteUsed", "RockWasMelted",
    "AuditCancelled", "Audited", "FaceDanced", "FaceDancerReplaced", "FaceDancerRevealed", "ResidualPlayed", "Planetology", "JuicePlayed",
    "KarmaEcazDial", "KarmaPrescience", "TraitorDiscarded", "BattleRevision", "TreacheryCalled", "HMSAdvantageChosen", "SwitchedSkilledLeader",
    "BattleClaimed",
];

/// Battle actions whose branches bots and plain steering rarely reach; tried now and then even
/// when not applicable (and then rejected).
const RARE_BATTLE_ACTION_TYPES: [&str; 4] = ["KarmaEcazDial", "JuicePlayed", "ResidualPlayed", "AuditCancelled"];

pub fn battle_in_progress(g: &Game) -> bool {
    g.current_main_phase == MainPhase::Battle && g.current_battle.is_some()
}

fn type_named(name: &str) -> Option<GameEventType> {
    GameEventType::ALL.iter().copied().find(|t| t.name() == name)
}

/// Generates events and remembers which (type, phase) pairs were executed, to prefer rare ones.
pub struct Fuzzer {
    pub rng: Rng,
    accepted: BTreeMap<(&'static str, i32), u32>,
    opportunistic_tries: BTreeMap<(&'static str, i32), u32>,
    /// Steered picks per (type, phase), so a type that is always applicable but rarely valid
    /// (DealAccepted, AutomationConfigured) doesn't take every steered step.
    steered_tries: BTreeMap<(&'static str, i32), u32>,
    /// Executions per type in any phase: types executed all game long (deals, donations) weigh less.
    accepted_total: BTreeMap<&'static str, u32>,
    /// The latest accepted battle plan of each faction, for [`Fuzzer::changed_plan`].
    pub plans: BTreeMap<i32, Map<String, Value>>,
    /// The bot's latest event (player, JSON without time) and how often it came again in a row.
    pub last_bot_event: Option<(usize, String)>,
    pub repeats: u32,
}

pub const RANDOM_EVENT: f64 = 0.12;
pub const MUTATE_BOT_EVENT: f64 = 0.10;
pub const UNDO: f64 = 0.004;
pub const RESTORE_OR_DISMISS_AFTER_UNDO: f64 = 0.3;
pub const OPPORTUNISTIC: f64 = 0.3;
pub const BATTLE_ACTION: f64 = 0.25;
pub const CHANGED_PLAN: f64 = 0.3;
/// Share of random events that are steered (the rest: a random player's applicable or any type).
pub const STEERED: f64 = 0.85;
/// Candidates built per event, looking for one that passes validation.
const ATTEMPTS: usize = 8;

impl Fuzzer {
    pub fn new(seed: u64) -> Fuzzer {
        Fuzzer { rng: Rng::new(seed), accepted: BTreeMap::new(), opportunistic_tries: BTreeMap::new(), steered_tries: BTreeMap::new(), accepted_total: BTreeMap::new(), plans: BTreeMap::new(), last_bot_event: None, repeats: 0 }
    }

    pub fn note_accepted(&mut self, t: &'static str, phase: Phase) {
        *self.accepted.entry((t, phase.0)).or_default() += 1;
        *self.accepted_total.entry(t).or_default() += 1;
    }

    fn rare(&self, t: GameEventType, phase: Phase) -> bool {
        self.accepted.get(&(t.name(), phase.0)).copied().unwrap_or(0) < 2
    }

    /// Novelty weight of a type in a phase: 1 / (1 + accepted)^1.5, as in the C# oracle, damped by
    /// how often steering already picked it.
    fn weight(&self, t: GameEventType, phase: Phase) -> f64 {
        let a = self.accepted.get(&(t.name(), phase.0)).copied().unwrap_or(0) as f64;
        let tries = self.steered_tries.get(&(t.name(), phase.0)).copied().unwrap_or(0) as f64;
        let total = self.accepted_total.get(t.name()).copied().unwrap_or(0) as f64;
        1.0 / ((1.0 + a).powf(1.5) * (1.0 + tries / 8.0) * (1.0 + total / 30.0))
    }

    fn weighted<T: Copy>(&mut self, items: &[(T, f64)]) -> Option<T> {
        let total: f64 = items.iter().map(|(_, w)| w).sum();
        if items.is_empty() || total <= 0.0 {
            return None;
        }
        let mut x = self.rng.f64() * total;
        for (item, w) in items {
            x -= w;
            if x < 0.0 {
                return Some(*item);
            }
        }
        items.last().map(|(i, _)| *i)
    }

    /// A (player or nobody, type) from everything applicable, weighted towards types rarely
    /// executed in this phase (the C# oracle's `SteeredEvent`).
    pub fn steered_pick(&mut self, applicable: &[(Option<usize>, Vec<GameEventType>)], phase: Phase) -> Option<(Option<usize>, GameEventType)> {
        let items: Vec<((Option<usize>, GameEventType), f64)> = applicable.iter().flat_map(|(i, ts)| ts.iter().map(move |t| ((*i, *t), 0.0))).collect();
        let items: Vec<_> = items.into_iter().map(|((i, t), _)| ((i, t), self.weight(t, phase))).collect();
        let pick = self.weighted(&items)?;
        *self.steered_tries.entry((pick.1.name(), phase.0)).or_default() += 1;
        Some(pick)
    }

    /// A battle action for a player in the current battle (the C# oracle's `BattleAction`):
    /// an applicable one, weighted towards rare types; now and then a rare one regardless.
    pub fn battle_action(&mut self, applicable: &[(usize, Vec<GameEventType>)], phase: Phase) -> Option<(usize, GameEventType)> {
        let items: Vec<((usize, GameEventType), f64)> = applicable
            .iter()
            .flat_map(|(i, ts)| ts.iter().filter(|t| BATTLE_ACTION_TYPES.contains(&t.name())).map(move |t| (*i, *t)))
            .map(|(i, t)| ((i, t), 0.0))
            .collect();
        if !items.is_empty() && self.rng.chance(0.9) {
            let items: Vec<_> = items.into_iter().map(|((i, t), _)| ((i, t), self.weight(t, phase))).collect();
            return self.weighted(&items);
        }
        if self.rng.f64() > 0.15 || applicable.is_empty() {
            return None;
        }
        let i = applicable[self.rng.below(applicable.len())].0;
        let t = type_named(RARE_BATTLE_ACTION_TYPES[self.rng.below(RARE_BATTLE_ACTION_TYPES.len())])?;
        Some((i, t))
    }

    /// A battle plan this faction submitted before, with its leader, a card or its dial changed
    /// to another choice its form offers. After a `BattleRevision`, parts the opponent saw with
    /// prescience must stay (the C# oracle's `ChangedSeenPlan`); other changes are allowed.
    pub fn changed_plan(&mut self, shadow: &Game, player: usize) -> Option<Map<String, Value>> {
        let faction = shadow.players.get(player)?.faction.0;
        let mut plan = self.plans.get(&faction)?.clone();
        let pools = Pools::of(shadow, player, GameEventType::Battle);
        let (field, list): (&str, Vec<Value>) = match self.rng.below(4) {
            0 => ("_heroId", pools.lists.iter().find(|(n, _, _)| n == "hero")?.1.clone()),
            1 => ("_weaponCardId", pools.lists.iter().find(|(n, _, _)| n == "weapon")?.1.clone()),
            2 => ("_defenseCardId", pools.lists.iter().find(|(n, _, _)| n == "defense")?.1.clone()),
            _ => ("Forces", pools.mixes.iter().filter_map(|m| m.first().map(|f| Value::from(*f))).collect()),
        };
        let others: Vec<Value> = list.into_iter().filter(|v| Some(v) != plan.get(field)).collect();
        let v = self.rng.pick(&others)?.clone();
        plan.insert(field.into(), v);
        plan.remove("Time");
        Some(plan)
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
    /// candidates, each the draft the client's form sends with fields filled from the choices the
    /// form offers. Returns the event JSON object (without `Time`), preferring one that passes
    /// validation.
    pub fn build(&mut self, shadow: &Game, player: Option<usize>, t: GameEventType) -> Option<Map<String, Value>> {
        let faction = player.filter(|i| *i < shadow.players.len()).map(|i| shadow.players[i].faction).unwrap_or(Faction::None);
        let pools = player.map(|i| Pools::of(shadow, i, t)).unwrap_or_default();
        if let Some(b) = self.rng.pick(&pools.buttons).filter(|_| self.rng.chance(0.9)) {
            let mut e = b.clone();
            e.remove("Time");
            return Some(e);
        }
        let draft = pools.draft.clone().map(|d| Value::Object(d).to_string()).or_else(|| {
            player.filter(|i| *i < shadow.players.len()).and_then(|i| {
                let p = shadow.player_at(i);
                let rv = ResolvedViewer { player: Some(p), faction: Some(faction), is_host: false, is_observer: false };
                quiet(|| treachery_view::actions::default_draft(shadow, &rv, faction, t).map(|e| json::serialize(&e))).flatten()
            })
        });
        let base = draft.unwrap_or_else(|| {
            let minimal = format!("{{\"$type\":\"{}\",\"Initiator\":{}}}", t.name(), faction.0);
            quiet(|| json::deserialize::<GameEvent>(&minimal).ok().map(|e| json::serialize(&e))).flatten().unwrap_or(minimal)
        });
        let Ok(Value::Object(mut base)) = serde_json::from_str::<Value>(&base) else { return None };
        base.remove("Time");
        // A form with a faction to pick has one picked: another faction in play.
        if base.get("Target").and_then(|v| v.as_i64()) == Some(0) {
            let others: Vec<i32> = shadow.players.iter().map(|p| p.faction.0).filter(|f| *f != 0 && *f != faction.0).collect();
            if let Some(f) = self.rng.pick(&others) {
                base.insert("Target".into(), Value::from(*f));
            }
        }

        let mut last = None;
        // After a valid candidate is passed over, the next one is sent unchecked (mostly invalid).
        let mut search = true;
        for attempt in 0..ATTEMPTS {
            let mut candidate = base.clone();
            if attempt > 0 || self.rng.chance(0.7) {
                let p = [0.3, 0.6, 0.9][self.rng.below(3)];
                steer(&mut candidate, &pools, shadow, player, p, &mut self.rng);
                // Now and then a value from the wider domains of the game (still real ids).
                if self.rng.chance(0.1) {
                    mutate(&mut candidate, shadow, player, 1, &mut self.rng);
                }
            }
            if t == GameEventType::JuicePlayed && self.rng.chance(0.7) {
                if pools.lists.iter().any(|(n, l, _)| n == "choice" && l.contains(&Value::from(JuiceType::Aggressor.0))) {
                    candidate.insert("Type".into(), Value::from(JuiceType::Aggressor.0));
                }
            }
            if !search {
                return Some(candidate);
            }
            let text = Value::Object(candidate.clone()).to_string();
            let valid = quiet(|| {
                let e: GameEvent = json::deserialize(&text).ok()?;
                e.initialize_game(shadow);
                Some(e.validate(shadow).is_none())
            })
            .flatten()
            .unwrap_or(false);
            if valid && self.rng.chance(0.9) {
                return Some(candidate);
            }
            last = Some(candidate);
            search = !valid;
        }
        last
    }
}
