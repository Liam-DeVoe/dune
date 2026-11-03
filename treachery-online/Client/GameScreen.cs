/*
 * Copyright (C) 2020-2025 Ronald Ossendrijver (admin@treachery.online)
 * This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License
 * as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version. This
 * program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details. You should have
 * received a copy of the GNU General Public License along with this program. If not, see <http://www.gnu.org/licenses/>.
 */

namespace Treachery.Client;

// The variables Styles/components/_game-screen.scss sizes the map column
// with, for the in-game row in Main.razor and Scene.razor.
public static class GameScreen
{
    // The menu bar under the board, as .menu-bar sets it.
    private const int MenuBarHeight = 36;

    // The undo timeline with its margins, shown between the board and the
    // menu bar while the host has the undo panel open.
    private const int UndoTimelineHeight = 116;

    public static string Style(IGameService client)
    {
        var map = client.CurrentSkin.MapDimensions;
        var belowMap = MenuBarHeight + (client.UndoSession != null ? UndoTimelineHeight : 0);
        return FormattableString.Invariant($"--map-ratio: {map.Y / map.X:0.#####}; --below-map: {belowMap}px;");
    }
}
