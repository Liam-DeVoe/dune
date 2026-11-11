/*
 * Copyright (C) 2020-2025 Ronald Ossendrijver (admin@treachery.online)
 * This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License
 * as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version. This
 * program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details. You should have
 * received a copy of the GNU General Public License along with this program. If not, see <http://www.gnu.org/licenses/>.
 */

using System;
using System.ComponentModel.DataAnnotations;

namespace Treachery.Shared;

public class EndPhase : GameEvent
{
    #region Construction

    public EndPhase(Game game, Faction initiator) : base(game, initiator)
    {
    }

    public EndPhase()
    {
    }

    #endregion Construction

    #region Validation

    public override Message? Validate()
    {
        return null;
    }

    #endregion Construction

    #region Execution

    protected override void ExecuteConcreteEvent()
    {
        switch (Game.CurrentPhase)
        {
            case Phase.SelectingFactions:
                Game.AssignFactionsAndEnterFactionTrade();
                break;

            case Phase.TradingFactions:
                EstablishDecks();
                break;

            case Phase.BeginningOfStorm:
                Game.MoveHmsBeforeDiallingStorm();
                break;

            case Phase.MetheorAndStormSpell:
                EnterNormalStormPhase();
                break;

            case Phase.StormReport:
                Game.EnterSpiceBlowPhase();
                break;

            case Phase.Thumper:
                Game.EnterBlowA();
                break;

            case Phase.HarvesterA:
            case Phase.HarvesterB:
                Game.MoveToNextPhaseAfterResourceBlow();
                break;

            case Phase.AllianceA:
            case Phase.AllianceB:
                Game.EndNexus();
                break;

            case Phase.BlowReport:
                if (Game.Applicable(Rule.HasCharityPhase))
                    EnterCharityPhase();
                else
                    Game.EnterBiddingPhase();
                break;

            case Phase.BeginningOfCharity:
                StartClaimingCharity();
                break;

            case Phase.ClaimingCharity:
                EndCharityPhase();
                break;

            case Phase.CharityReport:
                Game.EnterBiddingPhase();
                break;

            case Phase.BeginningOfBidding:
                Game.StartBiddingPhase();
                break;

            case Phase.ReplacingCardJustWon:
                if (Game.CardJustWon == Game.CardSoldOnBlackMarket)
                    Game.EnterRicheseBidding();
                else
                    Game.DetermineNextStepAfterCardWasSold();
                break;

            case Phase.WaitingForNextBiddingRound:
                Game.PutNextCardOnAuction();
                break;

            case Phase.BiddingReport:
                EnterRevivalPhase();
                break;

            case Phase.BeginningOfResurrection:
                ContinueToRevivals();
                break;

            case Phase.Resurrection:
                EndResurrectionPhase();
                break;

            case Phase.ResurrectionReport:
                EnterShipmentAndMovePhase();
                break;

            case Phase.BeginningOfShipAndMove:
                Game.StartShipAndMoveSequence();
                break;

            case Phase.ShipmentAndMoveConcluded:
                EnterBattlePhase();
                break;

            case Phase.BeginningOfBattle:
                Game.Enter(Phase.BattlePhase);
                break;

            case Phase.CancellingTraitor:
                Game.Enter(Phase.CallTraitorOrPass);
                Game.HandleRevealedBattlePlans();
                break;

            case Phase.BattleReport:
                ResetBattle();
                Game.Enter(Game.NextPlayerToBattle != null, Phase.BattlePhase, EnterSpiceCollectionPhase);
                break;

            case Phase.BeginningOfCollection:
                Game.StartCollection();
                break;

            case Phase.CollectionReport:
                Game.EnterContemplatePhase();
                break;

            case Phase.Extortion:
                Game.EndContemplatePause();
                break;

            case Phase.Contemplate:
                Game.ContinueContemplatePhase();
                break;

            case Phase.TurnConcluded:
                if (Game.Version < 108) Game.AddBribesToPlayerResources();
                Game.EnterStormPhase();
                Game.FlipBeneGesseritAdvisorsWhenAlone();
                break;
        }
    }

    private void ContinueToRevivals()
    {
        Game.Enter(Phase.Resurrection);

        foreach (var p in Game.Players.Where(p => Game.IsAutomated(AutomationRuleType.RevivalAutoClaimFreeRevival, p)))
        {
            Game.ClaimFreeRevival(p);
        } 
    }

    private void EstablishDecks()
    {
        if (Game.IsPlaying(Faction.Richese)) Game.RicheseCache = TreacheryCardManager.GetRicheseCards();

        Game.Enter(Game.Applicable(Rule.CustomDecks) && Game.Version >= 134, Phase.CustomizingDecks, Game.EnterSetupPhase);
    }

