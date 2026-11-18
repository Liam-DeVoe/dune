//! The skin: names, images and the text of log messages.
//!
//! [`Skin::render`] renders a message, [`Skin::describe`] any [`Part`], and `describe_<type>` one value of a
//! known type, such as [`Skin::describe_faction`].
//!
//! The faction rules sheets are in [`crate::faction_manual`], and loading and saving skins in
//! [`crate::skin_json`].

use treachery_engine::enumerations::*;
use treachery_engine::messages::part::Part;
use treachery_engine::parts;
use treachery_engine::game::Game;
use treachery_engine::map::homeworld::Homeworld;
use treachery_engine::heroes::hero::Hero;
use treachery_engine::heroes::leader::Leader;
use treachery_engine::map::location::Location;
use treachery_engine::map::Map;
use treachery_engine::messages::message::Message;
use crate::point::PointD;
use treachery_engine::cards::resource_card::ResourceCard;
use treachery_engine::map::territory::Territory;
use treachery_engine::cards::treachery_card::TreacheryCard;
use treachery_engine::cards::treachery_card_manager::TreacheryCardManager;

/// A key of a skin dictionary: an `int` or an enum.
pub trait SkinKey: Copy + PartialEq + 'static {
    /// The key's `ToString()`, as `KeyNotFoundException` and System.Text.Json print it.
    fn key_text(self) -> String;
    /// Reads a dictionary key the way System.Text.Json does. `None` if it is not a valid key.
    fn parse_key(text: &str) -> Option<Self>;
}

impl SkinKey for i32 {
    fn key_text(self) -> String {
        self.to_string()
    }
    fn parse_key(text: &str) -> Option<Self> {
        crate::skin_json::parse_int_key(text)
    }
}

/// Keys of an enum whose C# `None` member is absence in Rust: `None` is the key "None".
macro_rules! optional_enum_keys {
    ($($t:ty),* $(,)?) => {$(
        impl SkinKey for Option<$t> {
            fn key_text(self) -> String {
                treachery_engine::enum_type::name_or_absent(self)
            }
            fn parse_key(text: &str) -> Option<Self> {
                crate::skin_json::parse_optional_enum_key::<$t>(text)
            }
        }
    )*};
}

optional_enum_keys!(Faction, MainPhase, TechToken, Concept, Milestone, World, Nexus, DiscoveryToken, LeaderSkill, StrongholdAdvantage, TerrorType, DiscoveryTokenType, TreacheryCardType, Ambassador);

/// A C# `Dictionary<K, V>` of a skin. Entries keep insertion order (nothing is ever removed),
/// which is the order .NET enumerates and serializes them in.
#[derive(Clone, Debug, PartialEq)]
pub struct SkinDictionary<K, V = String> {
    pub entries: Vec<(K, V)>,
}

impl<K, V> Default for SkinDictionary<K, V> {
    fn default() -> Self {
        SkinDictionary { entries: Vec::new() }
    }
}

impl<K: SkinKey, V> SkinDictionary<K, V> {
    pub fn from_vec(entries: Vec<(K, V)>) -> Self {
        SkinDictionary { entries }
    }

    pub fn get(&self, key: K) -> Option<&V> {
        self.entries.iter().find(|(k, _)| *k == key).map(|(_, v)| v)
    }

    pub fn contains_key(&self, key: K) -> bool {
        self.get(key).is_some()
    }

    /// The indexer `dict[key]`: KeyNotFoundException when the key is missing.
    pub fn index(&self, key: K) -> &V {
        match self.get(key) {
            Some(v) => v,
            None => panic!("KeyNotFoundException: The given key '{}' was not present in the dictionary.", key.key_text()),
        }
    }

    /// `dict[key] = value`
    pub fn set(&mut self, key: K, value: V) {
        match self.entries.iter_mut().find(|(k, _)| *k == key) {
            Some(e) => e.1 = value,
            None => self.entries.push((key, value)),
        }
    }

