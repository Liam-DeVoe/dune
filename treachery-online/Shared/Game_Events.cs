/*
 * Copyright (C) 2020-2025 Ronald Ossendrijver (admin@treachery.online)
 * This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License
 * as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version. This
 * program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details. You should have
 * received a copy of the GNU General Public License along with this program. If not, see <http://www.gnu.org/licenses/>.
 */

namespace Treachery.Shared;

public partial class Game
{
    public List<Type> GetApplicableEvents(Player? player, bool isHost)
    {
        List<Type> result = [];

        if (CurrentPhase == Phase.SelectingFactions && (player is null || player.Faction is Faction.None)) 
            result.Add(typeof(FactionSelected));
        else if (player != null && player.Faction != Faction.None) 
            AddPlayerActions(player, isHost, result);

        if (isHost) AddHostActions(result);

        return [..result];
    }

    private void AddHostActions(List<Type> result)
    {
        switch (CurrentPhase)
        {
            case Phase.AwaitingPlayers:
                result.Add(typeof(EstablishPlayers));
                break;
            case Phase.CustomizingDecks:
                result.Add(typeof(CardsDetermined));
                break;
            case Phase.PerformCustomSetup:
                result.Add(typeof(PerformSetup));
                break;
            case Phase.HmsPlacement:
                if (!IsPlaying(Faction.Ixian) && Applicable(Rule.HmSwithoutIxian)) result.Add(typeof(PerformHmsPlacement));
                break;
            case Phase.SelectingFactions:
            case Phase.TradingFactions:
            case Phase.BeginningOfStorm:
            case Phase.MetheorAndStormSpell:
            case Phase.StormReport:
            case Phase.Thumper:
            case Phase.HarvesterA:
            case Phase.HarvesterB:
            case Phase.AllianceA when Version < 188:
            case Phase.AllianceB when Version < 188:
            case Phase.BlowReport:
            case Phase.BeginningOfCharity:
            case Phase.ClaimingCharity:
            case Phase.CharityReport:
            case Phase.BeginningOfBidding:
            case Phase.ReplacingCardJustWon when Version > 150:
            case Phase.WaitingForNextBiddingRound:
            case Phase.BiddingReport:
            case Phase.BeginningOfResurrection:
            case Phase.Resurrection:
            case Phase.ResurrectionReport:
            case Phase.BeginningOfShipAndMove:
            case Phase.ShipmentAndMoveConcluded:
            case Phase.BeginningOfBattle:
            case Phase.BattleReport:
            case Phase.BeginningOfCollection:
            case Phase.CollectionReport:
            case Phase.Extortion:
            case Phase.Contemplate:
            case Phase.TurnConcluded:
            case Phase.CancellingTraitor:
                result.Add(typeof(EndPhase));
                break;
        }

        if (Version < 170 && CurrentMainPhase >= MainPhase.Setup) result.Add(typeof(PlayerReplaced));
    }

