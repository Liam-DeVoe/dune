//! `Message.ToString()` without a describer, under the invariant culture,
//! against tests/golden/messages.json (csharp-ref/MessageCorpus.cs).
//!
//! The corpus has one entry per element: floats and doubles from random bit
//! patterns plus NaN, the infinities, -0, subnormals and large exponents;
//! every value of every enum, with undefined values and flag combinations;
//! every location, territory, leader, card and resource card; collections of
//! each runtime type; exceptions of each type; nested messages. Each is
//! rebuilt as an `ExpressionElement` and compared through `to_cs_string`.
//!
//! The `params` section checks how `Message.Express` treats each kind of
//! single argument (spread or one element) against the `@params` form of the
//! macros.

mod common;

use common::*;
use serde_json::{json, Value};
use treachery_engine::compat::cs;
use treachery_engine::enumerations::Faction;
use treachery_engine::messages::expression::{EnumValue, ExpressionElement as E};
use treachery_engine::heroes::hero::Hero;
use treachery_engine::heroes::leader::Leader;
use treachery_engine::map::location::Location;
use treachery_engine::map::Map;
use treachery_engine::messages::message::{Message, MessagePart};
use treachery_engine::messages::payment::Payment;
use treachery_engine::map::territory::Territory;
use treachery_engine::cards::treachery_card::TreacheryCard;
use treachery_engine::{express, express_part, express_to};

fn items(v: &Value) -> &Vec<Value> {
    v["items"].as_array().unwrap()
}

fn decode_all<T>(v: &Value, f: impl Fn(&Value) -> T) -> Vec<T> {
    items(v).iter().map(f).collect()
}

/// Rebuilds the element an encoded input stands for.
fn element(v: &Value) -> E {
    match v {
        Value::Null => E::Null,
        Value::Bool(b) => E::Bool(*b),
        Value::Number(n) => match i32::try_from(n.as_i64().unwrap()) {
            Ok(i) => E::Int(i),
            Err(_) => E::Long(n.as_i64().unwrap()),
        },
        Value::String(s) if s == "null" => E::Null,
        Value::String(s) => {
            let (tag, rest) = s.split_once(':').unwrap();
            match tag {
                "fbits" => E::Float(f32::from_bits(rest.parse().unwrap())),
                "dbits" => E::Double(f64::from_bits(rest.parse().unwrap())),
                "f" => E::Float(float(v)),
                "s" => E::String(rest.to_string()),
                "e" => {
                    let (name, value) = rest.rsplit_once(':').unwrap();
                    let ty = enum_info(name).unwrap_or_else(|| panic!("C# enum {name} has no Rust port (add it to common::enum_info)"));
                    E::Enum(EnumValue { ty, value: value.parse().unwrap() })
                }
                "o" => match object(v).0 {
                    "Territory" => E::from(territory(v)),
                    "Leader" | "Messiah" | "TreacheryCard" => E::from(hero(v)),
                    _ => E::from(location(v)),
                },
                "rc" => E::from(resource_card(v)),
                "payment" => E::from(Payment::of(rest.parse().unwrap())),
                _ => panic!("unknown element encoding {v}"),
            }
        }
        Value::Array(a) => E::list(a.iter().map(element).collect::<Vec<E>>()),
        Value::Object(o) if o.contains_key("list") => collection(v),
        Value::Object(o) if o.contains_key("exception") => {
            // The corpus holds C# exception objects, which render as `Exception.ToString()`:
            // "System.X: message", or just "System.X" when the message is empty. A GameError
            // holding that text renders the same.
            let type_name = o["exception"].as_str().unwrap();
            let message = o["message"].as_str().map_or_else(|| cs::default_message(type_name), str::to_string);
            let full_type_name = cs::full_type_name(type_name);
            let text = if message.is_empty() { full_type_name.to_string() } else { format!("{full_type_name}: {message}") };
            E::Error(treachery_engine::error::GameError::new(text))
        }
        Value::Object(o) if o.contains_key("message") => {
            E::from(Message::express(o["message"].as_array().unwrap().iter().map(element).collect()))
        }
        Value::Object(o) if o.contains_key("part") => {
            E::from(MessagePart::express(o["part"].as_array().unwrap().iter().map(element).collect()))
        }
        _ => panic!("unknown element encoding {v}"),
    }
}