    private void EnterNormalStormPhase()
    {
        Log(Message.Express("The storm moves ", Game.NextStormMoves, " sectors").WithLog(new StormMoved(Game.SectorInStorm, Game.NextStormMoves)));
        Game.MoveStormAndDetermineNext(Game.NextStormMoves);
        Game.EndStormPhase();
    }

    private void EnterCharityPhase()
    {
        Game.MainPhaseStart(MainPhase.Charity);
        Game.HasActedOrPassed.Clear();
        Game.Monsters.Clear();
        Game.ResourceTechTokenIncome = false;
        Game.Allow(FactionAdvantage.FremenControlsMonster);
        Game.Allow(FactionAdvantage.FremenProtectedFromMonster);

        if (Game.Version < 122)
        {
            StartClaimingCharity();
        }
        else
        {
            Game.Enter(Phase.BeginningOfCharity);
        }
    }

    private void StartClaimingCharity()
    {
        if (!Game.Prevented(FactionAdvantage.ChoamControllingCharity))
        {
            var choam = GetPlayer(Faction.Choam);
            if (choam != null)
            {
                var toCollect = Game.Players.Count * 2 * Game.CurrentCharityMultiplier;
                choam.Resources += toCollect;
                Log(Faction.Choam, " collect ", Payment.Of(toCollect));
            }
        }
        else
        {
            Game.LogPreventionByKarma(FactionAdvantage.ChoamControllingCharity);
        }

        var bene_gesserit = GetPlayer(Faction.BeneGesserit);
        if (bene_gesserit != null && Game.Applicable(Rule.BeneGesseritAutoCharity))
        {
            if (Game.Version < 179 || !Game.CharityIsCancelled)
            {
                if (!Game.Prevented(FactionAdvantage.BeneGesseritCharity))
                {
                    Game.HasActedOrPassed.Add(Faction.BeneGesserit);
                    Game.GiveCharity(bene_gesserit, 2 * Game.CurrentCharityMultiplier);
                    Game.Stone(Milestone.CharityClaimed);
                }
                else
                {
                    Game.LogPreventionByKarma(FactionAdvantage.BeneGesseritCharity);
                    if (!Game.Applicable(Rule.FullPhaseKarma)) Game.Allow(FactionAdvantage.BeneGesseritCharity);
                }
            }
        }
        
        foreach (var p in Game.Players.Where(p => CharityClaimed.CanBePlayed(Game, p) && Game.IsAutomated(AutomationRuleType.CharityAutoClaim, p)))
        {
            Game.ClaimCharity(p);
        } 

        Game.MainPhaseMiddle();
        Game.Enter(Phase.ClaimingCharity);
    }

    private void EndCharityPhase()
    {
        if (Game.ResourceTechTokenIncome) Game.ReceiveTechIncome(TechToken.Resources);

        if (Game.Version < 122)
        {
            Game.EnterBiddingPhase();
        }
        else
        {
            if (Game.Version >= 132) Game.MainPhaseEnd();
            Game.Enter(Phase.CharityReport);
        }
    }

    private void EnterRevivalPhase()
    {
        Game.CardJustWon = null;
        Game.MainPhaseStart(MainPhase.Resurrection);
        Game.Allow(FactionAdvantage.HarkonnenFreeCard);
        Game.Allow(FactionAdvantage.EmperorReceiveBid);
        Game.Allow(FactionAdvantage.IxianAllyDiscardingCard);
        Game.RevivalTechTokenIncome = false;
        Game.AmbassadorsPlacedThisTurn = 0;
        Game.FreeRevivalsThisTurn.Clear();
        Game.TotalRevivalsThisTurn.Clear();
        Game.LeaderRevivalsThisTurn.Clear();
        //Game.HasActedOrPassed.Clear();
        Game.TleilaxuStartedRevivalWithLowThreshold = Game.HasLowThreshold(Faction.Tleilaxu);

        if (Game.Version < 122)
            Game.Enter(Phase.Resurrection);
        else
            Game.Enter(Phase.BeginningOfResurrection);
    }

    private void EndResurrectionPhase()
    {
        Game.Allow(FactionAdvantage.TleilaxuReviveGhola);
        if (Game.RevivalTechTokenIncome) Game.ReceiveTechIncome(TechToken.Graveyard);
        Game.CurrentKarmaRevivalPrevention = null;
        Game.CurrentFremenSecretAlly = null;
        Game.CurrentRecruitsPlayed = null;
        
        if (Game.Version < 122)
        {
            EnterShipmentAndMovePhase();
        }
        else
        {
            if (Game.Version >= 132) Game.MainPhaseEnd();
            Game.Enter(Phase.ResurrectionReport);
        }
    }

