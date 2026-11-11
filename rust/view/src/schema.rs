//! The per-player game view: what one viewer may see, plus the actions they can take now.
//!
//! These types are the wire format of the Game view section of `rust/docs/protocol.md`. They
//! serialize to camelCase JSON, and `tests/ts_bindings.rs` generates the TypeScript types in
//! `web/src/generated/` from them. Enum values are the C# member names (`"Atreides"`, `"Bidding"`),
//! the keys the skin JSON uses. Ids of static game objects (locations, territories, leaders,
//! treachery cards, resource cards) are the C# `Id`s; the client gets names, images and map
//! positions from the skin and the static data file, never from the view.
//!
//! The fields that are not panels are complete (client-plan.md, package 1). The `bidding` and
//! `battle` fields hold what the map draws; packages 7a and 7b add what their forms need, and
//! the action options are added form by form (package 7). protocol.md lists what is planned.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

mod revival;
pub use revival::*;
mod other;
pub use other::*;
mod ship_move;
pub use ship_move::*;
mod battle;
pub use battle::*;
mod setup;
pub use setup::*;

/// One rendered piece of a log line or prompt: the server's rendering of a C# `Expression`
/// element. The client draws each kind as `ExpressionComponent.razor` does. `label` is the
/// skin's description of the element (`Skin.Describe`), rendered on the server with the game's
/// skin; ids and member names are there for images, colours and popovers.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "t", rename_all = "camelCase")]
pub enum Part {
    /// Plain text.
    Text { v: String },
    /// C# `null`: drawn as a bold "none".
    None,
    /// An `int`, `float` or `double`, drawn as a number token (`SimpleNumberComponent`). Floats
    /// and doubles are already rounded like `Support.Round` (three decimals, to even). A `long`
    /// is not a number token in C# (`case int` doesn't match it); it arrives as `text`.
    Num { v: f64 },
    /// A `bool`, drawn as its label ("Yes", "No").
    Bool { v: bool, label: String },
    /// A spice amount, optionally followed by "by <faction>".
    Payment {
        amount: i32,
        #[ts(type = "Faction | null")]
        by: Option<String>,
    },
    /// A faction chip.
    Faction {
        #[ts(type = "Faction")]
        v: String,
        label: String,
    },
    /// Any other enum value: `ty` is the C# enum type name, `v` the member name. The client picks
    /// the drawing by type (bold for `TreacheryCardType`, a force token for `FactionForce`, the
    /// worm icon for `Concept.Monster`, ...).
    Enum { ty: String, v: String, label: String },
    /// A place chip. `label` is `Describe(Location)` (the chip's title); the chip's name and
    /// sector come from the skin and the static data.
    Location { id: i32, label: String },
    Territory { id: i32, label: String },
    /// `skill` is the leader's skill in the game (`Game.Skill`), which its popover shows.
    Leader {
        id: i32,
        label: String,
        #[ts(type = "LeaderSkill | null")]
        skill: Option<String>,
    },
    Messiah { label: String },
    TreacheryCard { id: i32, label: String },
    ResourceCard { id: i32, label: String },
}

/// A message or prompt, rendered to parts. Nested messages and lists are flattened, with the
/// ", " separators `ExpressionComponent.DetermineElements` adds.
pub type RichText = Vec<Part>;

/// A C# `Message` as the log shows it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LogLine {
    /// The rendered expression.
    pub text: RichText,
    /// True for a message sent only to this viewer (`Message.Target`), drawn in italics.
    pub private: bool,
    /// `Message.Heading`: the concept whose icon heads a section of the log.
    #[ts(type = "string | null")]
    pub heading: Option<String>,
    /// `Message.Log`: the structured record a phase's log design draws from
    /// (`Shared/Model/Log/`). Built in `log.rs`.
    pub record: Option<crate::log::LogRecord>,
    /// `Message.Route`: where the forces of a shipment or move went.
    pub route: Option<crate::log::Route>,
    /// `Message.Plans`: both revealed battle plans.
    pub plans: Option<crate::log::BattlePlans>,
    /// `Message.Fact`: one fact of a battle's outcome.
    pub fact: Option<crate::log::BattleFact>,
}

/// One entry of `Game.Reports`, without its messages. The log shows one phase at a time and
/// asks the server for older ones (`historyRequest` in protocol.md).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ReportHeader {
    pub index: i32,
    pub turn: i32,
    #[ts(type = "MainPhase")]
    pub about: String,
    pub is_battle: bool,
    #[ts(type = "Faction")]
    pub aggressor: String,
    #[ts(type = "Faction")]
    pub defender: String,
    pub territory: Option<i32>,
    pub battle_decided: bool,
    #[ts(type = "Faction")]
    pub battle_winner: String,
}

/// The log as the report panel needs it: every report header, plus the lines of the reports
/// of the latest phase (the one the panel follows by default).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LogView {
    pub reports: Vec<ReportHeader>,
    /// Lines of the reports in `latest`, in order, filtered to what this viewer may read.
    pub latest: Vec<ReportLines>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ReportLines {
    pub index: i32,
    pub lines: Vec<LogLine>,
}

/// Who is looking. Derived from `Participation` the way `Client.cs` does it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ViewerInfo {
    /// The viewer's faction when they hold a seat (`Client.Faction`).
    #[ts(type = "Faction | null")]
    pub faction: Option<String>,
    pub is_host: bool,
    pub is_observer: bool,
}

