//! Every Describe/Format/GetImageUrl/GetPopup overload on every static object, against C#.
//!
//! `tests/golden/static.jsonl.gz` comes from the C# probe (`csharp-ref/Static.cs`); regenerate
//! it as described in rust/docs/skin.md. Each line names a call and its arguments; this test
//! makes the same call in Rust. An unknown call fails the test, so a call added to the probe
//! must be added here too.

mod common;

use common::*;
use serde_json::Value;
use treachery_engine::cs::Shared;
use treachery_engine::expression::ExpressionElement;
use treachery_engine::game::Game;
use treachery_engine::i_hero::Hero;
use treachery_engine::location::Location;
use treachery_engine::map::Map;
use treachery_engine::participation::Participation;
use treachery_engine::point::PointD;
use treachery_skin::default_skin;

fn p(p: PointD) -> String {
    format!("{},{}", dotnet_compat::fmt::f64_to_string(p.x), dotnet_compat::fmt::f64_to_string(p.y))
}

fn location(tok: &Value) -> Location {
    match element(tok) {
        ExpressionElement::Location(l) => l,
        _ => panic!("{tok} is not a location"),
    }
}

fn hero(tok: &Value) -> Hero {
    match element(tok) {
        ExpressionElement::Hero(h) => h,
        _ => panic!("{tok} is not a hero"),
    }
}

#[test]
fn static_corpus_matches_csharp() {
    let s = default_skin();
    let map = Map::new();
    let mut attach_game = Game::new_int32_participation(Game::LATEST_VERSION, &Shared::new(Participation::default()));
    let mut diffs = Diffs::new();

    for line in read_lines(&golden("static.jsonl.gz")) {
        let v: Value = serde_json::from_str(&line).unwrap();
        let call = v["call"].as_str().unwrap();
        let a = v["a"].as_array().unwrap();
        let expected = v["out"].as_str().unwrap();
        let actual = outcome(|| match call {
            "Describe" => s.describe(&element(&a[0])),
            "GetImageUrl" => s.get_image_url(&element(&a[0])),
            "DescribeErrorType" => s.describe_error_type(enum_arg(&a[0])),
            "DescribeDetailed" => s.describe_detailed(enum_arg(&a[0])),
            "GetSound" => s.get_sound(enum_arg(&a[0])),
            "GetImageUrlWorld" => s.get_image_url_world(enum_arg(&a[0])),
            "GetHomeworldCardImageUrl" => s.get_homeworld_card_image_url(enum_arg(&a[0])),
            "GetFactionColor" => s.get_faction_color(enum_arg(&a[0])),
            "GetFactionColorTransparent" => s.get_faction_color_transparent(enum_arg(&a[0]), "80"),
            "GetFactionTableImageUrl" => s.get_faction_table_image_url(enum_arg(&a[0])),
            "GetFactionFaceDownImageUrl" => s.get_faction_face_down_image_url(enum_arg(&a[0])),
            "GetFactionForceImageUrl" => s.get_faction_force_image_url(enum_arg(&a[0])),
            "GetFactionSpecialForceImageUrl" => s.get_faction_special_force_image_url(enum_arg(&a[0])),
            "FactionChipHtml" => {
                let ids: Vec<i32> = a[2..].iter().map(|t| int_arg(t) as i32).collect();
                let you = bool_arg(&a[1]);
                s.faction_chip_html(enum_arg(&a[0]), you, if a.len() > 2 { Some(&ids) } else { None })
            }
            "GetPopupStrongholdAdvantageFaction" => s.get_popup_stronghold_advantage_faction(enum_arg(&a[0]), enum_arg(&a[1])),
            "GetImageUrlDiscoveryTokenType" => s.get_image_url_discovery_token_type(enum_arg(&a[0])),
            "GetImageUrlDiscoveryToken" => s.get_image_url_discovery_token(enum_arg(&a[0])),
            "GetDiscoveryTokenDescription" => s.get_discovery_token_description(enum_arg(&a[0])),
            "GetPopupDiscoveryToken" => s.get_popup_discovery_token(enum_arg(&a[0])),
            "GetTerrorTypeDescription" => s.get_terror_type_description(enum_arg(&a[0])),
            "GetPopupTerrorType" => s.get_popup_terror_type(enum_arg(&a[0])),
            "GetAmbassadorDescription" => s.get_ambassador_description(enum_arg(&a[0])),
            "GetPopupAmbassador" => s.get_popup_ambassador(enum_arg(&a[0])),
            "GetPopupLeaderSkill" => s.get_popup_leader_skill(enum_arg(&a[0])),
            "GetPopupTechToken" => s.get_popup_tech_token(enum_arg(&a[0])),
            "GetNexusCardPopup" => s.get_nexus_card_popup(enum_arg(&a[0])),
            "GetPopupStrongholdAdvantage" => s.get_popup_stronghold_advantage(enum_arg(&a[0])),
            "DescribeLocation" => s.describe_location(location(&a[0])),
            "GetCenter" => p(s.get_center(location(&a[0]), &map)),
            "GetSpiceLocation" => p(s.get_spice_location(location(&a[0]))),
            "GetPopupHomeworld" => s.get_popup_homeworld(location(&a[0]).as_homeworld().expect("homeworld")),
            "GetTerritoryBorder" => match element(&a[0]) {
                ExpressionElement::Territory(t) => s.get_territory_border(Some(t)),
                _ => panic!("not a territory"),
            },
            "GetCenterAttached" => {
                let attached = location(&a[0]).as_attached_location().expect("attached location");
                attached.point_at(&mut attach_game.map, location(&a[1]));
                p(s.get_center(location(&a[0]), &attach_game.map))
            }
            "DescribeHero" => s.describe_hero(Some(hero(&a[0]))),
            "GetImageUrlHero" => s.get_image_url_hero(Some(hero(&a[0]))),
            "GetPopupHero" => s.get_popup_hero(hero(&a[0])),
            "GetPopupLeaderSkillPair" => s.get_popup_leader_leader_skill(hero(&a[0]).as_leader(), enum_arg(&a[1])),
            "GetImageUrlTreacheryCard" => s.get_image_url_treachery_card(hero(&a[0]).as_treachery_card()),
            "GetPopupTreacheryCard" => s.get_popup_treachery_card(hero(&a[0]).as_treachery_card()),
            "WithSpiceIcons" => s.with_spice_icons(&str_arg(&a[0]), int_arg(&a[1]) as i32, bool_arg(&a[2])),
            "SpiceHtml" => s.spice_html(&str_arg(&a[0]), int_arg(&a[1]) as i32),
            "Format" => {
                let args: Vec<ExpressionElement> = a[1..].iter().map(element).collect();
                s.format(&str_arg(&a[0]), &args)
            }
            other => panic!("unknown call {other} in the static corpus: add it to this test"),
        });
        diffs.check(&format!("{call}({})", Value::Array(a.clone())), expected, &actual);
    }
    diffs.finish("static corpus");
}
