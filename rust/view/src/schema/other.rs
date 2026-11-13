//! Options of the "other events" forms (`actions/other.rs`). Each struct is the payload of one
//! or more [`ActionOptions`](super::ActionOptions) variants; the variant is named after the
//! event type.
//!
//! As in `schema/revival.rs`, every form carries `draft`: the event the form sends with its
//! current inputs, in savegame JSON, after the server has adjusted them the way the form does (a
//! required image choice or select falls back to its first value, a number picker clamps into
//! its range). The validation next to the options is that draft's.
//! Forms with a second button (`Pass`, `Other`) send the draft with the field that button sets.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::{FactionOption, HeroOption, RichText};

/// An enum value to pick in a select, radio or segmented control: `value` is its number, which
/// the event's JSON holds, `label` its `Skin.Describe`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct EnumOption {
    pub name: String,
    pub value: i32,
    pub label: String,
}

/// A form whose only inputs are its buttons (`ReplacedCardWon`, `RicheseKeepsUnsoldCard`,
/// `RicheseAnnouncesAuction`, `StormSpellPlayed`'s number).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DraftOptions {
    #[ts(type = "Record<string, unknown>")]
    pub draft: serde_json::Value,
}

/// The `MulliganPerformed` form (`MulliganPerformed.MayMulligan(Player)`) and
/// The `NexusCardDrawn` form (`NexusCardDrawn.MayDraw(Game, Player)`): without it only "No".
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct MayOptions {
    pub may: bool,
    #[ts(type = "Record<string, unknown>")]
    pub draft: serde_json::Value,
}

/// A form that picks one faction from `targets`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct FactionChoiceOptions {
    pub targets: Vec<FactionOption>,
    #[ts(type = "Record<string, unknown>")]
    pub draft: serde_json::Value,
}

/// A form that picks one treachery card (by id) from `cards`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CardChoiceOptions {
    pub cards: Vec<i32>,
    #[ts(type = "Record<string, unknown>")]
    pub draft: serde_json::Value,
}

/// A form that picks one hero from `heroes` (`TraitorsSelected`: `Player.Traitors`;
/// `TraitorDiscarded`: `TraitorDiscarded.ValidTraitors(Player)`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct HeroChoiceOptions {
    pub heroes: Vec<HeroOption>,
    #[ts(type = "Record<string, unknown>")]
    pub draft: serde_json::Value,
}

/// A form with a Yes and a No button.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct YesNoOptions {
    pub title: RichText,
    pub alert: bool,
    #[ts(type = "Record<string, unknown>")]
    pub draft: serde_json::Value,
}

/// What a bot ally tells its human ally, in the `AllyPermission` form.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct BotAllyClaims {
    pub owns_weapon: bool,
    pub owns_defense: bool,
    /// The ally's `KnownNonTraitors`, `Traitors` and `FaceDancers` of the player's faction.
    pub safe_leaders: Vec<HeroOption>,
    pub resources: i32,
}

/// The `AllyPermission` form. The draft starts from the permissions in force; `current` repeats
/// them for the "(currently: ...)" texts.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AllyPermissionOptions {
    #[ts(type = "Faction")]
    pub ally: String,
    pub bot_claims: Option<BotAllyClaims>,
    /// `Player.Resources`: the spice picker's maximum.
    pub max_resources: i32,
    /// `Game.GetPermittedUseOfAllyResources(Player.Ally)`.
    pub current_resources: i32,
    /// `Karma.ValidKarmaCards(Game, Player)` when `Karma.CanKarma(Game, Player)`, else null.
    pub karma_cards: Option<Vec<i32>>,
    /// `Game.GetPermittedUseOfAllyKarma(Player.Ally)`.
    pub current_karma_card: Option<i32>,
    /// The event in force: `Game.*` permissions as an `AllyPermission` (savegame JSON).
    #[ts(type = "Record<string, unknown>")]
    pub current: serde_json::Value,
    /// `Game.Applicable(Rule.FremenDeterminesStorm)` and
    /// `Game.Applicable(Rule.FremenAllyGetsDialedResourcesRefunded)`: Fremen's extra boxes.
    pub fremen_determines_storm: bool,
    pub fremen_refunds_dial: bool,
    #[ts(type = "Record<string, unknown>")]
    pub draft: serde_json::Value,
}

