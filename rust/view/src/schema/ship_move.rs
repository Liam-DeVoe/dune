//! Payloads of the ship-and-move forms, the `ActionOptions` variants built in
//! `actions/ship_move.rs`. Location and territory ids are the engine's; a territory pick lists
//! every sector of each territory.
//!
//! As in the other areas, every form carries `draft`: the event the form sends with its current
//! inputs, in savegame JSON, after the server has adjusted the inputs the way the form does (a
//! No-Field, smuggled force, Karama card or ally payment that
//! no longer applies is dropped, the Bene Gesserit advisor flag follows the destination, a
//! required choice falls back to its first value). The validation next to the options is that
//! draft's. The client shows these values and sends its next draft from them.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::{HeroOption, RichText};

/// A location and the acting player's forces in it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ForcesAt {
    pub location: i32,
    /// `Player.ForcesIn(l)`.
    pub forces: i32,
    /// `Player.SpecialForcesIn(l)`.
    pub special_forces: i32,
}

/// A homeworld forces may be shipped from (`Shipment.HomeworldsToShipFrom`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ShipmentWorld {
    pub location: i32,
    /// `Name(world)`.
    pub label: String,
    /// In `HomeworldsToShipFrom(Player, false)`.
    pub normal: bool,
    /// In `HomeworldsToShipFrom(Player, true)`.
    pub special: bool,
    pub forces: i32,
    pub special_forces: i32,
}

/// A Karama card for a shipment at the Guild rate (`Shipment.ValidKarmaCards`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct KarmaChoice {
    pub card: i32,
    /// `Player.TreacheryCards.Contains(card)`; otherwise the ally's.
    pub own: bool,
}

/// The `Shipment` form, for the draft's shipment type, source, destination and No-Field.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ShipmentOptions {
    /// `Shipment.MayShipCrossPlanet(Game, Player)`.
    pub may_ship_cross_planet: bool,
    /// `Shipment.MayShipToReserves(Game, Player)`.
    pub may_ship_to_reserves: bool,
    /// Karama stops this (non-Fremen) faction from shipping, and it may not ship cross-planet.
    pub prevented_from_shipping: bool,
    /// `Shipment.ValidSourceLocations` when site-to-site or back-to-reserves shipping is allowed.
    pub sources: Vec<ForcesAt>,
    /// `Shipment.ValidShipmentLocations` for the draft's type (none for back to reserves).
    pub targets: Vec<i32>,
    /// The `Shipment::valid_max_*_shipment_forces` helper for the draft's type, No-Field and
    /// source.
    pub max_forces: i32,
    pub max_special_forces: i32,
    /// `MayShipHereFromReserves`: the draft's site-to-site source is a valid normal destination.
    pub may_ship_here_from_reserves: bool,
    /// `Shipment.MayUseRicheseSecretAlly`.
    pub may_use_richese_secret_ally: bool,
    /// `Shipment.MayUseNoField`, `ValidNoFieldValues`.
    pub may_use_no_field: bool,
    pub no_field_values: Vec<i32>,
    /// `Shipment.MayUseCunningNoField`, and `ValidCunningNoFieldValues` for the draft's No-Field.
    pub may_use_cunning_no_field: bool,
    pub cunning_no_field_values: Vec<i32>,
    /// `Shipment.DefaultNoFieldSpecialForceAmount(Faction, NoFieldValue)` for the draft's No-Field.
    pub default_no_field_special_force_amount: i32,
    /// `Game.HasHighThreshold(Faction.Richese)`.
    pub richese_high_threshold: bool,
    /// `HomeworldsToShipFrom(Player, false)` and `(Player, true)`, merged.
    pub homeworlds: Vec<ShipmentWorld>,
    /// `Shipment.MaySmuggle(Game, Player, To)` for the draft's destination.
    pub may_smuggle: bool,
    /// Karama may pay for the shipment: `Shipment::can_karma`, not Fremen unless site-to-site,
    /// and no cross-planet shipping.
    pub may_karma: bool,
    pub karma_cards: Vec<KarmaChoice>,
    /// `Shipment.DetermineCost(Game, Player, draft)`.
    pub cost: i32,
    /// `Game.ResourcesYourAllyCanPay(Player)` when there is an ally and a cost, else 0.
    pub ally_can_pay: i32,
    /// `Game.ContainsConflictingAlly(Player, destination)`.
    pub conflicting_ally: bool,
    /// `Game.Map.TheGreatFlat.Territory`, for the Fremen text.
    pub great_flat: i32,
    /// `Skin.Describe(TreacheryCardType.Karma)`.
    pub karma_name: String,
    /// The draft with the inputs as the form shows them (see the module docs).
    #[ts(type = "Record<string, unknown>")]
    pub draft: serde_json::Value,
}

