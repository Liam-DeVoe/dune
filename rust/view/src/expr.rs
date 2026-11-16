//! Renders engine messages to [`RichText`]: one part per message part, so the client only has to
//! draw parts (`web/src/components/RichText.tsx`). Labels are the skin's description of the
//! part. `tests/log_lines.rs` keeps snapshots of every log line of the fixtures.

use treachery_engine::enum_type::EnumTypeInfo;
use treachery_engine::heroes::hero::Hero;
use treachery_engine::identity::Identifiable;
use treachery_engine::messages::message::Message;
use treachery_engine::messages::part::Part as P;
use treachery_engine::prelude::*;

use crate::describe::Describer;
use crate::schema::{Part, RichText};

/// The member name of an enum value, or the number for a value that is not a declared member.
pub fn enum_name(info: &EnumTypeInfo, value: i32) -> String {
    info.name_of(value).map(str::to_string).unwrap_or_else(|| value.to_string())
}

/// A `float` or `double` rounded to three decimals, to even, before it becomes a number part.
/// The client prints the result in JavaScript's shortest round-trip form.
pub fn support_round(x: f64) -> f64 {
    dotnet_compat::math::round_f64_digits(x, 3)
}

pub fn message(m: &Message, d: &dyn Describer) -> RichText {
    parts(&m.parts, d)
}

pub fn parts(parts: &[P], d: &dyn Describer) -> RichText {
    let mut out = Vec::new();
    for p in parts {
        element(p, d, &mut out);
    }
    out
}

/// A message, with whether each part is drawn with a text border. Items of a list keep the
/// border, the parts of a nested message or phrase don't. The view flattens nesting, so a
/// caller that draws a border (the card flash) needs these marks.
pub fn message_with_borders(m: &Message, d: &dyn Describer) -> (RichText, Vec<bool>) {
    let mut out = Vec::new();
    let mut borders = Vec::new();
    for p in &m.parts {
        element_marked(p, d, &mut out, &mut borders, true);
    }
    (out, borders)
}

/// List items are separated by ", " unless the first item is a faction, a phrase, a force token
/// or a leader.
fn shows_with_separator(first: &P) -> bool {
    !matches!(first, P::Phrase(_) | P::Leader(_) | P::Faction(_) | P::FactionForce(_) | P::FactionSpecialForce(_))
}

/// Renders one message part (a `FlashInfo.ToShow`, for instance) into `out`.
pub fn element_into(el: &P, d: &dyn Describer, out: &mut RichText) {
    element(el, d, out)
}

/// One message part as a single rich-text part, for the game objects in log records (a
/// location, a hero, a card). Null is `None`.
pub fn part(el: &P, d: &dyn Describer) -> Option<Part> {
    if matches!(el, P::Null) {
        return None;
    }
    let mut out = Vec::new();
    element(el, d, &mut out);
    out.into_iter().next()
}

fn element(el: &P, d: &dyn Describer, out: &mut RichText) {
    element_marked(el, d, out, &mut Vec::new(), true)
}

/// `element`, also pushing to `borders` whether each part gets the text border.
fn element_marked(el: &P, d: &dyn Describer, out: &mut RichText, borders: &mut Vec<bool>, border: bool) {
    match el {
        // A bold "none".
        P::Null => out.push(Part::None),
        P::Text(s) => out.push(Part::Text { v: s.to_string() }),
        P::Error(_) => out.push(Part::Text { v: d.describe(el) }),
        // The items, with separators.
        P::List(items) => {
            let separators = items.first().is_some_and(shows_with_separator);
            for (i, item) in items.iter().enumerate() {
                if separators && i > 0 {
                    out.push(Part::Text { v: ", ".to_string() });
                    borders.push(border);
                }
                element_marked(item, d, out, borders, border);
            }
        }
        P::Phrase(parts) => {
            for p in parts {
                element_marked(p, d, out, borders, false);
            }
        }
        P::Message(m) => {
            for p in &m.parts {
                element_marked(p, d, out, borders, false);
            }
        }
        // Numbers, as a number token.
        P::Number(i) => out.push(Part::Num { v: *i as f64 }),
        P::Float(f) => out.push(Part::Num { v: support_round(*f as f64) }),
        P::Double(x) => out.push(Part::Num { v: support_round(*x) }),
        P::Payment { amount, by } => out.push(Part::Payment { amount: *amount, by: (*by != Faction::None).then(|| by.to_string()) }),
        P::Faction(_) => {
            let (info, value) = el.enum_value().expect("an enum");
            out.push(Part::Faction { v: enum_name(info, value), label: d.describe(el) })
        }
        P::Leader(l) => out.push(hero_part(Hero::Leader(*l), d)),
        P::Messiah => out.push(hero_part(Hero::Messiah, d)),
        P::Card(c) => out.push(hero_part(Hero::TreacheryCard(*c), d)),
        P::SpiceCard(c) => out.push(Part::ResourceCard { id: c.skin_id(), label: d.describe(el) }),
        P::Location(l) => out.push(Part::Location { id: l.id(), label: d.describe(el) }),
        P::Territory(t) => out.push(Part::Territory { id: t.id(), label: d.describe(el) }),
        P::Bool(b) => out.push(Part::Bool { v: *b, label: d.describe(el) }),
        // Every other enum: the client picks the drawing by type (the rich-text, monster, image
        // and bold cases of the switch, or plain text).
        other => {
            let (info, value) = other.enum_value().expect("every part that isn't an enum is drawn above");
            out.push(Part::Enum { ty: info.name.to_string(), v: enum_name(info, value), label: d.describe(el) })
        }
    }
    // The parts a leaf case pushed.
    borders.resize(out.len(), border);
}

pub fn hero_part(h: Hero, d: &dyn Describer) -> Part {
    let label = d.describe(&P::from(h));
    match h {
        Hero::Leader(l) => {
            // The popover of a leader shows its skill card (`Skin.GetPopup(IHero, Game)`).
            let skill = d.game().and_then(|g| g.skill(Some(h))).map(|s| s.to_string());
            Part::Leader { id: l.id(), label, skill }
        }
        Hero::Messiah => Part::Messiah { label },
        Hero::TreacheryCard(c) => Part::TreacheryCard { id: c.id(), label },
    }
}
