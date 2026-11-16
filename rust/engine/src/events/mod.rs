//! Every game event, grouped by the phase or topic it belongs to, and the polymorphic `Event`.
//!
//! A [`Event`] value is an enum with one variant per concrete event, each holding an `Rc` to
//! the event struct. Cloning shares the object and `==` is reference equality. Game fields typed
//! as a concrete event hold `Rc<ThatEvent>`.

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
use crate::messages::part::Part;

/// `match` over every variant, binding the `&Rc<Event>` to `$e`.
macro_rules! dispatch {
    ($self:expr, $e:ident => $body:expr) => {
        match $self {
            Event::AcceptOrCancelTleilaxuRevival($e) => $body,
            Event::AllianceBroken($e) => $body,
            Event::AllianceByAmbassador($e) => $body,
            Event::AllianceByTerror($e) => $body,
            Event::AllianceOffered($e) => $body,
            Event::AlliancePassed($e) => $body,
            Event::AllyPermission($e) => $body,
            Event::AmalPlayed($e) => $body,
            Event::AmbassadorActivated($e) => $body,
            Event::AmbassadorPlaced($e) => $body,
            Event::AuditCancelled($e) => $body,
            Event::Audited($e) => $body,
            Event::AutomationConfigured($e) => $body,
            Event::Battle($e) => $body,
            Event::BattleClaimed($e) => $body,
            Event::BattleConcluded($e) => $body,
            Event::BattleInitiated($e) => $body,
            Event::BattleRevision($e) => $body,
            Event::BeneGesseritAccompanies($e) => $body,
            Event::BeneGesseritBattleAnnouncement($e) => $body,
            Event::BeneGesseritFlip($e) => $body,
            Event::BeneGesseritPrediction($e) => $body,
            Event::Bid($e) => $body,
            Event::BlackMarketBid($e) => $body,
            Event::Bureaucracy($e) => $body,
            Event::Captured($e) => $body,
            Event::Caravan($e) => $body,
            Event::CardGiven($e) => $body,
            Event::CardTraded($e) => $body,
            Event::CardsDetermined($e) => $body,
            Event::CharityClaimed($e) => $body,
            Event::ChoamDiscarded($e) => $body,
            Event::ChoamEconomics($e) => $body,
            Event::ChoamExtraMove($e) => $body,
            Event::ChoamFreeRevivalPrevention($e) => $body,
            Event::ChoamKarmaPrevention($e) => $body,
            Event::ChoamMovePrevention($e) => $body,
            Event::ChoamRemoveForce($e) => $body,
            Event::ClairVoyanceAnswered($e) => $body,
            Event::ClairVoyancePlayed($e) => $body,
            Event::DealAccepted($e) => $body,
            Event::DealOffered($e) => $body,
            Event::DealRejected($e) => $body,
            Event::Diplomacy($e) => $body,
            Event::Discarded($e) => $body,
            Event::DiscardedSearched($e) => $body,
            Event::DiscardedSearchedAnnounced($e) => $body,
            Event::DiscardedTaken($e) => $body,
            Event::DiscoveryEntered($e) => $body,
            Event::DiscoveryRevealed($e) => $body,
            Event::DistransUsed($e) => $body,
            Event::DivideResources($e) => $body,
            Event::DivideResourcesAccepted($e) => $body,
            Event::Donated($e) => $body,
            Event::EmperorBidSupport($e) => $body,
            Event::EmperorDiscarded($e) => $body,
            Event::EndPhase($e) => $body,
            Event::EstablishPlayers($e) => $body,
            Event::ExtortionPrevented($e) => $body,
            Event::FaceDanced($e) => $body,
            Event::FaceDancerReplaced($e) => $body,
            Event::FaceDancerRevealed($e) => $body,
            Event::FactionSelected($e) => $body,
            Event::FactionTradeOffered($e) => $body,
            Event::FlightDiscoveryUsed($e) => $body,
            Event::FlightUsed($e) => $body,
            Event::FremenRidesMonster($e) => $body,
            Event::FremenSentMonster($e) => $body,
            Event::GuildDelay($e) => $body,
            Event::HMSAdvantageChosen($e) => $body,
            Event::HarvesterPlayed($e) => $body,
            Event::HideSecrets($e) => $body,
            Event::IxianRemovedCardFromAuction($e) => $body,
            Event::IxianSelectedStartingCard($e) => $body,
            Event::IxianSwappedCardOnBid($e) => $body,
            Event::JuicePlayed($e) => $body,
            Event::Karma($e) => $body,
            Event::KarmaChoamDiscard($e) => $body,
            Event::KarmaEcazDial($e) => $body,
            Event::KarmaFreeRevival($e) => $body,
            Event::KarmaHandSwap($e) => $body,
            Event::KarmaHandSwapInitiated($e) => $body,
            Event::KarmaHmsMovement($e) => $body,
            Event::KarmaMonster($e) => $body,
            Event::KarmaPrescience($e) => $body,
            Event::KarmaRevivalPrevention($e) => $body,
            Event::KarmaRicheseBuy($e) => $body,
            Event::KarmaShipmentPrevention($e) => $body,
            Event::LoserConcluded($e) => $body,
            Event::MetheorPlayed($e) => $body,
            Event::Move($e) => $body,
            Event::MulliganPerformed($e) => $body,
            Event::NexusCardDrawn($e) => $body,
            Event::NexusPlayed($e) => $body,
            Event::NexusVoted($e) => $body,
            Event::PerformBeneGesseritPlacement($e) => $body,
            Event::PerformFremenSetup($e) => $body,
            Event::PerformHmsMovement($e) => $body,
            Event::PerformHmsPlacement($e) => $body,
            Event::PerformMoritaniSetup($e) => $body,
            Event::PerformSetup($e) => $body,
            Event::Planetology($e) => $body,
            Event::PlayerReplaced($e) => $body,
            Event::PoisonToothCancelled($e) => $body,
            Event::PortableAntidoteUsed($e) => $body,
            Event::Prescience($e) => $body,
            Event::RaiseDeadPlayed($e) => $body,
            Event::RecruitsPlayed($e) => $body,
            Event::ReplacedCardWon($e) => $body,
            Event::RequestTleilaxuRevival($e) => $body,
            Event::ResidualPlayed($e) => $body,
            Event::ResourcesAudited($e) => $body,
            Event::ResourcesTransferred($e) => $body,
            Event::Retreat($e) => $body,
            Event::Revival($e) => $body,
            Event::RicheseAnnouncesAuction($e) => $body,
            Event::RicheseAnnouncesBlackMarket($e) => $body,
            Event::RicheseGaveCard($e) => $body,
            Event::RicheseKeepsUnsoldCard($e) => $body,
            Event::RicheseRevealedNoField($e) => $body,
            Event::RicheseSpecifiesAuction($e) => $body,
            Event::RockWasMelted($e) => $body,
            Event::SetIncreasedRevivalLimits($e) => $body,
            Event::SetShipmentPermission($e) => $body,
            Event::Shipment($e) => $body,
            Event::SkillAssigned($e) => $body,
            Event::StormDialled($e) => $body,
            Event::StormSpellPlayed($e) => $body,
            Event::SwitchedSkilledLeader($e) => $body,
            Event::TakeLosses($e) => $body,
            Event::TerrorPlanted($e) => $body,
            Event::TerrorRevealed($e) => $body,
            Event::TestingStationUsed($e) => $body,
            Event::Thought($e) => $body,
            Event::ThoughtAnswered($e) => $body,
            Event::ThumperPlayed($e) => $body,
            Event::TraitorDiscarded($e) => $body,
            Event::TraitorsSelected($e) => $body,
            Event::TreacheryCalled($e) => $body,
            Event::Voice($e) => $body,
            Event::VoiceDeclined($e) => $body,
        }
    };
}
pub(crate) use dispatch;

