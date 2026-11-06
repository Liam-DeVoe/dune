/*
 * Copyright (C) 2020-2025 Ronald Ossendrijver (admin@treachery.online)
 * This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License
 * as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version. This
 * program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details. You should have
 * received a copy of the GNU General Public License along with this program. If not, see <http://www.gnu.org/licenses/>.
 */

namespace Treachery.Shared;

public enum AsideKind
{
    // A card play, a deal, a bribe: its own aside.
    Aside,

    // The answer to a Truthtrance question. Joins the question's aside when it
    // follows it directly.
    Answer,

    // The end of an earlier effect ("... is no longer prevented"): a small note.
    After
}

// Marks a message that can happen in any phase (design/battle-log/log-matrix.html,
// "Any phase"). The report draws it as an aside, whatever phase it is in.
public record AsideLog(AsideKind Kind = AsideKind.Aside)
{
    public static readonly AsideLog Aside = new();
    public static readonly AsideLog Answer = new(AsideKind.Answer);
    public static readonly AsideLog After = new(AsideKind.After);
}
