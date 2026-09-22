/*
 * Copyright (C) 2020-2025 Ronald Ossendrijver (admin@treachery.online)
 * This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License
 * as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version. This
 * program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details. You should have
 * received a copy of the GNU General Public License along with this program. If not, see <http://www.gnu.org/licenses/>.
 */

using System.Linq;
using Treachery.Client.GenericComponents;

namespace Treachery.Client.GameEventComponents;

// Setup deployment made on the map: a click stages one force in a sector, a
// shift-click stages one special force, a right click takes the last one
// back. The staged forces are drawn on the map until the player confirms.
public abstract class StagedPlacementComponent<T> : GameEventComponent<T> where T : GameEvent, new()
{
    // The razor base classes get their own injected copy; a plain C# class has to ask for one.
    [Microsoft.AspNetCore.Components.Inject]
    protected IGameService Client { get; set; } = null!;

    protected readonly Dictionary<Location, Battalion> forces = new();

    // Order of staging, so a right click can undo the most recent one in a sector.
    private readonly List<(Location location, bool special)> history = [];

    private MapInteraction? localMap;
    private HashSet<Location>? targets;

    // The player whose forces are being placed. This is not always the
    // player at the keyboard: the host places everyone's forces in a custom setup.
    protected abstract Player Placer { get; }

    protected abstract IEnumerable<Location> ValidLocations { get; }

    protected virtual bool MayStageSpecial => Placer.HasSpecialForces;

    // Cap on regular plus special forces staged together, when the rules fix
    // the total. The reserve caps apply on top of this.
    protected virtual int MaxTotal => int.MaxValue;

    protected MapInteraction Map => Client.MapInteraction ?? (localMap ??= new MapInteraction());

    protected bool OwnsMap => Map.IsOwnedBy(this);

    protected IEnumerable<KeyValuePair<Location, Battalion>> Staged =>
        forces.Where(kvp => kvp.Value.TotalAmountOfForces > 0).OrderBy(kvp => Name(kvp.Key));

    protected int StagedForces => forces.Values.Sum(b => b.AmountOfForces);

    protected int StagedSpecialForces => forces.Values.Sum(b => b.AmountOfSpecialForces);

    // How many more forces of the given type may be staged right now.
    protected int Room(bool special)
    {
        var reserve = special ? Placer.SpecialForcesInReserve - StagedSpecialForces : Placer.ForcesInReserve - StagedForces;
        var total = MaxTotal - StagedForces - StagedSpecialForces;
        return Math.Max(0, Math.Min(reserve, total));
    }

    private HashSet<Location> Targets => targets ??= ValidLocations.ToHashSet();

    protected override void OnAfterRender(bool firstRender)
    {
        if (!firstRender) return;

        Client.OnLocationSelected += HandleAdd;
        Client.OnLocationSelectedWithShift += HandleAddSpecial;
        Client.OnLocationRightClicked += HandleRemove;
        Map.Changed += HandleMapChanged;
        ClaimIfFree();
    }

    public override void Dispose()
    {
        base.Dispose();
        Client.OnLocationSelected -= HandleAdd;
        Client.OnLocationSelectedWithShift -= HandleAddSpecial;
        Client.OnLocationRightClicked -= HandleRemove;
        Map.Changed -= HandleMapChanged;
        Map.Release(this);
    }

    // Drops everything staged, for when the faction being set up changes.
    protected void ResetStaging()
    {
        forces.Clear();
        history.Clear();
        targets = null;
        if (OwnsMap) PushToMap();
    }

    protected void Add(Location l, bool special)
    {
        if (!Targets.Contains(l)) return;
        if (special && !MayStageSpecial) return;

        // Nobody can place more than they hold in reserve, or beyond the fixed total.
        if (Room(special) == 0) return;

        if (!forces.TryGetValue(l, out var battalion))
        {
            battalion = new Battalion(Placer.Faction, 0, 0, l);
            forces[l] = battalion;
        }

        if (special) battalion.ChangeSpecialForces(1);
        else battalion.ChangeForces(1);
        history.Add((l, special));
        PushToMap();
        StateHasChanged();
    }

    // Takes back the most recent force staged in the sector. With a type given,
    // the most recent one of that type.
    protected void Remove(Location l, bool? special = null)
    {
        var index = history.FindLastIndex(h => h.location == l && (special == null || h.special == special));
        if (index < 0) return;

        var (_, wasSpecial) = history[index];
        history.RemoveAt(index);

        var battalion = forces[l];
        if (wasSpecial) battalion.ChangeSpecialForces(-1);
        else battalion.ChangeForces(-1);
        if (battalion.TotalAmountOfForces == 0) forces.Remove(l);

        PushToMap();
        StateHasChanged();
    }

    private void ClaimIfFree()
    {
        if (!Map.IsFree) return;
        Map.Claim(this);
        PushToMap();
    }

    private void HandleMapChanged()
    {
        if (Map.IsFree)
        {
            ClaimIfFree();
        }
        else if (!OwnsMap && forces.Count > 0)
        {
            // Another action took the map; what was staged would be stale.
            forces.Clear();
            history.Clear();
        }
        StateHasChanged();
    }

    private void PushToMap()
    {
        if (!OwnsMap) return;
        Map.Faction = Placer.Faction;
        Map.Selectable.Clear();
        Map.Selected.Clear();
        Map.Target = null;
        Map.Pin = null;
        Map.ShowCursorArrow = false;
        Map.Targets.Clear();
        foreach (var l in Targets) Map.Targets.Add(l);
        Map.Staged.Clear();
        foreach (var kvp in forces) Map.Staged[kvp.Key] = kvp.Value;
        Map.Refresh();
    }

    private void HandleAdd(object? sender, Location l)
    {
        if (OwnsMap) Add(l, false);
    }

    private void HandleAddSpecial(object? sender, Location l)
    {
        if (OwnsMap) Add(l, true);
    }

    private void HandleRemove(object? sender, Location l)
    {
        if (OwnsMap) Remove(l);
    }
}