    pub fn keys(&self) -> impl Iterator<Item = K> + '_ {
        self.entries.iter().map(|(k, _)| *k)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl<K: SkinKey> SkinDictionary<K, String> {
    pub fn get_value_or_default(&self, key: K, fallback: &str) -> String {
        self.get(key).cloned().unwrap_or_else(|| fallback.to_string())
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Skin {
    pub version: i32,

    pub concept_str: SkinDictionary<Option<Concept>>,
    pub main_phase_str: SkinDictionary<Option<MainPhase>>,
    pub person_name_str: SkinDictionary<i32>,
    pub person_image_url: SkinDictionary<i32>,
    pub territory_name_str: SkinDictionary<i32>,
    pub territory_border_svg: SkinDictionary<i32>,
    pub location_center_point: SkinDictionary<i32, PointD>,
    pub location_spice_point: SkinDictionary<i32, PointD>,
    pub faction_name_str: SkinDictionary<Option<Faction>>,
    pub faction_image_url: SkinDictionary<Option<Faction>>,
    pub faction_table_image_url: SkinDictionary<Option<Faction>>,
    pub faction_facedown_image_url: SkinDictionary<Option<Faction>>,
    pub faction_force_image_url: SkinDictionary<Option<Faction>>,
    pub faction_special_force_image_url: SkinDictionary<Option<Faction>>,
    pub faction_color: SkinDictionary<Option<Faction>>,
    pub force_name_str: SkinDictionary<Option<Faction>>,
    pub special_force_name_str: SkinDictionary<Option<Faction>>,
    pub tech_token_name_str: SkinDictionary<Option<TechToken>>,
    pub tech_token_image_url: SkinDictionary<Option<TechToken>>,
    pub sound: SkinDictionary<Option<Milestone>>,
    pub treachery_card_type_str: SkinDictionary<Option<TreacheryCardType>>,
    pub treachery_card_name_str: SkinDictionary<i32>,
    pub treachery_card_description_str: SkinDictionary<i32>,
    pub treachery_card_image_url: SkinDictionary<i32>,
    pub tech_token_description_str: SkinDictionary<Option<TechToken>>,
    pub resource_card_image_url: SkinDictionary<i32>,
    pub leader_skill_card_name_str: SkinDictionary<Option<LeaderSkill>>,
    pub leader_skill_card_image_url: SkinDictionary<Option<LeaderSkill>>,
    pub stronghold_card_name_str: SkinDictionary<i32>,
    pub stronghold_card_image_url: SkinDictionary<Option<StrongholdAdvantage>>,
    pub home_world_image_url: SkinDictionary<Option<World>>,
    pub home_world_card_image_url: SkinDictionary<Option<World>>,
    pub nexus_card_image_url: SkinDictionary<Option<Nexus>>,
    pub terror_token_name_str: SkinDictionary<Option<TerrorType>>,
    pub terror_token_description_str: SkinDictionary<Option<TerrorType>>,
    pub discovery_token_name_str: SkinDictionary<Option<DiscoveryToken>>,
    pub discovery_token_description_str: SkinDictionary<Option<DiscoveryToken>>,
    pub discovery_token_image_url: SkinDictionary<Option<DiscoveryToken>>,
    pub discovery_token_type_name_str: SkinDictionary<Option<DiscoveryTokenType>>,
    pub discovery_token_type_image_url: SkinDictionary<Option<DiscoveryTokenType>>,
    pub ambassador_image_url: SkinDictionary<Option<Ambassador>>,
    pub ambassador_name_str: SkinDictionary<Option<Ambassador>>,
    pub ambassador_description_str: SkinDictionary<Option<Ambassador>>,

    pub draw_resource_icons_on_map: bool,
    pub show_verbose_toolips_on_map: bool,
    pub show_arrows_for_recent_moves: bool,

    pub music_general_url: String,
    pub music_resource_blow_url: String,
    pub music_setup_url: String,
    pub music_bidding_url: String,
    pub music_shipment_and_move_url: String,
    pub music_battle_url: String,
    pub music_battle_climax_url: String,
    pub music_mentat_url: String,

    pub sound_your_turn_url: String,
    pub sound_chatmessage_url: String,

    pub map_url: String,
    pub eye_url: String,
    pub eye_slash_url: String,
    pub card_back_resource_card_url: String,
    pub card_back_treachery_card_url: String,
    pub battle_screen_url: String,
    pub messiah_url: String,
    pub monster_url: String,
    pub harvester_url: String,
    pub resource_url: String,
    pub hms_url: String,
    pub high_threshold_url: String,
    pub low_threshold_url: String,

    pub map_dimensions: PointD,
    pub planet_center: PointD,
    pub planet_radius: i32,
    pub map_radius: i32,
    pub player_token_radius: i32,

    pub spice_deck_location: PointD,
    pub treachery_deck_location: PointD,
    pub card_size: PointD,

    pub battle_screen_width: i32,
    pub battle_screen_hero_x: i32,
    pub battle_screen_hero_y: i32,
    pub battle_wheel_hero_width: i32,
    pub battle_wheel_hero_height: i32,
    pub battle_wheel_forces_x: i32,
    pub battle_wheel_forces_y: i32,
    pub battle_wheel_card_x: i32,
    pub battle_wheel_card_y: i32,
    pub battle_wheel_card_width: i32,
    pub battle_wheel_card_height: i32,

    pub monstertoken_radius: i32,

    pub forcetoken_font: String,
    pub forcetoken_fontcolor: String,
    pub forcetoken_font_bordercolor: String,
    pub forcetoken_font_borderwidth: i32,
    pub forcetoken_radius: i32,

    pub resourcetoken_font: String,
    pub resourcetoken_fontcolor: String,
    pub resourcetoken_font_bordercolor: String,
    pub resourcetoken_font_borderwidth: i32,
    pub resourcetoken_radius: i32,

    pub highlight_overlay_color: String,
    pub metheor_overlay_color: String,
    pub blownshieldwall_overlay_color: String,
    pub storm_overlay_color: String,
    pub storm_prescience_overlay_color: String,

    pub cardpile_font: String,
    pub cardpile_fontcolor: String,
    pub cardpile_font_bordercolor: String,
    pub cardpile_font_borderwidth: i32,

    pub phase_font: String,
    pub phase_active_font: String,
    pub phase_fontcolor: String,
    pub phase_active_fontcolor: String,
    pub phase_font_bordercolor: String,
    pub phase_font_borderwidth: i32,
    pub phase_active_font_borderwidth: i32,

    pub playername_font: String,
    pub playername_fontcolor: String,
    pub playername_font_bordercolor: String,
    pub playername_font_borderwidth: i32,

    pub skill_font: String,
    pub skill_fontcolor: String,
    pub skill_font_bordercolor: String,
    pub skill_font_borderwidth: i32,

    pub tableposition_backgroundcolor: String,

    pub turn_font: String,
    pub turn_font_color: String,
    pub turn_font_bordercolor: String,
    pub turn_font_borderwidth: i32,

    pub wheel_font: String,
    pub wheel_fontcolor: String,
    pub wheel_font_aggressor_bordercolor: String,
    pub wheel_font_defender_bordercolor: String,
    pub wheel_font_borderwidth: i32,

    pub shadow: String,

    pub faction_informationcardstyle: String,
}

/// `GetLabel<T>(Dictionary<T, string>? labels, T key)`: the label, or `""` when it is missing.
pub(crate) fn get_label<K: SkinKey>(labels: &SkinDictionary<K>, key: K) -> String {
    labels.get(key).cloned().unwrap_or_default()
}

/// C# `string.ToUpper()` under the invariant culture: simple (one-to-one) case mapping, so
/// `ß` stays `ß` where Rust's `to_uppercase` would give `SS`.
pub(crate) fn to_upper_invariant(s: &str) -> String {
    s.chars()
        .map(|c| {
            let mut up = c.to_uppercase();
            match (up.next(), up.next()) {
                (Some(u), None) => u,
                _ => c,
            }
        })
        .collect()
}

/// The first (or last) part of `p` that shows anything, looking into the parts that hold other
/// parts.
fn edge(p: &Part, last: bool) -> Option<&Part> {
    let inner: &[Part] = match p {
        Part::Null => return None,
        Part::Phrase(parts) => parts,
        Part::Message(m) => &m.parts,
        _ => return Some(p),
    };
    if last { inner.iter().rev().find_map(|q| edge(q, true)) } else { inner.iter().find_map(|q| edge(q, false)) }
}

/// Whether the client draws the edge of `p` as a token of its own: anything but text, which
/// carries its own spaces.
fn is_token(p: &Part, last: bool) -> bool {
    edge(p, last).is_some_and(|e| !matches!(e, Part::Text(_)))
}

/// Whether `before` followed by `after` would read as one word.
fn runs_together(before: &str, after: &str) -> bool {
    let end = before.chars().next_back().is_some_and(char::is_alphanumeric);
    let start = after.chars().next().is_some_and(char::is_alphanumeric);
    end && start
}

impl Skin {
    pub const LATEST_VERSION: i32 = 151;

    // #region Descriptions

    /// A message's text: each part described, one after the other.
    pub fn render(&self, m: &Message) -> String {
        self.render_parts(&m.parts)
    }

    /// The text of these parts, each described, one after the other. Two neighbouring parts that
    /// the client draws as separate tokens (a number, a faction, a force icon, ...) get a space
    /// between them where their texts would otherwise run together, as in "3 forces" or
    /// "Emperor 3 forces". The client draws its own gap between such tokens, so this only
    /// changes plain text.
    pub fn render_parts(&self, parts: &[Part]) -> String {
        let mut out = String::new();
        let mut previous: Option<(&Part, String)> = None;
        for p in parts {
            let text = self.describe(p);
            if let Some((before, before_text)) = &previous {
                if is_token(before, true) && is_token(p, false) && runs_together(before_text, &text) {
                    out.push(' ');
                }
            }
            out.push_str(&text);
            previous = Some((p, text));
        }
        out
    }

    /// The described items joined with `", "`, or `"-"` when that is empty.
    pub fn join(&self, items: &[Part]) -> String {
        let result = items.iter().map(|i| self.describe(i)).collect::<Vec<_>>().join(", ");
        if result.is_empty() {
            return "-".to_string();
        }
        result
    }

    pub fn get_center(&self, location: Location, map: &Map) -> PointD {
        if let Some(hms) = location.as_hidden_mobile_stronghold() {
            if let Some(attached) = hms.attached_to_location(map) {
                let rock = attached.territory() == map.rock_outcroppings();
                let d_x = if rock { 22 } else { 0 };
                let d_y = if rock { -10 } else { 0 };
                let attached_to_center = self.get_center(attached, map);
                return PointD::new(
                    attached_to_center.x + self.hms_dx() as i32 as f64 + d_x as f64,
                    attached_to_center.y + d_y as f64,
                );
            }
            return PointD::new(self.hms_dx() as i32 as f64, self.hms_dx() as i32 as f64);
        }

        if let Some(ds) = location.as_discovered_location() {
            if let Some(attached) = ds.attached_to_location(map) {
                let attached_to_center = self.get_center(attached, map);
                let offset = (1.5f32 * self.forcetoken_radius as f32) as f64;
                return PointD::new(attached_to_center.x - offset, attached_to_center.y - offset);
            }
            return PointD::new((-4 * self.forcetoken_radius) as f64, (-4 * self.forcetoken_radius) as f64);
        }

        *self.location_center_point.index(location.id())
    }

    /// `HmsDx => -3 * PlayerTokenRadius` (a `float`)
    pub fn hms_dx(&self) -> f32 {
        (-3 * self.player_token_radius) as f32
    }

    pub fn hms_radius(&self) -> f32 {
        1.5f32 * self.player_token_radius as f32
    }

    pub fn get_spice_location(&self, location: Location) -> PointD {
        if location.spice_blow_amount() != 0 { *self.location_spice_point.index(location.id()) } else { PointD::new(0.0, 0.0) }
    }

    /// The text of one part.
    pub fn describe(&self, part: &Part) -> String {
        match part {
            Part::Null => String::new(),
            Part::Text(s) => s.to_string(),
            Part::Bool(b) => if *b { "Yes" } else { "No" }.to_string(),
            Part::Number(i) => i.to_string(),
            // Printed as .NET prints them, under the invariant culture.
            Part::Strength(h) => h.to_string(),
            Part::Payment { amount, .. } => self.render_parts(&parts![*amount, " ", Concept::Resource]),
            Part::Location(l) => self.describe_location(*l),
            Part::Territory(t) => self.describe_territory(*t),
            Part::Leader(l) => self.describe_hero(Some(Hero::Leader(*l))),
            Part::Messiah => self.describe_hero(Some(Hero::Messiah)),
            Part::Card(c) => self.describe_treachery_card(*c),
            Part::SpiceCard(c) => self.describe_resource_card(Some(*c)),
            Part::List(items) => self.join(items),
            Part::Phrase(parts) => self.render_parts(parts),
            Part::Message(m) => self.render(m),
            Part::Error(e) => e.to_string(),
            Part::Concept(v) => self.describe_concept(*v),
            Part::Faction(v) => self.describe_faction(*v),
            Part::Ambassador(v) => self.describe_ambassador(*v),
            Part::PrescienceAspect(v) => self.describe_prescience_aspect(*v),
            Part::FactionForce(v) => self.describe_faction_force(*v),
            Part::FactionSpecialForce(v) => self.describe_faction_special_force(*v),
            Part::FactionAdvantage(v) => self.describe_faction_advantage(*v),
            Part::TreacheryCardType(v) => self.describe_treachery_card_type(*v),
            Part::LeaderSkill(v) => self.describe_leader_skill(*v),
            Part::TerrorType(v) => self.describe_terror_type(*v),
            Part::DiscoveryToken(v) => self.describe_discovery_token(*v),
            Part::DiscoveryTokenType(v) => self.describe_discovery_token_type(*v),
            Part::Ruleset(v) => describe_ruleset(*v),
            Part::AutomationRuleType(v) => self.describe_automation_rule_type(*v),
            Part::RuleGroup(v) => describe_rule_group(*v),
            Part::Rule(v) => self.describe_rule(*v),
            Part::MainPhase(v) => self.describe_main_phase(*v),
            Part::TechToken(v) => self.describe_tech_token(*v),
            Part::WinMethod(v) => self.describe_win_method(*v),
            Part::Phase(v) => describe_phase(*v),
            Part::ChoamEconomicsStatus(v) => describe_choam_economics_status(*v),
            Part::AuctionType(v) => describe_auction_type(*v),
            Part::JuiceType(v) => describe_juice_type(*v),
            Part::CaptureDecision(v) => describe_capture_decision(*v),
            Part::StrongholdAdvantage(v) => self.describe_stronghold_advantage(*v),
            Part::ClairVoyanceAnswer(v) => describe_clair_voyance_answer(*v),
            // Any other enum: its member name.
            other => match other.enum_value() {
                Some((info, value)) => info.format(value),
                None => unreachable!("every part that isn't an enum is described above"),
            },
        }
    }

    pub fn describe_territory(&self, t: Territory) -> String {
        get_label(&self.territory_name_str, t.skin_id())
    }

    pub fn describe_hero(&self, hero: Option<Hero>) -> String {
        match hero {
            None => "?".to_string(),
            Some(Hero::Leader(l)) => get_label(&self.person_name_str, l.skin_id()),
            Some(Hero::Messiah) => self.describe_concept(Some(Concept::Messiah)),
            Some(Hero::TreacheryCard(tc)) => self.describe_treachery_card(tc),
        }
    }

    pub fn describe_location(&self, l: Location) -> String {
        if !l.orientation().is_empty() {
            return self.render_parts(&parts![l.territory(), " (", l.orientation(), " Sector)"]);
        }
        self.describe_territory(l.territory())
    }

    pub fn describe_faction(&self, f: Option<Faction>) -> String {
        get_label(&self.faction_name_str, f)
    }

    pub fn describe_concept(&self, c: Option<Concept>) -> String {
        get_label(&self.concept_str, c)
    }

    pub fn describe_treachery_card_type(&self, t: Option<TreacheryCardType>) -> String {
        get_label(&self.treachery_card_type_str, t)
    }

    pub fn describe_resource_card(&self, c: Option<ResourceCard>) -> String {
        let Some(c) = c else { return "?".to_string() };
        if c.is_shai_hulud() {
            return self.describe_concept(Some(Concept::Monster));
        }
        if c.is_great_maker() {
            return self.describe_concept(Some(Concept::GreatMonster));
        }
        if c.is_sand_trout() {
            return self.describe_concept(Some(Concept::BabyMonster));
        }
        self.describe_territory(c.location().expect("location").territory())
    }

    pub fn describe_treachery_card(&self, c: TreacheryCard) -> String {
        get_label(&self.treachery_card_name_str, c.skin_id())
    }

    pub fn describe_prescience_aspect(&self, aspect: Option<PrescienceAspect>) -> String {
        match aspect {
            Some(PrescienceAspect::Dial) => "Dial",
            Some(PrescienceAspect::Leader) => "Leader",
            Some(PrescienceAspect::Weapon) => "Weapon",
            Some(PrescienceAspect::Defense) => "Defense",
            None => "",
        }
        .to_string()
    }

    pub fn describe_leader_skill(&self, l: Option<LeaderSkill>) -> String {
        get_label(&self.leader_skill_card_name_str, l)
    }

    pub fn describe_discovery_token(&self, dt: Option<DiscoveryToken>) -> String {
        get_label(&self.discovery_token_name_str, dt)
    }

    pub fn get_discovery_token_description(&self, dt: DiscoveryToken) -> String {
        get_label(&self.discovery_token_description_str, Some(dt))
    }

    /// `SpiceHtml(object amount, int size = 18)`: a spice amount drawn on the harvester icon.
    pub fn spice_html(&self, amount: &str, size: i32) -> String {
        let name = self.describe_concept(Some(Concept::Resource));
        format!(
            "<span class='spice-amt' style='--s:{size}px' title='{name}'><img class='spice-amt__img' src='{}' alt='{name}'/><b class='spice-amt__value'>{amount}</b></span>",
            self.harvester_url
        )
    }

    /// `WithSpiceIcons(string text, int size = 18, bool encode = true)`: the text as HTML, with
    /// each amount of spice in it drawn by [`Skin::spice_html`].
    pub fn with_spice_icons(&self, text: &str, size: i32, encode: bool) -> String {
        if text.is_empty() {
            return String::new();
        }
        let text = if encode { crate::text::html_encode(text) } else { text.to_string() };
        let name = self.describe_concept(Some(Concept::Resource));
        crate::text::replace_spice_amounts(&text, &name, |digits| self.spice_html(digits, size))
    }

    fn get_token_popup(&self, image_url: Option<&str>, name: &str, rules_text: &str) -> String {
        let art = match image_url {
            None | Some("") => String::new(),
            Some(url) => format!("<div class='pop-token__art'><img class='pop-token__img' src='{url}'/></div>"),
        };
        format!(
            "<div class='pop-token'>{art}<div class='pop-token__body'><h6 class='pop-token__title'>{name}</h6><p class='pop-token__text'>{}</p></div></div>",
            self.with_spice_icons(rules_text, 18, true)
        )
    }

    pub fn get_popup_terror_type(&self, t: TerrorType) -> String {
        self.get_token_popup(None, &self.describe_terror_type(Some(t)), &self.get_terror_type_description(t))
    }

    pub fn get_popup_ambassador(&self, a: Ambassador) -> String {
        let name = self.describe_ambassador(Some(a));
        let mut text = self.get_ambassador_description(a);
        if let Some(dash) = text.find(" - ") {
            text = text[dash + 3..].to_string();
        }
        self.get_token_popup(Some(&self.get_image_url_ambassador(Some(a))), &format!("{name} Ambassador"), &text)
    }

    pub fn get_popup_discovery_token(&self, dt: DiscoveryToken) -> String {
        self.get_token_popup(
            Some(&self.get_image_url_discovery_token(dt)),
            &self.describe_discovery_token(Some(dt)),
            &self.get_discovery_token_description(dt),
        )
    }

    pub fn get_popup_leader_skill(&self, c: LeaderSkill) -> String {
        get_framed_card_html(&self.get_image_url_leader_skill(Some(c)))
    }

    pub fn get_popup_tech_token(&self, tt: TechToken) -> String {
        self.get_token_popup(
            Some(&self.get_image_url_tech_token(Some(tt))),
            &self.describe_tech_token(Some(tt)),
            &get_label(&self.tech_token_description_str, Some(tt)),
        )
    }

    pub fn get_nexus_card_popup(&self, n: Nexus) -> String {
        get_framed_card_html(&self.get_image_url_nexus(Some(n)))
    }

    pub fn get_popup_homeworld(&self, w: Homeworld) -> String {
        get_framed_card_html(&self.get_homeworld_card_image_url(w.world()))
    }

    pub fn get_popup_treachery_card(&self, c: Option<TreacheryCard>) -> String {
        let Some(c) = c else { return String::new() };
        let (category, type_) = get_treachery_card_category(c);
        format!(
            "<div class='pop-card'><img class='pop-card__art' src='{}'/><div class='pop-card__body'><h6 class='pop-card__title'>{}</h6><div class='pop-card__type'><i class='pop-card__swatch pop-card__swatch--{category}'></i>{type_}</div><p class='pop-card__text'>{}</p></div></div>",
            self.get_image_url_treachery_card(Some(c)),
            self.describe_treachery_card(c),
            self.with_spice_icons(&self.get_treachery_card_rules_text(c), 18, true)
        )
    }

    fn get_treachery_card_rules_text(&self, c: TreacheryCard) -> String {
        let text = self.get_treachery_card_description(Some(c));

        for prefix in ["Weapon - ", "Defense - "] {
            if !starts_with_culture(&text, prefix) {
                continue;
            }
            return match text[prefix.len()..].find(" - ") {
                Some(end) => text[prefix.len() + end + 3..].to_string(),
                None => text,
            };
        }

        for prefix in ["Worthless Card - ".to_string(), self.describe_treachery_card(c) + " - "] {
            if starts_with_culture(&text, &prefix) {
                return text[prefix.len()..].to_string();
            }
        }

        text
    }

    pub fn get_popup_hero(&self, h: Hero) -> String {
        if let Some(tc) = h.as_treachery_card() {
            return self.get_popup_treachery_card(Some(tc));
        }
        if let Some(l) = h.as_leader() {
            return self.get_leader_popup(l);
        }
        self.get_messiah_popup()
    }

    /// The leader popover's frame for the Messiah, with its paragraph of the Atreides sheet
    /// (`faction_manual.rs`) under the name, one sentence a line and its nouns drawn as icons.
    /// `messiahPopup` in web/src/popups.ts is the same.
    fn get_messiah_popup(&self) -> String {
        let image = &self.messiah_url;
        let name = self.describe_concept(Some(Concept::Messiah));
        let color = self.get_faction_color(Faction::Atreides);
        let chip = self.faction_chip_html(Faction::Atreides, false, None);
        let kh = format!("<img class='kh' src='{image}' alt='{name}' title='{name}'/>");
        let card = |t: TreacheryCardType| {
            let first = TreacheryCardManager::items().iter().copied().find(|c| c.type_() == t);
            format!(
                "<span class='card-chip'><img class='card-chip__img' src='{}' alt=''/>{}</span>",
                self.get_image_url_treachery_card(first),
                self.describe_treachery_card_type(Some(t))
            )
        };
        let cheap_hero = card(TreacheryCardType::Mercenary);
        let forces = format!(
            "<span class='pop-leader__forces'>7<img class='pop-leader__force' src='{}' alt='forces' title='forces'/></span>",
            self.get_faction_force_image_url(Faction::Atreides)
        );
        let rules = [
            format!("After losing a total of at least {forces} in battle, you may use {kh}."),
            format!("It cannot be used alone in battle but may add its <span class='pop-leader__plus'>+2</span> strength to any one leader or {cheap_hero} per turn."),
            format!("If the leader or {cheap_hero} is killed, {kh} has no effect in the battle."),
            format!("A leader accompanied by {kh} <b class='pop-leader__em'>cannot turn traitor</b>."),
            format!(
                "{kh} can only be killed if blown up by a <span class='pop-leader__nowrap'>{}/{}</span> explosion.",
                card(TreacheryCardType::Laser),
                card(TreacheryCardType::Shield)
            ),
            format!("If killed, {kh} must be revived like any other leader."),
            format!("{kh} has no effect on {chip} leader revival."),
        ];
        let rules: String = rules.iter().map(|r| format!("<p class='pop-leader__line'>{r}</p>")).collect();
        format!(
            "<div class='pop-leader pop-leader--messiah'><div class='pop-leader__art' style='--fc:{color}'><img class='pop-leader__img' src='{image}'/></div><div class='pop-leader__body'><h6 class='pop-leader__title'>{name}</h6><div class='pop-leader__head'>{chip}<span class='pop-leader__plus pop-leader__plus--head'>+2</span></div><div class='pop-leader__rules'>{rules}</div></div></div>"
        )
    }

    fn get_leader_popup(&self, l: Leader) -> String {
        let color = self.get_faction_color(l.faction());
        format!(
            "<div class='pop-leader'><div class='pop-leader__art' style='--fc:{color}'><img class='pop-leader__img' src='{}'/></div><div class='pop-leader__body'><h6 class='pop-leader__title'>{}</h6>{}</div></div>",
            self.get_image_url_hero(Some(Hero::Leader(l))),
            self.describe_hero(Some(Hero::Leader(l))),
            self.faction_chip_html(l.faction(), false, None)
        )
    }

    /// `FactionChipHtml(Faction f, bool you = false, IEnumerable<int>? locationIds = null)`: the
    /// markup FactionChip.razor renders.
    pub fn faction_chip_html(&self, f: Faction, you: bool, location_ids: Option<&[i32]>) -> String {
        let name = crate::text::html_encode(&self.describe_faction(Some(f)));
        let locations = match location_ids {
            Some(ids) => ids.iter().map(|i| i.to_string()).collect::<Vec<_>>().join(","),
            None => String::new(),
        };
        format!(
            "<span class='chip chip--faction{}' data-ref-faction='{f}' data-ref-locations='{locations}' style='--fc:{}' title='{name}'><img class='chip__img' src='{}' alt=''/><span class='chip__name'>{name}</span></span>",
            if you { " chip--you" } else { "" },
            self.get_faction_color(f),
            self.get_image_url_faction(f)
        )
    }

    pub fn get_popup_hero_in_game(&self, h: Option<Hero>, g: &Game) -> String {
        let Some(h) = h else { return String::new() };
        if let Some(tc) = h.as_treachery_card() {
            return self.get_popup_treachery_card(Some(tc));
        }

        let skill = g.skill(Some(h));

        match skill {
            None => self.get_popup_hero(h),
            Some(skill) => self.get_popup_skilled_leader(h.as_leader(), skill),
        }
    }

    pub fn get_popup_skilled_leader(&self, l: Option<Leader>, s: LeaderSkill) -> String {
        match l {
            None => String::new(),
            Some(l) => format!(
                "<div style='position:relative'><img style='position:relative;filter:drop-shadow(-3px 3px 2px black);' src='{}' width=300/><img src='{}' width=140 style='position:absolute;left:200px;top:120px;filter:drop-shadow(-2px 2px 2px black);'/></div>",
                self.get_image_url_leader_skill(Some(s)),
                self.get_image_url_hero(Some(Hero::Leader(l)))
            ),
        }
    }

    pub fn get_popup_stronghold_advantage(&self, adv: StrongholdAdvantage) -> String {
        get_image_hover_html(&self.get_image_url_stronghold_advantage(Some(adv)))
    }

    pub fn get_popup_stronghold_advantage_for(&self, adv: StrongholdAdvantage, f: Faction) -> String {
        format!(
            "<div style='position:relative'><img style='position:relative;filter:drop-shadow(-3px 3px 2px black);' src='{}' width=300/><img src='{}' width=100 style='position:absolute;left:220px;top:40px;filter:drop-shadow(-3px 3px 2px black);'/></div>",
            self.get_image_url_stronghold_advantage(Some(adv)),
            self.get_image_url_faction(f)
        )
    }

    /// The image of a card, hero or enum value, or `""` for any other part.
    pub fn get_image_url(&self, part: &Part) -> String {
        match part {
            Part::Card(tc) => self.get_image_url_treachery_card(Some(*tc)),
            Part::SpiceCard(rc) => self.get_image_url_resource_card(Some(*rc)),
            Part::Leader(l) => self.get_image_url_hero(Some(Hero::Leader(*l))),
            Part::Messiah => self.get_image_url_hero(Some(Hero::Messiah)),
            Part::LeaderSkill(v) => self.get_image_url_leader_skill(*v),
            Part::TechToken(v) => self.get_image_url_tech_token(*v),
            Part::Faction(Some(v)) => self.get_image_url_faction(*v),
            Part::Nexus(v) => self.get_image_url_nexus(*v),
            Part::Ambassador(v) => self.get_image_url_ambassador(*v),
            Part::FactionForce(v) => self.get_image_url_faction_force(*v),
            Part::FactionSpecialForce(v) => self.get_image_url_faction_special_force(*v),
            Part::StrongholdAdvantage(v) => self.get_image_url_stronghold_advantage(*v),
            _ => String::new(),
        }
    }

    pub fn get_image_url_discovery_token_type(&self, dtt: DiscoveryTokenType) -> String {
        get_label(&self.discovery_token_type_image_url, Some(dtt))
    }

    pub fn get_image_url_discovery_token(&self, dt: DiscoveryToken) -> String {
        get_label(&self.discovery_token_image_url, Some(dt))
    }

    pub fn describe_discovery_token_type(&self, dtt: Option<DiscoveryTokenType>) -> String {
        get_label(&self.discovery_token_type_name_str, dtt)
    }

    pub fn describe_terror_type(&self, terr: Option<TerrorType>) -> String {
        get_label(&self.terror_token_name_str, terr)
    }

    pub fn get_terror_type_description(&self, terr: TerrorType) -> String {
        get_label(&self.terror_token_description_str, Some(terr))
    }

    pub fn describe_ambassador(&self, ambassador: Option<Ambassador>) -> String {
        get_label(&self.ambassador_name_str, ambassador)
    }

    pub fn get_ambassador_description(&self, ambassador: Ambassador) -> String {
        get_label(&self.ambassador_description_str, Some(ambassador))
    }

    pub fn get_image_url_ambassador(&self, ambassador: Option<Ambassador>) -> String {
        get_label(&self.ambassador_image_url, ambassador)
    }

    pub fn describe_main_phase(&self, p: Option<MainPhase>) -> String {
        get_label(&self.main_phase_str, p)
    }

    pub fn describe_tech_token(&self, tt: Option<TechToken>) -> String {
        get_label(&self.tech_token_name_str, tt)
    }

    pub fn describe_faction_force(&self, f: Option<FactionForce>) -> String {
        get_label(&self.force_name_str, get_faction_faction_force(f))
    }

    pub fn describe_faction_special_force(&self, f: Option<FactionSpecialForce>) -> String {
        get_label(&self.special_force_name_str, get_faction_faction_special_force(f))
    }

    pub fn describe_faction_advantage(&self, advantage: Option<FactionAdvantage>) -> String {
        use FactionAdvantage as A;
        use FactionSpecialForce as Fsf;
        match advantage {
            None => "- Any stoppable advantage not listed below -".to_string(),
            Some(A::AtreidesBiddingPrescience) => self.render_parts(&parts![Faction::Atreides, " seeing the card on auction"]),
            Some(A::AtreidesSpiceBlowPrescience) => {
                let resource = self.concept_str.index(Some(Concept::Resource)).clone();
                self.render_parts(&parts![Faction::Atreides, " seeing the top ", resource, " card this turn"])
            }
            Some(A::AtreidesBattlePlanPrescience) => self.render_parts(&parts![Faction::Atreides, " seeing part of the opponent battle plan"]),
            Some(A::AtreidesUseMessiah) => {
                let messiah = self.concept_str.index(Some(Concept::Messiah)).clone();
                self.render_parts(&parts![Faction::Atreides, " using the ", messiah, " in this battle"])
            }
            Some(A::HarkonnenFreeCard) => self.render_parts(&parts![Faction::Harkonnen, " taking a free treachery card"]),
            Some(A::HarkonnenCaptureLeader) => self.render_parts(&parts![Faction::Harkonnen, " capturing a leader after this battle"]),
            Some(A::HarkonnenCallTraitorForAlly) => self.render_parts(&parts![Faction::Harkonnen, " calling TREACHERY for their ally in this battle"]),
            Some(A::BeneGesseritAccompanies) => self.render_parts(&parts![Faction::BeneGesserit, " accompanying shipment"]),
            Some(A::BeneGesseritAnnouncesBattle) => self.render_parts(&parts![Faction::BeneGesserit, " flipping advisors to fighters*"]),
            Some(A::BeneGesseritIntrusion) => self.render_parts(&parts![Faction::BeneGesserit, " becoming advisors on intrusion"]),
            Some(A::BeneGesseritUsingVoice) => self.render_parts(&parts![Faction::BeneGesserit, " using Voice during this battle"]),
            Some(A::BeneGesseritWorthlessAsKarma) => self.render_parts(&parts![Faction::BeneGesserit, " using a ", TreacheryCardType::Useless, " as a ", TreacheryCardType::Karma, " card*"]),
            Some(A::BeneGesseritCharity) => {
                self.render_parts(&parts![Faction::BeneGesserit, " receiving 2 ", Concept::Resource, " at ", MainPhase::Charity])
            }
            Some(A::FremenControlsMonster) => self.render_parts(&parts![Faction::Fremen, " sending ", Concept::Monster, " when a subsequent ", Concept::Monster, " card is drawn"]),
            Some(A::FremenRidesMonster) => self.render_parts(&parts![Faction::Fremen, " riding ", Concept::Monster]),
            Some(A::FremenNotPayingForBattles) => self.render_parts(&parts![Faction::Fremen, " not paying for forces in this battle"]),
            Some(A::FremenSpecialForceBonus) => {
                self.render_parts(&parts![Faction::Fremen, " counting ", Fsf::Fremen, " bonus in this battle"])
            }
            Some(A::FremenExtraMove) => {
                self.render_parts(&parts![Faction::Fremen, " moving two territories as part of their move action"])
            }
            Some(A::FremenProtectedFromStorm) => self.render_parts(&parts![Faction::Fremen, " not taking storm losses when hit"]),
            Some(A::FremenProtectedFromMonster) => {
                self.render_parts(&parts![Faction::Fremen, " not being devoured by ", Concept::Monster])
            }
            Some(A::FremenProtectedFromMonsterAlly) => {
                self.render_parts(&parts![Faction::Fremen, " ally not being devoured by ", Concept::Monster])
            }
            Some(A::FremenStormPrescience) => self.render_parts(&parts![Faction::Fremen, " seeing the storm movement this turn"]),
            Some(A::EmperorSpecialForceBonus) => self.render_parts(&parts![Faction::Emperor, " counting ", Fsf::Emperor, " bonus in this battle"]),
            Some(A::EmperorReceiveBid) => {
                self.render_parts(&parts![Faction::Emperor, " receiving ", Concept::Resource, " for a treachery card"])
            }
            Some(A::EmperorGiveSpiceToAlly) => self.render_parts(&parts![Faction::Emperor, " giving ", Concept::Resource, " to their ally*"]),
            Some(A::EmperorLetAllyReviveExtraForces) => self.render_parts(&parts![Faction::Emperor, " allowing their ally to revive 3 extra forces"]),
            Some(A::GuildDetermineMoveMoment) => self.render_parts(&parts![Faction::Guild, " shipping out of turn order (play before their normal turn)"]),
            Some(A::GuildSpecialShipments) => {
                self.render_parts(&parts![Faction::Guild, " (and ally) shipping site-to-site or back to reserves"])
            }
            Some(A::GuildShipmentsDiscount) => self.render_parts(&parts![Faction::Guild, " shipping at half price"]),
            Some(A::GuildShipmentsDiscountAlly) => self.render_parts(&parts![Faction::Guild, " ally shipping at half price"]),
            Some(A::GuildReceiveShipment) => {
                self.render_parts(&parts![Faction::Guild, " receiving ", Concept::Resource, " for a shipment"])
            }
            Some(A::TleilaxuRevivalDiscount) => self.render_parts(&parts![Faction::Tleilaxu, " reviving at half price"]),
            Some(A::TleilaxuRevivalDiscountAlly) => self.render_parts(&parts![Faction::Tleilaxu, " ally reviving at half price"]),
            Some(A::TleilaxuReplacingFaceDancer) => {
                self.render_parts(&parts![Faction::Tleilaxu, " replacing a face dancer at end of turn"])
            }
            Some(A::TleilaxuIncreasingRevivalLimits) => self.render_parts(&parts![Faction::Tleilaxu, " increasing revival limits"]),
            Some(A::TleilaxuReceiveRevive) => {
                self.render_parts(&parts![Faction::Tleilaxu, " receiving ", Concept::Resource, " for revival"])
            }
            Some(A::TleilaxuEarlyLeaderRevive) => {
                self.render_parts(&parts![Faction::Tleilaxu, " allowing early revival of a leader*"])
            }
            Some(A::TleilaxuReviveGhola) => self.render_parts(&parts![Faction::Tleilaxu, " reviving a leader as a Ghola"]),
            Some(A::IxianMovingHms) => self.render_parts(&parts![Faction::Ixian, " moving the Hidden Mobile Stronghold"]),
            Some(A::IxianSpecialForceBonus) => self.render_parts(&parts![Faction::Ixian, " counting ", Fsf::Ixian, " bonus in this battle"]),
            Some(A::IxianSelectingCardsOnAuction) => self.render_parts(&parts![Faction::Ixian, " selecting a card to go on top or bottom (play before they draw cards on auction)"]),
            Some(A::IxianCyborgExtraMove) => self.render_parts(&parts![Faction::Ixian, " moving ", Fsf::Ixian, " two territories"]),
            Some(A::IxianReplacingSpecialForces) => self.render_parts(&parts![Faction::Ixian, " replacing ", Fsf::Ixian, " lost in battle with ", FactionForce::Ixian]),
            Some(A::IxianAllyDiscardingCard) => {
                self.render_parts(&parts![Faction::Ixian, " allowing their ally to replace a won card"])
            }
            Some(A::IxianSwappingCard) => self.render_parts(&parts![Faction::Ixian, " replacing a treachery card during bidding"]),
            Some(A::ChoamControllingCharity) => self.render_parts(&parts![Faction::Choam, " receiving and giving ", Concept::Resource, " during ", MainPhase::Charity]),
            Some(A::ChoamDiscarding) => self.render_parts(&parts![Faction::Choam, " discarding cards for ", Concept::Resource, " or a ", TreacheryCardType::Useless, " card for its special effect*"]),
            Some(A::ChoamRevival) => self.render_parts(&parts![Faction::Choam, " having unlimited force revival and reduced revival cost"]),
            Some(A::ChoamEconomics) => self.render_parts(&parts![Faction::Choam, " playing their Inflation token during ", MainPhase::Contemplate]),
            Some(A::ChoamReceiveForcePayment) => self.render_parts(&parts![Faction::Choam, " collecting ", Concept::Resource, " payment for forces this battle"]),
            Some(A::ChoamAudit) => self.render_parts(&parts![Faction::Choam, " auditing their opponent after this battle"]),
            Some(A::RicheseAuction) => self.render_parts(&parts![Faction::Richese, " auctioning a card from their card cache"]),
            Some(A::RicheseNofield) => self.render_parts(&parts![Faction::Richese, " using a No-Field to ship"]),
            Some(A::RicheseBlackMarket) => self.render_parts(&parts![Faction::Richese, " selling a card from their hand"]),

            _ => "Unknown".to_string(),
        }
    }

    pub fn describe_automation_rule_type(&self, s: AutomationRuleType) -> String {
        use AutomationRuleType as A;
        match s {
            A::CharityAutoClaim => "Claim charity if possible".to_string(),
            A::BiddingPassWhenAtreidesOrAtreidesAllyPassed => {
                format!("Pass if most recent bid by {} or their ally passed", self.describe_faction(Some(Faction::Atreides)))
            }
            A::BiddingPassAboveAmount => "Pass if current bid is equal to or higher than...".to_string(),
            A::BiddingPassWhenHighestBidByFaction => "Pass if current highest bid by...".to_string(),
            A::RevivalAutoClaimFreeRevival => "Auto claim free revival".to_string(),
            A::ShipmentGuildAutoDelay => "Auto delay shipment until last".to_string(),
            _ => "unknown automation rule".to_string(),
        }
    }

    pub fn describe_win_method(&self, m: Option<WinMethod>) -> String {
        match m {
            Some(WinMethod::Strongholds) => "by number of victory points".to_string(),
            Some(WinMethod::Prediction) => "by prediction".to_string(),
            Some(WinMethod::Timeout) => "by running out of time".to_string(),
            Some(WinMethod::Forfeit) => "by forfeit".to_string(),
            Some(WinMethod::FremenSpecial) => self.render_parts(&parts!["by ", Faction::Fremen, " special victory"]),
            Some(WinMethod::GuildSpecial) => self.render_parts(&parts!["by ", Faction::Guild, " special victory"]),
            _ => "None".to_string(),
        }
    }

    pub fn describe_detailed(&self, t: Option<AuctionType>) -> String {
        match t {
            Some(AuctionType::Normal) => "Normal".to_string(),
            Some(AuctionType::BlackMarketNormal) => "Black Market / Normal".to_string(),
            Some(AuctionType::BlackMarketSilent) => "Black Market / Silent".to_string(),
            Some(AuctionType::BlackMarketOnceAround) => "Black Market / Once Around".to_string(),
            Some(AuctionType::RicheseSilent) => self.render_parts(&parts![Faction::Richese, " / Silent"]),
            Some(AuctionType::RicheseOnceAround) => self.render_parts(&parts![Faction::Richese, " / Once Around"]),
            None => "None".to_string(),
        }
    }

    /// `private string Describe(StrongholdAdvantage sa)`. The labels come from the dictionary's
    /// indexer, which throws KeyNotFoundException for a skin without them.
    pub fn describe_stronghold_advantage(&self, sa: Option<StrongholdAdvantage>) -> String {
        use StrongholdAdvantage as S;
        match sa {
            Some(S::CollectResourcesForDial) => self.stronghold_card_name_str.index(5).clone(),
            Some(S::CollectResourcesForUseless) => self.stronghold_card_name_str.index(4).clone(),
            Some(S::CountDefensesAsAntidote) => self.stronghold_card_name_str.index(2).clone(),
            Some(S::FreeResourcesForBattles) => self.stronghold_card_name_str.index(3).clone(),
            Some(S::WinTies) => self.stronghold_card_name_str.index(6).clone(),
            _ => "None".to_string(),
        }
    }

    pub fn describe_rule(&self, r: Option<Rule>) -> String {
        let Some(r) = r else { return "unknown rule".to_string() };
        use FactionSpecialForce as Fsf;
        use TreacheryCardType as Tct;
        let s = |t: &str| t.to_string();
        match r {
            Rule::BasicTreacheryCards => s("Basic Treachery Cards"),
            Rule::HasCharityPhase => s("Charity Phase"),

            Rule::AdvancedCombat => s("Advanced Combat"),
            Rule::IncreasedResourceFlow => s("Increased Spice Flow"),
            Rule::AdvancedKarama => s("Advanced Karama Cards"),
            Rule::FremenDeterminesStorm => {
                self.render_parts(&parts![Faction::Fremen, " determine storm movement with the Storm Deck"])
            }
            Rule::FremenStormLosses => self.render_parts(&parts![Faction::Fremen, " storm losses are halved"]),
            Rule::FremenSendingMonster => self.render_parts(&parts![Faction::Fremen, " place additional ", Concept::Monster, "s"]),
            Rule::FremenSpecialForces => self.render_parts(&parts![Faction::Fremen, " can use ", Fsf::Fremen]),
            Rule::AtreidesMessiah => self.render_parts(&parts!["Enable ", Concept::Messiah]),
            Rule::HarkonnenCapturesOrKillsLeaders => self.render_parts(&parts![Faction::Harkonnen, " can kill or capture leaders"]),
            Rule::BeneGesseritFirstForceInAnyTerritory => {
                self.render_parts(&parts![Faction::BeneGesserit, " can put their initial force anywhere"])
            }
            Rule::BeneGesseritAutoCharity => self.render_parts(&parts![Faction::BeneGesserit, " get 2 charity each turn"]),
            Rule::BeneGesseritWorthlessAsKarma => {
                self.render_parts(&parts![Faction::BeneGesserit, " can use ", Tct::Useless, " cards for ", Tct::Karma])
            }
            Rule::BeneGesseritAdvisors => self.render_parts(&parts![Faction::BeneGesserit, " can use ", Fsf::BeneGesserit]),
            Rule::BeneGesseritAccompaniesToShipmentLocation => {
                self.render_parts(&parts![Faction::BeneGesserit, " can accompany to the shipped to territory"])
            }
            Rule::GuildDetermineShipment => {
                self.render_parts(&parts![Faction::Guild, " can determine when they ship and move"])
            }
            Rule::EmperorSpecialForces => self.render_parts(&parts![Faction::Emperor, " can use ", Fsf::Emperor]),
            Rule::BribesAreImmediate => {
                self.render_parts(&parts![Concept::Resource, " bribes are immediately added to reserves"])
            }
            Rule::ContestedStongholdsCountAsOccupied => s("Contested strongholds count as occupied @ end of turn"),
            Rule::AdvisorsDontConflictWithAlly => {
                self.render_parts(&parts![Faction::BeneGesserit, " ", Fsf::BeneGesserit, " can coexist with allied forces"])
            }
            Rule::GuildShipmentContributionsFlowBack => {
                self.render_parts(&parts![Faction::Guild, " contributions to shipments flow back to them"])
            }
            Rule::CustomInitialForcesAndResources => {
                self.render_parts(&parts!["Custom initial ", Concept::Resource, " and force positions (for Spice Harvest)"])
            }
            Rule::HmSwithoutIxian => {
                self.render_parts(&parts!["Use the ", "Hidden Mobile Stronghold", " if ", Faction::Ixian, " are not in play"])
            }
            Rule::StormDeckWithoutFremen => {
                self.render_parts(&parts!["Use the Storm Deck if ", Faction::Fremen, " are not in play"])
            }

            Rule::Ssw => {
                self.render_parts(&parts!["SSW: ", "Shield Wall", " counts for victory after fourth ", Concept::Monster])
            }
            Rule::HarkonnenMulligan => {
                self.render_parts(&parts![Faction::Harkonnen, " mulligan traitors when they drew > 1 of their own"])
            }

            Rule::TechTokens => s("Tech Tokens"),
            Rule::ExpansionTreacheryCards => s("Expansion Treachery Cards"),
            Rule::ExpansionTreacheryCardsExceptPBandSSandAmal => self.render_parts(&parts!["Treachery Cards: all except ", Tct::ProjectileAndPoison, ", ", Tct::ShieldAndAntidote, " and ", Tct::Amal]),
            Rule::ExpansionTreacheryCardsPBandSs => {
                self.render_parts(&parts!["Treachery Cards: ", Tct::ProjectileAndPoison, " and ", Tct::ShieldAndAntidote])
            }
            Rule::ExpansionTreacheryCardsAmal => self.render_parts(&parts!["Treachery Card: ", Tct::Amal]),
            Rule::CheapHeroTraitor => s("Cheap Hero Traitor"),
            Rule::SandTrout => self.describe_concept(Some(Concept::BabyMonster)),
            Rule::TleilaxuGholas => self.render_parts(&parts![Faction::Tleilaxu, " may revive leaders as Gholas"]),
            Rule::IxianSwappingCardOnBid => {
                self.render_parts(&parts![Faction::Ixian, " may swap one card on bid with a card from their hand"])
            }

            Rule::LeaderSkills => s("Leader Skills"),
            Rule::Expansion2TreacheryCards => {
                self.render_parts(&parts!["Treachery Cards: ", Tct::ArtilleryStrike, " and ", Tct::PoisonTooth])
            }
            Rule::StrongholdBonus => s("Stronghold Bonus"),
            Rule::ChoamAuditor => self.render_parts(&parts![Faction::Choam, " gains the Auditor leader"]),
            Rule::RicheseBlackMarket => self.render_parts(&parts![Faction::Richese, " Black Market bidding"]),

            Rule::GreatMaker => s("Great Maker"),
            Rule::DiscoveryTokens => s("Discovery Tokens"),
            Rule::Homeworlds => s("Homeworlds"),
            Rule::NexusCards => s("Nexus Cards"),
            Rule::Expansion3TreacheryCards => s("Expansion Treachery Cards"),
            Rule::MoritaniAssassinate => self.render_parts(&parts![Faction::Moritani, " can assassinate leaders"]),
            Rule::EcazLoyalty => self.render_parts(&parts![Faction::Ecaz, " gain a loyal leader"]),

            Rule::CustomDecks => s("Customized Treachery Card Deck"),
            Rule::ExtraKaramaCards => self.render_parts(&parts!["Add three extra ", Tct::Karma, " cards to the game"]),
            Rule::FullPhaseKarma => self.render_parts(&parts!["Full phase ", Tct::Karma, " (instead of single instance)"]),
            Rule::FremenMayMoveIntoStorm => self.render_parts(&parts![Faction::Fremen, " may move into storm"]),
            Rule::BeneGesseritVoiceMustNameSpecialCards => s("Voice must target special cards by name"),
            Rule::BattlesUnderStorm => s("Battles may happen under the storm"),
            Rule::MovementBonusRequiresOccupationBeforeMovement => {
                s("Arrakeen/Carthag must be occupied before Ship&Move to grant ornithopters")
            }
            Rule::AssistedNotekeeping => s("Mentat: auto notekeeping of knowable info (spice owned, cards seen, ...)"),
            Rule::AssistedNotekeepingForAtreides => {
                self.render_parts(&parts!["Mentat: auto notekeeping for ", Faction::Atreides])
            }
            Rule::ResourceBonusForStrongholds => s("Stronghold spice bonus (even without increased spice flow)"),
            Rule::DisableNovaFlipping => s("No Nova flipping (advisors will not auto flip after shipment phase)"),

            Rule::FillWithBots => s("Fill empty seats with random Bots"),
            Rule::GuildBot => self.render_parts(&parts![Faction::Guild, "Bot"]),
            Rule::EmperorBot => self.render_parts(&parts![Faction::Emperor, "Bot"]),
            Rule::HarkonnenBot => self.render_parts(&parts![Faction::Harkonnen, "Bot"]),
            Rule::TleilaxuBot => self.render_parts(&parts![Faction::Tleilaxu, "Bot"]),
            Rule::AtreidesBot => self.render_parts(&parts![Faction::Atreides, "Bot"]),
            Rule::BeneGesseritBot => self.render_parts(&parts![Faction::BeneGesserit, "Bot"]),
            Rule::FremenBot => self.render_parts(&parts![Faction::Fremen, "Bot"]),
            Rule::IxianBot => self.render_parts(&parts![Faction::Ixian, "Bot"]),

            Rule::BotsCannotAlly => s("Bots may not initiate alliances"),
            Rule::CardsCanBeTraded => s("Allow players to give cards to each other"),
            Rule::PlayersChooseFactions => s("Let players choose their factions at start"),
            Rule::EmperorSupportingNonAllyBids => {
                self.render_parts(&parts![Faction::Emperor, " may support bids of non-ally players"])
            }
            Rule::BattleWithoutLeader => s("Allow leaderless battles even if leaders are available"),
            Rule::CapturedLeadersAreTraitorsToOwnFaction => {
                s("Captured leaders can be called as traitors by their original factions without a traitor card")
            }
            Rule::DisableEndOfGameReport => s("Disable end-of-game report (don't reveal player shields)"),
            Rule::DisableGuildSpecialVictory => {
                self.render_parts(&parts!["Disable ", Faction::Guild, " special victory condition"])
            }
            Rule::DisableResourceTransfers => {
                self.render_parts(&parts!["Only allow transfer of ", Concept::Resource, " by alliance rules"])
            }
            Rule::FremenAllyGetsDialedResourcesRefunded => self.render_parts(&parts![Faction::Fremen, " ally may get ", Concept::Resource, " dialled in battles refunded in ", MainPhase::Contemplate, " phase"]),
            Rule::BreakAlliancesOnAlliancePhase => s("Alliances are broken at the start of each alliance phase"),

            _ => s("unknown rule"),
        }
    }

    // #endregion Descriptions

    // #region NamesAndImages

    pub fn get_territory_border(&self, t: Option<Territory>) -> String {
        match t {
            Some(t) => get_label(&self.territory_border_svg, t.skin_id()),
            None => String::new(),
        }
    }

    pub fn get_image_url_treachery_card(&self, c: Option<TreacheryCard>) -> String {
        match c {
            Some(c) => get_label(&self.treachery_card_image_url, c.skin_id()),
            None => String::new(),
        }
    }

    pub fn get_image_url_world(&self, w: World) -> String {
        get_label(&self.home_world_image_url, Some(w))
    }

    pub fn get_image_url_stronghold_advantage(&self, a: Option<StrongholdAdvantage>) -> String {
        get_label(&self.stronghold_card_image_url, a)
    }

    pub fn get_image_url_resource_card(&self, c: Option<ResourceCard>) -> String {
        match c {
            Some(c) => get_label(&self.resource_card_image_url, c.skin_id()),
            None => String::new(),
        }
    }

    pub fn get_image_url_leader_skill(&self, s: Option<LeaderSkill>) -> String {
        get_label(&self.leader_skill_card_image_url, s)
    }

    pub fn get_treachery_card_description(&self, c: Option<TreacheryCard>) -> String {
        match c {
            Some(c) => get_label(&self.treachery_card_description_str, c.skin_id()),
            None => String::new(),
        }
    }

    pub fn get_image_url_hero(&self, h: Option<Hero>) -> String {
        let Some(h) = h else { return String::new() };
        match h {
            Hero::TreacheryCard(_) => get_label(&self.treachery_card_image_url, h.skin_id()),
            Hero::Messiah => self.messiah_url.clone(),
            Hero::Leader(_) => get_label(&self.person_image_url, h.skin_id()),
        }
    }

    pub fn get_image_url_faction(&self, faction: Faction) -> String {
        get_label(&self.faction_image_url, Some(faction))
    }

    pub fn get_faction_table_image_url(&self, faction: Faction) -> String {
        get_label(&self.faction_table_image_url, Some(faction))
    }

    pub fn get_faction_face_down_image_url(&self, faction: Faction) -> String {
        get_label(&self.faction_facedown_image_url, Some(faction))
    }

    pub fn get_faction_force_image_url(&self, f: Faction) -> String {
        get_label(&self.faction_force_image_url, Some(f))
    }

    pub fn get_faction_special_force_image_url(&self, f: Faction) -> String {
        get_label(&self.faction_special_force_image_url, Some(f))
    }

    pub fn get_image_url_faction_force(&self, ff: Option<FactionForce>) -> String {
        get_label(&self.faction_force_image_url, get_faction_faction_force(ff))
    }

    pub fn get_image_url_faction_special_force(&self, fsf: Option<FactionSpecialForce>) -> String {
        get_label(&self.faction_special_force_image_url, get_faction_faction_special_force(fsf))
    }

    pub fn get_homeworld_card_image_url(&self, w: World) -> String {
        get_label(&self.home_world_card_image_url, Some(w))
    }

    pub fn get_image_url_nexus(&self, n: Option<Nexus>) -> String {
        get_label(&self.nexus_card_image_url, n)
    }

    pub fn get_image_url_tech_token(&self, tech: Option<TechToken>) -> String {
        get_label(&self.tech_token_image_url, tech)
    }

    pub fn get_sound(&self, m: Milestone) -> String {
        get_label(&self.sound, Some(m))
    }

    pub fn get_faction_color(&self, faction: Faction) -> String {
        get_label(&self.faction_color, Some(faction))
    }

    pub fn get_faction_color_transparent(&self, faction: Faction, transparency: &str) -> String {
        get_label(&self.faction_color, Some(faction)) + transparency
    }

    // #endregion NamesAndImages


}

/// `string.StartsWith(string)`, which is culture-sensitive in .NET. Every prefix the skin tests
/// is ASCII text without ignorable characters, so this is ordinal.
fn starts_with_culture(text: &str, prefix: &str) -> bool {
    text.starts_with(prefix)
}

pub fn describe_clair_voyance_answer(answer: Option<ClairVoyanceAnswer>) -> String {
    match answer {
        Some(ClairVoyanceAnswer::No) => "No.",
        Some(ClairVoyanceAnswer::Yes) => "Yes.",
        Some(ClairVoyanceAnswer::Unknown) => "I don't know...",
        _ => "?",
    }
    .to_string()
}

pub fn describe_user_status(us: UserStatus) -> String {
    match us {
        UserStatus::None => "No status",
        UserStatus::Online => "Online",
        UserStatus::Away => "Away (not available)",
        UserStatus::Lfg => "Looking for game",
        UserStatus::Lfm => "Looking for others to join",
        UserStatus::InGame => "In game",
    }
    .to_string()
}

pub fn describe_subscription_type(st: SubscriptionType) -> String {
    match st {
        SubscriptionType::DontParticipate => "no",
        SubscriptionType::MaybeAsPlayer => "maybe",
        SubscriptionType::CertainAsPlayer => "yes",
        SubscriptionType::MaybeAsHost => "maybe*",
        SubscriptionType::CertainAsHost => "yes*",
        _ => "unknown",
    }
    .to_string()
}

fn get_framed_card_html(image_url: &str) -> String {
    format!("<div class='pop-art'><img class='pop-art__img' src='{image_url}'/></div>")
}

fn get_treachery_card_category(c: TreacheryCard) -> (&'static str, &'static str) {
    use TreacheryCardType as T;
    match c.type_() {
        T::Projectile => ("weapon", "Projectile weapon"),
        T::Poison => ("weapon", "Poison weapon"),
        T::ProjectileAndPoison => ("weapon", "Projectile and poison weapon"),
        T::Laser | T::WeirdingWay | T::PoisonTooth | T::ArtilleryStrike | T::MirrorWeapon | T::Rockmelter => {
            ("weapon", "Special weapon")
        }
        T::Shield | T::ProjectileDefense => ("defense", "Projectile defense"),
        T::Antidote | T::PoisonDefense | T::PortableAntidote => ("defense", "Poison defense"),
        T::ShieldAndAntidote => ("defense", "Projectile and poison defense"),
        T::Chemistry => ("defense", "Special defense"),
        T::Useless => ("worthless", "Worthless"),
        _ => ("special", "Special"),
    }
}

fn get_image_hover_html(image_url: &str) -> String {
    format!("<img src='{image_url}' width=300 class='img-fluid' style='filter:drop-shadow(-3px 3px 2px black);'/>")
}

pub fn get_faction_faction_force(f: Option<FactionForce>) -> Option<Faction> {
    match f {
        Some(FactionForce::Atreides) => Some(Faction::Atreides),
        Some(FactionForce::Harkonnen) => Some(Faction::Harkonnen),
        Some(FactionForce::Fremen) => Some(Faction::Fremen),
        Some(FactionForce::Emperor) => Some(Faction::Emperor),
        Some(FactionForce::Guild) => Some(Faction::Guild),
        Some(FactionForce::BeneGesserit) => Some(Faction::BeneGesserit),
        Some(FactionForce::Ixian) => Some(Faction::Ixian),
        Some(FactionForce::Tleilaxu) => Some(Faction::Tleilaxu),
        Some(FactionForce::Choam) => Some(Faction::Choam),
        Some(FactionForce::Richese) => Some(Faction::Richese),
        Some(FactionForce::Ecaz) => Some(Faction::Ecaz),
        Some(FactionForce::Moritani) => Some(Faction::Moritani),
        None => None,
    }
}

pub fn get_faction_faction_special_force(f: Option<FactionSpecialForce>) -> Option<Faction> {
    match f {
        Some(FactionSpecialForce::Emperor) => Some(Faction::Emperor),
        Some(FactionSpecialForce::Fremen) => Some(Faction::Fremen),
        Some(FactionSpecialForce::BeneGesserit) => Some(Faction::BeneGesserit),
        Some(FactionSpecialForce::Ixian) => Some(Faction::Ixian),
        Some(FactionSpecialForce::Richese) => Some(Faction::Richese),
        None => None,
    }
}

pub fn describe_ruleset(s: Option<Ruleset>) -> String {
    let Some(s) = s else { return "unknown rule set".to_string() };
    match s {
        Ruleset::BasicGame => "Standard Dune - Basic",
        Ruleset::AdvancedGame => "Standard Dune - Advanced",
        Ruleset::ExpansionBasicGame => "Ixians & Tleilaxu Expansion - Basic",
        Ruleset::ExpansionAdvancedGame => "Ixians & Tleilaxu Expansion - Advanced",
        Ruleset::Expansion2BasicGame => "CHOAM & Richese Expansion - Basic",
        Ruleset::Expansion2AdvancedGame => "CHOAM & Richese Expansion - Advanced",
        Ruleset::Expansion3BasicGame => "Ecaz & Moritani Expansion - Basic",
        Ruleset::Expansion3AdvancedGame => "Ecaz & Moritani Expansion - Advanced",
        Ruleset::AllExpansionsBasicGame => "All Expansions - Basic",
        Ruleset::AllExpansionsAdvancedGame => "All Expansions - Advanced",
        Ruleset::ServerClassic => "Server Classic",
        Ruleset::Custom => "Custom",
    }
    .to_string()
}

pub fn describe_rule_group(s: Option<RuleGroup>) -> String {
    let Some(s) = s else { return "unknown rule group".to_string() };
    match s {
        RuleGroup::CoreAdvanced => "Core Game, Advanced Rules",
        RuleGroup::CoreBasicExceptions => "Core Game, Exceptions to Basic Rules",
        RuleGroup::CoreAdvancedExceptions => "Core Game, Exceptions to Advanced Rules",

        RuleGroup::ExpansionIxAndBtBasic => "Ixians & Tleilaxu Expansion",
        RuleGroup::ExpansionIxAndBtAdvanced => "Ixians & Tleilaxu Expansion, Advanced Rules",

        RuleGroup::ExpansionChoamAndRicheseBasic => "CHOAM & Richese Expansion",
        RuleGroup::ExpansionChoamAndRicheseAdvanced => "CHOAM & Richese Expansion, Advanced Rules",

        RuleGroup::ExpansionEcazAndMoritaniBasic => "Ecaz & Moritani Expansion",
        RuleGroup::ExpansionEcazAndMoritaniAdvanced => "Ecaz & Moritani Expansion, Advanced Rules",

        RuleGroup::House => "House Rules",

        _ => "unknown rule group",
    }
    .to_string()
}

pub fn describe_phase(p: Option<Phase>) -> String {
    match p {
        Some(Phase::TurnConcluded) => "End of turn",
        Some(Phase::Bidding) => "Next bidding round",
        Some(Phase::BiddingReport) => "End of bidding phase",
        Some(Phase::ShipmentAndMoveConcluded) => "End of movement phase",
        Some(Phase::BattleReport) => "End of battle phase",
        Some(Phase::GameEnded) => "End of game",
        _ => "unknown",
    }
    .to_string()
}

pub fn describe_choam_economics_status(p: Option<ChoamEconomicsStatus>) -> String {
    match p {
        Some(ChoamEconomicsStatus::Cancel) => "Cancel",
        Some(ChoamEconomicsStatus::CancelFlipped) => "Cancel",
        Some(ChoamEconomicsStatus::Double) => "Double",
        Some(ChoamEconomicsStatus::DoubleFlipped) => "Double",
        Some(ChoamEconomicsStatus::RemovedFromGame) => "Removed from game",
        _ => "None",
    }
    .to_string()
}

pub fn describe_auction_type(t: Option<AuctionType>) -> String {
    match t {
        Some(AuctionType::Normal) => "Normal",
        Some(AuctionType::BlackMarketNormal) => "Normal",
        Some(AuctionType::BlackMarketSilent) => "Silent",
        Some(AuctionType::BlackMarketOnceAround) => "Once Around",
        Some(AuctionType::RicheseSilent) => "Silent",
        Some(AuctionType::RicheseOnceAround) => "Once Around",
        None => "None",
    }
    .to_string()
}

pub fn describe_juice_type(jt: Option<JuiceType>) -> String {
    match jt {
        Some(JuiceType::GoFirst) => "be considered first in storm order",
        Some(JuiceType::GoLast) => "be considered last in storm order",
        Some(JuiceType::Aggressor) => "be considered aggressor in this battle",
        _ => "None",
    }
    .to_string()
}

pub fn describe_capture_decision(c: Option<CaptureDecision>) -> String {
    match c {
        Some(CaptureDecision::Capture) => "Keep",
        Some(CaptureDecision::Kill) => "Kill to gain 2 spice",
        Some(CaptureDecision::DontCapture) => "Neither keep nor kill",
        _ => "None",
    }
    .to_string()
}