/// A rule of `Game.AutomationRules` of the player.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AutomationRule {
    /// `AutomationRuleId`, for the delete event.
    pub id: i32,
    /// `GetDescription()`.
    pub description: RichText,
}

/// The `AutomationConfigured` form (opened from the menu bar under the map).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AutomationConfiguredOptions {
    /// `AutomationConfigured.GetAvailableRuleTypes(Game, Player)`.
    pub rule_types: Vec<EnumOption>,
    /// `AutomationConfigured.GetValidBiddingFactions(Game, Player)`.
    pub bidding_factions: Vec<FactionOption>,
    pub rules: Vec<AutomationRule>,
    #[ts(type = "Record<string, unknown>")]
    pub draft: serde_json::Value,
}

/// The `BeneGesseritPrediction` form.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct BeneGesseritPredictionOptions {
    /// `BeneGesseritPrediction.ValidTargets(Game, Player)`.
    pub targets: Vec<FactionOption>,
    /// `Game.MaximumTurns`: the turn picker's maximum (its minimum is 1).
    pub max_turn: i32,
    #[ts(type = "Record<string, unknown>")]
    pub draft: serde_json::Value,
}

/// The `CardTraded` form.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CardTradedOptions {
    /// `CardTraded.ValidCards(Player)`.
    pub cards: Vec<i32>,
    /// With a bot ally, `CardTraded.ValidCards(Player.AlliedPlayer)`: the card to ask in return
    /// (not required). Null with a human ally.
    pub ally_cards: Option<Vec<i32>>,
    #[ts(type = "Record<string, unknown>")]
    pub draft: serde_json::Value,
}

/// The `ChoamEconomics` form: Double or Cancel.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ChoamEconomicsOptions {
    pub statuses: Vec<EnumOption>,
    #[ts(type = "Record<string, unknown>")]
    pub draft: serde_json::Value,
}

/// The `ChoamKarmaPrevention` form.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ChoamKarmaPreventionOptions {
    /// `ChoamKarmaPrevention.CardToUse(Player)`, drawn as rich text in the heading; with no card
    /// the heading says "Nexus Cunning".
    pub card: Option<RichText>,
    /// `Game.PlayersOtherThan(Player)`.
    pub targets: Vec<FactionOption>,
    #[ts(type = "Record<string, unknown>")]
    pub draft: serde_json::Value,
}

/// A deal type of the `DealOffered` form's select.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DealTypeOption {
    pub value: i32,
    /// `Deal.Express(DealType).ToString(Skin)`.
    pub label: String,
    /// `ApplicableEndPhases` for this type, for the "Until" control.
    pub end_phases: Vec<EnumOption>,
}

/// The `DealOffered` form (opened from the trade menu). The factions the offer goes to are the
/// form's own state: every target starts picked, and the event's `To` is empty when all are.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DealOfferedOptions {
    /// `Game.PlayersOtherThan(Player)` without the ally.
    pub targets: Vec<FactionOption>,
    /// `DealOffered.GetStandardDealTypes(Game, Player)`.
    pub deal_types: Vec<DealTypeOption>,
    #[ts(type = "Record<string, unknown>")]
    pub draft: serde_json::Value,
}

/// The `DivideResources` form: `DivideResources.GetResourcesToBeDivided(Game)` and what each
/// side gets when the player passes, for the draft's portion.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DivideResourcesOptions {
    pub amount: i32,
    pub territory: i32,
    #[ts(type = "Faction")]
    pub other_faction: String,
    /// `DivideResources.GainedByFirstFaction(tbd, false, portion)`.
    pub gained_if_passed: i32,
    /// `DivideResources.GainedByOtherFaction(tbd, false, portion)`.
    pub other_gains_if_passed: i32,
    #[ts(type = "Record<string, unknown>")]
    pub draft: serde_json::Value,
}

