/*
 * Copyright (C) 2020-2025 Ronald Ossendrijver (admin@treachery.online)
 * This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License
 * as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version. This
 * program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details. You should have
 * received a copy of the GNU General Public License along with this program. If not, see <http://www.gnu.org/licenses/>.
 */

namespace Treachery.Client;

// What the menu bar under the map has open. The bar (MenuComponent) and the
// panels it opens at the top of a side column (MenuPanels) are in different
// columns, so they share this instead of one holding the other. Whoever
// changes it calls Client.Refresh, which redraws both.
public class MenuState
{
    public MenuPanel Panel { get; private set; }

    // Set when a saved game or skin fails to load.
    public string LoadError { get; set; } = "";

    public void Toggle(MenuPanel panel)
    {
        Panel = Panel == panel ? MenuPanel.None : panel;
        MarkedDeal = null;
    }

    public void Open(MenuPanel panel) => Panel = panel;

    public void Close()
    {
        Panel = MenuPanel.None;
        MarkedDeal = null;
    }

    // The offer whose toast (DealToasts) was clicked, marked in Deals until it closes.
    public DealOffered? MarkedDeal { get; set; }

    // The treachery card whose play panel is open beside your faction card
    // (CardPlayPopover), and the height, in window pixels, it was clicked at.
    public TreacheryCard? CardPlay { get; private set; }

    public double CardPlayY { get; private set; }

    public void ToggleCardPlay(TreacheryCard card, double y)
    {
        CardPlay = CardPlay == card ? null : card;
        CardPlayY = y;
    }

    public void CloseCardPlay() => CardPlay = null;
}

public enum MenuPanel
{
    None,
    Players,
    Undo,
    Automation,

    // These open in a popover above the menu bar (TradePopover), not in a side column.
    Deals,
    Alliance,
    Cards
}
