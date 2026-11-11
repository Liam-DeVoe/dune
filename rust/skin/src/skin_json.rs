//! Port of the `#region LoadingAndSaving` of Model/Skin.cs: `Skin.Load`, `Fix` and
//! `SkinToString`, with System.Text.Json's encoding of a `Skin` (`Utilities.Serialize`, which
//! includes fields).
//!
//! The member order is the one System.Text.Json writes: the properties (`DrawResourceIconsOnMap`,
//! `ShowVerboseToolipsOnMap`, `ShowArrowsForRecentMoves`, `HmsDx`, `HmsRadius`) first, then the
//! fields in declaration order. `tests/skin_json.rs` checks both directions against C#.
//!
//! Reading follows System.Text.Json where a skin file can reach it: member names are
//! case-sensitive, unknown members are ignored, the get-only `HmsDx`/`HmsRadius` are skipped,
//! a dictionary key must parse as the key type, and a value of the wrong JSON type fails. Error
//! messages are not .NET's; only the exception type is.

use serde_json::Value;
use treachery_engine::cs::CsException;
use treachery_engine::json::write_escaped;
use treachery_engine::net_enum::NetEnum;
use treachery_engine::point::PointD;

use crate::skin::{Skin, SkinDictionary, SkinKey};

/// Reads an `int` dictionary key the way System.Text.Json does: an optional `-` and digits,
/// in range.
pub(crate) fn parse_int_key(text: &str) -> Option<i32> {
    let digits = text.strip_prefix('-').unwrap_or(text);
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}

/// Reads an enum dictionary key the way System.Text.Json does: a member name (case-sensitive)
/// in any case, or a number.
pub(crate) fn parse_enum_key<T: NetEnum>(text: &str) -> Option<T> {
    if let Some((_, v)) = T::INFO.declared.iter().find(|(n, _)| *n == text) {
        return Some(T::from_value(*v));
    }
    // Then ignoring case, as `Enum.TryParse(ignoreCase: true)`.
    if let Some((_, v)) = T::INFO.declared.iter().find(|(n, _)| n.eq_ignore_ascii_case(text)) {
        return Some(T::from_value(*v));
    }
    parse_int_key(text).map(T::from_value)
}

