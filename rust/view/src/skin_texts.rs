//! Skin texts that need the game and more of the skin than `Describe`: the faction sheets of the
//! map's faction popover and the rule names of the turn popover. Both read only public data:
//! the game's rules and which
//! factions play. The client builds the popover markup from these, because the place chips in a
//! sheet need its map geometry.
//!
//! Also the milestones of the latest event (`Game::recent_milestones`), for the sounds the
//! client plays after each event.

use serde::{Deserialize, Serialize};
use treachery_engine::messages::expression::ExpressionElement;
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
    game.player_indices().filter_map(|p| faction_sheet(skin, game, game.player(p).faction)).collect()
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
fn fill_placeholders(skin: &Skin, text: &str, parameters: &[ExpressionElement]) -> Vec<SheetPart> {
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
            Some(ExpressionElement::Location(l)) if !l.is_null() => out.push(SheetPart::Location { id: l.id() }),
            Some(ExpressionElement::Territory(t)) if !t.is_null() => out.push(SheetPart::Territory { id: t.id() }),
            Some(el) => push_text(&mut out, &skin.describe_in(el, None)),
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
    for r in game.rules.iter().copied() {
        let g = Game::get_rule_group(r);
        if g == RuleGroup::CoreBasic || g == RuleGroup::Bots || g == RuleGroup::None {
            continue;
        }
        let label = d.describe(&ExpressionElement::from(r));
        match groups.iter_mut().find(|(x, _)| *x == g) {
            Some((_, rules)) => rules.push(label),
            None => groups.push((g, vec![label])),
        }
    }
    RuleLabels {
        ruleset: d.describe(&ExpressionElement::from(game.ruleset)),
        groups: groups
            .into_iter()
            .map(|(g, rules)| RuleGroupLabels { group: d.describe(&ExpressionElement::from(g)), rules })
            .collect(),
    }
}

/// What the latest event achieved. Every viewer gets them, observers included: they mark public
/// outcomes (an auction won, a shuffle, a Karama card).
pub fn recent_milestones(game: &Game) -> Vec<String> {
    game.recent_milestones.iter().map(|m| m.to_string()).collect()
}
