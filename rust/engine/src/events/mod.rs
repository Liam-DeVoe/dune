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
pub use bidding::bid_trait;
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
    @legacy AuditCancelled,
    @legacy Audited,
    @legacy AutomationConfigured,
    @legacy Battle,
    @legacy BattleClaimed,
    @legacy BattleConcluded,
    @legacy BattleInitiated,
    @legacy BattleRevision,
    @legacy BeneGesseritAccompanies,
    @legacy BeneGesseritBattleAnnouncement,
    @legacy BeneGesseritFlip,
    @legacy BeneGesseritPrediction,
    @legacy Bid,
    @legacy BlackMarketBid,
    @legacy Bureaucracy,
    @legacy Captured,
    @legacy Caravan,
    @legacy CardGiven,
    @legacy CardTraded,
    @legacy CardsDetermined,
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
    @legacy Diplomacy,
    @legacy Discarded,
    @legacy DiscardedSearched,
    @legacy DiscardedSearchedAnnounced,
    @legacy DiscardedTaken,
    @legacy DiscoveryEntered,
    @legacy DiscoveryRevealed,
    @legacy DistransUsed,
    DivideResources,
    DivideResourcesAccepted,
    @legacy Donated,
    @legacy EmperorBidSupport,
    @legacy EmperorDiscarded,
    @legacy EndPhase,
    @legacy EstablishPlayers,
    ExtortionPrevented,
    @legacy FaceDanced,
    @legacy FaceDancerReplaced,
    @legacy FaceDancerRevealed,
    @legacy FactionSelected,
    @legacy FactionTradeOffered,
    @legacy FlightDiscoveryUsed,
    @legacy FlightUsed,
    @legacy FremenRidesMonster,
    @legacy FremenSentMonster,
    @legacy GuildDelay,
    @legacy HMSAdvantageChosen,
    @legacy HarvesterPlayed,
    @legacy HideSecrets,
    @legacy IxianRemovedCardFromAuction,
    @legacy IxianSelectedStartingCard,
    @legacy IxianSwappedCardOnBid,
    @legacy JuicePlayed,
    @legacy Karma,
    @legacy KarmaChoamDiscard,
    @legacy KarmaEcazDial,
    @legacy KarmaFreeRevival,
    @legacy KarmaHandSwap,
    @legacy KarmaHandSwapInitiated,
    @legacy KarmaHmsMovement,
    @legacy KarmaMonster,
    @legacy KarmaPrescience,
    @legacy KarmaRevivalPrevention,
    @legacy KarmaRicheseBuy,
    @legacy KarmaShipmentPrevention,
    @legacy LoserConcluded,
    @legacy MetheorPlayed,
    @legacy Move,
    @legacy MulliganPerformed,
    @legacy NexusCardDrawn,
    @legacy NexusPlayed,
    @legacy NexusVoted,
    @legacy PerformBeneGesseritPlacement,
    @legacy PerformFremenSetup,
    @legacy PerformHmsMovement,
    @legacy PerformHmsPlacement,
    @legacy PerformMoritaniSetup,
    @legacy PerformSetup,
    @legacy Planetology,
    @legacy PlayerReplaced,
    @legacy PoisonToothCancelled,
    @legacy PortableAntidoteUsed,
    @legacy Prescience,
    @legacy RaiseDeadPlayed,
    @legacy RecruitsPlayed,
    @legacy ReplacedCardWon,
    @legacy RequestTleilaxuRevival,
    @legacy ResidualPlayed,
    @legacy ResourcesAudited,
    @legacy ResourcesTransferred,
    @legacy Retreat,
    @legacy Revival,
    @legacy RicheseAnnouncesAuction,
    @legacy RicheseAnnouncesBlackMarket,
    @legacy RicheseGaveCard,
    @legacy RicheseKeepsUnsoldCard,
    @legacy RicheseRevealedNoField,
    @legacy RicheseSpecifiesAuction,
    @legacy RockWasMelted,
    @legacy SetIncreasedRevivalLimits,
    @legacy SetShipmentPermission,
    @legacy Shipment,
    @legacy SkillAssigned,
    @legacy StormDialled,
    @legacy StormSpellPlayed,
    @legacy SwitchedSkilledLeader,
    @legacy TakeLosses,
    @legacy TerrorPlanted,
    @legacy TerrorRevealed,
    @legacy TestingStationUsed,
    @legacy Thought,
    @legacy ThoughtAnswered,
    @legacy ThumperPlayed,
    @legacy TraitorDiscarded,
    @legacy TraitorsSelected,
    @legacy TreacheryCalled,
    @legacy Voice,
    @legacy VoiceDeclined,
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

    /// `((BidEvent)this).TotalAmount`; InvalidCastException for events that do not implement `BidEvent`.
    pub fn bid_total_amount(&self) -> i32 {
        match self {
            Event::Bid(e) => <Bid as bid_trait::BidEvent>::total_amount(&**e),
            Event::BlackMarketBid(e) => <BlackMarketBid as bid_trait::BidEvent>::total_amount(&**e),
            _ => panic!("{} is not a BidEvent", self.event_type().name()),
        }
    }

    /// `((BidEvent)this).Amount`; InvalidCastException for events that do not implement `BidEvent`.
    pub fn bid_amount(&self) -> i32 {
        match self {
            Event::Bid(e) => <Bid as bid_trait::BidEvent>::amount(&**e),
            Event::BlackMarketBid(e) => <BlackMarketBid as bid_trait::BidEvent>::amount(&**e),
            _ => panic!("{} is not a BidEvent", self.event_type().name()),
        }
    }

    /// `((BidEvent)this).AllyContributionAmount`; InvalidCastException for events that do not implement `BidEvent`.
    pub fn bid_ally_contribution_amount(&self) -> i32 {
        match self {
            Event::Bid(e) => <Bid as bid_trait::BidEvent>::ally_contribution_amount(&**e),
            Event::BlackMarketBid(e) => <BlackMarketBid as bid_trait::BidEvent>::ally_contribution_amount(&**e),
            _ => panic!("{} is not a BidEvent", self.event_type().name()),
        }
    }

    /// `((BidEvent)this).EmperorContributionAmount`; InvalidCastException for events that do not implement `BidEvent`.
    pub fn bid_emperor_contribution_amount(&self) -> i32 {
        match self {
            Event::Bid(e) => <Bid as bid_trait::BidEvent>::emperor_contribution_amount(&**e),
            Event::BlackMarketBid(e) => <BlackMarketBid as bid_trait::BidEvent>::emperor_contribution_amount(&**e),
            _ => panic!("{} is not a BidEvent", self.event_type().name()),
        }
    }

    /// `((BidEvent)this).Initiator`; InvalidCastException for events that do not implement `BidEvent`.
    pub fn bid_initiator(&self) -> Faction {
        match self {
            Event::Bid(e) => <Bid as bid_trait::BidEvent>::initiator(&**e),
            Event::BlackMarketBid(e) => <BlackMarketBid as bid_trait::BidEvent>::initiator(&**e),
            _ => panic!("{} is not a BidEvent", self.event_type().name()),
        }
    }

    /// `((BidEvent)this).Player`; InvalidCastException for events that do not implement `BidEvent`.
    pub fn bid_player(&self) -> Option<PlayerId> {
        match self {
            Event::Bid(e) => <Bid as bid_trait::BidEvent>::player(&**e),
            Event::BlackMarketBid(e) => <BlackMarketBid as bid_trait::BidEvent>::player(&**e),
            _ => panic!("{} is not a BidEvent", self.event_type().name()),
        }
    }

    /// `((BidEvent)this).Passed`; InvalidCastException for events that do not implement `BidEvent`.
    pub fn bid_passed(&self) -> bool {
        match self {
            Event::Bid(e) => <Bid as bid_trait::BidEvent>::passed(&**e),
            Event::BlackMarketBid(e) => <BlackMarketBid as bid_trait::BidEvent>::passed(&**e),
            _ => panic!("{} is not a BidEvent", self.event_type().name()),
        }
    }

    /// `((BidEvent)this).UsingKarmaToRemoveBidLimit`; InvalidCastException for events that do not implement `BidEvent`.
    pub fn bid_using_karma_to_remove_bid_limit(&self) -> bool {
        match self {
            Event::Bid(e) => <Bid as bid_trait::BidEvent>::using_karma_to_remove_bid_limit(&**e),
            Event::BlackMarketBid(e) => <BlackMarketBid as bid_trait::BidEvent>::using_karma_to_remove_bid_limit(&**e),
            _ => panic!("{} is not a BidEvent", self.event_type().name()),
        }
    }

    /// `((BidEvent)this).KarmaCard`; InvalidCastException for events that do not implement `BidEvent`.
    pub fn bid_karma_card(&self) -> Option<TreacheryCard> {
        match self {
            Event::Bid(e) => <Bid as bid_trait::BidEvent>::karma_card(&**e),
            Event::BlackMarketBid(e) => <BlackMarketBid as bid_trait::BidEvent>::karma_card(&**e),
            _ => panic!("{} is not a BidEvent", self.event_type().name()),
        }
    }

    /// `((BidEvent)this).KarmaBid`; InvalidCastException for events that do not implement `BidEvent`.
    pub fn bid_karma_bid(&self) -> bool {
        match self {
            Event::Bid(e) => <Bid as bid_trait::BidEvent>::karma_bid(&**e),
            Event::BlackMarketBid(e) => <BlackMarketBid as bid_trait::BidEvent>::karma_bid(&**e),
            _ => panic!("{} is not a BidEvent", self.event_type().name()),
        }
    }

    /// `((LocationEvent)this).To`; InvalidCastException for events that do not implement `LocationEvent`.
    pub fn location_event_to(&self, game: &Game) -> Option<Location> {
        match self {
            Event::AmbassadorActivated(e) => <AmbassadorActivated as location_event::LocationEvent>::to(&**e, game),
            Event::BeneGesseritAccompanies(e) => <BeneGesseritAccompanies as location_event::LocationEvent>::to(&**e, game),
            Event::Caravan(e) => <Caravan as location_event::LocationEvent>::to(&**e, game),
            Event::DiscoveryEntered(e) => <DiscoveryEntered as location_event::LocationEvent>::to(&**e, game),
            Event::FaceDanced(e) => <FaceDanced as location_event::LocationEvent>::to(&**e, game),
            Event::FremenRidesMonster(e) => <FremenRidesMonster as location_event::LocationEvent>::to(&**e, game),
            Event::Move(e) => <Move as location_event::LocationEvent>::to(&**e, game),
            Event::NexusPlayed(e) => <NexusPlayed as location_event::LocationEvent>::to(&**e, game),
            Event::PerformFremenSetup(e) => <PerformFremenSetup as location_event::LocationEvent>::to(&**e, game),
            Event::PerformSetup(e) => <PerformSetup as location_event::LocationEvent>::to(&**e, game),
            Event::RaiseDeadPlayed(e) => <RaiseDeadPlayed as location_event::LocationEvent>::to(&**e, game),
            Event::Revival(e) => <Revival as location_event::LocationEvent>::to(&**e, game),
            Event::Shipment(e) => <Shipment as location_event::LocationEvent>::to(&**e, game),
            Event::TerrorRevealed(e) => <TerrorRevealed as location_event::LocationEvent>::to(&**e, game),
            _ => panic!("{} is not a LocationEvent", self.event_type().name()),
        }
    }

    /// `((LocationEvent)this).TotalAmountOfForcesAddedToLocation`; InvalidCastException for events that do not implement `LocationEvent`.
    pub fn location_event_total_amount_of_forces_added_to_location(&self, game: &Game) -> i32 {
        match self {
            Event::AmbassadorActivated(e) => <AmbassadorActivated as location_event::LocationEvent>::total_amount_of_forces_added_to_location(&**e, game),
            Event::BeneGesseritAccompanies(e) => <BeneGesseritAccompanies as location_event::LocationEvent>::total_amount_of_forces_added_to_location(&**e, game),
            Event::Caravan(e) => <Caravan as location_event::LocationEvent>::total_amount_of_forces_added_to_location(&**e, game),
            Event::DiscoveryEntered(e) => <DiscoveryEntered as location_event::LocationEvent>::total_amount_of_forces_added_to_location(&**e, game),
            Event::FaceDanced(e) => <FaceDanced as location_event::LocationEvent>::total_amount_of_forces_added_to_location(&**e, game),
            Event::FremenRidesMonster(e) => <FremenRidesMonster as location_event::LocationEvent>::total_amount_of_forces_added_to_location(&**e, game),
            Event::Move(e) => <Move as location_event::LocationEvent>::total_amount_of_forces_added_to_location(&**e, game),
            Event::NexusPlayed(e) => <NexusPlayed as location_event::LocationEvent>::total_amount_of_forces_added_to_location(&**e, game),
            Event::PerformFremenSetup(e) => <PerformFremenSetup as location_event::LocationEvent>::total_amount_of_forces_added_to_location(&**e, game),
            Event::PerformSetup(e) => <PerformSetup as location_event::LocationEvent>::total_amount_of_forces_added_to_location(&**e, game),
            Event::RaiseDeadPlayed(e) => <RaiseDeadPlayed as location_event::LocationEvent>::total_amount_of_forces_added_to_location(&**e, game),
            Event::Revival(e) => <Revival as location_event::LocationEvent>::total_amount_of_forces_added_to_location(&**e, game),
            Event::Shipment(e) => <Shipment as location_event::LocationEvent>::total_amount_of_forces_added_to_location(&**e, game),
            Event::TerrorRevealed(e) => <TerrorRevealed as location_event::LocationEvent>::total_amount_of_forces_added_to_location(&**e, game),
            _ => panic!("{} is not a LocationEvent", self.event_type().name()),
        }
    }

    /// `((LocationEvent)this).ForcesAddedToLocation`; InvalidCastException for events that do not implement `LocationEvent`.
    pub fn location_event_forces_added_to_location(&self, game: &Game) -> i32 {
        match self {
            Event::AmbassadorActivated(e) => <AmbassadorActivated as location_event::LocationEvent>::forces_added_to_location(&**e, game),
            Event::BeneGesseritAccompanies(e) => <BeneGesseritAccompanies as location_event::LocationEvent>::forces_added_to_location(&**e, game),
            Event::Caravan(e) => <Caravan as location_event::LocationEvent>::forces_added_to_location(&**e, game),
            Event::DiscoveryEntered(e) => <DiscoveryEntered as location_event::LocationEvent>::forces_added_to_location(&**e, game),
            Event::FaceDanced(e) => <FaceDanced as location_event::LocationEvent>::forces_added_to_location(&**e, game),
            Event::FremenRidesMonster(e) => <FremenRidesMonster as location_event::LocationEvent>::forces_added_to_location(&**e, game),
            Event::Move(e) => <Move as location_event::LocationEvent>::forces_added_to_location(&**e, game),
            Event::NexusPlayed(e) => <NexusPlayed as location_event::LocationEvent>::forces_added_to_location(&**e, game),
            Event::PerformFremenSetup(e) => <PerformFremenSetup as location_event::LocationEvent>::forces_added_to_location(&**e, game),
            Event::PerformSetup(e) => <PerformSetup as location_event::LocationEvent>::forces_added_to_location(&**e, game),
            Event::RaiseDeadPlayed(e) => <RaiseDeadPlayed as location_event::LocationEvent>::forces_added_to_location(&**e, game),
            Event::Revival(e) => <Revival as location_event::LocationEvent>::forces_added_to_location(&**e, game),
            Event::Shipment(e) => <Shipment as location_event::LocationEvent>::forces_added_to_location(&**e, game),
            Event::TerrorRevealed(e) => <TerrorRevealed as location_event::LocationEvent>::forces_added_to_location(&**e, game),
            _ => panic!("{} is not a LocationEvent", self.event_type().name()),
        }
    }

    /// `((Placement)this).ForceLocations`; InvalidCastException for events that do not implement `Placement`.
    pub fn placement_force_locations(&self, game: &Game) -> IndexMap<Location, Battalion> {
        match self {
            Event::AmbassadorActivated(e) => <AmbassadorActivated as placement::Placement>::force_locations(&**e, game),
            Event::Caravan(e) => <Caravan as placement::Placement>::force_locations(&**e, game),
            Event::DiscoveryEntered(e) => <DiscoveryEntered as placement::Placement>::force_locations(&**e, game),
            Event::FaceDanced(e) => <FaceDanced as placement::Placement>::force_locations(&**e, game),
            Event::FremenRidesMonster(e) => <FremenRidesMonster as placement::Placement>::force_locations(&**e, game),
            Event::Move(e) => <Move as placement::Placement>::force_locations(&**e, game),
            Event::PerformFremenSetup(e) => <PerformFremenSetup as placement::Placement>::force_locations(&**e, game),
            Event::PerformSetup(e) => <PerformSetup as placement::Placement>::force_locations(&**e, game),
            _ => panic!("{} is not a Placement", self.event_type().name()),
        }
    }

    /// `((Placement)this).To`; InvalidCastException for events that do not implement `Placement`.
    pub fn placement_to(&self, game: &Game) -> Option<Location> {
        match self {
            Event::AmbassadorActivated(e) => <AmbassadorActivated as placement::Placement>::to(&**e, game),
            Event::Caravan(e) => <Caravan as placement::Placement>::to(&**e, game),
            Event::DiscoveryEntered(e) => <DiscoveryEntered as placement::Placement>::to(&**e, game),
            Event::FaceDanced(e) => <FaceDanced as placement::Placement>::to(&**e, game),
            Event::FremenRidesMonster(e) => <FremenRidesMonster as placement::Placement>::to(&**e, game),
            Event::Move(e) => <Move as placement::Placement>::to(&**e, game),
            Event::PerformFremenSetup(e) => <PerformFremenSetup as placement::Placement>::to(&**e, game),
            Event::PerformSetup(e) => <PerformSetup as placement::Placement>::to(&**e, game),
            _ => panic!("{} is not a Placement", self.event_type().name()),
        }
    }

}