/// The placement forms (`Move`, `Caravan` and `FremenRidesMonster`), for the draft's forces and
/// destination.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PlacementOptions {
    /// The heading of the `FremenRidesMonster` form; Move and Caravan have plain text headers.
    pub title: Option<RichText>,
    /// The sectors that may be clicked as a source (`SelectableSources`), with the player's
    /// forces in each.
    pub sources: Vec<ForcesAt>,
    /// `ValidTargets` for the draft's forces: none until some are selected.
    pub targets: Vec<i32>,
    /// `Game.CurrentPlanetology.MoveFromTwoTerritories` played by this player.
    pub may_move_from_two_territories: bool,
    /// `InformAboutCaravan`.
    pub inform_about_caravan: bool,
    /// `FromReserves` (the Great Maker), `MaxFromReserves(false)`, `MaxFromReserves(true)`.
    pub from_reserves: bool,
    pub max_from_reserves: i32,
    pub max_special_from_reserves: i32,
    /// Bene Gesserit choose between fighters and advisors at the draft's destination; when they
    /// can't, `asAdvisors` is the value the form forces.
    pub advisor_choice: bool,
    pub as_advisors: bool,
    /// `Game.ContainsConflictingAlly(Player, To)`.
    pub conflicting_ally: bool,
    /// The draft with the inputs as the form shows them (see the module docs).
    #[ts(type = "Record<string, unknown>")]
    pub draft: serde_json::Value,
}

/// The `PerformFremenSetup` form, which stages forces location by location.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct StagedPlacementOptions {
    /// `ValidLocations`.
    pub targets: Vec<i32>,
    /// `MaxTotal`.
    pub max_total: i32,
    /// `MayStageSpecial`: `Placer.HasSpecialForces`.
    pub may_stage_special: bool,
    pub forces_in_reserve: i32,
    pub special_forces_in_reserve: i32,
    /// `Game.Map.SietchTabr`, `FalseWallSouth`, `FalseWallWest` for the header.
    pub places: Vec<RichText>,
    /// The draft with the inputs as the form shows them (see the module docs).
    #[ts(type = "Record<string, unknown>")]
    pub draft: serde_json::Value,
}

/// The `DiscoveryEntered` form.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryEnteredOptions {
    /// `DiscoveryEntered.ValidTargets(Game, Player)`.
    pub targets: Vec<i32>,
    /// `DiscoveryEntered.ValidSources(Player, To)` for the draft's destination.
    pub sources: Vec<ForcesAt>,
    /// The draft with the inputs as the form shows them (see the module docs).
    #[ts(type = "Record<string, unknown>")]
    pub draft: serde_json::Value,
}