    private void AddPlayerActions(Player player, bool isHost, List<Type> result)
    {
        var faction = player.Faction;

        switch (CurrentPhase)
        {
            case Phase.Discarding:
                if (FactionsThatMustDiscard.Contains(faction)) result.Add(typeof(Discarded));
                break;
            case Phase.DiscardingTraitor:
                if (faction == FactionThatMustDiscardTraitor) result.Add(typeof(TraitorDiscarded));
                break;
            case Phase.AllianceByTerror:
                if (faction == AllianceByTerrorOfferedTo) result.Add(typeof(AllianceByTerror));
                break;
            case Phase.AllianceByAmbassador:
                if (faction == AllianceByAmbassadorOfferedTo) result.Add(typeof(AllianceByAmbassador));
                break;
            case Phase.Bureaucracy:
                if (player == PlayerSkilledAs(LeaderSkill.Bureaucrat)) result.Add(typeof(Bureaucracy));
                break;
            case Phase.AssigningInitialSkills:
            case Phase.AssigningSkill:
                if (player.SkillsToChooseFrom.Any()) result.Add(typeof(SkillAssigned));
                break;
            case Phase.SelectingFactions:
                if (player.Faction == Faction.None) result.Add(typeof(FactionSelected));
                break;
            case Phase.DiallingStorm:
                if (HasBattleWheel.Contains(player.Faction) && !HasActedOrPassed.Contains(player.Faction)) result.Add(typeof(StormDialled));
                break;
            case Phase.TradingFactions:
                if (Players.Count > 1) result.Add(typeof(FactionTradeOffered));
                break;
            case Phase.PerformCustomSetup:
                break;
            case Phase.BeneGesseritPredicting:
                if (faction == Faction.BeneGesserit) result.Add(typeof(BeneGesseritPrediction));
                break;
            case Phase.FremenSettingUp:
                if (faction == Faction.Fremen) result.Add(typeof(PerformFremenSetup));
                break;
            case Phase.BeneGesseritSettingUp:
                if (faction == Faction.BeneGesserit) result.Add(typeof(PerformBeneGesseritPlacement));
                break;
            case Phase.MoritaniSettingUp:
                if (faction == Faction.Moritani) result.Add(typeof(PerformMoritaniSetup));
                break;
            case Phase.HarkonnenMulligan:
                if (faction == Faction.Harkonnen) result.Add(typeof(MulliganPerformed));
                break;
            case Phase.SelectingTraitors:
                if (faction != Faction.Harkonnen && faction != Faction.Tleilaxu && !HasActedOrPassed.Contains(faction)) result.Add(typeof(TraitorsSelected));
                break;
            case Phase.IxianSelectingCard:
                if (faction == Faction.Ixian) result.Add(typeof(IxianSelectedStartingCard));
                break;
            case Phase.HmsPlacement:
                if (faction == Faction.Ixian) result.Add(typeof(PerformHmsPlacement));
                break;
            case Phase.BeginningOfStorm:
                if (DiscoveryEntered.ValidTargets(this, player).Any()) result.Add(typeof(DiscoveryEntered));
                break;
            case Phase.HmsMovement:
                if (faction == Faction.Ixian) result.Add(typeof(PerformHmsMovement));
                break;
            case Phase.MetheorAndStormSpell:
                if (player.Has(TreacheryCardType.StormSpell) && CurrentTurn > 1) result.Add(typeof(StormSpellPlayed));
                if (MetheorPlayed.MayPlayMetheor(this, player)) result.Add(typeof(MetheorPlayed));
                if (TestingStationUsed.CanBePlayed(this, player)) result.Add(typeof(TestingStationUsed));
                break;
            case Phase.Thumper:
                if (player.Has(TreacheryCardType.Thumper)) result.Add(typeof(ThumperPlayed));
                if (Version < 103 && player.Has(TreacheryCardType.Amal)) result.Add(typeof(AmalPlayed));
                break;
            case Phase.VoteAllianceA:
            case Phase.VoteAllianceB:
                if (NexusVotes.All(v => v.Initiator != faction)) result.Add(typeof(NexusVoted));
                break;
            case Phase.HarvesterA:
            case Phase.HarvesterB:
                if (player.Has(TreacheryCardType.Harvester)) result.Add(typeof(HarvesterPlayed));
                break;
            case Phase.StormLosses:
                if (faction == TakeLosses.LossesToTake(this).Faction) result.Add(typeof(TakeLosses));
                break;
            case Phase.FremenSendingMonsterA:
            case Phase.FremenSendingMonsterB:
                if (faction == Faction.Fremen) result.Add(typeof(FremenSentMonster));
                break;
            case Phase.FremenRidingMonsterA:
            case Phase.FremenRidingMonsterB:
                if (faction == Faction.Fremen) result.Add(typeof(FremenRidesMonster));
                break;
            case Phase.AllianceA:
            case Phase.AllianceB:
                if (!HasActedOrPassed.Contains(faction))
                {
                    if (player.Ally == Faction.None && Players.Count > 1) result.Add(typeof(AllianceOffered));
                    if (player.Ally != Faction.None) result.Add(typeof(AllianceBroken));
                    if (Version >= 188) result.Add(typeof(AlliancePassed));
                }
                break;
            case Phase.NexusCards:
                if (NexusCardDrawn.Applicable(this, player)) result.Add(typeof(NexusCardDrawn));
                break;
            case Phase.BlowReport:
                break;
            case Phase.ClaimingCharity:
                if (!isHost && faction == Faction.Atreides) result.Add(typeof(EndPhase));
                if (CharityClaimed.CanBePlayed(this, player)) result.Add(typeof(CharityClaimed));
                if (Version < 103 && player.Has(TreacheryCardType.Amal) && (Version <= 82 || HasActedOrPassed.Count == 0)) result.Add(typeof(AmalPlayed));
                break;
            case Phase.BlackMarketAnnouncement:
                if (faction == Faction.Richese) result.Add(typeof(RicheseAnnouncesBlackMarket));
                break;
            case Phase.BlackMarketBidding:
                if (BlackMarketBid.MayBePlayed(this, player)) result.Add(typeof(BlackMarketBid));
                if (faction == Faction.Emperor && Applicable(Rule.EmperorSupportingNonAllyBids)) result.Add(typeof(EmperorBidSupport));
                break;
            case Phase.RicheseAnnouncingAuction:
                if (faction == Faction.Richese) result.Add(typeof(RicheseAnnouncesAuction));
                break;
            case Phase.RicheseSpecifyingAuction:
            {
                if (RicheseSpecifiesAuction.MaySpecifyCard(this, player) || RicheseSpecifiesAuction.MaySpecifyTypeOfAuction(this, player)) result.Add(typeof(RicheseSpecifiesAuction));
                break;
            }
            case Phase.Bidding:
                if (Bid.MayBePlayed(this, player)) result.Add(typeof(Bid));
                if (Version < 103 && player.Has(TreacheryCardType.Amal) && CardNumber == 1 && !Bids.Any()) result.Add(typeof(AmalPlayed));
                if (faction == Faction.Emperor && Applicable(Rule.EmperorSupportingNonAllyBids)) result.Add(typeof(EmperorBidSupport));
                break;
            case Phase.RicheseKeepingUnsoldCard:
                if (faction == Faction.Richese) result.Add(typeof(RicheseKeepsUnsoldCard));
                break;
            case Phase.IxianRemovingCardFromBid:
            {
                var occupierOfIxianHomeworld = OccupierOf(World.Ixian);
                if ((occupierOfIxianHomeworld == null && faction == Faction.Ixian) || occupierOfIxianHomeworld == player) result.Add(typeof(IxianRemovedCardFromAuction));
                if (Version < 103 && player.Has(TreacheryCardType.Amal) && CardNumber == 1 && !Bids.Any()) result.Add(typeof(AmalPlayed));
                break;
            }
            case Phase.IxianSwappingCard:
            {
                if (faction == Faction.Ixian) result.Add(typeof(IxianSwappedCardOnBid));
                if (Version < 103 && player.Has(TreacheryCardType.Amal) && CardNumber == 1 && !Bids.Any()) result.Add(typeof(AmalPlayed));
                break;
            }
            case Phase.ReplacingCardJustWon:
                if (faction == FactionThatMayReplaceBoughtCard) result.Add(typeof(ReplacedCardWon));
                break;
            case Phase.WaitingForNextBiddingRound:
                if (!isHost && faction == Faction.Atreides) result.Add(typeof(EndPhase));
                break;
            case Phase.BiddingReport:
                if (faction == Faction.Tleilaxu && Players.Count > 1 && (Version < 113 || !Prevented(FactionAdvantage.TleilaxuIncreasingRevivalLimits))) result.Add(typeof(SetIncreasedRevivalLimits));
                if (faction == Faction.Emperor && player.HasHighThreshold() && player.Resources >= 2) result.Add(typeof(EmperorDiscarded));
                break;

            case Phase.BeginningOfResurrection:
                if (faction == Faction.Tleilaxu && Players.Count > 1 && (Version < 113 || !Prevented(FactionAdvantage.TleilaxuIncreasingRevivalLimits))) result.Add(typeof(SetIncreasedRevivalLimits));
                if (RecruitsPlayed.IsApplicable(this, player)) result.Add(typeof(RecruitsPlayed));
                break;

            case Phase.Resurrection:
                if (IsPlaying(Faction.Tleilaxu) && faction != Faction.Tleilaxu &&
                    KilledHeroes(player).Any() &&
                    !Revival.NormallyRevivableHeroes(this, player).Any()) result.Add(typeof(RequestTleilaxuRevival));

                if (faction == Faction.Tleilaxu && (CurrentRevivalRequests.Count != 0 || EarlyRevivalsOffers.Count != 0)) result.Add(typeof(AcceptOrCancelTleilaxuRevival));
                if (HasSomethingToRevive(player) && !PreventedFromReviving(faction)) result.Add(typeof(Revival));
                if (faction == Faction.Tleilaxu && Players.Count > 1 && (Version < 113 || !Prevented(FactionAdvantage.TleilaxuIncreasingRevivalLimits))) result.Add(typeof(SetIncreasedRevivalLimits));
                if (Version < 103 && player.Has(TreacheryCardType.Amal) && (Version <= 82 || HasActedOrPassed.Count == 0)) result.Add(typeof(AmalPlayed));
                break;

            case Phase.ResurrectionReport:
                if (faction == Faction.Ecaz && AmbassadorPlaced.IsApplicable(this, player)) result.Add(typeof(AmbassadorPlaced));
                break;

            case Phase.GuildShip:
                if (faction == Faction.Guild)
                {
                    if (GuildMayDelay) result.Add(typeof(GuildDelay));
                    result.Add(typeof(Shipment));
                    if (player.TreacheryCards.Any(c => c.Type == TreacheryCardType.Caravan) && PlacementEvent.ValidMovementSources(this, player).Any()) result.Add(typeof(Caravan));
                }
                if (Version < 103 && player.Has(TreacheryCardType.Amal) && BeginningOfShipmentAndMovePhase) result.Add(typeof(AmalPlayed));
                break;
            case Phase.BeneGesseritAccompaniesGuildShip:
            case Phase.BeneGesseritAccompaniesNonGuildShip:
                if (faction == Faction.BeneGesserit) result.Add(typeof(BeneGesseritAccompanies));
                break;
            case Phase.NonGuildShip:
                if (player == ShipmentAndMoveSequence?.CurrentPlayer)
                {
                    result.Add(typeof(Shipment));
                    if (player.TreacheryCards.Any(c => c.Type == TreacheryCardType.Caravan)) result.Add(typeof(Caravan));
                }
                if (Version < 103 && player.Has(TreacheryCardType.Amal) && BeginningOfShipmentAndMovePhase) result.Add(typeof(AmalPlayed));
                break;
            case Phase.GuildMove:
                if (faction == Faction.Guild)
                {
                    if (player.TreacheryCards.Any(c => c.Type == TreacheryCardType.Caravan)) result.Add(typeof(Caravan));
                    result.Add(typeof(Move));
                    if (FlightUsed.IsAvailable(player)) result.Add(typeof(FlightUsed));
                    if (FlightDiscoveryUsed.IsAvailable(this, player)) result.Add(typeof(FlightDiscoveryUsed));
                    if (Planetology.CanBePlayed(this, player)) result.Add(typeof(Planetology));
                }
                break;
            case Phase.NonGuildMove:
                if (player == ShipmentAndMoveSequence?.CurrentPlayer)
                {
                    if (player.TreacheryCards.Any(c => c.Type == TreacheryCardType.Caravan)) result.Add(typeof(Caravan));
                    result.Add(typeof(Move));
                    if (FlightUsed.IsAvailable(player)) result.Add(typeof(FlightUsed));
                    if (FlightDiscoveryUsed.IsAvailable(this, player)) result.Add(typeof(FlightDiscoveryUsed));
                    if (Planetology.CanBePlayed(this, player)) result.Add(typeof(Planetology));
                }
                break;

            case Phase.BeneGesseritIntrudedByGuildShip:
            case Phase.BeneGesseritIntrudedByNonGuildShip:
            case Phase.BeneGesseritIntrudedByGuildMove:
            case Phase.BeneGesseritIntrudedByNonGuildMove:
            case Phase.BeneGesseritIntrudedByFremenRidingMonsterA:
            case Phase.BeneGesseritIntrudedByFremenRidingMonsterB:
            case Phase.BeneGesseritIntrudedByCaravan:
            case Phase.BeneGesseritIntrudedByRevival:
                if (faction == Faction.BeneGesserit) result.Add(typeof(BeneGesseritFlip));
                break;

            case Phase.TerrorTriggeredByGuildShip:
            case Phase.TerrorTriggeredByBeneGesseritAccompaniesGuildShip:
            case Phase.TerrorTriggeredByNonGuildShip:
            case Phase.TerrorTriggeredByBeneGesseritAccompaniesNonGuildShip:
            case Phase.TerrorTriggeredByGuildMove:
            case Phase.TerrorTriggeredByNonGuildMove:
            case Phase.TerrorTriggeredByFremenRidingMonsterA:
            case Phase.TerrorTriggeredByFremenRidingMonsterB:
            case Phase.TerrorTriggeredByCaravan:
            case Phase.TerrorTriggeredByRevival:
                if (faction == Faction.Moritani) result.Add(typeof(TerrorRevealed));
                break;

            case Phase.AmbassadorTriggeredByGuildShip:
            case Phase.AmbassadorTriggeredByBeneGesseritAccompaniesGuildShip:
            case Phase.AmbassadorTriggeredByNonGuildShip:
            case Phase.AmbassadorTriggeredByBeneGesseritAccompaniesNonGuildShip:
            case Phase.AmbassadorTriggeredByGuildMove:
            case Phase.AmbassadorTriggeredByNonGuildMove:
            case Phase.AmbassadorTriggeredByFremenRidingMonsterA:
            case Phase.AmbassadorTriggeredByFremenRidingMonsterB:
            case Phase.AmbassadorTriggeredByCaravan:
            case Phase.AmbassadorTriggeredByRevival:
                if (faction == Faction.Ecaz || (player.Ally == Faction.Ecaz && EcazSharesAmbassadors)) result.Add(typeof(AmbassadorActivated));
                break;

            case Phase.ShipmentAndMoveConcluded:
                if (Version < 103 && player.Has(TreacheryCardType.Amal)) result.Add(typeof(AmalPlayed));
                break;

            case Phase.BattlePhase:
            {
                if (CurrentBattle == null && player == NextPlayerToBattle) result.Add(typeof(BattleInitiated));

                if (CurrentBattle != null && SwitchedSkilledLeader.CanBePlayed(this, player)) result.Add(typeof(SwitchedSkilledLeader));

                if (HMSAdvantageChosen.CanBePlayed(this, player)) result.Add(typeof(HMSAdvantageChosen));

                if (Voice.MayUseVoice(this, player)) result.Add(typeof(Voice));

                if (Thought.MayBeUsed(this, player)) result.Add(typeof(Thought));

                if (Prescience.MayUsePrescience(this, player)) result.Add(typeof(Prescience));

                if (CurrentBattle != null && faction == CurrentBattle.Aggressor && AggressorPlan == null)
                    result.Add(typeof(Battle));
                else if (CurrentBattle != null && faction == CurrentBattle.Defender && DefenderPlan == null) result.Add(typeof(Battle));

                if (CurrentBattle != null && faction == CurrentBattle.Aggressor && AggressorPlan != null)
                    result.Add(typeof(BattleRevision));
                else if (CurrentBattle != null && faction == CurrentBattle.Defender && DefenderPlan != null) result.Add(typeof(BattleRevision));

                if (CurrentBattle != null && ResidualPlayed.MayPlay(this, player)) result.Add(typeof(ResidualPlayed));

                if (Version < 103 && player.Has(TreacheryCardType.Amal) && NrOfBattlesFought == 0) result.Add(typeof(AmalPlayed));
            }
                break;

            case Phase.ClaimingBattle:
                if (HasLowThreshold(Faction.Ecaz))
                {
                    if (BattleAboutToStart?.OpponentOf(Faction.Ecaz)?.Is(faction) is true) result.Add(typeof(BattleClaimed));
                }
                else
                {
                    if (faction == Faction.Ecaz) result.Add(typeof(BattleClaimed));
                }
                break;

            case Phase.CallTraitorOrPass:

                if (CurrentBattle != null && AggressorPlan != null && DefenderPlan != null &&
                    ((AggressorTraitorAction == null && faction == CurrentBattle.Aggressor) ||
                     (AggressorTraitorAction == null 
                      && faction == Faction.Harkonnen 
                      && GetPlayer(AggressorPlan.Initiator)?.Ally == Faction.Harkonnen 
                      && DefenderPlan.Hero != null 
                      && player.Traitors.Contains(DefenderPlan.Hero)) ||
                     (DefenderTraitorAction == null 
                      && faction == CurrentBattle.Defender) ||
                     (DefenderTraitorAction == null 
                      && faction == Faction.Harkonnen 
                      && GetPlayer(DefenderPlan.Initiator)?.Ally == Faction.Harkonnen 
                      && AggressorPlan.Hero != null 
                      && player.Traitors.Contains(AggressorPlan.Hero))))
                    result.Add(typeof(TreacheryCalled));

                if (AggressorPlan != null && faction == AggressorPlan.Initiator && AggressorPlan.Weapon is { Type: TreacheryCardType.PoisonTooth } && !PoisonToothCancelled) result.Add(typeof(PoisonToothCancelled));

                if (DefenderPlan != null && faction == DefenderPlan.Initiator && DefenderPlan.Weapon is { Type: TreacheryCardType.PoisonTooth } && !PoisonToothCancelled) result.Add(typeof(PoisonToothCancelled));

                if (PortableAntidoteUsed.CanBePlayed(this, player)) result.Add(typeof(PortableAntidoteUsed));

                if (Diplomacy.CanBePlayed(this, player)) result.Add(typeof(Diplomacy));

                break;

            case Phase.Retreating:
                if (faction == BattleLoser) result.Add(typeof(Retreat));
                break;

            case Phase.MeltingRock:
                if (RockWasMelted.CanBePlayed(this, player)) result.Add(typeof(RockWasMelted));
                break;

            case Phase.CaptureDecision:
                if (faction == Faction.Harkonnen) result.Add(typeof(Captured));
                break;

            case Phase.BattleConclusion:
                if (faction == BattleWinner && !BattleWasConcludedByWinner) result.Add(typeof(BattleConcluded));
                if (LoserConcluded.IsApplicable(this, player)) result.Add(typeof(LoserConcluded));
                break;

            case Phase.AvoidingAudit:
                if (player == Auditee) result.Add(typeof(AuditCancelled));
                break;

            case Phase.Auditing:
                if (faction == Faction.Choam) result.Add(typeof(Audited));
                break;

            case Phase.RevealingFacedancer:
                if (faction == Faction.Tleilaxu) result.Add(typeof(FaceDancerRevealed));
                break;

            case Phase.Facedancing:
                if (faction == Faction.Tleilaxu) result.Add(typeof(FaceDanced));
                break;

            case Phase.BattleReport:
                if (Version < 103 && player.Has(TreacheryCardType.Amal) && NextPlayerToBattle == null) result.Add(typeof(AmalPlayed));
                break;

            case Phase.DividingCollectedResources:
                if (DivideResources.IsApplicable(this, player)) result.Add(typeof(DivideResources));
                break;

            case Phase.AcceptingResourceDivision:
                if (DivideResourcesAccepted.IsApplicable(this, player)) result.Add(typeof(DivideResourcesAccepted));
                break;

            case Phase.ReplacingFaceDancer:
                if (faction == Faction.Tleilaxu) result.Add(typeof(FaceDancerReplaced));
                break;

            case Phase.Extortion:
                if (ExtortionPrevented.CanBePlayed(this, player)) result.Add(typeof(ExtortionPrevented));
                break;
            
            case Phase.BeginningOfCollection or Phase.CollectionReport:
                if (faction == Faction.Moritani && TerrorPlanted.IsApplicable(this, player)) result.Add(typeof(TerrorPlanted));
                break;
            
            case Phase.Contemplate:
                if (Version < 103 && player.Has(TreacheryCardType.Amal)) result.Add(typeof(AmalPlayed));
                if (faction == Faction.Choam && !Prevented(FactionAdvantage.ChoamEconomics) && EconomicsStatus == ChoamEconomicsStatus.None) result.Add(typeof(ChoamEconomics));
                if (faction == Faction.Moritani && TerrorPlanted.IsApplicable(this, player)) result.Add(typeof(TerrorPlanted));
                break;

            case Phase.PerformingKarmaHandSwap:
                if (faction == Faction.Harkonnen && !KarmaPrevented(faction)) result.Add(typeof(KarmaHandSwap));
                break;

            case Phase.TradingCards:
                if (faction == CurrentCardTradeOffer?.Target) result.Add(typeof(CardTraded));
                break;

            case Phase.Clairvoyance:
                if (faction == LatestClairvoyance?.Target) result.Add(typeof(ClairVoyanceAnswered));
                break;

            case Phase.Thought:
                if (player == CurrentBattle?.OpponentOf(CurrentThought!.Initiator)) result.Add(typeof(ThoughtAnswered));
                break;

            case Phase.SearchingDiscarded:
                if (DiscardedSearched.CanBePlayed(player)) result.Add(typeof(DiscardedSearched));
                break;
        }
        
        if (CurrentPhase > Phase.TradingFactions) result.Add(typeof(AutomationConfigured));

        if (CurrentMainPhase > MainPhase.Setup && faction == Faction.Choam && player.HasLowThreshold() && !IsOccupied(World.Choam) && ResourcesAudited.ValidFactions(this, player).Any()) result.Add(typeof(ResourcesAudited));

        if (CurrentMainPhase == MainPhase.ShipmentAndMove && SetShipmentPermission.IsApplicable(this, player)) result.Add(typeof(SetShipmentPermission));

        if (CurrentMainPhase == MainPhase.Collection && DiscoveryRevealed.Applicable(this, player)) result.Add(typeof(DiscoveryRevealed));

        if (Version <= 123) result.Add(typeof(HideSecrets));

        if (NexusPlayed.IsApplicable(this, player)) result.Add(typeof(NexusPlayed));

        if (player.Ally != Faction.None) result.Add(typeof(AllyPermission));

        var isAfterSetup = Version < 107 ? CurrentPhase > Phase.TradingFactions : CurrentMainPhase > MainPhase.Setup;
        var hasFinalizedBattlePlanWaitingToBeResolved = (CurrentPhase == Phase.BattlePhase || CurrentPhase == Phase.MeltingRock || CurrentPhase == Phase.CallTraitorOrPass) && CurrentBattle != null && CurrentBattle.PlanOf(player) != null;

        if (JuicePlayed.CanBePlayedBy(this, player)) result.Add(typeof(JuicePlayed));

        if (CardGiven.IsApplicable(this, player)) result.Add(typeof(CardGiven));

        if (faction == Faction.Ecaz &&
            CurrentPhase == Phase.CallTraitorOrPass &&
            CurrentBattle != null &&
            CurrentBattle.IsAggressorOrDefender(player) &&
            !KarmaPrevented(faction) &&
            !player.SpecialKarmaPowerUsed &&
            player.HasKarma(this) &&
            Applicable(Rule.AdvancedKarama))
        {
            var planOfEcaz = CurrentBattle.PlanOf(Faction.Ecaz);
            if (planOfEcaz is { Defense: null, Weapon: null }) result.Add(typeof(KarmaEcazDial));
        }

        if (isAfterSetup && (Version < 100 || !hasFinalizedBattlePlanWaitingToBeResolved) && !CurrentPhaseCannotBeInterrupted)
        {
            if (CurrentMainPhase < MainPhase.Battle && player.NoFieldIsActive) result.Add(typeof(RicheseRevealedNoField));

            if (faction == Faction.Choam && !Prevented(FactionAdvantage.ChoamDiscarding) && ConsiderAsEndOfPhase && ChoamDiscarded.ValidCards(player).Any()) result.Add(typeof(ChoamDiscarded));

            if (!result.Contains(typeof(AmalPlayed)) && player.Has(TreacheryCardType.Amal) && ConsiderAsStartOfPhase) result.Add(typeof(AmalPlayed));

            if (DiscardedTaken.CanBePlayed(this, player)) result.Add(typeof(DiscardedTaken));

            if (CurrentPhase != Phase.SearchingDiscarded && DiscardedSearchedAnnounced.CanBePlayed(this, player)) result.Add(typeof(DiscardedSearchedAnnounced));

            if (CurrentMainPhase == MainPhase.ShipmentAndMove && ChoamMovePrevention.CanBePlayedBy(this, player)) result.Add(typeof(ChoamMovePrevention));

            if (ChoamKarmaPrevention.CanBePlayedBy(this, player)) result.Add(typeof(ChoamKarmaPrevention));

            if (CurrentMainPhase == MainPhase.ShipmentAndMove && ChoamExtraMove.CanBePlayedBy(this, player)) result.Add(typeof(ChoamExtraMove));

            if (CurrentMainPhase == MainPhase.Resurrection && ChoamFreeRevivalPrevention.CanBePlayedBy(this, player)) result.Add(typeof(ChoamFreeRevivalPrevention));

            if (CurrentMainPhase == MainPhase.Contemplate && ChoamRemoveForce.CanBePlayedBy(this, player)) result.Add(typeof(ChoamRemoveForce));

            if (
                ((faction == Faction.Choam && player.HasAlly) || player.Ally == Faction.Choam) &&
                ConsiderAsEndOfPhase &&
                player.AlliedPlayer!.TreacheryCards.Count > 0 &&
                player.TreacheryCards.Count > 0 &&
                LastTurnCardWasTraded < CurrentTurn)
                result.Add(typeof(CardTraded));

            if (player.Has(TreacheryCardType.RaiseDead)) result.Add(typeof(RaiseDeadPlayed));

            if (player.Has(TreacheryCardType.Clairvoyance) || (player.Occupies(Map.Shrine) && player.HasKarma(this))) result.Add(typeof(ClairVoyancePlayed));

            if (!KarmaPrevented(faction) && player.HasKarma(this)) result.Add(typeof(Karma));

            if (Players.Count > 1 &&
                faction == Faction.Harkonnen &&
                !KarmaPrevented(faction) &&
                !player.SpecialKarmaPowerUsed &&
                player.HasKarma(this) &&
                CurrentMainPhase == MainPhase.Bidding &&
                Applicable(Rule.AdvancedKarama))
                result.Add(typeof(KarmaHandSwapInitiated));

            if (faction == Faction.Guild &&
                !KarmaPrevented(faction) &&
                !player.SpecialKarmaPowerUsed &&
                player.HasKarma(this) &&
                CurrentMainPhase == MainPhase.ShipmentAndMove &&
                Applicable(Rule.AdvancedKarama))
                result.Add(typeof(KarmaShipmentPrevention));

            if (faction == Faction.Tleilaxu &&
                !KarmaPrevented(faction) &&
                !player.SpecialKarmaPowerUsed &&
                player.HasKarma(this) &&
                CurrentMainPhase == MainPhase.Resurrection &&
                Applicable(Rule.AdvancedKarama))
                result.Add(typeof(KarmaRevivalPrevention));


            if (faction == Faction.Choam &&
                !KarmaPrevented(faction) &&
                !player.SpecialKarmaPowerUsed &&
                player.HasKarma(this) &&
                Applicable(Rule.AdvancedKarama))
                result.Add(typeof(KarmaChoamDiscard));

            if (faction == Faction.Richese &&
                !KarmaPrevented(faction) &&
                !player.SpecialKarmaPowerUsed &&
                player.HasKarma(this) &&
                Applicable(Rule.AdvancedKarama) &&
                (Version <= 150 || !HasBidToPay(player)))
                result.Add(typeof(KarmaRicheseBuy));

            if (faction == Faction.Emperor &&
                !KarmaPrevented(faction) &&
                !player.SpecialKarmaPowerUsed &&
                player.HasKarma(this) &&
                CurrentMainPhase == MainPhase.Resurrection &&
                Applicable(Rule.AdvancedKarama))
                result.Add(typeof(KarmaFreeRevival));

            if (faction == Faction.Ixian &&
                ((!KarmaPrevented(faction) && !player.SpecialKarmaPowerUsed && player.HasKarma(this)) || KarmaHmsMovesLeft == 1) &&
                CurrentMainPhase == MainPhase.ShipmentAndMove &&
                player == ShipmentAndMoveSequence!.CurrentPlayer &&
                player.AnyForcesIn(Map.HiddenMobileStronghold) > 0 &&
                Applicable(Rule.AdvancedKarama))
                result.Add(typeof(KarmaHmsMovement));

            if (faction == Faction.Fremen && CurrentMainPhase == MainPhase.Blow && CurrentTurn > 1 &&
                !KarmaPrevented(faction) &&
                !player.SpecialKarmaPowerUsed &&
                player.HasKarma(this) &&
                Applicable(Rule.AdvancedKarama))
                result.Add(typeof(KarmaMonster));

            if (faction == Faction.Atreides && CurrentMainPhase == MainPhase.Battle &&
                CurrentBattle != null &&
                CurrentBattle.IsInvolved(player) &&
                !KarmaPrevented(faction) &&
                !player.SpecialKarmaPowerUsed &&
                player.HasKarma(this) &&
                Applicable(Rule.AdvancedKarama))
                result.Add(typeof(KarmaPrescience));

            if (faction == Faction.BeneGesserit && CurrentPhase > Phase.AllianceB &&
                ((Version <= 150 && CurrentPhase < Phase.NonGuildShip) || CurrentMainPhase < MainPhase.ShipmentAndMove || (player.Initiated(CurrentBeneGesseritCunning) && CurrentMainPhase == MainPhase.ShipmentAndMove)) &&
                BeneGesseritBattleAnnouncement.ValidTerritories(this, player).Any())
                result.Add(typeof(BeneGesseritBattleAnnouncement));

            if (ResourcesTransferred.CanBePlayed(this, player)) result.Add(typeof(ResourcesTransferred));

            if (Players.Count > 1 &&
                Donated.ValidTargets(this, player).Any() &&
                (isHost || player.Resources > 0) &&
                Donated.MayDonate(this, player) &&
                (AggressorPlan == null || faction != AggressorPlan.Initiator) &&
                (DefenderPlan == null || faction != DefenderPlan.Initiator))
                result.Add(typeof(Donated));

            if (faction == Faction.Richese &&
                CurrentPhase != Phase.BlackMarketBidding &&
                CurrentPhase != Phase.Bidding &&
                !hasFinalizedBattlePlanWaitingToBeResolved &&
                player.HasAlly &&
                RicheseGaveCard.ValidCards(player).Any() &&
                player.AlliedPlayer.HasRoomForCards)
                result.Add(typeof(RicheseGaveCard));

            if (CurrentPhase != Phase.BlackMarketBidding &&
                (CurrentPhase != Phase.Bidding || CurrentBid == null) &&
                !hasFinalizedBattlePlanWaitingToBeResolved &&
                DistransUsed.CanBePlayed(this, player))
                result.Add(typeof(DistransUsed));

            if (Version >= 110)
            {
                result.Add(typeof(DealOffered));
                result.Add(typeof(DealAccepted));
                result.Add(typeof(DealRejected));
            }
        }

        if (Version < 110)
        {
            if (player.Ally != Faction.None) result.Add(typeof(AllyPermission));

            result.Add(typeof(DealOffered));
            result.Add(typeof(DealAccepted));
            result.Add(typeof(DealRejected));
        }

    }

