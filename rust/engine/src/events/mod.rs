//! Every game event, grouped by the phase or topic it belongs to, and the polymorphic `GameEvent`.
//!
//! A [`GameEvent`] value is an enum with one variant per concrete event, each holding an `Rc` to
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
// the `GameEventMethods` ones, so `GameEvent` has a dispatching method for each member.
pub use bidding::bid_trait;
pub use movement::{location_event, placement};

use crate::prelude::*;

/// `match` over every variant, binding the `&Rc<Event>` to `$e`.
macro_rules! dispatch {
    ($self:expr, $e:ident => $body:expr) => {
        match $self {
            GameEvent::AcceptOrCancelTleilaxuRevival($e) => $body,
            GameEvent::AllianceBroken($e) => $body,
            GameEvent::AllianceByAmbassador($e) => $body,
            GameEvent::AllianceByTerror($e) => $body,
            GameEvent::AllianceOffered($e) => $body,
            GameEvent::AlliancePassed($e) => $body,
            GameEvent::AllyPermission($e) => $body,
            GameEvent::AmalPlayed($e) => $body,
            GameEvent::AmbassadorActivated($e) => $body,
            GameEvent::AmbassadorPlaced($e) => $body,
            GameEvent::AuditCancelled($e) => $body,
            GameEvent::Audited($e) => $body,
            GameEvent::AutomationConfigured($e) => $body,
            GameEvent::Battle($e) => $body,
            GameEvent::BattleClaimed($e) => $body,
            GameEvent::BattleConcluded($e) => $body,
            GameEvent::BattleInitiated($e) => $body,
            GameEvent::BattleRevision($e) => $body,
            GameEvent::BeneGesseritAccompanies($e) => $body,
            GameEvent::BeneGesseritBattleAnnouncement($e) => $body,
            GameEvent::BeneGesseritFlip($e) => $body,
            GameEvent::BeneGesseritPrediction($e) => $body,
            GameEvent::Bid($e) => $body,
            GameEvent::BlackMarketBid($e) => $body,
            GameEvent::Bureaucracy($e) => $body,
            GameEvent::Captured($e) => $body,
            GameEvent::Caravan($e) => $body,
            GameEvent::CardGiven($e) => $body,
            GameEvent::CardTraded($e) => $body,
            GameEvent::CardsDetermined($e) => $body,
            GameEvent::CharityClaimed($e) => $body,
            GameEvent::ChoamDiscarded($e) => $body,
            GameEvent::ChoamEconomics($e) => $body,
            GameEvent::ChoamExtraMove($e) => $body,
            GameEvent::ChoamFreeRevivalPrevention($e) => $body,
            GameEvent::ChoamKarmaPrevention($e) => $body,
            GameEvent::ChoamMovePrevention($e) => $body,
            GameEvent::ChoamRemoveForce($e) => $body,
            GameEvent::ClairVoyanceAnswered($e) => $body,
            GameEvent::ClairVoyancePlayed($e) => $body,
            GameEvent::DealAccepted($e) => $body,
            GameEvent::DealOffered($e) => $body,
            GameEvent::DealRejected($e) => $body,
            GameEvent::Diplomacy($e) => $body,
            GameEvent::Discarded($e) => $body,
            GameEvent::DiscardedSearched($e) => $body,
            GameEvent::DiscardedSearchedAnnounced($e) => $body,
            GameEvent::DiscardedTaken($e) => $body,
            GameEvent::DiscoveryEntered($e) => $body,
            GameEvent::DiscoveryRevealed($e) => $body,
            GameEvent::DistransUsed($e) => $body,
            GameEvent::DivideResources($e) => $body,
            GameEvent::DivideResourcesAccepted($e) => $body,
            GameEvent::Donated($e) => $body,
            GameEvent::EmperorBidSupport($e) => $body,
            GameEvent::EmperorDiscarded($e) => $body,
            GameEvent::EndPhase($e) => $body,
            GameEvent::EstablishPlayers($e) => $body,
            GameEvent::ExtortionPrevented($e) => $body,
            GameEvent::FaceDanced($e) => $body,
            GameEvent::FaceDancerReplaced($e) => $body,
            GameEvent::FaceDancerRevealed($e) => $body,
            GameEvent::FactionSelected($e) => $body,
            GameEvent::FactionTradeOffered($e) => $body,
            GameEvent::FlightDiscoveryUsed($e) => $body,
            GameEvent::FlightUsed($e) => $body,
            GameEvent::FremenRidesMonster($e) => $body,
            GameEvent::FremenSentMonster($e) => $body,
            GameEvent::GuildDelay($e) => $body,
            GameEvent::HMSAdvantageChosen($e) => $body,
            GameEvent::HarvesterPlayed($e) => $body,
            GameEvent::HideSecrets($e) => $body,
            GameEvent::IxianRemovedCardFromAuction($e) => $body,
            GameEvent::IxianSelectedStartingCard($e) => $body,
            GameEvent::IxianSwappedCardOnBid($e) => $body,
            GameEvent::JuicePlayed($e) => $body,
            GameEvent::Karma($e) => $body,
            GameEvent::KarmaChoamDiscard($e) => $body,
            GameEvent::KarmaEcazDial($e) => $body,
            GameEvent::KarmaFreeRevival($e) => $body,
            GameEvent::KarmaHandSwap($e) => $body,
            GameEvent::KarmaHandSwapInitiated($e) => $body,
            GameEvent::KarmaHmsMovement($e) => $body,
            GameEvent::KarmaMonster($e) => $body,
            GameEvent::KarmaPrescience($e) => $body,
            GameEvent::KarmaRevivalPrevention($e) => $body,
            GameEvent::KarmaRicheseBuy($e) => $body,
            GameEvent::KarmaShipmentPrevention($e) => $body,
            GameEvent::LoserConcluded($e) => $body,
            GameEvent::MetheorPlayed($e) => $body,
            GameEvent::Move($e) => $body,
            GameEvent::MulliganPerformed($e) => $body,
            GameEvent::NexusCardDrawn($e) => $body,
            GameEvent::NexusPlayed($e) => $body,
            GameEvent::NexusVoted($e) => $body,
            GameEvent::PerformBeneGesseritPlacement($e) => $body,
            GameEvent::PerformFremenSetup($e) => $body,
            GameEvent::PerformHmsMovement($e) => $body,
            GameEvent::PerformHmsPlacement($e) => $body,
            GameEvent::PerformMoritaniSetup($e) => $body,
            GameEvent::PerformSetup($e) => $body,
            GameEvent::Planetology($e) => $body,
            GameEvent::PlayerReplaced($e) => $body,
            GameEvent::PoisonToothCancelled($e) => $body,
            GameEvent::PortableAntidoteUsed($e) => $body,
            GameEvent::Prescience($e) => $body,
            GameEvent::RaiseDeadPlayed($e) => $body,
            GameEvent::RecruitsPlayed($e) => $body,
            GameEvent::ReplacedCardWon($e) => $body,
            GameEvent::RequestTleilaxuRevival($e) => $body,
            GameEvent::ResidualPlayed($e) => $body,
            GameEvent::ResourcesAudited($e) => $body,
            GameEvent::ResourcesTransferred($e) => $body,
            GameEvent::Retreat($e) => $body,
            GameEvent::Revival($e) => $body,
            GameEvent::RicheseAnnouncesAuction($e) => $body,
            GameEvent::RicheseAnnouncesBlackMarket($e) => $body,
            GameEvent::RicheseGaveCard($e) => $body,
            GameEvent::RicheseKeepsUnsoldCard($e) => $body,
            GameEvent::RicheseRevealedNoField($e) => $body,
            GameEvent::RicheseSpecifiesAuction($e) => $body,
            GameEvent::RockWasMelted($e) => $body,
            GameEvent::SetIncreasedRevivalLimits($e) => $body,
            GameEvent::SetShipmentPermission($e) => $body,
            GameEvent::Shipment($e) => $body,
            GameEvent::SkillAssigned($e) => $body,
            GameEvent::StormDialled($e) => $body,
            GameEvent::StormSpellPlayed($e) => $body,
            GameEvent::SwitchedSkilledLeader($e) => $body,
            GameEvent::TakeLosses($e) => $body,
            GameEvent::TerrorPlanted($e) => $body,
            GameEvent::TerrorRevealed($e) => $body,
            GameEvent::TestingStationUsed($e) => $body,
            GameEvent::Thought($e) => $body,
            GameEvent::ThoughtAnswered($e) => $body,
            GameEvent::ThumperPlayed($e) => $body,
            GameEvent::TraitorDiscarded($e) => $body,
            GameEvent::TraitorsSelected($e) => $body,
            GameEvent::TreacheryCalled($e) => $body,
            GameEvent::Voice($e) => $body,
        }
    };
}
pub(crate) use dispatch;

/// A reference to any concrete event.
#[derive(Clone)]
pub enum GameEvent {
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
}

impl From<Rc<AcceptOrCancelTleilaxuRevival>> for GameEvent {
    fn from(e: Rc<AcceptOrCancelTleilaxuRevival>) -> Self {
        GameEvent::AcceptOrCancelTleilaxuRevival(e)
    }
}

impl From<Rc<AllianceBroken>> for GameEvent {
    fn from(e: Rc<AllianceBroken>) -> Self {
        GameEvent::AllianceBroken(e)
    }
}

impl From<Rc<AllianceByAmbassador>> for GameEvent {
    fn from(e: Rc<AllianceByAmbassador>) -> Self {
        GameEvent::AllianceByAmbassador(e)
    }
}

impl From<Rc<AllianceByTerror>> for GameEvent {
    fn from(e: Rc<AllianceByTerror>) -> Self {
        GameEvent::AllianceByTerror(e)
    }
}

impl From<Rc<AllianceOffered>> for GameEvent {
    fn from(e: Rc<AllianceOffered>) -> Self {
        GameEvent::AllianceOffered(e)
    }
}

impl From<Rc<AlliancePassed>> for GameEvent {
    fn from(e: Rc<AlliancePassed>) -> Self {
        GameEvent::AlliancePassed(e)
    }
}

impl From<Rc<AllyPermission>> for GameEvent {
    fn from(e: Rc<AllyPermission>) -> Self {
        GameEvent::AllyPermission(e)
    }
}

impl From<Rc<AmalPlayed>> for GameEvent {
    fn from(e: Rc<AmalPlayed>) -> Self {
        GameEvent::AmalPlayed(e)
    }
}

impl From<Rc<AmbassadorActivated>> for GameEvent {
    fn from(e: Rc<AmbassadorActivated>) -> Self {
        GameEvent::AmbassadorActivated(e)
    }
}

impl From<Rc<AmbassadorPlaced>> for GameEvent {
    fn from(e: Rc<AmbassadorPlaced>) -> Self {
        GameEvent::AmbassadorPlaced(e)
    }
}

impl From<Rc<AuditCancelled>> for GameEvent {
    fn from(e: Rc<AuditCancelled>) -> Self {
        GameEvent::AuditCancelled(e)
    }
}

impl From<Rc<Audited>> for GameEvent {
    fn from(e: Rc<Audited>) -> Self {
        GameEvent::Audited(e)
    }
}

impl From<Rc<AutomationConfigured>> for GameEvent {
    fn from(e: Rc<AutomationConfigured>) -> Self {
        GameEvent::AutomationConfigured(e)
    }
}

impl From<Rc<Battle>> for GameEvent {
    fn from(e: Rc<Battle>) -> Self {
        GameEvent::Battle(e)
    }
}

impl From<Rc<BattleClaimed>> for GameEvent {
    fn from(e: Rc<BattleClaimed>) -> Self {
        GameEvent::BattleClaimed(e)
    }
}

impl From<Rc<BattleConcluded>> for GameEvent {
    fn from(e: Rc<BattleConcluded>) -> Self {
        GameEvent::BattleConcluded(e)
    }
}

impl From<Rc<BattleInitiated>> for GameEvent {
    fn from(e: Rc<BattleInitiated>) -> Self {
        GameEvent::BattleInitiated(e)
    }
}

impl From<Rc<BattleRevision>> for GameEvent {
    fn from(e: Rc<BattleRevision>) -> Self {
        GameEvent::BattleRevision(e)
    }
}

impl From<Rc<BeneGesseritAccompanies>> for GameEvent {
    fn from(e: Rc<BeneGesseritAccompanies>) -> Self {
        GameEvent::BeneGesseritAccompanies(e)
    }
}

impl From<Rc<BeneGesseritBattleAnnouncement>> for GameEvent {
    fn from(e: Rc<BeneGesseritBattleAnnouncement>) -> Self {
        GameEvent::BeneGesseritBattleAnnouncement(e)
    }
}