    private void EnterShipmentAndMovePhase()
    {
        Game.MainPhaseStart(MainPhase.ShipmentAndMove);
        Game.FactionsWithOrnithoptersAtStartOfMovement = Game.Players.Where(p => Game.OccupiesArrakeenOrCarthag(p)).Select(p => p.Faction).ToList();
        Game.RecentMoves.Clear();
        Game.BeginningOfShipmentAndMovePhase = true;
        Game.FactionsWithIncreasedRevivalLimits = [];
        Game.EarlyRevivalsOffers.Clear();

        Game.ShipsTechTokenIncome = false;
        Game.CurrentFreeRevivalPrevention = null;
        Game.Allow(FactionAdvantage.BeneGesseritAnnouncesBattle);
        Game.Allow(FactionAdvantage.EmperorLetAllyReviveExtraForces);
        Game.Allow(FactionAdvantage.TleilaxuReceiveRevive);
        Game.Allow(FactionAdvantage.ChoamRevival);

        Game.HasActedOrPassed.Clear();
        Game.LastShipmentOrMovement = null;

        Game.ShipmentAndMoveSequence = new PlayerSequence(Game);

        Game.Enter(Game.Version >= 107, Phase.BeginningOfShipAndMove, Game.StartShipAndMoveSequence);
    }

    private void EnterBattlePhase()
    {
        Game.MainPhaseStart(MainPhase.Battle);
        Game.NrOfBattlesFought = 0;
        Game.BattleSequence = new PlayerSequence(Game);
        Game.CurrentGuildCunning = null;
        Game.CurrentGuildSecretAlly = null;
        if (Game.KarmaHmsMovesLeft != 2) Game.KarmaHmsMovesLeft = 0;
        ResetBattle();
        Game.Enter(Game.NextPlayerToBattle == null, EnterSpiceCollectionPhase, Game.Version >= 107, Phase.BeginningOfBattle, Phase.BattlePhase);
    }

    private void ResetBattle()
    {
        Game.FacedancerWasCancelled = false;
        Game.CurrentBattle = null;
        Game.CurrentPrescience = null;
        Game.PrescienceSeenPlans.Clear();
        Game.CurrentThought = null;
        Game.CurrentVoice = null;
        Game.CurrentNexusPrescience = null;
        Game.CurrentBeneGesseritCunning = null;
        Game.CurrentEmperorCunning = null;
        Game.CurrentIxianCunning = null;
        Game.HarkonnenVictim = null;
        Game.AggressorPlan = null;
        Game.DefenderPlan = null;
        Game.PreviousAggressorPlan = null;
        Game.PreviousDefenderPlan = null;
        Game.AggressorTraitorAction = null;
        Game.DefenderTraitorAction = null;
        Game.PoisonToothCancelled = false;
        Game.IxianSpecialForceLossesToTake = 0;
        Game.BattleWinner = Faction.None;
        Game.BattleLoser = Faction.None;
        Game.HasActedOrPassed.Clear();
        Game.BattleTriggeredBureaucracy = null;
        Game.CardsToBeDiscardedByLoserAfterBattle.Clear();
        Game.BattleWasConcludedByWinner = false;
        Game.LoserMayTryToAssassinate = false;
        Game.BattleWinnerMayChooseToDiscard = true;
        Game.BattleAboutToStart = null;
        Game.CurrentEcazOrAllyFighter = Faction.None;
        Game.CurrentEcazBattleContribution = 0;
        Game.AtreidesKarma = false;
        Game.EcazKarmaBonus = 0;
    }

    private void EnterSpiceCollectionPhase()
    {
        Game.MainPhaseStart(MainPhase.Collection);
        Game.ResourcesCollectedByFremen = 0;
        Game.ResourcesCollectedByHarkonnenFromDesertOrHomeworld = 0;
        CallHeroesHome();
        Game.PendingDiscoveries = Game.DiscoveriesOnPlanet.Where(kvp => Game.AnyForcesIn(kvp.Key.Territory)).Select(kvp => kvp.Value.Token).ToList();

        if (Game.Version < 122)
            Game.StartCollection();
        else
            Game.Enter(Phase.BeginningOfCollection);
    }

    private void CallHeroesHome()
    {
        foreach (var ls in Game.LeaderState) ls.Value.CurrentTerritory = null;
    }

    public override Message GetMessage()
    {
        return Message.Express("Phase ended by ", Initiator);
    }

    #endregion Execution
}