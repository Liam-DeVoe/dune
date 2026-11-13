//! Port of Model/Skin.cs.
//!
//! The members keep their C# order. Overloads of one C# name get the parameter type appended
//! (engine-conventions.md, "Signatures"): `Describe(object)` is [`Skin::describe`],
//! `Describe(Faction)` is [`Skin::describe_faction`], `GetImageUrl(World)` is
//! [`Skin::get_image_url_world`].
//!
//! `Describe(object)` takes an [`ExpressionElement`], the engine's closed set of the runtime
//! types a message holds. A C# message keeps live collections by reference, so it shows their
//! contents at render time (hazards.md 4.5); the `_in` variants take the game to read those
//! from. Without a game they use the snapshot the engine took when the message was built.
//!
//! The faction rules sheets (`#region FactionManual`) are in [`crate::faction_manual`], and
//! `Load`/`SkinToString` (`#region LoadingAndSaving`) in [`crate::skin_json`].

use treachery_engine::compat::cs::{self, CsNull, NullRef};
use treachery_engine::enumerations::*;
use treachery_engine::messages::expression::ExpressionElement;
use treachery_engine::game::Game;
use treachery_engine::map::homeworld::Homeworld;
use treachery_engine::heroes::hero::Hero;
use treachery_engine::heroes::leader::Leader;
use treachery_engine::map::location::Location;
use treachery_engine::map::Map;
use treachery_engine::messages::message::{Message, MessagePart};
use treachery_engine::map::point::PointD;
use treachery_engine::cards::resource_card::ResourceCard;
use treachery_engine::map::territory::Territory;
use treachery_engine::cards::treachery_card::TreacheryCard;

/// The `Format` argument list: each argument converts with `ExpressionElement::from`, the way
/// C# boxes it into `params object[]`.
#[macro_export]
macro_rules! args {
    ($($a:expr),* $(,)?) => {
        vec![$(::treachery_engine::messages::expression::ExpressionElement::from($a)),*]
    };
}

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

macro_rules! enum_keys {
    ($($t:ty),* $(,)?) => {$(
        impl SkinKey for $t {
            fn key_text(self) -> String {
                self.to_string()
            }
            fn parse_key(text: &str) -> Option<Self> {
                crate::skin_json::parse_enum_key::<$t>(text)
            }
        }
    )*};
}

enum_keys!(
    Concept, MainPhase, Faction, TechToken, Milestone, TreacheryCardType, LeaderSkill, StrongholdAdvantage, World, Nexus,
    TerrorType, DiscoveryToken, DiscoveryTokenType, Ambassador,
);

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

    /// `TryGetValue`
    pub fn get(&self, key: K) -> Option<&V> {
        self.entries.iter().find(|(k, _)| *k == key).map(|(_, v)| v)
    }

    /// `ContainsKey`
    pub fn contains_key(&self, key: K) -> bool {
        self.get(key).is_some()
    }

    /// The indexer `dict[key]`: KeyNotFoundException when the key is missing.
    pub fn index(&self, key: K) -> &V {
        match self.get(key) {
            Some(v) => v,
            None => cs::key_not_found(&key.key_text()),
        }
    }

    /// `dict[key] = value`
    pub fn set(&mut self, key: K, value: V) {
        match self.entries.iter_mut().find(|(k, _)| *k == key) {
            Some(e) => e.1 = value,
            None => self.entries.push((key, value)),
        }
    }

    /// `Keys`
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
    /// `GetValueOrDefault(key, fallback)`
    pub fn get_value_or_default(&self, key: K, fallback: &str) -> String {
        self.get(key).cloned().unwrap_or_else(|| fallback.to_string())
    }
}

