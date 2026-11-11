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
    #region State

    public PlayerSequence? BidSequence { get; private set; }
    public Deck<TreacheryCard>? CardsOnAuction { get; private set; }
    public AuctionType CurrentAuctionType { get; private set; }
    public int CardNumber { get; private set; }
    public IBid? CurrentBid { get; internal set; }
    public Dictionary<Faction, IBid> Bids { get; } = new();
    public TreacheryCard? CardJustWon { get; internal set; }
    public IBid? WinningBid { get; internal set; }
    internal bool BiddingRoundWasStarted { get; private set; }
    internal bool RegularBiddingIsDone { get; set; }
    public TreacheryCard? CardThatMustBeKeptOrGivenToAlly { get; internal set; }
    internal int NumberOfCardsOnAuction { get; set; }
    internal TriggeredBureaucracy? BiddingTriggeredBureaucracy { get; set; }
    internal bool IxianSwappedCardOnBid { get; set; }

    public Faction FactionThatMayReplaceBoughtCard { get; internal set; }
    public bool ReplacingBoughtCardUsingNexus { get; internal set; }

    public int KarmaHandSwapNumberOfCards { get; internal set; }
    public Faction KarmaHandSwapTarget { get; internal set; }
    internal Phase KarmaHandSwapPausedPhase { get; set; }

    private AuctionType BlackMarketAuctionType { get; set; }
    public TreacheryCard? CardSoldOnBlackMarket { get; set; }
    internal bool RicheseAuctionShouldStillHappen { get; set; }
    public bool RicheseBiddingJustFinished { get; private set; }
    public bool RicheseOccupierSpecifiedCard { get; internal set; }

    #endregion

    #region BeginningOfBidding

    internal void EnterBiddingPhase()
    {
        MainPhaseStart(MainPhase.Bidding);
        Allow(FactionAdvantage.ChoamControllingCharity);
        Allow(FactionAdvantage.BeneGesseritCharity);
        CardsOnAuction = new Deck<TreacheryCard>(Random ?? throw new Exception("Random generator is not initialized"));
        IxianSwappedCardOnBid = false;
        CardSoldOnBlackMarket = null;
        BlackMarketAuctionType = AuctionType.None;
        RegularBiddingIsDone = false;
        BiddingRoundWasStarted = false;
        RicheseAuctionShouldStillHappen = false;
        CurrentAuctionType = AuctionType.None;
        BiddingTriggeredBureaucracy = null;
        BidSequence = new PlayerSequence(this);
        WinningBid = null;

        Enter(Version > 150, Phase.BeginningOfBidding, StartBiddingPhase);
    }

    internal void StartBiddingPhase()
    {
        var richese = GetPlayer(Faction.Richese);
        Enter(richese != null && Applicable(Rule.RicheseBlackMarket) && !Prevented(FactionAdvantage.RicheseBlackMarket) && richese.TreacheryCards.Count > 0,
            Phase.BlackMarketAnnouncement,
            EnterRicheseBidding);
    }

    internal void EnterRicheseBidding()
    {
        NumberOfCardsOnAuction = PlayersThatCanBid.Count();

        if (CardSoldOnBlackMarket != null) NumberOfCardsOnAuction--;

        //CardSoldOnBlackMarket = null;

        if (NumberOfCardsOnAuction == 0)
        {
            Log(Message.Express("Bidding is skipped because no faction is able to buy cards").WithLog(new BiddingSkipped()));
            EndBiddingPhase();
        }
        else
        {
            Enter(IsPlaying(Faction.Richese) && RicheseCache.Count > 0 && !Prevented(FactionAdvantage.RicheseAuction), Phase.RicheseAnnouncingAuction, StartRegularBidding);
        }
    }

    #endregion BeginningOfBidding

    #region Bidding

    internal void StartRegularBidding()
    {
        RegularBiddingIsDone = true;
        var numberOfCardsToDraw = NumberOfCardsOnAuction;

        if (Version <= 150)
            if (IsPlaying(Faction.Richese) && (Version < 150 || !RicheseAuctionShouldStillHappen || (RicheseAuctionShouldStillHappen && RicheseCache.Count > 0))) numberOfCardsToDraw--;

        if (numberOfCardsToDraw > 0 && IsPlaying(Faction.Ixian) && !Prevented(FactionAdvantage.IxianSelectingCardsOnAuction)) numberOfCardsToDraw++;

        Log(Message.Express(numberOfCardsToDraw, " cards were drawn for bidding").WithLog(new BiddingDrawn(numberOfCardsToDraw)));

        for (var i = 0; i < numberOfCardsToDraw; i++)
        {
            var card = DrawTreacheryCard();
            if (card != null) CardsOnAuction?.PutOnBottom(card);
        }

        StartBidSequenceAndAuctionType(AuctionType.Normal);

        Enter(IsPlaying(Faction.Ixian) && !Prevented(FactionAdvantage.IxianSelectingCardsOnAuction), Phase.IxianRemovingCardFromBid, StartBiddingRound);
    }

    internal void StartBiddingRound()
    {
        BiddingRoundWasStarted = true;
        SkipPlayersThatCantBid(BidSequence!);
        Enter(Phase.Bidding);
        CurrentBid = null;
        Bids.Clear();
        CardNumber = 1;
    }
    
    internal void ExecuteBid(Bid bid)
    {
        if (!bid.Passed)
        {
            ReturnKarmaCardUsedForBid();
            if (bid.UsingKarmaToRemoveBidLimit) bid.Player.TreacheryCards.Remove(bid.KarmaCard);
            CurrentBid = bid;
            Stone(Milestone.Bid);
        }

        BidSequence!.NextPlayer();
        SkipPlayersThatCantBid(BidSequence);
        Bids.Remove(bid.Initiator);
        Bids.Add(bid.Initiator, bid);
        CheckIfAtreidesOrAtreidesAllyPassedBid(bid);

        switch (CurrentAuctionType)
        {
            case AuctionType.Normal:

                bid.HandleNormalBid();
                break;

            case AuctionType.RicheseOnceAround:
            case AuctionType.RicheseSilent:

                bid.HandleRicheseBid();
                break;
        }
    }

    internal void CheckIfAtreidesOrAtreidesAllyPassedBid(IBid bid)
    {
        var atreides = GetPlayer(Faction.Atreides);
        if (atreides == null) return;

        if (bid.Initiator == atreides.Faction || bid.Initiator == atreides.Ally)
        {
            LatestBidByAtreidesOrAtreidesAllyWasPassed = bid.Passed;
        }
    }
    
    public bool LatestBidByAtreidesOrAtreidesAllyWasPassed { get; set; }

    internal void ReturnKarmaCardUsedForBid()
    {
        if (CurrentBid is { UsingKarmaToRemoveBidLimit: true }) CurrentBid.Player.TreacheryCards.Add(CurrentBid.KarmaCard);
    }

    // paidTo is the faction that received the spice, or Faction.None for the
    // bank. A card won for free (Secret Ally) carries no log record.
    internal void LogBid(Player initiator, int bidAmount, int bidAllyContributionAmount, int bidEmperorContributionAmount, MessagePart receiverIncome, Faction paidTo, bool free = false)
    {
        var bidTotalAmount = bidAmount + bidAllyContributionAmount + bidEmperorContributionAmount;
        var cardNumber = MessagePart.ExpressIf(CurrentAuctionType == AuctionType.Normal, CardNumber);

        var shares = new List<BidShare>();
        if (bidAllyContributionAmount > 0) shares.Add(new BidShare(initiator.Ally, bidAllyContributionAmount));
        if (bidEmperorContributionAmount > 0) shares.Add(new BidShare(Faction.Emperor, bidEmperorContributionAmount));

        LogBidding(free ? null : new CardSold(CardNumber, initiator.Faction, bidTotalAmount, paidTo, shares, null),
            initiator.Faction,
            " get card ",
            cardNumber,
            " for ",
            Payment.Of(bidTotalAmount),
            MessagePart.ExpressIf(
                bidAllyContributionAmount > 0 || bidEmperorContributionAmount > 0,
                "(", Payment.Of(bidAllyContributionAmount, initiator.Ally), MessagePart.ExpressIf(bidEmperorContributionAmount > 0, " and ", Payment.Of(bidEmperorContributionAmount, Faction.Emperor)), ") "),
            receiverIncome);
    }

    // Logs a message for everyone, with a Bidding log record in normal auctions.
    internal void LogBidding(object? record, params object?[] expression)
    {
        var message = Message.Express(expression);
        if (record != null && CurrentAuctionType == AuctionType.Normal) message.WithLog(record);
        CurrentReport.Express(message);
    }

    // Logs a message for one faction, with a Bidding log record in normal auctions.
    internal void LogBiddingTo(Faction faction, object record, params object?[] expression)
    {
        var message = Message.ExpressTo(faction, expression);
        if (CurrentAuctionType == AuctionType.Normal) message.WithLog(record);
        CurrentReport.Express(message);
    }

    internal TreacheryCard WinByHighestBid(Player winner, IBid bid, int bidAmount, int bidAllyContributionAmount, int bidEmperorContributionAmount, Faction paymentReceiver, Deck<TreacheryCard> toDrawFrom, bool usesEmperorSecretAlly)
    {
        var receiverIncomeMessage = MessagePart.Express();

        if (!usesEmperorSecretAlly)
        {
            var paidTo = PayForCard(winner, bid, bidAmount, bidAllyContributionAmount, bidEmperorContributionAmount, paymentReceiver, ref receiverIncomeMessage);
            LogBid(winner, bidAmount, bidAllyContributionAmount, bidEmperorContributionAmount, receiverIncomeMessage, paidTo);
        }
        else
        {
            PlayNexusCard(winner, "Secret Ally", "get this card for free");
            LogBid(winner, 0, 0, 0, receiverIncomeMessage, Faction.None, true);
        }

        Stone(Milestone.AuctionWon);
        var card = toDrawFrom.Draw();
        RegisterWonCardAsKnown(card);
        winner.TreacheryCards.Add(card);
        LogBiddingTo(winner.Faction, new CardWonShown(card), "You won: ", card);
        GivePlayerExtraCardIfApplicable(winner);
        return card;
    }

    internal void RegisterWonCardAsKnown(TreacheryCard card)
    {
        foreach (var p in Players.Where(HasBiddingPrescience)) RegisterKnown(p, card);
    }

    internal void SkipPlayersThatCantBid(PlayerSequence sequence)
    {
        for (var i = 0; i < Players.Count; i++)
        {
            if (sequence.CurrentPlayer.HasRoomForCards) break;
            sequence.NextPlayer();
        }
    }

    // Returns the faction that received the spice, or Faction.None when it went to the bank.
    internal Faction PayForCard(Player initiator, IBid bid, int bidAmount, int bidAllyContributionAmount, int bidEmperorContributionAmount, Faction paymentReceiver, ref MessagePart message)
    {
        var paidTo = Faction.None;
        initiator.Resources -= bidAmount;

        var receiverProfit = 0;

        if (bidAllyContributionAmount > 0 && initiator.HasAlly)
        {
            initiator.AlliedPlayer.Resources -= bidAllyContributionAmount;
            DecreasePermittedUseOfAllySpice(initiator.Faction, bidAllyContributionAmount);
        }

        if (bidEmperorContributionAmount > 0) GetPlayer(Faction.Emperor)!.Resources -= bidEmperorContributionAmount;

        var receiver = GetPlayer(paymentReceiver);
        var receiverAndAllyAreRichese = initiator.Ally == Faction.Richese && paymentReceiver == Faction.Richese;
        var totalAmount = bidAmount + (Version >= 139 && receiverAndAllyAreRichese ? 0 : bidAllyContributionAmount);

        var emperor = GetPlayer(Faction.Emperor);
        var totalAmountReceivedByEmperorInsteadOfRichese =
            Version >= 165 && receiverAndAllyAreRichese && emperor != null && !Prevented(FactionAdvantage.EmperorReceiveBid) ? bidAllyContributionAmount : 0;

        var emperorReceivedInsteadMessage = MessagePart.Express();
        if (emperor != null && totalAmountReceivedByEmperorInsteadOfRichese > 0)
        {
            ModifyIncomeBecauseOfLowThresholdOrOccupation(receiver!, ref totalAmountReceivedByEmperorInsteadOfRichese);
            emperor.Resources += totalAmountReceivedByEmperorInsteadOfRichese;
            emperorReceivedInsteadMessage = MessagePart.Express(" and ", Faction.Emperor, " get ", Payment.Of(totalAmountReceivedByEmperorInsteadOfRichese));

            if (totalAmountReceivedByEmperorInsteadOfRichese >= 5)
                //This should also take into account payments received by an occupier
                BiddingTriggeredBureaucracy = new TriggeredBureaucracy { PaymentFrom = Faction.Richese, PaymentTo = Faction.Emperor };

            SetRecentPayment(receiverProfit, initiator.Faction, receiver!.Faction, (GameEvent)bid);
        }
        
        if (receiver != null && initiator.Faction != paymentReceiver)
        {
            if (paymentReceiver == Faction.Emperor && Prevented(FactionAdvantage.EmperorReceiveBid))
            {
                message = MessagePart.Express(" → ", paymentReceiver, " do not receive ", Concept.Resource, " for this card");
                if (!Applicable(Rule.FullPhaseKarma)) Allow(FactionAdvantage.EmperorReceiveBid);
            }
            else
            {
                receiverProfit = totalAmount;
                ModifyIncomeBecauseOfLowThresholdOrOccupation(receiver, ref receiverProfit);
                receiver.Resources += receiverProfit;
                receiver.ResourcesAfterBidding += bidEmperorContributionAmount;
                paidTo = receiver.Faction;
                message = MessagePart.Express(" → ", receiver.Faction, " get ", Payment.Of(receiverProfit),
                    emperorReceivedInsteadMessage,
                    MessagePart.ExpressIf(bidEmperorContributionAmount > 0, " immediately and ", Payment.Of(bidEmperorContributionAmount), " at the end of the bidding phase"));

                if (receiverProfit >= 5)
                    //This should also take into account payments received by an occupier
                    BiddingTriggeredBureaucracy = new TriggeredBureaucracy { PaymentFrom = initiator.Faction, PaymentTo = paymentReceiver };

                SetRecentPayment(receiverProfit, initiator.Faction, receiver.Faction, (GameEvent)bid);
            }
        }
        else if (totalAmountReceivedByEmperorInsteadOfRichese > 0)
        {
            message = MessagePart.Express(" → ", Faction.Emperor, " get ", Payment.Of(totalAmountReceivedByEmperorInsteadOfRichese));
            paidTo = Faction.Emperor;
        }

        if (totalAmount - receiverProfit - totalAmountReceivedByEmperorInsteadOfRichese >= 4) ActivateBanker(initiator);

        return paidTo;
    }

    internal void GivePlayerExtraCardIfApplicable(Player winner)
    {
        if (winner.Is(Faction.Harkonnen))
        {
            var extraCardMustBeDecidedAbout = false;
            Player? receiver = null;

            var occupierOfHarkonnenHomeworld = OccupierOf(World.Harkonnen);
            if (occupierOfHarkonnenHomeworld != null)
            {
                if (occupierOfHarkonnenHomeworld.HasRoomForCards)
                {
                    receiver = occupierOfHarkonnenHomeworld;
                    extraCardMustBeDecidedAbout = occupierOfHarkonnenHomeworld.HasAlly && occupierOfHarkonnenHomeworld.AlliedPlayer.HasRoomForCards;

                }
                else if (occupierOfHarkonnenHomeworld.HasAlly && occupierOfHarkonnenHomeworld.AlliedPlayer.HasRoomForCards)
                {
                    receiver = occupierOfHarkonnenHomeworld.AlliedPlayer;
                }
            }
            else if (winner.HasRoomForCards)
            {
                if (!Prevented(FactionAdvantage.HarkonnenFreeCard))
                {
                    receiver = winner;
                }
                else
                {
                    LogPreventionByKarma(FactionAdvantage.HarkonnenFreeCard);
                    if (!Applicable(Rule.FullPhaseKarma)) Allow(FactionAdvantage.HarkonnenFreeCard);
                }
            }

            if (receiver != null)
            {
                var extraCard = DrawTreacheryCard();

                if (extraCard != null)
                {
                    receiver.TreacheryCards.Add(extraCard);
                    LogBidding(new ExtraCardReceived(receiver.Faction), receiver.Faction, " receive an extra treachery card");
                    LogBiddingTo(receiver.Faction, new ExtraCardShown(extraCard), "Your extra card is: ", extraCard);

                    if (extraCardMustBeDecidedAbout) CardThatMustBeKeptOrGivenToAlly = extraCard;
                }
            }
        }
    }

    internal TreacheryCard? DrawTreacheryCard()
    {
        if (TreacheryDeck?.IsEmpty is true)
        {
            Log(Message.Express("Shuffled ", TreacheryDiscardPile!.Items.Count, " cards from the treachery discard pile into a new deck").WithLog(new TreacheryReshuffled(TreacheryDiscardPile.Items.Count)));

            foreach (var i in TreacheryDiscardPile.Items)
            {
                TreacheryDeck.Items.Add(i);
                UnregisterKnown(i);
            }

            TreacheryDiscardPile.Clear();
            TreacheryDeck.Shuffle();
            Stone(Milestone.Shuffled);
        }

        return TreacheryDeck?.Items.Count > 0 
            ? TreacheryDeck.Draw() 
            : null;
    }

    internal void FinishBid(Player? winner, IBid? winningBid, TreacheryCard card, bool mightReplace)
    {
        CardJustWon = card;
        WinningBid = winningBid;
        CardSoldOnBlackMarket = null;
        CurrentBid = null;
        FactionThatMayReplaceBoughtCard = Faction.None;
        LatestBidByAtreidesOrAtreidesAllyWasPassed = false;
        
        Bids.Clear();

        if (!Applicable(Rule.FullPhaseKarma)) Allow(FactionAdvantage.AtreidesBiddingPrescience);

        var enterReplacingCardJustWon = mightReplace && Version > 150 &&  
                                        ((Version < 159 && Players.Any(p => p.Nexus != Faction.None)) || (Version >= 159 && winner.Nexus != Faction.None));

        if (mightReplace)
        {
            if (winner.Ally == Faction.Ixian && IxianAllowsReplacingCards && !(Version >= 164 && (CurrentAuctionType is AuctionType.RicheseSilent or AuctionType.RicheseOnceAround)))
            {
                if (!Prevented(FactionAdvantage.IxianAllyDiscardingCard))
                {
                    FactionThatMayReplaceBoughtCard = winner.Faction;
                    enterReplacingCardJustWon = true;
                }
                else
                {
                    if (!Applicable(Rule.FullPhaseKarma)) Allow(FactionAdvantage.IxianAllyDiscardingCard);

                    if (NexusAllowsReplacingBoughtCards(winner))
                    {
                        FactionThatMayReplaceBoughtCard = winner.Faction;
                        enterReplacingCardJustWon = true;
                        ReplacingBoughtCardUsingNexus = true;
                    }
                }
            }
            else if (NexusAllowsReplacingBoughtCards(winner))
            {
                FactionThatMayReplaceBoughtCard = winner.Faction;
                enterReplacingCardJustWon = true;
                ReplacingBoughtCardUsingNexus = true;
            }
        }

        Enter(enterReplacingCardJustWon, Phase.ReplacingCardJustWon, DetermineNextStepAfterCardWasSold);

        if (BiddingTriggeredBureaucracy != null)
        {
            ApplyBureaucracy(BiddingTriggeredBureaucracy.PaymentFrom, BiddingTriggeredBureaucracy.PaymentTo);
            BiddingTriggeredBureaucracy = null;
        }
    }

    internal void DetermineNextStepAfterCardWasSold()
    {
        RicheseBiddingJustFinished = CurrentAuctionType == AuctionType.RicheseOnceAround || CurrentAuctionType == AuctionType.RicheseSilent;

        if (CardsOnAuction?.IsEmpty is not true)
        {
            MoveToNextCardOnAuction();
        }
        else
        {
            if (RegularBiddingIsDone)
            {
                if (RicheseAuctionShouldStillHappen)
                    Enter(Phase.RicheseSpecifyingAuction);
                else
                    EndBiddingPhase();
            }
            else
            {
                StartRegularBidding();
            }
        }
    }

    private void MoveToNextCardOnAuction()
    {
        CardNumber++;
        CardThatMustBeKeptOrGivenToAlly = null;

        if (IxianMaySwapCardOnBid)
        {
            if (Version < 113 || !Prevented(FactionAdvantage.IxianSwappingCard))
            {
                Enter(Phase.IxianSwappingCard);
            }
            else
            {
                LogPreventionByKarma(FactionAdvantage.IxianSwappingCard);
                if (!Applicable(Rule.FullPhaseKarma)) Allow(FactionAdvantage.IxianSwappingCard);
                Enter(Version >= 107 || IsPlaying(Faction.Atreides), Phase.WaitingForNextBiddingRound, PutNextCardOnAuction);
            }
        }
        else
        {
            Enter(Version >= 107 || IsPlaying(Faction.Atreides), Phase.WaitingForNextBiddingRound, PutNextCardOnAuction);
        }
    }

    internal void PutNextCardOnAuction()
    {
        if (!RicheseBiddingJustFinished) BidSequence!.NextRound();

        SkipPlayersThatCantBid(BidSequence!);
        WinningBid = null;
        Enter(Phase.Bidding);
    }

    internal void ContinueAfterIxianCardSwapDecision()
    {
        if (!BiddingRoundWasStarted)
            StartBiddingRound();
        else
            Enter(IsPlaying(Faction.Atreides), Phase.WaitingForNextBiddingRound, PutNextCardOnAuction);
    }

    internal void StartBidSequenceAndAuctionType(AuctionType auctionType, Player? richesePlayer = null, int direction = 1)
    {
        switch (auctionType)
        {
            case AuctionType.BlackMarketNormal:
                BlackMarketAuctionType = AuctionType.BlackMarketNormal;
                BidSequence = new PlayerSequence(this, true, 1);
                break;

            case AuctionType.BlackMarketOnceAround:
                BlackMarketAuctionType = AuctionType.BlackMarketOnceAround;
                BidSequence = new PlayerSequence(this, true, direction, richesePlayer!, true);
                break;

            case AuctionType.BlackMarketSilent:
                BidSequence = new PlayerSequence(this, true, 1);
                BlackMarketAuctionType = AuctionType.BlackMarketSilent;
                break;

            case AuctionType.RicheseOnceAround:
                BidSequence = new PlayerSequence(this, true, direction, richesePlayer!, true);
                break;

            case AuctionType.RicheseSilent:
                BidSequence = new PlayerSequence(this, true, 1);
                break;

            case AuctionType.Normal:
                BidSequence = new PlayerSequence(this, true, 1);
                if (BlackMarketAuctionType == AuctionType.BlackMarketNormal)
                {
                    //Continue where black market bidding left off
                    BidSequence.NextRound();
                    WinningBid = null;
                }
                break;
        }

        CurrentAuctionType = auctionType;
    }

    internal IBid? DetermineHighestBid(Dictionary<Faction, IBid> bids)
    {
        var highestBidValue = bids.Values.Max(b => b.TotalAmount);
        var determineBidWinnerSequence = new PlayerSequence(this, false, 1);

        for (var i = 0; i < MaximumPlayers; i++)
        {
            var f = determineBidWinnerSequence.CurrentFaction;
            if (bids.TryGetValue(f, out var value) && value.TotalAmount == highestBidValue) return bids[f];

            determineBidWinnerSequence.NextPlayer();
        }

        return null;
    }

    #endregion Bidding

    #region EndOfBidding

    internal void EndBiddingPhase()
    {
        CardThatMustBeKeptOrGivenToAlly = null;
        CurrentIxianCunning = null;
        var emperor = GetPlayer(Faction.Emperor);
        if (emperor != null)
        {
            emperor.Resources += emperor.ResourcesAfterBidding;
            emperor.ResourcesAfterBidding = 0;
        }

        MainPhaseEnd();
        Enter(Phase.BiddingReport);
    }

    #endregion

    #region Information

    public int NumberOfCardsOnRegularAuction => CardNumber + CardsOnAuction!.Items.Count - 1;

    internal IEnumerable<Player> PlayersThatCanBid => Players.Where(p => p.HasRoomForCards);

    internal bool NexusAllowsReplacingBoughtCards(Player p)
    {
        return (p.Nexus == Faction.Ixian || (Version < 159 && p.Nexus == Faction.Richese)) &&
               NexusPlayed.CanUseSecretAlly(this, p);
    }

    public bool HasBidToPay(Player p)
    {
        return CurrentPhase == Phase.Bidding && CurrentBid != null &&
               (CurrentBid.Initiator == p.Faction ||
                (CurrentBid.Initiator == p.Ally && CurrentBid.AllyContributionAmount > 0));
    }

    internal bool IxianMaySwapCardOnBid => GetPlayer(Faction.Ixian)?.TreacheryCards.Count > 0 && Applicable(Rule.IxianSwappingCardOnBid) && !IxianSwappedCardOnBid;

    public IEnumerable<TreacheryCard> CardsOwnedBy(Player p)
    {
        var cardSetAside = GetCardSetAsideForBid(p);
        if (cardSetAside == null)
        {
            return p.TreacheryCards;
        }

        var result = p.TreacheryCards.ToList();
        result.Add(cardSetAside);
        return result;
    }

    public TreacheryCard? GetCardSetAsideForBid(Player p) 
        => CurrentBid != null && CurrentBid.Player == p && CurrentBid.UsingKarmaToRemoveBidLimit ? CurrentBid.KarmaCard : null;

    #endregion Information
}