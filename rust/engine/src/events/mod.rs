//! Every game event, grouped by the phase or topic it belongs to, and [`Event`], which holds any
//! of them. The `events!` list below declares them; engine-conventions.md, "Events", says how.

pub(crate) mod macros;
mod dispatch;
pub mod rules;
pub use rules::*;
pub mod game_event;
pub use game_event::*;
pub mod passable_game_event;
pub use passable_game_event::*;

mod alliances;
pub use alliances::*;
mod battle;
pub use battle::*;
mod bidding;
pub use bidding::*;
mod blow;
pub use blow::*;
mod cards;
pub use cards::*;
mod charity;
pub use charity::*;
mod collection;
pub use collection::*;
mod factions;
pub use factions::*;
mod host;
pub use host::*;
mod movement;
pub use movement::*;
mod revival;
pub use revival::*;
mod setup;
pub use setup::*;
mod storm;
pub use storm::*;

// Interfaces shared by several events. Not glob-exported: their method names clash with
// the `GameEventMethods` ones, so `Event` has a dispatching method for each member.
pub use movement::{location_event, placement};

use crate::prelude::*;

use macros::events;

events! {
    @legacy AcceptOrCancelTleilaxuRevival,
    @legacy AllianceBroken,
    @legacy AllianceByAmbassador,
    @legacy AllianceByTerror,
    @legacy AllianceOffered,
    @legacy AlliancePassed,
    @legacy AllyPermission,
    @legacy AmalPlayed,
    @legacy AmbassadorActivated,
    @legacy AmbassadorPlaced,
    AuditCancelled,
    Audited,
    AutomationConfigured,
    Battle,
    BattleClaimed,
    BattleConcluded,
    BattleInitiated,
    BattleRevision,
    BeneGesseritAccompanies,
    BeneGesseritBattleAnnouncement,
    BeneGesseritFlip,
    BeneGesseritPrediction,
    Bid,
    BlackMarketBid,
    @legacy Bureaucracy,
    Captured,
    Caravan,
    CardGiven,
    @legacy CardTraded,
    CardsDetermined,
    CharityClaimed,
    @legacy ChoamDiscarded,
    @legacy ChoamEconomics,
    @legacy ChoamExtraMove,
    @legacy ChoamFreeRevivalPrevention,
    @legacy ChoamKarmaPrevention,
    @legacy ChoamMovePrevention,
    @legacy ChoamRemoveForce,
    @legacy ClairVoyanceAnswered,
    @legacy ClairVoyancePlayed,
    @legacy DealAccepted,
    @legacy DealOffered,
    @legacy DealRejected,
    Diplomacy,
    @legacy Discarded,
    @legacy DiscardedSearched,
    @legacy DiscardedSearchedAnnounced,
    @legacy DiscardedTaken,
    DiscoveryEntered,
    DiscoveryRevealed,
    @legacy DistransUsed,
    DivideResources,
    DivideResourcesAccepted,
    @legacy Donated,
    EmperorBidSupport,
    EmperorDiscarded,
    EndPhase,
    EstablishPlayers,
    ExtortionPrevented,
    FaceDanced,
    FaceDancerReplaced,
    FaceDancerRevealed,
    FactionSelected,
    FactionTradeOffered,
    FlightDiscoveryUsed,
    FlightUsed,
    FremenRidesMonster,
    FremenSentMonster,
    GuildDelay,
    HMSAdvantageChosen,
    HarvesterPlayed,
    HideSecrets,
    IxianRemovedCardFromAuction,
    IxianSelectedStartingCard,
    IxianSwappedCardOnBid,
    @legacy JuicePlayed,
    @legacy Karma,
    @legacy KarmaChoamDiscard,
    KarmaEcazDial,
    @legacy KarmaFreeRevival,
    KarmaHandSwap,
    KarmaHandSwapInitiated,
    KarmaHmsMovement,
    KarmaMonster,
    KarmaPrescience,
    @legacy KarmaRevivalPrevention,
    KarmaRicheseBuy,
    KarmaShipmentPrevention,
    LoserConcluded,
    MetheorPlayed,
    Move,
    MulliganPerformed,
    NexusCardDrawn,
    @legacy NexusPlayed,
    NexusVoted,
    PerformBeneGesseritPlacement,
    PerformFremenSetup,
    PerformHmsMovement,
    PerformHmsPlacement,
    PerformMoritaniSetup,
    PerformSetup,
    Planetology,
    PlayerReplaced,
    PoisonToothCancelled,
    PortableAntidoteUsed,
    Prescience,
    @legacy RaiseDeadPlayed,
    @legacy RecruitsPlayed,
    ReplacedCardWon,
    @legacy RequestTleilaxuRevival,
    ResidualPlayed,
    @legacy ResourcesAudited,
    @legacy ResourcesTransferred,
    Retreat,
    @legacy Revival,
    RicheseAnnouncesAuction,
    RicheseAnnouncesBlackMarket,
    @legacy RicheseGaveCard,
    RicheseKeepsUnsoldCard,
    RicheseRevealedNoField,
    RicheseSpecifiesAuction,
    RockWasMelted,
    @legacy SetIncreasedRevivalLimits,
    SetShipmentPermission,
    Shipment,
    SkillAssigned,
    StormDialled,
    StormSpellPlayed,
    SwitchedSkilledLeader,
    TakeLosses,
    @legacy TerrorPlanted,
    @legacy TerrorRevealed,
    TestingStationUsed,
    Thought,
    ThoughtAnswered,
    ThumperPlayed,
    TraitorDiscarded,
    TraitorsSelected,
    TreacheryCalled,
    Voice,
    VoiceDeclined,
}

