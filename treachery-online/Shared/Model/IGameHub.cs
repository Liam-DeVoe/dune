using System.Threading.Tasks;

namespace Treachery.Shared;

public interface IGameHub
{
    //Authentication
    
    Task<Result<LoginInfo>> RequestCreateUser(string userName, string hashedPassword, string email, string playerName);
    Task<Result<LoginInfo>> RequestLogin(int version, string userName, string hashedPassword);
    Task<VoidResult> RequestPasswordReset(string email);
    Task<Result<LoginInfo>> RequestSetPassword(string userName, string passwordResetToken, string newHashedPassword);
    Task<Result<LoginInfo>> GetLoginInfo(string userToken);
    Task<Result<LoginInfo>> RequestUpdateUserInfo(string userToken, string hashedPassword, string playerName, string email);
    Task<Result<ServerStatus>> RequestSetUserStatus(string userToken, UserStatus status);

    //Game Management
    
    Task<Result<GameInitInfo>> RequestCreateGame(string name, string userToken, string hashedPassword, string stateData, string skinData);
    Task<VoidResult> RequestUpdateSettings(string userToken, string gameId, GameSettings settings);
    Task<Result<ServerStatus>> RequestCloseGame(string userToken, string gameId);
    Task<Result<GameInitInfo>> RequestJoinGame(string userToken, string gameId, string hashedPassword, int seat);
    Task<Result<GameInitInfo>> RequestObserveGame(string userToken, string gameId, string hashedPassword);
    Task<Result<GameInitInfo>> RequestReconnectGame(string userToken, string gameId);
    Task<VoidResult> RequestSetOrUnsetHost(string userToken, string gameId, int userId);
    Task<VoidResult> RequestOpenOrCloseSeat(string userToken, string gameId, int seat);
    Task<Result<ServerStatus>> RequestLeaveGame(string userToken, string gameId);
    Task<VoidResult> RequestKick(string userToken, string gameId, int userId);
    Task<VoidResult> RequestLoadGame(string userToken, string hashedPassword, string state, string skin);
    Task<VoidResult> RequestSetSkin(string userToken, string gameId, string skin);
    Task<VoidResult> RequestUndo(string userToken, string gameId, int untilEventNr);
    Task<VoidResult> RequestRestoreRecentlyUndone(string userToken, string gameId);
    Task<VoidResult> RequestDismissRecentlyUndone(string userToken, string gameId);
    Task<Result<GameInitInfo>> RequestGameState(string userToken, string gameId);
    Task<VoidResult> RequestSetBotSpeed(string userToken, string gameId, int speed);
    Task<Result<ServerStatus>> RequestHeartbeat(string userToken, GameListScope scope);
    Task<VoidResult> RequestAssignSeats(string userToken, string gameId, Dictionary<int, int> assignment);
    Task<Result<ServerStatus>> RequestScheduleGame(string userToken,  
        DateTimeOffset dateTime, Ruleset? ruleset, int numberOfPlayers, int maximumTurns, 
        List<Faction> allowedFactionsInPlay, bool asyncPlay);
    Task<Result<ServerStatus>> RequestCancelGame(string userToken, string scheduledGameId);
    Task<Result<ServerStatus>> RequestSubscribeGame(string userToken, string scheduledGameId, SubscriptionType subscription);
    