/// The `Donated` form (opened from the trade menu). Targets and range follow the draft's
/// `FromBank` (host only).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DonatedOptions {
    /// `Donated.ValidTargets(Game, Player)`, or every player when giving from the bank.
    pub targets: Vec<FactionOption>,
    /// `Donated.MinAmount(Game, Player)` and `Donated.MaxAmount(Player)`, or -99 and 99.
    pub min: i32,
    pub max: i32,
    /// `Game.Applicable(Rule.CardsCanBeTraded)`: the hand, to give one card (not required).
    pub cards: Option<Vec<i32>>,
    /// The viewer is a host: the "from bank" box.
    pub is_host: bool,
    #[ts(type = "Record<string, unknown>")]
    pub draft: serde_json::Value,
}

/// The `EmperorBidSupport` form: one picker per non-allied opponent, from 0 to `max`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct EmperorBidSupportOptions {
    pub targets: Vec<FactionOption>,
    /// `Player.Resources`.
    pub max: i32,
    #[ts(type = "Record<string, unknown>")]
    pub draft: serde_json::Value,
}

/// The `FactionTradeOffered` form.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct FactionTradeOfferedOptions {
    /// `FactionTradeOffered.ValidTargets(Game, Player)`.
    pub targets: Vec<FactionOption>,
    /// The targets some player holds: the button says "Offer Trade with" for these, "Pick"
    /// for the others.
    #[ts(type = "Array<Faction>")]
    pub in_play: Vec<String>,
    #[ts(type = "Record<string, unknown>")]
    pub draft: serde_json::Value,
}

/// The `Karma` form.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct KarmaOptions {
    /// `Karma.ValidKarmaCards(Game, Player)`.
    pub cards: Vec<i32>,
    /// `Karma.ValidFactionAdvantages(Game, Player)`.
    pub advantages: Vec<EnumOption>,
    #[ts(type = "Record<string, unknown>")]
    pub draft: serde_json::Value,
}

/// The `KarmaChoamDiscard` form: the Karma card picker is shown when there is more than one.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct KarmaChoamDiscardOptions {
    /// `Karma.ValidKarmaCards(Game, Player)`.
    pub karma_cards: Vec<i32>,
    /// `Player.TreacheryCards` without the draft's Karma card.
    pub cards: Vec<i32>,
    #[ts(type = "Record<string, unknown>")]
    pub draft: serde_json::Value,
}

/// The `KarmaHandSwap` form: return `number` cards to `target`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct KarmaHandSwapOptions {
    pub number: i32,
    #[ts(type = "Faction")]
    pub target: String,
    pub cards: Vec<i32>,
    #[ts(type = "Record<string, unknown>")]
    pub draft: serde_json::Value,
}

/// A form that picks a territory on the map (`KarmaMonster`: `KarmaMonster.ValidTargets`).
/// `locations` are the sectors of those territories, which the map outlines.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TerritoryChoiceOptions {
    pub territories: Vec<i32>,
    pub locations: Vec<i32>,
    #[ts(type = "Record<string, unknown>")]
    pub draft: serde_json::Value,
}

/// The `ResourcesTransferred` form.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ResourcesTransferredOptions {
    /// `ResourcesTransferred.MaxAmount(Player)`.
    pub max: i32,
    #[ts(type = "Record<string, unknown>")]
    pub draft: serde_json::Value,
}

/// The `SkillAssigned` form.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SkillAssignedOptions {
    /// `SkillAssigned.ValidSkills(Player)`.
    pub skills: Vec<EnumOption>,
    /// `SkillAssigned.ValidLeaders(Game, Player)`.
    pub leaders: Vec<HeroOption>,
    #[ts(type = "Record<string, unknown>")]
    pub draft: serde_json::Value,
}

