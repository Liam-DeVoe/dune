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
    #region ChoamCardTrading

    public CardTraded? CurrentCardTradeOffer { get; internal set; }
    internal Phase? PhaseBeforeCardTrade { get; set; }
    internal int LastTurnCardWasTraded { get; set; } = -1;

    #endregion

    #region Nexus

    public List<AllianceOffered> CurrentAllianceOffers { get; } = [];
    internal bool NexusHasOccured { get; set; }

    internal Dictionary<Faction, int> PermittedUseOfAllySpice { get; } = new();
    internal Dictionary<Faction, int> PermittedUseOfEmperorSpice { get; set; } = new();
    internal Dictionary<Faction, TreacheryCard> PermittedUseOfAllyKarma { get; } = new();

    public bool AtreidesSharesPrescience { get; internal set; }
    public bool FremenWillProtectFromMonster { get; internal set; }
    public bool FremenAllowsThreeFreeRevivals { get; internal set; }
    public bool FremenSharesPrescience { get; internal set; }
    public bool FremenRefundsBattleDial { get; internal set; }
    public int EmperorWillPayForExtraRevival { get; internal set; }
    public bool GuildAllowsShippingDiscount { get; internal set; }
    public bool BeneGesseritAllowsUseOfVoice { get; internal set; }
    public bool IxianAllowsReplacingCards { get; internal set; }
    public bool TleilaxuAllowsRevivalDiscount { get; internal set; }
    public bool RicheseAllowsUseOfNoField { get; internal set; }
    public bool MoritaniAllowsKeepingCards { get; internal set; }
    public bool EcazSharesAmbassadors { get; internal set; }

    internal void MakeAlliance(Faction a, Faction b)
    {
        var playerA = GetPlayer(a);
        var playerB = GetPlayer(b);
        playerA!.Ally = b;
        playerB!.Ally = a;
        DiscardNexusCard(playerA);
        DiscardNexusCard(playerB);
        Log(Message.Express(a, " and ", b, " are now allies").WithLog(new NexusLog(Alliance: new NexusAlliance(NexusAllianceChange.Made, a, b))));

        if (Version > 150)
        {
            SetPermissions(a, true);
            SetPermissions(b, true);
        }
            
        while (playerA.HandSizeExceeded) Discard(playerA, playerA.TreacheryCards.RandomOrDefault(Random!));

        while (playerB.HandSizeExceeded) Discard(playerB, playerB.TreacheryCards.RandomOrDefault(Random!));
    }

    internal void BreakAllAlliances()
    {
        foreach (var p in Players.Where(p => p.HasAlly).ToList())
            if (p.HasAlly)
            {
                Log(Message.Express(p.Faction, " and ", p.Ally, " alliance is broken").WithLog(new NexusLog(Alliance: new NexusAlliance(NexusAllianceChange.Ended, p.Faction, p.Ally))));
                BreakAlliance(p.Faction);
            }
    }

    internal void BreakAlliance(Faction f)
    {
        var initiator = GetPlayer(f);
        var currentAlly = initiator?.AlliedPlayer;

        if (initiator is null || currentAlly is null) return;
        
        if (Version <= 150)
        {
            if (f == Faction.Guild || initiator.Ally == Faction.Guild) GuildAllowsShippingDiscount = false;

            if (f == Faction.Emperor || initiator.Ally == Faction.Emperor) EmperorWillPayForExtraRevival = 0;

            if (f == Faction.Fremen || initiator.Ally == Faction.Fremen)
            {
                FremenWillProtectFromMonster = false;
                FremenAllowsThreeFreeRevivals = false;
            }
        }
        else
        {
            SetPermissions(f, false);
            SetPermissions(initiator.Ally, false);
        }

        PermittedUseOfAllySpice.Remove(f);
        PermittedUseOfAllySpice.Remove(initiator.Ally);
        PermittedUseOfAllyKarma.Remove(f);
        PermittedUseOfAllyKarma.Remove(initiator.Ally);

        initiator.Ally = Faction.None;
        currentAlly.Ally = Faction.None;
    }

    internal void DecreasePermittedUseOfAllySpice(Faction f, int amount)
    {
        if (PermittedUseOfAllySpice.ContainsKey(f))
        {
            var newValue = PermittedUseOfAllySpice[f] - amount;
            PermittedUseOfAllySpice[f] = Math.Max(0, newValue);
        }
    }

    private void SetPermissions(Faction f, bool permission)
    {
        switch (f)
        {
            case Faction.Atreides: AtreidesSharesPrescience = permission; break;
            case Faction.Fremen:
                FremenWillProtectFromMonster = permission;
                FremenAllowsThreeFreeRevivals = permission;
                FremenSharesPrescience = permission;
                FremenRefundsBattleDial = permission;
                break;
            case Faction.Emperor: EmperorWillPayForExtraRevival = permission ? 3 : 0; break;
            case Faction.Guild: GuildAllowsShippingDiscount = permission; break;
            case Faction.BeneGesserit: BeneGesseritAllowsUseOfVoice = permission; break;
            case Faction.Ixian: IxianAllowsReplacingCards = permission; break;
            case Faction.Tleilaxu: TleilaxuAllowsRevivalDiscount = permission; break;
            case Faction.Richese: RicheseAllowsUseOfNoField = permission; break;
            case Faction.Moritani: MoritaniAllowsKeepingCards = permission; break;
            case Faction.Ecaz: EcazSharesAmbassadors = permission; break;
        }
    }

    #endregion Nexus

    #region Information

    internal bool AreAllies(Faction a, Faction b)
    {
        return GetPlayer(a)?.Ally == b;
    }

    public int ResourcesYourAllyCanPay(Player p)
    {
        if (!PermittedUseOfAllySpice.TryGetValue(p.Faction, out var value)) return 0;
        var ally = GetPlayer(p.Ally);
        return ally == null ? 0 : Math.Min(value, ally.Resources);

    }

    public int SpiceForBidsEmperorCanPay(Faction f)
    {
        if (!PermittedUseOfEmperorSpice.TryGetValue(f, out var value)) return 0;
        
        var emperor = GetPlayer(Faction.Emperor);
        return emperor == null ? 0 : Math.Min(value, emperor.Resources);
    }

    public TreacheryCard? GetPermittedUseOfAllyKarma(Faction f)
    {
        var ally = Players.SingleOrDefault(p => p.Ally == f);

        if (ally != null && PermittedUseOfAllyKarma.TryGetValue(f, out var value) && ally.Has(value))
            return PermittedUseOfAllyKarma[f];
        
        return null;
    }

    public int GetPermittedUseOfAllyResources(Faction f)
    {
        var ally = GetPlayer(f)?.AlliedPlayer;

        if (!PermittedUseOfAllySpice.TryGetValue(f, out var value) || ally == null)
            return 0;
        return Math.Min(value, ally.Resources);
    }

    #endregion Information
}