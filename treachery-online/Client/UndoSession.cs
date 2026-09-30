/*
 * Copyright (C) 2020-2025 Ronald Ossendrijver (admin@treachery.online)
 * This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License
 * as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version. This
 * program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details. You should have
 * received a copy of the GNU General Public License along with this program. If not, see <http://www.gnu.org/licenses/>.
 */

namespace Treachery.Client;

// The host choosing a moment to undo to. A moment is a gap between two
// events: gap n sits just before event n, so undoing to it is Undo(n), which
// keeps events 0 to n-1. While a session is open the game service shows
// Preview, the game as it was at that gap, in place of the live game.
public class UndoSession(Game live, int userId)
{
    // Events the undo list has always left out.
    public static bool IsListed(GameEvent e) => e is not (AllyPermission or BattleRevision or AutomationConfigured);

    public Game Live { get; } = live;

    // Null while the chosen moment is now. Kept apart from Live.History.Count
    // so that "now" stays now when new events arrive during the session.
    private int? _gap;

    public int Gap => _gap ?? Live.History.Count;

    public bool AtNow => _gap == null;

    // The game at Gap, or null at now.
    public Game? Preview { get; private set; }

    public GameStatus? PreviewStatus { get; private set; }

    // Event 0 sets the game up and can't be undone.
    public const int FirstGap = 1;

    public void MoveTo(int gap)
    {
        gap = Math.Clamp(gap, FirstGap, Live.History.Count);
        if (gap == Gap) return;

        if (gap == Live.History.Count)
        {
            _gap = null;
            Preview = null;
            PreviewStatus = null;
            return;
        }

        var preview = Live.Undo(gap);

        // Undo(n) remembers the dropped events so the real undo can be
        // restored. The preview isn't a real undo, so it must not offer that.
        preview.RecentlyUndoneEventCount = 0;
        preview.RecentlyUndoneEvents.Clear();

        // Replaying re-binds each event to the game it runs in. The live
        // game keeps receiving events during the session, so bind its
        // history back to it.
        RebindLiveHistory();

        _gap = gap;
        Preview = preview;
        PreviewStatus = GameStatus.DetermineStatus(preview, preview.GetPlayerByUserId(userId), !preview.IsObserver(userId));
    }

    // The events that can be undone and are listed, oldest first. Gaps offered
    // to the host are the gaps just before these, and now.
    public List<int> ListedEvents()
    {
        var result = new List<int>();
        for (var i = FirstGap; i < Live.History.Count; i++)
            if (IsListed(Live.History[i]))
                result.Add(i);
        return result;
    }

    // The turn and phase event n happened in.
    public Moment MomentOf(int eventNr) => Live.Moments[eventNr - 1];

    public void RebindLiveHistory()
    {
        foreach (var e in Live.History)
            e.Initialize(Live);
    }
}
