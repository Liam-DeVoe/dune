//! The data the web client's lobby and setup screens need outside any game view
//! (`web/public/static/lobby.json`, package 8 of client-plan.md).
//!
//! The lobby protocol mirrors the C# hub, so it carries enums as numbers (protocol.md,
//! Encoding), while the skin and the view name them. This file gives the client, per enum it
//! meets in the lobby, every member's name, number and `DefaultSkin` description. It also has
//! the static tables the C# lobby screens call: `EstablishPlayers.AvailableFactions`,
//! `GetMaximumNumberOfPlayers`, `GetMaximumNumberOfTurns`, `AvailableRuleGroups`,
//! `Game.GetAvailableRulesets`, `Game.RulesetDefinition`, `Game.GetRulesInGroup` and
//! `Game.LatestVersion`.
//!
//! Texts come from `DefaultSkin`, as the C# lobby shows them when no custom skin is stored.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use treachery_engine::enumerations::*;
use treachery_engine::messages::expression::{EnumValue, ExpressionElement};
use treachery_engine::game::Game;
use treachery_engine::compat::net_enum::NetEnum;
use treachery_engine::prelude::EstablishPlayers;
use ts_rs::TS;

/// One enum member.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
pub struct EnumMember {
    /// The C# member name, as the view and the skin JSON use it.
    pub name: String,
    /// The number the lobby protocol uses.
    pub value: i32,
    /// `Skin.Describe(value)` with the default skin.
    pub label: String,
}

/// See the module documentation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LobbyData {
    /// `Game.LatestVersion`: sent with `RequestLogin`, shown under the login form.
    pub latest_version: i32,
    /// `EstablishPlayers.GetMaximumNumberOfPlayers()`.
    pub maximum_players: i32,
    /// `EstablishPlayers.GetMaximumNumberOfTurns()`.
    pub maximum_turns: i32,
    /// Per enum type (`Faction`, `Ruleset`, `Rule`, `RuleGroup`, `MainPhase`, `Phase`,
    /// `ErrorType`, `UserStatus`, `SubscriptionType`), its members in `Enum.GetValues` order.
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

fn label<T: NetEnum>(v: T) -> String {
    treachery_skin::default_skin().describe(&ExpressionElement::Enum(EnumValue::of(v)))
}

fn members<T: NetEnum>() -> Vec<EnumMember> {
    members_with(label::<T>)
}

fn members_with<T: NetEnum>(label: impl Fn(T) -> String) -> Vec<EnumMember> {
    get_values::<T>()
        .map(|v| EnumMember { name: T::INFO.name_of(v.value()).unwrap_or_default().to_string(), value: v.value(), label: label(v) })
        .collect()
}

fn name<T: NetEnum>(v: T) -> String {
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
        // `Skin.Describe(ErrorType)` is its own overload, not the `Describe(object)` dispatch.
        ("ErrorType".to_string(), members_with(|e| treachery_skin::default_skin().describe_error_type(e))),
        ("UserStatus".to_string(), members::<UserStatus>()),
        ("SubscriptionType".to_string(), members::<SubscriptionType>()),
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
        rules_in_group: get_values::<RuleGroup>()
            .map(|g| (name(g), Game::get_rules_in_group(g, Game::EXPANSION_LEVEL).into_iter().map(name).collect()))
            .collect(),
        rule_group: get_values::<Rule>().map(|r| (name(r), name(Game::get_rule_group(r)))).collect(),
    }
}