/// A reference to any concrete event.
#[derive(Clone)]
pub enum Event {
    AcceptOrCancelTleilaxuRevival(Rc<AcceptOrCancelTleilaxuRevival>),
    AllianceBroken(Rc<AllianceBroken>),
    AllianceByAmbassador(Rc<AllianceByAmbassador>),
    AllianceByTerror(Rc<AllianceByTerror>),
    AllianceOffered(Rc<AllianceOffered>),
    AlliancePassed(Rc<AlliancePassed>),
    AllyPermission(Rc<AllyPermission>),
    AmalPlayed(Rc<AmalPlayed>),
    AmbassadorActivated(Rc<AmbassadorActivated>),
    AmbassadorPlaced(Rc<AmbassadorPlaced>),
    AuditCancelled(Rc<AuditCancelled>),
    Audited(Rc<Audited>),
    AutomationConfigured(Rc<AutomationConfigured>),
    Battle(Rc<Battle>),
    BattleClaimed(Rc<BattleClaimed>),
    BattleConcluded(Rc<BattleConcluded>),
    BattleInitiated(Rc<BattleInitiated>),
    BattleRevision(Rc<BattleRevision>),
    BeneGesseritAccompanies(Rc<BeneGesseritAccompanies>),
    BeneGesseritBattleAnnouncement(Rc<BeneGesseritBattleAnnouncement>),
    BeneGesseritFlip(Rc<BeneGesseritFlip>),
    BeneGesseritPrediction(Rc<BeneGesseritPrediction>),
    Bid(Rc<Bid>),
    BlackMarketBid(Rc<BlackMarketBid>),
    Bureaucracy(Rc<Bureaucracy>),
    Captured(Rc<Captured>),
    Caravan(Rc<Caravan>),
    CardGiven(Rc<CardGiven>),
    CardTraded(Rc<CardTraded>),
    CardsDetermined(Rc<CardsDetermined>),
    CharityClaimed(Rc<CharityClaimed>),
    ChoamDiscarded(Rc<ChoamDiscarded>),
    ChoamEconomics(Rc<ChoamEconomics>),
    ChoamExtraMove(Rc<ChoamExtraMove>),
    ChoamFreeRevivalPrevention(Rc<ChoamFreeRevivalPrevention>),
    ChoamKarmaPrevention(Rc<ChoamKarmaPrevention>),
    ChoamMovePrevention(Rc<ChoamMovePrevention>),
    ChoamRemoveForce(Rc<ChoamRemoveForce>),
    ClairVoyanceAnswered(Rc<ClairVoyanceAnswered>),
    ClairVoyancePlayed(Rc<ClairVoyancePlayed>),
    DealAccepted(Rc<DealAccepted>),
    DealOffered(Rc<DealOffered>),
    DealRejected(Rc<DealRejected>),
    Diplomacy(Rc<Diplomacy>),
    Discarded(Rc<Discarded>),
    DiscardedSearched(Rc<DiscardedSearched>),
    DiscardedSearchedAnnounced(Rc<DiscardedSearchedAnnounced>),
    DiscardedTaken(Rc<DiscardedTaken>),
    DiscoveryEntered(Rc<DiscoveryEntered>),
    DiscoveryRevealed(Rc<DiscoveryRevealed>),
    DistransUsed(Rc<DistransUsed>),
    DivideResources(Rc<DivideResources>),
    DivideResourcesAccepted(Rc<DivideResourcesAccepted>),
    Donated(Rc<Donated>),
    EmperorBidSupport(Rc<EmperorBidSupport>),
    EmperorDiscarded(Rc<EmperorDiscarded>),
    EndPhase(Rc<EndPhase>),
    EstablishPlayers(Rc<EstablishPlayers>),
    ExtortionPrevented(Rc<ExtortionPrevented>),
    FaceDanced(Rc<FaceDanced>),
    FaceDancerReplaced(Rc<FaceDancerReplaced>),
    FaceDancerRevealed(Rc<FaceDancerRevealed>),
    FactionSelected(Rc<FactionSelected>),
    FactionTradeOffered(Rc<FactionTradeOffered>),
    FlightDiscoveryUsed(Rc<FlightDiscoveryUsed>),
    FlightUsed(Rc<FlightUsed>),
    FremenRidesMonster(Rc<FremenRidesMonster>),
    FremenSentMonster(Rc<FremenSentMonster>),
    GuildDelay(Rc<GuildDelay>),
    HMSAdvantageChosen(Rc<HMSAdvantageChosen>),
    HarvesterPlayed(Rc<HarvesterPlayed>),
    HideSecrets(Rc<HideSecrets>),
    IxianRemovedCardFromAuction(Rc<IxianRemovedCardFromAuction>),
    IxianSelectedStartingCard(Rc<IxianSelectedStartingCard>),
    IxianSwappedCardOnBid(Rc<IxianSwappedCardOnBid>),
    JuicePlayed(Rc<JuicePlayed>),
    Karma(Rc<Karma>),
    KarmaChoamDiscard(Rc<KarmaChoamDiscard>),
    KarmaEcazDial(Rc<KarmaEcazDial>),
    KarmaFreeRevival(Rc<KarmaFreeRevival>),
    KarmaHandSwap(Rc<KarmaHandSwap>),
    KarmaHandSwapInitiated(Rc<KarmaHandSwapInitiated>),
    KarmaHmsMovement(Rc<KarmaHmsMovement>),
    KarmaMonster(Rc<KarmaMonster>),
    KarmaPrescience(Rc<KarmaPrescience>),
    KarmaRevivalPrevention(Rc<KarmaRevivalPrevention>),
    KarmaRicheseBuy(Rc<KarmaRicheseBuy>),
    KarmaShipmentPrevention(Rc<KarmaShipmentPrevention>),
    LoserConcluded(Rc<LoserConcluded>),
    MetheorPlayed(Rc<MetheorPlayed>),
    Move(Rc<Move>),
    MulliganPerformed(Rc<MulliganPerformed>),
    NexusCardDrawn(Rc<NexusCardDrawn>),
    NexusPlayed(Rc<NexusPlayed>),
    NexusVoted(Rc<NexusVoted>),
    PerformBeneGesseritPlacement(Rc<PerformBeneGesseritPlacement>),
    PerformFremenSetup(Rc<PerformFremenSetup>),
    PerformHmsMovement(Rc<PerformHmsMovement>),
    PerformHmsPlacement(Rc<PerformHmsPlacement>),
    PerformMoritaniSetup(Rc<PerformMoritaniSetup>),
    PerformSetup(Rc<PerformSetup>),
    Planetology(Rc<Planetology>),
    PlayerReplaced(Rc<PlayerReplaced>),
    PoisonToothCancelled(Rc<PoisonToothCancelled>),
    PortableAntidoteUsed(Rc<PortableAntidoteUsed>),
    Prescience(Rc<Prescience>),
    RaiseDeadPlayed(Rc<RaiseDeadPlayed>),
    RecruitsPlayed(Rc<RecruitsPlayed>),
    ReplacedCardWon(Rc<ReplacedCardWon>),
    RequestTleilaxuRevival(Rc<RequestTleilaxuRevival>),
    ResidualPlayed(Rc<ResidualPlayed>),
    ResourcesAudited(Rc<ResourcesAudited>),
    ResourcesTransferred(Rc<ResourcesTransferred>),
    Retreat(Rc<Retreat>),
    Revival(Rc<Revival>),
    RicheseAnnouncesAuction(Rc<RicheseAnnouncesAuction>),
    RicheseAnnouncesBlackMarket(Rc<RicheseAnnouncesBlackMarket>),
    RicheseGaveCard(Rc<RicheseGaveCard>),
    RicheseKeepsUnsoldCard(Rc<RicheseKeepsUnsoldCard>),
    RicheseRevealedNoField(Rc<RicheseRevealedNoField>),
    RicheseSpecifiesAuction(Rc<RicheseSpecifiesAuction>),
    RockWasMelted(Rc<RockWasMelted>),
    SetIncreasedRevivalLimits(Rc<SetIncreasedRevivalLimits>),
    SetShipmentPermission(Rc<SetShipmentPermission>),
    Shipment(Rc<Shipment>),
    SkillAssigned(Rc<SkillAssigned>),
    StormDialled(Rc<StormDialled>),
    StormSpellPlayed(Rc<StormSpellPlayed>),
    SwitchedSkilledLeader(Rc<SwitchedSkilledLeader>),
    TakeLosses(Rc<TakeLosses>),
    TerrorPlanted(Rc<TerrorPlanted>),
    TerrorRevealed(Rc<TerrorRevealed>),
    TestingStationUsed(Rc<TestingStationUsed>),
    Thought(Rc<Thought>),
    ThoughtAnswered(Rc<ThoughtAnswered>),
    ThumperPlayed(Rc<ThumperPlayed>),
    TraitorDiscarded(Rc<TraitorDiscarded>),
    TraitorsSelected(Rc<TraitorsSelected>),
    TreacheryCalled(Rc<TreacheryCalled>),
    Voice(Rc<Voice>),
    VoiceDeclined(Rc<VoiceDeclined>),
}

impl From<Rc<AcceptOrCancelTleilaxuRevival>> for Event {
    fn from(e: Rc<AcceptOrCancelTleilaxuRevival>) -> Self {
        Event::AcceptOrCancelTleilaxuRevival(e)
    }
}

impl From<Rc<AllianceBroken>> for Event {
    fn from(e: Rc<AllianceBroken>) -> Self {
        Event::AllianceBroken(e)
    }
}

impl From<Rc<AllianceByAmbassador>> for Event {
    fn from(e: Rc<AllianceByAmbassador>) -> Self {
        Event::AllianceByAmbassador(e)
    }
}

impl From<Rc<AllianceByTerror>> for Event {
    fn from(e: Rc<AllianceByTerror>) -> Self {
        Event::AllianceByTerror(e)
    }
}

impl From<Rc<AllianceOffered>> for Event {
    fn from(e: Rc<AllianceOffered>) -> Self {
        Event::AllianceOffered(e)
    }
}

impl From<Rc<AlliancePassed>> for Event {
    fn from(e: Rc<AlliancePassed>) -> Self {
        Event::AlliancePassed(e)
    }
}

impl From<Rc<AllyPermission>> for Event {
    fn from(e: Rc<AllyPermission>) -> Self {
        Event::AllyPermission(e)
    }
}

impl From<Rc<AmalPlayed>> for Event {
    fn from(e: Rc<AmalPlayed>) -> Self {
        Event::AmalPlayed(e)
    }
}

impl From<Rc<AmbassadorActivated>> for Event {
    fn from(e: Rc<AmbassadorActivated>) -> Self {
        Event::AmbassadorActivated(e)
    }
}

impl From<Rc<AmbassadorPlaced>> for Event {
    fn from(e: Rc<AmbassadorPlaced>) -> Self {
        Event::AmbassadorPlaced(e)
    }
}

impl From<Rc<AuditCancelled>> for Event {
    fn from(e: Rc<AuditCancelled>) -> Self {
        Event::AuditCancelled(e)
    }
}

impl From<Rc<Audited>> for Event {
    fn from(e: Rc<Audited>) -> Self {
        Event::Audited(e)
    }
}

impl From<Rc<AutomationConfigured>> for Event {
    fn from(e: Rc<AutomationConfigured>) -> Self {
        Event::AutomationConfigured(e)
    }
}

impl From<Rc<Battle>> for Event {
    fn from(e: Rc<Battle>) -> Self {
        Event::Battle(e)
    }
}

impl From<Rc<BattleClaimed>> for Event {
    fn from(e: Rc<BattleClaimed>) -> Self {
        Event::BattleClaimed(e)
    }
}

impl From<Rc<BattleConcluded>> for Event {
    fn from(e: Rc<BattleConcluded>) -> Self {
        Event::BattleConcluded(e)
    }
}

impl From<Rc<BattleInitiated>> for Event {
    fn from(e: Rc<BattleInitiated>) -> Self {
        Event::BattleInitiated(e)
    }
}

impl From<Rc<BattleRevision>> for Event {
    fn from(e: Rc<BattleRevision>) -> Self {
        Event::BattleRevision(e)
    }
}

impl From<Rc<BeneGesseritAccompanies>> for Event {
    fn from(e: Rc<BeneGesseritAccompanies>) -> Self {
        Event::BeneGesseritAccompanies(e)
    }
}

impl From<Rc<BeneGesseritBattleAnnouncement>> for Event {
    fn from(e: Rc<BeneGesseritBattleAnnouncement>) -> Self {
        Event::BeneGesseritBattleAnnouncement(e)
    }
}

impl From<Rc<BeneGesseritFlip>> for Event {
    fn from(e: Rc<BeneGesseritFlip>) -> Self {
        Event::BeneGesseritFlip(e)
    }
}

impl From<Rc<BeneGesseritPrediction>> for Event {
    fn from(e: Rc<BeneGesseritPrediction>) -> Self {
        Event::BeneGesseritPrediction(e)
    }
}

impl From<Rc<Bid>> for Event {
    fn from(e: Rc<Bid>) -> Self {
        Event::Bid(e)
    }
}

impl From<Rc<BlackMarketBid>> for Event {
    fn from(e: Rc<BlackMarketBid>) -> Self {
        Event::BlackMarketBid(e)
    }
}

impl From<Rc<Bureaucracy>> for Event {
    fn from(e: Rc<Bureaucracy>) -> Self {
        Event::Bureaucracy(e)
    }
}