/// The `AmbassadorActivated` form.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AmbassadorActivatedOptions {
    /// `AmbassadorActivated.GetAmbassador(Game)`, `GetVictim(Game)`.
    #[ts(type = "Ambassador")]
    pub ambassador: String,
    #[ts(type = "Faction")]
    pub victim: String,
    #[ts(type = "Array<Ambassador>")]
    pub bene_gesserit_ambassadors: Vec<String>,
    pub choam_cards: Vec<i32>,
    pub ixian_cards: Vec<i32>,
    pub vidal_can_be_taken: bool,
    pub alliance_can_be_offered: bool,
    pub vidal_can_be_offered_to_new_ally: bool,
    /// `Game.Vidal`.
    pub vidal: Option<i32>,
    /// `FremenSources`: sectors of `ValidFremenSources` holding forces, out of the storm.
    pub fremen_sources: Vec<ForcesAt>,
    pub fremen_targets: Vec<i32>,
    pub guild_targets: Vec<i32>,
    pub guild_max_forces: i32,
    pub tleilaxu_max_amount: i32,
    pub tleilaxu_heroes: Vec<HeroOption>,
    /// `Revival.MayAssignSkill(Game, Player, TleilaxuHero)` for the draft's hero.
    pub may_assign_skill: bool,
    pub may_pass: bool,
    /// The draft with the inputs as the form shows them (see the module docs).
    #[ts(type = "Record<string, unknown>")]
    pub draft: serde_json::Value,
}

/// The `AmbassadorPlaced` form.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AmbassadorPlacedOptions {
    #[ts(type = "Array<Ambassador>")]
    pub ambassadors: Vec<String>,
    /// The sectors of `AmbassadorPlaced.ValidStrongholds`.
    pub targets: Vec<i32>,
    /// `Game.AmbassadorsPlacedThisTurn + 1`.
    pub cost: i32,
    /// The draft with the inputs as the form shows them (see the module docs).
    #[ts(type = "Record<string, unknown>")]
    pub draft: serde_json::Value,
}

/// The `BeneGesseritAccompanies` form.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct BeneGesseritAccompaniesOptions {
    pub targets: Vec<i32>,
    /// `BeneGesseritAccompanies.MaySendExtraAdvisor(Game, Player, Location)` for the draft's location.
    pub may_send_extra_advisor: bool,
    /// The draft with the inputs as the form shows them (see the module docs).
    #[ts(type = "Record<string, unknown>")]
    pub draft: serde_json::Value,
}

/// Forms that pick one place on the map and nothing else.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LocationPickOptions {
    /// The locations that may be clicked; for a territory pick, every sector of each territory.
    pub targets: Vec<i32>,
    /// The draft with the inputs as the form shows them (see the module docs).
    #[ts(type = "Record<string, unknown>")]
    pub draft: serde_json::Value,
}

/// The `PerformHmsMovement` and `KarmaHmsMovement` forms.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct HmsMovementOptions {
    pub targets: Vec<i32>,
    /// `Game.HmsMovesLeft` or `Game.KarmaHmsMovesLeft`.
    pub moves_left: i32,
    /// The draft with the inputs as the form shows them (see the module docs).
    #[ts(type = "Record<string, unknown>")]
    pub draft: serde_json::Value,
}

/// The `ChoamMovePrevention` form.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ChoamMovePreventionOptions {
    /// `ChoamMovePrevention.CardToUse(Player)`; none means Nexus Cunning.
    pub card: Option<i32>,
    /// The sectors of `ChoamMovePrevention.ValidTerritories(Player)`.
    pub targets: Vec<i32>,
    /// The draft with the inputs as the form shows them (see the module docs).
    #[ts(type = "Record<string, unknown>")]
    pub draft: serde_json::Value,
}

/// A faction whose force CHOAM may remove, with `ChoamRemoveForce.ValidSpecialForceChoices`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ChoamRemoveForceFaction {
    #[ts(type = "Faction")]
    pub faction: String,
    pub special_choices: Vec<bool>,
}

/// The `ChoamRemoveForce` form.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ChoamRemoveForceOptions {
    pub card: Option<i32>,
    /// `ChoamRemoveForce.ValidLocations(Game)`.
    pub targets: Vec<i32>,
    /// `ChoamRemoveForce.ValidFactions(Game, Player, Location)` for the draft's location, each with
    /// its force choices there.
    pub factions: Vec<ChoamRemoveForceFaction>,
    /// The draft with the inputs as the form shows them (see the module docs).
    #[ts(type = "Record<string, unknown>")]
    pub draft: serde_json::Value,
}

