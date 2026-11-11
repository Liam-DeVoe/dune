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

    internal bool ResourceTechTokenIncome { get; set; }
    internal bool CharityIsCancelled => EconomicsStatus is ChoamEconomicsStatus.Cancel or ChoamEconomicsStatus.CancelFlipped;

    #endregion State

    #region Charity

    public void ReceiveTechIncome(TechToken token)
    {
        var techTokenOwner = Players.FirstOrDefault(p => p.TechTokens.Contains(token));
        if (techTokenOwner != null)
        {
            var amount = techTokenOwner.TechTokens.Count;
            techTokenOwner.Resources += amount;
            Log(techTokenOwner.Faction, " receive ", Payment.Of(amount), " from ", token);
        }
    }

    internal int CurrentCharityMultiplier
    {
        get
        {
            if (EconomicsStatus is ChoamEconomicsStatus.Double or ChoamEconomicsStatus.DoubleFlipped)
                return 2;
            if (EconomicsStatus is ChoamEconomicsStatus.Cancel or ChoamEconomicsStatus.CancelFlipped)
                return 0;
            return 1;
        }
    }
    
    internal void ClaimCharity(Player player)
    {
        HasActedOrPassed.Add(player.Faction);

        GiveCharity(player, (2 - player.Resources) * CurrentCharityMultiplier);

        if (!player.Is(Faction.BeneGesserit)) ResourceTechTokenIncome = true;

        Stone(Milestone.CharityClaimed);
    }

    internal void GiveCharity(Player to, int basicAmount)
    {
        var homeworldBonus = 0;
        if (GetsExtraCharityAndFreeRevivalDueToLowThreshold(to)) homeworldBonus = 1;

        var choam = GetPlayer(Faction.Choam);

        to.Resources += basicAmount + homeworldBonus;
        if (choam != null && !Prevented(FactionAdvantage.ChoamControllingCharity))
        {
            if (choam.Resources >= basicAmount)
            {
                choam.Resources -= basicAmount;
                Log(to.Faction, " get ", Payment.Of(basicAmount), " from ", Faction.Choam, MessagePart.ExpressIf(homeworldBonus > 0, " and ", Payment.Of(homeworldBonus), " from the bank"));
            }
            else
            {
                Log(to.Faction, " are unable to claim ", Payment.Of(basicAmount), " from ", Faction.Choam, MessagePart.ExpressIf(homeworldBonus > 0, " but get ", Payment.Of(homeworldBonus), " from the bank"));
            }
        }
        else
        {
            var amount = basicAmount + homeworldBonus;
            Log(Message.Express(to.Faction, " claim ", Payment.Of(amount)).WithLog(new CharityClaim(to.Faction, amount)));
        }
    }

    #endregion Charity
}