impl From<Rc<Captured>> for Event {
    fn from(e: Rc<Captured>) -> Self {
        Event::Captured(e)
    }
}

impl From<Rc<Caravan>> for Event {
    fn from(e: Rc<Caravan>) -> Self {
        Event::Caravan(e)
    }
}

impl From<Rc<CardGiven>> for Event {
    fn from(e: Rc<CardGiven>) -> Self {
        Event::CardGiven(e)
    }
}

impl From<Rc<CardTraded>> for Event {
    fn from(e: Rc<CardTraded>) -> Self {
        Event::CardTraded(e)
    }
}

impl From<Rc<CardsDetermined>> for Event {
    fn from(e: Rc<CardsDetermined>) -> Self {
        Event::CardsDetermined(e)
    }
}

impl From<Rc<CharityClaimed>> for Event {
    fn from(e: Rc<CharityClaimed>) -> Self {
        Event::CharityClaimed(e)
    }
}

impl From<Rc<ChoamDiscarded>> for Event {
    fn from(e: Rc<ChoamDiscarded>) -> Self {
        Event::ChoamDiscarded(e)
    }
}

impl From<Rc<ChoamEconomics>> for Event {
    fn from(e: Rc<ChoamEconomics>) -> Self {
        Event::ChoamEconomics(e)
    }
}

impl From<Rc<ChoamExtraMove>> for Event {
    fn from(e: Rc<ChoamExtraMove>) -> Self {
        Event::ChoamExtraMove(e)
    }
}

impl From<Rc<ChoamFreeRevivalPrevention>> for Event {
    fn from(e: Rc<ChoamFreeRevivalPrevention>) -> Self {
        Event::ChoamFreeRevivalPrevention(e)
    }
}

impl From<Rc<ChoamKarmaPrevention>> for Event {
    fn from(e: Rc<ChoamKarmaPrevention>) -> Self {
        Event::ChoamKarmaPrevention(e)
    }
}

impl From<Rc<ChoamMovePrevention>> for Event {
    fn from(e: Rc<ChoamMovePrevention>) -> Self {
        Event::ChoamMovePrevention(e)
    }
}

impl From<Rc<ChoamRemoveForce>> for Event {
    fn from(e: Rc<ChoamRemoveForce>) -> Self {
        Event::ChoamRemoveForce(e)
    }
}

impl From<Rc<ClairVoyanceAnswered>> for Event {
    fn from(e: Rc<ClairVoyanceAnswered>) -> Self {
        Event::ClairVoyanceAnswered(e)
    }
}

impl From<Rc<ClairVoyancePlayed>> for Event {
    fn from(e: Rc<ClairVoyancePlayed>) -> Self {
        Event::ClairVoyancePlayed(e)
    }
}

impl From<Rc<DealAccepted>> for Event {
    fn from(e: Rc<DealAccepted>) -> Self {
        Event::DealAccepted(e)
    }
}

impl From<Rc<DealOffered>> for Event {
    fn from(e: Rc<DealOffered>) -> Self {
        Event::DealOffered(e)
    }
}

impl From<Rc<DealRejected>> for Event {
    fn from(e: Rc<DealRejected>) -> Self {
        Event::DealRejected(e)
    }
}

impl From<Rc<Diplomacy>> for Event {
    fn from(e: Rc<Diplomacy>) -> Self {
        Event::Diplomacy(e)
    }
}

impl From<Rc<Discarded>> for Event {
    fn from(e: Rc<Discarded>) -> Self {
        Event::Discarded(e)
    }
}

impl From<Rc<DiscardedSearched>> for Event {
    fn from(e: Rc<DiscardedSearched>) -> Self {
        Event::DiscardedSearched(e)
    }
}

impl From<Rc<DiscardedSearchedAnnounced>> for Event {
    fn from(e: Rc<DiscardedSearchedAnnounced>) -> Self {
        Event::DiscardedSearchedAnnounced(e)
    }
}

impl From<Rc<DiscardedTaken>> for Event {
    fn from(e: Rc<DiscardedTaken>) -> Self {
        Event::DiscardedTaken(e)
    }
}

impl From<Rc<DiscoveryEntered>> for Event {
    fn from(e: Rc<DiscoveryEntered>) -> Self {
        Event::DiscoveryEntered(e)
    }
}

impl From<Rc<DiscoveryRevealed>> for Event {
    fn from(e: Rc<DiscoveryRevealed>) -> Self {
        Event::DiscoveryRevealed(e)
    }
}

impl From<Rc<DistransUsed>> for Event {
    fn from(e: Rc<DistransUsed>) -> Self {
        Event::DistransUsed(e)
    }
}

impl From<Rc<DivideResources>> for Event {
    fn from(e: Rc<DivideResources>) -> Self {
        Event::DivideResources(e)
    }
}

impl From<Rc<DivideResourcesAccepted>> for Event {
    fn from(e: Rc<DivideResourcesAccepted>) -> Self {
        Event::DivideResourcesAccepted(e)
    }
}

impl From<Rc<Donated>> for Event {
    fn from(e: Rc<Donated>) -> Self {
        Event::Donated(e)
    }
}

impl From<Rc<EmperorBidSupport>> for Event {
    fn from(e: Rc<EmperorBidSupport>) -> Self {
        Event::EmperorBidSupport(e)
    }
}

impl From<Rc<EmperorDiscarded>> for Event {
    fn from(e: Rc<EmperorDiscarded>) -> Self {
        Event::EmperorDiscarded(e)
    }
}

impl From<Rc<EndPhase>> for Event {
    fn from(e: Rc<EndPhase>) -> Self {
        Event::EndPhase(e)
    }
}

impl From<Rc<EstablishPlayers>> for Event {
    fn from(e: Rc<EstablishPlayers>) -> Self {
        Event::EstablishPlayers(e)
    }
}

impl From<Rc<ExtortionPrevented>> for Event {
    fn from(e: Rc<ExtortionPrevented>) -> Self {
        Event::ExtortionPrevented(e)
    }
}

impl From<Rc<FaceDanced>> for Event {
    fn from(e: Rc<FaceDanced>) -> Self {
        Event::FaceDanced(e)
    }
}

impl From<Rc<FaceDancerReplaced>> for Event {
    fn from(e: Rc<FaceDancerReplaced>) -> Self {
        Event::FaceDancerReplaced(e)
    }
}

impl From<Rc<FaceDancerRevealed>> for Event {
    fn from(e: Rc<FaceDancerRevealed>) -> Self {
        Event::FaceDancerRevealed(e)
    }
}

impl From<Rc<FactionSelected>> for Event {
    fn from(e: Rc<FactionSelected>) -> Self {
        Event::FactionSelected(e)
    }
}

impl From<Rc<FactionTradeOffered>> for Event {
    fn from(e: Rc<FactionTradeOffered>) -> Self {
        Event::FactionTradeOffered(e)
    }
}

impl From<Rc<FlightDiscoveryUsed>> for Event {
    fn from(e: Rc<FlightDiscoveryUsed>) -> Self {
        Event::FlightDiscoveryUsed(e)
    }
}

impl From<Rc<FlightUsed>> for Event {
    fn from(e: Rc<FlightUsed>) -> Self {
        Event::FlightUsed(e)
    }
}

impl From<Rc<FremenRidesMonster>> for Event {
    fn from(e: Rc<FremenRidesMonster>) -> Self {
        Event::FremenRidesMonster(e)
    }
}

impl From<Rc<FremenSentMonster>> for Event {
    fn from(e: Rc<FremenSentMonster>) -> Self {
        Event::FremenSentMonster(e)
    }
}

impl From<Rc<GuildDelay>> for Event {
    fn from(e: Rc<GuildDelay>) -> Self {
        Event::GuildDelay(e)
    }
}

impl From<Rc<HMSAdvantageChosen>> for Event {
    fn from(e: Rc<HMSAdvantageChosen>) -> Self {
        Event::HMSAdvantageChosen(e)
    }
}

impl From<Rc<HarvesterPlayed>> for Event {
    fn from(e: Rc<HarvesterPlayed>) -> Self {
        Event::HarvesterPlayed(e)
    }
}

impl From<Rc<HideSecrets>> for Event {
    fn from(e: Rc<HideSecrets>) -> Self {
        Event::HideSecrets(e)
    }
}

impl From<Rc<IxianRemovedCardFromAuction>> for Event {
    fn from(e: Rc<IxianRemovedCardFromAuction>) -> Self {
        Event::IxianRemovedCardFromAuction(e)
    }
}

impl From<Rc<IxianSelectedStartingCard>> for Event {
    fn from(e: Rc<IxianSelectedStartingCard>) -> Self {
        Event::IxianSelectedStartingCard(e)
    }
}

impl From<Rc<IxianSwappedCardOnBid>> for Event {
    fn from(e: Rc<IxianSwappedCardOnBid>) -> Self {
        Event::IxianSwappedCardOnBid(e)
    }
}

impl From<Rc<JuicePlayed>> for Event {
    fn from(e: Rc<JuicePlayed>) -> Self {
        Event::JuicePlayed(e)
    }
}

impl From<Rc<Karma>> for Event {
    fn from(e: Rc<Karma>) -> Self {
        Event::Karma(e)
    }
}

impl From<Rc<KarmaChoamDiscard>> for Event {
    fn from(e: Rc<KarmaChoamDiscard>) -> Self {
        Event::KarmaChoamDiscard(e)
    }
}

impl From<Rc<KarmaEcazDial>> for Event {
    fn from(e: Rc<KarmaEcazDial>) -> Self {
        Event::KarmaEcazDial(e)
    }
}

impl From<Rc<KarmaFreeRevival>> for Event {
    fn from(e: Rc<KarmaFreeRevival>) -> Self {
        Event::KarmaFreeRevival(e)
    }
}

impl From<Rc<KarmaHandSwap>> for Event {
    fn from(e: Rc<KarmaHandSwap>) -> Self {
        Event::KarmaHandSwap(e)
    }
}

impl From<Rc<KarmaHandSwapInitiated>> for Event {
    fn from(e: Rc<KarmaHandSwapInitiated>) -> Self {
        Event::KarmaHandSwapInitiated(e)
    }
}

impl From<Rc<KarmaHmsMovement>> for Event {
    fn from(e: Rc<KarmaHmsMovement>) -> Self {
        Event::KarmaHmsMovement(e)
    }
}

impl From<Rc<KarmaMonster>> for Event {
    fn from(e: Rc<KarmaMonster>) -> Self {
        Event::KarmaMonster(e)
    }
}

