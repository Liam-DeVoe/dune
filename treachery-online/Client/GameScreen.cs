/*
 * Copyright (C) 2020-2025 Ronald Ossendrijver (admin@treachery.online)
 * This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License
 * as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version. This
 * program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details. You should have
 * received a copy of the GNU General Public License along with this program. If not, see <http://www.gnu.org/licenses/>.
 */

namespace Treachery.Client;

// The variable Styles/components/_game-screen.scss sizes the in-game row
// with, for Main.razor and Scene.razor.
public static class GameScreen
{
    // The height the turn banner's spacer adds above the row, as
    // $turn-banner-h in _turn-banner.scss sets it.
    private const int TurnBannerHeight = 40;

    // Whether it's the player's turn, so TurnBanner shows.
    public static bool ShowsTurnBanner(IGameService client) =>
        !client.IsObserver && client.Player is { } player && client.Status is { } status && status.WaitingForMe(player, client.IsHost);

    public static string Style(IGameService client)
    {
        var banner = ShowsTurnBanner(client) ? TurnBannerHeight : 0;
        return FormattableString.Invariant($"--turn-banner-h: {banner}px;");
    }
}