/// The `DistransUsed` form.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DistransUsedOptions {
    /// `DistransUsed.ValidTargets(Game, Player)`.
    pub targets: Vec<FactionOption>,
    /// `DistransUsed.ValidCards(Player)`.
    pub cards: Vec<i32>,
    #[ts(type = "Record<string, unknown>")]
    pub draft: serde_json::Value,
}

/// The `RicheseAnnouncesBlackMarket` form.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct RicheseAnnouncesBlackMarketOptions {
    /// `RicheseAnnouncesBlackMarket.ValidCards(Player)`.
    pub cards: Vec<i32>,
    /// Normal, Once Around and Silent, with the form's labels.
    pub auction_types: Vec<EnumOption>,
    #[ts(type = "Record<string, unknown>")]
    pub draft: serde_json::Value,
}

/// The `RicheseSpecifiesAuction` form.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct RicheseSpecifiesAuctionOptions {
    /// `RicheseSpecifiesAuction.MaySpecifyCard(Game, Player)`.
    pub may_specify_card: bool,
    /// `RicheseSpecifiesAuction.MaySpecifyTypeOfAuction(Game, Player)`.
    pub may_specify_type: bool,
    /// `RicheseSpecifiesAuction.ValidCards(Game)`.
    pub cards: Vec<i32>,
    /// Once Around and Silent, with the form's labels.
    pub auction_types: Vec<EnumOption>,
    #[ts(type = "Record<string, unknown>")]
    pub draft: serde_json::Value,
}

/// The Tleilaxu Secret Ally part of the `NexusPlayed` form.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TleilaxuNexusOptions {
    /// `NexusPlayed.ValidTleilaxuMaxAmount(Game, Player, false/true)`.
    pub max_forces: i32,
    pub max_special_forces: i32,
    /// `Revival.NumberOfSpecialForcesThatMayBePlacedOnPlanet` for the draft's special forces,
    /// 0 without `Revival.ValidRevivedForceLocations`.
    pub max_special_forces_on_planet: i32,
    /// `Revival.ValidRevivedForceLocations(Game, Player)`, picked on the map.
    pub locations: Vec<i32>,
    /// `NexusPlayed.ValidTleilaxuHeroes(Game, Player)` (not required).
    pub heroes: Vec<HeroOption>,
    /// `Revival.MayAssignSkill(Game, Player, Hero)` for the draft's hero.
    pub may_assign_skill: bool,
    /// `NexusPlayed.DetermineTleilaxuCost(Forces, SpecialForces)` for the draft.
    pub cost: i32,
}

/// The `NexusPlayed` form. `text` is the line the form shows for the card, the phase and the
/// kind of use; the inputs below are set only for the cases that show them.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct NexusPlayedOptions {
    #[ts(type = "Faction")]
    pub nexus: String,
    /// `NexusTypeText`: "Cunning", "Secret Ally" or "Betrayal".
    pub kind: String,
    pub text: Option<RichText>,
    /// Atreides Cunning or Secret Ally: `Prescience.ValidAspects(Game, Player)`.
    pub aspects: Option<Vec<EnumOption>>,
    pub tleilaxu: Option<TleilaxuNexusOptions>,
    /// CHOAM Secret Ally in Collection: `NexusPlayed.ValidChoamCards(Player)`.
    pub choam_cards: Option<Vec<i32>>,
    /// Ecaz Secret Ally: `NexusPlayed.ValidEcazFactions(Game)`.
    pub ecaz_factions: Option<Vec<FactionOption>>,
    /// Ecaz Betrayal (`NexusPlayed.ValidEcazTerritories`) or Moritani Betrayal
    /// (`NexusPlayed.ValidMoritaniTerritories`): picked on the map; `locations` are their sectors.
    pub territories: Option<Vec<i32>>,
    pub locations: Option<Vec<i32>>,
    /// Which draft member the map pick sets: `EcazTerritory`, `MoritaniTerritory` or
    /// `TleilaxuLocation`; null while nothing is picked on the map.
    pub map_member: Option<String>,
    #[ts(type = "Record<string, unknown>")]
    pub draft: serde_json::Value,
}
