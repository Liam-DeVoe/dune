/*
 * Copyright (C) 2020-2025 Ronald Ossendrijver (admin@treachery.online)
 * This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License
 * as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version. This
 * program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details. You should have
 * received a copy of the GNU General Public License along with this program. If not, see <http://www.gnu.org/licenses/>.
 */

namespace Treachery.Shared;

public class BlackMarketBid : PassableGameEvent, IBid
{
    #region Construction

    public BlackMarketBid(Game game, Faction initiator) : base(game, initiator)
    {
    }

    public BlackMarketBid()
    {
    }

    #endregion Construction

    #region Properties

    public int Amount { get; set; }

    public int AllyContributionAmount { get; set; }

    public int EmperorContributionAmount { get; set; }

    [JsonIgnore]
    public bool UsesEmperorSecretAlly => false;

    [JsonIgnore]
    public int TotalAmount => Amount + AllyContributionAmount + EmperorContributionAmount;

    [JsonIgnore]
    public bool UsingKarmaToRemoveBidLimit => false;

    [JsonIgnore]
    public TreacheryCard? KarmaCard => null;

    [JsonIgnore]
    public bool KarmaBid => false;

    #endregion Properties

    #region Execution

    protected override void ExecuteConcreteEvent()
    {
        Game.BidSequence!.NextPlayer();
        Game.SkipPlayersThatCantBid(Game.BidSequence);
        Game.Stone(Milestone.Bid);
        Game.Bids.Remove(Initiator);
        Game.Bids.Add(Initiator, this);
        Game.CheckIfAtreidesOrAtreidesAllyPassedBid(this);
        
        if (!Passed) Game.CurrentBid = this;

        switch (Game.CurrentAuctionType)
        {
            case AuctionType.BlackMarketNormal:
                HandleBlackMarketNormalBid();
                break;

            case AuctionType.BlackMarketOnceAround:
            case AuctionType.BlackMarketSilent:
                HandleBlackMarketSpecialBid();
                break;
        }
    }

    private void HandleBlackMarketNormalBid()
    {
        if (Passed)
        {
            var winningBid = Game.CurrentBid;

            if (winningBid != null && Game.BidSequence!.CurrentFaction == winningBid.Initiator)
            {
                if (Game.CardsOnAuction == null) return;

                var card = Game.WinByHighestBid(
                    winningBid.Player,
                    winningBid,
                    winningBid.Amount,
                    winningBid.AllyContributionAmount,
                    winningBid.EmperorContributionAmount,
                    winningBid.Initiator != Faction.Richese ? Faction.Richese : Faction.Emperor,
                    Game.CardsOnAuction, false);

                FinishBlackMarketBid(winningBid.Player, winningBid, card);
            }
            else if (winningBid == null && Game.Bids.Count >= Game.PlayersThatCanBid.Count())
            {
                var richese = GetPlayer(Faction.Richese);
                var card = Game.CardsOnAuction?.Draw();
                if (richese != null && card != null) richese.TreacheryCards.Add(card);
                Log(Faction.Richese, " keep their card as no faction bid on it");
                Game.EnterRicheseBidding();
            }
        }
    }

    private void HandleBlackMarketSpecialBid()
    {
        var isLastBid = Game.Version < 140 ? Game.Players.Count(p => p.HasRoomForCards) == Game.Bids.Count :
            (Game.CurrentAuctionType == AuctionType.BlackMarketSilent && Game.Players.Count(p => p.HasRoomForCards) == Game.Bids.Count) ||
            (Game.CurrentAuctionType == AuctionType.BlackMarketOnceAround && Initiator == Faction.Richese) || (GetPlayer(Faction.Richese)?.HasRoomForCards == false && Game.BidSequence!.HasPassedRichese);

        if (isLastBid)
        {
            if (Game.CurrentAuctionType == AuctionType.BlackMarketSilent) Log(Game.Bids.Select(b => MessagePart.Express(b.Key, Payment.Of(b.Value.TotalAmount), " ")).ToList());

            var highestBid = Game.DetermineHighestBid(Game.Bids);
            if (highestBid is { TotalAmount: > 0 })
            {
                if (Game.CardsOnAuction == null) return;

                var card = Game.WinByHighestBid(
                    highestBid.Player,
                    highestBid,
                    highestBid.Amount,
                    highestBid.AllyContributionAmount,
                    highestBid.EmperorContributionAmount,
                    highestBid.Initiator != Faction.Richese ? Faction.Richese : Faction.Emperor,
                    Game.CardsOnAuction, false);

                FinishBlackMarketBid(highestBid.Player, highestBid, card);
            }
            else
            {
                var richese = GetPlayer(Faction.Richese);
                var card = Game.CardsOnAuction?.Draw();
                if (richese != null && card != null) richese.TreacheryCards.Add(card);
                Log(Faction.Richese, " keep their card as no faction bid on it");
                Game.EnterRicheseBidding();
            }
        }
    }

