/*
 * Copyright (C) 2020-2025 Ronald Ossendrijver (admin@treachery.online)
 * This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License
 * as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version. This
 * program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details. You should have
 * received a copy of the GNU General Public License along with this program. If not, see <http://www.gnu.org/licenses/>.
 */

namespace Treachery.Shared;

public class DealRejected : GameEvent
{
    #region Construction

    public DealRejected(Game game, Faction initiator) : base(game, initiator)
    {
    }

    public DealRejected()
    {
    }

    #endregion Construction

    #region Properties

    public Faction BoundFaction { get; set; }

    public DealType Type { get; set; }

    public string? DealParameter1 { get; set; }

    public string? DealParameter2 { get; set; }

    public string? Text { get; set; }

    public Phase End { get; set; }

    public int Price { get; set; }

    public int Benefit { get; set; }

    #endregion Properties

    #region Validation

    public override Message? Validate()
    {
        if (!Game.DealOffers.Any(offer => offer.IsRejectedBy(this))) return Message.Express("Offer is not valid (anymore)");
        if (OpenOffer == null) return Message.Express("You already answered this offer");

        return null;
    }

    // The matching offer this faction may still answer.
    private DealOffered? OpenOffer => DealAccepted.AcceptableDeals(Game, Player).FirstOrDefault(offer => offer.IsRejectedBy(this));

    #endregion Validation

    #region Execution

    protected override void ExecuteConcreteEvent()
    {
        Log(GetMessage().WithLog(AsideLog.Aside));

        var offer = OpenOffer;
        if (offer != null) Game.RecordDealResponse(offer, Initiator, false);
    }

    public override Message GetMessage()
    {
        return Message.Express(
            Initiator,
            " decline ",
            BoundFaction,
            " offer for ",
            Payment.Of(Price),
            ": ",
            Deal.DealContentsDescription(Game, Type, Text, Benefit, End, DealParameter1));
    }

    public override Message GetShortMessage()
    {
        return Message.Express(Initiator, " decline a deal by ", BoundFaction);
    }

    #endregion Execution
}