    internal bool CurrentPhaseCannotBeInterrupted =>
        CurrentMainPhase == MainPhase.Ended ||
        CurrentPhase == Phase.Discarding ||
        CurrentPhase == Phase.AssigningSkill ||
        CurrentPhase == Phase.Clairvoyance ||
        CurrentPhase == Phase.Thought ||
        CurrentPhase == Phase.TradingCards ||
        CurrentPhase == Phase.Bureaucracy ||
        CurrentPhase == Phase.SearchingDiscarded ||
        CurrentPhase == Phase.PerformingKarmaHandSwap ||
        CurrentPhase == Phase.ReplacingCardJustWon ||
        CurrentPhase == Phase.DividingCollectedResources ||
        CurrentPhase == Phase.AcceptingResourceDivision;

    private bool ConsiderAsEndOfPhase =>
        CurrentMoment == MainPhaseMoment.End ||
        CurrentMoment == MainPhaseMoment.Start && CurrentMainPhase is MainPhase.Bidding or MainPhase.ShipmentAndMove;

    private bool ConsiderAsStartOfPhase =>
        CurrentMoment == MainPhaseMoment.Start ||
        (CurrentMainPhase == MainPhase.Blow && CurrentMoment == MainPhaseMoment.End) ||
        (Version < 109 && CurrentMoment == MainPhaseMoment.End);
}