/// C#: `public class Skin : IDescriber` (Model/Skin.cs:14)
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Skin {
    pub version: i32,

    pub concept_str: SkinDictionary<Concept>,
    pub main_phase_str: SkinDictionary<MainPhase>,
    pub person_name_str: SkinDictionary<i32>,
    pub person_image_url: SkinDictionary<i32>,
    pub territory_name_str: SkinDictionary<i32>,
    pub territory_border_svg: SkinDictionary<i32>,
    pub location_center_point: SkinDictionary<i32, PointD>,
    pub location_spice_point: SkinDictionary<i32, PointD>,
    pub faction_name_str: SkinDictionary<Faction>,
    pub faction_image_url: SkinDictionary<Faction>,
    pub faction_table_image_url: SkinDictionary<Faction>,
    pub faction_facedown_image_url: SkinDictionary<Faction>,
    pub faction_force_image_url: SkinDictionary<Faction>,
    pub faction_special_force_image_url: SkinDictionary<Faction>,
    pub faction_color: SkinDictionary<Faction>,
    pub force_name_str: SkinDictionary<Faction>,
    pub special_force_name_str: SkinDictionary<Faction>,
    pub tech_token_name_str: SkinDictionary<TechToken>,
    pub tech_token_image_url: SkinDictionary<TechToken>,
    pub sound: SkinDictionary<Milestone>,
    pub treachery_card_type_str: SkinDictionary<TreacheryCardType>,
    pub treachery_card_name_str: SkinDictionary<i32>,
    pub treachery_card_description_str: SkinDictionary<i32>,
    pub treachery_card_image_url: SkinDictionary<i32>,
    pub tech_token_description_str: SkinDictionary<TechToken>,
    pub resource_card_image_url: SkinDictionary<i32>,
    pub leader_skill_card_name_str: SkinDictionary<LeaderSkill>,
    pub leader_skill_card_image_url: SkinDictionary<LeaderSkill>,
    pub stronghold_card_name_str: SkinDictionary<i32>,
    pub stronghold_card_image_url: SkinDictionary<StrongholdAdvantage>,
    pub home_world_image_url: SkinDictionary<World>,
    pub home_world_card_image_url: SkinDictionary<World>,
    pub nexus_card_image_url: SkinDictionary<Nexus>,
    pub terror_token_name_str: SkinDictionary<TerrorType>,
    pub terror_token_description_str: SkinDictionary<TerrorType>,
    pub discovery_token_name_str: SkinDictionary<DiscoveryToken>,
    pub discovery_token_description_str: SkinDictionary<DiscoveryToken>,
    pub discovery_token_image_url: SkinDictionary<DiscoveryToken>,
    pub discovery_token_type_name_str: SkinDictionary<DiscoveryTokenType>,
    pub discovery_token_type_image_url: SkinDictionary<DiscoveryTokenType>,
    pub ambassador_image_url: SkinDictionary<Ambassador>,
    pub ambassador_name_str: SkinDictionary<Ambassador>,
    pub ambassador_description_str: SkinDictionary<Ambassador>,

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

impl Skin {
    /// `public const int LatestVersion = 151`
    pub const LATEST_VERSION: i32 = 151;

    // #region Descriptions

    /// `Format(string m, params object[] list)`: `string.Format` over the described arguments.
    /// Any exception, in `Describe` or in `string.Format`, gives back `m` unchanged.
    pub fn format(&self, m: &str, list: &[ExpressionElement]) -> String {
        self.format_in(m, list, None)
    }

    /// [`Skin::format`], reading live collections from `game`.
    pub fn format_in(&self, m: &str, list: &[ExpressionElement], game: Option<&Game>) -> String {
        if list.is_empty() {
            return m.to_string();
        }
        let args = self.describe_array(list, game);
        let args: Vec<Option<&str>> = args.iter().map(|a| Some(a.as_str())).collect();
        dotnet_compat::fmt::string_format(m, &args).unwrap_or_else(|_| m.to_string())
    }

    /// `Join<T>(IEnumerable<T>? items)`: the described items joined with `", "`, or `"-"` when
    /// that is empty.
    pub fn join(&self, items: Option<&[ExpressionElement]>) -> String {
        self.join_in(items, None)
    }

    pub fn join_in(&self, items: Option<&[ExpressionElement]>, game: Option<&Game>) -> String {
        let Some(items) = items else { return String::new() };
        let result = items.iter().map(|i| self.describe_in(i, game)).collect::<Vec<_>>().join(", ");
        if result.is_empty() {
            return "-".to_string();
        }
        result
    }

    /// `GetCenter(Location location, Map map)`
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

    /// `HmsRadius => 1.5f * PlayerTokenRadius`
    pub fn hms_radius(&self) -> f32 {
        1.5f32 * self.player_token_radius as f32
    }

    /// `GetSpiceLocation(Location location)`
    pub fn get_spice_location(&self, location: Location) -> PointD {
        if location.spice_blow_amount() != 0 { *self.location_spice_point.index(location.id()) } else { PointD::new(0.0, 0.0) }
    }

    /// `Describe(object? value)`
    pub fn describe(&self, value: &ExpressionElement) -> String {
        self.describe_in(value, None)
    }

    /// `Describe(object? value)`, reading live collections from `game`.
    pub fn describe_in(&self, value: &ExpressionElement, game: Option<&Game>) -> String {
        use ExpressionElement as E;
        match value {
            E::Null => String::new(),
            E::String(s) => s.clone(),
            E::Bool(b) => if *b { "Yes" } else { "No" }.to_string(),
            E::Message(m) => self.message_to_string_in(m, game),
            E::MessagePart(p) => self.message_part_to_string_in(p, game),
            E::Payment(p) => self.format_in("{0} {1}", &args![p.amount, Concept::Resource], game),
            E::Enum(e) => {
                if let Some(v) = e.get::<UserStatus>() {
                    describe_user_status(v)
                } else if let Some(v) = e.get::<SubscriptionType>() {
                    describe_subscription_type(v)
                } else if let Some(v) = e.get::<Concept>() {
                    self.describe_concept(v)
                } else if let Some(v) = e.get::<Faction>() {
                    self.describe_faction(v)
                } else if let Some(v) = e.get::<Ambassador>() {
                    self.describe_ambassador(v)
                } else if let Some(v) = e.get::<PrescienceAspect>() {
                    self.describe_prescience_aspect(v)
                } else if let Some(v) = e.get::<FactionForce>() {
                    self.describe_faction_force(v)
                } else if let Some(v) = e.get::<FactionSpecialForce>() {
                    self.describe_faction_special_force(v)
                } else if let Some(v) = e.get::<FactionAdvantage>() {
                    self.describe_faction_advantage(v)
                } else if let Some(v) = e.get::<TreacheryCardType>() {
                    self.describe_treachery_card_type(v)
                } else if let Some(v) = e.get::<LeaderSkill>() {
                    self.describe_leader_skill(v)
                } else if let Some(v) = e.get::<TerrorType>() {
                    self.describe_terror_type(v)
                } else if let Some(v) = e.get::<DiscoveryToken>() {
                    self.describe_discovery_token(v)
                } else if let Some(v) = e.get::<DiscoveryTokenType>() {
                    self.describe_discovery_token_type(v)
                } else if let Some(v) = e.get::<Ruleset>() {
                    describe_ruleset(v)
                } else if let Some(v) = e.get::<AutomationRuleType>() {
                    self.describe_automation_rule_type(v)
                } else if let Some(v) = e.get::<RuleGroup>() {
                    describe_rule_group(v)
                } else if let Some(v) = e.get::<Rule>() {
                    self.describe_rule(v)
                } else if let Some(v) = e.get::<MainPhase>() {
                    self.describe_main_phase(v)
                } else if let Some(v) = e.get::<TechToken>() {
                    self.describe_tech_token(v)
                } else if let Some(v) = e.get::<WinMethod>() {
                    self.describe_win_method(v)
                } else if let Some(v) = e.get::<Phase>() {
                    describe_phase(v)
                } else if let Some(v) = e.get::<ChoamEconomicsStatus>() {
                    describe_choam_economics_status(v)
                } else if let Some(v) = e.get::<AuctionType>() {
                    describe_auction_type(v)
                } else if let Some(v) = e.get::<JuiceType>() {
                    describe_juice_type(v)
                } else if let Some(v) = e.get::<CaptureDecision>() {
                    describe_capture_decision(v)
                } else if let Some(v) = e.get::<StrongholdAdvantage>() {
                    self.describe_stronghold_advantage(v)
                } else if let Some(v) = e.get::<ClairVoyanceAnswer>() {
                    describe_clair_voyance_answer(v)
                } else {
                    // `value.ToString()`: the enum's name (or number, or flags list).
                    value.to_cs_string()
                }
            }
            E::ResourceCard(rc) => self.describe_resource_card(rc.opt()),
            E::Hero(Hero::TreacheryCard(tc)) if !tc.is_null() => self.describe_treachery_card(*tc),
            E::Territory(t) if !t.is_null() => self.describe_territory(*t),
            E::Location(l) if !l.is_null() => self.describe_location(*l),
            E::Hero(h) if !h.is_null() => self.describe_hero(Some(*h)),
            // A null handle is a C# null.
            E::Hero(_) | E::Territory(_) | E::Location(_) => String::new(),
            E::List { items, live, .. } => {
                // C# enumerates the collection now; a live one shows the game's current contents.
                let current = match (live, game) {
                    (Some(source), Some(g)) => source.read(g),
                    _ => None,
                };
                self.join_in(Some(current.as_deref().unwrap_or(items)), game)
            }
            // `value.ToString()`: numbers (invariant culture), exceptions, `Player`.
            E::Int(_) | E::Long(_) | E::Float(_) | E::Double(_) | E::Error(_) | E::Player(_) => value.to_cs_string(),
        }
    }

    /// `private object[] Describe(object[] objects)`
    fn describe_array(&self, objects: &[ExpressionElement], game: Option<&Game>) -> Vec<String> {
        objects.iter().map(|o| self.describe_in(o, game)).collect()
    }

    /// `Describe(Territory t)`
    pub fn describe_territory(&self, t: Territory) -> String {
        get_label(&self.territory_name_str, t.skin_id())
    }

    /// `Describe(IHero? hero)`
    pub fn describe_hero(&self, hero: Option<Hero>) -> String {
        let hero = hero.and_then(CsNull::opt);
        match hero {
            None => "?".to_string(),
            Some(Hero::Leader(l)) => get_label(&self.person_name_str, l.skin_id()),
            Some(Hero::Messiah) => self.describe_concept(Concept::Messiah),
            Some(Hero::TreacheryCard(tc)) => self.describe_treachery_card(tc),
        }
    }

    /// `Describe(Location l)`
    pub fn describe_location(&self, l: Location) -> String {
        if !l.orientation().is_empty() {
            return self.format("{0} ({1} Sector)", &args![l.territory(), l.orientation()]);
        }
        self.describe_territory(l.territory())
    }

    /// `Describe(Faction f)`
    pub fn describe_faction(&self, f: Faction) -> String {
        get_label(&self.faction_name_str, f)
    }

    /// `Describe(Concept c)`
    pub fn describe_concept(&self, c: Concept) -> String {
        get_label(&self.concept_str, c)
    }

    /// `Describe(TreacheryCardType t)`
    pub fn describe_treachery_card_type(&self, t: TreacheryCardType) -> String {
        get_label(&self.treachery_card_type_str, t)
    }

    /// `Describe(ResourceCard? c)`
    pub fn describe_resource_card(&self, c: Option<ResourceCard>) -> String {
        let c = c.and_then(CsNull::opt);
        let Some(c) = c else { return "?".to_string() };
        if c.is_shai_hulud() {
            return self.describe_concept(Concept::Monster);
        }
        if c.is_great_maker() {
            return self.describe_concept(Concept::GreatMonster);
        }
        if c.is_sand_trout() {
            return self.describe_concept(Concept::BabyMonster);
        }
        self.describe_territory(c.location().nre().territory())
    }

    /// `Describe(TreacheryCard c)`
    pub fn describe_treachery_card(&self, c: TreacheryCard) -> String {
        get_label(&self.treachery_card_name_str, c.skin_id())
    }

    /// `Describe(PrescienceAspect aspect)`
    pub fn describe_prescience_aspect(&self, aspect: PrescienceAspect) -> String {
        match aspect {
            PrescienceAspect::Dial => "Dial",
            PrescienceAspect::Leader => "Leader",
            PrescienceAspect::Weapon => "Weapon",
            PrescienceAspect::Defense => "Defense",
            _ => "",
        }
        .to_string()
    }

    /// `Describe(LeaderSkill l)`
    pub fn describe_leader_skill(&self, l: LeaderSkill) -> String {
        get_label(&self.leader_skill_card_name_str, l)
    }

    /// `Describe(DiscoveryToken dt)`
    pub fn describe_discovery_token(&self, dt: DiscoveryToken) -> String {
        get_label(&self.discovery_token_name_str, dt)
    }

    /// `GetDiscoveryTokenDescription(DiscoveryToken dt)`
    pub fn get_discovery_token_description(&self, dt: DiscoveryToken) -> String {
        get_label(&self.discovery_token_description_str, dt)
    }

    /// `private static string GetPopup(string c)`
    pub fn get_popup_string(c: &str) -> String {
        format!("<div style='background-color:white;border-color:black;border-width:1px;border-style:solid;color:black;padding:2px;'>{c}</div>")
    }

    /// `SpiceHtml(object amount, int size = 18)`: a spice amount drawn on the harvester icon.
    pub fn spice_html(&self, amount: &str, size: i32) -> String {
        let name = self.describe_concept(Concept::Resource);
        format!(
            "<span class='spice-amt' style='--s:{size}px' title='{name}'><img src='{}' alt='{name}'/><b>{amount}</b></span>",
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
        let name = self.describe_concept(Concept::Resource);
        crate::text::replace_spice_amounts(&text, &name, |digits| self.spice_html(digits, size))
    }

    /// `private string GetTokenPopup(string? imageUrl, string name, string rulesText)`
    fn get_token_popup(&self, image_url: Option<&str>, name: &str, rules_text: &str) -> String {
        let art = match image_url {
            None | Some("") => String::new(),
            Some(url) => format!("<div class='pop-token-art'><img src='{url}'/></div>"),
        };
        format!(
            "<div class='pop-token'>{art}<div class='pop-token-body'><h6>{name}</h6><p>{}</p></div></div>",
            self.with_spice_icons(rules_text, 18, true)
        )
    }

    /// `GetPopup(TerrorType t)`
    pub fn get_popup_terror_type(&self, t: TerrorType) -> String {
        self.get_token_popup(None, &self.describe_terror_type(t), &self.get_terror_type_description(t))
    }

    /// `GetPopup(Ambassador a)`
    pub fn get_popup_ambassador(&self, a: Ambassador) -> String {
        let name = self.describe_ambassador(a);
        let mut text = self.get_ambassador_description(a);
        if let Some(dash) = text.find(" - ") {
            text = text[dash + 3..].to_string();
        }
        self.get_token_popup(Some(&self.get_image_url_ambassador(a)), &format!("{name} Ambassador"), &text)
    }

    /// `GetPopup(DiscoveryToken dt)`
    pub fn get_popup_discovery_token(&self, dt: DiscoveryToken) -> String {
        self.get_token_popup(
            Some(&self.get_image_url_discovery_token(dt)),
            &self.describe_discovery_token(dt),
            &self.get_discovery_token_description(dt),
        )
    }

    /// `GetPopup(LeaderSkill c)`
    pub fn get_popup_leader_skill(&self, c: LeaderSkill) -> String {
        get_framed_card_html(&self.get_image_url_leader_skill(c))
    }

    /// `GetPopup(TechToken tt)`
    pub fn get_popup_tech_token(&self, tt: TechToken) -> String {
        self.get_token_popup(
            Some(&self.get_image_url_tech_token(tt)),
            &self.describe_tech_token(tt),
            &get_label(&self.tech_token_description_str, tt),
        )
    }

    /// `GetNexusCardPopup(Nexus n)`
    pub fn get_nexus_card_popup(&self, n: Nexus) -> String {
        get_framed_card_html(&self.get_image_url_nexus(n))
    }

    /// `GetPopup(Homeworld w)`
    pub fn get_popup_homeworld(&self, w: Homeworld) -> String {
        get_framed_card_html(&self.get_homeworld_card_image_url(w.world()))
    }

    /// `GetPopup(TreacheryCard? c)`
    pub fn get_popup_treachery_card(&self, c: Option<TreacheryCard>) -> String {
        let c = c.and_then(CsNull::opt);
        let Some(c) = c else { return String::new() };
        let (category, type_) = get_treachery_card_category(c);
        format!(
            "<div class='pop-card'><img class='pop-card-art' src='{}'/><div class='pop-card-body'><h6>{}</h6><div class='pop-card-type'><i class='{category}'></i>{type_}</div><p>{}</p></div></div>",
            self.get_image_url_treachery_card(Some(c)),
            self.describe_treachery_card(c),
            self.with_spice_icons(&self.get_treachery_card_rules_text(c), 18, true)
        )
    }

    /// `private string GetTreacheryCardRulesText(TreacheryCard c)`
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

    /// `GetPopup(IHero h)`
    pub fn get_popup_hero(&self, h: Hero) -> String {
        if let Some(tc) = h.as_treachery_card() {
            return self.get_popup_treachery_card(Some(tc));
        }
        if let Some(l) = h.as_leader() {
            return self.get_leader_popup(l);
        }
        get_image_hover_html(&self.get_image_url_hero(h.opt()))
    }

    /// `private string GetLeaderPopup(Leader l)`
    fn get_leader_popup(&self, l: Leader) -> String {
        let color = self.get_faction_color(l.faction());
        format!(
            "<div class='pop-leader'><div class='pop-leader-art' style='--fc:{color}'><img src='{}'/></div><div class='pop-leader-body'><h6>{}</h6>{}</div></div>",
            self.get_image_url_hero(Some(Hero::Leader(l))),
            self.describe_hero(Some(Hero::Leader(l))),
            self.faction_chip_html(l.faction(), false, None)
        )
    }

    /// `FactionChipHtml(Faction f, bool you = false, IEnumerable<int>? locationIds = null)`: the
    /// markup FactionChip.razor renders.
    pub fn faction_chip_html(&self, f: Faction, you: bool, location_ids: Option<&[i32]>) -> String {
        let name = crate::text::html_encode(&self.describe_faction(f));
        let locations = match location_ids {
            Some(ids) => ids.iter().map(|i| i.to_string()).collect::<Vec<_>>().join(","),
            None => String::new(),
        };
        format!(
            "<span class='chip faction{}' data-ref-faction='{f}' data-ref-locations='{locations}' style='--fc:{}' title='{name}'><img src='{}' alt=''/><span class='name'>{name}</span></span>",
            if you { " you" } else { "" },
            self.get_faction_color(f),
            self.get_image_url_faction(f)
        )
    }

    /// `GetPopup(IHero? h, Game g)`
    pub fn get_popup_hero_game(&self, h: Option<Hero>, g: &Game) -> String {
        let h = h.and_then(CsNull::opt);
        let Some(h) = h else { return String::new() };
        if let Some(tc) = h.as_treachery_card() {
            return self.get_popup_treachery_card(Some(tc));
        }

        let skill = g.skill(Some(h));

        if skill == LeaderSkill::None {
            return self.get_popup_hero(h);
        }

        self.get_popup_leader_leader_skill(h.as_leader(), skill)
    }

    /// `GetPopup(Leader? l, LeaderSkill s)`
    pub fn get_popup_leader_leader_skill(&self, l: Option<Leader>, s: LeaderSkill) -> String {
        let l = l.and_then(CsNull::opt);
        match l {
            None => String::new(),
            Some(l) => format!(
                "<div style='position:relative'><img style='position:relative;filter:drop-shadow(-3px 3px 2px black);' src='{}' width=300/><img src='{}' width=140 style='position:absolute;left:200px;top:120px;filter:drop-shadow(-2px 2px 2px black);'/></div>",
                self.get_image_url_leader_skill(s),
                self.get_image_url_hero(Some(Hero::Leader(l)))
            ),
        }
    }

    /// `GetPopup(StrongholdAdvantage adv)`
    pub fn get_popup_stronghold_advantage(&self, adv: StrongholdAdvantage) -> String {
        get_image_hover_html(&self.get_image_url_stronghold_advantage(adv))
    }

    /// `GetPopup(StrongholdAdvantage adv, Faction f)`
    pub fn get_popup_stronghold_advantage_faction(&self, adv: StrongholdAdvantage, f: Faction) -> String {
        format!(
            "<div style='position:relative'><img style='position:relative;filter:drop-shadow(-3px 3px 2px black);' src='{}' width=300/><img src='{}' width=100 style='position:absolute;left:220px;top:40px;filter:drop-shadow(-3px 3px 2px black);'/></div>",
            self.get_image_url_stronghold_advantage(adv),
            self.get_image_url_faction(f)
        )
    }

    /// `GetImageUrl(object obj)`
    pub fn get_image_url(&self, obj: &ExpressionElement) -> String {
        use ExpressionElement as E;
        match obj {
            E::Hero(Hero::TreacheryCard(tc)) if !tc.is_null() => self.get_image_url_treachery_card(Some(*tc)),
            E::ResourceCard(rc) if !rc.is_null() => self.get_image_url_resource_card(Some(*rc)),
            E::Hero(h) if !h.is_null() => self.get_image_url_hero(Some(*h)),
            E::Enum(e) => {
                if let Some(v) = e.get::<LeaderSkill>() {
                    self.get_image_url_leader_skill(v)
                } else if let Some(v) = e.get::<TechToken>() {
                    self.get_image_url_tech_token(v)
                } else if let Some(v) = e.get::<Faction>() {
                    self.get_image_url_faction(v)
                } else if let Some(v) = e.get::<Nexus>() {
                    self.get_image_url_nexus(v)
                } else if let Some(v) = e.get::<Ambassador>() {
                    self.get_image_url_ambassador(v)
                } else if let Some(v) = e.get::<FactionForce>() {
                    self.get_image_url_faction_force(v)
                } else if let Some(v) = e.get::<FactionSpecialForce>() {
                    self.get_image_url_faction_special_force(v)
                } else if let Some(v) = e.get::<StrongholdAdvantage>() {
                    self.get_image_url_stronghold_advantage(v)
                } else {
                    String::new()
                }
            }
            _ => String::new(),
        }
    }

    /// `GetImageUrl(DiscoveryTokenType dtt)`
    pub fn get_image_url_discovery_token_type(&self, dtt: DiscoveryTokenType) -> String {
        get_label(&self.discovery_token_type_image_url, dtt)
    }

    /// `GetImageUrl(DiscoveryToken dt)`
    pub fn get_image_url_discovery_token(&self, dt: DiscoveryToken) -> String {
        get_label(&self.discovery_token_image_url, dt)
    }

    /// `Describe(DiscoveryTokenType dtt)`
    pub fn describe_discovery_token_type(&self, dtt: DiscoveryTokenType) -> String {
        get_label(&self.discovery_token_type_name_str, dtt)
    }

    /// `private string Describe(TerrorType terr)`
    pub fn describe_terror_type(&self, terr: TerrorType) -> String {
        get_label(&self.terror_token_name_str, terr)
    }

    /// `GetTerrorTypeDescription(TerrorType terr)`
    pub fn get_terror_type_description(&self, terr: TerrorType) -> String {
        get_label(&self.terror_token_description_str, terr)
    }

    /// `Describe(Ambassador ambassador)`
    pub fn describe_ambassador(&self, ambassador: Ambassador) -> String {
        get_label(&self.ambassador_name_str, ambassador)
    }

    /// `GetAmbassadorDescription(Ambassador ambassador)`
    pub fn get_ambassador_description(&self, ambassador: Ambassador) -> String {
        get_label(&self.ambassador_description_str, ambassador)
    }

    /// `GetImageUrl(Ambassador ambassador)`
    pub fn get_image_url_ambassador(&self, ambassador: Ambassador) -> String {
        get_label(&self.ambassador_image_url, ambassador)
    }

    /// `Describe(MainPhase p)`
    pub fn describe_main_phase(&self, p: MainPhase) -> String {
        get_label(&self.main_phase_str, p)
    }

    /// `private string Describe(TechToken tt)`
    pub fn describe_tech_token(&self, tt: TechToken) -> String {
        get_label(&self.tech_token_name_str, tt)
    }

    /// `Describe(FactionForce f)`
    pub fn describe_faction_force(&self, f: FactionForce) -> String {
        get_label(&self.force_name_str, get_faction_faction_force(f))
    }

    /// `Describe(FactionSpecialForce f)`
    pub fn describe_faction_special_force(&self, f: FactionSpecialForce) -> String {
        get_label(&self.special_force_name_str, get_faction_faction_special_force(f))
    }

    /// `private string Describe(FactionAdvantage advantage)`
    pub fn describe_faction_advantage(&self, advantage: FactionAdvantage) -> String {
        use FactionAdvantage as A;
        use FactionSpecialForce as Fsf;
        match advantage {
            A::None => "- Any stoppable advantage not listed below -".to_string(),
            A::AtreidesBiddingPrescience => self.format("{0} seeing the card on auction", &args![Faction::Atreides]),
            A::AtreidesSpiceBlowPrescience => {
                let resource = self.concept_str.index(Concept::Resource).clone();
                self.format("{0} seeing the top {1} card this turn", &args![Faction::Atreides, resource])
            }
            A::AtreidesBattlePlanPrescience => self.format("{0} seeing part of the opponent battle plan", &args![Faction::Atreides]),
            A::AtreidesUseMessiah => {
                let messiah = self.concept_str.index(Concept::Messiah).clone();
                self.format("{0} using the {1} in this battle", &args![Faction::Atreides, messiah])
            }
            A::HarkonnenFreeCard => self.format("{0} taking a free treachery card", &args![Faction::Harkonnen]),
            A::HarkonnenCaptureLeader => self.format("{0} capturing a leader after this battle", &args![Faction::Harkonnen]),
            A::HarkonnenCallTraitorForAlly => self.format("{0} calling TREACHERY for their ally in this battle", &args![Faction::Harkonnen]),
            A::BeneGesseritAccompanies => self.format("{0} accompanying shipment", &args![Faction::BeneGesserit]),
            A::BeneGesseritAnnouncesBattle => self.format("{0} flipping advisors to fighters*", &args![Faction::BeneGesserit]),
            A::BeneGesseritIntrusion => self.format("{0} becoming advisors on intrusion", &args![Faction::BeneGesserit]),
            A::BeneGesseritUsingVoice => self.format("{0} using Voice during this battle", &args![Faction::BeneGesserit]),
            A::BeneGesseritWorthlessAsKarma => self.format(
                "{0} using a {1} as a {2} card*",
                &args![Faction::BeneGesserit, TreacheryCardType::Useless, TreacheryCardType::Karma],
            ),
            A::BeneGesseritCharity => {
                self.format("{0} receiving 2 {1} at {2}", &args![Faction::BeneGesserit, Concept::Resource, MainPhase::Charity])
            }
            A::FremenControlsMonster => self.format(
                "{0} sending {1} when a subsequent {1} card is drawn",
                &args![Faction::Fremen, Concept::Monster],
            ),
            A::FremenRidesMonster => self.format("{0} riding {1}", &args![Faction::Fremen, Concept::Monster]),
            A::FremenNotPayingForBattles => self.format("{0} not paying for forces in this battle", &args![Faction::Fremen]),
            A::FremenSpecialForceBonus => {
                self.format("{0} counting {1} bonus in this battle", &args![Faction::Fremen, Fsf::Fremen])
            }
            A::FremenExtraMove => {
                self.format("{0} moving two territories as part of their move action", &args![Faction::Fremen])
            }
            A::FremenProtectedFromStorm => self.format("{0} not taking storm losses when hit", &args![Faction::Fremen]),
            A::FremenProtectedFromMonster => {
                self.format("{0} not being devoured by {1}", &args![Faction::Fremen, Concept::Monster])
            }
            A::FremenProtectedFromMonsterAlly => {
                self.format("{0} ally not being devoured by {1}", &args![Faction::Fremen, Concept::Monster])
            }
            A::FremenStormPrescience => self.format("{0} seeing the storm movement this turn", &args![Faction::Fremen]),
            A::EmperorSpecialForceBonus => self.format("{0} counting {1} bonus in this battle", &args![Faction::Emperor, Fsf::Emperor]),
            A::EmperorReceiveBid => {
                self.format("{0} receiving {1} for a treachery card", &args![Faction::Emperor, Concept::Resource])
            }
            A::EmperorGiveSpiceToAlly => self.format("{0} giving {1} to their ally*", &args![Faction::Emperor, Concept::Resource]),
            A::EmperorLetAllyReviveExtraForces => self.format(
                "{0} allowing their ally to revive 3 extra forces",
                &args![Faction::Emperor, Concept::Resource],
            ),
            A::GuildDetermineMoveMoment => self.format(
                "{0} shipping out of turn order (play before their normal turn)",
                &args![Faction::Guild],
            ),
            A::GuildSpecialShipments => {
                self.format("{0} (and ally) shipping site-to-site or back to reserves", &args![Faction::Guild])
            }
            A::GuildShipmentsDiscount => self.format("{0} shipping at half price", &args![Faction::Guild]),
            A::GuildShipmentsDiscountAlly => self.format("{0} ally shipping at half price", &args![Faction::Guild]),
            A::GuildReceiveShipment => {
                self.format("{0} receiving {1} for a shipment", &args![Faction::Guild, Concept::Resource])
            }
            A::TleilaxuRevivalDiscount => self.format("{0} reviving at half price", &args![Faction::Tleilaxu]),
            A::TleilaxuRevivalDiscountAlly => self.format("{0} ally reviving at half price", &args![Faction::Tleilaxu]),
            A::TleilaxuReplacingFaceDancer => {
                self.format("{0} replacing a face dancer at end of turn", &args![Faction::Tleilaxu])
            }
            A::TleilaxuIncreasingRevivalLimits => self.format("{0} increasing revival limits", &args![Faction::Tleilaxu]),
            A::TleilaxuReceiveRevive => {
                self.format("{0} receiving {1} for revival", &args![Faction::Tleilaxu, Concept::Resource])
            }
            A::TleilaxuEarlyLeaderRevive => {
                self.format("{0} allowing early revival of a leader*", &args![Faction::Tleilaxu])
            }
            A::TleilaxuReviveGhola => self.format("{0} reviving a leader as a Ghola", &args![Faction::Tleilaxu]),
            A::IxianMovingHms => self.format("{0} moving the Hidden Mobile Stronghold", &args![Faction::Ixian]),
            A::IxianSpecialForceBonus => self.format("{0} counting {1} bonus in this battle", &args![Faction::Ixian, Fsf::Ixian]),
            A::IxianSelectingCardsOnAuction => self.format(
                "{0} selecting a card to go on top or bottom (play before they draw cards on auction)",
                &args![Faction::Ixian],
            ),
            A::IxianCyborgExtraMove => self.format("{0} moving {1} two territories", &args![Faction::Ixian, Fsf::Ixian]),
            A::IxianReplacingSpecialForces => self.format(
                "{0} replacing {1} lost in battle with {2}",
                &args![Faction::Ixian, Fsf::Ixian, FactionForce::Ixian],
            ),
            A::IxianAllyDiscardingCard => {
                self.format("{0} allowing their ally to replace a won card", &args![Faction::Ixian])
            }
            A::IxianSwappingCard => self.format("{0} replacing a treachery card during bidding", &args![Faction::Ixian]),
            A::ChoamControllingCharity => self.format(
                "{0} receiving and giving {1} during {2}",
                &args![Faction::Choam, Concept::Resource, MainPhase::Charity],
            ),
            A::ChoamDiscarding => self.format(
                "{0} discarding cards for {1} or a {2} card for its special effect*",
                &args![Faction::Choam, Concept::Resource, TreacheryCardType::Useless],
            ),
            A::ChoamRevival => self.format(
                "{0} having unlimited force revival and reduced revival cost",
                &args![Faction::Choam],
            ),
            A::ChoamEconomics => self.format(
                "{0} playing their Inflation token during {1}",
                &args![Faction::Choam, MainPhase::Contemplate],
            ),
            A::ChoamReceiveForcePayment => self.format(
                "{0} collecting {1} payment for forces this battle",
                &args![Faction::Choam, Concept::Resource],
            ),
            A::ChoamAudit => self.format("{0} auditing their opponent after this battle", &args![Faction::Choam]),
            A::RicheseAuction => self.format("{0} auctioning a card from their card cache", &args![Faction::Richese]),
            A::RicheseNofield => self.format("{0} using a No-Field to ship", &args![Faction::Richese]),
            A::RicheseBlackMarket => self.format("{0} selling a card from their hand", &args![Faction::Richese]),

            _ => "Unknown".to_string(),
        }
    }

    /// `private string Describe(AutomationRuleType s)`
    pub fn describe_automation_rule_type(&self, s: AutomationRuleType) -> String {
        use AutomationRuleType as A;
        match s {
            A::CharityAutoClaim => "Claim charity if possible".to_string(),
            A::BiddingPassWhenAtreidesOrAtreidesAllyPassed => {
                format!("Pass if most recent bid by {} or their ally passed", self.describe_faction(Faction::Atreides))
            }
            A::BiddingPassAboveAmount => "Pass if current bid is equal to or higher than...".to_string(),
            A::BiddingPassWhenHighestBidByFaction => "Pass if current highest bid by...".to_string(),
            A::RevivalAutoClaimFreeRevival => "Auto claim free revival".to_string(),
            A::ShipmentGuildAutoDelay => "Auto delay shipment until last".to_string(),
            _ => "unknown automation rule".to_string(),
        }
    }

    /// `Describe(WinMethod m)`
    pub fn describe_win_method(&self, m: WinMethod) -> String {
        match m {
            WinMethod::Strongholds => "by number of victory points".to_string(),
            WinMethod::Prediction => "by prediction".to_string(),
            WinMethod::Timeout => "by running out of time".to_string(),
            WinMethod::Forfeit => "by forfeit".to_string(),
            WinMethod::FremenSpecial => self.format("by {0} special victory", &args![Faction::Fremen]),
            WinMethod::GuildSpecial => self.format("by {0} special victory", &args![Faction::Guild]),
            _ => "None".to_string(),
        }
    }

    /// `DescribeDetailed(AuctionType t)`
    pub fn describe_detailed(&self, t: AuctionType) -> String {
        match t {
            AuctionType::Normal => "Normal".to_string(),
            AuctionType::BlackMarketNormal => "Black Market / Normal".to_string(),
            AuctionType::BlackMarketSilent => "Black Market / Silent".to_string(),
            AuctionType::BlackMarketOnceAround => "Black Market / Once Around".to_string(),
            AuctionType::RicheseSilent => self.format("{0} / Silent", &args![Faction::Richese]),
            AuctionType::RicheseOnceAround => self.format("{0} / Once Around", &args![Faction::Richese]),
            _ => "None".to_string(),
        }
    }

    /// `private string Describe(StrongholdAdvantage sa)`. The labels come from the dictionary's
    /// indexer, which throws KeyNotFoundException for a skin without them.
    pub fn describe_stronghold_advantage(&self, sa: StrongholdAdvantage) -> String {
        use StrongholdAdvantage as S;
        match sa {
            S::CollectResourcesForDial => self.stronghold_card_name_str.index(5).clone(),
            S::CollectResourcesForUseless => self.stronghold_card_name_str.index(4).clone(),
            S::CountDefensesAsAntidote => self.stronghold_card_name_str.index(2).clone(),
            S::FreeResourcesForBattles => self.stronghold_card_name_str.index(3).clone(),
            S::WinTies => self.stronghold_card_name_str.index(6).clone(),
            _ => "None".to_string(),
        }
    }

    /// `Describe(Rule r)`
    pub fn describe_rule(&self, r: Rule) -> String {
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
                self.format("{0} determine storm movement with the Storm Deck", &args![Faction::Fremen])
            }
            Rule::FremenStormLosses => self.format("{0} storm losses are halved", &args![Faction::Fremen]),
            Rule::FremenSendingMonster => self.format("{0} place additional {1}s", &args![Faction::Fremen, Concept::Monster]),
            Rule::FremenSpecialForces => self.format("{0} can use {1}", &args![Faction::Fremen, Fsf::Fremen]),
            Rule::AtreidesMessiah => self.format("Enable {0}", &args![Concept::Messiah]),
            Rule::HarkonnenCapturesOrKillsLeaders => self.format("{0} can kill or capture leaders", &args![Faction::Harkonnen]),
            Rule::BeneGesseritFirstForceInAnyTerritory => {
                self.format("{0} can put their initial force anywhere", &args![Faction::BeneGesserit])
            }
            Rule::BeneGesseritAutoCharity => self.format("{0} get 2 charity each turn", &args![Faction::BeneGesserit]),
            Rule::BeneGesseritWorthlessAsKarma => {
                self.format("{0} can use {1} cards for {2}", &args![Faction::BeneGesserit, Tct::Useless, Tct::Karma])
            }
            Rule::BeneGesseritAdvisors => self.format("{0} can use {1}", &args![Faction::BeneGesserit, Fsf::BeneGesserit]),
            Rule::BeneGesseritAccompaniesToShipmentLocation => {
                self.format("{0} can accompany to the shipped to territory", &args![Faction::BeneGesserit])
            }
            Rule::GuildDetermineShipment => {
                self.format("{0} can determine when they ship and move", &args![Faction::Guild])
            }
            Rule::EmperorSpecialForces => self.format("{0} can use {1}", &args![Faction::Emperor, Fsf::Emperor]),
            Rule::BribesAreImmediate => {
                self.format("{0} bribes are immediately added to reserves", &args![Concept::Resource])
            }
            Rule::ContestedStongholdsCountAsOccupied => s("Contested strongholds count as occupied @ end of turn"),
            Rule::AdvisorsDontConflictWithAlly => {
                self.format("{0} {1} can coexist with allied forces", &args![Faction::BeneGesserit, Fsf::BeneGesserit])
            }
            Rule::GuildShipmentContributionsFlowBack => {
                self.format("{0} contributions to shipments flow back to them", &args![Faction::Guild])
            }
            Rule::CustomInitialForcesAndResources => {
                self.format("Custom initial {0} and force positions (for Spice Harvest)", &args![Concept::Resource])
            }
            Rule::HmSwithoutIxian => {
                self.format("Use the {0} if {1} are not in play", &args!["Hidden Mobile Stronghold", Faction::Ixian])
            }
            Rule::StormDeckWithoutFremen => {
                self.format("Use the Storm Deck if {0} are not in play", &args![Faction::Fremen])
            }

            Rule::Ssw => {
                self.format("SSW: {0} counts for victory after fourth {1}", &args!["Shield Wall", Concept::Monster])
            }
            Rule::HarkonnenMulligan => {
                self.format("{0} mulligan traitors when they drew > 1 of their own", &args![Faction::Harkonnen])
            }

            Rule::TechTokens => s("Tech Tokens"),
            Rule::ExpansionTreacheryCards => s("Expansion Treachery Cards"),
            Rule::ExpansionTreacheryCardsExceptPBandSSandAmal => self.format(
                "Treachery Cards: all except {0}, {1} and {2}",
                &args![Tct::ProjectileAndPoison, Tct::ShieldAndAntidote, Tct::Amal],
            ),
            Rule::ExpansionTreacheryCardsPBandSs => {
                self.format("Treachery Cards: {0} and {1}", &args![Tct::ProjectileAndPoison, Tct::ShieldAndAntidote])
            }
            Rule::ExpansionTreacheryCardsAmal => self.format("Treachery Card: {0}", &args![Tct::Amal]),
            Rule::CheapHeroTraitor => s("Cheap Hero Traitor"),
            Rule::SandTrout => self.describe_concept(Concept::BabyMonster),
            Rule::TleilaxuGholas => self.format("{0} may revive leaders as Gholas", &args![Faction::Tleilaxu]),
            Rule::IxianSwappingCardOnBid => {
                self.format("{0} may swap one card on bid with a card from their hand", &args![Faction::Ixian])
            }

            Rule::LeaderSkills => s("Leader Skills"),
            Rule::Expansion2TreacheryCards => {
                self.format("Treachery Cards: {0} and {1}", &args![Tct::ArtilleryStrike, Tct::PoisonTooth])
            }
            Rule::StrongholdBonus => s("Stronghold Bonus"),
            Rule::ChoamAuditor => self.format("{0} gains the Auditor leader", &args![Faction::Choam]),
            Rule::RicheseBlackMarket => self.format("{0} Black Market bidding", &args![Faction::Richese]),

            Rule::GreatMaker => s("Great Maker"),
            Rule::DiscoveryTokens => s("Discovery Tokens"),
            Rule::Homeworlds => s("Homeworlds"),
            Rule::NexusCards => s("Nexus Cards"),
            Rule::Expansion3TreacheryCards => s("Expansion Treachery Cards"),
            Rule::MoritaniAssassinate => self.format("{0} can assassinate leaders", &args![Faction::Moritani]),
            Rule::EcazLoyalty => self.format("{0} gain a loyal leader", &args![Faction::Ecaz]),

            Rule::CustomDecks => s("Customized Treachery Card Deck"),
            Rule::ExtraKaramaCards => self.format("Add three extra {0} cards to the game", &args![Tct::Karma]),
            Rule::FullPhaseKarma => self.format("Full phase {0} (instead of single instance)", &args![Tct::Karma]),
            Rule::FremenMayMoveIntoStorm => self.format("{0} may move into storm", &args![Faction::Fremen]),
            Rule::BeneGesseritVoiceMustNameSpecialCards => s("Voice must target special cards by name"),
            Rule::BattlesUnderStorm => s("Battles may happen under the storm"),
            Rule::MovementBonusRequiresOccupationBeforeMovement => {
                s("Arrakeen/Carthag must be occupied before Ship&Move to grant ornithopters")
            }
            Rule::AssistedNotekeeping => s("Mentat: auto notekeeping of knowable info (spice owned, cards seen, ...)"),
            Rule::AssistedNotekeepingForAtreides => {
                self.format("Mentat: auto notekeeping for {0}", &args![Faction::Atreides])
            }
            Rule::ResourceBonusForStrongholds => s("Stronghold spice bonus (even without increased spice flow)"),
            Rule::DisableNovaFlipping => s("No Nova flipping (advisors will not auto flip after shipment phase)"),

            Rule::FillWithBots => s("Fill empty seats with random Bots"),
            Rule::GuildBot => self.format("{0}Bot", &args![Faction::Guild]),
            Rule::EmperorBot => self.format("{0}Bot", &args![Faction::Emperor]),
            Rule::HarkonnenBot => self.format("{0}Bot", &args![Faction::Harkonnen]),
            Rule::TleilaxuBot => self.format("{0}Bot", &args![Faction::Tleilaxu]),
            Rule::AtreidesBot => self.format("{0}Bot", &args![Faction::Atreides]),
            Rule::BeneGesseritBot => self.format("{0}Bot", &args![Faction::BeneGesserit]),
            Rule::FremenBot => self.format("{0}Bot", &args![Faction::Fremen]),
            Rule::IxianBot => self.format("{0}Bot", &args![Faction::Ixian]),

            Rule::BotsCannotAlly => s("Bots may not initiate alliances"),
            // `Format(m)` with no arguments returns `m` as is.
            Rule::CardsCanBeTraded => self.format("Allow players to give cards to each other", &[]),
            Rule::PlayersChooseFactions => self.format("Let players choose their factions at start", &[]),
            Rule::EmperorSupportingNonAllyBids => {
                self.format("{0} may support bids of non-ally players", &args![Faction::Emperor])
            }
            Rule::BattleWithoutLeader => s("Allow leaderless battles even if leaders are available"),
            Rule::CapturedLeadersAreTraitorsToOwnFaction => {
                s("Captured leaders can be called as traitors by their original factions without a traitor card")
            }
            Rule::DisableEndOfGameReport => s("Disable end-of-game report (don't reveal player shields)"),
            Rule::DisableGuildSpecialVictory => {
                self.format("Disable {0} special victory condition", &args![Faction::Guild])
            }
            Rule::DisableResourceTransfers => {
                self.format("Only allow transfer of {0} by alliance rules", &args![Concept::Resource])
            }
            Rule::FremenAllyGetsDialedResourcesRefunded => self.format(
                "{0} ally may get {1} dialled in battles refunded in {2} phase",
                &args![Faction::Fremen, Concept::Resource, MainPhase::Contemplate],
            ),
            Rule::BreakAlliancesOnAlliancePhase => s("Alliances are  broken at the start of each alliance phase"),

            _ => s("unknown rule"),
        }
    }

    /// `Describe(ErrorType e)`
    pub fn describe_error_type(&self, e: ErrorType) -> String {
        use ErrorType as E;
        match e {
            E::None => "",
            E::UserNotFound => "User not found",
            E::GameNotFound => "Game not found",
            E::GameIdNotFound => "No game was given",
            E::UserTokenNotFound => "You are not logged in",
            E::ScheduledGameNotFound => "Game not found",
            E::UserNameTooShort => "Username must be more than 3 characters",
            E::PlayerNameTooShort => "Player name must be more than 3 characters",
            E::UserNameTooLong => "Username must be 40 characters or less",
            E::PlayerNameTooLong => "Player name must be 40 characters or less",
            E::UserNameExists => "This username already exists",
            E::EmailExists => "This e-mail address is already in use",
            E::UserCreationFailed => "User creation failed",
            E::InvalidGameVersion => "Invalid game version",
            E::InvalidUserNameOrPassword => "Invalid user name or password",
            E::NotAdmin => "You are not ad administrator",
            E::UnknownUsernameOrEmailAddress => "Unknown user name or email address",
            E::NoEmailAddressForUser => "This account has no e-mail address, so its password cannot be reset",
            E::ResetRequestTooSoon => "Please wait at least 5 minutes before requesting another password reset",
            E::UnknownUserName => "Unknown user name",
            E::InvalidResetToken => "Invalid password reset token",
            E::ResetTokenExpired => "Your password reset token has expired",
            E::NoHost => "You are not a host",
            E::InvalidGameEvent => "Invalid game action",
            E::TooManyGames => "You cannot have more than 3 active games, delete a game first",
            E::TooManyScheduledGames => "You cannot schedule more than 10 games, delete a game first",
            E::IncorrectGamePassword => "Incorrect password",
            E::SeatNotAvailableNotOpen => "Seat is not available",
            E::SeatNotAvailableKicked => "You were removed from this game",
            E::AlreadyObserver => "You are already an observer in this game",
            E::CannotRemoveLastHost => "You cannot remove the only remaining host from the game",
            E::NoCreator => "You are not the creator of this game",
            E::AlreadyPlayer => "You are already a player in this game",
            E::UserNotInGame => "User not found in game",

            // An error shown to the player must never take the client down.
            _ => return e.to_string(),
        }
        .to_string()
    }

    // #endregion Descriptions

    // #region NamesAndImages

    /// `GetTerritoryBorder(Territory? t)`
    pub fn get_territory_border(&self, t: Option<Territory>) -> String {
        match t.and_then(CsNull::opt) {
            Some(t) => get_label(&self.territory_border_svg, t.skin_id()),
            None => String::new(),
        }
    }

    /// `GetImageUrl(TreacheryCard? c)`
    pub fn get_image_url_treachery_card(&self, c: Option<TreacheryCard>) -> String {
        match c.and_then(CsNull::opt) {
            Some(c) => get_label(&self.treachery_card_image_url, c.skin_id()),
            None => String::new(),
        }
    }

    /// `GetImageUrl(World w)`
    pub fn get_image_url_world(&self, w: World) -> String {
        get_label(&self.home_world_image_url, w)
    }

    /// `GetImageUrl(StrongholdAdvantage a)`
    pub fn get_image_url_stronghold_advantage(&self, a: StrongholdAdvantage) -> String {
        get_label(&self.stronghold_card_image_url, a)
    }

    /// `GetImageUrl(ResourceCard? c)`
    pub fn get_image_url_resource_card(&self, c: Option<ResourceCard>) -> String {
        match c.and_then(CsNull::opt) {
            Some(c) => get_label(&self.resource_card_image_url, c.skin_id()),
            None => String::new(),
        }
    }

    /// `GetImageUrl(LeaderSkill s)`
    pub fn get_image_url_leader_skill(&self, s: LeaderSkill) -> String {
        get_label(&self.leader_skill_card_image_url, s)
    }

    /// `private string GetTreacheryCardDescription(TreacheryCard? c)`
    pub fn get_treachery_card_description(&self, c: Option<TreacheryCard>) -> String {
        match c.and_then(CsNull::opt) {
            Some(c) => get_label(&self.treachery_card_description_str, c.skin_id()),
            None => String::new(),
        }
    }

    /// `GetImageUrl(IHero? h)`
    pub fn get_image_url_hero(&self, h: Option<Hero>) -> String {
        let Some(h) = h.and_then(CsNull::opt) else { return String::new() };
        match h {
            Hero::TreacheryCard(_) => get_label(&self.treachery_card_image_url, h.skin_id()),
            Hero::Messiah => self.messiah_url.clone(),
            Hero::Leader(_) => get_label(&self.person_image_url, h.skin_id()),
        }
    }

    /// `GetImageUrl(Faction faction)`
    pub fn get_image_url_faction(&self, faction: Faction) -> String {
        get_label(&self.faction_image_url, faction)
    }

    /// `GetFactionTableImageUrl(Faction faction)`
    pub fn get_faction_table_image_url(&self, faction: Faction) -> String {
        get_label(&self.faction_table_image_url, faction)
    }

    /// `GetFactionFaceDownImageUrl(Faction faction)`
    pub fn get_faction_face_down_image_url(&self, faction: Faction) -> String {
        get_label(&self.faction_facedown_image_url, faction)
    }

    /// `GetFactionForceImageUrl(Faction f)`
    pub fn get_faction_force_image_url(&self, f: Faction) -> String {
        get_label(&self.faction_force_image_url, f)
    }

    /// `GetFactionSpecialForceImageUrl(Faction f)`
    pub fn get_faction_special_force_image_url(&self, f: Faction) -> String {
        get_label(&self.faction_special_force_image_url, f)
    }

    /// `private string GetImageUrl(FactionForce ff)`
    pub fn get_image_url_faction_force(&self, ff: FactionForce) -> String {
        self.get_faction_force_image_url(get_faction_faction_force(ff))
    }

    /// `private string GetImageUrl(FactionSpecialForce fsf)`
    pub fn get_image_url_faction_special_force(&self, fsf: FactionSpecialForce) -> String {
        self.get_faction_special_force_image_url(get_faction_faction_special_force(fsf))
    }

    /// `GetHomeworldCardImageUrl(World w)`
    pub fn get_homeworld_card_image_url(&self, w: World) -> String {
        get_label(&self.home_world_card_image_url, w)
    }

    /// `GetImageUrl(Nexus n)`
    pub fn get_image_url_nexus(&self, n: Nexus) -> String {
        get_label(&self.nexus_card_image_url, n)
    }

    /// `GetImageUrl(TechToken tech)`
    pub fn get_image_url_tech_token(&self, tech: TechToken) -> String {
        get_label(&self.tech_token_image_url, tech)
    }

    /// `GetSound(Milestone m)`
    pub fn get_sound(&self, m: Milestone) -> String {
        get_label(&self.sound, m)
    }

    /// `GetFactionColor(Faction faction)`
    pub fn get_faction_color(&self, faction: Faction) -> String {
        get_label(&self.faction_color, faction)
    }

    /// `GetFactionColorTransparent(Faction faction, string transparency)`
    pub fn get_faction_color_transparent(&self, faction: Faction, transparency: &str) -> String {
        get_label(&self.faction_color, faction) + transparency
    }

    // #endregion NamesAndImages

    // Message.cs: the rendering half of `Message` and `MessagePart`.

    /// `Message.ToString(IDescriber describer)`: every element described and concatenated.
    pub fn message_to_string(&self, m: &Message) -> String {
        self.message_to_string_in(m, None)
    }

    pub fn message_to_string_in(&self, m: &Message, game: Option<&Game>) -> String {
        m.expression.elements.iter().map(|x| self.describe_in(x, game)).collect()
    }

    /// `Message.ToString()` while `Message.DefaultDescriber` is this skin (the C# server sets
    /// `DefaultSkin.Default`): `ToString(DefaultDescriber) + "*"`.
    pub fn message_to_string_with_default_describer(&self, m: &Message) -> String {
        self.message_to_string(m) + "*"
    }

    /// `MessagePart.ToString(IDescriber describer)`
    pub fn message_part_to_string(&self, p: &MessagePart) -> String {
        self.message_part_to_string_in(p, None)
    }

    pub fn message_part_to_string_in(&self, p: &MessagePart, game: Option<&Game>) -> String {
        p.expression.elements.iter().map(|x| self.describe_in(x, game)).collect()
    }

    /// `MessagePart.ToString()` while `Message.DefaultDescriber` is this skin.
    pub fn message_part_to_string_with_default_describer(&self, p: &MessagePart) -> String {
        self.message_part_to_string(p) + "*"
    }
}

