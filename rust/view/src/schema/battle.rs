//! Payloads of the battle action forms (client-plan.md, package 7b), built in
//! `actions/battle.rs`. Option values that go into the event (enum members, ids) are the numbers
//! of the savegame format, with the member name for images and the skin's label for text.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::{HeroRef, RichText};

/// An enum member offered by a form: `value` goes into the event, `name` picks images,
/// `label` is `Skin.Describe` of the member.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
pub struct BattleFormEnum {
    pub value: i32,
    pub name: String,
    pub label: String,
}

/// A `ClairVoyancePlayed` question parameter: the string the event stores, and its label.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
pub struct BattleFormParam {
    pub value: String,
    pub label: String,
}

/// A hero offered by a form, with the id the event stores.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
pub struct BattleFormHero {
    pub id: i32,
    pub hero: HeroRef,
    pub label: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AuditCancelledOptions {
    /// `AuditCancelled.Cost(Game)`.
    pub cost: i32,
    /// `AuditCancelled.GetNumberOfCardsThatMayBeAudited(Game)`.
    pub cards: i32,
    /// The event the form sends with its current inputs, in savegame JSON: the default draft in
    /// a view, the adjusted draft in a query answer.
    #[ts(type = "Record<string, unknown>")]
    pub draft: serde_json::Value,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AuditedOptions {
    /// `Game.AuditedCards`: the cards CHOAM looks at.
    pub cards: Vec<i32>,
    /// The event the form sends with its current inputs, in savegame JSON: the default draft in
    /// a view, the adjusted draft in a query answer.
    #[ts(type = "Record<string, unknown>")]
    pub draft: serde_json::Value,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct BattleInitiatedFight {
    pub faction: BattleFormEnum,
    pub territory: i32,
    pub territory_label: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct BattleInitiatedOptions {
    /// `Battle.BattlesToBeFought(Game, Player)`. With one fight, the C# form confirms it after
    /// half a second.
    pub fights: Vec<BattleInitiatedFight>,
    /// The event the form sends with its current inputs, in savegame JSON: the default draft in
    /// a view, the adjusted draft in a query answer.
    #[ts(type = "Record<string, unknown>")]
    pub draft: serde_json::Value,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct BeneGesseritBattleAnnouncementOptions {
    /// `BeneGesseritBattleAnnouncement.ValidTerritories(Game, Player)`, picked on the map.
    pub territories: Vec<i32>,
    /// Their sectors (`SectorsOf`): what the map outlines.
    pub locations: Vec<i32>,
    /// The event the form sends with its current inputs, in savegame JSON: the default draft in
    /// a view, the adjusted draft in a query answer.
    #[ts(type = "Record<string, unknown>")]
    pub draft: serde_json::Value,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CardsDeterminedOptions {
    /// `Game.TreacheryDeck.Items`. Under custom decks the deck is not shuffled before this
    /// event, so the order is the construction order and reveals nothing.
    pub treachery_cards: Vec<i32>,
    /// `Game.RicheseCache`.
    pub richese_cards: Vec<i32>,
    /// The event the form sends with its current inputs, in savegame JSON: the default draft in
    /// a view, the adjusted draft in a query answer.
    #[ts(type = "Record<string, unknown>")]
    pub draft: serde_json::Value,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ClairVoyanceAnsweredOptions {
    /// `Game.LatestClairvoyance.Initiator`.
    #[ts(type = "Faction")]
    pub questioner: String,
    /// `Game.LatestClairvoyance.GetQuestion()`.
    pub question: RichText,
    /// The event the form sends with its current inputs, in savegame JSON: the default draft in
    /// a view, the adjusted draft in a query answer.
    #[ts(type = "Record<string, unknown>")]
    pub draft: serde_json::Value,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ClairVoyancePlayedOptions {
    /// `ClairVoyancePlayed.ValidTargets(Game, Player)`.
    pub targets: Vec<BattleFormEnum>,
    /// `ClairVoyancePlayed.ValidQuestions(Game, target)` for the draft's target, labelled with
    /// `ClairVoyancePlayed.Express(q)`.
    pub questions: Vec<BattleFormEnum>,
    /// `Voice.ValidTypes(Game)`, for the card type questions.
    pub card_types: Vec<BattleFormParam>,
    /// `Game.TraitorsInPlay` by name, for the leader questions.
    pub leaders: Vec<BattleFormParam>,
    /// `Game.Players` factions, for the prediction question.
    pub factions: Vec<BattleFormParam>,
    /// `BeneGesseritPrediction.ValidTurns(Game)`.
    pub turns: Vec<BattleFormParam>,
    /// `Game.Map.Territories(true)`, strongholds first, then by name.
    pub territories: Vec<BattleFormParam>,
    /// The event the form sends with its current inputs, in savegame JSON: the default draft in
    /// a view, the adjusted draft in a query answer.
    #[ts(type = "Record<string, unknown>")]
    pub draft: serde_json::Value,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct BattleCardsOptions {
    pub cards: Vec<i32>,
    /// The event the form sends with its current inputs, in savegame JSON: the default draft in
    /// a view, the adjusted draft in a query answer.
    #[ts(type = "Record<string, unknown>")]
    pub draft: serde_json::Value,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct FaceDancedSource {
    pub location: i32,
    /// `Player.ForcesIn(location)` and `SpecialForcesIn(location)`.
    pub forces: i32,
    pub special_forces: i32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct FaceDancedOptions {
    /// `Game.Version > 150`: the form only places forces; before, it also asks whether to reveal.
    pub place_only: bool,
    /// `FaceDanced.MayCallFaceDancer(Game, Player)` (forms before version 151).
    pub may_call: bool,
    /// `FaceDanced.MaximumNumberOfForces(Game, Player)`.
    pub maximum_forces: i32,
    pub forces_in_reserve: i32,
    /// `Game.CurrentBattle.Territory`.
    pub territory: i32,
    /// `FaceDanced.ValidSourceLocations(Game, Player)` with the player's forces there.
    pub sources: Vec<FaceDancedSource>,
    /// `FaceDanced.ValidTargetLocations(Game)` with the player's forces there.
    pub targets: Vec<FaceDancedSource>,
    /// The event the form sends with its current inputs, in savegame JSON: the default draft in
    /// a view, the adjusted draft in a query answer.
    #[ts(type = "Record<string, unknown>")]
    pub draft: serde_json::Value,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct BattleHeroesOptions {
    pub heroes: Vec<BattleFormHero>,
    /// The event the form sends with its current inputs, in savegame JSON: the default draft in
    /// a view, the adjusted draft in a query answer.
    #[ts(type = "Record<string, unknown>")]
    pub draft: serde_json::Value,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct BattleMayCallOptions {
    pub may_call: bool,
    /// The event the form sends with its current inputs, in savegame JSON: the default draft in
    /// a view, the adjusted draft in a query answer.
    #[ts(type = "Record<string, unknown>")]
    pub draft: serde_json::Value,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct BattleChoicesOptions {
    pub choices: Vec<BattleFormEnum>,
    /// The event the form sends with its current inputs, in savegame JSON: the default draft in
    /// a view, the adjusted draft in a query answer.
    #[ts(type = "Record<string, unknown>")]
    pub draft: serde_json::Value,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LoserConcludedAssassination {
    /// `LoserConcluded.TargetOfAssassination(Game, Player)`.
    pub target: i32,
    /// `LoserConcluded.AssassinationReward(Game, winner, target)`; null when the leader is in
    /// the tanks (or there is no winner).
    pub reward: Option<i32>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LoserConcludedOptions {
    /// `LoserConcluded.CardsLoserMayKeep(Game)`.
    pub cards_to_keep: Vec<i32>,
    /// When `LoserConcluded.CanAssassinate(Game, Player)`.
    pub assassination: Option<LoserConcludedAssassination>,
    /// `LoserConcluded.MayUseKarmaToForceKeptCards(Game, Player)`.
    pub may_use_karma: bool,
    /// `LoserConcluded.CardsThatMayBeKeptOrDiscarded(Game)`, when Karma may be used.
    pub cards_kept_or_discarded: Vec<i32>,
    /// The event the form sends with its current inputs, in savegame JSON: the default draft in
    /// a view, the adjusted draft in a query answer.
    #[ts(type = "Record<string, unknown>")]
    pub draft: serde_json::Value,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PrescienceOptions {
    /// `Prescience.ValidAspects(Game, Player)`.
    pub aspects: Vec<BattleFormEnum>,
    /// Bene Gesserit is in the battle: "Voice should be used before Prescience".
    pub voice_note: bool,
    /// The event the form sends with its current inputs, in savegame JSON: the default draft in
    /// a view, the adjusted draft in a query answer.
    #[ts(type = "Record<string, unknown>")]
    pub draft: serde_json::Value,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct RetreatOptions {
    /// `Retreat.ValidTargets(Game, Player)`, picked on the map.
    pub targets: Vec<i32>,
    /// `Retreat.MaxForces`, `MaxSpecialForces`, `MaxTotalForces`.
    pub max_forces: i32,
    pub max_special_forces: i32,
    pub max_total_forces: i32,
    /// The event the form sends with its current inputs, in savegame JSON: the default draft in
    /// a view, the adjusted draft in a query answer.
    #[ts(type = "Record<string, unknown>")]
    pub draft: serde_json::Value,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TakeLossesOptions {
    /// `TakeLosses.LossesToTake(Game)`.
    pub amount: i32,
    pub location: Option<i32>,
    /// `TakeLosses.ValidMaxForceAmount` and `ValidMaxSpecialForceAmount`.
    pub max_forces: i32,
    pub max_special_forces: i32,
    /// CHOAM with `TakeLosses.CanPreventLosses`: `ValidUselessCardToPreventLosses`.
    pub useless_card: Option<i32>,
    /// The label of that card (`Skin.Format("Use {0} to prevent losses", card)`).
    pub useless_label: Option<String>,
    /// The event the form sends with its current inputs, in savegame JSON: the default draft in
    /// a view, the adjusted draft in a query answer.
    #[ts(type = "Record<string, unknown>")]
    pub draft: serde_json::Value,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct VoiceOptions {
    /// `Voice.ValidTypes(Game)` by name.
    pub types: Vec<BattleFormEnum>,
    /// Atreides is in the battle: "Voice should be used before Prescience".
    pub prescience_note: bool,
    /// The event the form sends with its current inputs, in savegame JSON: the default draft in
    /// a view, the adjusted draft in a query answer.
    #[ts(type = "Record<string, unknown>")]
    pub draft: serde_json::Value,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct BattleConcludedCapture {
    /// `Game.HarkonnenVictim`.
    pub victim: i32,
    /// `BattleConcluded.ValidCaptureDecisions(Game)`.
    pub decisions: Vec<BattleFormEnum>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct BattleConcludedOptions {
    /// `Faction == Game.BattleWinner`: only the winner gets the choices.
    pub winner: bool,
    /// `Battle.DetermineForceSupplier(Game, Player)`.
    #[ts(type = "Faction")]
    pub force_supplier: String,
    /// `BattleConcluded.ValidMaxReplacementForceAmount(Game, supplier)`.
    pub max_replacement_forces: i32,
    /// With `BattleConcluded.MayChooseToDiscardCards(Game)`: the winner's mercenary, weapon and
    /// defense still in hand.
    pub discardable_mercenary: Option<i32>,
    pub discardable_weapon: Option<i32>,
    pub discardable_defense: Option<i32>,
    /// When `BattleConcluded.MayCaptureOrKill(Game, Player)`.
    pub capture: Option<BattleConcludedCapture>,
    /// The loser's tech tokens.
    pub tech_tokens: Vec<BattleFormEnum>,
    /// `Game.TraitorsDeciphererCanLookAt`.
    pub decipherer_traitors: Vec<BattleFormHero>,
    /// `Game.DeciphererMayReplaceTraitor`: `BattleConcluded.ValidTraitorsToReplace(Player)`.
    pub traitors_to_replace: Option<Vec<BattleFormHero>>,
    /// `BattleConcluded.MayAddExtraForce(Game, Player)`: the battle's territory.
    pub add_extra_force_in: Option<i32>,
    /// The event the form sends with its current inputs, in savegame JSON: the default draft in
    /// a view, the adjusted draft in a query answer.
    #[ts(type = "Record<string, unknown>")]
    pub draft: serde_json::Value,
}

/// One side of the battle at the top of the form.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct BattleSide {
    #[ts(type = "Faction")]
    pub faction: String,
    pub aggressor: bool,
    /// The locations of the player's forces on the planet (hover tint).
    pub locations: Vec<i32>,
    pub groups: Vec<BattleForceGroup>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct BattleForceGroup {
    #[ts(type = "Faction")]
    pub faction: String,
    pub forces: i32,
    pub special_forces: i32,
    /// The opponent's Richese special forces are drawn without their number.
    pub hide_special_count: bool,
}

/// A note above the battle plan, in the C# order.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "t", rename_all = "camelCase")]
pub enum BattleNote {
    /// Ecaz forces add to the opponent's dial.
    EcazOpponent { amount: i32 },
    /// The opponent has no leader for this battle.
    OpponentNoLeaders,
    /// The current Voice: `mine` when the viewer's side used it.
    #[serde(rename_all = "camelCase")]
    Voice { mine: bool, must: bool, card_type: String },
    /// A part of the opponent's plan seen by prescience or Karama. `shown` is the card or leader
    /// (null for none); `dial` is set for the dial.
    #[serde(rename_all = "camelCase")]
    Seen { aspect: String, dial: Option<String>, shown: Option<RichText> },
    /// The opponent hasn't confirmed their plan yet. `aspect` is null under Karama.
    Later { aspect: Option<String> },
    /// Ecaz forces add to the viewer's dial.
    EcazMine { amount: i32 },
}

/// The leader part of the strength shown under the plan.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct BattleLeaderStrength {
    /// `Hero.ValueInCombatAgainst(null)`.
    pub value: i32,
    /// `Battle.DetermineSkillBonus` and the skill's label when it is not 0.
    pub skill_bonus: i32,
    pub skill_label: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct BattleOptions {
    pub territory: i32,
    pub territory_label: String,
    pub territory_locations: Vec<i32>,
    pub you: BattleSide,
    pub opponent: BattleSide,
    pub notes: Vec<BattleNote>,
    /// `Battle.DetermineForceSupplier(Game, Player)`: whose force tokens are dialled.
    #[ts(type = "Faction")]
    pub supplier: String,
    /// `Battle.MaxForces(Game, supplier, false/true)`: the number of tokens per row.
    pub normal_tokens: i32,
    pub special_tokens: i32,
    /// Whether a token can be at half strength (`MustPayFor...` and a positive force cost).
    pub half_normal: bool,
    pub half_special: bool,
    /// `Battle.MustPayForAnyForcesInBattle(Game, Player)`: the spice stepper is shown.
    pub must_pay: bool,
    /// `Player.Resources` and what a CHOAM ally can pay (`Game.ResourcesYourAllyCanPay`).
    pub resources: i32,
    pub ally_cap: i32,
    #[ts(type = "Faction")]
    pub ally: String,
    /// Every affordable way to make up the forces: `[full specials, half specials, full normal,
    /// half normal, dial, cost]`, with `Battle.Dial` and `Battle.Cost`.
    pub mixes: Vec<(i32, i32, i32, i32, f32, i32)>,
    /// `Battle.ValidBattleHeroes(Game, Player)`.
    pub heroes: Vec<BattleFormHero>,
    /// `!Game.Applicable(Rule.BattleWithoutLeader)`.
    pub leader_required: bool,
    /// `Battle.ValidWeapons(Game, Player, defense, hero, territory)` and
    /// `ValidDefenses(Game, Player, weapon, territory)` for the draft, by name.
    pub weapons: Vec<i32>,
    pub defenses: Vec<i32>,
    /// `Player.MessiahAvailable` and not prevented.
    pub messiah_available: bool,
    /// `Battle.MaxBankerBoost(Game, Player, hero)` for the draft's hero.
    pub max_banker_boost: i32,
    /// `Skin.Format("Boost {0} {1} with {2}?", Banker, hero, Resource)`.
    pub banker_label: String,
    /// The draft's leader strength; null without a leader.
    pub leader: Option<BattleLeaderStrength>,
    /// The event the form sends with its current inputs, in savegame JSON: the default draft in
    /// a view, the adjusted draft in a query answer.
    #[ts(type = "Record<string, unknown>")]
    pub draft: serde_json::Value,
}

/// A form with buttons only (`PlanetologyComponent`, `RockWasMeltedComponent`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct BattleButtonsOptions {
    #[ts(type = "Record<string, unknown>")]
    pub draft: serde_json::Value,
}