impl std::fmt::Display for EventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

impl std::fmt::Debug for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.event_type().name())
    }
}

/// The rules of the event inside, for a submitter `by` (see [`By`]).
impl Event {
    /// Whether the rules allow the event now.
    pub fn validate(&self, game: &Game, by: By) -> Result<(), Rejection> {
        self.rules().validate(game, by)
    }

    pub fn is_valid(&self, game: &Game, by: By) -> bool {
        self.validate(game, by).is_ok()
    }

    /// The validation `Game::submit` runs (see [`EventRules::try_validate`]).
    pub fn try_validate(&self, game: &Game, by: By) -> GameResult<Result<(), Rejection>> {
        self.rules().try_validate(game, by)
    }

    /// Changes the game, without validating. `Game::submit` is the way to play an event.
    pub(crate) fn apply(&self, game: &mut Game, by: By) -> GameResult {
        self.rules().apply(game, by)
    }

    /// The event's log line.
    pub fn describe(&self, game: &Game, by: By) -> Message {
        self.rules().describe(game, by)
    }

    pub fn try_describe(&self, game: &Game, by: By) -> GameResult<Message> {
        self.rules().try_describe(game, by)
    }

    pub fn describe_short(&self, game: &Game, by: By) -> Message {
        self.rules().describe_short(game, by)
    }

    /// Readies an `@legacy` event for `by`: its base takes the initiator and its player. Does
    /// nothing for a converted event.
    pub fn prepare(&self, game: &Game, by: Option<Faction>) {
        self.rules().prepare(game, by)
    }

    /// The initiator of an `@legacy` event that game state holds (`bidding.current_bid`,
    /// `ship_move.last_shipment_or_movement`, ...). Panics for a converted event: game state
    /// that holds one must keep its submitter itself (a state struct, or a `HistoryEntry`).
    #[track_caller]
    pub fn legacy_initiator(&self) -> Option<Faction> {
        self.legacy_base().map(|b| b.initiator.get()).unwrap_or_else(|| panic!("{} is converted: its initiator isn't in the event", self.event_type().name()))
    }

    /// The player of an `@legacy` event that game state holds. Panics for a converted event,
    /// as [`legacy_initiator`](Event::legacy_initiator) does.
    #[track_caller]
    pub fn legacy_player(&self) -> Option<PlayerId> {
        self.legacy_base().map(|b| b.player.get()).unwrap_or_else(|| panic!("{} is converted: its player isn't in the event", self.event_type().name()))
    }

    /// Prepares the event for `by` and validates it.
    pub fn validate_for(&self, game: &Game, by: Option<Faction>) -> Result<(), Rejection> {
        self.prepare(game, by);
        self.validate(game, By::new(game, by))
    }

    /// `((LocationEvent)this).To`; InvalidCastException for events that do not implement `LocationEvent`.
    pub fn location_event_to(&self, game: &Game) -> Option<Location> {
        match self {
            Event::AmbassadorActivated(e) => <AmbassadorActivated as location_event::LocationEvent>::to(&**e, game),
            Event::BeneGesseritAccompanies(e) => <BeneGesseritAccompanies as location_event::LocationEvent>::to(e, game),
            Event::Caravan(e) => <Caravan as location_event::LocationEvent>::to(e, game),
            Event::DiscoveryEntered(e) => <DiscoveryEntered as location_event::LocationEvent>::to(e, game),
            Event::FremenRidesMonster(e) => <FremenRidesMonster as location_event::LocationEvent>::to(e, game),
            Event::Move(e) => <Move as location_event::LocationEvent>::to(e, game),
            Event::NexusPlayed(e) => <NexusPlayed as location_event::LocationEvent>::to(&**e, game),
            Event::PerformFremenSetup(e) => <PerformFremenSetup as location_event::LocationEvent>::to(e, game),
            Event::PerformSetup(e) => <PerformSetup as location_event::LocationEvent>::to(e, game),
            Event::RaiseDeadPlayed(e) => <RaiseDeadPlayed as location_event::LocationEvent>::to(&**e, game),
            Event::Revival(e) => <Revival as location_event::LocationEvent>::to(&**e, game),
            Event::Shipment(e) => <Shipment as location_event::LocationEvent>::to(e, game),
            Event::TerrorRevealed(e) => <TerrorRevealed as location_event::LocationEvent>::to(&**e, game),
            _ => panic!("{} is not a LocationEvent", self.event_type().name()),
        }
    }

