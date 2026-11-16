//! The data the web client's lobby and setup screens need outside any game view
//! (`web/public/static/lobby.json`).
//!
//! The lobby protocol carries enums as member names (protocol.md, Encoding), as the skin and the
//! view do. This file gives the client, per enum it meets in the lobby, every member's name,
//! number and `DefaultSkin` description. The numbers are for the game settings the browser
//! remembers, which it stores in the C# format. It also has the static tables the lobby screens
//! need: `EstablishPlayers.AvailableFactions`,
//! `GetMaximumNumberOfPlayers`, `GetMaximumNumberOfTurns`, `AvailableRuleGroups`,
//! `Game.GetAvailableRulesets`, `Game.RulesetDefinition`, `Game.GetRulesInGroup` and
//! `Game.LatestVersion`.
//!
//! Texts come from `DefaultSkin`, which the lobby shows when no custom skin is stored.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use treachery_engine::enumerations::*;
use treachery_engine::messages::part::PartEnum;
use treachery_engine::game::Game;
use treachery_engine::enum_type::{name_or_absent, value_or_absent, GameEnum};
use treachery_engine::prelude::EstablishPlayers;
use ts_rs::TS;

/// One enum member.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
pub struct EnumMember {
    /// The member name, as the view and the skin JSON use it.
    pub name: String,
    /// The C# member value.
    pub value: i32,
    /// `Skin.Describe(value)` with the default skin.
    pub label: String,
}

/// See the module documentation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LobbyData {
    /// `Game::LATEST_VERSION`, shown under the login form.
    pub latest_version: i32,
    /// `EstablishPlayers.GetMaximumNumberOfPlayers()`.
    pub maximum_players: i32,
    /// `EstablishPlayers.GetMaximumNumberOfTurns()`.
    pub maximum_turns: i32,
    /// Per enum type (`Faction`, `Ruleset`, `Rule`, `RuleGroup`, `MainPhase`, `Phase`,
    /// `UserStatus`, `SubscriptionType`), its members in `Enum.GetValues` order.
    pub enums: BTreeMap<String, Vec<EnumMember>>,
    /// `EstablishPlayers.AvailableFactions()`, names.
    #[ts(type = "Array<Faction>")]
    pub available_factions: Vec<String>,
    /// `Game.GetAvailableRulesets()`, names.
    #[ts(type = "Array<Ruleset>")]
    pub available_rulesets: Vec<String>,
    /// `Game.RulesetDefinition`: the rules of each ruleset, names.
    #[ts(type = "Record<string, Array<Rule>>")]
    pub ruleset_rules: BTreeMap<String, Vec<String>>,
    /// `EstablishPlayers.AvailableRuleGroups()`, names.
    #[ts(type = "Array<RuleGroup>")]
    pub available_rule_groups: Vec<String>,
    /// `Game.GetRulesInGroup(group, Game.ExpansionLevel)` for every group, names.
    #[ts(type = "Record<string, Array<Rule>>")]
    pub rules_in_group: BTreeMap<String, Vec<String>>,
    /// `Game.GetRuleGroup(rule)` for every rule.
    #[ts(type = "Record<string, RuleGroup>")]
    pub rule_group: BTreeMap<String, String>,
}

/// The skin's label of a member, or of the C# `None` member of a closed enum (`None`).
fn label<T: PartEnum>(v: Option<T>) -> String {
    treachery_skin::default_skin().describe(&T::part(v))
}

fn members<T: PartEnum>() -> Vec<EnumMember> {
    members_with(label::<T>)
}

/// Every member in value order, with the C# `None` member of a closed enum first (as `None`), as the
/// lobby always listed them.
fn members_with<T: GameEnum>(label: impl Fn(Option<T>) -> String) -> Vec<EnumMember> {
    with_absent::<T>().map(|v| EnumMember { name: name_or_absent(v), value: value_or_absent(v), label: label(v) }).collect()
}

fn with_absent<T: GameEnum>() -> impl Iterator<Item = Option<T>> {
    T::INFO.absent.then_some(None).into_iter().chain(get_values::<T>().map(Some))
}

fn name<T: GameEnum>(v: T) -> String {
    T::INFO.name_of(v.value()).unwrap_or_default().to_string()
}

/// Builds the lobby data from the engine and the default skin.
pub fn lobby_data() -> LobbyData {
    let enums = BTreeMap::from([
        ("Faction".to_string(), members::<Faction>()),
        ("Ruleset".to_string(), members::<Ruleset>()),
        ("Rule".to_string(), members::<Rule>()),
        ("RuleGroup".to_string(), members::<RuleGroup>()),
        ("MainPhase".to_string(), members::<MainPhase>()),
        ("Phase".to_string(), members::<Phase>()),
        ("UserStatus".to_string(), members_with(|v: Option<UserStatus>| treachery_skin::skin::describe_user_status(v.expect("UserStatus has no absent member")))),
        ("SubscriptionType".to_string(), members_with(|v: Option<SubscriptionType>| treachery_skin::skin::describe_subscription_type(v.expect("SubscriptionType has no absent member")))),
    ]);
    LobbyData {
        latest_version: Game::LATEST_VERSION,
        maximum_players: EstablishPlayers::get_maximum_number_of_players(),
        maximum_turns: EstablishPlayers::get_maximum_number_of_turns(),
        enums,
        available_factions: EstablishPlayers::available_factions().into_iter().map(name).collect(),
        available_rulesets: Game::get_available_rulesets().into_iter().map(name).collect(),
        ruleset_rules: Game::ruleset_definition().iter().map(|(rs, rules)| (name(*rs), rules.iter().map(|r| name(*r)).collect())).collect(),
        available_rule_groups: EstablishPlayers::available_rule_groups().into_iter().map(name).collect(),
        rules_in_group: with_absent::<RuleGroup>()
            .map(|g| (name_or_absent(g), Game::get_rules_in_group(g, Game::EXPANSION_LEVEL).into_iter().map(name).collect()))
            .collect(),
        rule_group: with_absent::<Rule>().map(|r| (name_or_absent(r), name_or_absent(r.and_then(Game::get_rule_group)))).collect(),
    }
}
