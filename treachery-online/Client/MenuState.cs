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

    public void Toggle(MenuPanel panel) => Panel = Panel == panel ? MenuPanel.None : panel;

    public void Open(MenuPanel panel) => Panel = panel;

    public void Close() => Panel = MenuPanel.None;

    // The faction whose rules sheet the FactionRules panel shows. Clicking a
    // faction's token on the map opens it; clicking the same faction again closes it.
    public Faction RulesFaction { get; private set; }

    public void ShowRules(Faction faction)
    {
        if (Panel == MenuPanel.FactionRules && RulesFaction == faction)
        {
            Close();
            return;
        }

        RulesFaction = faction;
        Panel = MenuPanel.FactionRules;
    }
}

public enum MenuPanel
{
    None,
    Players,
    Undo,
    FactionRules
}
