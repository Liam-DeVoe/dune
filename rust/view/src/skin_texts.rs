//! Skin texts that need the game and more of the skin than `Describe`: the faction sheets of the
//! map's faction popover and the rule names of the turn popover. Both read only public data:
//! the game's rules and which
//! factions play. The client builds the popover markup from these, because the place chips in a
//! sheet need its map geometry.
//!
//! Also the milestones of the latest event (`Game::recent_milestones`), for the sounds the
//! client plays after each event.

use serde::{Deserialize, Serialize};
use treachery_engine::messages::part::Part as P;
use treachery_engine::prelude::*;
use treachery_skin::Skin;
use ts_rs::TS;

use crate::describe::Describer;

/// `Skin.GetFactionSheet` and `GetFactionInfo_HTML` of one faction in the game.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct FactionSheetView {
    #[ts(type = "Faction")]
    pub faction: String,
    /// `"<AtStart>. Start with <Resources> {16}."` with the placeholders filled in, as
    /// `FactionPopupHtml` does: places become place chips, everything else `Skin.Describe`.
    /// The text is HTML (not encoded), and the client still applies `WithSpiceIcons(.., 18,
    /// encode: false)` to the whole.
    pub at_start: Vec<SheetPart>,
    pub free_revival: i32,
    /// HTML.
    pub theme: String,
    /// The advantages' names, HTML.
    pub advantages: Vec<String>,
    pub advanced_advantages: Vec<String>,
    /// `GetFactionInfo_HTML(game, faction, "")`: the full sheet, shown while Shift is held.
    pub html: String,
}

/// A piece of [`FactionSheetView::at_start`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "t", rename_all = "camelCase")]
pub enum SheetPart {
    Text { v: String },
    Location { id: i32 },
    Territory { id: i32 },
}

/// The turn popover's ruleset and rules, described by the skin.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct RuleLabels {
    /// `Skin.Describe(Game.Ruleset)`.
    pub ruleset: String,
    /// The groups of `Game.Rules` other than CoreBasic, Bots and None, in order of their first
    /// rule, each with its rules described.
    pub groups: Vec<RuleGroupLabels>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct RuleGroupLabels {
    pub group: String,
    pub rules: Vec<String>,
}

/// The sheets of the factions in the game, in player order. Empty for a describer without a
/// skin.
pub fn faction_sheets(game: &Game, d: &dyn Describer) -> Vec<FactionSheetView> {
    let Some(skin) = d.skin() else { return vec![] };
    game.player_indices().filter_map(|p| game.player(p).faction.and_then(|f| faction_sheet(skin, game, f))).collect()
}

/// The faction sheets of the last view built, reused while what they're made from stays the
/// same. A [`crate::replica::ViewReplica`] keeps one, so its views don't build the sheets again
/// for every viewer after every event (they are about half of a view's JSON).
///
/// The sheets read the skin, the game's rules (`Game::applicable`, which reads the active rules
/// and the rules for bots), which factions play and in what order, and fixed locations of the
/// map. The key holds all but the skin and the map: whoever owns the cache clears it when the
/// skin changes, and the map's named locations don't change.
#[derive(Default)]
pub struct FactionSheetCache {
    cached: Option<(SheetKey, Vec<FactionSheetView>)>,
}

#[derive(PartialEq)]
struct SheetKey {
    version: i32,
    active: Vec<Rule>,
    rules_for_bots: Vec<Rule>,
    factions: Vec<Option<Faction>>,
}

impl SheetKey {
    fn of(game: &Game) -> SheetKey {
        SheetKey {
            version: game.rules.version,
            active: game.rules.active.clone(),
            rules_for_bots: game.rules.rules_for_bots.clone(),
            factions: game.players.iter().map(|p| p.faction).collect(),
        }
    }
}