/// One member of the serialized `Skin`.
enum Member<'a> {
    Bool(&'a mut bool),
    Int(&'a mut i32),
    /// `None` is a JSON `null`, which `Fix` replaces with the donor's value.
    Str(&'a mut String),
    Point(&'a mut PointD),
    Dict(&'a mut dyn DictMember),
    /// A get-only property: written, skipped when read.
    Computed(Value),
}

trait DictMember {
    fn write(&self, out: &mut String);
    fn read(&mut self, v: &serde_json::Map<String, Value>) -> Result<(), CsException>;
}

impl<K: SkinKey> DictMember for SkinDictionary<K, String> {
    fn write(&self, out: &mut String) {
        out.push('{');
        for (i, (k, v)) in self.entries.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            write_escaped(out, &k.key_text());
            out.push(':');
            write_escaped(out, v);
        }
        out.push('}');
    }

    fn read(&mut self, v: &serde_json::Map<String, Value>) -> Result<(), CsException> {
        let mut d = SkinDictionary::default();
        for (k, v) in v {
            let key = K::parse_key(k).ok_or_else(|| json_error(&format!("invalid key {k}")))?;
            let value = v.as_str().ok_or_else(|| json_error("a label must be a string"))?;
            d.set(key, value.to_string());
        }
        *self = d;
        Ok(())
    }
}

impl<K: SkinKey> DictMember for SkinDictionary<K, PointD> {
    fn write(&self, out: &mut String) {
        out.push('{');
        for (i, (k, v)) in self.entries.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            write_escaped(out, &k.key_text());
            out.push(':');
            write_point(out, *v);
        }
        out.push('}');
    }

    fn read(&mut self, v: &serde_json::Map<String, Value>) -> Result<(), CsException> {
        let mut d = SkinDictionary::default();
        for (k, v) in v {
            let key = K::parse_key(k).ok_or_else(|| json_error(&format!("invalid key {k}")))?;
            d.set(key, read_point(v)?);
        }
        *self = d;
        Ok(())
    }
}

fn json_error(message: &str) -> CsException {
    CsException { type_name: "JsonException", message: message.to_string() }
}

fn write_point(out: &mut String, p: PointD) {
    out.push_str(&format!(
        "{{\"X\":{},\"Y\":{}}}",
        dotnet_compat::fmt::f64_to_string(p.x),
        dotnet_compat::fmt::f64_to_string(p.y)
    ));
}

fn read_point(v: &Value) -> Result<PointD, CsException> {
    let o = v.as_object().ok_or_else(|| json_error("a point must be an object"))?;
    let mut p = PointD::default();
    for (k, v) in o {
        let x = || v.as_f64().ok_or_else(|| json_error("a coordinate must be a number"));
        match k.as_str() {
            "X" => p.x = x()?,
            "Y" => p.y = x()?,
            _ => {}
        }
    }
    Ok(p)
}

/// Every serialized member, in System.Text.Json's order.
fn members(s: &mut Skin) -> Vec<(&'static str, Member<'_>)> {
    use Member::*;
    let hms_dx = Value::from(s.hms_dx() as f64);
    let hms_radius = Value::String(dotnet_compat::fmt::f32_to_string(s.hms_radius()));
    vec![
        ("DrawResourceIconsOnMap", Bool(&mut s.draw_resource_icons_on_map)),
        ("ShowVerboseToolipsOnMap", Bool(&mut s.show_verbose_toolips_on_map)),
        ("ShowArrowsForRecentMoves", Bool(&mut s.show_arrows_for_recent_moves)),
        ("HmsDx", Computed(hms_dx)),
        ("HmsRadius", Computed(hms_radius)),
        ("Version", Int(&mut s.version)),
        ("Concept_STR", Dict(&mut s.concept_str)),
        ("MainPhase_STR", Dict(&mut s.main_phase_str)),
        ("PersonName_STR", Dict(&mut s.person_name_str)),
        ("PersonImage_URL", Dict(&mut s.person_image_url)),
        ("TerritoryName_STR", Dict(&mut s.territory_name_str)),
        ("TerritoryBorder_SVG", Dict(&mut s.territory_border_svg)),
        ("LocationCenter_Point", Dict(&mut s.location_center_point)),
        ("LocationSpice_Point", Dict(&mut s.location_spice_point)),
        ("FactionName_STR", Dict(&mut s.faction_name_str)),
        ("FactionImage_URL", Dict(&mut s.faction_image_url)),
        ("FactionTableImage_URL", Dict(&mut s.faction_table_image_url)),
        ("FactionFacedownImage_URL", Dict(&mut s.faction_facedown_image_url)),
        ("FactionForceImage_URL", Dict(&mut s.faction_force_image_url)),
        ("FactionSpecialForceImage_URL", Dict(&mut s.faction_special_force_image_url)),
        ("FactionColor", Dict(&mut s.faction_color)),
        ("ForceName_STR", Dict(&mut s.force_name_str)),
        ("SpecialForceName_STR", Dict(&mut s.special_force_name_str)),
        ("TechTokenName_STR", Dict(&mut s.tech_token_name_str)),
        ("TechTokenImage_URL", Dict(&mut s.tech_token_image_url)),
        ("Sound", Dict(&mut s.sound)),
        ("TreacheryCardType_STR", Dict(&mut s.treachery_card_type_str)),
        ("TreacheryCardName_STR", Dict(&mut s.treachery_card_name_str)),
        ("TreacheryCardDescription_STR", Dict(&mut s.treachery_card_description_str)),
        ("TreacheryCardImage_URL", Dict(&mut s.treachery_card_image_url)),
        ("TechTokenDescription_STR", Dict(&mut s.tech_token_description_str)),
        ("ResourceCardImage_URL", Dict(&mut s.resource_card_image_url)),
        ("LeaderSkillCardName_STR", Dict(&mut s.leader_skill_card_name_str)),
        ("LeaderSkillCardImage_URL", Dict(&mut s.leader_skill_card_image_url)),
        ("StrongholdCardName_STR", Dict(&mut s.stronghold_card_name_str)),
        ("StrongholdCardImage_URL", Dict(&mut s.stronghold_card_image_url)),
        ("HomeWorldImage_URL", Dict(&mut s.home_world_image_url)),
        ("HomeWorldCardImage_URL", Dict(&mut s.home_world_card_image_url)),
        ("NexusCardImage_URL", Dict(&mut s.nexus_card_image_url)),
        ("TerrorTokenName_STR", Dict(&mut s.terror_token_name_str)),
        ("TerrorTokenDescription_STR", Dict(&mut s.terror_token_description_str)),
        ("DiscoveryTokenName_STR", Dict(&mut s.discovery_token_name_str)),
        ("DiscoveryTokenDescription_STR", Dict(&mut s.discovery_token_description_str)),
        ("DiscoveryTokenImage_URL", Dict(&mut s.discovery_token_image_url)),
        ("DiscoveryTokenTypeName_STR", Dict(&mut s.discovery_token_type_name_str)),
        ("DiscoveryTokenTypeImage_URL", Dict(&mut s.discovery_token_type_image_url)),
        ("AmbassadorImage_URL", Dict(&mut s.ambassador_image_url)),
        ("AmbassadorName_STR", Dict(&mut s.ambassador_name_str)),
        ("AmbassadorDescription_STR", Dict(&mut s.ambassador_description_str)),
        ("MusicGeneral_URL", Str(&mut s.music_general_url)),
        ("MusicResourceBlow_URL", Str(&mut s.music_resource_blow_url)),
        ("MusicSetup_URL", Str(&mut s.music_setup_url)),
        ("MusicBidding_URL", Str(&mut s.music_bidding_url)),
        ("MusicShipmentAndMove_URL", Str(&mut s.music_shipment_and_move_url)),
        ("MusicBattle_URL", Str(&mut s.music_battle_url)),
        ("MusicBattleClimax_URL", Str(&mut s.music_battle_climax_url)),
        ("MusicMentat_URL", Str(&mut s.music_mentat_url)),
        ("Sound_YourTurn_URL", Str(&mut s.sound_your_turn_url)),
        ("Sound_Chatmessage_URL", Str(&mut s.sound_chatmessage_url)),
        ("Map_URL", Str(&mut s.map_url)),
        ("Eye_URL", Str(&mut s.eye_url)),
        ("EyeSlash_URL", Str(&mut s.eye_slash_url)),
        ("CardBack_ResourceCard_URL", Str(&mut s.card_back_resource_card_url)),
        ("CardBack_TreacheryCard_URL", Str(&mut s.card_back_treachery_card_url)),
        ("BattleScreen_URL", Str(&mut s.battle_screen_url)),
        ("Messiah_URL", Str(&mut s.messiah_url)),
        ("Monster_URL", Str(&mut s.monster_url)),
        ("Harvester_URL", Str(&mut s.harvester_url)),
        ("Resource_URL", Str(&mut s.resource_url)),
        ("HMS_URL", Str(&mut s.hms_url)),
        ("HighThreshold_URL", Str(&mut s.high_threshold_url)),
        ("LowThreshold_URL", Str(&mut s.low_threshold_url)),
        ("MapDimensions", Point(&mut s.map_dimensions)),
        ("PlanetCenter", Point(&mut s.planet_center)),
        ("PlanetRadius", Int(&mut s.planet_radius)),
        ("MapRadius", Int(&mut s.map_radius)),
        ("PlayerTokenRadius", Int(&mut s.player_token_radius)),
        ("SpiceDeckLocation", Point(&mut s.spice_deck_location)),
        ("TreacheryDeckLocation", Point(&mut s.treachery_deck_location)),
        ("CardSize", Point(&mut s.card_size)),
        ("BattleScreenWidth", Int(&mut s.battle_screen_width)),
        ("BattleScreenHeroX", Int(&mut s.battle_screen_hero_x)),
        ("BattleScreenHeroY", Int(&mut s.battle_screen_hero_y)),
        ("BattleWheelHeroWidth", Int(&mut s.battle_wheel_hero_width)),
        ("BattleWheelHeroHeight", Int(&mut s.battle_wheel_hero_height)),
        ("BattleWheelForcesX", Int(&mut s.battle_wheel_forces_x)),
        ("BattleWheelForcesY", Int(&mut s.battle_wheel_forces_y)),
        ("BattleWheelCardX", Int(&mut s.battle_wheel_card_x)),
        ("BattleWheelCardY", Int(&mut s.battle_wheel_card_y)),
        ("BattleWheelCardWidth", Int(&mut s.battle_wheel_card_width)),
        ("BattleWheelCardHeight", Int(&mut s.battle_wheel_card_height)),
        ("MONSTERTOKEN_RADIUS", Int(&mut s.monstertoken_radius)),
        ("FORCETOKEN_FONT", Str(&mut s.forcetoken_font)),
        ("FORCETOKEN_FONTCOLOR", Str(&mut s.forcetoken_fontcolor)),
        ("FORCETOKEN_FONT_BORDERCOLOR", Str(&mut s.forcetoken_font_bordercolor)),
        ("FORCETOKEN_FONT_BORDERWIDTH", Int(&mut s.forcetoken_font_borderwidth)),
        ("FORCETOKEN_RADIUS", Int(&mut s.forcetoken_radius)),
        ("RESOURCETOKEN_FONT", Str(&mut s.resourcetoken_font)),
        ("RESOURCETOKEN_FONTCOLOR", Str(&mut s.resourcetoken_fontcolor)),
        ("RESOURCETOKEN_FONT_BORDERCOLOR", Str(&mut s.resourcetoken_font_bordercolor)),
        ("RESOURCETOKEN_FONT_BORDERWIDTH", Int(&mut s.resourcetoken_font_borderwidth)),
        ("RESOURCETOKEN_RADIUS", Int(&mut s.resourcetoken_radius)),
        ("HIGHLIGHT_OVERLAY_COLOR", Str(&mut s.highlight_overlay_color)),
        ("METHEOR_OVERLAY_COLOR", Str(&mut s.metheor_overlay_color)),
        ("BLOWNSHIELDWALL_OVERLAY_COLOR", Str(&mut s.blownshieldwall_overlay_color)),
        ("STORM_OVERLAY_COLOR", Str(&mut s.storm_overlay_color)),
        ("STORM_PRESCIENCE_OVERLAY_COLOR", Str(&mut s.storm_prescience_overlay_color)),
        ("CARDPILE_FONT", Str(&mut s.cardpile_font)),
        ("CARDPILE_FONTCOLOR", Str(&mut s.cardpile_fontcolor)),
        ("CARDPILE_FONT_BORDERCOLOR", Str(&mut s.cardpile_font_bordercolor)),
        ("CARDPILE_FONT_BORDERWIDTH", Int(&mut s.cardpile_font_borderwidth)),
        ("PHASE_FONT", Str(&mut s.phase_font)),
        ("PHASE_ACTIVE_FONT", Str(&mut s.phase_active_font)),
        ("PHASE_FONTCOLOR", Str(&mut s.phase_fontcolor)),
        ("PHASE_ACTIVE_FONTCOLOR", Str(&mut s.phase_active_fontcolor)),
        ("PHASE_FONT_BORDERCOLOR", Str(&mut s.phase_font_bordercolor)),
        ("PHASE_FONT_BORDERWIDTH", Int(&mut s.phase_font_borderwidth)),
        ("PHASE_ACTIVE_FONT_BORDERWIDTH", Int(&mut s.phase_active_font_borderwidth)),
        ("PLAYERNAME_FONT", Str(&mut s.playername_font)),
        ("PLAYERNAME_FONTCOLOR", Str(&mut s.playername_fontcolor)),
        ("PLAYERNAME_FONT_BORDERCOLOR", Str(&mut s.playername_font_bordercolor)),
        ("PLAYERNAME_FONT_BORDERWIDTH", Int(&mut s.playername_font_borderwidth)),
        ("SKILL_FONT", Str(&mut s.skill_font)),
        ("SKILL_FONTCOLOR", Str(&mut s.skill_fontcolor)),
        ("SKILL_FONT_BORDERCOLOR", Str(&mut s.skill_font_bordercolor)),
        ("SKILL_FONT_BORDERWIDTH", Int(&mut s.skill_font_borderwidth)),
        ("TABLEPOSITION_BACKGROUNDCOLOR", Str(&mut s.tableposition_backgroundcolor)),
        ("TURN_FONT", Str(&mut s.turn_font)),
        ("TURN_FONT_COLOR", Str(&mut s.turn_font_color)),
        ("TURN_FONT_BORDERCOLOR", Str(&mut s.turn_font_bordercolor)),
        ("TURN_FONT_BORDERWIDTH", Int(&mut s.turn_font_borderwidth)),
        ("WHEEL_FONT", Str(&mut s.wheel_font)),
        ("WHEEL_FONTCOLOR", Str(&mut s.wheel_fontcolor)),
        ("WHEEL_FONT_AGGRESSOR_BORDERCOLOR", Str(&mut s.wheel_font_aggressor_bordercolor)),
        ("WHEEL_FONT_DEFENDER_BORDERCOLOR", Str(&mut s.wheel_font_defender_bordercolor)),
        ("WHEEL_FONT_BORDERWIDTH", Int(&mut s.wheel_font_borderwidth)),
        ("SHADOW", Str(&mut s.shadow)),
        ("FACTION_INFORMATIONCARDSTYLE", Str(&mut s.faction_informationcardstyle)),
    ]
}

/// What a member held after reading: `null` survives into `Fix`.
#[derive(Clone, Copy, PartialEq)]
enum ReadState {
    Missing,
    Null,
    Set,
}

impl Skin {
    /// `SkinToString() => Utilities.Serialize(this)`
    pub fn skin_to_string(&self) -> String {
        let mut copy = self.clone();
        let mut out = String::from("{");
        for (i, (name, m)) in members(&mut copy).into_iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            write_escaped(&mut out, name);
            out.push(':');
            match m {
                Member::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
                Member::Int(i) => out.push_str(&i.to_string()),
                Member::Str(s) => write_escaped(&mut out, s),
                Member::Point(p) => write_point(&mut out, *p),
                Member::Dict(d) => d.write(&mut out),
                Member::Computed(Value::String(s)) => out.push_str(&s),
                Member::Computed(v) => out.push_str(&dotnet_compat::fmt::f64_to_string(v.as_f64().unwrap())),
            }
        }
        out.push('}');
        out
    }

    /// `public static Skin Load(string data, Skin donor)`: reads a skin and fills what it lacks
    /// from `donor` (`Fix`). C# throws; this returns the exception.
    pub fn load(data: &str, donor: &Skin) -> Result<Skin, CsException> {
        let v: Value = serde_json::from_str(data).map_err(|e| json_error(&e.to_string()))?;
        let o = match v {
            Value::Null => return Err(CsException { type_name: "Exception", message: "Invalid skin".into() }),
            Value::Object(o) => o,
            _ => return Err(json_error("a skin must be an object")),
        };
        let mut result = Skin::default();
        let mut states = Vec::new();
        for (name, m) in members(&mut result) {
            let state = match o.get(name) {
                None => ReadState::Missing,
                Some(Value::Null) => ReadState::Null,
                Some(v) => {
                    read_member(m, v)?;
                    ReadState::Set
                }
            };
            states.push((name, state));
        }
        fix(&mut result, donor, &states)?;
        Ok(result)
    }
}

fn read_member(m: Member<'_>, v: &Value) -> Result<(), CsException> {
    match m {
        Member::Bool(b) => *b = v.as_bool().ok_or_else(|| json_error("expected a boolean"))?,
        Member::Int(i) => {
            *i = v.as_i64().and_then(|x| i32::try_from(x).ok()).ok_or_else(|| json_error("expected an Int32"))?;
        }
        Member::Str(s) => *s = v.as_str().ok_or_else(|| json_error("expected a string"))?.to_string(),
        Member::Point(p) => *p = read_point(v)?,
        Member::Dict(d) => d.read(v.as_object().ok_or_else(|| json_error("expected an object"))?)?,
        Member::Computed(_) => {}
    }
    Ok(())
}

/// `private static void Fix(Skin toFix, Skin donor)`: a dictionary that lacks any of the donor's
/// keys is replaced by the donor's whole dictionary (`FixDictionary`); a `null` string and a
/// zero number or point take the donor's value (`FixValue`). An empty string is kept, because
/// `"".Equals(default(string))` is false. A `null` dictionary throws NullReferenceException in
/// `FixDictionary` unless the donor's is empty.
fn fix(to_fix: &mut Skin, donor: &Skin, states: &[(&str, ReadState)]) -> Result<(), CsException> {
    let mut donor = donor.clone();
    let donor_members = members(&mut donor);
    let to_fix_members = members(to_fix);
    for (((name, m), (_, d)), (_, state)) in to_fix_members.into_iter().zip(donor_members).zip(states) {
        match (m, d) {
            (Member::Dict(t), Member::Dict(d)) => {
                let mut td = String::new();
                t.write(&mut td);
                let mut dd = String::new();
                d.write(&mut dd);
                let t_keys = keys_of(&td);
                let d_keys = keys_of(&dd);
                if *state == ReadState::Null && !d_keys.is_empty() {
                    return Err(CsException {
                        type_name: "NullReferenceException",
                        message: treachery_engine::cs::NULL_REFERENCE_MESSAGE.into(),
                    });
                }
                if d_keys.iter().any(|k| !t_keys.contains(k)) {
                    let parsed: Value = serde_json::from_str(&dd).unwrap();
                    t.read(parsed.as_object().unwrap())?;
                }
            }
            (Member::Str(t), Member::Str(d)) => {
                if *state == ReadState::Null {
                    *t = d.clone();
                }
            }
            (Member::Int(t), Member::Int(d)) => {
                // Version has no FixValue.
                if *t == 0 && name != "Version" {
                    *t = *d;
                }
            }
            (Member::Point(t), Member::Point(d)) => {
                if *t == PointD::default() {
                    *t = *d;
                }
            }
            _ => {}
        }
    }
    Ok(())
}

/// The keys of a serialized dictionary, in order.
fn keys_of(json: &str) -> Vec<String> {
    let v: Value = serde_json::from_str(json).unwrap();
    v.as_object().unwrap().keys().cloned().collect()
}
