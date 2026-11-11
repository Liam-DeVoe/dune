//! The unboxing casts of `bots/src/support.rs` against .NET.
//!
//! `golden/b6-unbox.json` comes from `bots/csharp-ref` (see B6.cs, `Unbox`):
//!
//! ```sh
//! export DOTNET_ROOT="$HOME/dotnet10" PATH="$HOME/dotnet10:$PATH"
//! cd rust/bots/csharp-ref && dotnet run -c Release -- b6unbox ../tests/golden/b6-unbox.json
//! ```

use serde_json::Value;
use treachery_bots::prelude::*;
use treachery_bots::support::{cast_hero, unbox_enum, unbox_float, unbox_int};
use treachery_engine::expression::EnumValue;
use treachery_engine::i_fetcher::IFetcher;

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
fn unboxing_casts_match_dotnet() {
    let path = format!("{}/tests/golden/b6-unbox.json", env!("CARGO_MANIFEST_DIR"));
    let golden: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    let game = Game::new();
    let mut failures = Vec::new();
    let cases = golden.as_array().unwrap();
    for case in cases {
        let (kind, cast, expected) = (case[0].as_str().unwrap(), case[1].as_str().unwrap(), case[2].as_str().unwrap());
        let o = value(&game, kind);
        let actual = cs::catch(|| match cast {
            "TreacheryCardType" => format!("enum:{}", unbox_enum::<TreacheryCardType>(&o).0),
            "Faction" => format!("enum:{}", unbox_enum::<Faction>(&o).0),
            "int" => format!("int:{}", unbox_int(&o)),
            "float" => format!("float:{}", dotnet_compat::fmt::f32_to_string(unbox_float(&o))),
            "IHero" => cast_hero(&o).map_or("null".to_string(), |h| format!("hero:{}", h.id())),
            _ => panic!("unknown cast {cast}"),
        })
        .unwrap_or_else(|e| format!("EX:{}: {}", cs::full_type_name(e.type_name), e.message));
        if actual != expected {
            failures.push(format!("({cast}){kind}: C# {expected}, Rust {actual}"));
        }
    }
    assert!(cases.len() >= 50);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