    //Game Events
    Task<VoidResult> SetTimer(string userToken, string gameId, int value);
    Task<VoidResult> RequestPhaseDone(string userToken, string gameId);
    Task<VoidResult> RequestPhaseHold(string userToken, string gameId);
    Task<VoidResult> RequestPhaseRelease(string userToken, string gameId);
    Task<VoidResult> RequestPhaseContinue(string userToken, string gameId, int eventNumber);
    Task<VoidResult> RequestEstablishPlayers(string userToken, string gameId, EstablishPlayers e);
    Task<VoidResult> RequestEndPhase(string userToken, string gameId, EndPhase e);
    Task<VoidResult> RequestDonated(string userToken, string gameId, Donated e);
    Task<VoidResult> RequestResourcesTransferred(string userToken, string gameId, ResourcesTransferred e);
    Task<VoidResult> RequestFactionSelected(string userToken, string gameId, FactionSelected e);
    Task<VoidResult> RequestFactionTradeOffered(string userToken, string gameId, FactionTradeOffered e);
    Task<VoidResult> RequestPerformSetup(string userToken, string gameId, PerformSetup e);
    Task<VoidResult> RequestCardsDetermined(string userToken, string gameId, CardsDetermined e);
    Task<VoidResult> RequestPerformFremenSetup(string userToken, string gameId, PerformFremenSetup e);
    Task<VoidResult> RequestBeneGesseritPrediction(string userToken, string gameId, BeneGesseritPrediction e);
    Task<VoidResult> RequestCharityClaimed(string userToken, string gameId, CharityClaimed e);
    Task<VoidResult> RequestPerformBeneGesseritPlacement(string userToken, string gameId, PerformBeneGesseritPlacement e);
    Task<VoidResult> RequestTraitorsSelected(string userToken, string gameId, TraitorsSelected e);
    Task<VoidResult> RequestStormSpellPlayed(string userToken, string gameId, StormSpellPlayed e);
    Task<VoidResult> RequestTestingStationUsed(string userToken, string gameId, TestingStationUsed e);
    Task<VoidResult> RequestTakeLosses(string userToken, string gameId, TakeLosses e);
    Task<VoidResult> RequestMetheorPlayed(string userToken, string gameId, MetheorPlayed e);
    Task<VoidResult> RequestFremenSentMonster(string userToken, string gameId, FremenSentMonster e);
    Task<VoidResult> RequestFremenRidesMonster(string userToken, string gameId, FremenRidesMonster e);
    Task<VoidResult> RequestAllianceOffered(string userToken, string gameId, AllianceOffered e);
    Task<VoidResult> RequestAllianceBroken(string userToken, string gameId, AllianceBroken e);
    Task<VoidResult> RequestAlliancePassed(string userToken, string gameId, AlliancePassed e);
    Task<VoidResult> RequestBid(string userToken, string gameId, Bid e);
    Task<VoidResult> RequestRevival(string userToken, string gameId, Revival e);
    Task<VoidResult> RequestBeneGesseritBattleAnnouncement(string userToken, string gameId, BeneGesseritBattleAnnouncement e);
    Task<VoidResult> RequestShipment(string userToken, string gameId, Shipment e);
    Task<VoidResult> RequestGuildDelay(string userToken, string gameId, GuildDelay e);
    Task<VoidResult> RequestBeneGesseritAccompanies(string userToken, string gameId, BeneGesseritAccompanies e);
    Task<VoidResult> RequestBeneGesseritFlip(string userToken, string gameId, BeneGesseritFlip e);
    Task<VoidResult> RequestMove(string userToken, string gameId, Move e);
    Task<VoidResult> RequestCaravan(string userToken, string gameId, Caravan e);
    Task<VoidResult> RequestBattleInitiated(string userToken, string gameId, BattleInitiated e);
    Task<VoidResult> RequestBattle(string userToken, string gameId, Battle e);
    Task<VoidResult> RequestBattleRevision(string userToken, string gameId, BattleRevision e);
    Task<VoidResult> RequestTreacheryCalled(string userToken, string gameId, TreacheryCalled e);
    Task<VoidResult> RequestBattleConcluded(string userToken, string gameId, BattleConcluded e);
    Task<VoidResult> RequestClairvoyancePlayed(string userToken, string gameId, ClairVoyancePlayed e);
    Task<VoidResult> RequestClairvoyanceAnswered(string userToken, string gameId, ClairVoyanceAnswered e);
    Task<VoidResult> RequestRaiseDeadPlayed(string userToken, string gameId, RaiseDeadPlayed e);
    Task<VoidResult> RequestKarma(string userToken, string gameId, Karma e);
    Task<VoidResult> RequestKarmaFreeRevival(string userToken, string gameId, KarmaFreeRevival e);
    Task<VoidResult> RequestKarmaShipmentPrevention(string userToken, string gameId, KarmaShipmentPrevention e);
    Task<VoidResult> RequestKarmaRevivalPrevention(string userToken, string gameId, KarmaRevivalPrevention e);
    Task<VoidResult> RequestKarmaHandSwapInitiated(string userToken, string gameId, KarmaHandSwapInitiated e);
    Task<VoidResult> RequestKarmaHandSwap(string userToken, string gameId, KarmaHandSwap e);
    Task<VoidResult> RequestKarmaMonster(string userToken, string gameId, KarmaMonster e);
    Task<VoidResult> RequestKarmaRicheseBuy(string userToken, string gameId, KarmaRicheseBuy e);
    Task<VoidResult> RequestAllyPermission(string userToken, string gameId, AllyPermission e);
    Task<VoidResult> RequestMulliganPerformed(string userToken, string gameId, MulliganPerformed e);
    Task<VoidResult> RequestFaceDancerRevealed(string userToken, string gameId, FaceDancerRevealed e);
    Task<VoidResult> RequestFaceDanced(string userToken, string gameId, FaceDanced e);
    Task<VoidResult> RequestFaceDancerReplaced(string userToken, string gameId, FaceDancerReplaced e);
    Task<VoidResult> RequestSetIncreasedRevivalLimits(string userToken, string gameId, SetIncreasedRevivalLimits e);
    Task<VoidResult> RequestSetShipmentPermission(string userToken, string gameId, SetShipmentPermission e);
    Task<VoidResult> RequestRequestTleilaxuRevival(string userToken, string gameId, RequestTleilaxuRevival e);
    Task<VoidResult> RequestAcceptOrCancelTleilaxuRevival(string userToken, string gameId, AcceptOrCancelTleilaxuRevival e);
    Task<VoidResult> RequestPerformHmsPlacement(string userToken, string gameId, PerformHmsPlacement e);
    Task<VoidResult> RequestPerformHmsMovement(string userToken, string gameId, PerformHmsMovement e);
    Task<VoidResult> RequestKarmaHmsMovement(string userToken, string gameId, KarmaHmsMovement e);
    Task<VoidResult> RequestAmalPlayed(string userToken, string gameId, AmalPlayed e);
    Task<VoidResult> RequestIxianRemovedCardFromAuction(string userToken, string gameId, IxianRemovedCardFromAuction e);
    Task<VoidResult> RequestIxianSelectedStartingCard(string userToken, string gameId, IxianSelectedStartingCard e);
    Task<VoidResult> RequestIxianSwappedCardOnBid(string userToken, string gameId, IxianSwappedCardOnBid e);
    Task<VoidResult> RequestHarvesterPlayed(string userToken, string gameId, HarvesterPlayed e);
    Task<VoidResult> RequestPoisonToothCancelled(string userToken, string gameId, PoisonToothCancelled e);
    Task<VoidResult> RequestReplacedCardWon(string userToken, string gameId, ReplacedCardWon e);
    Task<VoidResult> RequestThumperPlayed(string userToken, string gameId, ThumperPlayed e);
    Task<VoidResult> RequestVoice(string userToken, string gameId, Voice e);
    Task<VoidResult> RequestPrescience(string userToken, string gameId, Prescience e);
    Task<VoidResult> RequestKarmaPrescience(string userToken, string gameId, KarmaPrescience e);
    Task<VoidResult> RequestEmperorBidSupport(string userToken, string gameId, EmperorBidSupport e);
    Task<VoidResult> RequestDealOffered(string userToken, string gameId, DealOffered e);
    Task<VoidResult> RequestDealAccepted(string userToken, string gameId, DealAccepted e);
    Task<VoidResult> RequestDealRejected(string userToken, string gameId, DealRejected e);
    Task<VoidResult> RequestDiscoveryEntered(string userToken, string gameId, DiscoveryEntered e);
    Task<VoidResult> RequestStormDialled(string userToken, string gameId, StormDialled e);
    Task<VoidResult> RequestHideSecrets(string userToken, string gameId, HideSecrets e);
    Task<VoidResult> RequestPlayerReplaced(string userToken, string gameId, PlayerReplaced e);
    Task<VoidResult> RequestChoamDiscarded(string userToken, string gameId, ChoamDiscarded e);
    Task<VoidResult> RequestEmperorDiscarded(string userToken, string gameId, EmperorDiscarded e);
    Task<VoidResult> RequestChoamEconomics(string userToken, string gameId, ChoamEconomics e);
    Task<VoidResult> RequestCardTraded(string userToken, string gameId, CardTraded e);
    Task<VoidResult> RequestKarmaChoamDiscard(string userToken, string gameId, KarmaChoamDiscard e);
    Task<VoidResult> RequestAuditCancelled(string userToken, string gameId, AuditCancelled e);
    Task<VoidResult> RequestAudited(string userToken, string gameId, Audited e);
    Task<VoidResult> RequestChoamMovePrevention(string userToken, string gameId, ChoamMovePrevention e);
    Task<VoidResult> RequestChoamKarmaPrevention(string userToken, string gameId, ChoamKarmaPrevention e);
    Task<VoidResult> RequestChoamExtraMove(string userToken, string gameId, ChoamExtraMove e);
    Task<VoidResult> RequestChoamFreeRevivalPrevention(string userToken, string gameId, ChoamFreeRevivalPrevention e);
    Task<VoidResult> RequestChoamRemoveForce(string userToken, string gameId, ChoamRemoveForce e);
    Task<VoidResult> RequestRicheseAnnouncesBlackMarket(string userToken, string gameId, RicheseAnnouncesBlackMarket e);
    Task<VoidResult> RequestBlackMarketBid(string userToken, string gameId, BlackMarketBid e);
    Task<VoidResult> RequestRicheseAnnouncesAuction(string userToken, string gameId, RicheseAnnouncesAuction e);
    Task<VoidResult> RequestRicheseSpecifiesAuction(string userToken, string gameId, RicheseSpecifiesAuction e);
    Task<VoidResult> RequestRicheseKeepsUnsoldCard(string userToken, string gameId, RicheseKeepsUnsoldCard e);
    Task<VoidResult> RequestRicheseRevealedNoField(string userToken, string gameId, RicheseRevealedNoField e);
    Task<VoidResult> RequestRicheseGaveCard(string userToken, string gameId, RicheseGaveCard e);
    Task<VoidResult> RequestCardGiven(string userToken, string gameId, CardGiven e);
    Task<VoidResult> RequestRockWasMelted(string userToken, string gameId, RockWasMelted e);
    Task<VoidResult> RequestResidualPlayed(string userToken, string gameId, ResidualPlayed e);
    Task<VoidResult> RequestFlightUsed(string userToken, string gameId, FlightUsed e);
    Task<VoidResult> RequestFlightDiscoveryUsed(string userToken, string gameId, FlightDiscoveryUsed e);
    Task<VoidResult> RequestDistransUsed(string userToken, string gameId, DistransUsed e);
    Task<VoidResult> RequestDiscardedTaken(string userToken, string gameId, DiscardedTaken e);
    Task<VoidResult> RequestDiscardedSearchedAnnounced(string userToken, string gameId, DiscardedSearchedAnnounced e);
    Task<VoidResult> RequestDiscardedSearched(string userToken, string gameId, DiscardedSearched e);
    Task<VoidResult> RequestJuicePlayed(string userToken, string gameId, JuicePlayed e);
    Task<VoidResult> RequestPortableAntidoteUsed(string userToken, string gameId, PortableAntidoteUsed e);
    Task<VoidResult> RequestBureaucracy(string userToken, string gameId, Bureaucracy e);
    Task<VoidResult> RequestDiplomacy(string userToken, string gameId, Diplomacy e);
    Task<VoidResult> RequestSkillAssigned(string userToken, string gameId, SkillAssigned e);
    Task<VoidResult> RequestSwitchedSkilledLeader(string userToken, string gameId, SwitchedSkilledLeader e);
    Task<VoidResult> RequestThought(string userToken, string gameId, Thought e);
    Task<VoidResult> RequestThoughtAnswered(string userToken, string gameId, ThoughtAnswered e);
    Task<VoidResult> RequestHmsAdvantageChosen(string userToken, string gameId, HMSAdvantageChosen e);
    Task<VoidResult> RequestRetreat(string userToken, string gameId, Retreat e);
    Task<VoidResult> RequestPlanetology(string userToken, string gameId, Planetology e);
    Task<VoidResult> RequestCaptured(string userToken, string gameId, Captured e);
    Task<VoidResult> RequestNexusCardDrawn(string userToken, string gameId, NexusCardDrawn e);
    Task<VoidResult> RequestTerrorPlanted(string userToken, string gameId, TerrorPlanted e);
    Task<VoidResult> RequestTerrorRevealed(string userToken, string gameId, TerrorRevealed e);
    Task<VoidResult> RequestDiscoveryRevealed(string userToken, string gameId, DiscoveryRevealed e);
    Task<VoidResult> RequestAmbassadorPlaced(string userToken, string gameId, AmbassadorPlaced e);
    Task<VoidResult> RequestAmbassadorActivated(string userToken, string gameId, AmbassadorActivated e);
    Task<VoidResult> RequestExtortionPrevented(string userToken, string gameId, ExtortionPrevented e);
    Task<VoidResult> RequestDiscarded(string userToken, string gameId, Discarded e);
    Task<VoidResult> RequestAllianceByTerror(string userToken, string gameId, AllianceByTerror e);
    Task<VoidResult> RequestNexusVoted(string userToken, string gameId, NexusVoted e);
    Task<VoidResult> RequestAllianceByAmbassador(string userToken, string gameId, AllianceByAmbassador e);
    Task<VoidResult> RequestLoserConcluded(string userToken, string gameId, LoserConcluded e);
    Task<VoidResult> RequestPerformMoritaniSetup(string userToken, string gameId, PerformMoritaniSetup e);
    Task<VoidResult> RequestDivideResources(string userToken, string gameId, DivideResources e);
    Task<VoidResult> RequestDivideResourcesAccepted(string userToken, string gameId, DivideResourcesAccepted e);
    Task<VoidResult> RequestBattleClaimed(string userToken, string gameId, BattleClaimed e);
    Task<VoidResult> RequestKarmaEcazDial(string userToken, string gameId, KarmaEcazDial e);
    Task<VoidResult> RequestTraitorDiscarded(string userToken, string gameId, TraitorDiscarded e);
    Task<VoidResult> RequestNexusPlayed(string userToken, string gameId, NexusPlayed e);
    Task<VoidResult> RequestResourcesAudited(string userToken, string gameId, ResourcesAudited e);
    Task<VoidResult> RequestRecruitsPlayed(string userToken, string gameId, RecruitsPlayed e);
    
    //Chat
    Task<VoidResult> SendChatMessage(string userToken, string gameId, GameChatMessage e);
    Task<VoidResult> SendGlobalChatMessage(string userToken, GlobalChatMessage message);

    //Other
    Task<Result<ServerInfo>> Connect();
    Task<VoidResult> RequestNudgeBots(string userToken, string gameId);
    
    //Admin
    Task<Result<string>> AdminUpdateMaintenance(string userToken, DateTimeOffset maintenanceDate);
    Task<Result<string>> AdminPersistState(string userToken);
    Task<Result<string>> AdminRestoreState(string userToken);
    Task<Result<string>> AdminCloseGame(string userToken, string gameId);
    Task<Result<string>> AdminDownloadGame(string userToken, string gameId);
    Task<Result<string>> AdminCancelGame(string userToken, string scheduledGameId);
    Task<Result<string>> DeleteUser(string userToken, int userId);
    Task<Result<AdminInfo>> GetAdminInfo(string userToken);
}