impl From<Rc<KarmaPrescience>> for Event {
    fn from(e: Rc<KarmaPrescience>) -> Self {
        Event::KarmaPrescience(e)
    }
}

impl From<Rc<KarmaRevivalPrevention>> for Event {
    fn from(e: Rc<KarmaRevivalPrevention>) -> Self {
        Event::KarmaRevivalPrevention(e)
    }
}

impl From<Rc<KarmaRicheseBuy>> for Event {
    fn from(e: Rc<KarmaRicheseBuy>) -> Self {
        Event::KarmaRicheseBuy(e)
    }
}

impl From<Rc<KarmaShipmentPrevention>> for Event {
    fn from(e: Rc<KarmaShipmentPrevention>) -> Self {
        Event::KarmaShipmentPrevention(e)
    }
}

impl From<Rc<LoserConcluded>> for Event {
    fn from(e: Rc<LoserConcluded>) -> Self {
        Event::LoserConcluded(e)
    }
}

impl From<Rc<MetheorPlayed>> for Event {
    fn from(e: Rc<MetheorPlayed>) -> Self {
        Event::MetheorPlayed(e)
    }
}

impl From<Rc<Move>> for Event {
    fn from(e: Rc<Move>) -> Self {
        Event::Move(e)
    }
}

impl From<Rc<MulliganPerformed>> for Event {
    fn from(e: Rc<MulliganPerformed>) -> Self {
        Event::MulliganPerformed(e)
    }
}

impl From<Rc<NexusCardDrawn>> for Event {
    fn from(e: Rc<NexusCardDrawn>) -> Self {
        Event::NexusCardDrawn(e)
    }
}

impl From<Rc<NexusPlayed>> for Event {
    fn from(e: Rc<NexusPlayed>) -> Self {
        Event::NexusPlayed(e)
    }
}

impl From<Rc<NexusVoted>> for Event {
    fn from(e: Rc<NexusVoted>) -> Self {
        Event::NexusVoted(e)
    }
}

impl From<Rc<PerformBeneGesseritPlacement>> for Event {
    fn from(e: Rc<PerformBeneGesseritPlacement>) -> Self {
        Event::PerformBeneGesseritPlacement(e)
    }
}

impl From<Rc<PerformFremenSetup>> for Event {
    fn from(e: Rc<PerformFremenSetup>) -> Self {
        Event::PerformFremenSetup(e)
    }
}

impl From<Rc<PerformHmsMovement>> for Event {
    fn from(e: Rc<PerformHmsMovement>) -> Self {
        Event::PerformHmsMovement(e)
    }
}

impl From<Rc<PerformHmsPlacement>> for Event {
    fn from(e: Rc<PerformHmsPlacement>) -> Self {
        Event::PerformHmsPlacement(e)
    }
}

impl From<Rc<PerformMoritaniSetup>> for Event {
    fn from(e: Rc<PerformMoritaniSetup>) -> Self {
        Event::PerformMoritaniSetup(e)
    }
}

impl From<Rc<PerformSetup>> for Event {
    fn from(e: Rc<PerformSetup>) -> Self {
        Event::PerformSetup(e)
    }
}

impl From<Rc<Planetology>> for Event {
    fn from(e: Rc<Planetology>) -> Self {
        Event::Planetology(e)
    }
}

impl From<Rc<PlayerReplaced>> for Event {
    fn from(e: Rc<PlayerReplaced>) -> Self {
        Event::PlayerReplaced(e)
    }
}

impl From<Rc<PoisonToothCancelled>> for Event {
    fn from(e: Rc<PoisonToothCancelled>) -> Self {
        Event::PoisonToothCancelled(e)
    }
}

impl From<Rc<PortableAntidoteUsed>> for Event {
    fn from(e: Rc<PortableAntidoteUsed>) -> Self {
        Event::PortableAntidoteUsed(e)
    }
}

impl From<Rc<Prescience>> for Event {
    fn from(e: Rc<Prescience>) -> Self {
        Event::Prescience(e)
    }
}

impl From<Rc<RaiseDeadPlayed>> for Event {
    fn from(e: Rc<RaiseDeadPlayed>) -> Self {
        Event::RaiseDeadPlayed(e)
    }
}

impl From<Rc<RecruitsPlayed>> for Event {
    fn from(e: Rc<RecruitsPlayed>) -> Self {
        Event::RecruitsPlayed(e)
    }
}

impl From<Rc<ReplacedCardWon>> for Event {
    fn from(e: Rc<ReplacedCardWon>) -> Self {
        Event::ReplacedCardWon(e)
    }
}

impl From<Rc<RequestTleilaxuRevival>> for Event {
    fn from(e: Rc<RequestTleilaxuRevival>) -> Self {
        Event::RequestTleilaxuRevival(e)
    }
}

impl From<Rc<ResidualPlayed>> for Event {
    fn from(e: Rc<ResidualPlayed>) -> Self {
        Event::ResidualPlayed(e)
    }
}

impl From<Rc<ResourcesAudited>> for Event {
    fn from(e: Rc<ResourcesAudited>) -> Self {
        Event::ResourcesAudited(e)
    }
}

impl From<Rc<ResourcesTransferred>> for Event {
    fn from(e: Rc<ResourcesTransferred>) -> Self {
        Event::ResourcesTransferred(e)
    }
}

impl From<Rc<Retreat>> for Event {
    fn from(e: Rc<Retreat>) -> Self {
        Event::Retreat(e)
    }
}

impl From<Rc<Revival>> for Event {
    fn from(e: Rc<Revival>) -> Self {
        Event::Revival(e)
    }
}

impl From<Rc<RicheseAnnouncesAuction>> for Event {
    fn from(e: Rc<RicheseAnnouncesAuction>) -> Self {
        Event::RicheseAnnouncesAuction(e)
    }
}

impl From<Rc<RicheseAnnouncesBlackMarket>> for Event {
    fn from(e: Rc<RicheseAnnouncesBlackMarket>) -> Self {
        Event::RicheseAnnouncesBlackMarket(e)
    }
}

impl From<Rc<RicheseGaveCard>> for Event {
    fn from(e: Rc<RicheseGaveCard>) -> Self {
        Event::RicheseGaveCard(e)
    }
}

impl From<Rc<RicheseKeepsUnsoldCard>> for Event {
    fn from(e: Rc<RicheseKeepsUnsoldCard>) -> Self {
        Event::RicheseKeepsUnsoldCard(e)
    }
}

impl From<Rc<RicheseRevealedNoField>> for Event {
    fn from(e: Rc<RicheseRevealedNoField>) -> Self {
        Event::RicheseRevealedNoField(e)
    }
}

impl From<Rc<RicheseSpecifiesAuction>> for Event {
    fn from(e: Rc<RicheseSpecifiesAuction>) -> Self {
        Event::RicheseSpecifiesAuction(e)
    }
}

impl From<Rc<RockWasMelted>> for Event {
    fn from(e: Rc<RockWasMelted>) -> Self {
        Event::RockWasMelted(e)
    }
}

impl From<Rc<SetIncreasedRevivalLimits>> for Event {
    fn from(e: Rc<SetIncreasedRevivalLimits>) -> Self {
        Event::SetIncreasedRevivalLimits(e)
    }
}

impl From<Rc<SetShipmentPermission>> for Event {
    fn from(e: Rc<SetShipmentPermission>) -> Self {
        Event::SetShipmentPermission(e)
    }
}

impl From<Rc<Shipment>> for Event {
    fn from(e: Rc<Shipment>) -> Self {
        Event::Shipment(e)
    }
}

impl From<Rc<SkillAssigned>> for Event {
    fn from(e: Rc<SkillAssigned>) -> Self {
        Event::SkillAssigned(e)
    }
}

impl From<Rc<StormDialled>> for Event {
    fn from(e: Rc<StormDialled>) -> Self {
        Event::StormDialled(e)
    }
}

impl From<Rc<StormSpellPlayed>> for Event {
    fn from(e: Rc<StormSpellPlayed>) -> Self {
        Event::StormSpellPlayed(e)
    }
}

impl From<Rc<SwitchedSkilledLeader>> for Event {
    fn from(e: Rc<SwitchedSkilledLeader>) -> Self {
        Event::SwitchedSkilledLeader(e)
    }
}

impl From<Rc<TakeLosses>> for Event {
    fn from(e: Rc<TakeLosses>) -> Self {
        Event::TakeLosses(e)
    }
}

impl From<Rc<TerrorPlanted>> for Event {
    fn from(e: Rc<TerrorPlanted>) -> Self {
        Event::TerrorPlanted(e)
    }
}

impl From<Rc<TerrorRevealed>> for Event {
    fn from(e: Rc<TerrorRevealed>) -> Self {
        Event::TerrorRevealed(e)
    }
}

impl From<Rc<TestingStationUsed>> for Event {
    fn from(e: Rc<TestingStationUsed>) -> Self {
        Event::TestingStationUsed(e)
    }
}

impl From<Rc<Thought>> for Event {
    fn from(e: Rc<Thought>) -> Self {
        Event::Thought(e)
    }
}

impl From<Rc<ThoughtAnswered>> for Event {
    fn from(e: Rc<ThoughtAnswered>) -> Self {
        Event::ThoughtAnswered(e)
    }
}

impl From<Rc<ThumperPlayed>> for Event {
    fn from(e: Rc<ThumperPlayed>) -> Self {
        Event::ThumperPlayed(e)
    }
}

impl From<Rc<TraitorDiscarded>> for Event {
    fn from(e: Rc<TraitorDiscarded>) -> Self {
        Event::TraitorDiscarded(e)
    }
}

impl From<Rc<TraitorsSelected>> for Event {
    fn from(e: Rc<TraitorsSelected>) -> Self {
        Event::TraitorsSelected(e)
    }
}

impl From<Rc<TreacheryCalled>> for Event {
    fn from(e: Rc<TreacheryCalled>) -> Self {
        Event::TreacheryCalled(e)
    }
}

impl From<Rc<Voice>> for Event {
    fn from(e: Rc<Voice>) -> Self {
        Event::Voice(e)
    }
}

impl From<Rc<VoiceDeclined>> for Event {
    fn from(e: Rc<VoiceDeclined>) -> Self {
        Event::VoiceDeclined(e)
    }
}

