//! Options of the eight revival forms (`actions/revival.rs`). Each struct is the payload of one
//! [`ActionOptions`](super::ActionOptions) variant, named after the event type.
//!
//! Every form carries `draft`: the event the form sends with its current inputs, in savegame
//! JSON, after the server has adjusted the inputs the way the form does
//! (a number picker clamps its value into its range, a hidden picker resets to 0, a required
//! image choice falls back to the first value, the map pick is dropped while no forces go to the
//! planet). The validation next to the options is that draft's. The client shows these values and
//! sends its next draft from them.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::{HeroRef, RichText};

/// A hero to pick in a form: `id` is the value of the event's `_heroId`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct HeroOption {
    pub id: i32,
    pub hero: HeroRef,
}

/// A faction to pick in a form: `value` is its number, which the event's JSON holds.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct FactionOption {
    #[ts(type = "Faction")]
    pub faction: String,
    pub value: i32,
}

/// The `Revival` form.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct RevivalOptions {
    /// `Player.Nexus == Emperor && NexusPlayed.CanUseSecretAlly(Game, Player)`: the checkbox
    /// for 3 extra free revivals.
    pub may_use_secret_ally: bool,
    /// `Revival.ValidMaxRevivals(Game, Player, false, UsesEmperorSecretAlly)`.
    pub max_forces: i32,
    /// `Revival.ValidMaxRevivals(Game, Player, true, UsesEmperorSecretAlly)`.
    pub max_special_forces: i32,
    /// `Revival.ValidMaxRevivalsByEmperor(Game, Player)`.
    pub max_by_emperor: i32,
    /// `Revival.ValidRevivedForceLocations(Game, Player)`: where revived forces may go on the map.
    pub locations: Vec<i32>,
    /// `Revival.NumberOfForcesThatMayBePlacedOnPlanet` for the forces chosen, 0 without locations.
    pub max_forces_on_planet: i32,
    /// `Revival.NumberOfSpecialForcesThatMayBePlacedOnPlanet` for the special forces chosen, 0
    /// without locations.
    pub max_special_forces_on_planet: i32,
    /// `Revival.ValidRevivalHeroes(Game, Player)`.
    pub heroes: Vec<HeroOption>,
    /// `Revival.MayAssignSkill(Game, Player, Hero)` for the chosen hero.
    pub may_assign_skill: bool,
    /// `Revival.DetermineCost(...).TotalCostForPlayer` for the draft.
    pub cost_for_player: i32,
    /// `Revival.DetermineCost(...).CostForEmperor`, when the player's ally is the Emperor.
    pub cost_for_emperor: Option<i32>,
    #[ts(type = "Record<string, unknown>")]
    pub draft: serde_json::Value,
}

/// The `RaiseDeadPlayed` form.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct RaiseDeadPlayedOptions {
    /// `Skin.Describe(TreacheryCardType.RaiseDead)`, for the heading and the button.
    pub card_name: String,
    /// `RaiseDeadPlayed.ValidMaxAmount(Game, Player, false)`.
    pub max_forces: i32,
    /// `RaiseDeadPlayed.ValidMaxAmount(Game, Player, true)`.
    pub max_special_forces: i32,
    /// `Revival.ValidRevivedForceLocations(Game, Player)`.
    pub locations: Vec<i32>,
    /// `Revival.NumberOfSpecialForcesThatMayBePlacedOnPlanet` for the special forces chosen, 0
    /// without locations.
    pub max_special_forces_on_planet: i32,
    /// `RaiseDeadPlayed.ValidHeroes(Game, Player)`.
    pub heroes: Vec<HeroOption>,
    /// `Revival.MayAssignSkill(Game, Player, Hero)` for the chosen hero.
    pub may_assign_skill: bool,
    #[ts(type = "Record<string, unknown>")]
    pub draft: serde_json::Value,
}

/// The `KarmaFreeRevival` form.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct KarmaFreeRevivalOptions {
    /// `Skin.Describe(TreacheryCardType.Karma)`, for the heading and the button.
    pub card_name: String,
    /// `KarmaFreeRevival.ValidMaxAmount(Player, false)`.
    pub max_forces: i32,
    /// `KarmaFreeRevival.ValidMaxAmount(Player, true)`.
    pub max_special_forces: i32,
    /// `Game.KilledHeroes(Player)`.
    pub heroes: Vec<HeroOption>,
    /// `Revival.MayAssignSkill(Game, Player, Hero)` for the chosen hero.
    pub may_assign_skill: bool,
    #[ts(type = "Record<string, unknown>")]
    pub draft: serde_json::Value,
}

/// The `KarmaRevivalPrevention` form.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct KarmaRevivalPreventionOptions {
    /// `Skin.Describe(TreacheryCardType.Karma)`, for the heading.
    pub card_name: String,
    /// `KarmaRevivalPrevention.GetValidTargets(Game, Player)`.
    pub targets: Vec<FactionOption>,
    #[ts(type = "Record<string, unknown>")]
    pub draft: serde_json::Value,
}

/// The `ChoamFreeRevivalPrevention` form.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ChoamFreeRevivalPreventionOptions {
    /// `ChoamFreeRevivalPrevention.CardToUse(Player)`, drawn as rich text in the heading; with
    /// no card the heading says "Nexus Cunning".
    pub card: Option<RichText>,
    /// `Game.PlayersOtherThan(Player)`.
    pub targets: Vec<FactionOption>,
    #[ts(type = "Record<string, unknown>")]
    pub draft: serde_json::Value,
}

/// The `RequestTleilaxuRevival` form.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct RequestTleilaxuRevivalOptions {
    /// `RequestTleilaxuRevival.ValidTargets(Game, Player)`.
    pub heroes: Vec<HeroOption>,
    #[ts(type = "Record<string, unknown>")]
    pub draft: serde_json::Value,
}

/// An entry of `Game.EarlyRevivalsOffers`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct EarlyRevivalOffer {
    pub hero: HeroOption,
    /// The price, or `None` for a denial (`int.MaxValue`).
    pub price: Option<i32>,
}

/// The `AcceptOrCancelTleilaxuRevival` form.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AcceptOrCancelTleilaxuRevivalOptions {
    /// The heroes of `Game.CurrentRevivalRequests`. While there are any, the card is urgent.
    pub requests: Vec<HeroOption>,
    /// Some request comes from a bot (`req.Player.IsBot`).
    pub bot_awaits: bool,
    /// `Game.EarlyRevivalsOffers`.
    pub offers: Vec<EarlyRevivalOffer>,
    /// `Skin.Format("Offer {0} for {1}", Hero, Price)` for the draft.
    pub offer_label: String,
    /// `Skin.Format("Deny revival of {0}", Hero)` for the draft.
    pub deny_label: String,
    #[ts(type = "Record<string, unknown>")]
    pub draft: serde_json::Value,
}

/// The `SetIncreasedRevivalLimits` form.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SetIncreasedRevivalLimitsOptions {
    /// `SetIncreasedRevivalLimits.ValidTargets(Game, Player)`.
    pub targets: Vec<FactionOption>,
    #[ts(type = "Record<string, unknown>")]
    pub draft: serde_json::Value,
}