/// What everybody sees of a player (the map ring, faction chips, the tanks).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PublicPlayer {
    #[ts(type = "Faction")]
    pub faction: String,
    pub seat: i32,
    /// `Game.GetPlayerName(p)`.
    pub name: String,
    pub is_bot: bool,
    #[ts(type = "Faction")]
    pub ally: String,
    pub forces_in_reserve: i32,
    pub special_forces_in_reserve: i32,
    pub forces_killed: i32,
    pub special_forces_killed: i32,
    /// The number of treachery cards held (`TreacheryCards.Count`), never their identity.
    pub card_count: i32,
    pub bribes: i32,
    pub resources_after_bidding: i32,
    pub extortion: i32,
    pub banked_resources: i32,
    #[ts(type = "Array<TechToken>")]
    pub tech_tokens: Vec<String>,
    /// Ecaz's ambassadors not yet placed, drawn on the ring (`Players.razor`).
    #[ts(type = "Array<Ambassador>")]
    pub ambassadors: Vec<String>,
    /// `Player.HasSpecialForces`: whether the reserves show a special force token.
    pub has_special_forces: bool,
    /// `Player.HasRoomForCards`: "FULL" on the ring during bidding, a emperor count in the prompt.
    pub has_room_for_cards: bool,
    /// `Game.CardsOwnedBy(p).Count()`: the hand plus a Karma card set aside for a bid.
    pub cards_owned: i32,
    /// `Player.HomeWorlds`, drawn on the ring (`Players.razor:47-121`). The forces on them are
    /// in `board.forces`.
    pub homeworlds: Vec<HomeworldView>,
    /// `Game.GetSkilledLeaders(p).Where(IsInFrontOfShield)`, with `Game.Skill(l)`.
    pub skilled_leaders: Vec<SkilledLeader>,
    /// Ecaz only: `Game.EcazLoyalLeader`.
    pub loyal_leader: Option<i32>,
    /// Richese only: `Game.LatestRevealedNoFieldValue` once a No-Field was revealed.
    pub latest_revealed_no_field: Option<i32>,
}

/// One homeworld of a player (`Players.razor`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct HomeworldView {
    #[ts(type = "World")]
    pub world: String,
    /// The homeworld's location id (forces on it are in `board.forces`).
    pub location: i32,
    /// `Game.OccupierOf(world).Faction` when `Game.IsOccupied(world)`.
    #[ts(type = "Faction | null")]
    pub occupier: Option<String>,
    /// `Player.HasHighThreshold(world)`.
    pub high_threshold: bool,
    /// `Player.HasLowThreshold(world)`.
    pub low_threshold: bool,
}

/// A skilled leader in front of a player's shield.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SkilledLeader {
    pub id: i32,
    #[ts(type = "LeaderSkill")]
    pub skill: String,
}

/// One card in the viewer's hand.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct HandCard {
    pub id: i32,
    /// The applicable event types that play this card (`CardPlays.PlaysCard` over
    /// `Client.Actions`). Empty when the card can't be played now. A non-empty list makes the
    /// card glow, and clicking it opens these actions' forms (`CardPlayPopover`). An action
    /// that appears here for some card is left out of the action column
    /// (`CardPlays.PlayedFromHand`).
    pub playable_with: Vec<String>,
}

/// A traitor or face dancer card held by the viewer.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct HeroCard {
    pub hero: HeroRef,
    /// In `RevealedTraitors` / `RevealedFaceDancers`: drawn blurred.
    pub revealed: bool,
    /// `Game.Skill(hero)` when the leader has a skill: its popover shows the skill card.
    #[ts(type = "LeaderSkill | null")]
    pub skill: Option<String>,
}

/// An `IHero`: a leader, the Messiah, or a treachery card used as a hero (Cheap Hero).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "t", rename_all = "camelCase")]
pub enum HeroRef {
    Leader { id: i32 },
    Messiah,
    TreacheryCard { id: i32 },
}

/// A leader of the viewer's faction, for the faction panel.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct OwnLeader {
    pub id: i32,
    pub alive: bool,
    /// `Game.IsSkilled(l)`: drawn highlighted.
    pub skilled: bool,
    /// `Game.Skill(l)` when the leader has a skill: its popover shows the skill card.
    #[ts(type = "LeaderSkill | null")]
    pub skill: Option<String>,
}

/// The viewer's own faction: their shield. Only ever sent to the player holding the seat.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PrivatePlayer {
    #[ts(type = "Faction")]
    pub faction: String,
    pub resources: i32,
    pub hand: Vec<HandCard>,
    /// `Game.GetCardSetAsideForBid(Player)`, shown after the hand.
    pub card_set_aside: Option<i32>,
    pub leaders: Vec<OwnLeader>,
    pub traitors: Vec<HeroCard>,
    pub face_dancers: Vec<HeroCard>,
    /// Bene Gesserit's prediction, once made (`Phase > BeneGesseritPredicting`).
    #[ts(type = "Faction | null")]
    pub predicted_faction: Option<String>,
    pub predicted_turn: Option<i32>,
    /// The Nexus card held (`Player.Nexus`).
    #[ts(type = "Faction | null")]
    pub nexus: Option<String>,
    /// `Game.NextStormMoves` while `HasStormPrescience(Player)` and the storm is dialled.
    pub next_storm_moves: Option<i32>,
    /// Atreides only, with `Rule.AtreidesMessiah`: the Messiah's state (`PlayerPanel.razor:106-133`).
    pub messiah: Option<MessiahState>,
    /// Moritani only: `Game.UnplacedTerrorTokens`.
    #[ts(type = "Array<TerrorType> | null")]
    pub unplaced_terror: Option<Vec<String>>,
    /// `Game.CardJustWon` when the viewer knows it (`Game.KnownCards(Player)`); the Atreides
    /// tracker's card picker marks it (`CardPickerComponent.razor:41`).
    pub card_just_won: Option<i32>,
}

/// The Messiah as the faction panel and the notes describe it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct MessiahState {
    /// `Player.TotalForcesKilledInBattle`: the Messiah comes at 7.
    pub forces_killed_in_battle: i32,
    /// `Player.MessiahAvailable`.
    pub available: bool,
}

