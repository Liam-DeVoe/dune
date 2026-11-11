/*
 * Copyright (C) 2020-2025 Ronald Ossendrijver (admin@treachery.online)
 * This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License
 * as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version. This
 * program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details. You should have
 * received a copy of the GNU General Public License along with this program. If not, see <http://www.gnu.org/licenses/>.
 */


namespace Treachery.Shared;

public class RicheseSpecifiesAuction : GameEvent
{
    #region Construction

    public RicheseSpecifiesAuction(Game game, Faction initiator) : base(game, initiator)
    {
    }

    public RicheseSpecifiesAuction()
    {
    }

    #endregion Construction

    #region Properties

    public AuctionType AuctionType { get; set; }

    public int Direction { get; set; }

    public int _cardId;

    [JsonIgnore]
    public TreacheryCard Card
    {
        get => TreacheryCardManager.Get(_cardId);
        set => _cardId = TreacheryCardManager.GetId(value);
    }

    #endregion Properties

    #region Validation

    public override Message? Validate()
    {
        if (!Game.RicheseOccupierSpecifiedCard && !ValidCards(Game).Contains(Card)) return Message.Express("Invalid card");

        if (AuctionType != AuctionType.RicheseOnceAround && AuctionType != AuctionType.RicheseSilent) return Message.Express("Invalid auction type");

        return null;
    }

    public static IEnumerable<TreacheryCard> ValidCards(Game g)
    {
        return g.RicheseCache;
    }

    public static bool MaySpecifyCard(Game g, Player p)
    {
        var occupierOfRicheseHomeworld = g.OccupierOf(World.Richese);
        return (occupierOfRicheseHomeworld == null && p.Faction == Faction.Richese) ||
               (occupierOfRicheseHomeworld != null && !g.RicheseOccupierSpecifiedCard && occupierOfRicheseHomeworld.Is(p.Faction));
    }

    public static bool MaySpecifyTypeOfAuction(Game g, Player p)
    {
        var occupierOfRicheseHomeworld = g.OccupierOf(World.Richese);
        return (occupierOfRicheseHomeworld == null && p.Faction == Faction.Richese) ||
               (occupierOfRicheseHomeworld != null && g.RicheseOccupierSpecifiedCard && p.Faction == Faction.Richese);
    }

    #endregion Validation

    #region Execution

    protected override void ExecuteConcreteEvent()
    {
        if (!Game.RicheseOccupierSpecifiedCard)
        {
            Game.RicheseAuctionShouldStillHappen = false;
            Game.CardsOnAuction.PutOnTop(Card);
            Game.RicheseCache.Remove(Card);
            Game.RegisterKnown(Card);

            var occupierOfRicheseHomeworld = Game.OccupierOf(World.Richese);
            if (occupierOfRicheseHomeworld == null)
            {
                Log();
                Game.StartBidSequenceAndAuctionType(AuctionType, Player, Direction);
                Game.StartBiddingRound();
            }
            else
            {
                Log(Initiator, " put ", Card, " on auction");
                Game.RicheseOccupierSpecifiedCard = true;
            }
        }
        else
        {
            var directionText = "";
            if (AuctionType == AuctionType.RicheseOnceAround)
            {
                if (Direction == 1)
                    directionText = " (counter-clockwise)";
                else
                    directionText = " (clockwise)";
            }

            Log(Initiator, " put select a ", AuctionType, " auction", directionText);

            Game.StartBidSequenceAndAuctionType(AuctionType, Player, Direction);
            Game.StartBiddingRound();
        }
    }

    public override Message GetMessage()
    {
        var directionText = "";
        if (AuctionType == AuctionType.RicheseOnceAround)
        {
            if (Direction == 1)
                directionText = " (counter-clockwise)";
            else
                directionText = " (clockwise)";
        }

        return Message.Express(Initiator, " put a ", Faction.Richese, " card on ", AuctionType, " auction", directionText);
    }

    #endregion Execution
}