/// A collection of a given C# runtime type.
fn collection(v: &Value) -> E {
    match v["list"].as_str().unwrap() {
        "List<Faction>" => E::list(decode_all(v, en::<Faction>)),
        "List<int>" => E::list(decode_all(v, int)),
        "List<string>" => E::list(decode_all(v, string)),
        "List<Location>" => E::list(decode_all(v, location)),
        "List<Territory>" => E::list(decode_all(v, territory)),
        "List<Leader>" => E::list(decode_all(v, leader)),
        "List<IHero>" => E::list(decode_all(v, hero)),
        "List<TreacheryCard>" => E::list(decode_all(v, card)),
        "List<object>" => E::list(decode_all(v, element)),
        "Faction[]" => E::array(decode_all(v, en::<Faction>)),
        "int[]" => E::array(decode_all(v, int)),
        "Location[]" => E::array(decode_all(v, location)),
        "TreacheryCard[]" => E::array(decode_all(v, card)),
        other => panic!("messages.json has collection type {other}, which this test does not know"),
    }
}

#[test]
fn element_to_string() {
    let mut failures = Failures::default();
    let entries = golden("messages.json")["elements"].as_array().unwrap();
    for e in entries {
        let input = &e["input"];
        let actual = try_enc(|| express!(element(input)).to_cs_string());
        failures.check(|| format!("{input}"), &e["expected"], &actual);
    }
    eprintln!("message corpus: {} elements", entries.len());
    failures.finish("Message.ToString corpus");
}

/// Encodes an element like Enc.cs, for the params check. Arrays are
/// `{"list": "<Type.Name>", "items": [...]}`, other collections plain arrays.
fn enc_element(e: &E) -> Value {
    match e {
        E::Null => Value::Null,
        E::String(s) => s.enc(),
        E::Bool(b) => b.enc(),
        E::Int(i) => i.enc(),
        E::Long(i) => i.enc(),
        E::Float(f) => f.enc(),
        E::Double(f) => f.enc(),
        E::Enum(v) => json!(format!("e:{}:{}", v.ty.name, v.value)),
        E::Location(l) => l.enc(),
        E::Territory(t) => t.enc(),
        E::Hero(h) => h.enc(),
        E::ResourceCard(c) => c.enc(),
        E::Payment(_) => json!("?:Payment"),
        E::Message(m) => m.enc(),
        E::MessagePart(m) => m.enc(),
        E::Error(x) => json!(format!("?:{}", x.message().split(':').next().unwrap_or("").trim_start_matches("System.").rsplit('.').next().unwrap_or(""))),
        E::Player(_) => json!("?:Player"),
        E::List { type_name, items, .. } if type_name.ends_with("[]") => json!({
            "list": type_name.rsplit('.').next().unwrap(),
            "items": items.iter().map(enc_element).collect::<Vec<_>>(),
        }),
        E::List { items, .. } => Value::Array(items.iter().map(enc_element).collect()),
    }
}

#[test]
fn params_spreading() {
    let m = Map::new();
    let objects = || vec![E::from("a"), E::from(1)];
    let mut failures = Failures::default();
    for c in golden("messages.json")["params"].as_array().unwrap() {
        let case = c["case"].as_str().unwrap();
        let msg = match case {
            "object[]" => express!(@params objects()),
            "string[]" => express!(@params ["a", "b"]),
            "Location[]" => express!(@params [m.arrakeen(), m.carthag()]),
            "Faction[]" => express!([Faction::Atreides, Faction::Harkonnen]),
            "int[]" => express!([1, 2]),
            "List<object>" => express!(objects()),
            "object[] and more" => express!(E::array(objects()), "x"),
            "(object)object[]" => express!(E::array(objects())),
            "no arguments" => express!(),
            "ExpressTo object[]" => express_to!(Faction::Atreides; @params objects()),
            "MessagePart object[]" => express!(express_part!(@params objects())),
            // C# passes a null array: Elements is null and ToString throws.
            // Rust has no null params array; this checks the C# side still
            // behaves as documented in message.rs.
            "null object[]" | "null literal" => {
                assert!(c["elements"].is_null(), "{case}");
                assert_eq!(c["to_string"], "x:ArgumentNullException:Value cannot be null. (Parameter 'values')", "{case}");
                continue;
            }
            other => panic!("messages.json has params case {other:?}, which this test does not know"),
        };
        let actual = json!({
            "elements": msg.expression.elements.iter().map(enc_element).collect::<Vec<_>>(),
            "to_string": msg.to_cs_string().enc(),
        });
        let expected = json!({ "elements": c["elements"], "to_string": c["to_string"] });
        failures.check(|| case.to_string(), &expected, &actual);
    }
    failures.finish("params spreading");
}

#[allow(dead_code)]
fn _types(_: Hero, _: Leader, _: Location, _: Territory, _: TreacheryCard) {}
