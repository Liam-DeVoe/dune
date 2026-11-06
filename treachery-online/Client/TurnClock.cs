/*
 * Copyright (C) 2020-2025 Ronald Ossendrijver (admin@treachery.online)
 * This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License
 * as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version. This
 * program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details. You should have
 * received a copy of the GNU General Public License along with this program. If not, see <http://www.gnu.org/licenses/>.
 */


namespace Treachery.Client;

// One clock for every "how long has the game been waiting" display (the turn
// banner and the prompt card), so they always show the same time and change
// together. It ticks on whole seconds; Now is the time of the latest tick.
public sealed class TurnClock : IDisposable
{
    private readonly System.Threading.Timer _timer;

    public DateTimeOffset Now { get; private set; } = Floor(DateTimeOffset.Now);

    // Raised on every tick. Displays redraw on it.
    public event Action? Tick;

    public TurnClock()
    {
        var untilNextSecond = 1000 - DateTimeOffset.Now.Millisecond;
        _timer = new System.Threading.Timer(_ =>
        {
            Now = Floor(DateTimeOffset.Now);
            Tick?.Invoke();
        }, null, untilNextSecond, 1000);
    }

    // How long since the given moment, as m:ss, with all the hours in front once there are any.
    public string Since(DateTimeOffset since)
    {
        var t = Now - since;
        if (t < TimeSpan.Zero) t = TimeSpan.Zero;
        return t.TotalHours >= 1
            ? $"{(int)t.TotalHours}:{t.Minutes:00}:{t.Seconds:00}"
            : $"{t.Minutes}:{t.Seconds:00}";
    }

    private static DateTimeOffset Floor(DateTimeOffset t) => t.AddTicks(-(t.Ticks % TimeSpan.TicksPerSecond));

    public void Dispose() => _timer.Dispose();
}