/// `string.StartsWith(string)`, which is culture-sensitive in .NET. Every prefix the skin tests
/// is ASCII text without ignorable characters, so this is ordinal.
fn starts_with_culture(text: &str, prefix: &str) -> bool {
    text.starts_with(prefix)
}

/// `private static string Describe(ClairVoyanceAnswer answer)`
pub fn describe_clair_voyance_answer(answer: ClairVoyanceAnswer) -> String {
    match answer {
        ClairVoyanceAnswer::No => "No.",
        ClairVoyanceAnswer::Yes => "Yes.",
        ClairVoyanceAnswer::Unknown => "I don't know...",
        _ => "?",
    }
    .to_string()
}

/// `private static string Describe(UserStatus us)`
pub fn describe_user_status(us: UserStatus) -> String {
    match us {
        UserStatus::None => "No status",
        UserStatus::Online => "Online",
        UserStatus::Away => "Away (not available)",
        UserStatus::Lfg => "Looking for game",
        UserStatus::Lfm => "Looking for others to join",
        UserStatus::InGame => "In game",
        _ => "Unknown status",
    }
    .to_string()
}

/// `private static string Describe(SubscriptionType st)`
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

/// `private static string GetFramedCardHtml(string imageUrl)`
fn get_framed_card_html(image_url: &str) -> String {
    format!("<div class='pop-art'><img src='{image_url}'/></div>")
}

