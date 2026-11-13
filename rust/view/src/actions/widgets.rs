//! Helpers the form builders share: how a form's widgets adjust its inputs, and how choices and
//! drafts are written into the options.

use treachery_engine::enum_type::GameEnum;
use treachery_engine::json;
use treachery_engine::messages::expression::ExpressionElement;
use treachery_engine::prelude::*;

use crate::describe::Describer;
use crate::schema::{FactionOption, HeroOption};
use crate::shield::hero_ref;

/// A number picker's value: below `min` becomes `min`, else above `max` becomes `max`. Unlike
/// `i32::clamp` it doesn't panic for an empty range, where `min` wins.
pub(super) fn picker(value: i32, min: i32, max: i32) -> i32 {
    if value < min { min } else { value.min(max) }
}

/// A required select or image choice: a value not among the values becomes the first one, or
/// `default` when there are none.
pub(super) fn required<T: PartialEq + Copy>(value: T, values: &[T], default: T) -> T {
    if values.contains(&value) { value } else { values.first().copied().unwrap_or(default) }
}

pub(super) fn faction_options(factions: impl IntoIterator<Item = Faction>) -> Vec<FactionOption> {
    factions.into_iter().map(|f| FactionOption { faction: f.to_string(), value: f.value() }).collect()
}

pub(super) fn hero_options(heroes: impl IntoIterator<Item = Hero>) -> Vec<HeroOption> {
    heroes.into_iter().map(|h| HeroOption { id: h.id(), hero: hero_ref(h) }).collect()
}

pub(super) fn card_ids(cards: impl IntoIterator<Item = TreacheryCard>) -> Vec<i32> {
    cards.into_iter().map(|c| c.id()).collect()
}

/// The draft as savegame JSON, for the options' `draft`.
pub(super) fn draft_json(e: &GameEvent) -> serde_json::Value {
    serde_json::from_str(&json::serialize(e)).expect("the engine writes valid JSON")
}

/// The skin's description of a value, without a game.
pub(super) fn describe(d: &dyn Describer, el: impl Into<ExpressionElement>) -> String {
    d.describe_in(&el.into(), None)
}
