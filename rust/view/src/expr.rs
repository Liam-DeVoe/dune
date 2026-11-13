//! Renders engine messages to [`RichText`]: what `ExpressionComponent.razor` draws, as data.
//!
//! Each element becomes the part the component's `switch` picks for it, in the same order of
//! cases, so the client only has to draw parts (`web/src/components/RichText.tsx`). Labels are
//! `Skin.Describe` of the element. The C# reference is `rust/view/csharp-ref` (LogLines.cs),
//! and `tests/log_lines.rs` compares every log line of the fixtures with it.

use treachery_engine::messages::expression::{EnumValue, Expression, ExpressionElement};
use treachery_engine::heroes::hero::Hero;
use treachery_engine::identity::IIdentifiable;
use treachery_engine::messages::message::Message;
use treachery_engine::prelude::*;

use crate::describe::Describer;
use crate::schema::{Part, RichText};

/// The name of an enum value as C# `ToString()` writes it for a single member, or the number
/// for a value that is not a declared member.
pub fn enum_name(v: &EnumValue) -> String {
    v.ty.declared.iter().find(|(_, value)| *value == v.value).map(|(n, _)| n.to_string()).unwrap_or_else(|| v.value.to_string())
}

/// `Support.Round` of the client (`Math.Round(x, 3)`, to even), which `ExpressionComponent`
/// applies to `float` and `double` before `SimpleNumberComponent` prints them. The client prints
/// the result as JavaScript's shortest round-trip form, which is .NET's `double.ToString()` for
/// the magnitudes a game produces.
pub fn support_round(x: f64) -> f64 {
    dotnet_compat::math::round_f64_digits(x, 3)
}

pub fn message(m: &Message, d: &dyn Describer) -> RichText {
    expression(&m.expression, d)
}

pub fn expression(e: &Expression, d: &dyn Describer) -> RichText {
    let mut out = Vec::new();
    for el in &e.elements {
        element(el, d, &mut out);
    }
    out
}

/// A message, with whether `ExpressionComponent` draws each part with its `TextBorder`. The
/// component passes the border on to the items of a list but not to a nested `Message` or
/// `MessagePart`, whose parts it draws without one. The view flattens nesting, so a caller that
/// draws a border (`ImageFlashComponent`) needs these marks.
pub fn message_with_borders(m: &Message, d: &dyn Describer) -> (RichText, Vec<bool>) {
    let mut out = Vec::new();
    let mut borders = Vec::new();
    for el in &m.expression.elements {
        element_marked(el, d, &mut out, &mut borders, true);
    }
    (out, borders)
}

/// `ExpressionComponent.ShowWithSeparator`: list items are separated by ", " unless the first
/// item is a faction, a message part, a force token or a leader.
fn shows_with_separator(first: &ExpressionElement) -> bool {
    match first {
        ExpressionElement::MessagePart(_) => false,
        ExpressionElement::Hero(Hero::Leader(_)) => false,
        ExpressionElement::Enum(v) => !matches!(v.ty.name, "Faction" | "FactionForce" | "FactionSpecialForce"),
        _ => true,
    }
}

/// Renders one expression element (a `FlashInfo.ToShow`, for instance) into `out`.
pub fn element_into(el: &ExpressionElement, d: &dyn Describer, out: &mut RichText) {
    element(el, d, out)
}

/// One element as a single part, for the game objects in log records (a location, a hero, a
/// card). Null is `None`.
pub fn part(el: &ExpressionElement, d: &dyn Describer) -> Option<Part> {
    if matches!(el, ExpressionElement::Null) {
        return None;
    }
    let mut out = Vec::new();
    element(el, d, &mut out);
    out.into_iter().next()
}

fn element(el: &ExpressionElement, d: &dyn Describer, out: &mut RichText) {
    element_marked(el, d, out, &mut Vec::new(), true)
}