/// The type of a concrete event (`GetType()`, `typeof(X)`).
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub enum EventType {
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

impl EventType {
    pub const ALL: &'static [EventType] = &[
        EventType::AcceptOrCancelTleilaxuRevival,
        EventType::AllianceBroken,
        EventType::AllianceByAmbassador,
        EventType::AllianceByTerror,
        EventType::AllianceOffered,
        EventType::AlliancePassed,
        EventType::AllyPermission,
        EventType::AmalPlayed,
        EventType::AmbassadorActivated,
        EventType::AmbassadorPlaced,
        EventType::AuditCancelled,
        EventType::Audited,
        EventType::AutomationConfigured,
        EventType::Battle,
        EventType::BattleClaimed,
        EventType::BattleConcluded,
        EventType::BattleInitiated,
        EventType::BattleRevision,
        EventType::BeneGesseritAccompanies,
        EventType::BeneGesseritBattleAnnouncement,
        EventType::BeneGesseritFlip,
        EventType::BeneGesseritPrediction,
        EventType::Bid,
        EventType::BlackMarketBid,
        EventType::Bureaucracy,
        EventType::Captured,
        EventType::Caravan,
        EventType::CardGiven,
        EventType::CardTraded,
        EventType::CardsDetermined,
        EventType::CharityClaimed,
        EventType::ChoamDiscarded,
        EventType::ChoamEconomics,
        EventType::ChoamExtraMove,
        EventType::ChoamFreeRevivalPrevention,
        EventType::ChoamKarmaPrevention,
        EventType::ChoamMovePrevention,
        EventType::ChoamRemoveForce,
        EventType::ClairVoyanceAnswered,
        EventType::ClairVoyancePlayed,
        EventType::DealAccepted,
        EventType::DealOffered,
        EventType::DealRejected,
        EventType::Diplomacy,
        EventType::Discarded,
        EventType::DiscardedSearched,
        EventType::DiscardedSearchedAnnounced,
        EventType::DiscardedTaken,
        EventType::DiscoveryEntered,
        EventType::DiscoveryRevealed,
        EventType::DistransUsed,
        EventType::DivideResources,
        EventType::DivideResourcesAccepted,
        EventType::Donated,
        EventType::EmperorBidSupport,
        EventType::EmperorDiscarded,
        EventType::EndPhase,
        EventType::EstablishPlayers,
        EventType::ExtortionPrevented,
        EventType::FaceDanced,
        EventType::FaceDancerReplaced,
        EventType::FaceDancerRevealed,
        EventType::FactionSelected,
        EventType::FactionTradeOffered,
        EventType::FlightDiscoveryUsed,
        EventType::FlightUsed,
        EventType::FremenRidesMonster,
        EventType::FremenSentMonster,
        EventType::GuildDelay,
        EventType::HMSAdvantageChosen,
        EventType::HarvesterPlayed,
        EventType::HideSecrets,
        EventType::IxianRemovedCardFromAuction,
        EventType::IxianSelectedStartingCard,
        EventType::IxianSwappedCardOnBid,
        EventType::JuicePlayed,
        EventType::Karma,
        EventType::KarmaChoamDiscard,
        EventType::KarmaEcazDial,
        EventType::KarmaFreeRevival,
        EventType::KarmaHandSwap,
        EventType::KarmaHandSwapInitiated,
        EventType::KarmaHmsMovement,
        EventType::KarmaMonster,
        EventType::KarmaPrescience,
        EventType::KarmaRevivalPrevention,
        EventType::KarmaRicheseBuy,
        EventType::KarmaShipmentPrevention,
        EventType::LoserConcluded,
        EventType::MetheorPlayed,
        EventType::Move,
        EventType::MulliganPerformed,
        EventType::NexusCardDrawn,
        EventType::NexusPlayed,
        EventType::NexusVoted,
        EventType::PerformBeneGesseritPlacement,
        EventType::PerformFremenSetup,
        EventType::PerformHmsMovement,
        EventType::PerformHmsPlacement,
        EventType::PerformMoritaniSetup,
        EventType::PerformSetup,
        EventType::Planetology,
        EventType::PlayerReplaced,
        EventType::PoisonToothCancelled,
        EventType::PortableAntidoteUsed,
        EventType::Prescience,
        EventType::RaiseDeadPlayed,
        EventType::RecruitsPlayed,
        EventType::ReplacedCardWon,
        EventType::RequestTleilaxuRevival,
        EventType::ResidualPlayed,
        EventType::ResourcesAudited,
        EventType::ResourcesTransferred,
        EventType::Retreat,
        EventType::Revival,
        EventType::RicheseAnnouncesAuction,
        EventType::RicheseAnnouncesBlackMarket,
        EventType::RicheseGaveCard,
        EventType::RicheseKeepsUnsoldCard,
        EventType::RicheseRevealedNoField,
        EventType::RicheseSpecifiesAuction,
        EventType::RockWasMelted,
        EventType::SetIncreasedRevivalLimits,
        EventType::SetShipmentPermission,
        EventType::Shipment,
        EventType::SkillAssigned,
        EventType::StormDialled,
        EventType::StormSpellPlayed,
        EventType::SwitchedSkilledLeader,
        EventType::TakeLosses,
        EventType::TerrorPlanted,
        EventType::TerrorRevealed,
        EventType::TestingStationUsed,
        EventType::Thought,
        EventType::ThoughtAnswered,
        EventType::ThumperPlayed,
        EventType::TraitorDiscarded,
        EventType::TraitorsSelected,
        EventType::TreacheryCalled,
        EventType::Voice,
        EventType::VoiceDeclined,
    ];

    pub fn name(self) -> &'static str {
        match self {
            EventType::AcceptOrCancelTleilaxuRevival => "AcceptOrCancelTleilaxuRevival",
            EventType::AllianceBroken => "AllianceBroken",
            EventType::AllianceByAmbassador => "AllianceByAmbassador",
            EventType::AllianceByTerror => "AllianceByTerror",
            EventType::AllianceOffered => "AllianceOffered",
            EventType::AlliancePassed => "AlliancePassed",
            EventType::AllyPermission => "AllyPermission",
            EventType::AmalPlayed => "AmalPlayed",
            EventType::AmbassadorActivated => "AmbassadorActivated",
            EventType::AmbassadorPlaced => "AmbassadorPlaced",
            EventType::AuditCancelled => "AuditCancelled",
            EventType::Audited => "Audited",
            EventType::AutomationConfigured => "AutomationConfigured",
            EventType::Battle => "Battle",
            EventType::BattleClaimed => "BattleClaimed",
            EventType::BattleConcluded => "BattleConcluded",
            EventType::BattleInitiated => "BattleInitiated",
            EventType::BattleRevision => "BattleRevision",
            EventType::BeneGesseritAccompanies => "BeneGesseritAccompanies",
            EventType::BeneGesseritBattleAnnouncement => "BeneGesseritBattleAnnouncement",
            EventType::BeneGesseritFlip => "BeneGesseritFlip",
            EventType::BeneGesseritPrediction => "BeneGesseritPrediction",
            EventType::Bid => "Bid",
            EventType::BlackMarketBid => "BlackMarketBid",
            EventType::Bureaucracy => "Bureaucracy",
            EventType::Captured => "Captured",
            EventType::Caravan => "Caravan",
            EventType::CardGiven => "CardGiven",
            EventType::CardTraded => "CardTraded",
            EventType::CardsDetermined => "CardsDetermined",
            EventType::CharityClaimed => "CharityClaimed",
            EventType::ChoamDiscarded => "ChoamDiscarded",
            EventType::ChoamEconomics => "ChoamEconomics",
            EventType::ChoamExtraMove => "ChoamExtraMove",
            EventType::ChoamFreeRevivalPrevention => "ChoamFreeRevivalPrevention",
            EventType::ChoamKarmaPrevention => "ChoamKarmaPrevention",
            EventType::ChoamMovePrevention => "ChoamMovePrevention",
            EventType::ChoamRemoveForce => "ChoamRemoveForce",
            EventType::ClairVoyanceAnswered => "ClairVoyanceAnswered",
            EventType::ClairVoyancePlayed => "ClairVoyancePlayed",
            EventType::DealAccepted => "DealAccepted",
            EventType::DealOffered => "DealOffered",
            EventType::DealRejected => "DealRejected",
            EventType::Diplomacy => "Diplomacy",
            EventType::Discarded => "Discarded",
            EventType::DiscardedSearched => "DiscardedSearched",
            EventType::DiscardedSearchedAnnounced => "DiscardedSearchedAnnounced",
            EventType::DiscardedTaken => "DiscardedTaken",
            EventType::DiscoveryEntered => "DiscoveryEntered",
            EventType::DiscoveryRevealed => "DiscoveryRevealed",
            EventType::DistransUsed => "DistransUsed",
            EventType::DivideResources => "DivideResources",
            EventType::DivideResourcesAccepted => "DivideResourcesAccepted",
            EventType::Donated => "Donated",
            EventType::EmperorBidSupport => "EmperorBidSupport",
            EventType::EmperorDiscarded => "EmperorDiscarded",
            EventType::EndPhase => "EndPhase",
            EventType::EstablishPlayers => "EstablishPlayers",
            EventType::ExtortionPrevented => "ExtortionPrevented",
            EventType::FaceDanced => "FaceDanced",
            EventType::FaceDancerReplaced => "FaceDancerReplaced",
            EventType::FaceDancerRevealed => "FaceDancerRevealed",
            EventType::FactionSelected => "FactionSelected",
            EventType::FactionTradeOffered => "FactionTradeOffered",
            EventType::FlightDiscoveryUsed => "FlightDiscoveryUsed",
            EventType::FlightUsed => "FlightUsed",
            EventType::FremenRidesMonster => "FremenRidesMonster",
            EventType::FremenSentMonster => "FremenSentMonster",
            EventType::GuildDelay => "GuildDelay",
            EventType::HMSAdvantageChosen => "HMSAdvantageChosen",
            EventType::HarvesterPlayed => "HarvesterPlayed",
            EventType::HideSecrets => "HideSecrets",
            EventType::IxianRemovedCardFromAuction => "IxianRemovedCardFromAuction",
            EventType::IxianSelectedStartingCard => "IxianSelectedStartingCard",
            EventType::IxianSwappedCardOnBid => "IxianSwappedCardOnBid",
            EventType::JuicePlayed => "JuicePlayed",
            EventType::Karma => "Karma",
            EventType::KarmaChoamDiscard => "KarmaChoamDiscard",
            EventType::KarmaEcazDial => "KarmaEcazDial",
            EventType::KarmaFreeRevival => "KarmaFreeRevival",
            EventType::KarmaHandSwap => "KarmaHandSwap",
            EventType::KarmaHandSwapInitiated => "KarmaHandSwapInitiated",
            EventType::KarmaHmsMovement => "KarmaHmsMovement",
            EventType::KarmaMonster => "KarmaMonster",
            EventType::KarmaPrescience => "KarmaPrescience",
            EventType::KarmaRevivalPrevention => "KarmaRevivalPrevention",
            EventType::KarmaRicheseBuy => "KarmaRicheseBuy",
            EventType::KarmaShipmentPrevention => "KarmaShipmentPrevention",
            EventType::LoserConcluded => "LoserConcluded",
            EventType::MetheorPlayed => "MetheorPlayed",
            EventType::Move => "Move",
            EventType::MulliganPerformed => "MulliganPerformed",
            EventType::NexusCardDrawn => "NexusCardDrawn",
            EventType::NexusPlayed => "NexusPlayed",
            EventType::NexusVoted => "NexusVoted",
            EventType::PerformBeneGesseritPlacement => "PerformBeneGesseritPlacement",
            EventType::PerformFremenSetup => "PerformFremenSetup",
            EventType::PerformHmsMovement => "PerformHmsMovement",
            EventType::PerformHmsPlacement => "PerformHmsPlacement",
            EventType::PerformMoritaniSetup => "PerformMoritaniSetup",
            EventType::PerformSetup => "PerformSetup",
            EventType::Planetology => "Planetology",
            EventType::PlayerReplaced => "PlayerReplaced",
            EventType::PoisonToothCancelled => "PoisonToothCancelled",
            EventType::PortableAntidoteUsed => "PortableAntidoteUsed",
            EventType::Prescience => "Prescience",
            EventType::RaiseDeadPlayed => "RaiseDeadPlayed",
            EventType::RecruitsPlayed => "RecruitsPlayed",
            EventType::ReplacedCardWon => "ReplacedCardWon",
            EventType::RequestTleilaxuRevival => "RequestTleilaxuRevival",
            EventType::ResidualPlayed => "ResidualPlayed",
            EventType::ResourcesAudited => "ResourcesAudited",
            EventType::ResourcesTransferred => "ResourcesTransferred",
            EventType::Retreat => "Retreat",
            EventType::Revival => "Revival",
            EventType::RicheseAnnouncesAuction => "RicheseAnnouncesAuction",
            EventType::RicheseAnnouncesBlackMarket => "RicheseAnnouncesBlackMarket",
            EventType::RicheseGaveCard => "RicheseGaveCard",
            EventType::RicheseKeepsUnsoldCard => "RicheseKeepsUnsoldCard",
            EventType::RicheseRevealedNoField => "RicheseRevealedNoField",
            EventType::RicheseSpecifiesAuction => "RicheseSpecifiesAuction",
            EventType::RockWasMelted => "RockWasMelted",
            EventType::SetIncreasedRevivalLimits => "SetIncreasedRevivalLimits",
            EventType::SetShipmentPermission => "SetShipmentPermission",
            EventType::Shipment => "Shipment",
            EventType::SkillAssigned => "SkillAssigned",
            EventType::StormDialled => "StormDialled",
            EventType::StormSpellPlayed => "StormSpellPlayed",
            EventType::SwitchedSkilledLeader => "SwitchedSkilledLeader",
            EventType::TakeLosses => "TakeLosses",
            EventType::TerrorPlanted => "TerrorPlanted",
            EventType::TerrorRevealed => "TerrorRevealed",
            EventType::TestingStationUsed => "TestingStationUsed",
            EventType::Thought => "Thought",
            EventType::ThoughtAnswered => "ThoughtAnswered",
            EventType::ThumperPlayed => "ThumperPlayed",
            EventType::TraitorDiscarded => "TraitorDiscarded",
            EventType::TraitorsSelected => "TraitorsSelected",
            EventType::TreacheryCalled => "TreacheryCalled",
            EventType::Voice => "Voice",
            EventType::VoiceDeclined => "VoiceDeclined",
        }
    }

    /// The `$type` discriminator from `[JsonDerivedType]` on `Event`; `None` for types without one.
    pub fn json_discriminator(self) -> Option<&'static str> {
        match self {
            EventType::AcceptOrCancelTleilaxuRevival => Some("AcceptOrCancelTleilaxuRevival"),
            EventType::AllianceBroken => Some("AllianceBroken"),
            EventType::AllianceByAmbassador => Some("AllianceByAmbassador"),
            EventType::AllianceByTerror => Some("AllianceByTerror"),
            EventType::AllianceOffered => Some("AllianceOffered"),
            EventType::AlliancePassed => Some("AlliancePassed"),
            EventType::AllyPermission => Some("AllyPermission"),
            EventType::AmalPlayed => Some("AmalPlayed"),
            EventType::AmbassadorActivated => Some("AmbassadorActivated"),
            EventType::AmbassadorPlaced => Some("AmbassadorPlaced"),
            EventType::AuditCancelled => Some("AuditCancelled"),
            EventType::Audited => Some("Audited"),
            EventType::AutomationConfigured => Some("AutomationConfigured"),
            EventType::Battle => Some("Battle"),
            EventType::BattleClaimed => Some("BattleClaimed"),
            EventType::BattleConcluded => Some("BattleConcluded"),
            EventType::BattleInitiated => Some("BattleInitiated"),
            EventType::BattleRevision => Some("BattleRevision"),
            EventType::BeneGesseritAccompanies => Some("BeneGesseritAccompanies"),
            EventType::BeneGesseritBattleAnnouncement => Some("BeneGesseritBattleAnnouncement"),
            EventType::BeneGesseritFlip => Some("BeneGesseritFlip"),
            EventType::BeneGesseritPrediction => Some("BeneGesseritPrediction"),
            EventType::Bid => Some("Bid"),
            EventType::BlackMarketBid => Some("BlackMarketBid"),
            EventType::Bureaucracy => Some("Bureaucracy"),
            EventType::Captured => Some("Captured"),
            EventType::Caravan => Some("Caravan"),
            EventType::CardGiven => Some("CardGiven"),
            EventType::CardTraded => Some("CardTraded"),
            EventType::CardsDetermined => Some("CardsDetermined"),
            EventType::CharityClaimed => Some("CharityClaimed"),
            EventType::ChoamDiscarded => Some("ChoamDiscarded"),
            EventType::ChoamEconomics => Some("ChoamEconomics"),
            EventType::ChoamExtraMove => Some("ChoamExtraMove"),
            EventType::ChoamFreeRevivalPrevention => Some("ChoamFreeRevivalPrevention"),
            EventType::ChoamKarmaPrevention => Some("ChoamKarmaPrevention"),
            EventType::ChoamMovePrevention => Some("ChoamMovePrevention"),
            EventType::ChoamRemoveForce => Some("ChoamRemoveForce"),
            EventType::ClairVoyanceAnswered => Some("ClairVoyanceAnswered"),
            EventType::ClairVoyancePlayed => Some("ClairVoyancePlayed"),
            EventType::DealAccepted => Some("DealAccepted"),
            EventType::DealOffered => Some("DealOffered"),
            EventType::DealRejected => Some("DealRejected"),
            EventType::Diplomacy => Some("Diplomacy"),
            EventType::Discarded => Some("Discarded"),
            EventType::DiscardedSearched => Some("DiscardedSearched"),
            EventType::DiscardedSearchedAnnounced => Some("DiscardedSearchedAnnounced"),
            EventType::DiscardedTaken => Some("DiscardedTaken"),
            EventType::DiscoveryEntered => Some("DiscoveryEntered"),
            EventType::DiscoveryRevealed => Some("DiscoveryRevealed"),
            EventType::DistransUsed => Some("DistransUsed"),
            EventType::DivideResources => Some("DivideResources"),
            EventType::DivideResourcesAccepted => Some("DivideResourcesAccepted"),
            EventType::Donated => Some("Donated"),
            EventType::EmperorBidSupport => Some("EmperorBidSupport"),
            EventType::EmperorDiscarded => Some("EmperorDiscarded"),
            EventType::EndPhase => Some("EndPhase"),
            EventType::EstablishPlayers => Some("EstablishPlayers"),
            EventType::ExtortionPrevented => Some("ExtortionPrevented"),
            EventType::FaceDanced => Some("FaceDanced"),
            EventType::FaceDancerReplaced => Some("FaceDancerReplaced"),
            EventType::FaceDancerRevealed => Some("FaceDancerRevealed"),
            EventType::FactionSelected => Some("FactionSelected"),
            EventType::FactionTradeOffered => Some("FactionTradeOffered"),
            EventType::FlightDiscoveryUsed => Some("FlightDiscoveryUsed"),
            EventType::FlightUsed => Some("FlightUsed"),
            EventType::FremenRidesMonster => Some("FremenRidesMonster"),
            EventType::FremenSentMonster => Some("FremenSentMonster"),
            EventType::GuildDelay => Some("GuildDelay"),
            EventType::HMSAdvantageChosen => Some("HMSAdvantageChosen"),
            EventType::HarvesterPlayed => Some("HarvesterPlayed"),
            EventType::HideSecrets => Some("HideSecrets"),
            EventType::IxianRemovedCardFromAuction => Some("IxianRemovedCardFromAuction"),
            EventType::IxianSelectedStartingCard => Some("IxianSelectedStartingCard"),
            EventType::IxianSwappedCardOnBid => Some("IxianSwappedCardOnBid"),
            EventType::JuicePlayed => Some("JuicePlayed"),
            EventType::Karma => Some("Karma"),
            EventType::KarmaChoamDiscard => Some("KarmaChoamDiscard"),
            EventType::KarmaEcazDial => Some("KarmaEcazDial"),
            EventType::KarmaFreeRevival => Some("KarmaFreeRevival"),
            EventType::KarmaHandSwap => Some("KarmaHandSwap"),
            EventType::KarmaHandSwapInitiated => Some("KarmaHandSwapInitiated"),
            EventType::KarmaHmsMovement => Some("KarmaHmsMovement"),
            EventType::KarmaMonster => Some("KarmaMonster"),
            EventType::KarmaPrescience => Some("KarmaPrescience"),
            EventType::KarmaRevivalPrevention => Some("KarmaRevivalPrevention"),
            EventType::KarmaRicheseBuy => Some("KarmaRicheseBuy"),
            EventType::KarmaShipmentPrevention => Some("KarmaShipmentPrevention"),
            EventType::LoserConcluded => Some("LoserConcluded"),
            EventType::MetheorPlayed => Some("MetheorPlayed"),
            EventType::Move => Some("Move"),
            EventType::MulliganPerformed => Some("MulliganPerformed"),
            EventType::NexusCardDrawn => Some("NexusCardDrawn"),
            EventType::NexusPlayed => Some("NexusPlayed"),
            EventType::NexusVoted => Some("NexusVoted"),
            EventType::PerformBeneGesseritPlacement => Some("PerformBeneGesseritPlacement"),
            EventType::PerformFremenSetup => Some("PerformFremenSetup"),
            EventType::PerformHmsMovement => Some("PerformHmsMovement"),
            EventType::PerformHmsPlacement => Some("PerformHmsPlacement"),
            EventType::PerformMoritaniSetup => Some("PerformMoritaniSetup"),
            EventType::PerformSetup => Some("PerformSetup"),
            EventType::Planetology => Some("Planetology"),
            EventType::PlayerReplaced => Some("PlayerReplaced"),
            EventType::PoisonToothCancelled => Some("PoisonToothCancelled"),
            EventType::PortableAntidoteUsed => Some("PortableAntidoteUsed"),
            EventType::Prescience => Some("Prescience"),
            EventType::RaiseDeadPlayed => Some("RaiseDeadPlayed"),
            EventType::RecruitsPlayed => Some("RecruitsPlayed"),
            EventType::ReplacedCardWon => Some("ReplacedCardWon"),
            EventType::RequestTleilaxuRevival => Some("RequestTleilaxuRevival"),
            EventType::ResidualPlayed => Some("ResidualPlayed"),
            EventType::ResourcesAudited => Some("ResourcesAudited"),
            EventType::ResourcesTransferred => Some("ResourcesTransferred"),
            EventType::Retreat => Some("Retreat"),
            EventType::Revival => Some("Revival"),
            EventType::RicheseAnnouncesAuction => Some("RicheseAnnouncesAuction"),
            EventType::RicheseAnnouncesBlackMarket => Some("RicheseAnnouncesBlackMarket"),
            EventType::RicheseGaveCard => Some("RicheseGaveCard"),
            EventType::RicheseKeepsUnsoldCard => Some("RicheseKeepsUnsoldCard"),
            EventType::RicheseRevealedNoField => Some("RicheseRevealedNoField"),
            EventType::RicheseSpecifiesAuction => Some("RicheseSpecifiesAuction"),
            EventType::RockWasMelted => Some("RockWasMelted"),
            EventType::SetIncreasedRevivalLimits => Some("SetIncreasedRevivalLimits"),
            EventType::SetShipmentPermission => Some("SetShipmentPermission"),
            EventType::Shipment => Some("Shipment"),
            EventType::SkillAssigned => Some("SkillAssigned"),
            EventType::StormDialled => Some("StormDialled"),
            EventType::StormSpellPlayed => Some("StormSpellPlayed"),
            EventType::SwitchedSkilledLeader => Some("SwitchedSkilledLeader"),
            EventType::TakeLosses => Some("TakeLosses"),
            EventType::TerrorPlanted => Some("TerrorPlanted"),
            EventType::TerrorRevealed => Some("TerrorRevealed"),
            EventType::TestingStationUsed => Some("TestingStationUsed"),
            EventType::Thought => Some("Thought"),
            EventType::ThoughtAnswered => Some("ThoughtAnswered"),
            EventType::ThumperPlayed => Some("ThumperPlayed"),
            EventType::TraitorDiscarded => Some("TraitorDiscarded"),
            EventType::TraitorsSelected => Some("TraitorsSelected"),
            EventType::TreacheryCalled => Some("TreacheryCalled"),
            EventType::Voice => Some("Voice"),
            EventType::VoiceDeclined => Some("VoiceDeclined"),
        }
    }

    /// Inverse of [`Self::json_discriminator`] (ordinal, case-sensitive).
    pub fn from_json_discriminator(s: &str) -> Option<EventType> {
        match s {
            "AcceptOrCancelTleilaxuRevival" => Some(EventType::AcceptOrCancelTleilaxuRevival),
            "AllianceBroken" => Some(EventType::AllianceBroken),
            "AllianceByAmbassador" => Some(EventType::AllianceByAmbassador),
            "AllianceByTerror" => Some(EventType::AllianceByTerror),
            "AllianceOffered" => Some(EventType::AllianceOffered),
            "AlliancePassed" => Some(EventType::AlliancePassed),
            "AllyPermission" => Some(EventType::AllyPermission),
            "AmalPlayed" => Some(EventType::AmalPlayed),
            "AmbassadorActivated" => Some(EventType::AmbassadorActivated),
            "AmbassadorPlaced" => Some(EventType::AmbassadorPlaced),
            "AuditCancelled" => Some(EventType::AuditCancelled),
            "Audited" => Some(EventType::Audited),
            "AutomationConfigured" => Some(EventType::AutomationConfigured),
            "Battle" => Some(EventType::Battle),
            "BattleClaimed" => Some(EventType::BattleClaimed),
            "BattleConcluded" => Some(EventType::BattleConcluded),
            "BattleInitiated" => Some(EventType::BattleInitiated),
            "BattleRevision" => Some(EventType::BattleRevision),
            "BeneGesseritAccompanies" => Some(EventType::BeneGesseritAccompanies),
            "BeneGesseritBattleAnnouncement" => Some(EventType::BeneGesseritBattleAnnouncement),
            "BeneGesseritFlip" => Some(EventType::BeneGesseritFlip),
            "BeneGesseritPrediction" => Some(EventType::BeneGesseritPrediction),
            "Bid" => Some(EventType::Bid),
            "BlackMarketBid" => Some(EventType::BlackMarketBid),
            "Bureaucracy" => Some(EventType::Bureaucracy),
            "Captured" => Some(EventType::Captured),
            "Caravan" => Some(EventType::Caravan),
            "CardGiven" => Some(EventType::CardGiven),
            "CardTraded" => Some(EventType::CardTraded),
            "CardsDetermined" => Some(EventType::CardsDetermined),
            "CharityClaimed" => Some(EventType::CharityClaimed),
            "ChoamDiscarded" => Some(EventType::ChoamDiscarded),
            "ChoamEconomics" => Some(EventType::ChoamEconomics),
            "ChoamExtraMove" => Some(EventType::ChoamExtraMove),
            "ChoamFreeRevivalPrevention" => Some(EventType::ChoamFreeRevivalPrevention),
            "ChoamKarmaPrevention" => Some(EventType::ChoamKarmaPrevention),
            "ChoamMovePrevention" => Some(EventType::ChoamMovePrevention),
            "ChoamRemoveForce" => Some(EventType::ChoamRemoveForce),
            "ClairVoyanceAnswered" => Some(EventType::ClairVoyanceAnswered),
            "ClairVoyancePlayed" => Some(EventType::ClairVoyancePlayed),
            "DealAccepted" => Some(EventType::DealAccepted),
            "DealOffered" => Some(EventType::DealOffered),
            "DealRejected" => Some(EventType::DealRejected),
            "Diplomacy" => Some(EventType::Diplomacy),
            "Discarded" => Some(EventType::Discarded),
            "DiscardedSearched" => Some(EventType::DiscardedSearched),
            "DiscardedSearchedAnnounced" => Some(EventType::DiscardedSearchedAnnounced),
            "DiscardedTaken" => Some(EventType::DiscardedTaken),
            "DiscoveryEntered" => Some(EventType::DiscoveryEntered),
            "DiscoveryRevealed" => Some(EventType::DiscoveryRevealed),
            "DistransUsed" => Some(EventType::DistransUsed),
            "DivideResources" => Some(EventType::DivideResources),
            "DivideResourcesAccepted" => Some(EventType::DivideResourcesAccepted),
            "Donated" => Some(EventType::Donated),
            "EmperorBidSupport" => Some(EventType::EmperorBidSupport),
            "EmperorDiscarded" => Some(EventType::EmperorDiscarded),
            "EndPhase" => Some(EventType::EndPhase),
            "EstablishPlayers" => Some(EventType::EstablishPlayers),
            "ExtortionPrevented" => Some(EventType::ExtortionPrevented),
            "FaceDanced" => Some(EventType::FaceDanced),
            "FaceDancerReplaced" => Some(EventType::FaceDancerReplaced),
            "FaceDancerRevealed" => Some(EventType::FaceDancerRevealed),
            "FactionSelected" => Some(EventType::FactionSelected),
            "FactionTradeOffered" => Some(EventType::FactionTradeOffered),
            "FlightDiscoveryUsed" => Some(EventType::FlightDiscoveryUsed),
            "FlightUsed" => Some(EventType::FlightUsed),
            "FremenRidesMonster" => Some(EventType::FremenRidesMonster),
            "FremenSentMonster" => Some(EventType::FremenSentMonster),
            "GuildDelay" => Some(EventType::GuildDelay),
            "HMSAdvantageChosen" => Some(EventType::HMSAdvantageChosen),
            "HarvesterPlayed" => Some(EventType::HarvesterPlayed),
            "HideSecrets" => Some(EventType::HideSecrets),
            "IxianRemovedCardFromAuction" => Some(EventType::IxianRemovedCardFromAuction),
            "IxianSelectedStartingCard" => Some(EventType::IxianSelectedStartingCard),
            "IxianSwappedCardOnBid" => Some(EventType::IxianSwappedCardOnBid),
            "JuicePlayed" => Some(EventType::JuicePlayed),
            "Karma" => Some(EventType::Karma),
            "KarmaChoamDiscard" => Some(EventType::KarmaChoamDiscard),
            "KarmaEcazDial" => Some(EventType::KarmaEcazDial),
            "KarmaFreeRevival" => Some(EventType::KarmaFreeRevival),
            "KarmaHandSwap" => Some(EventType::KarmaHandSwap),
            "KarmaHandSwapInitiated" => Some(EventType::KarmaHandSwapInitiated),
            "KarmaHmsMovement" => Some(EventType::KarmaHmsMovement),
            "KarmaMonster" => Some(EventType::KarmaMonster),
            "KarmaPrescience" => Some(EventType::KarmaPrescience),
            "KarmaRevivalPrevention" => Some(EventType::KarmaRevivalPrevention),
            "KarmaRicheseBuy" => Some(EventType::KarmaRicheseBuy),
            "KarmaShipmentPrevention" => Some(EventType::KarmaShipmentPrevention),
            "LoserConcluded" => Some(EventType::LoserConcluded),
            "MetheorPlayed" => Some(EventType::MetheorPlayed),
            "Move" => Some(EventType::Move),
            "MulliganPerformed" => Some(EventType::MulliganPerformed),
            "NexusCardDrawn" => Some(EventType::NexusCardDrawn),
            "NexusPlayed" => Some(EventType::NexusPlayed),
            "NexusVoted" => Some(EventType::NexusVoted),
            "PerformBeneGesseritPlacement" => Some(EventType::PerformBeneGesseritPlacement),
            "PerformFremenSetup" => Some(EventType::PerformFremenSetup),
            "PerformHmsMovement" => Some(EventType::PerformHmsMovement),
            "PerformHmsPlacement" => Some(EventType::PerformHmsPlacement),
            "PerformMoritaniSetup" => Some(EventType::PerformMoritaniSetup),
            "PerformSetup" => Some(EventType::PerformSetup),
            "Planetology" => Some(EventType::Planetology),
            "PlayerReplaced" => Some(EventType::PlayerReplaced),
            "PoisonToothCancelled" => Some(EventType::PoisonToothCancelled),
            "PortableAntidoteUsed" => Some(EventType::PortableAntidoteUsed),
            "Prescience" => Some(EventType::Prescience),
            "RaiseDeadPlayed" => Some(EventType::RaiseDeadPlayed),
            "RecruitsPlayed" => Some(EventType::RecruitsPlayed),
            "ReplacedCardWon" => Some(EventType::ReplacedCardWon),
            "RequestTleilaxuRevival" => Some(EventType::RequestTleilaxuRevival),
            "ResidualPlayed" => Some(EventType::ResidualPlayed),
            "ResourcesAudited" => Some(EventType::ResourcesAudited),
            "ResourcesTransferred" => Some(EventType::ResourcesTransferred),
            "Retreat" => Some(EventType::Retreat),
            "Revival" => Some(EventType::Revival),
            "RicheseAnnouncesAuction" => Some(EventType::RicheseAnnouncesAuction),
            "RicheseAnnouncesBlackMarket" => Some(EventType::RicheseAnnouncesBlackMarket),
            "RicheseGaveCard" => Some(EventType::RicheseGaveCard),
            "RicheseKeepsUnsoldCard" => Some(EventType::RicheseKeepsUnsoldCard),
            "RicheseRevealedNoField" => Some(EventType::RicheseRevealedNoField),
            "RicheseSpecifiesAuction" => Some(EventType::RicheseSpecifiesAuction),
            "RockWasMelted" => Some(EventType::RockWasMelted),
            "SetIncreasedRevivalLimits" => Some(EventType::SetIncreasedRevivalLimits),
            "SetShipmentPermission" => Some(EventType::SetShipmentPermission),
            "Shipment" => Some(EventType::Shipment),
            "SkillAssigned" => Some(EventType::SkillAssigned),
            "StormDialled" => Some(EventType::StormDialled),
            "StormSpellPlayed" => Some(EventType::StormSpellPlayed),
            "SwitchedSkilledLeader" => Some(EventType::SwitchedSkilledLeader),
            "TakeLosses" => Some(EventType::TakeLosses),
            "TerrorPlanted" => Some(EventType::TerrorPlanted),
            "TerrorRevealed" => Some(EventType::TerrorRevealed),
            "TestingStationUsed" => Some(EventType::TestingStationUsed),
            "Thought" => Some(EventType::Thought),
            "ThoughtAnswered" => Some(EventType::ThoughtAnswered),
            "ThumperPlayed" => Some(EventType::ThumperPlayed),
            "TraitorDiscarded" => Some(EventType::TraitorDiscarded),
            "TraitorsSelected" => Some(EventType::TraitorsSelected),
            "TreacheryCalled" => Some(EventType::TreacheryCalled),
            "Voice" => Some(EventType::Voice),
            "VoiceDeclined" => Some(EventType::VoiceDeclined),
            _ => None,
        }
    }
}