impl From<Rc<BeneGesseritFlip>> for GameEvent {
    fn from(e: Rc<BeneGesseritFlip>) -> Self {
        GameEvent::BeneGesseritFlip(e)
    }
}

impl From<Rc<BeneGesseritPrediction>> for GameEvent {
    fn from(e: Rc<BeneGesseritPrediction>) -> Self {
        GameEvent::BeneGesseritPrediction(e)
    }
}

impl From<Rc<Bid>> for GameEvent {
    fn from(e: Rc<Bid>) -> Self {
        GameEvent::Bid(e)
    }
}

impl From<Rc<BlackMarketBid>> for GameEvent {
    fn from(e: Rc<BlackMarketBid>) -> Self {
        GameEvent::BlackMarketBid(e)
    }
}

impl From<Rc<Bureaucracy>> for GameEvent {
    fn from(e: Rc<Bureaucracy>) -> Self {
        GameEvent::Bureaucracy(e)
    }
}

impl From<Rc<Captured>> for GameEvent {
    fn from(e: Rc<Captured>) -> Self {
        GameEvent::Captured(e)
    }
}

impl From<Rc<Caravan>> for GameEvent {
    fn from(e: Rc<Caravan>) -> Self {
        GameEvent::Caravan(e)
    }
}

impl From<Rc<CardGiven>> for GameEvent {
    fn from(e: Rc<CardGiven>) -> Self {
        GameEvent::CardGiven(e)
    }
}

impl From<Rc<CardTraded>> for GameEvent {
    fn from(e: Rc<CardTraded>) -> Self {
        GameEvent::CardTraded(e)
    }
}

impl From<Rc<CardsDetermined>> for GameEvent {
    fn from(e: Rc<CardsDetermined>) -> Self {
        GameEvent::CardsDetermined(e)
    }
}

impl From<Rc<CharityClaimed>> for GameEvent {
    fn from(e: Rc<CharityClaimed>) -> Self {
        GameEvent::CharityClaimed(e)
    }
}

impl From<Rc<ChoamDiscarded>> for GameEvent {
    fn from(e: Rc<ChoamDiscarded>) -> Self {
        GameEvent::ChoamDiscarded(e)
    }
}

impl From<Rc<ChoamEconomics>> for GameEvent {
    fn from(e: Rc<ChoamEconomics>) -> Self {
        GameEvent::ChoamEconomics(e)
    }
}

impl From<Rc<ChoamExtraMove>> for GameEvent {
    fn from(e: Rc<ChoamExtraMove>) -> Self {
        GameEvent::ChoamExtraMove(e)
    }
}

impl From<Rc<ChoamFreeRevivalPrevention>> for GameEvent {
    fn from(e: Rc<ChoamFreeRevivalPrevention>) -> Self {
        GameEvent::ChoamFreeRevivalPrevention(e)
    }
}

impl From<Rc<ChoamKarmaPrevention>> for GameEvent {
    fn from(e: Rc<ChoamKarmaPrevention>) -> Self {
        GameEvent::ChoamKarmaPrevention(e)
    }
}

impl From<Rc<ChoamMovePrevention>> for GameEvent {
    fn from(e: Rc<ChoamMovePrevention>) -> Self {
        GameEvent::ChoamMovePrevention(e)
    }
}

impl From<Rc<ChoamRemoveForce>> for GameEvent {
    fn from(e: Rc<ChoamRemoveForce>) -> Self {
        GameEvent::ChoamRemoveForce(e)
    }
}

impl From<Rc<ClairVoyanceAnswered>> for GameEvent {
    fn from(e: Rc<ClairVoyanceAnswered>) -> Self {
        GameEvent::ClairVoyanceAnswered(e)
    }
}

impl From<Rc<ClairVoyancePlayed>> for GameEvent {
    fn from(e: Rc<ClairVoyancePlayed>) -> Self {
        GameEvent::ClairVoyancePlayed(e)
    }
}

impl From<Rc<DealAccepted>> for GameEvent {
    fn from(e: Rc<DealAccepted>) -> Self {
        GameEvent::DealAccepted(e)
    }
}

impl From<Rc<DealOffered>> for GameEvent {
    fn from(e: Rc<DealOffered>) -> Self {
        GameEvent::DealOffered(e)
    }
}

impl From<Rc<DealRejected>> for GameEvent {
    fn from(e: Rc<DealRejected>) -> Self {
        GameEvent::DealRejected(e)
    }
}

impl From<Rc<Diplomacy>> for GameEvent {
    fn from(e: Rc<Diplomacy>) -> Self {
        GameEvent::Diplomacy(e)
    }
}

impl From<Rc<Discarded>> for GameEvent {
    fn from(e: Rc<Discarded>) -> Self {
        GameEvent::Discarded(e)
    }
}

impl From<Rc<DiscardedSearched>> for GameEvent {
    fn from(e: Rc<DiscardedSearched>) -> Self {
        GameEvent::DiscardedSearched(e)
    }
}

impl From<Rc<DiscardedSearchedAnnounced>> for GameEvent {
    fn from(e: Rc<DiscardedSearchedAnnounced>) -> Self {
        GameEvent::DiscardedSearchedAnnounced(e)
    }
}

impl From<Rc<DiscardedTaken>> for GameEvent {
    fn from(e: Rc<DiscardedTaken>) -> Self {
        GameEvent::DiscardedTaken(e)
    }
}

impl From<Rc<DiscoveryEntered>> for GameEvent {
    fn from(e: Rc<DiscoveryEntered>) -> Self {
        GameEvent::DiscoveryEntered(e)
    }
}

impl From<Rc<DiscoveryRevealed>> for GameEvent {
    fn from(e: Rc<DiscoveryRevealed>) -> Self {
        GameEvent::DiscoveryRevealed(e)
    }
}

impl From<Rc<DistransUsed>> for GameEvent {
    fn from(e: Rc<DistransUsed>) -> Self {
        GameEvent::DistransUsed(e)
    }
}

impl From<Rc<DivideResources>> for GameEvent {
    fn from(e: Rc<DivideResources>) -> Self {
        GameEvent::DivideResources(e)
    }
}

impl From<Rc<DivideResourcesAccepted>> for GameEvent {
    fn from(e: Rc<DivideResourcesAccepted>) -> Self {
        GameEvent::DivideResourcesAccepted(e)
    }
}

impl From<Rc<Donated>> for GameEvent {
    fn from(e: Rc<Donated>) -> Self {
        GameEvent::Donated(e)
    }
}

impl From<Rc<EmperorBidSupport>> for GameEvent {
    fn from(e: Rc<EmperorBidSupport>) -> Self {
        GameEvent::EmperorBidSupport(e)
    }
}

impl From<Rc<EmperorDiscarded>> for GameEvent {
    fn from(e: Rc<EmperorDiscarded>) -> Self {
        GameEvent::EmperorDiscarded(e)
    }
}

impl From<Rc<EndPhase>> for GameEvent {
    fn from(e: Rc<EndPhase>) -> Self {
        GameEvent::EndPhase(e)
    }
}

impl From<Rc<EstablishPlayers>> for GameEvent {
    fn from(e: Rc<EstablishPlayers>) -> Self {
        GameEvent::EstablishPlayers(e)
    }
}

impl From<Rc<ExtortionPrevented>> for GameEvent {
    fn from(e: Rc<ExtortionPrevented>) -> Self {
        GameEvent::ExtortionPrevented(e)
    }
}

impl From<Rc<FaceDanced>> for GameEvent {
    fn from(e: Rc<FaceDanced>) -> Self {
        GameEvent::FaceDanced(e)
    }
}

impl From<Rc<FaceDancerReplaced>> for GameEvent {
    fn from(e: Rc<FaceDancerReplaced>) -> Self {
        GameEvent::FaceDancerReplaced(e)
    }
}

impl From<Rc<FaceDancerRevealed>> for GameEvent {
    fn from(e: Rc<FaceDancerRevealed>) -> Self {
        GameEvent::FaceDancerRevealed(e)
    }
}

impl From<Rc<FactionSelected>> for GameEvent {
    fn from(e: Rc<FactionSelected>) -> Self {
        GameEvent::FactionSelected(e)
    }
}

impl From<Rc<FactionTradeOffered>> for GameEvent {
    fn from(e: Rc<FactionTradeOffered>) -> Self {
        GameEvent::FactionTradeOffered(e)
    }
}

impl From<Rc<FlightDiscoveryUsed>> for GameEvent {
    fn from(e: Rc<FlightDiscoveryUsed>) -> Self {
        GameEvent::FlightDiscoveryUsed(e)
    }
}

impl From<Rc<FlightUsed>> for GameEvent {
    fn from(e: Rc<FlightUsed>) -> Self {
        GameEvent::FlightUsed(e)
    }
}

impl From<Rc<FremenRidesMonster>> for GameEvent {
    fn from(e: Rc<FremenRidesMonster>) -> Self {
        GameEvent::FremenRidesMonster(e)
    }
}

impl From<Rc<FremenSentMonster>> for GameEvent {
    fn from(e: Rc<FremenSentMonster>) -> Self {
        GameEvent::FremenSentMonster(e)
    }
}

impl From<Rc<GuildDelay>> for GameEvent {
    fn from(e: Rc<GuildDelay>) -> Self {
        GameEvent::GuildDelay(e)
    }
}

impl From<Rc<HMSAdvantageChosen>> for GameEvent {
    fn from(e: Rc<HMSAdvantageChosen>) -> Self {
        GameEvent::HMSAdvantageChosen(e)
    }
}

impl From<Rc<HarvesterPlayed>> for GameEvent {
    fn from(e: Rc<HarvesterPlayed>) -> Self {
        GameEvent::HarvesterPlayed(e)
    }
}

impl From<Rc<HideSecrets>> for GameEvent {
    fn from(e: Rc<HideSecrets>) -> Self {
        GameEvent::HideSecrets(e)
    }
}

impl From<Rc<IxianRemovedCardFromAuction>> for GameEvent {
    fn from(e: Rc<IxianRemovedCardFromAuction>) -> Self {
        GameEvent::IxianRemovedCardFromAuction(e)
    }
}

impl From<Rc<IxianSelectedStartingCard>> for GameEvent {
    fn from(e: Rc<IxianSelectedStartingCard>) -> Self {
        GameEvent::IxianSelectedStartingCard(e)
    }
}

impl From<Rc<IxianSwappedCardOnBid>> for GameEvent {
    fn from(e: Rc<IxianSwappedCardOnBid>) -> Self {
        GameEvent::IxianSwappedCardOnBid(e)
    }
}

impl From<Rc<JuicePlayed>> for GameEvent {
    fn from(e: Rc<JuicePlayed>) -> Self {
        GameEvent::JuicePlayed(e)
    }
}

impl From<Rc<Karma>> for GameEvent {
    fn from(e: Rc<Karma>) -> Self {
        GameEvent::Karma(e)
    }
}

impl From<Rc<KarmaChoamDiscard>> for GameEvent {
    fn from(e: Rc<KarmaChoamDiscard>) -> Self {
        GameEvent::KarmaChoamDiscard(e)
    }
}

impl From<Rc<KarmaEcazDial>> for GameEvent {
    fn from(e: Rc<KarmaEcazDial>) -> Self {
        GameEvent::KarmaEcazDial(e)
    }
}

impl From<Rc<KarmaFreeRevival>> for GameEvent {
    fn from(e: Rc<KarmaFreeRevival>) -> Self {
        GameEvent::KarmaFreeRevival(e)
    }
}

impl From<Rc<KarmaHandSwap>> for GameEvent {
    fn from(e: Rc<KarmaHandSwap>) -> Self {
        GameEvent::KarmaHandSwap(e)
    }
}

impl From<Rc<KarmaHandSwapInitiated>> for GameEvent {
    fn from(e: Rc<KarmaHandSwapInitiated>) -> Self {
        GameEvent::KarmaHandSwapInitiated(e)
    }
}

impl From<Rc<KarmaHmsMovement>> for GameEvent {
    fn from(e: Rc<KarmaHmsMovement>) -> Self {
        GameEvent::KarmaHmsMovement(e)
    }
}

impl From<Rc<KarmaMonster>> for GameEvent {
    fn from(e: Rc<KarmaMonster>) -> Self {
        GameEvent::KarmaMonster(e)
    }
}

impl From<Rc<KarmaPrescience>> for GameEvent {
    fn from(e: Rc<KarmaPrescience>) -> Self {
        GameEvent::KarmaPrescience(e)
    }
}

