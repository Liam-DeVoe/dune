/*
 * Copyright (C) 2020-2025 Ronald Ossendrijver (admin@treachery.online)
 * This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License
 * as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version. This
 * program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details. You should have
 * received a copy of the GNU General Public License along with this program. If not, see <http://www.gnu.org/licenses/>.
 */

namespace Treachery.Shared;

public class DealOffered : GameEvent
{
    #region Construction

    public DealOffered(Game game, Faction initiator) : base(game, initiator)
    {
    }

    public DealOffered()
    {
    }

    #endregion Construction

    #region Properties

    public Faction[] To { get; set; } = [];

    public DealType Type { get; set; }

    public string? DealParameter1 { get; set; }

    public string? DealParameter2 { get; set; }

    public string? Text { get; set; }

    public Phase EndPhase { get; set; }

    public int Price { get; set; }

    public int Benefit { get; set; }

    public bool Cancel { get; set; }

    #endregion Properties

    #region Validation

    public override Message? Validate()
    {
        return null;
    }

    public DealOffered Cancellation()
    {
        var result = (DealOffered)MemberwiseClone();
        result.Cancel = true;
        return result;
    }

    public DealAccepted Acceptance(Faction by)
    {
        return new DealAccepted(Game, by)
        {
            BoundFaction = Initiator,
            Price = Price,
            Type = Type,
            DealParameter1 = DealParameter1,
            DealParameter2 = DealParameter2,
            Text = Text,
            Benefit = Benefit,
            End = EndPhase
        };
    }

    public bool Same(DealOffered offeredDeal)
    {
        return
            offeredDeal.Price == Price &&
            offeredDeal.Type == Type &&
            offeredDeal.DealParameter1 == DealParameter1 &&
            offeredDeal.DealParameter2 == DealParameter2 &&
            offeredDeal.Text == Text &&
            offeredDeal.Benefit == Benefit &&
            offeredDeal.EndPhase == EndPhase &&
            offeredDeal.To.SequenceEqual(To);
    }

    public DealRejected Rejection(Faction by)
    {
        return new DealRejected(Game, by)
        {
            BoundFaction = Initiator,
            Price = Price,
            Type = Type,
            DealParameter1 = DealParameter1,
            DealParameter2 = DealParameter2,
            Text = Text,
            Benefit = Benefit,
            End = EndPhase
        };
    }

    public bool IsAcceptedBy(DealAccepted acceptedDeal)
    {
        return Matches(acceptedDeal.Initiator, acceptedDeal.BoundFaction, acceptedDeal.Price, acceptedDeal.Type, acceptedDeal.DealParameter1,
            acceptedDeal.DealParameter2, acceptedDeal.Text, acceptedDeal.Benefit, acceptedDeal.End);
    }

    public bool IsRejectedBy(DealRejected rejectedDeal)
    {
        return Matches(rejectedDeal.Initiator, rejectedDeal.BoundFaction, rejectedDeal.Price, rejectedDeal.Type, rejectedDeal.DealParameter1,
            rejectedDeal.DealParameter2, rejectedDeal.Text, rejectedDeal.Benefit, rejectedDeal.End);
    }

    private bool Matches(Faction by, Faction boundFaction, int price, DealType type, string? dealParameter1, string? dealParameter2, string? text, int benefit, Phase end)
    {
        return
            boundFaction == Initiator &&
            price == Price &&
            type == Type &&
            dealParameter1 == DealParameter1 &&
            dealParameter2 == DealParameter2 &&
            text == Text &&
            benefit == Benefit &&
            end == EndPhase &&
            (To.Length == 0 || To.Contains(by));
    }

    public static IEnumerable<DealType> GetStandardDealTypes(Game g, Player p)
    {
        var result = new List<DealType>
        {
            DealType.None,
            DealType.ForfeitBattle,
            DealType.TellDiscardedTraitors
        };

        switch (p.Faction)
        {
            case Faction.Atreides:
                if (g.HasBiddingPrescience(p)) result.Add(DealType.ShareBiddingPrescience);
                if (g.HasResourceDeckPrescience(p)) result.Add(DealType.ShareResourceDeckPrescience);
                break;

            case Faction.Fremen:
                if (g.HasStormPrescience(p)) result.Add(DealType.ShareStormPrescience);
                break;
        }

        return result;
    }

    #endregion Validation

    #region Execution

    public Message GetDealDescription()
    {
        return Deal.DealContentsDescription(Game, Type, Text, Benefit, EndPhase, DealParameter1);
    }

    protected override void ExecuteConcreteEvent()
    {
        if (Cancel)
        {
            var sameOffer = Game.DealOffers.FirstOrDefault(o => o.Same(this));
            if (sameOffer != null)
            {
                Game.DealOffers.Remove(sameOffer);
                Game.DealResponses.Remove(sameOffer);
            }
        }
        else
        {
            Game.DealOffers.Add(this);
        }
    }

    public override Message GetMessage()
    {
        return !Cancel 
            ? Message.Express(Initiator, " offer ", MessagePart.ExpressIf(To.Length != 0, To, " "), "for ", Payment.Of(Price), ": ", Deal.DealContentsDescription(Game, Type, Text, Benefit, EndPhase, DealParameter1)) 
            : Message.Express(Initiator, " withdraw a deal offer");
    }

    public override Message GetShortMessage()
    {
        if (!Cancel)
            return Message.Express(Initiator, " offer a deal");
        return Message.Express(Initiator, " withdraw a deal offer");
    }

    #endregion Execution
}