impl std::fmt::Display for EventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

impl PartialEq for Event {
    /// Identity: two events are equal only if they are the same object.
    fn eq(&self, other: &Self) -> bool {
        self.ptr() == other.ptr()
    }
}

impl Eq for Event {}

impl std::hash::Hash for Event {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.ptr().hash(state)
    }
}

impl std::fmt::Debug for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}@{:p}", self.event_type().name(), self.ptr())
    }
}

/// Virtual dispatch: each method calls the concrete event's implementation of the
/// `GameEventMethods` member of the same name.
impl Event {
    /// Address of the event object (reference identity).
    pub fn ptr(&self) -> *const () {
        dispatch!(self, e => Rc::as_ptr(e) as *const ())
    }

    pub fn event_type(&self) -> EventType {
        dispatch!(self, e => e.event_type())
    }

    pub fn base(&self) -> &GameEventBase {
        dispatch!(self, e => &e.base)
    }

    pub fn initiator(&self) -> Option<Faction> {
        dispatch!(self, e => GameEventMethods::initiator(&**e))
    }

    pub fn time(&self) -> DateTimeOffset {
        dispatch!(self, e => GameEventMethods::time(&**e))
    }

    pub fn player(&self) -> PlayerId {
        dispatch!(self, e => GameEventMethods::player(&**e))
    }