/// `private static (string Category, string Type) GetTreacheryCardCategory(TreacheryCard c)`
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

/// `private string GetImageHoverHtml(string imageUrl)`
fn get_image_hover_html(image_url: &str) -> String {
    format!("<img src='{image_url}' width=300 class='img-fluid' style='filter:drop-shadow(-3px 3px 2px black);'/>")
}

/// `private static Faction GetFaction(FactionForce f)`
pub fn get_faction_faction_force(f: FactionForce) -> Faction {
    match f {
        FactionForce::Atreides => Faction::Atreides,
        FactionForce::Harkonnen => Faction::Harkonnen,
        FactionForce::Fremen => Faction::Fremen,
        FactionForce::Emperor => Faction::Emperor,
        FactionForce::Guild => Faction::Guild,
        FactionForce::BeneGesserit => Faction::BeneGesserit,
        FactionForce::Ixian => Faction::Ixian,
        FactionForce::Tleilaxu => Faction::Tleilaxu,
        FactionForce::Choam => Faction::Choam,
        FactionForce::Richese => Faction::Richese,
        FactionForce::Ecaz => Faction::Ecaz,
        FactionForce::Moritani => Faction::Moritani,
        _ => Faction::None,
    }
}

/// `private static Faction GetFaction(FactionSpecialForce f)`
pub fn get_faction_faction_special_force(f: FactionSpecialForce) -> Faction {
    match f {
        FactionSpecialForce::Emperor => Faction::Emperor,
        FactionSpecialForce::Fremen => Faction::Fremen,
        FactionSpecialForce::BeneGesserit => Faction::BeneGesserit,
        FactionSpecialForce::Ixian => Faction::Ixian,
        FactionSpecialForce::Richese => Faction::Richese,
        _ => Faction::None,
    }
}