/// `element`, also pushing to `borders` whether each part gets the text border.
fn element_marked(el: &ExpressionElement, d: &dyn Describer, out: &mut RichText, borders: &mut Vec<bool>, border: bool) {
    match el {
        // `case null`: a bold "none".
        ExpressionElement::Null => out.push(Part::None),
        ExpressionElement::String(s) => out.push(Part::Text { v: s.clone() }),
        // `case IEnumerable`: the items, with separators (DetermineElements). A collection the
        // game still holds is enumerated now, as the C# component did when it rendered.
        ExpressionElement::List { items, live, .. } => {
            let current = match (live, d.game()) {
                (Some(source), Some(g)) => source.read(g),
                _ => None,
            };
            let items = current.as_deref().unwrap_or(items);
            let separators = items.first().is_some_and(shows_with_separator);
            for (i, item) in items.iter().enumerate() {
                if separators && i > 0 {
                    out.push(Part::Text { v: ", ".to_string() });
                    borders.push(border);
                }
                element_marked(item, d, out, borders, border);
            }
        }
        ExpressionElement::Message(_) | ExpressionElement::MessagePart(_) => {
            let e = match el {
                ExpressionElement::Message(m) => &m.expression,
                ExpressionElement::MessagePart(m) => &m.expression,
                _ => unreachable!(),
            };
            for el in &e.elements {
                element_marked(el, d, out, borders, false);
            }
        }
        // `case int`, `case float`, `case double`: a number token.
        ExpressionElement::Int(i) => out.push(Part::Num { v: *i as f64 }),
        ExpressionElement::Float(f) => out.push(Part::Num { v: support_round(*f as f64) }),
        ExpressionElement::Double(x) => out.push(Part::Num { v: support_round(*x) }),
        ExpressionElement::Payment(p) => {
            out.push(Part::Payment { amount: p.amount, by: (p.by != Faction::None).then(|| p.by.to_string()) })
        }
        ExpressionElement::Enum(v) if v.ty.name == "Faction" => out.push(Part::Faction { v: enum_name(v), label: d.describe(el) }),
        ExpressionElement::Hero(h) => out.push(hero_part(*h, d)),
        ExpressionElement::ResourceCard(c) => out.push(Part::ResourceCard { id: c.skin_id(), label: d.describe(el) }),
        ExpressionElement::Location(l) => out.push(Part::Location { id: l.id(), label: d.describe(el) }),
        ExpressionElement::Territory(t) => out.push(Part::Territory { id: t.id(), label: d.describe(el) }),
        // Every other enum: the client picks the drawing by type (the rich-text, monster,
        // image and bold cases of the switch, or plain text).
        ExpressionElement::Enum(v) => out.push(Part::Enum { ty: v.ty.name.to_string(), v: enum_name(v), label: d.describe(el) }),
        ExpressionElement::Bool(b) => out.push(Part::Bool { v: *b, label: d.describe(el) }),
        // `default`: `Describe(part)` as text. A `long` lands here, not in `case int`; so do the
        // "Game Error" exception (C# appends a stack trace the engine doesn't have) and a
        // `Player` (its type name).
        ExpressionElement::Long(_) | ExpressionElement::Exception(_) | ExpressionElement::Player(_) => {
            out.push(Part::Text { v: d.describe(el) })
        }
    }
    // The parts a leaf case pushed.
    borders.resize(out.len(), border);
}

pub fn hero_part(h: Hero, d: &dyn Describer) -> Part {
    let label = d.describe(&ExpressionElement::Hero(h));
    match h {
        Hero::Leader(l) => {
            // The popover of a leader shows its skill card (`Skin.GetPopup(IHero, Game)`).
            let skill = d.game().map(|g| g.skill(Some(h))).filter(|s| *s != LeaderSkill::None).map(|s| s.to_string());
            Part::Leader { id: l.id(), label, skill }
        }
        Hero::Messiah => Part::Messiah { label },
        Hero::TreacheryCard(c) => Part::TreacheryCard { id: c.id(), label },
    }
}