impl From<Rc<KarmaRevivalPrevention>> for GameEvent {
    fn from(e: Rc<KarmaRevivalPrevention>) -> Self {
        GameEvent::KarmaRevivalPrevention(e)
    }
}

impl From<Rc<KarmaRicheseBuy>> for GameEvent {
    fn from(e: Rc<KarmaRicheseBuy>) -> Self {
        GameEvent::KarmaRicheseBuy(e)
    }
}

impl From<Rc<KarmaShipmentPrevention>> for GameEvent {
    fn from(e: Rc<KarmaShipmentPrevention>) -> Self {
        GameEvent::KarmaShipmentPrevention(e)
    }
}

impl From<Rc<LoserConcluded>> for GameEvent {
    fn from(e: Rc<LoserConcluded>) -> Self {
        GameEvent::LoserConcluded(e)
    }
}

impl From<Rc<MetheorPlayed>> for GameEvent {
    fn from(e: Rc<MetheorPlayed>) -> Self {
        GameEvent::MetheorPlayed(e)
    }
}

impl From<Rc<Move>> for GameEvent {
    fn from(e: Rc<Move>) -> Self {
        GameEvent::Move(e)
    }
}

impl From<Rc<MulliganPerformed>> for GameEvent {
    fn from(e: Rc<MulliganPerformed>) -> Self {
        GameEvent::MulliganPerformed(e)
    }
}

impl From<Rc<NexusCardDrawn>> for GameEvent {
    fn from(e: Rc<NexusCardDrawn>) -> Self {
        GameEvent::NexusCardDrawn(e)
    }
}

impl From<Rc<NexusPlayed>> for GameEvent {
    fn from(e: Rc<NexusPlayed>) -> Self {
        GameEvent::NexusPlayed(e)
    }
}

impl From<Rc<NexusVoted>> for GameEvent {
    fn from(e: Rc<NexusVoted>) -> Self {
        GameEvent::NexusVoted(e)
    }
}

impl From<Rc<PerformBeneGesseritPlacement>> for GameEvent {
    fn from(e: Rc<PerformBeneGesseritPlacement>) -> Self {
        GameEvent::PerformBeneGesseritPlacement(e)
    }
}

impl From<Rc<PerformFremenSetup>> for GameEvent {
    fn from(e: Rc<PerformFremenSetup>) -> Self {
        GameEvent::PerformFremenSetup(e)
    }
}

impl From<Rc<PerformHmsMovement>> for GameEvent {
    fn from(e: Rc<PerformHmsMovement>) -> Self {
        GameEvent::PerformHmsMovement(e)
    }
}

impl From<Rc<PerformHmsPlacement>> for GameEvent {
    fn from(e: Rc<PerformHmsPlacement>) -> Self {
        GameEvent::PerformHmsPlacement(e)
    }
}

impl From<Rc<PerformMoritaniSetup>> for GameEvent {
    fn from(e: Rc<PerformMoritaniSetup>) -> Self {
        GameEvent::PerformMoritaniSetup(e)
    }
}

impl From<Rc<PerformSetup>> for GameEvent {
    fn from(e: Rc<PerformSetup>) -> Self {
        GameEvent::PerformSetup(e)
    }
}

impl From<Rc<Planetology>> for GameEvent {
    fn from(e: Rc<Planetology>) -> Self {
        GameEvent::Planetology(e)
    }
}

impl From<Rc<PlayerReplaced>> for GameEvent {
    fn from(e: Rc<PlayerReplaced>) -> Self {
        GameEvent::PlayerReplaced(e)
    }
}

impl From<Rc<PoisonToothCancelled>> for GameEvent {
    fn from(e: Rc<PoisonToothCancelled>) -> Self {
        GameEvent::PoisonToothCancelled(e)
    }
}

impl From<Rc<PortableAntidoteUsed>> for GameEvent {
    fn from(e: Rc<PortableAntidoteUsed>) -> Self {
        GameEvent::PortableAntidoteUsed(e)
    }
}

impl From<Rc<Prescience>> for GameEvent {
    fn from(e: Rc<Prescience>) -> Self {
        GameEvent::Prescience(e)
    }
}

impl From<Rc<RaiseDeadPlayed>> for GameEvent {
    fn from(e: Rc<RaiseDeadPlayed>) -> Self {
        GameEvent::RaiseDeadPlayed(e)
    }
}

impl From<Rc<RecruitsPlayed>> for GameEvent {
    fn from(e: Rc<RecruitsPlayed>) -> Self {
        GameEvent::RecruitsPlayed(e)
    }
}

impl From<Rc<ReplacedCardWon>> for GameEvent {
    fn from(e: Rc<ReplacedCardWon>) -> Self {
        GameEvent::ReplacedCardWon(e)
    }
}

impl From<Rc<RequestTleilaxuRevival>> for GameEvent {
    fn from(e: Rc<RequestTleilaxuRevival>) -> Self {
        GameEvent::RequestTleilaxuRevival(e)
    }
}

impl From<Rc<ResidualPlayed>> for GameEvent {
    fn from(e: Rc<ResidualPlayed>) -> Self {
        GameEvent::ResidualPlayed(e)
    }
}

impl From<Rc<ResourcesAudited>> for GameEvent {
    fn from(e: Rc<ResourcesAudited>) -> Self {
        GameEvent::ResourcesAudited(e)
    }
}

impl From<Rc<ResourcesTransferred>> for GameEvent {
    fn from(e: Rc<ResourcesTransferred>) -> Self {
        GameEvent::ResourcesTransferred(e)
    }
}

impl From<Rc<Retreat>> for GameEvent {
    fn from(e: Rc<Retreat>) -> Self {
        GameEvent::Retreat(e)
    }
}

impl From<Rc<Revival>> for GameEvent {
    fn from(e: Rc<Revival>) -> Self {
        GameEvent::Revival(e)
    }
}

impl From<Rc<RicheseAnnouncesAuction>> for GameEvent {
    fn from(e: Rc<RicheseAnnouncesAuction>) -> Self {
        GameEvent::RicheseAnnouncesAuction(e)
    }
}

impl From<Rc<RicheseAnnouncesBlackMarket>> for GameEvent {
    fn from(e: Rc<RicheseAnnouncesBlackMarket>) -> Self {
        GameEvent::RicheseAnnouncesBlackMarket(e)
    }
}

impl From<Rc<RicheseGaveCard>> for GameEvent {
    fn from(e: Rc<RicheseGaveCard>) -> Self {
        GameEvent::RicheseGaveCard(e)
    }
}

impl From<Rc<RicheseKeepsUnsoldCard>> for GameEvent {
    fn from(e: Rc<RicheseKeepsUnsoldCard>) -> Self {
        GameEvent::RicheseKeepsUnsoldCard(e)
    }
}

impl From<Rc<RicheseRevealedNoField>> for GameEvent {
    fn from(e: Rc<RicheseRevealedNoField>) -> Self {
        GameEvent::RicheseRevealedNoField(e)
    }
}

impl From<Rc<RicheseSpecifiesAuction>> for GameEvent {
    fn from(e: Rc<RicheseSpecifiesAuction>) -> Self {
        GameEvent::RicheseSpecifiesAuction(e)
    }
}

impl From<Rc<RockWasMelted>> for GameEvent {
    fn from(e: Rc<RockWasMelted>) -> Self {
        GameEvent::RockWasMelted(e)
    }
}

impl From<Rc<SetIncreasedRevivalLimits>> for GameEvent {
    fn from(e: Rc<SetIncreasedRevivalLimits>) -> Self {
        GameEvent::SetIncreasedRevivalLimits(e)
    }
}

impl From<Rc<SetShipmentPermission>> for GameEvent {
    fn from(e: Rc<SetShipmentPermission>) -> Self {
        GameEvent::SetShipmentPermission(e)
    }
}

impl From<Rc<Shipment>> for GameEvent {
    fn from(e: Rc<Shipment>) -> Self {
        GameEvent::Shipment(e)
    }
}

impl From<Rc<SkillAssigned>> for GameEvent {
    fn from(e: Rc<SkillAssigned>) -> Self {
        GameEvent::SkillAssigned(e)
    }
}

impl From<Rc<StormDialled>> for GameEvent {
    fn from(e: Rc<StormDialled>) -> Self {
        GameEvent::StormDialled(e)
    }
}

impl From<Rc<StormSpellPlayed>> for GameEvent {
    fn from(e: Rc<StormSpellPlayed>) -> Self {
        GameEvent::StormSpellPlayed(e)
    }
}

impl From<Rc<SwitchedSkilledLeader>> for GameEvent {
    fn from(e: Rc<SwitchedSkilledLeader>) -> Self {
        GameEvent::SwitchedSkilledLeader(e)
    }
}

impl From<Rc<TakeLosses>> for GameEvent {
    fn from(e: Rc<TakeLosses>) -> Self {
        GameEvent::TakeLosses(e)
    }
}

impl From<Rc<TerrorPlanted>> for GameEvent {
    fn from(e: Rc<TerrorPlanted>) -> Self {
        GameEvent::TerrorPlanted(e)
    }
}

impl From<Rc<TerrorRevealed>> for GameEvent {
    fn from(e: Rc<TerrorRevealed>) -> Self {
        GameEvent::TerrorRevealed(e)
    }
}

impl From<Rc<TestingStationUsed>> for GameEvent {
    fn from(e: Rc<TestingStationUsed>) -> Self {
        GameEvent::TestingStationUsed(e)
    }
}

impl From<Rc<Thought>> for GameEvent {
    fn from(e: Rc<Thought>) -> Self {
        GameEvent::Thought(e)
    }
}

impl From<Rc<ThoughtAnswered>> for GameEvent {
    fn from(e: Rc<ThoughtAnswered>) -> Self {
        GameEvent::ThoughtAnswered(e)
    }
}

impl From<Rc<ThumperPlayed>> for GameEvent {
    fn from(e: Rc<ThumperPlayed>) -> Self {
        GameEvent::ThumperPlayed(e)
    }
}

impl From<Rc<TraitorDiscarded>> for GameEvent {
    fn from(e: Rc<TraitorDiscarded>) -> Self {
        GameEvent::TraitorDiscarded(e)
    }
}

impl From<Rc<TraitorsSelected>> for GameEvent {
    fn from(e: Rc<TraitorsSelected>) -> Self {
        GameEvent::TraitorsSelected(e)
    }
}

impl From<Rc<TreacheryCalled>> for GameEvent {
    fn from(e: Rc<TreacheryCalled>) -> Self {
        GameEvent::TreacheryCalled(e)
    }
}

impl From<Rc<Voice>> for GameEvent {
    fn from(e: Rc<Voice>) -> Self {
        GameEvent::Voice(e)
    }
}

/// The type of a concrete event (`GetType()`, `typeof(X)`).
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub enum GameEventType {
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
}

