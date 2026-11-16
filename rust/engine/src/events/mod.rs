//! Every game event, grouped by the phase or topic it belongs to, and [`Event`], which holds any
//! of them. The `events!` list below declares them; engine-conventions.md, "Events", says how.

pub(crate) mod macros;
pub mod rules;
pub use rules::*;
mod game_event;

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

// The interface shared by the events that move forces to a location. Not glob-exported:
// `Event` has a dispatching method for each member.
pub use movement::location_event;

use crate::prelude::*;

use macros::events;

events! {
    AcceptOrCancelTleilaxuRevival,
    AllianceBroken,
    AllianceByAmbassador,
    AllianceByTerror,
    AllianceOffered,
    AlliancePassed,
    AllyPermission,
    AmalPlayed,
    AmbassadorActivated,
    AmbassadorPlaced,
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
    Bureaucracy,
    Captured,
    Caravan,
    CardGiven,
    CardTraded,
    CardsDetermined,
    CharityClaimed,
    ChoamDiscarded,
    ChoamEconomics,
    ChoamExtraMove,
    ChoamFreeRevivalPrevention,
    ChoamKarmaPrevention,
    ChoamMovePrevention,
    ChoamRemoveForce,
    ClairVoyanceAnswered,
    ClairVoyancePlayed,
    DealAccepted,
    DealOffered,
    DealRejected,
    Diplomacy,
    Discarded,
    DiscardedSearched,
    DiscardedSearchedAnnounced,
    DiscardedTaken,
    DiscoveryEntered,
    DiscoveryRevealed,
    DistransUsed,
    DivideResources,
    DivideResourcesAccepted,
    Donated,
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
    JuicePlayed,
    Karma,
    KarmaChoamDiscard,
    KarmaEcazDial,
    KarmaFreeRevival,
    KarmaHandSwap,
    KarmaHandSwapInitiated,
    KarmaHmsMovement,
    KarmaMonster,
    KarmaPrescience,
    KarmaRevivalPrevention,
    KarmaRicheseBuy,
    KarmaShipmentPrevention,
    LoserConcluded,
    MetheorPlayed,
    Move,
    MulliganPerformed,
    NexusCardDrawn,
    NexusPlayed,
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
    RaiseDeadPlayed,
    RecruitsPlayed,
    ReplacedCardWon,
    RequestTleilaxuRevival,
    ResidualPlayed,
    ResourcesAudited,
    ResourcesTransferred,
    Retreat,
    Revival,
    RicheseAnnouncesAuction,
    RicheseAnnouncesBlackMarket,
    RicheseGaveCard,
    RicheseKeepsUnsoldCard,
    RicheseRevealedNoField,
    RicheseSpecifiesAuction,
    RockWasMelted,
    SetIncreasedRevivalLimits,
    SetShipmentPermission,
    Shipment,
    SkillAssigned,
    StormDialled,
    StormSpellPlayed,
    SwitchedSkilledLeader,
    TakeLosses,
    TerrorPlanted,
    TerrorRevealed,
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

    /// Validates the event for the submitter `by`.
    pub fn validate_for(&self, game: &Game, by: Option<Faction>) -> Result<(), Rejection> {
        self.validate(game, By::new(game, by))
    }

    /// `((LocationEvent)this).To`; InvalidCastException for events that do not implement `LocationEvent`.
    pub fn location_event_to(&self, game: &Game) -> Option<Location> {
        match self {
            Event::AmbassadorActivated(e) => <AmbassadorActivated as location_event::LocationEvent>::to(e, game),
            Event::BeneGesseritAccompanies(e) => <BeneGesseritAccompanies as location_event::LocationEvent>::to(e, game),
            Event::Caravan(e) => <Caravan as location_event::LocationEvent>::to(e, game),
            Event::DiscoveryEntered(e) => <DiscoveryEntered as location_event::LocationEvent>::to(e, game),
            Event::FremenRidesMonster(e) => <FremenRidesMonster as location_event::LocationEvent>::to(e, game),
            Event::Move(e) => <Move as location_event::LocationEvent>::to(e, game),
            Event::NexusPlayed(e) => <NexusPlayed as location_event::LocationEvent>::to(e, game),
            Event::PerformFremenSetup(e) => <PerformFremenSetup as location_event::LocationEvent>::to(e, game),
            Event::PerformSetup(e) => <PerformSetup as location_event::LocationEvent>::to(e, game),
            Event::RaiseDeadPlayed(e) => <RaiseDeadPlayed as location_event::LocationEvent>::to(e, game),
            Event::Revival(e) => <Revival as location_event::LocationEvent>::to(e, game),
            Event::Shipment(e) => <Shipment as location_event::LocationEvent>::to(e, game),
            Event::TerrorRevealed(e) => <TerrorRevealed as location_event::LocationEvent>::to(e, game),
            _ => panic!("{} is not a LocationEvent", self.event_type().name()),
        }
    }

    /// `((LocationEvent)this).TotalAmountOfForcesAddedToLocation`; InvalidCastException for events that do not implement `LocationEvent`.
    pub fn location_event_total_amount_of_forces_added_to_location(&self, game: &Game) -> i32 {
        match self {
            Event::AmbassadorActivated(e) => <AmbassadorActivated as location_event::LocationEvent>::total_amount_of_forces_added_to_location(e, game),
            Event::BeneGesseritAccompanies(e) => <BeneGesseritAccompanies as location_event::LocationEvent>::total_amount_of_forces_added_to_location(e, game),
            Event::Caravan(e) => <Caravan as location_event::LocationEvent>::total_amount_of_forces_added_to_location(e, game),
            Event::DiscoveryEntered(e) => <DiscoveryEntered as location_event::LocationEvent>::total_amount_of_forces_added_to_location(e, game),
            Event::FremenRidesMonster(e) => <FremenRidesMonster as location_event::LocationEvent>::total_amount_of_forces_added_to_location(e, game),
            Event::Move(e) => <Move as location_event::LocationEvent>::total_amount_of_forces_added_to_location(e, game),
            Event::NexusPlayed(e) => <NexusPlayed as location_event::LocationEvent>::total_amount_of_forces_added_to_location(e, game),
            Event::PerformFremenSetup(e) => <PerformFremenSetup as location_event::LocationEvent>::total_amount_of_forces_added_to_location(e, game),
            Event::PerformSetup(e) => <PerformSetup as location_event::LocationEvent>::total_amount_of_forces_added_to_location(e, game),
            Event::RaiseDeadPlayed(e) => <RaiseDeadPlayed as location_event::LocationEvent>::total_amount_of_forces_added_to_location(e, game),
            Event::Revival(e) => <Revival as location_event::LocationEvent>::total_amount_of_forces_added_to_location(e, game),
            Event::Shipment(e) => <Shipment as location_event::LocationEvent>::total_amount_of_forces_added_to_location(e, game),
            Event::TerrorRevealed(e) => <TerrorRevealed as location_event::LocationEvent>::total_amount_of_forces_added_to_location(e, game),
            _ => panic!("{} is not a LocationEvent", self.event_type().name()),
        }
    }

    /// `((LocationEvent)this).ForcesAddedToLocation`; InvalidCastException for events that do not implement `LocationEvent`.
    pub fn location_event_forces_added_to_location(&self, game: &Game) -> i32 {
        match self {
            Event::AmbassadorActivated(e) => <AmbassadorActivated as location_event::LocationEvent>::forces_added_to_location(e, game),
            Event::BeneGesseritAccompanies(e) => <BeneGesseritAccompanies as location_event::LocationEvent>::forces_added_to_location(e, game),
            Event::Caravan(e) => <Caravan as location_event::LocationEvent>::forces_added_to_location(e, game),
            Event::DiscoveryEntered(e) => <DiscoveryEntered as location_event::LocationEvent>::forces_added_to_location(e, game),
            Event::FremenRidesMonster(e) => <FremenRidesMonster as location_event::LocationEvent>::forces_added_to_location(e, game),
            Event::Move(e) => <Move as location_event::LocationEvent>::forces_added_to_location(e, game),
            Event::NexusPlayed(e) => <NexusPlayed as location_event::LocationEvent>::forces_added_to_location(e, game),
            Event::PerformFremenSetup(e) => <PerformFremenSetup as location_event::LocationEvent>::forces_added_to_location(e, game),
            Event::PerformSetup(e) => <PerformSetup as location_event::LocationEvent>::forces_added_to_location(e, game),
            Event::RaiseDeadPlayed(e) => <RaiseDeadPlayed as location_event::LocationEvent>::forces_added_to_location(e, game),
            Event::Revival(e) => <Revival as location_event::LocationEvent>::forces_added_to_location(e, game),
            Event::Shipment(e) => <Shipment as location_event::LocationEvent>::forces_added_to_location(e, game),
            Event::TerrorRevealed(e) => <TerrorRevealed as location_event::LocationEvent>::forces_added_to_location(e, game),
            _ => panic!("{} is not a LocationEvent", self.event_type().name()),
        }
    }
}
