//! The casts `bots/src/support.rs` applies to the untyped parameters of a Clairvoyance question,
//! for every kind of value a parameter can hold.
//!
//! `golden/b6-unbox.json` lists `[value, cast, expected result]`; a cast that panics has the
//! result `EX:<panic message>`. The results are snapshots of the current behavior. After a
//! deliberate change, rewrite them with
//! `UPDATE_GOLDENS=1 cargo test --release -p treachery-bots --test b6_unbox` and review the diff.

mod common;

use serde_json::json;
use treachery_bots::prelude::*;
use treachery_engine::error::catch_panic;
use treachery_bots::support::{cast_hero, unbox_enum, unbox_float, unbox_int};
use treachery_engine::messages::expression::EnumValue;
use treachery_engine::identity::IFetcher;

use common::{read_json, updating, write_json};

fn value(game: &Game, kind: &str) -> ExpressionElement {
    let (k, v) = kind.split_once(':').unwrap_or((kind, ""));
    let n = || v.parse::<i32>().unwrap();
    match k {
        "null" => ExpressionElement::Null,
        "TreacheryCardType" => ExpressionElement::Enum(EnumValue::of(TreacheryCardType(n()))),
        "Faction" => ExpressionElement::Enum(EnumValue::of(Faction(n()))),
        "int" => ExpressionElement::Int(n()),
        "float" => ExpressionElement::Float(v.parse().unwrap()),
        "double" => ExpressionElement::Double(v.parse().unwrap()),
        "string" => ExpressionElement::String(v.to_string()),
        "leader" => ExpressionElement::Hero(LeaderManager::hero_lookup().find(n()).unwrap()),
        "messiah" => ExpressionElement::Hero(Hero::Messiah),
        "card" => ExpressionElement::Hero(Hero::TreacheryCard(TreacheryCardManager::lookup().find(n()).unwrap())),
        "territory" => ExpressionElement::Territory(game.map.territory_lookup().find(n()).unwrap()),
        _ => panic!("unknown value kind {kind}"),
    }
}

#[test]
fn unboxing_casts() {
    let golden = read_json("b6-unbox.json");
    let game = Game::new();
    let mut failures = Vec::new();
    let mut updated = Vec::new();
    let cases = golden.as_array().unwrap();
    for case in cases {
        let (kind, cast, expected) = (case[0].as_str().unwrap(), case[1].as_str().unwrap(), case[2].as_str().unwrap());
        let o = value(&game, kind);
        let actual = catch_panic(|| match cast {
            "TreacheryCardType" => format!("enum:{}", unbox_enum::<TreacheryCardType>(&o).0),
            "Faction" => format!("enum:{}", unbox_enum::<Faction>(&o).0),
            "int" => format!("int:{}", unbox_int(&o)),
            "float" => format!("float:{}", dotnet_compat::fmt::f32_to_string(unbox_float(&o))),
            "IHero" => cast_hero(&o).map_or("null".to_string(), |h| format!("hero:{}", h.id())),
            _ => panic!("unknown cast {cast}"),
        })
        .unwrap_or_else(|e| format!("EX:{e}"));
        if actual != expected {
            failures.push(format!("({cast}){kind}: expected {expected}, actual {actual}"));
        }
        updated.push(json!([kind, cast, actual]));
    }
    assert!(cases.len() >= 50);
    if updating() {
        write_json("b6-unbox.json", &json!(updated));
        return;
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