impl GameEventType {
    pub const ALL: &'static [GameEventType] = &[
        GameEventType::AcceptOrCancelTleilaxuRevival,
        GameEventType::AllianceBroken,
        GameEventType::AllianceByAmbassador,
        GameEventType::AllianceByTerror,
        GameEventType::AllianceOffered,
        GameEventType::AlliancePassed,
        GameEventType::AllyPermission,
        GameEventType::AmalPlayed,
        GameEventType::AmbassadorActivated,
        GameEventType::AmbassadorPlaced,
        GameEventType::AuditCancelled,
        GameEventType::Audited,
        GameEventType::AutomationConfigured,
        GameEventType::Battle,
        GameEventType::BattleClaimed,
        GameEventType::BattleConcluded,
        GameEventType::BattleInitiated,
        GameEventType::BattleRevision,
        GameEventType::BeneGesseritAccompanies,
        GameEventType::BeneGesseritBattleAnnouncement,
        GameEventType::BeneGesseritFlip,
        GameEventType::BeneGesseritPrediction,
        GameEventType::Bid,
        GameEventType::BlackMarketBid,
        GameEventType::Bureaucracy,
        GameEventType::Captured,
        GameEventType::Caravan,
        GameEventType::CardGiven,
        GameEventType::CardTraded,
        GameEventType::CardsDetermined,
        GameEventType::CharityClaimed,
        GameEventType::ChoamDiscarded,
        GameEventType::ChoamEconomics,
        GameEventType::ChoamExtraMove,
        GameEventType::ChoamFreeRevivalPrevention,
        GameEventType::ChoamKarmaPrevention,
        GameEventType::ChoamMovePrevention,
        GameEventType::ChoamRemoveForce,
        GameEventType::ClairVoyanceAnswered,
        GameEventType::ClairVoyancePlayed,
        GameEventType::DealAccepted,
        GameEventType::DealOffered,
        GameEventType::DealRejected,
        GameEventType::Diplomacy,
        GameEventType::Discarded,
        GameEventType::DiscardedSearched,
        GameEventType::DiscardedSearchedAnnounced,
        GameEventType::DiscardedTaken,
        GameEventType::DiscoveryEntered,
        GameEventType::DiscoveryRevealed,
        GameEventType::DistransUsed,
        GameEventType::DivideResources,
        GameEventType::DivideResourcesAccepted,
        GameEventType::Donated,
        GameEventType::EmperorBidSupport,
        GameEventType::EmperorDiscarded,
        GameEventType::EndPhase,
        GameEventType::EstablishPlayers,
        GameEventType::ExtortionPrevented,
        GameEventType::FaceDanced,
        GameEventType::FaceDancerReplaced,
        GameEventType::FaceDancerRevealed,
        GameEventType::FactionSelected,
        GameEventType::FactionTradeOffered,
        GameEventType::FlightDiscoveryUsed,
        GameEventType::FlightUsed,
        GameEventType::FremenRidesMonster,
        GameEventType::FremenSentMonster,
        GameEventType::GuildDelay,
        GameEventType::HMSAdvantageChosen,
        GameEventType::HarvesterPlayed,
        GameEventType::HideSecrets,
        GameEventType::IxianRemovedCardFromAuction,
        GameEventType::IxianSelectedStartingCard,
        GameEventType::IxianSwappedCardOnBid,
        GameEventType::JuicePlayed,
        GameEventType::Karma,
        GameEventType::KarmaChoamDiscard,
        GameEventType::KarmaEcazDial,
        GameEventType::KarmaFreeRevival,
        GameEventType::KarmaHandSwap,
        GameEventType::KarmaHandSwapInitiated,
        GameEventType::KarmaHmsMovement,
        GameEventType::KarmaMonster,
        GameEventType::KarmaPrescience,
        GameEventType::KarmaRevivalPrevention,
        GameEventType::KarmaRicheseBuy,
        GameEventType::KarmaShipmentPrevention,
        GameEventType::LoserConcluded,
        GameEventType::MetheorPlayed,
        GameEventType::Move,
        GameEventType::MulliganPerformed,
        GameEventType::NexusCardDrawn,
        GameEventType::NexusPlayed,
        GameEventType::NexusVoted,
        GameEventType::PerformBeneGesseritPlacement,
        GameEventType::PerformFremenSetup,
        GameEventType::PerformHmsMovement,
        GameEventType::PerformHmsPlacement,
        GameEventType::PerformMoritaniSetup,
        GameEventType::PerformSetup,
        GameEventType::Planetology,
        GameEventType::PlayerReplaced,
        GameEventType::PoisonToothCancelled,
        GameEventType::PortableAntidoteUsed,
        GameEventType::Prescience,
        GameEventType::RaiseDeadPlayed,
        GameEventType::RecruitsPlayed,
        GameEventType::ReplacedCardWon,
        GameEventType::RequestTleilaxuRevival,
        GameEventType::ResidualPlayed,
        GameEventType::ResourcesAudited,
        GameEventType::ResourcesTransferred,
        GameEventType::Retreat,
        GameEventType::Revival,
        GameEventType::RicheseAnnouncesAuction,
        GameEventType::RicheseAnnouncesBlackMarket,
        GameEventType::RicheseGaveCard,
        GameEventType::RicheseKeepsUnsoldCard,
        GameEventType::RicheseRevealedNoField,
        GameEventType::RicheseSpecifiesAuction,
        GameEventType::RockWasMelted,
        GameEventType::SetIncreasedRevivalLimits,
        GameEventType::SetShipmentPermission,
        GameEventType::Shipment,
        GameEventType::SkillAssigned,
        GameEventType::StormDialled,
        GameEventType::StormSpellPlayed,
        GameEventType::SwitchedSkilledLeader,
        GameEventType::TakeLosses,
        GameEventType::TerrorPlanted,
        GameEventType::TerrorRevealed,
        GameEventType::TestingStationUsed,
        GameEventType::Thought,
        GameEventType::ThoughtAnswered,
        GameEventType::ThumperPlayed,
        GameEventType::TraitorDiscarded,
        GameEventType::TraitorsSelected,
        GameEventType::TreacheryCalled,
        GameEventType::Voice,
    ];

    /// `Type.Name`
    pub fn name(self) -> &'static str {
        match self {
            GameEventType::AcceptOrCancelTleilaxuRevival => "AcceptOrCancelTleilaxuRevival",
            GameEventType::AllianceBroken => "AllianceBroken",
            GameEventType::AllianceByAmbassador => "AllianceByAmbassador",
            GameEventType::AllianceByTerror => "AllianceByTerror",
            GameEventType::AllianceOffered => "AllianceOffered",
            GameEventType::AlliancePassed => "AlliancePassed",
            GameEventType::AllyPermission => "AllyPermission",
            GameEventType::AmalPlayed => "AmalPlayed",
            GameEventType::AmbassadorActivated => "AmbassadorActivated",
            GameEventType::AmbassadorPlaced => "AmbassadorPlaced",
            GameEventType::AuditCancelled => "AuditCancelled",
            GameEventType::Audited => "Audited",
            GameEventType::AutomationConfigured => "AutomationConfigured",
            GameEventType::Battle => "Battle",
            GameEventType::BattleClaimed => "BattleClaimed",
            GameEventType::BattleConcluded => "BattleConcluded",
            GameEventType::BattleInitiated => "BattleInitiated",
            GameEventType::BattleRevision => "BattleRevision",
            GameEventType::BeneGesseritAccompanies => "BeneGesseritAccompanies",
            GameEventType::BeneGesseritBattleAnnouncement => "BeneGesseritBattleAnnouncement",
            GameEventType::BeneGesseritFlip => "BeneGesseritFlip",
            GameEventType::BeneGesseritPrediction => "BeneGesseritPrediction",
            GameEventType::Bid => "Bid",
            GameEventType::BlackMarketBid => "BlackMarketBid",
            GameEventType::Bureaucracy => "Bureaucracy",
            GameEventType::Captured => "Captured",
            GameEventType::Caravan => "Caravan",
            GameEventType::CardGiven => "CardGiven",
            GameEventType::CardTraded => "CardTraded",
            GameEventType::CardsDetermined => "CardsDetermined",
            GameEventType::CharityClaimed => "CharityClaimed",
            GameEventType::ChoamDiscarded => "ChoamDiscarded",
            GameEventType::ChoamEconomics => "ChoamEconomics",
            GameEventType::ChoamExtraMove => "ChoamExtraMove",
            GameEventType::ChoamFreeRevivalPrevention => "ChoamFreeRevivalPrevention",
            GameEventType::ChoamKarmaPrevention => "ChoamKarmaPrevention",
            GameEventType::ChoamMovePrevention => "ChoamMovePrevention",
            GameEventType::ChoamRemoveForce => "ChoamRemoveForce",
            GameEventType::ClairVoyanceAnswered => "ClairVoyanceAnswered",
            GameEventType::ClairVoyancePlayed => "ClairVoyancePlayed",
            GameEventType::DealAccepted => "DealAccepted",
            GameEventType::DealOffered => "DealOffered",
            GameEventType::DealRejected => "DealRejected",
            GameEventType::Diplomacy => "Diplomacy",
            GameEventType::Discarded => "Discarded",
            GameEventType::DiscardedSearched => "DiscardedSearched",
            GameEventType::DiscardedSearchedAnnounced => "DiscardedSearchedAnnounced",
            GameEventType::DiscardedTaken => "DiscardedTaken",
            GameEventType::DiscoveryEntered => "DiscoveryEntered",
            GameEventType::DiscoveryRevealed => "DiscoveryRevealed",
            GameEventType::DistransUsed => "DistransUsed",
            GameEventType::DivideResources => "DivideResources",
            GameEventType::DivideResourcesAccepted => "DivideResourcesAccepted",
            GameEventType::Donated => "Donated",
            GameEventType::EmperorBidSupport => "EmperorBidSupport",
            GameEventType::EmperorDiscarded => "EmperorDiscarded",
            GameEventType::EndPhase => "EndPhase",
            GameEventType::EstablishPlayers => "EstablishPlayers",
            GameEventType::ExtortionPrevented => "ExtortionPrevented",
            GameEventType::FaceDanced => "FaceDanced",
            GameEventType::FaceDancerReplaced => "FaceDancerReplaced",
            GameEventType::FaceDancerRevealed => "FaceDancerRevealed",
            GameEventType::FactionSelected => "FactionSelected",
            GameEventType::FactionTradeOffered => "FactionTradeOffered",
            GameEventType::FlightDiscoveryUsed => "FlightDiscoveryUsed",
            GameEventType::FlightUsed => "FlightUsed",
            GameEventType::FremenRidesMonster => "FremenRidesMonster",
            GameEventType::FremenSentMonster => "FremenSentMonster",
            GameEventType::GuildDelay => "GuildDelay",
            GameEventType::HMSAdvantageChosen => "HMSAdvantageChosen",
            GameEventType::HarvesterPlayed => "HarvesterPlayed",
            GameEventType::HideSecrets => "HideSecrets",
            GameEventType::IxianRemovedCardFromAuction => "IxianRemovedCardFromAuction",
            GameEventType::IxianSelectedStartingCard => "IxianSelectedStartingCard",
            GameEventType::IxianSwappedCardOnBid => "IxianSwappedCardOnBid",
            GameEventType::JuicePlayed => "JuicePlayed",
            GameEventType::Karma => "Karma",
            GameEventType::KarmaChoamDiscard => "KarmaChoamDiscard",
            GameEventType::KarmaEcazDial => "KarmaEcazDial",
            GameEventType::KarmaFreeRevival => "KarmaFreeRevival",
            GameEventType::KarmaHandSwap => "KarmaHandSwap",
            GameEventType::KarmaHandSwapInitiated => "KarmaHandSwapInitiated",
            GameEventType::KarmaHmsMovement => "KarmaHmsMovement",
            GameEventType::KarmaMonster => "KarmaMonster",
            GameEventType::KarmaPrescience => "KarmaPrescience",
            GameEventType::KarmaRevivalPrevention => "KarmaRevivalPrevention",
            GameEventType::KarmaRicheseBuy => "KarmaRicheseBuy",
            GameEventType::KarmaShipmentPrevention => "KarmaShipmentPrevention",
            GameEventType::LoserConcluded => "LoserConcluded",
            GameEventType::MetheorPlayed => "MetheorPlayed",
            GameEventType::Move => "Move",
            GameEventType::MulliganPerformed => "MulliganPerformed",
            GameEventType::NexusCardDrawn => "NexusCardDrawn",
            GameEventType::NexusPlayed => "NexusPlayed",
            GameEventType::NexusVoted => "NexusVoted",
            GameEventType::PerformBeneGesseritPlacement => "PerformBeneGesseritPlacement",
            GameEventType::PerformFremenSetup => "PerformFremenSetup",
            GameEventType::PerformHmsMovement => "PerformHmsMovement",
            GameEventType::PerformHmsPlacement => "PerformHmsPlacement",
            GameEventType::PerformMoritaniSetup => "PerformMoritaniSetup",
            GameEventType::PerformSetup => "PerformSetup",
            GameEventType::Planetology => "Planetology",
            GameEventType::PlayerReplaced => "PlayerReplaced",
            GameEventType::PoisonToothCancelled => "PoisonToothCancelled",
            GameEventType::PortableAntidoteUsed => "PortableAntidoteUsed",
            GameEventType::Prescience => "Prescience",
            GameEventType::RaiseDeadPlayed => "RaiseDeadPlayed",
            GameEventType::RecruitsPlayed => "RecruitsPlayed",
            GameEventType::ReplacedCardWon => "ReplacedCardWon",
            GameEventType::RequestTleilaxuRevival => "RequestTleilaxuRevival",
            GameEventType::ResidualPlayed => "ResidualPlayed",
            GameEventType::ResourcesAudited => "ResourcesAudited",
            GameEventType::ResourcesTransferred => "ResourcesTransferred",
            GameEventType::Retreat => "Retreat",
            GameEventType::Revival => "Revival",
            GameEventType::RicheseAnnouncesAuction => "RicheseAnnouncesAuction",
            GameEventType::RicheseAnnouncesBlackMarket => "RicheseAnnouncesBlackMarket",
            GameEventType::RicheseGaveCard => "RicheseGaveCard",
            GameEventType::RicheseKeepsUnsoldCard => "RicheseKeepsUnsoldCard",
            GameEventType::RicheseRevealedNoField => "RicheseRevealedNoField",
            GameEventType::RicheseSpecifiesAuction => "RicheseSpecifiesAuction",
            GameEventType::RockWasMelted => "RockWasMelted",
            GameEventType::SetIncreasedRevivalLimits => "SetIncreasedRevivalLimits",
            GameEventType::SetShipmentPermission => "SetShipmentPermission",
            GameEventType::Shipment => "Shipment",
            GameEventType::SkillAssigned => "SkillAssigned",
            GameEventType::StormDialled => "StormDialled",
            GameEventType::StormSpellPlayed => "StormSpellPlayed",
            GameEventType::SwitchedSkilledLeader => "SwitchedSkilledLeader",
            GameEventType::TakeLosses => "TakeLosses",
            GameEventType::TerrorPlanted => "TerrorPlanted",
            GameEventType::TerrorRevealed => "TerrorRevealed",
            GameEventType::TestingStationUsed => "TestingStationUsed",
            GameEventType::Thought => "Thought",
            GameEventType::ThoughtAnswered => "ThoughtAnswered",
            GameEventType::ThumperPlayed => "ThumperPlayed",
            GameEventType::TraitorDiscarded => "TraitorDiscarded",
            GameEventType::TraitorsSelected => "TraitorsSelected",
            GameEventType::TreacheryCalled => "TreacheryCalled",
            GameEventType::Voice => "Voice",
        }
    }

    /// The `$type` discriminator from `[JsonDerivedType]` on `GameEvent`; `None` for types without one.
    pub fn json_discriminator(self) -> Option<&'static str> {
        match self {
            GameEventType::AcceptOrCancelTleilaxuRevival => Some("AcceptOrCancelTleilaxuRevival"),
            GameEventType::AllianceBroken => Some("AllianceBroken"),
            GameEventType::AllianceByAmbassador => Some("AllianceByAmbassador"),
            GameEventType::AllianceByTerror => Some("AllianceByTerror"),
            GameEventType::AllianceOffered => Some("AllianceOffered"),
            GameEventType::AlliancePassed => Some("AlliancePassed"),
            GameEventType::AllyPermission => Some("AllyPermission"),
            GameEventType::AmalPlayed => Some("AmalPlayed"),
            GameEventType::AmbassadorActivated => Some("AmbassadorActivated"),
            GameEventType::AmbassadorPlaced => Some("AmbassadorPlaced"),
            GameEventType::AuditCancelled => Some("AuditCancelled"),
            GameEventType::Audited => Some("Audited"),
            GameEventType::AutomationConfigured => Some("AutomationConfigured"),
            GameEventType::Battle => Some("Battle"),
            GameEventType::BattleClaimed => Some("BattleClaimed"),
            GameEventType::BattleConcluded => Some("BattleConcluded"),
            GameEventType::BattleInitiated => Some("BattleInitiated"),
            GameEventType::BattleRevision => Some("BattleRevision"),
            GameEventType::BeneGesseritAccompanies => Some("BeneGesseritAccompanies"),
            GameEventType::BeneGesseritBattleAnnouncement => Some("BeneGesseritBattleAnnouncement"),
            GameEventType::BeneGesseritFlip => Some("BeneGesseritFlip"),
            GameEventType::BeneGesseritPrediction => Some("BeneGesseritPrediction"),
            GameEventType::Bid => Some("Bid"),
            GameEventType::BlackMarketBid => Some("BlackMarketBid"),
            GameEventType::Bureaucracy => Some("Bureaucracy"),
            GameEventType::Captured => Some("Captured"),
            GameEventType::Caravan => Some("Caravan"),
            GameEventType::CardGiven => Some("CardGiven"),
            GameEventType::CardTraded => Some("CardTraded"),
            GameEventType::CardsDetermined => Some("CardsDetermined"),
            GameEventType::CharityClaimed => Some("CharityClaimed"),
            GameEventType::ChoamDiscarded => Some("ChoamDiscarded"),
            GameEventType::ChoamEconomics => Some("ChoamEconomics"),
            GameEventType::ChoamExtraMove => Some("ChoamExtraMove"),
            GameEventType::ChoamFreeRevivalPrevention => Some("ChoamFreeRevivalPrevention"),
            GameEventType::ChoamKarmaPrevention => Some("ChoamKarmaPrevention"),
            GameEventType::ChoamMovePrevention => Some("ChoamMovePrevention"),
            GameEventType::ChoamRemoveForce => Some("ChoamRemoveForce"),
            GameEventType::ClairVoyanceAnswered => Some("ClairVoyanceAnswered"),
            GameEventType::ClairVoyancePlayed => Some("ClairVoyancePlayed"),
            GameEventType::DealAccepted => Some("DealAccepted"),
            GameEventType::DealOffered => Some("DealOffered"),
            GameEventType::DealRejected => Some("DealRejected"),
            GameEventType::Diplomacy => Some("Diplomacy"),
            GameEventType::Discarded => Some("Discarded"),
            GameEventType::DiscardedSearched => Some("DiscardedSearched"),
            GameEventType::DiscardedSearchedAnnounced => Some("DiscardedSearchedAnnounced"),
            GameEventType::DiscardedTaken => Some("DiscardedTaken"),
            GameEventType::DiscoveryEntered => Some("DiscoveryEntered"),
            GameEventType::DiscoveryRevealed => Some("DiscoveryRevealed"),
            GameEventType::DistransUsed => Some("DistransUsed"),
            GameEventType::DivideResources => Some("DivideResources"),
            GameEventType::DivideResourcesAccepted => Some("DivideResourcesAccepted"),
            GameEventType::Donated => Some("Donated"),
            GameEventType::EmperorBidSupport => Some("EmperorBidSupport"),
            GameEventType::EmperorDiscarded => Some("EmperorDiscarded"),
            GameEventType::EndPhase => Some("EndPhase"),
            GameEventType::EstablishPlayers => Some("EstablishPlayers"),
            GameEventType::ExtortionPrevented => Some("ExtortionPrevented"),
            GameEventType::FaceDanced => Some("FaceDanced"),
            GameEventType::FaceDancerReplaced => Some("FaceDancerReplaced"),
            GameEventType::FaceDancerRevealed => Some("FaceDancerRevealed"),
            GameEventType::FactionSelected => Some("FactionSelected"),
            GameEventType::FactionTradeOffered => Some("FactionTradeOffered"),
            GameEventType::FlightDiscoveryUsed => Some("FlightDiscoveryUsed"),
            GameEventType::FlightUsed => Some("FlightUsed"),
            GameEventType::FremenRidesMonster => Some("FremenRidesMonster"),
            GameEventType::FremenSentMonster => Some("FremenSentMonster"),
            GameEventType::GuildDelay => Some("GuildDelay"),
            GameEventType::HMSAdvantageChosen => Some("HMSAdvantageChosen"),
            GameEventType::HarvesterPlayed => Some("HarvesterPlayed"),
            GameEventType::HideSecrets => Some("HideSecrets"),
            GameEventType::IxianRemovedCardFromAuction => Some("IxianRemovedCardFromAuction"),
            GameEventType::IxianSelectedStartingCard => Some("IxianSelectedStartingCard"),
            GameEventType::IxianSwappedCardOnBid => Some("IxianSwappedCardOnBid"),
            GameEventType::JuicePlayed => Some("JuicePlayed"),
            GameEventType::Karma => Some("Karma"),
            GameEventType::KarmaChoamDiscard => Some("KarmaChoamDiscard"),
            GameEventType::KarmaEcazDial => Some("KarmaEcazDial"),
            GameEventType::KarmaFreeRevival => Some("KarmaFreeRevival"),
            GameEventType::KarmaHandSwap => Some("KarmaHandSwap"),
            GameEventType::KarmaHandSwapInitiated => Some("KarmaHandSwapInitiated"),
            GameEventType::KarmaHmsMovement => Some("KarmaHmsMovement"),
            GameEventType::KarmaMonster => Some("KarmaMonster"),
            GameEventType::KarmaPrescience => Some("KarmaPrescience"),
            GameEventType::KarmaRevivalPrevention => Some("KarmaRevivalPrevention"),
            GameEventType::KarmaRicheseBuy => Some("KarmaRicheseBuy"),
            GameEventType::KarmaShipmentPrevention => Some("KarmaShipmentPrevention"),
            GameEventType::LoserConcluded => Some("LoserConcluded"),
            GameEventType::MetheorPlayed => Some("MetheorPlayed"),
            GameEventType::Move => Some("Move"),
            GameEventType::MulliganPerformed => Some("MulliganPerformed"),
            GameEventType::NexusCardDrawn => Some("NexusCardDrawn"),
            GameEventType::NexusPlayed => Some("NexusPlayed"),
            GameEventType::NexusVoted => Some("NexusVoted"),
            GameEventType::PerformBeneGesseritPlacement => Some("PerformBeneGesseritPlacement"),
            GameEventType::PerformFremenSetup => Some("PerformFremenSetup"),
            GameEventType::PerformHmsMovement => Some("PerformHmsMovement"),
            GameEventType::PerformHmsPlacement => Some("PerformHmsPlacement"),
            GameEventType::PerformMoritaniSetup => Some("PerformMoritaniSetup"),
            GameEventType::PerformSetup => Some("PerformSetup"),
            GameEventType::Planetology => Some("Planetology"),
            GameEventType::PlayerReplaced => Some("PlayerReplaced"),
            GameEventType::PoisonToothCancelled => Some("PoisonToothCancelled"),
            GameEventType::PortableAntidoteUsed => Some("PortableAntidoteUsed"),
            GameEventType::Prescience => Some("Prescience"),
            GameEventType::RaiseDeadPlayed => Some("RaiseDeadPlayed"),
            GameEventType::RecruitsPlayed => Some("RecruitsPlayed"),
            GameEventType::ReplacedCardWon => Some("ReplacedCardWon"),
            GameEventType::RequestTleilaxuRevival => Some("RequestTleilaxuRevival"),
            GameEventType::ResidualPlayed => Some("ResidualPlayed"),
            GameEventType::ResourcesAudited => Some("ResourcesAudited"),
            GameEventType::ResourcesTransferred => Some("ResourcesTransferred"),
            GameEventType::Retreat => Some("Retreat"),
            GameEventType::Revival => Some("Revival"),
            GameEventType::RicheseAnnouncesAuction => Some("RicheseAnnouncesAuction"),
            GameEventType::RicheseAnnouncesBlackMarket => Some("RicheseAnnouncesBlackMarket"),
            GameEventType::RicheseGaveCard => Some("RicheseGaveCard"),
            GameEventType::RicheseKeepsUnsoldCard => Some("RicheseKeepsUnsoldCard"),
            GameEventType::RicheseRevealedNoField => Some("RicheseRevealedNoField"),
            GameEventType::RicheseSpecifiesAuction => Some("RicheseSpecifiesAuction"),
            GameEventType::RockWasMelted => Some("RockWasMelted"),
            GameEventType::SetIncreasedRevivalLimits => Some("SetIncreasedRevivalLimits"),
            GameEventType::SetShipmentPermission => Some("SetShipmentPermission"),
            GameEventType::Shipment => Some("Shipment"),
            GameEventType::SkillAssigned => Some("SkillAssigned"),
            GameEventType::StormDialled => Some("StormDialled"),
            GameEventType::StormSpellPlayed => Some("StormSpellPlayed"),
            GameEventType::SwitchedSkilledLeader => Some("SwitchedSkilledLeader"),
            GameEventType::TakeLosses => Some("TakeLosses"),
            GameEventType::TerrorPlanted => Some("TerrorPlanted"),
            GameEventType::TerrorRevealed => Some("TerrorRevealed"),
            GameEventType::TestingStationUsed => Some("TestingStationUsed"),
            GameEventType::Thought => Some("Thought"),
            GameEventType::ThoughtAnswered => Some("ThoughtAnswered"),
            GameEventType::ThumperPlayed => Some("ThumperPlayed"),
            GameEventType::TraitorDiscarded => Some("TraitorDiscarded"),
            GameEventType::TraitorsSelected => Some("TraitorsSelected"),
            GameEventType::TreacheryCalled => Some("TreacheryCalled"),
            GameEventType::Voice => Some("Voice"),
        }
    }

    /// Inverse of [`Self::json_discriminator`] (ordinal, case-sensitive).
    pub fn from_json_discriminator(s: &str) -> Option<GameEventType> {
        match s {
            "AcceptOrCancelTleilaxuRevival" => Some(GameEventType::AcceptOrCancelTleilaxuRevival),
            "AllianceBroken" => Some(GameEventType::AllianceBroken),
            "AllianceByAmbassador" => Some(GameEventType::AllianceByAmbassador),
            "AllianceByTerror" => Some(GameEventType::AllianceByTerror),
            "AllianceOffered" => Some(GameEventType::AllianceOffered),
            "AlliancePassed" => Some(GameEventType::AlliancePassed),
            "AllyPermission" => Some(GameEventType::AllyPermission),
            "AmalPlayed" => Some(GameEventType::AmalPlayed),
            "AmbassadorActivated" => Some(GameEventType::AmbassadorActivated),
            "AmbassadorPlaced" => Some(GameEventType::AmbassadorPlaced),
            "AuditCancelled" => Some(GameEventType::AuditCancelled),
            "Audited" => Some(GameEventType::Audited),
            "AutomationConfigured" => Some(GameEventType::AutomationConfigured),
            "Battle" => Some(GameEventType::Battle),
            "BattleClaimed" => Some(GameEventType::BattleClaimed),
            "BattleConcluded" => Some(GameEventType::BattleConcluded),
            "BattleInitiated" => Some(GameEventType::BattleInitiated),
            "BattleRevision" => Some(GameEventType::BattleRevision),
            "BeneGesseritAccompanies" => Some(GameEventType::BeneGesseritAccompanies),
            "BeneGesseritBattleAnnouncement" => Some(GameEventType::BeneGesseritBattleAnnouncement),
            "BeneGesseritFlip" => Some(GameEventType::BeneGesseritFlip),
            "BeneGesseritPrediction" => Some(GameEventType::BeneGesseritPrediction),
            "Bid" => Some(GameEventType::Bid),
            "BlackMarketBid" => Some(GameEventType::BlackMarketBid),
            "Bureaucracy" => Some(GameEventType::Bureaucracy),
            "Captured" => Some(GameEventType::Captured),
            "Caravan" => Some(GameEventType::Caravan),
            "CardGiven" => Some(GameEventType::CardGiven),
            "CardTraded" => Some(GameEventType::CardTraded),
            "CardsDetermined" => Some(GameEventType::CardsDetermined),
            "CharityClaimed" => Some(GameEventType::CharityClaimed),
            "ChoamDiscarded" => Some(GameEventType::ChoamDiscarded),
            "ChoamEconomics" => Some(GameEventType::ChoamEconomics),
            "ChoamExtraMove" => Some(GameEventType::ChoamExtraMove),
            "ChoamFreeRevivalPrevention" => Some(GameEventType::ChoamFreeRevivalPrevention),
            "ChoamKarmaPrevention" => Some(GameEventType::ChoamKarmaPrevention),
            "ChoamMovePrevention" => Some(GameEventType::ChoamMovePrevention),
            "ChoamRemoveForce" => Some(GameEventType::ChoamRemoveForce),
            "ClairVoyanceAnswered" => Some(GameEventType::ClairVoyanceAnswered),
            "ClairVoyancePlayed" => Some(GameEventType::ClairVoyancePlayed),
            "DealAccepted" => Some(GameEventType::DealAccepted),
            "DealOffered" => Some(GameEventType::DealOffered),
            "DealRejected" => Some(GameEventType::DealRejected),
            "Diplomacy" => Some(GameEventType::Diplomacy),
            "Discarded" => Some(GameEventType::Discarded),
            "DiscardedSearched" => Some(GameEventType::DiscardedSearched),
            "DiscardedSearchedAnnounced" => Some(GameEventType::DiscardedSearchedAnnounced),
            "DiscardedTaken" => Some(GameEventType::DiscardedTaken),
            "DiscoveryEntered" => Some(GameEventType::DiscoveryEntered),
            "DiscoveryRevealed" => Some(GameEventType::DiscoveryRevealed),
            "DistransUsed" => Some(GameEventType::DistransUsed),
            "DivideResources" => Some(GameEventType::DivideResources),
            "DivideResourcesAccepted" => Some(GameEventType::DivideResourcesAccepted),
            "Donated" => Some(GameEventType::Donated),
            "EmperorBidSupport" => Some(GameEventType::EmperorBidSupport),
            "EmperorDiscarded" => Some(GameEventType::EmperorDiscarded),
            "EndPhase" => Some(GameEventType::EndPhase),
            "EstablishPlayers" => Some(GameEventType::EstablishPlayers),
            "ExtortionPrevented" => Some(GameEventType::ExtortionPrevented),
            "FaceDanced" => Some(GameEventType::FaceDanced),
            "FaceDancerReplaced" => Some(GameEventType::FaceDancerReplaced),
            "FaceDancerRevealed" => Some(GameEventType::FaceDancerRevealed),
            "FactionSelected" => Some(GameEventType::FactionSelected),
            "FactionTradeOffered" => Some(GameEventType::FactionTradeOffered),
            "FlightDiscoveryUsed" => Some(GameEventType::FlightDiscoveryUsed),
            "FlightUsed" => Some(GameEventType::FlightUsed),
            "FremenRidesMonster" => Some(GameEventType::FremenRidesMonster),
            "FremenSentMonster" => Some(GameEventType::FremenSentMonster),
            "GuildDelay" => Some(GameEventType::GuildDelay),
            "HMSAdvantageChosen" => Some(GameEventType::HMSAdvantageChosen),
            "HarvesterPlayed" => Some(GameEventType::HarvesterPlayed),
            "HideSecrets" => Some(GameEventType::HideSecrets),
            "IxianRemovedCardFromAuction" => Some(GameEventType::IxianRemovedCardFromAuction),
            "IxianSelectedStartingCard" => Some(GameEventType::IxianSelectedStartingCard),
            "IxianSwappedCardOnBid" => Some(GameEventType::IxianSwappedCardOnBid),
            "JuicePlayed" => Some(GameEventType::JuicePlayed),
            "Karma" => Some(GameEventType::Karma),
            "KarmaChoamDiscard" => Some(GameEventType::KarmaChoamDiscard),
            "KarmaEcazDial" => Some(GameEventType::KarmaEcazDial),
            "KarmaFreeRevival" => Some(GameEventType::KarmaFreeRevival),
            "KarmaHandSwap" => Some(GameEventType::KarmaHandSwap),
            "KarmaHandSwapInitiated" => Some(GameEventType::KarmaHandSwapInitiated),
            "KarmaHmsMovement" => Some(GameEventType::KarmaHmsMovement),
            "KarmaMonster" => Some(GameEventType::KarmaMonster),
            "KarmaPrescience" => Some(GameEventType::KarmaPrescience),
            "KarmaRevivalPrevention" => Some(GameEventType::KarmaRevivalPrevention),
            "KarmaRicheseBuy" => Some(GameEventType::KarmaRicheseBuy),
            "KarmaShipmentPrevention" => Some(GameEventType::KarmaShipmentPrevention),
            "LoserConcluded" => Some(GameEventType::LoserConcluded),
            "MetheorPlayed" => Some(GameEventType::MetheorPlayed),
            "Move" => Some(GameEventType::Move),
            "MulliganPerformed" => Some(GameEventType::MulliganPerformed),
            "NexusCardDrawn" => Some(GameEventType::NexusCardDrawn),
            "NexusPlayed" => Some(GameEventType::NexusPlayed),
            "NexusVoted" => Some(GameEventType::NexusVoted),
            "PerformBeneGesseritPlacement" => Some(GameEventType::PerformBeneGesseritPlacement),
            "PerformFremenSetup" => Some(GameEventType::PerformFremenSetup),
            "PerformHmsMovement" => Some(GameEventType::PerformHmsMovement),
            "PerformHmsPlacement" => Some(GameEventType::PerformHmsPlacement),
            "PerformMoritaniSetup" => Some(GameEventType::PerformMoritaniSetup),
            "PerformSetup" => Some(GameEventType::PerformSetup),
            "Planetology" => Some(GameEventType::Planetology),
            "PlayerReplaced" => Some(GameEventType::PlayerReplaced),
            "PoisonToothCancelled" => Some(GameEventType::PoisonToothCancelled),
            "PortableAntidoteUsed" => Some(GameEventType::PortableAntidoteUsed),
            "Prescience" => Some(GameEventType::Prescience),
            "RaiseDeadPlayed" => Some(GameEventType::RaiseDeadPlayed),
            "RecruitsPlayed" => Some(GameEventType::RecruitsPlayed),
            "ReplacedCardWon" => Some(GameEventType::ReplacedCardWon),
            "RequestTleilaxuRevival" => Some(GameEventType::RequestTleilaxuRevival),
            "ResidualPlayed" => Some(GameEventType::ResidualPlayed),
            "ResourcesAudited" => Some(GameEventType::ResourcesAudited),
            "ResourcesTransferred" => Some(GameEventType::ResourcesTransferred),
            "Retreat" => Some(GameEventType::Retreat),
            "Revival" => Some(GameEventType::Revival),
            "RicheseAnnouncesAuction" => Some(GameEventType::RicheseAnnouncesAuction),
            "RicheseAnnouncesBlackMarket" => Some(GameEventType::RicheseAnnouncesBlackMarket),
            "RicheseGaveCard" => Some(GameEventType::RicheseGaveCard),
            "RicheseKeepsUnsoldCard" => Some(GameEventType::RicheseKeepsUnsoldCard),
            "RicheseRevealedNoField" => Some(GameEventType::RicheseRevealedNoField),
            "RicheseSpecifiesAuction" => Some(GameEventType::RicheseSpecifiesAuction),
            "RockWasMelted" => Some(GameEventType::RockWasMelted),
            "SetIncreasedRevivalLimits" => Some(GameEventType::SetIncreasedRevivalLimits),
            "SetShipmentPermission" => Some(GameEventType::SetShipmentPermission),
            "Shipment" => Some(GameEventType::Shipment),
            "SkillAssigned" => Some(GameEventType::SkillAssigned),
            "StormDialled" => Some(GameEventType::StormDialled),
            "StormSpellPlayed" => Some(GameEventType::StormSpellPlayed),
            "SwitchedSkilledLeader" => Some(GameEventType::SwitchedSkilledLeader),
            "TakeLosses" => Some(GameEventType::TakeLosses),
            "TerrorPlanted" => Some(GameEventType::TerrorPlanted),
            "TerrorRevealed" => Some(GameEventType::TerrorRevealed),
            "TestingStationUsed" => Some(GameEventType::TestingStationUsed),
            "Thought" => Some(GameEventType::Thought),
            "ThoughtAnswered" => Some(GameEventType::ThoughtAnswered),
            "ThumperPlayed" => Some(GameEventType::ThumperPlayed),
            "TraitorDiscarded" => Some(GameEventType::TraitorDiscarded),
            "TraitorsSelected" => Some(GameEventType::TraitorsSelected),
            "TreacheryCalled" => Some(GameEventType::TreacheryCalled),
            "Voice" => Some(GameEventType::Voice),
            _ => None,
        }
    }
}