    pub fn player_opt(&self) -> Option<PlayerId> {
        dispatch!(self, e => GameEventMethods::player_opt(&**e))
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        dispatch!(self, e => GameEventMethods::validate(&**e, game))
    }

    pub fn is_valid(&self, game: &Game) -> bool {
        dispatch!(self, e => GameEventMethods::is_valid(&**e, game))
    }

    pub fn is_applicable(&self, game: &mut Game, is_host: bool) -> bool {
        dispatch!(self, e => GameEventMethods::is_applicable(&**e, game, is_host))
    }

    pub fn initialize(&self, game: &Game, initiator: Option<Faction>) {
        dispatch!(self, e => GameEventMethods::initialize(&**e, game, initiator))
    }

    pub fn initialize_game(&self, game: &Game) {
        dispatch!(self, e => GameEventMethods::initialize_game(&**e, game))
    }

    pub fn execute(&self, game: &mut Game, perform_validation: bool, is_host: bool) -> Option<Message> {
        dispatch!(self, e => GameEventMethods::execute(e, game, perform_validation, is_host))
    }

    pub fn execute_concrete_event(&self, game: &mut Game) -> GameResult {
        dispatch!(self, e => GameEventMethods::execute_concrete_event(e, game))
    }

    pub fn get_message(&self, game: &Game) -> Message {
        dispatch!(self, e => GameEventMethods::get_message(&**e, game))
    }