/// What the map shows. The slice fills the storm, forces and spice; protocol.md lists the rest.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct BoardView {
    pub sector_in_storm: i32,
    pub shield_wall_destroyed: bool,
    /// `Game.ForcesOnPlanetExcludingEmptyLocations(true)`, one entry per location and faction,
    /// homeworlds included.
    pub forces: Vec<ForcesInLocation>,
    /// `Game.ResourcesOnPlanet`.
    pub spice: Vec<SpiceInLocation>,
    /// `GameStatus.HighlightedTerritories`.
    pub highlighted_territories: Vec<i32>,
    /// `Game.StrongholdOwnership`: the owner's token on a stronghold, and its advantage's popover
    /// (the advantage is in the static data).
    pub strongholds: Vec<StrongholdOwner>,
    /// `Game.TerrorIn(territory)`: one entry per placed token. The token's identity is sent to
    /// Moritani only (`ForcesOnPlanet.razor:27`); others see a token.
    pub terror: Vec<TerrorOnPlanet>,
    /// `Game.AmbassadorIn(territory)` for every territory with one.
    pub ambassadors: Vec<AmbassadorOnPlanet>,
    /// `Game.DiscoveriesOnPlanet`: unrevealed discovery tokens. The token is sent to Fremen for
    /// the Fremen type and to the Guild for the Guild type (`ForcesOnPlanet.razor:101`).
    pub discoveries: Vec<DiscoveryOnPlanet>,
    /// The Hidden Mobile Stronghold and the discovered strongholds that are on the map
    /// (`Location.Visible`), with the location each is attached to. The skin places them
    /// relative to it (`Skin.GetCenter`).
    pub attached: Vec<AttachedLocationView>,
    /// Living leaders and the Messiah standing in a territory (`LeaderState.CurrentTerritory`),
    /// drawn with the forces (`ForcesInLocation.razor:41-49`).
    pub leaders: Vec<LeaderOnPlanet>,
    /// `Game.Monsters`: worms on the planet now.
    pub monsters: Vec<MonsterView>,
    /// `Game.NumberOfMonsters`: worm tokens on the Shield Wall under `Rule.Ssw` (`SSW.razor`).
    pub number_of_monsters: i32,
    /// Territories for which `Game.IsSpecialStronghold` holds (the Shield Wall under SSW).
    pub special_strongholds: Vec<i32>,
    /// `Game.AtomicsAftermath`.
    pub atomics_aftermath: Option<i32>,
    /// Locations for which `Game.IsProtectedFromStorm(l)` holds (the territory tooltip).
    pub protected_from_storm: Vec<i32>,
    /// Richese only: `Game.CurrentNoFieldValue`, the value of the No-Field token on the planet.
    /// Others see "?" (`MapComponent.razor:219`).
    pub no_field_value: Option<i32>,
    /// `Game.NextStormMoves` when `Game.HasStormPrescience(Player)`: the sectors the next storm
    /// passes over are tinted (`Planet.razor:107`). Unlike `me.nextStormMoves` this has no phase
    /// gate, as in C#.
    pub storm_prescience_moves: Option<i32>,
    /// During ship and move, an arrow per origin of each of `Game.RecentMoves`, following
    /// `Map.FindFirstShortestPath` through the Polar Sink (`RecentMoves.razor`).
    pub recent_moves: Vec<MoveArrow>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ForcesInLocation {
    pub location: i32,
    #[ts(type = "Faction")]
    pub faction: String,
    pub forces: i32,
    pub special_forces: i32,
    /// `Game.ThreatenedByAllyPresence(player, territory)`: drawn with a emperor outline.
    pub threatened: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct StrongholdOwner {
    pub location: i32,
    #[ts(type = "Faction")]
    pub owner: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TerrorOnPlanet {
    pub territory: i32,
    /// The token, for Moritani; null for everyone else.
    #[ts(type = "TerrorType | null")]
    pub token: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AmbassadorOnPlanet {
    pub territory: i32,
    #[ts(type = "Ambassador")]
    pub ambassador: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryOnPlanet {
    pub location: i32,
    #[ts(type = "DiscoveryTokenType")]
    pub token_type: String,
    /// The token, for the faction that may know it; null otherwise.
    #[ts(type = "DiscoveryToken | null")]
    pub token: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AttachedLocationView {
    pub location: i32,
    /// `AttachedToLocation`.
    pub attached_to: i32,
    /// `Location.Sector` (the host's sector, or -1 in old game versions).
    pub sector: i32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LeaderOnPlanet {
    pub hero: HeroRef,
    pub territory: i32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct MonsterView {
    pub territory: i32,
    pub is_great_monster: bool,
}

/// One arrow of `RecentMoves.razor`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct MoveArrow {
    #[ts(type = "Faction")]
    pub faction: String,
    pub from: i32,
    pub to: i32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SpiceInLocation {
    pub location: i32,
    pub amount: i32,
}

/// The prompt card (`GameStatusComponent`): what is happening, who is expected to act.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct StatusView {
    /// `GameStatus.GetMessage(Player, IsHost)`: the awaited text when the viewer is awaited,
    /// the waiting text otherwise. Observers get the waiting text.
    pub message: RichText,
    /// `WaitingForMe(Player, IsHost)`.
    pub waiting_for_me: bool,
    pub waiting_for_host: bool,
    #[ts(type = "Array<Faction>")]
    pub waiting_for: Vec<String>,
    /// `WaitingInSequence`: every player of a sequence in turn order, with whose turn it is.
    pub waiting_in_sequence: Vec<SequenceEntry>,
    /// The event the phase timer counts from (`TimedEvent.Time`), as ISO 8601.
    pub timed_event_time: Option<String>,
    /// `GameStatus.FlashInfo`: the images flashed after this event (`ImageFlashComponent`).
    pub flash: Vec<FlashView>,
    /// During the bidding main phase, `Bid.PlayersToBid` (or `BlackMarketBid.PlayersToBid` in
    /// the black market phases): the bidders the prompt shows with their card counts
    /// (`GameStatusComponent.razor:28-50`). Empty otherwise.
    pub to_bid: Vec<SequenceEntry>,
}

/// One flashed image with its caption (`FlashInfo`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct FlashView {
    /// `FlashInfo.ToShow`: a faction, card, leader, skill, Nexus card, ... as a rich text part,
    /// whose kind picks the image.
    pub to_show: Part,
    pub message: RichText,
    /// For each part of `message`, whether `ImageFlashComponent` draws it with the text border:
    /// `ExpressionComponent` leaves the border off the parts of a nested message.
    pub bordered: Vec<bool>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SequenceEntry {
    #[ts(type = "Faction")]
    pub faction: String,
    pub has_turn: bool,
}

/// One action the viewer can take now: an entry of `Game.GetApplicableEvents(Player, IsHost)`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ActionView {
    /// The C# event class name, which is also the `$type` of the event to submit.
    pub event_type: String,
    /// The choices the form offers, for the form's default inputs. `None` for event types whose
    /// option builder is not written yet: the client then shows only the event's name.
    pub options: Option<ActionOptions>,
    /// `Validate()` of the event the form would send with its default inputs, rendered.
    /// The confirm button shows it and is disabled while it is set.
    pub validation: Option<RichText>,
}

/// The choices of one action form, by event type. A form whose choices depend on its own other
/// inputs gets them again from a draft query (protocol.md, Dependent inputs) with the same shape.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "form")]
pub enum ActionOptions {
    /// A form with only a confirm button (`SimpleActionComponent`), such as `AllianceBroken` or
    /// `AlliancePassed`. The server sends the heading and button label that `ActionPanel.razor`
    /// passes the component, because many of them name a faction, card or territory.
    #[serde(rename_all = "camelCase")]
    Simple {
        title: Option<RichText>,
        confirm_label: String,
        /// `Alert`: the form is urgent (drawn as the act card's alert state).
        alert: bool,
        /// `Dismissable`: a second button with this label hides the form until the next view
        /// (`HideSecrets`: "No").
        #[serde(default)]
        dismiss_label: Option<String>,
        /// `AllowAutoConfirm` (`GuildDelay`): the "Next time, auto ... after 3 seconds" box.
        #[serde(default)]
        allow_auto_confirm: bool,
    },
    /// `AllianceOfferedComponent`: pick a faction to offer an alliance to.
    AllianceOffered {
        /// `AllianceOffered.ValidTargets(Game, Player)`.
        #[ts(type = "Array<Faction>")]
        targets: Vec<String>,
    },
    /// `StormDialledComponent`: dial a number between `min` and `max`.
    StormDialled { min: i32, max: i32 },
    // Ship and move forms (package 7c): payloads in schema/ship_move.rs, built in
    // actions/ship_move.rs.
    Shipment(ShipmentOptions),
    Move(PlacementOptions),
    Caravan(PlacementOptions),
    FremenRidesMonster(PlacementOptions),
    PerformFremenSetup(StagedPlacementOptions),
    DiscoveryEntered(DiscoveryEnteredOptions),
    AmbassadorActivated(AmbassadorActivatedOptions),
    AmbassadorPlaced(AmbassadorPlacedOptions),
    BeneGesseritAccompanies(BeneGesseritAccompaniesOptions),
    BeneGesseritFlip(BeneGesseritFlipOptions),
    ChoamMovePrevention(ChoamMovePreventionOptions),
    ChoamRemoveForce(ChoamRemoveForceOptions),
    DiscoveryRevealed(DiscoveryRevealedOptions),
    FlightUsed(NoOptions),
    TestingStationUsed(NoOptions),
    KarmaHmsMovement(HmsMovementOptions),
    PerformHmsMovement(HmsMovementOptions),
    KarmaShipmentPrevention(FactionTargetsOptions),
    PerformBeneGesseritPlacement(LocationPickOptions),
    PerformMoritaniSetup(LocationPickOptions),
    PerformHmsPlacement(LocationPickOptions),
    FremenSentMonster(LocationPickOptions),
    SetShipmentPermission(SetShipmentPermissionOptions),
    TerrorPlanted(TerrorPlantedOptions),
    TerrorRevealed(TerrorRevealedOptions),
    // Revival forms (package 7d): payloads in schema/revival.rs.
    Revival(RevivalOptions),
    RaiseDeadPlayed(RaiseDeadPlayedOptions),
    KarmaFreeRevival(KarmaFreeRevivalOptions),
    KarmaRevivalPrevention(KarmaRevivalPreventionOptions),
    ChoamFreeRevivalPrevention(ChoamFreeRevivalPreventionOptions),
    RequestTleilaxuRevival(RequestTleilaxuRevivalOptions),
    AcceptOrCancelTleilaxuRevival(AcceptOrCancelTleilaxuRevivalOptions),
    SetIncreasedRevivalLimits(SetIncreasedRevivalLimitsOptions),
    /// `NormalBiddingPanel` (`BiddingPanel.razor`), the bidder's part of the bidding panel
    /// during `Phase.Bidding`. The panel's lot (card, current bid) is `GameView.bidding.panel`.
    /// Built in `actions/bidding.rs`.
    #[serde(rename_all = "camelCase")]
    Bid {
        /// `Bid.ValidMaxAmount(Player, KarmaCard != null)`: depends on the draft's Karma card.
        max_amount: i32,
        /// `Bid.ValidMaxAllyAmount(Game, Player)`. The ally's picker is shown when the player
        /// has an ally and this is above 0; otherwise the ally pays nothing.
        max_ally_amount: i32,
        /// `Game.SpiceForBidsEmperorCanPay(Faction)`. Emperor's picker is shown when above 0.
        max_emperor_amount: i32,
        /// `Bid.ValidKarmaCards(Game, Player)` when `Bid.CanKarma(Game, Player)`, else empty.
        /// The Karma picker and "Win immediately" are shown when this is not empty.
        karma_cards: Vec<i32>,
        /// `Bid.MayUseEmperorSecretAlly(Game, Player)`: the Nexus card checkbox.
        may_use_emperor_secret_ally: bool,
        /// The form's starting `Amount` and `AllyContributionAmount`
        /// (`BiddingPanel.OnParametersSetAsync`: the lowest bid that beats the current one,
        /// paid by the ally first, then clamped by the pickers). 0 in silent auctions.
        default_amount: i32,
        default_ally_amount: i32,
    },
    /// `BlackMarketBiddingPanel` (`BiddingPanel.razor`), during `Phase.BlackMarketBidding`.
    /// No Karma and no Nexus card.
    #[serde(rename_all = "camelCase")]
    BlackMarketBid {
        /// `BlackMarketBid.ValidMaxAmount(Player)`.
        max_amount: i32,
        /// `BlackMarketBid.ValidMaxAllyAmount(Game, Player)`.
        max_ally_amount: i32,
        /// `Game.SpiceForBidsEmperorCanPay(Faction)`.
        max_emperor_amount: i32,
        default_amount: i32,
        default_ally_amount: i32,
    },
    // Other events (package 7e): payloads in schema/other.rs, built in actions/other.rs.
    YesNo(YesNoOptions),
    AllyPermission(AllyPermissionOptions),
    AutomationConfigured(AutomationConfiguredOptions),
    BeneGesseritPrediction(BeneGesseritPredictionOptions),
    CardTraded(CardTradedOptions),
    ChoamDiscarded(CardChoiceOptions),
    ChoamEconomics(ChoamEconomicsOptions),
    ChoamKarmaPrevention(ChoamKarmaPreventionOptions),
    DealOffered(DealOfferedOptions),
    Discarded(CardChoiceOptions),
    DiscardedSearched(CardChoiceOptions),
    DiscardedTaken(CardChoiceOptions),
    DistransUsed(DistransUsedOptions),
    DivideResources(DivideResourcesOptions),
    Donated(DonatedOptions),
    EmperorBidSupport(EmperorBidSupportOptions),
    EmperorDiscarded(CardChoiceOptions),
    FactionSelected(FactionChoiceOptions),
    FactionTradeOffered(FactionTradeOfferedOptions),
    IxianRemovedCardFromAuction(CardChoiceOptions),
    IxianSelectedStartingCard(CardChoiceOptions),
    IxianSwappedCardOnBid(CardChoiceOptions),
    Karma(KarmaOptions),
    KarmaChoamDiscard(KarmaChoamDiscardOptions),
    KarmaHandSwap(KarmaHandSwapOptions),
    KarmaHandSwapInitiated(FactionChoiceOptions),
    KarmaMonster(TerritoryChoiceOptions),
    KarmaRicheseBuy(CardChoiceOptions),
    MulliganPerformed(MayOptions),
    NexusCardDrawn(MayOptions),
    NexusPlayed(NexusPlayedOptions),
    PlayerReplaced(PlayerReplacedOptions),
    ReplacedCardWon(DraftOptions),
    ResourcesAudited(FactionChoiceOptions),
    ResourcesTransferred(ResourcesTransferredOptions),
    SkillAssigned(SkillAssignedOptions),
    StormSpellPlayed(DraftOptions),
    TraitorDiscarded(HeroChoiceOptions),
    TraitorsSelected(HeroChoiceOptions),
    RicheseAnnouncesAuction(DraftOptions),
    RicheseAnnouncesBlackMarket(RicheseAnnouncesBlackMarketOptions),
    RicheseGaveCard(CardChoiceOptions),
    RicheseKeepsUnsoldCard(DraftOptions),
    RicheseSpecifiesAuction(RicheseSpecifiesAuctionOptions),
    // Battle forms (package 7b): payloads in schema/battle.rs, built in actions/battle.rs.
    AuditCancelled(AuditCancelledOptions),
    Audited(AuditedOptions),
    Battle(BattleOptions),
    BattleConcluded(BattleConcludedOptions),
    BattleInitiated(BattleInitiatedOptions),
    BeneGesseritBattleAnnouncement(BeneGesseritBattleAnnouncementOptions),
    CardsDetermined(CardsDeterminedOptions),
    ClairVoyanceAnswered(ClairVoyanceAnsweredOptions),
    ClairVoyancePlayed(ClairVoyancePlayedOptions),
    Diplomacy(BattleCardsOptions),
    FaceDanced(FaceDancedOptions),
    FaceDancerReplaced(BattleHeroesOptions),
    FaceDancerRevealed(BattleMayCallOptions),
    HMSAdvantageChosen(BattleChoicesOptions),
    JuicePlayed(BattleChoicesOptions),
    LoserConcluded(LoserConcludedOptions),
    Planetology(BattleButtonsOptions),
    Prescience(PrescienceOptions),
    Retreat(RetreatOptions),
    RockWasMelted(BattleButtonsOptions),
    TakeLosses(TakeLossesOptions),
    ThoughtAnswered(BattleCardsOptions),
    Thought(BattleCardsOptions),
    TreacheryCalled(BattleMayCallOptions),
    Voice(VoiceOptions),
    // Setup forms (package 8): payloads in schema/setup.rs, built in actions/setup.rs.
    EstablishPlayers(EstablishPlayersOptions),
    PerformSetup(PerformSetupOptions),
}

/// The answer to a draft query: the form's options for the draft's inputs, and the draft's
/// validation. Same shapes as in [`ActionView`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DraftResult {
    pub event_type: String,
    pub options: Option<ActionOptions>,
    pub validation: Option<RichText>,
}

/// The whole view of one game for one viewer, sent after every event.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct GameView {
    /// `Game.History.Count`: the number of events so far. Draft queries and submissions name the
    /// view they were made from with it.
    pub event_number: i32,
    pub version: i32,
    pub turn: i32,
    pub maximum_turns: i32,
    /// `Game.MaximumPlayers`: the seats around the map, which place each player's token
    /// (`SectorGeometry.PlayerTokenPosition`).
    pub maximum_players: i32,
    #[ts(type = "MainPhase")]
    pub main_phase: String,
    #[ts(type = "Phase")]
    pub phase: String,
    pub viewer: ViewerInfo,
    /// Every player, in seat order.
    pub players: Vec<PublicPlayer>,
    /// The viewer's own faction, if they hold a seat.
    pub me: Option<PrivatePlayer>,
    pub board: BoardView,
    pub status: StatusView,
    pub actions: Vec<ActionView>,
    pub log: LogView,
    #[ts(type = "Ruleset")]
    pub ruleset: String,
    /// `Game.Rules`. `Game.Applicable(rule)` is true for these and for `rulesForBots`.
    #[ts(type = "Array<Rule>")]
    pub rules: Vec<String>,
    /// `Game.RulesForBots`.
    #[ts(type = "Array<Rule>")]
    pub rules_for_bots: Vec<String>,
    /// `Game.EconomicsStatus`, drawn by the Charity phase name (`Phases.razor`).
    #[ts(type = "ChoamEconomicsStatus")]
    pub economics_status: String,
    /// The card piles beside the map (`CardPiles.razor`).
    pub piles: PilesView,
    /// The tanks (`Tanks.razor`), one entry per player in `Game.Players` order.
    pub tanks: Vec<TanksEntry>,
    /// During the bidding main phase: what the map shows of the auction (`Bids.razor`).
    pub bidding: Option<BiddingView>,
    /// While there is a current battle: its sides and the battle plans this viewer may see
    /// (`BattlePlans.razor`, `BattlePlan.razor`).
    pub battle: Option<BattleView>,
    /// With assisted notekeeping (`Game.AssistedNoteKeepingEnabled(Player)`): what the viewer
    /// could have noted of every other player (`NoteComponent.razor`).
    pub notes: Option<NotesView>,
    /// Game start, durations and the bidding timer (`Phases.razor`, `GameStatusComponent`).
    pub timing: TimingView,
    /// The viewer's deals (`DealComponent.razor`); null for observers.
    pub deals: Option<DealsView>,
    /// `Participation.BotsSpeed`: 0 paused, 1, 2 or 4 (the host's menu).
    pub bots_speed: i32,
    /// Atreides's card tracker (`TrackerComponent.razor`); null for everyone else.
    pub tracker: Option<TrackerView>,
    /// The cards of the menu's "List of Cards" (`MentatComponent.razor`). The same for everyone.
    pub mentat: MentatView,
    /// After `GameEnded`: the winners and the story (`WinnerPanel`, `Story*`), with the
    /// secrets of factions that chose `HideSecrets` left out. Built in `ending.rs`.
    pub ending: Option<crate::ending::EndingView>,
    /// The rules sheets of the factions in the game, in player order, for the faction popover on
    /// the map (`Support.FactionPopupHtml`). The same for everyone. Built in `skin_texts.rs`.
    pub faction_sheets: Vec<crate::skin_texts::FactionSheetView>,
    /// The ruleset and rules as the turn popover names them (`Phases.razor` `TurnPopover`).
    pub rule_labels: crate::skin_texts::RuleLabels,
    /// `Game.RecentMilestones`: the milestones of the latest event, whose sounds the client
    /// plays (`Client.PlaySoundsForMilestones`). The same for everyone.
    #[ts(type = "Array<Milestone>")]
    pub recent_milestones: Vec<String>,
}

/// `CardPiles.razor`. Treachery cards by id, resource cards by `SkinId`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PilesView {
    /// `TreacheryDeck.Items.Count`.
    pub treachery_deck_count: i32,
    /// `TreacheryDiscardPile.Items.Count`.
    pub treachery_discard_count: i32,
    /// `TreacheryDiscardPile.Top`: face up for everyone.
    pub treachery_discard_top: Option<i32>,
    /// With notekeeping: every card of the discard pile, top first; null for a card not in
    /// `Game.KnownCards(Player)` (drawn face down).
    pub treachery_discard: Option<Vec<Option<i32>>>,
    /// `ResourceCardDeck.Items.Count`.
    pub resource_deck_count: i32,
    /// `ResourceCardDeck.Top` during ship and move under `HasResourceDeckPrescience(Player)`;
    /// shown on the deck and in the faction panel (`PlayerPanel.razor:155`).
    pub resource_deck_top: Option<i32>,
    /// `Game.LatestSpiceCardA` and `B`.
    pub latest_spice_card_a: Option<i32>,
    pub latest_spice_card_b: Option<i32>,
    /// With notekeeping: `ResourceCardDiscardPileA` and `B`, top first.
    pub resource_discard_a: Option<Vec<i32>>,
    pub resource_discard_b: Option<Vec<i32>>,
    /// `Game.SandTrout` once `SandTroutOccured`.
    pub sand_trout: Option<i32>,
    /// `RicheseCache.Count`.
    pub richese_cache_count: i32,
    /// Richese only: the cards of `Game.RicheseCache`.
    pub richese_cache: Option<Vec<i32>>,
}

/// One player's row of the tanks.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TanksEntry {
    #[ts(type = "Faction")]
    pub faction: String,
    pub forces_killed: i32,
    pub special_forces_killed: i32,
    /// Dead leaders in order of death (`LeaderState.TimeOfDeath`).
    pub leaders: Vec<DeadLeader>,
    /// Atreides only: the Messiah is dead (`!Game.MessiahIsAlive`).
    pub messiah_dead: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DeadLeader {
    /// The leader, or null for a face-down leader the viewer may not identify (only its owner
    /// and Tleilaxu may: `Tanks.razor`, `MayIdentify`).
    pub id: Option<i32>,
    /// `LeaderState.IsFaceDownDead`.
    pub face_down: bool,
}

/// What the map shows of the auction, and the lot the bidding panel shows.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct BiddingView {
    #[ts(type = "AuctionType")]
    pub auction_type: String,
    /// `Game.Bids`, as `Bids.razor` labels them.
    pub bids: Vec<MapBid>,
    /// During `Phase.Bidding` and `Phase.BlackMarketBidding`: the bidding panel's lot, which
    /// `ActionPanel.razor` shows to every player, bidding or not. Observers get the lot a player
    /// without prescience sees (the C# panel throws for them: `HasBiddingPrescience(null)`;
    /// protocol.md, deviation 5).
    pub panel: Option<BiddingPanelView>,
    /// `Game.RecentMilestones.Contains(Milestone.AuctionWon)`: the client turns auto-pass off
    /// (`Client.ResetAutoPassThreshold`, after every event in the bidding main phase).
    pub auction_won: bool,
}

/// `BiddingPanel.razor` without the bidder's inputs (those are the `Bid` and `BlackMarketBid`
/// action options).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct BiddingPanelView {
    /// `BlackMarketBiddingPanel` (titled "Black Market") rather than `NormalBiddingPanel`.
    pub black_market: bool,
    /// `CardIsVisible`: `Game.CardsOnAuction.Top` under bidding prescience or in Richese's
    /// auctions; null otherwise (the card back).
    pub card: Option<i32>,
    /// Normal auctions: `Game.CardNumber` of `Game.NumberOfCardsOnRegularAuction`.
    pub card_of: Option<CardOf>,
    /// `BlackMarketSilent` or `RicheseSilent`: no current bid, no Pass button.
    pub silent: bool,
    /// `Game.CurrentBid` outside silent auctions.
    pub current_bid: Option<CurrentBid>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CardOf {
    pub number: i32,
    pub of: i32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CurrentBid {
    #[ts(type = "Faction")]
    pub faction: String,
    /// `IBid.TotalAmount`.
    pub amount: i32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct MapBid {
    #[ts(type = "Faction")]
    pub faction: String,
    pub bid: BidShown,
}

/// A bid as the ring shows it. In silent auctions every bid is "ready": the C# client also
/// colours a pass differently, which reveals it; the view doesn't (protocol.md, Visibility).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "t", rename_all = "camelCase")]
pub enum BidShown {
    Pass,
    Bid { amount: i32 },
    Ready,
}

/// The current battle, as the map draws it. Package 7b adds what the battle forms need.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct BattleView {
    pub territory: Option<i32>,
    #[ts(type = "Faction")]
    pub aggressor: String,
    #[ts(type = "Faction")]
    pub defender: String,
    /// The aggressor's plan as this viewer may see it; null when hidden or not made yet, and
    /// outside the battle main phase and during the battle report (`BattlePlans.razor:11`).
    pub aggressor_plan: Option<PlanView>,
    pub defender_plan: Option<PlanView>,
}

/// A battle plan on the map (`BattlePlan.razor`). `full` is `ShowEntirePlan`: the viewer's own
/// plan, both plans once both are in, and the opponent's plan under Atreides Karama. Otherwise only
/// the aspect revealed by prescience is set (`*Shown`), and only for the prescience user and
/// their ally (C# shows it to everyone; protocol.md, Visibility).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PlanView {
    #[ts(type = "Faction")]
    pub faction: String,
    pub is_aggressor: bool,
    pub full: bool,
    /// `Plan.Dial(Game, opponent)`, unrounded; set when the dial is shown.
    pub dial: Option<f64>,
    /// `Game.CurrentEcazBattleContribution` when positive and the plan is Ecaz's or their ally's.
    pub ecaz_contribution: Option<i32>,
    /// Ecaz's plan: `Game.EcazKarmaBonus` when not 0.
    pub ecaz_karma_bonus: Option<i32>,
    /// `Player.GetHomeworldBattleContributionAndLasgunShieldLimit(territory)` when not 0, with
    /// the homeworld fought on.
    pub homeworld_bonus: Option<HomeworldBonus>,
    /// Full plans: `Plan.HasReinforcements` (+2).
    pub reinforcements: bool,
    /// Full plans: `Plan.Cost(Game)` when positive.
    pub cost: Option<i32>,
    /// Full plans: `Game.HasStrongholdAdvantage(initiator, FreeResourcesForBattles, territory)`.
    pub free_resources_advantage: bool,
    /// Whether the leader part is drawn (full plans, or leader prescience).
    pub hero_shown: bool,
    /// The leader or Cheap Hero, when shown and present. A shown part without a hero reads
    /// "no leader" in a partial plan.
    pub hero: Option<PlanHero>,
    /// The opponent called this plan's leader a traitor ("TRAITOR!").
    pub traitor_called: bool,
    /// The leader was a face dancer ("FACE DANCER!").
    pub facedanced: bool,
    /// Atreides's plan with the Messiah.
    pub messiah: bool,
    /// Whether the defense part is drawn; `defense` null in a partial plan reads "no defense".
    pub defense_shown: bool,
    /// The defense card drawn (Portable Antidote when it is being used).
    pub defense: Option<i32>,
    pub weapon_shown: bool,
    pub weapon: Option<i32>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct HomeworldBonus {
    #[ts(type = "World")]
    pub world: String,
    pub amount: i32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "t", rename_all = "camelCase")]
pub enum PlanHero {
    #[serde(rename_all = "camelCase")]
    Leader {
        id: i32,
        /// `Game.Skill(hero)` when not `None`.
        #[ts(type = "LeaderSkill | null")]
        skill: Option<String>,
        /// `Game.GetSkill(Plan.Player)` when it differs from `skill` and matters for a plan.
        #[ts(type = "LeaderSkill | null")]
        passive_skill: Option<String>,
        /// `Battle.DetermineSkillBonus` when not 0.
        bonus: Option<i32>,
        /// After plans are revealed: `Battle.DetermineSkillPenalty` from the opponent's plan,
        /// when not 0, with the skill that causes it.
        penalty: Option<SkillPenalty>,
    },
    /// A treachery card used as a leader (Cheap Hero).
    TreacheryCard { id: i32 },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SkillPenalty {
    pub amount: i32,
    #[ts(type = "LeaderSkill")]
    pub skill: String,
}

/// `NoteComponent.razor`: per other player, what notekeeping tracks.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct NotesView {
    pub players: Vec<NotesPlayer>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct NotesPlayer {
    #[ts(type = "Faction")]
    pub faction: String,
    pub resources: i32,
    pub bribes: i32,
    pub resources_after_bidding: i32,
    /// `Game.CardsOwnedBy(p)`: a card, or null for one not in the viewer's own
    /// `Player.KnownCards` (`NoteComponent.razor:196`).
    pub cards: Vec<Option<i32>>,
    pub revealed_traitors: Vec<HeroRef>,
    pub revealed_face_dancers: Vec<HeroRef>,
    /// `Game.NumberOfVictoryPoints(p, true)`.
    pub victory_points: i32,
    /// Atreides with `Rule.AtreidesMessiah`.
    pub messiah: Option<MessiahState>,
}

/// Times for the turn popover and the prompt's timer.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TimingView {
    /// `Game.Started`, ISO 8601.
    pub started: String,
    /// `Game.History.Last().Time`, ISO 8601, when there is an event.
    pub last_activity: Option<String>,
    /// `Game.Duration`, in seconds.
    pub duration: f64,
    /// `Game.TimeSpent(faction, phase)` per player, in seconds.
    pub time_spent: Vec<TimeSpent>,
    /// The bidding time limit in seconds, or -1 (`Client.Timer`, set by `SetTimer`; server
    /// state, not game state).
    pub timer: i32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TimeSpent {
    #[ts(type = "Faction")]
    pub faction: String,
    pub bidding: f64,
    pub shipment_and_move: f64,
    pub battle: f64,
}

/// `DealComponent.razor`: the deals of a seated viewer (package 5, `side.rs`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DealsView {
    /// `DealAccepted.AcceptableDeals(Game, Player)`: offers the viewer may accept. The menu bar
    /// counts them.
    pub acceptable: Vec<DealOfferView>,
    /// `DealAccepted.CancellableDeals(Game, Player)`: the viewer's own offers.
    pub cancellable: Vec<DealOfferView>,
    /// `DealAccepted.CurrentDeals(Game)`: every deal in force.
    pub current: Vec<CurrentDeal>,
}

/// One offer (`DealOffered`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DealOfferView {
    /// The offer's event number (its place in `Game.History`), which names it across views.
    pub id: i32,
    #[ts(type = "Faction")]
    pub initiator: String,
    pub price: i32,
    /// `GetDealDescription()`.
    pub description: RichText,
    /// `GetMessage()`, for the toast announcing the offer (`DealToasts.razor`).
    pub message: RichText,
    /// `Acceptance(Faction).Validate()`: Accept is disabled while it is set.
    pub accept_error: Option<RichText>,
    /// `Acceptance(Faction)` and `Rejection(Faction)` in savegame JSON, to submit as they are
    /// (null for the viewer's own offers).
    #[ts(type = "Record<string, unknown> | null")]
    pub accept: serde_json::Value,
    #[ts(type = "Record<string, unknown> | null")]
    pub decline: serde_json::Value,
    /// `Cancellation()` in savegame JSON, for the viewer's own offers.
    #[ts(type = "Record<string, unknown> | null")]
    pub withdraw: Option<serde_json::Value>,
    /// For the viewer's own offers: `Game.DealRecipients(offer)` with `Game.DealResponse`
    /// (true accepted, false declined, null deciding).
    pub answers: Vec<DealAnswer>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DealAnswer {
    #[ts(type = "Faction")]
    pub faction: String,
    pub accepted: Option<bool>,
}

/// A deal in force (`Deal`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CurrentDeal {
    #[ts(type = "Faction")]
    pub bound: String,
    #[ts(type = "Faction")]
    pub consuming: String,
    /// `DealContentsDescription(Game)`.
    pub description: RichText,
}