impl std::fmt::Display for GameEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

impl PartialEq for GameEvent {
    /// Identity: two events are equal only if they are the same object.
    fn eq(&self, other: &Self) -> bool {
        self.ptr() == other.ptr()
    }
}

impl Eq for GameEvent {}

impl std::hash::Hash for GameEvent {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.ptr().hash(state)
    }
}

impl std::fmt::Debug for GameEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}@{:p}", self.event_type().name(), self.ptr())
    }
}

/// Virtual dispatch: each method calls the concrete event's implementation of the
/// `GameEventMethods` member of the same name.
impl GameEvent {
    /// Address of the event object (reference identity).
    pub fn ptr(&self) -> *const () {
        dispatch!(self, e => Rc::as_ptr(e) as *const ())
    }

    /// `GetType()`
    pub fn event_type(&self) -> GameEventType {
        dispatch!(self, e => e.event_type())
    }

    pub fn base(&self) -> &GameEventBase {
        dispatch!(self, e => &e.base)
    }

    pub fn initiator(&self) -> Faction {
        dispatch!(self, e => GameEventMethods::initiator(&**e))
    }

    pub fn time(&self) -> DateTimeOffset {
        dispatch!(self, e => GameEventMethods::time(&**e))
    }

    pub fn player(&self) -> PlayerIndex {
        dispatch!(self, e => GameEventMethods::player(&**e))
    }