    /// `((LocationEvent)this).TotalAmountOfForcesAddedToLocation`; InvalidCastException for events that do not implement `LocationEvent`.
    pub fn location_event_total_amount_of_forces_added_to_location(&self, game: &Game) -> i32 {
        match self {
            Event::AmbassadorActivated(e) => <AmbassadorActivated as location_event::LocationEvent>::total_amount_of_forces_added_to_location(&**e, game),
            Event::BeneGesseritAccompanies(e) => <BeneGesseritAccompanies as location_event::LocationEvent>::total_amount_of_forces_added_to_location(e, game),
            Event::Caravan(e) => <Caravan as location_event::LocationEvent>::total_amount_of_forces_added_to_location(e, game),
            Event::DiscoveryEntered(e) => <DiscoveryEntered as location_event::LocationEvent>::total_amount_of_forces_added_to_location(e, game),
            Event::FremenRidesMonster(e) => <FremenRidesMonster as location_event::LocationEvent>::total_amount_of_forces_added_to_location(e, game),
            Event::Move(e) => <Move as location_event::LocationEvent>::total_amount_of_forces_added_to_location(e, game),
            Event::NexusPlayed(e) => <NexusPlayed as location_event::LocationEvent>::total_amount_of_forces_added_to_location(&**e, game),
            Event::PerformFremenSetup(e) => <PerformFremenSetup as location_event::LocationEvent>::total_amount_of_forces_added_to_location(e, game),
            Event::PerformSetup(e) => <PerformSetup as location_event::LocationEvent>::total_amount_of_forces_added_to_location(e, game),
            Event::RaiseDeadPlayed(e) => <RaiseDeadPlayed as location_event::LocationEvent>::total_amount_of_forces_added_to_location(&**e, game),
            Event::Revival(e) => <Revival as location_event::LocationEvent>::total_amount_of_forces_added_to_location(&**e, game),
            Event::Shipment(e) => <Shipment as location_event::LocationEvent>::total_amount_of_forces_added_to_location(e, game),
            Event::TerrorRevealed(e) => <TerrorRevealed as location_event::LocationEvent>::total_amount_of_forces_added_to_location(&**e, game),
            _ => panic!("{} is not a LocationEvent", self.event_type().name()),
        }
    }

    /// `((LocationEvent)this).ForcesAddedToLocation`; InvalidCastException for events that do not implement `LocationEvent`.
    pub fn location_event_forces_added_to_location(&self, game: &Game) -> i32 {
        match self {
            Event::AmbassadorActivated(e) => <AmbassadorActivated as location_event::LocationEvent>::forces_added_to_location(&**e, game),
            Event::BeneGesseritAccompanies(e) => <BeneGesseritAccompanies as location_event::LocationEvent>::forces_added_to_location(e, game),
            Event::Caravan(e) => <Caravan as location_event::LocationEvent>::forces_added_to_location(e, game),
            Event::DiscoveryEntered(e) => <DiscoveryEntered as location_event::LocationEvent>::forces_added_to_location(e, game),
            Event::FremenRidesMonster(e) => <FremenRidesMonster as location_event::LocationEvent>::forces_added_to_location(e, game),
            Event::Move(e) => <Move as location_event::LocationEvent>::forces_added_to_location(e, game),
            Event::NexusPlayed(e) => <NexusPlayed as location_event::LocationEvent>::forces_added_to_location(&**e, game),
            Event::PerformFremenSetup(e) => <PerformFremenSetup as location_event::LocationEvent>::forces_added_to_location(e, game),
            Event::PerformSetup(e) => <PerformSetup as location_event::LocationEvent>::forces_added_to_location(e, game),
            Event::RaiseDeadPlayed(e) => <RaiseDeadPlayed as location_event::LocationEvent>::forces_added_to_location(&**e, game),
            Event::Revival(e) => <Revival as location_event::LocationEvent>::forces_added_to_location(&**e, game),
            Event::Shipment(e) => <Shipment as location_event::LocationEvent>::forces_added_to_location(e, game),
            Event::TerrorRevealed(e) => <TerrorRevealed as location_event::LocationEvent>::forces_added_to_location(&**e, game),
            _ => panic!("{} is not a LocationEvent", self.event_type().name()),
        }
    }

    /// `((Placement)this).ForceLocations`; InvalidCastException for events that do not implement `Placement`.
    pub fn placement_force_locations(&self, game: &Game) -> IndexMap<Location, Battalion> {
        match self {
            Event::AmbassadorActivated(e) => <AmbassadorActivated as placement::Placement>::force_locations(&**e, game),
            _ => panic!("{} is not a Placement", self.event_type().name()),
        }
    }

    /// `((Placement)this).To`; InvalidCastException for events that do not implement `Placement`.
    pub fn placement_to(&self, game: &Game) -> Option<Location> {
        match self {
            Event::AmbassadorActivated(e) => <AmbassadorActivated as placement::Placement>::to(&**e, game),
            _ => panic!("{} is not a Placement", self.event_type().name()),
        }
    }

}