/// A discovery the player may reveal (`DiscoveryRevealed.GetLocations`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct RevealableDiscovery {
    pub location: i32,
    pub territory: i32,
    #[ts(type = "DiscoveryToken")]
    pub token: String,
}

/// The `DiscoveryRevealed` form.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryRevealedOptions {
    pub locations: Vec<RevealableDiscovery>,
    /// The draft with the inputs as the form shows them (see the module docs).
    #[ts(type = "Record<string, unknown>")]
    pub draft: serde_json::Value,
}

/// The `KarmaShipmentPrevention` form, and the targets of the `SetShipmentPermission` form.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct FactionTargetsOptions {
    #[ts(type = "Array<Faction>")]
    pub targets: Vec<String>,
    /// The draft with the inputs as the form shows them (see the module docs).
    #[ts(type = "Record<string, unknown>")]
    pub draft: serde_json::Value,
}

/// One row of `Game.ShipmentPermissions`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ShipmentPermissionRow {
    #[ts(type = "Faction")]
    pub faction: String,
    pub cross: bool,
    pub to_homeworld: bool,
    pub guild_rate: bool,
}

/// The `SetShipmentPermission` form.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SetShipmentPermissionOptions {
    pub permissions: Vec<ShipmentPermissionRow>,
    /// `SetShipmentPermission.ValidTargets(Game, Player)`.
    #[ts(type = "Array<Faction>")]
    pub targets: Vec<String>,
    /// The draft with the inputs as the form shows them (see the module docs).
    #[ts(type = "Record<string, unknown>")]
    pub draft: serde_json::Value,
}

/// The `TerrorPlanted` form.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TerrorPlantedOptions {
    /// `TerrorPlanted.MayRemoveTokens(Game, Player)`.
    pub may_remove: bool,
    /// `ValidTerrorTypes(Game, true)` and `(Game, false)`.
    #[ts(type = "Array<TerrorType>")]
    pub remove_types: Vec<String>,
    #[ts(type = "Array<TerrorType>")]
    pub place_types: Vec<String>,
    /// `Game.UnplacedTerrorTokens`, for "Place token" or "Move token".
    #[ts(type = "Array<TerrorType>")]
    pub unplaced: Vec<String>,
    /// The sectors of `TerrorPlanted.ValidStrongholds`.
    pub targets: Vec<i32>,
    /// The draft with the inputs as the form shows them (see the module docs).
    #[ts(type = "Record<string, unknown>")]
    pub draft: serde_json::Value,
}

/// The `TerrorRevealed` form.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TerrorRevealedOptions {
    #[ts(type = "Faction")]
    pub victim: String,
    pub territory: Option<i32>,
    #[ts(type = "Array<TerrorType>")]
    pub types: Vec<String>,
    pub may_offer_alliance: bool,
    pub may_pass: bool,
    pub sneak_attack_targets: Vec<i32>,
    pub max_sneak_attack_forces: i32,
    /// `Player.TreacheryCards`, to give in a sabotage.
    pub cards: Vec<i32>,
    /// The draft with the inputs as the form shows them (see the module docs).
    #[ts(type = "Record<string, unknown>")]
    pub draft: serde_json::Value,
}

/// The `BeneGesseritFlip` form.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct BeneGesseritFlipOptions {
    /// `BeneGesseritFlip.GetTerritory(Game)`.
    pub territory: i32,
    /// The draft with the inputs as the form shows them (see the module docs).
    #[ts(type = "Record<string, unknown>")]
    pub draft: serde_json::Value,
}

/// Forms whose choices are fixed in the form (`FlightUsed`, `TestingStationUsed`): only the
/// validation varies.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct NoOptions {
    /// The draft with the inputs as the form shows them (see the module docs).
    #[ts(type = "Record<string, unknown>")]
    pub draft: serde_json::Value,
}