/// `private static string Describe(Ruleset s)`
pub fn describe_ruleset(s: Ruleset) -> String {
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

        _ => "unknown rule set",
    }
    .to_string()
}

/// `private static string Describe(RuleGroup s)`
pub fn describe_rule_group(s: RuleGroup) -> String {
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

/// `private static string Describe(Phase p)`
pub fn describe_phase(p: Phase) -> String {
    match p {
        Phase::TurnConcluded => "End of turn",
        Phase::Bidding => "Next bidding round",
        Phase::BiddingReport => "End of bidding phase",
        Phase::ShipmentAndMoveConcluded => "End of movement phase",
        Phase::BattleReport => "End of battle phase",
        Phase::GameEnded => "End of game",
        _ => "unknown",
    }
    .to_string()
}

/// `private static string Describe(ChoamEconomicsStatus p)`
pub fn describe_choam_economics_status(p: ChoamEconomicsStatus) -> String {
    match p {
        ChoamEconomicsStatus::Cancel => "Cancel",
        ChoamEconomicsStatus::CancelFlipped => "Cancel",
        ChoamEconomicsStatus::Double => "Double",
        ChoamEconomicsStatus::DoubleFlipped => "Double",
        ChoamEconomicsStatus::RemovedFromGame => "Removed from game",
        _ => "None",
    }
    .to_string()
}

/// `private static string Describe(AuctionType t)`
pub fn describe_auction_type(t: AuctionType) -> String {
    match t {
        AuctionType::Normal => "Normal",
        AuctionType::BlackMarketNormal => "Normal",
        AuctionType::BlackMarketSilent => "Silent",
        AuctionType::BlackMarketOnceAround => "Once Around",
        AuctionType::RicheseSilent => "Silent",
        AuctionType::RicheseOnceAround => "Once Around",
        _ => "None",
    }
    .to_string()
}

/// `private static string Describe(JuiceType jt)`
pub fn describe_juice_type(jt: JuiceType) -> String {
    match jt {
        JuiceType::GoFirst => "be considered first in storm order",
        JuiceType::GoLast => "be considered last in storm order",
        JuiceType::Aggressor => "be considered aggressor in this battle",
        _ => "None",
    }
    .to_string()
}

/// `private static string Describe(CaptureDecision c)`
pub fn describe_capture_decision(c: CaptureDecision) -> String {
    match c {
        CaptureDecision::Capture => "Keep",
        CaptureDecision::Kill => "Kill to gain 2 spice",
        CaptureDecision::DontCapture => "Neither keep nor kill",
        _ => "None",
    }
    .to_string()
}