impl FactionSheetCache {
    /// [`faction_sheets`], from the cache when its key matches. `d` must have the skin the
    /// cache was filled with: call [`FactionSheetCache::clear`] when it changes.
    pub fn get(&mut self, game: &Game, d: &dyn Describer) -> Vec<FactionSheetView> {
        let key = SheetKey::of(game);
        match &self.cached {
            Some((k, sheets)) if *k == key => sheets.clone(),
            _ => {
                let sheets = faction_sheets(game, d);
                self.cached = Some((key, sheets.clone()));
                sheets
            }
        }
    }

    pub fn clear(&mut self) {
        self.cached = None;
    }
}

fn faction_sheet(skin: &Skin, game: &Game, f: Faction) -> Option<FactionSheetView> {
    let sheet = skin.get_faction_sheet(game, f)?;
    let text = format!("{}. Start with {} {{16}}.", sheet.at_start, sheet.resources);
    Some(FactionSheetView {
        faction: f.to_string(),
        at_start: fill_placeholders(skin, &text, &sheet.parameters),
        free_revival: sheet.free_revival,
        theme: sheet.theme,
        advantages: sheet.advantages,
        advanced_advantages: sheet.advanced_advantages,
        html: skin.get_faction_info_html(game, f, Some("")),
    })
}

/// `Regex.Replace(atStart, @"\{(\d+)\}", ...)`: places become chips, the rest is described.
fn fill_placeholders(skin: &Skin, text: &str, parameters: &[P]) -> Vec<SheetPart> {
    let mut out: Vec<SheetPart> = Vec::new();
    let push_text = |out: &mut Vec<SheetPart>, s: &str| {
        if s.is_empty() {
            return;
        }
        if let Some(SheetPart::Text { v }) = out.last_mut() {
            v.push_str(s);
        } else {
            out.push(SheetPart::Text { v: s.to_string() });
        }
    };
    let mut rest = text;
    while let Some(open) = rest.find('{') {
        let after = &rest[open + 1..];
        let digits = after.bytes().take_while(|b| b.is_ascii_digit()).count();
        if digits == 0 || !after[digits..].starts_with('}') {
            push_text(&mut out, &rest[..open + 1]);
            rest = after;
            continue;
        }
        push_text(&mut out, &rest[..open]);
        let index: usize = after[..digits].parse().unwrap_or(usize::MAX);
        match parameters.get(index) {
            Some(P::Location(l)) => out.push(SheetPart::Location { id: l.id() }),
            Some(P::Territory(t)) => out.push(SheetPart::Territory { id: t.id() }),
            Some(el) => push_text(&mut out, &skin.describe(el)),
            None => {}
        }
        rest = &after[digits + 1..];
    }
    push_text(&mut out, rest);
    out
}

/// `TurnPopover`'s descriptions of `Game.Ruleset` and `Game.Rules`.
pub fn rule_labels(game: &Game, d: &dyn Describer) -> RuleLabels {
    let mut groups: Vec<(RuleGroup, Vec<String>)> = Vec::new();
    for r in game.rules.active.iter().copied() {
        let Some(g) = Game::get_rule_group(r).filter(|g| *g != RuleGroup::CoreBasic && *g != RuleGroup::Bots) else {
            continue;
        };
        let label = d.describe(&P::from(r));
        match groups.iter_mut().find(|(x, _)| *x == g) {
            Some((_, rules)) => rules.push(label),
            None => groups.push((g, vec![label])),
        }
    }
    RuleLabels {
        ruleset: d.describe(&P::from(game.rules.ruleset)),
        groups: groups
            .into_iter()
            .map(|(g, rules)| RuleGroupLabels { group: d.describe(&P::from(g)), rules })
            .collect(),
    }
}

/// What the latest event achieved. Every viewer gets them, observers included: they mark public
/// outcomes (an auction won, a shuffle, a Karama card).
pub fn recent_milestones(game: &Game) -> Vec<String> {
    game.turn.recent_milestones.iter().map(|m| m.to_string()).collect()
}