/// `MentatComponent.razor`, the menu's "List of Cards": the cards this game's rules put in play.
/// The other sections of the list follow from `rules` and the players' factions.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct MentatView {
    /// `TreacheryCardManager.GetCardsInPlay(Game)`, by id.
    pub treachery_cards: Vec<i32>,
    /// `TreacheryCardManager.GetRicheseCards()` when Richese plays, else empty.
    pub richese_cards: Vec<i32>,
    /// `Map.GetResourceCardsInPlay(Game)`, by `SkinId`.
    pub resource_cards: Vec<i32>,
}

/// `TrackerComponent.razor` and `AtreidesIntelligence.cs`: what Atreides's card tracker needs from
/// the game. The notes themselves stay in the browser (local storage), as in C#.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TrackerView {
    /// `TreacheryCardManager.GetCardsInPlay(Game)`, with Richese's cards when Richese plays:
    /// the cards a tracked card can be.
    pub cards_in_play: Vec<i32>,
    /// `Game.TraitorsInPlay`: the heroes a tracked traitor can be.
    pub traitors_in_play: Vec<HeroRef>,
    /// The resource cards left in the deck (`ResourceCardDeck.Items`), by `SkinId`, as a set
    /// (sorted, so the order says nothing).
    pub resource_cards: Vec<i32>,
    /// `ResourceCardDiscardPileA` and `B`, by `SkinId`, as the tracker lists them.
    pub resource_discard_a: Vec<i32>,
    pub resource_discard_b: Vec<i32>,
    /// Per player: how many cards, traitors and face dancers the tracker has places for.
    pub players: Vec<TrackerPlayer>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TrackerPlayer {
    #[ts(type = "Faction")]
    pub faction: String,
    /// `Player.MaximumNumberOfCards`.
    pub maximum_number_of_cards: i32,
    /// `Player.NumberOfTraitors`.
    pub number_of_traitors: i32,
    /// `Player.NumberOfFaceDancers`.
    pub number_of_face_dancers: i32,
}