    pub fn player_opt(&self) -> Option<PlayerIndex> {
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

    pub fn initialize(&self, game: &Game, initiator: Faction) {
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

    pub fn by(&self, f: Faction) -> bool {
        dispatch!(self, e => GameEventMethods::by(&**e, f))
    }

    pub fn by_ally_of(&self, game: &Game, f: Faction) -> bool {
        dispatch!(self, e => GameEventMethods::by_ally_of(&**e, game, f))
    }

    pub fn by_player(&self, game: &Game, p: PlayerIndex) -> bool {
        dispatch!(self, e => GameEventMethods::by_player(&**e, game, p))
    }

    pub fn cs_clone(&self) -> Option<GameEvent> {
        dispatch!(self, e => GameEventMethods::cs_clone(&**e))
    }

    pub fn log(&self, game: &mut Game) {
        dispatch!(self, e => GameEventMethods::log(&**e, game))
    }

    pub fn log_objects(&self, game: &mut Game, expression: Vec<ExpressionElement>) {
        dispatch!(self, e => GameEventMethods::log_objects(&**e, game, expression))
    }

    pub fn log_if(&self, game: &mut Game, condition: bool, expression: Vec<ExpressionElement>) {
        dispatch!(self, e => GameEventMethods::log_if(&**e, game, condition, expression))
    }

    pub fn log_to(&self, game: &mut Game, faction: Faction, expression: Vec<ExpressionElement>) {
        dispatch!(self, e => GameEventMethods::log_to(&**e, game, faction, expression))
    }

    pub fn get_player(&self, game: &Game, f: Option<Faction>) -> Option<PlayerIndex> {
        dispatch!(self, e => GameEventMethods::get_player(&**e, game, f))
    }

    pub fn is_playing(&self, game: &Game, f: Faction) -> bool {
        dispatch!(self, e => GameEventMethods::is_playing(&**e, game, f))
    }

    /// `((IBid)this).TotalAmount`; InvalidCastException for events that do not implement `IBid`.
    pub fn i_bid_total_amount(&self) -> i32 {
        match self {
            GameEvent::Bid(e) => <Bid as bid_trait::IBid>::total_amount(&**e),
            GameEvent::BlackMarketBid(e) => <BlackMarketBid as bid_trait::IBid>::total_amount(&**e),
            _ => panic!("Unable to cast object of type 'Treachery.Shared.{}' to type 'Treachery.Shared.IBid'.", self.event_type().name()),
        }
    }

    /// `((IBid)this).Amount`; InvalidCastException for events that do not implement `IBid`.
    pub fn i_bid_amount(&self) -> i32 {
        match self {
            GameEvent::Bid(e) => <Bid as bid_trait::IBid>::amount(&**e),
            GameEvent::BlackMarketBid(e) => <BlackMarketBid as bid_trait::IBid>::amount(&**e),
            _ => panic!("Unable to cast object of type 'Treachery.Shared.{}' to type 'Treachery.Shared.IBid'.", self.event_type().name()),
        }
    }

    /// `((IBid)this).AllyContributionAmount`; InvalidCastException for events that do not implement `IBid`.
    pub fn i_bid_ally_contribution_amount(&self) -> i32 {
        match self {
            GameEvent::Bid(e) => <Bid as bid_trait::IBid>::ally_contribution_amount(&**e),
            GameEvent::BlackMarketBid(e) => <BlackMarketBid as bid_trait::IBid>::ally_contribution_amount(&**e),
            _ => panic!("Unable to cast object of type 'Treachery.Shared.{}' to type 'Treachery.Shared.IBid'.", self.event_type().name()),
        }
    }

    /// `((IBid)this).EmperorContributionAmount`; InvalidCastException for events that do not implement `IBid`.
    pub fn i_bid_emperor_contribution_amount(&self) -> i32 {
        match self {
            GameEvent::Bid(e) => <Bid as bid_trait::IBid>::emperor_contribution_amount(&**e),
            GameEvent::BlackMarketBid(e) => <BlackMarketBid as bid_trait::IBid>::emperor_contribution_amount(&**e),
            _ => panic!("Unable to cast object of type 'Treachery.Shared.{}' to type 'Treachery.Shared.IBid'.", self.event_type().name()),
        }
    }

    /// `((IBid)this).UsesEmperorSecretAlly`; InvalidCastException for events that do not implement `IBid`.
    pub fn i_bid_uses_emperor_secret_ally(&self) -> bool {
        match self {
            GameEvent::Bid(e) => <Bid as bid_trait::IBid>::uses_emperor_secret_ally(&**e),
            GameEvent::BlackMarketBid(e) => <BlackMarketBid as bid_trait::IBid>::uses_emperor_secret_ally(&**e),
            _ => panic!("Unable to cast object of type 'Treachery.Shared.{}' to type 'Treachery.Shared.IBid'.", self.event_type().name()),
        }
    }

    /// `((IBid)this).Initiator`; InvalidCastException for events that do not implement `IBid`.
    pub fn i_bid_initiator(&self) -> Faction {
        match self {
            GameEvent::Bid(e) => <Bid as bid_trait::IBid>::initiator(&**e),
            GameEvent::BlackMarketBid(e) => <BlackMarketBid as bid_trait::IBid>::initiator(&**e),
            _ => panic!("Unable to cast object of type 'Treachery.Shared.{}' to type 'Treachery.Shared.IBid'.", self.event_type().name()),
        }
    }

    /// `((IBid)this).Player`; InvalidCastException for events that do not implement `IBid`.
    pub fn i_bid_player(&self) -> Option<PlayerIndex> {
        match self {
            GameEvent::Bid(e) => <Bid as bid_trait::IBid>::player(&**e),
            GameEvent::BlackMarketBid(e) => <BlackMarketBid as bid_trait::IBid>::player(&**e),
            _ => panic!("Unable to cast object of type 'Treachery.Shared.{}' to type 'Treachery.Shared.IBid'.", self.event_type().name()),
        }
    }

    /// `((IBid)this).Passed`; InvalidCastException for events that do not implement `IBid`.
    pub fn i_bid_passed(&self) -> bool {
        match self {
            GameEvent::Bid(e) => <Bid as bid_trait::IBid>::passed(&**e),
            GameEvent::BlackMarketBid(e) => <BlackMarketBid as bid_trait::IBid>::passed(&**e),
            _ => panic!("Unable to cast object of type 'Treachery.Shared.{}' to type 'Treachery.Shared.IBid'.", self.event_type().name()),
        }
    }

    /// `((IBid)this).UsingKarmaToRemoveBidLimit`; InvalidCastException for events that do not implement `IBid`.
    pub fn i_bid_using_karma_to_remove_bid_limit(&self) -> bool {
        match self {
            GameEvent::Bid(e) => <Bid as bid_trait::IBid>::using_karma_to_remove_bid_limit(&**e),
            GameEvent::BlackMarketBid(e) => <BlackMarketBid as bid_trait::IBid>::using_karma_to_remove_bid_limit(&**e),
            _ => panic!("Unable to cast object of type 'Treachery.Shared.{}' to type 'Treachery.Shared.IBid'.", self.event_type().name()),
        }
    }

    /// `((IBid)this).KarmaCard`; InvalidCastException for events that do not implement `IBid`.
    pub fn i_bid_karma_card(&self) -> Option<TreacheryCard> {
        match self {
            GameEvent::Bid(e) => <Bid as bid_trait::IBid>::karma_card(&**e),
            GameEvent::BlackMarketBid(e) => <BlackMarketBid as bid_trait::IBid>::karma_card(&**e),
            _ => panic!("Unable to cast object of type 'Treachery.Shared.{}' to type 'Treachery.Shared.IBid'.", self.event_type().name()),
        }
    }

    /// `((IBid)this).GetMessage`; InvalidCastException for events that do not implement `IBid`.
    pub fn i_bid_get_message(&self, game: &Game) -> Message {
        match self {
            GameEvent::Bid(e) => <Bid as bid_trait::IBid>::get_message(&**e, game),
            GameEvent::BlackMarketBid(e) => <BlackMarketBid as bid_trait::IBid>::get_message(&**e, game),
            _ => panic!("Unable to cast object of type 'Treachery.Shared.{}' to type 'Treachery.Shared.IBid'.", self.event_type().name()),
        }
    }

    /// `((IBid)this).KarmaBid`; InvalidCastException for events that do not implement `IBid`.
    pub fn i_bid_karma_bid(&self) -> bool {
        match self {
            GameEvent::Bid(e) => <Bid as bid_trait::IBid>::karma_bid(&**e),
            GameEvent::BlackMarketBid(e) => <BlackMarketBid as bid_trait::IBid>::karma_bid(&**e),
            _ => panic!("Unable to cast object of type 'Treachery.Shared.{}' to type 'Treachery.Shared.IBid'.", self.event_type().name()),
        }
    }

    /// `this is IBid`
    pub fn is_i_bid(&self) -> bool {
        matches!(self, GameEvent::Bid(_) | GameEvent::BlackMarketBid(_))
    }

    /// `((ILocationEvent)this).Initiator`; InvalidCastException for events that do not implement `ILocationEvent`.
    pub fn i_location_event_initiator(&self) -> Faction {
        match self {
            GameEvent::AmbassadorActivated(e) => <AmbassadorActivated as location_event::ILocationEvent>::initiator(&**e),
            GameEvent::BeneGesseritAccompanies(e) => <BeneGesseritAccompanies as location_event::ILocationEvent>::initiator(&**e),
            GameEvent::Caravan(e) => <Caravan as location_event::ILocationEvent>::initiator(&**e),
            GameEvent::DiscoveryEntered(e) => <DiscoveryEntered as location_event::ILocationEvent>::initiator(&**e),
            GameEvent::FaceDanced(e) => <FaceDanced as location_event::ILocationEvent>::initiator(&**e),
            GameEvent::FremenRidesMonster(e) => <FremenRidesMonster as location_event::ILocationEvent>::initiator(&**e),
            GameEvent::Move(e) => <Move as location_event::ILocationEvent>::initiator(&**e),
            GameEvent::NexusPlayed(e) => <NexusPlayed as location_event::ILocationEvent>::initiator(&**e),
            GameEvent::PerformFremenSetup(e) => <PerformFremenSetup as location_event::ILocationEvent>::initiator(&**e),
            GameEvent::PerformSetup(e) => <PerformSetup as location_event::ILocationEvent>::initiator(&**e),
            GameEvent::RaiseDeadPlayed(e) => <RaiseDeadPlayed as location_event::ILocationEvent>::initiator(&**e),
            GameEvent::Revival(e) => <Revival as location_event::ILocationEvent>::initiator(&**e),
            GameEvent::Shipment(e) => <Shipment as location_event::ILocationEvent>::initiator(&**e),
            GameEvent::TerrorRevealed(e) => <TerrorRevealed as location_event::ILocationEvent>::initiator(&**e),
            _ => panic!("Unable to cast object of type 'Treachery.Shared.{}' to type 'Treachery.Shared.ILocationEvent'.", self.event_type().name()),
        }
    }

    /// `((ILocationEvent)this).To`; InvalidCastException for events that do not implement `ILocationEvent`.
    pub fn i_location_event_to(&self, game: &Game) -> Option<Location> {
        match self {
            GameEvent::AmbassadorActivated(e) => <AmbassadorActivated as location_event::ILocationEvent>::to(&**e, game),
            GameEvent::BeneGesseritAccompanies(e) => <BeneGesseritAccompanies as location_event::ILocationEvent>::to(&**e, game),
            GameEvent::Caravan(e) => <Caravan as location_event::ILocationEvent>::to(&**e, game),
            GameEvent::DiscoveryEntered(e) => <DiscoveryEntered as location_event::ILocationEvent>::to(&**e, game),
            GameEvent::FaceDanced(e) => <FaceDanced as location_event::ILocationEvent>::to(&**e, game),
            GameEvent::FremenRidesMonster(e) => <FremenRidesMonster as location_event::ILocationEvent>::to(&**e, game),
            GameEvent::Move(e) => <Move as location_event::ILocationEvent>::to(&**e, game),
            GameEvent::NexusPlayed(e) => <NexusPlayed as location_event::ILocationEvent>::to(&**e, game),
            GameEvent::PerformFremenSetup(e) => <PerformFremenSetup as location_event::ILocationEvent>::to(&**e, game),
            GameEvent::PerformSetup(e) => <PerformSetup as location_event::ILocationEvent>::to(&**e, game),
            GameEvent::RaiseDeadPlayed(e) => <RaiseDeadPlayed as location_event::ILocationEvent>::to(&**e, game),
            GameEvent::Revival(e) => <Revival as location_event::ILocationEvent>::to(&**e, game),
            GameEvent::Shipment(e) => <Shipment as location_event::ILocationEvent>::to(&**e, game),
            GameEvent::TerrorRevealed(e) => <TerrorRevealed as location_event::ILocationEvent>::to(&**e, game),
            _ => panic!("Unable to cast object of type 'Treachery.Shared.{}' to type 'Treachery.Shared.ILocationEvent'.", self.event_type().name()),
        }
    }

    /// `((ILocationEvent)this).TotalAmountOfForcesAddedToLocation`; InvalidCastException for events that do not implement `ILocationEvent`.
    pub fn i_location_event_total_amount_of_forces_added_to_location(&self, game: &Game) -> i32 {
        match self {
            GameEvent::AmbassadorActivated(e) => <AmbassadorActivated as location_event::ILocationEvent>::total_amount_of_forces_added_to_location(&**e, game),
            GameEvent::BeneGesseritAccompanies(e) => <BeneGesseritAccompanies as location_event::ILocationEvent>::total_amount_of_forces_added_to_location(&**e, game),
            GameEvent::Caravan(e) => <Caravan as location_event::ILocationEvent>::total_amount_of_forces_added_to_location(&**e, game),
            GameEvent::DiscoveryEntered(e) => <DiscoveryEntered as location_event::ILocationEvent>::total_amount_of_forces_added_to_location(&**e, game),
            GameEvent::FaceDanced(e) => <FaceDanced as location_event::ILocationEvent>::total_amount_of_forces_added_to_location(&**e, game),
            GameEvent::FremenRidesMonster(e) => <FremenRidesMonster as location_event::ILocationEvent>::total_amount_of_forces_added_to_location(&**e, game),
            GameEvent::Move(e) => <Move as location_event::ILocationEvent>::total_amount_of_forces_added_to_location(&**e, game),
            GameEvent::NexusPlayed(e) => <NexusPlayed as location_event::ILocationEvent>::total_amount_of_forces_added_to_location(&**e, game),
            GameEvent::PerformFremenSetup(e) => <PerformFremenSetup as location_event::ILocationEvent>::total_amount_of_forces_added_to_location(&**e, game),
            GameEvent::PerformSetup(e) => <PerformSetup as location_event::ILocationEvent>::total_amount_of_forces_added_to_location(&**e, game),
            GameEvent::RaiseDeadPlayed(e) => <RaiseDeadPlayed as location_event::ILocationEvent>::total_amount_of_forces_added_to_location(&**e, game),
            GameEvent::Revival(e) => <Revival as location_event::ILocationEvent>::total_amount_of_forces_added_to_location(&**e, game),
            GameEvent::Shipment(e) => <Shipment as location_event::ILocationEvent>::total_amount_of_forces_added_to_location(&**e, game),
            GameEvent::TerrorRevealed(e) => <TerrorRevealed as location_event::ILocationEvent>::total_amount_of_forces_added_to_location(&**e, game),
            _ => panic!("Unable to cast object of type 'Treachery.Shared.{}' to type 'Treachery.Shared.ILocationEvent'.", self.event_type().name()),
        }
    }

    /// `((ILocationEvent)this).ForcesAddedToLocation`; InvalidCastException for events that do not implement `ILocationEvent`.
    pub fn i_location_event_forces_added_to_location(&self, game: &Game) -> i32 {
        match self {
            GameEvent::AmbassadorActivated(e) => <AmbassadorActivated as location_event::ILocationEvent>::forces_added_to_location(&**e, game),
            GameEvent::BeneGesseritAccompanies(e) => <BeneGesseritAccompanies as location_event::ILocationEvent>::forces_added_to_location(&**e, game),
            GameEvent::Caravan(e) => <Caravan as location_event::ILocationEvent>::forces_added_to_location(&**e, game),
            GameEvent::DiscoveryEntered(e) => <DiscoveryEntered as location_event::ILocationEvent>::forces_added_to_location(&**e, game),
            GameEvent::FaceDanced(e) => <FaceDanced as location_event::ILocationEvent>::forces_added_to_location(&**e, game),
            GameEvent::FremenRidesMonster(e) => <FremenRidesMonster as location_event::ILocationEvent>::forces_added_to_location(&**e, game),
            GameEvent::Move(e) => <Move as location_event::ILocationEvent>::forces_added_to_location(&**e, game),
            GameEvent::NexusPlayed(e) => <NexusPlayed as location_event::ILocationEvent>::forces_added_to_location(&**e, game),
            GameEvent::PerformFremenSetup(e) => <PerformFremenSetup as location_event::ILocationEvent>::forces_added_to_location(&**e, game),
            GameEvent::PerformSetup(e) => <PerformSetup as location_event::ILocationEvent>::forces_added_to_location(&**e, game),
            GameEvent::RaiseDeadPlayed(e) => <RaiseDeadPlayed as location_event::ILocationEvent>::forces_added_to_location(&**e, game),
            GameEvent::Revival(e) => <Revival as location_event::ILocationEvent>::forces_added_to_location(&**e, game),
            GameEvent::Shipment(e) => <Shipment as location_event::ILocationEvent>::forces_added_to_location(&**e, game),
            GameEvent::TerrorRevealed(e) => <TerrorRevealed as location_event::ILocationEvent>::forces_added_to_location(&**e, game),
            _ => panic!("Unable to cast object of type 'Treachery.Shared.{}' to type 'Treachery.Shared.ILocationEvent'.", self.event_type().name()),
        }
    }

    /// `((ILocationEvent)this).SpecialForcesAddedToLocation`; InvalidCastException for events that do not implement `ILocationEvent`.
    pub fn i_location_event_special_forces_added_to_location(&self, game: &Game) -> i32 {
        match self {
            GameEvent::AmbassadorActivated(e) => <AmbassadorActivated as location_event::ILocationEvent>::special_forces_added_to_location(&**e, game),
            GameEvent::BeneGesseritAccompanies(e) => <BeneGesseritAccompanies as location_event::ILocationEvent>::special_forces_added_to_location(&**e, game),
            GameEvent::Caravan(e) => <Caravan as location_event::ILocationEvent>::special_forces_added_to_location(&**e, game),
            GameEvent::DiscoveryEntered(e) => <DiscoveryEntered as location_event::ILocationEvent>::special_forces_added_to_location(&**e, game),
            GameEvent::FaceDanced(e) => <FaceDanced as location_event::ILocationEvent>::special_forces_added_to_location(&**e, game),
            GameEvent::FremenRidesMonster(e) => <FremenRidesMonster as location_event::ILocationEvent>::special_forces_added_to_location(&**e, game),
            GameEvent::Move(e) => <Move as location_event::ILocationEvent>::special_forces_added_to_location(&**e, game),
            GameEvent::NexusPlayed(e) => <NexusPlayed as location_event::ILocationEvent>::special_forces_added_to_location(&**e, game),
            GameEvent::PerformFremenSetup(e) => <PerformFremenSetup as location_event::ILocationEvent>::special_forces_added_to_location(&**e, game),
            GameEvent::PerformSetup(e) => <PerformSetup as location_event::ILocationEvent>::special_forces_added_to_location(&**e, game),
            GameEvent::RaiseDeadPlayed(e) => <RaiseDeadPlayed as location_event::ILocationEvent>::special_forces_added_to_location(&**e, game),
            GameEvent::Revival(e) => <Revival as location_event::ILocationEvent>::special_forces_added_to_location(&**e, game),
            GameEvent::Shipment(e) => <Shipment as location_event::ILocationEvent>::special_forces_added_to_location(&**e, game),
            GameEvent::TerrorRevealed(e) => <TerrorRevealed as location_event::ILocationEvent>::special_forces_added_to_location(&**e, game),
            _ => panic!("Unable to cast object of type 'Treachery.Shared.{}' to type 'Treachery.Shared.ILocationEvent'.", self.event_type().name()),
        }
    }

    /// `this is ILocationEvent`
    pub fn is_i_location_event(&self) -> bool {
        matches!(self, GameEvent::AmbassadorActivated(_) | GameEvent::BeneGesseritAccompanies(_) | GameEvent::Caravan(_) | GameEvent::DiscoveryEntered(_) | GameEvent::FaceDanced(_) | GameEvent::FremenRidesMonster(_) | GameEvent::Move(_) | GameEvent::NexusPlayed(_) | GameEvent::PerformFremenSetup(_) | GameEvent::PerformSetup(_) | GameEvent::RaiseDeadPlayed(_) | GameEvent::Revival(_) | GameEvent::Shipment(_) | GameEvent::TerrorRevealed(_))
    }

    /// `((IPlacement)this).Initiator`; InvalidCastException for events that do not implement `IPlacement`.
    pub fn i_placement_initiator(&self) -> Faction {
        match self {
            GameEvent::AmbassadorActivated(e) => <AmbassadorActivated as placement::IPlacement>::initiator(&**e),
            GameEvent::Caravan(e) => <Caravan as placement::IPlacement>::initiator(&**e),
            GameEvent::DiscoveryEntered(e) => <DiscoveryEntered as placement::IPlacement>::initiator(&**e),
            GameEvent::FaceDanced(e) => <FaceDanced as placement::IPlacement>::initiator(&**e),
            GameEvent::FremenRidesMonster(e) => <FremenRidesMonster as placement::IPlacement>::initiator(&**e),
            GameEvent::Move(e) => <Move as placement::IPlacement>::initiator(&**e),
            GameEvent::PerformFremenSetup(e) => <PerformFremenSetup as placement::IPlacement>::initiator(&**e),
            GameEvent::PerformSetup(e) => <PerformSetup as placement::IPlacement>::initiator(&**e),
            _ => panic!("Unable to cast object of type 'Treachery.Shared.{}' to type 'Treachery.Shared.IPlacement'.", self.event_type().name()),
        }
    }

    /// `((IPlacement)this).ForceLocations`; InvalidCastException for events that do not implement `IPlacement`.
    pub fn i_placement_force_locations(&self, game: &Game) -> NetDictionary<Location, Battalion> {
        match self {
            GameEvent::AmbassadorActivated(e) => <AmbassadorActivated as placement::IPlacement>::force_locations(&**e, game),
            GameEvent::Caravan(e) => <Caravan as placement::IPlacement>::force_locations(&**e, game),
            GameEvent::DiscoveryEntered(e) => <DiscoveryEntered as placement::IPlacement>::force_locations(&**e, game),
            GameEvent::FaceDanced(e) => <FaceDanced as placement::IPlacement>::force_locations(&**e, game),
            GameEvent::FremenRidesMonster(e) => <FremenRidesMonster as placement::IPlacement>::force_locations(&**e, game),
            GameEvent::Move(e) => <Move as placement::IPlacement>::force_locations(&**e, game),
            GameEvent::PerformFremenSetup(e) => <PerformFremenSetup as placement::IPlacement>::force_locations(&**e, game),
            GameEvent::PerformSetup(e) => <PerformSetup as placement::IPlacement>::force_locations(&**e, game),
            _ => panic!("Unable to cast object of type 'Treachery.Shared.{}' to type 'Treachery.Shared.IPlacement'.", self.event_type().name()),
        }
    }

    /// `((IPlacement)this).To`; InvalidCastException for events that do not implement `IPlacement`.
    pub fn i_placement_to(&self, game: &Game) -> Option<Location> {
        match self {
            GameEvent::AmbassadorActivated(e) => <AmbassadorActivated as placement::IPlacement>::to(&**e, game),
            GameEvent::Caravan(e) => <Caravan as placement::IPlacement>::to(&**e, game),
            GameEvent::DiscoveryEntered(e) => <DiscoveryEntered as placement::IPlacement>::to(&**e, game),
            GameEvent::FaceDanced(e) => <FaceDanced as placement::IPlacement>::to(&**e, game),
            GameEvent::FremenRidesMonster(e) => <FremenRidesMonster as placement::IPlacement>::to(&**e, game),
            GameEvent::Move(e) => <Move as placement::IPlacement>::to(&**e, game),
            GameEvent::PerformFremenSetup(e) => <PerformFremenSetup as placement::IPlacement>::to(&**e, game),
            GameEvent::PerformSetup(e) => <PerformSetup as placement::IPlacement>::to(&**e, game),
            _ => panic!("Unable to cast object of type 'Treachery.Shared.{}' to type 'Treachery.Shared.IPlacement'.", self.event_type().name()),
        }
    }

    /// `this is IPlacement`
    pub fn is_i_placement(&self) -> bool {
        matches!(self, GameEvent::AmbassadorActivated(_) | GameEvent::Caravan(_) | GameEvent::DiscoveryEntered(_) | GameEvent::FaceDanced(_) | GameEvent::FremenRidesMonster(_) | GameEvent::Move(_) | GameEvent::PerformFremenSetup(_) | GameEvent::PerformSetup(_))
    }

}