    private void FinishBlackMarketBid(Player winner, IBid winningBid, TreacheryCard card)
    {
        Game.CardJustWon = card;
        Game.WinningBid = winningBid;
        Game.CardSoldOnBlackMarket = card;
        Game.CurrentBid = null;
        Game.FactionThatMayReplaceBoughtCard = Faction.None;

        Game.Bids.Clear();
        Game.LatestBidByAtreidesOrAtreidesAllyWasPassed = false;

        var enterReplacingCardJustWon = Game.Version > 150 &&
                                        ((Game.Version < 164 && Game.Players.Any(p => p.Nexus != Faction.None)) ||
                                         (Game.Version >= 164 && winner.Nexus != Faction.None));


        if (winner.Ally == Faction.Ixian && Game.IxianAllowsReplacingCards)
        {
            if (!Game.Prevented(FactionAdvantage.IxianAllyDiscardingCard))
            {
                Game.FactionThatMayReplaceBoughtCard = winner.Faction;
                enterReplacingCardJustWon = true;
            }
            else
            {
                if (!Game.Applicable(Rule.FullPhaseKarma)) Game.Allow(FactionAdvantage.IxianAllyDiscardingCard);

                if (Game.NexusAllowsReplacingBoughtCards(winner))
                {
                    Game.FactionThatMayReplaceBoughtCard = winner.Faction;
                    enterReplacingCardJustWon = true;
                    Game.ReplacingBoughtCardUsingNexus = true;
                }
            }
        }
        else if (Game.NexusAllowsReplacingBoughtCards(winner))
        {
            Game.FactionThatMayReplaceBoughtCard = winner.Faction;
            enterReplacingCardJustWon = true;
            Game.ReplacingBoughtCardUsingNexus = true;
        }

        Game.Enter(enterReplacingCardJustWon, Phase.ReplacingCardJustWon, Game.EnterRicheseBidding);

        if (Game.BiddingTriggeredBureaucracy != null)
        {
            Game.ApplyBureaucracy(Game.BiddingTriggeredBureaucracy.PaymentFrom,
                Game.BiddingTriggeredBureaucracy.PaymentTo);
            Game.BiddingTriggeredBureaucracy = null;
        }
    }

    public override Message GetMessage()
    {
        if (!Passed)
            return Message.Express(Initiator, " bid");
        
        return Message.Express(Initiator, " pass");
    }

    #endregion Execution

    #region Validation

    public override Message? Validate()
    {
        if (Passed) return null;

        if (Game.CurrentAuctionType != AuctionType.BlackMarketSilent && TotalAmount < 1) return Message.Express("Bid must be higher than 0");
        if (Game.CurrentAuctionType != AuctionType.BlackMarketSilent && Game.CurrentBid != null && TotalAmount <= Game.CurrentBid.TotalAmount) return Message.Express("Bid not high enough");

        if (AllyContributionAmount > ValidMaxAllyAmount(Game, Player)) return Message.Express("your ally won't pay that much");

        var emperor = Game.GetPlayer(Faction.Emperor);
        if (EmperorContributionAmount > 0 && EmperorContributionAmount > (emperor?.Resources ?? 0)) return Message.Express(Faction.Emperor, " won't pay that much");

        if (Game.Version >= 155)
        {
            if (Game.CurrentAuctionType == AuctionType.BlackMarketSilent && TotalAmount > Player.Resources) return Message.Express("In a Silent auction, you can't bid more than you have");
        }

        return null;
    }

    public static int ValidMaxAmount(Player p)
    {
        return p.Resources;
    }

    public static int ValidMaxAllyAmount(Game g, Player p)
    {
        return g.ResourcesYourAllyCanPay(p);
    }

    public static IEnumerable<SequenceElement> PlayersToBid(Game g)
    {
        var playersInSequence = g.BidSequence?.GetPlayersInSequence() ?? Array.Empty<SequenceElement>();
        return g.CurrentAuctionType switch
        {
            AuctionType.BlackMarketNormal or AuctionType.BlackMarketOnceAround => playersInSequence,
            AuctionType.BlackMarketSilent => g.Players.Select(p => new SequenceElement { Player = p, HasTurn = p.HasRoomForCards && !g.Bids.ContainsKey(p.Faction) }),
            _ => Array.Empty<SequenceElement>()
        };
    }

    public static bool MayBePlayed(Game game, Player player)
    {
        return (game.CurrentAuctionType == AuctionType.BlackMarketSilent && !game.Bids.ContainsKey(player.Faction) && player.HasRoomForCards) ||
               (game.CurrentAuctionType != AuctionType.BlackMarketSilent && player == game.BidSequence?.CurrentPlayer);
    }

    #endregion Validation
}