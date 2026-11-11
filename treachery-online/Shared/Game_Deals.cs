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
    public List<DealOffered> DealOffers { get; } = new();

    public List<Deal> Deals { get; } = new();

    // How each recipient answered an open offer: true if they accepted, false
    // if they declined. A recipient with no entry hasn't answered yet.
    public Dictionary<DealOffered, Dictionary<Faction, bool>> DealResponses { get; } = new();

    internal void RecordDealResponse(DealOffered offer, Faction faction, bool accepted)
    {
        if (!DealResponses.TryGetValue(offer, out var responses))
        {
            responses = new Dictionary<Faction, bool>();
            DealResponses.Add(offer, responses);
        }

        responses[faction] = accepted;
    }

    public bool? DealResponse(DealOffered offer, Faction faction)
    {
        return DealResponses.TryGetValue(offer, out var responses) && responses.TryGetValue(faction, out var accepted) ? accepted : null;
    }

    public bool HasRespondedToDeal(DealOffered offer, Faction faction) => DealResponse(offer, faction) != null;

    // The factions that may accept an offer, in seat order.
    public IEnumerable<Faction> DealRecipients(DealOffered offer)
    {
        return Players
            .Where(p => p.Faction != offer.Initiator && p.Ally != offer.Initiator && (offer.To.Length == 0 || offer.To.Contains(p.Faction)))
            .Select(p => p.Faction);
    }

    internal void StartDeal(Deal deal)
    {
        Deals.Add(deal);
    }

    private bool HasDeal(Faction f, DealType type)
    {
        return Deals.Any(deal => deal.ConsumingFaction == f && deal.Type == type);
    }

    private void RemoveEndedDeals(Phase phase)
    {
        foreach (var deal in Deals.ToArray())
            if (deal.End == phase) Deals.Remove(deal);
    }

    public bool IsGhola(IHero l)
    {
        var tleilaxu = GetPlayer(Faction.Tleilaxu);
        return l.Faction != Faction.Tleilaxu && tleilaxu != null && tleilaxu.Leaders.Contains(l);
    }
}