    pub fn get_short_message(&self, game: &Game) -> Message {
        dispatch!(self, e => GameEventMethods::get_short_message(&**e, game))
    }

    pub fn by(&self, f: impl Into<Option<Faction>>) -> bool {
        let f: Option<Faction> = f.into();
        dispatch!(self, e => GameEventMethods::by(&**e, f))
    }

    pub fn by_ally_of(&self, game: &Game, f: Faction) -> bool {
        dispatch!(self, e => GameEventMethods::by_ally_of(&**e, game, f))
    }

    pub fn by_player(&self, game: &Game, p: PlayerId) -> bool {
        dispatch!(self, e => GameEventMethods::by_player(&**e, game, p))
    }

    pub fn shallow_copy(&self) -> Option<Event> {
        dispatch!(self, e => GameEventMethods::shallow_copy(&**e))
    }

    pub fn log(&self, game: &mut Game) {
        dispatch!(self, e => GameEventMethods::log(&**e, game))
    }

    pub fn log_objects(&self, game: &mut Game, expression: Vec<Part>) {
        dispatch!(self, e => GameEventMethods::log_objects(&**e, game, expression))
    }

    pub fn log_if(&self, game: &mut Game, condition: bool, expression: Vec<Part>) {
        dispatch!(self, e => GameEventMethods::log_if(&**e, game, condition, expression))
    }

    pub fn log_to(&self, game: &mut Game, faction: Faction, expression: Vec<Part>) {
        dispatch!(self, e => GameEventMethods::log_to(&**e, game, faction, expression))
    }

    pub fn get_player(&self, game: &Game, f: Option<Faction>) -> Option<PlayerId> {
        dispatch!(self, e => GameEventMethods::get_player(&**e, game, f))
    }

    pub fn is_playing(&self, game: &Game, f: impl Into<Option<Faction>>) -> bool {
        let f: Option<Faction> = f.into();
        dispatch!(self, e => GameEventMethods::is_playing(&**e, game, f))
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

    /// `((LocationEvent)this).Initiator`; InvalidCastException for events that do not implement `LocationEvent`.
    pub fn location_event_initiator(&self) -> Faction {
        match self {
            Event::AmbassadorActivated(e) => <AmbassadorActivated as location_event::LocationEvent>::initiator(&**e),
            Event::BeneGesseritAccompanies(e) => <BeneGesseritAccompanies as location_event::LocationEvent>::initiator(&**e),
            Event::Caravan(e) => <Caravan as location_event::LocationEvent>::initiator(&**e),
            Event::DiscoveryEntered(e) => <DiscoveryEntered as location_event::LocationEvent>::initiator(&**e),
            Event::FaceDanced(e) => <FaceDanced as location_event::LocationEvent>::initiator(&**e),
            Event::FremenRidesMonster(e) => <FremenRidesMonster as location_event::LocationEvent>::initiator(&**e),
            Event::Move(e) => <Move as location_event::LocationEvent>::initiator(&**e),
            Event::NexusPlayed(e) => <NexusPlayed as location_event::LocationEvent>::initiator(&**e),
            Event::PerformFremenSetup(e) => <PerformFremenSetup as location_event::LocationEvent>::initiator(&**e),
            Event::PerformSetup(e) => <PerformSetup as location_event::LocationEvent>::initiator(&**e),
            Event::RaiseDeadPlayed(e) => <RaiseDeadPlayed as location_event::LocationEvent>::initiator(&**e),
            Event::Revival(e) => <Revival as location_event::LocationEvent>::initiator(&**e),
            Event::Shipment(e) => <Shipment as location_event::LocationEvent>::initiator(&**e),
            Event::TerrorRevealed(e) => <TerrorRevealed as location_event::LocationEvent>::initiator(&**e),
            _ => panic!("{} is not a LocationEvent", self.event_type().name()),
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

    /// `((Placement)this).Initiator`; InvalidCastException for events that do not implement `Placement`.
    pub fn placement_initiator(&self) -> Faction {
        match self {
            Event::AmbassadorActivated(e) => <AmbassadorActivated as placement::Placement>::initiator(&**e),
            Event::Caravan(e) => <Caravan as placement::Placement>::initiator(&**e),
            Event::DiscoveryEntered(e) => <DiscoveryEntered as placement::Placement>::initiator(&**e),
            Event::FaceDanced(e) => <FaceDanced as placement::Placement>::initiator(&**e),
            Event::FremenRidesMonster(e) => <FremenRidesMonster as placement::Placement>::initiator(&**e),
            Event::Move(e) => <Move as placement::Placement>::initiator(&**e),
            Event::PerformFremenSetup(e) => <PerformFremenSetup as placement::Placement>::initiator(&**e),
            Event::PerformSetup(e) => <PerformSetup as placement::Placement>::initiator(&**e),
            _ => panic!("{} is not a Placement", self.event_type().name()),
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
