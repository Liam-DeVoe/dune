using System.Linq;
using Microsoft.AspNetCore.Components;

namespace Treachery.Client;

// Drives the map for an action that picks one location on it, optionally after
// picking some source sectors. The owning component says what may be clicked
// and what the pinned panel shows; the picker claims the shared MapInteraction
// on the component's behalf and hands it back when the component goes away or
// no longer needs the map.
public sealed class MapPicker : IDisposable
{
    private readonly IGameService _client;
    private MapInteraction? _localMap;
    private bool _started;
    private bool _owned;

    public MapPicker(IGameService client)
    {
        _client = client;
    }

    // Sectors that may be clicked as a source. Leave empty for a plain pick.
    public Func<IEnumerable<Location>> Sources { get; set; } = () => [];

    // Locations that may be clicked as the destination, given the current sources.
    public Func<IEnumerable<Location>> Targets { get; set; } = () => [];

    // Whether the map is needed right now. False hands it back to whoever wants it.
    public Func<bool> Enabled { get; set; } = () => true;

    // When the choice is a territory, clicking any sector of the pinned
    // territory unpins it instead of re-pinning at that sector.
    public Func<bool> ByTerritory { get; set; } = () => false;

    // Content shown next to the pinned destination.
    public Func<RenderFragment?> Pin { get; set; } = () => null;

    // Called after a click changed the selection or the map changed hands.
    public Action? Changed { get; set; }

    public List<Location> Selected { get; } = [];

    public Location? Target { get; private set; }

    // Tests render components without a real map; a private one keeps the protocol the same.
    private MapInteraction Map => _client.MapInteraction ?? (_localMap ??= new MapInteraction());

    public bool OwnsMap => Map.IsOwnedBy(this);

    // Call after every render. Claims the map when it is needed and free, and
    // releases it when it is no longer needed. While owning, redraws the map
    // so the pinned panel reflects the owner's latest state.
    public void Sync()
    {
        if (!_started)
        {
            _started = true;
            _client.OnLocationSelected += HandleLocationSelected;
            Map.Changed += HandleMapChanged;
        }

        if (Enabled())
        {
            if (OwnsMap) Push();
            else if (Map.IsFree) Claim();
        }
        else if (OwnsMap)
        {
            Selected.Clear();
            Target = null;
            Map.Release(this);
        }
    }

    // Forgets the selection, for instance when the game state was replaced.
    public void Reset()
    {
        Selected.Clear();
        Target = null;
        if (OwnsMap) Push();
    }

    public void Clear()
    {
        Selected.Clear();
        Target = null;
        Push();
        Changed?.Invoke();
    }

    public void Push()
    {
        if (!OwnsMap) return;

        var sources = Sources().ToHashSet();
        Selected.RemoveAll(l => !sources.Contains(l));
        var targets = Targets().ToHashSet();
        if (Target != null && !targets.Contains(Target)) Target = null;

        Map.Faction = _client.Faction ?? Faction.None;
        Map.Selectable.Clear();
        Map.Selectable.UnionWith(sources);
        Map.Selected.Clear();
        Map.Selected.AddRange(Selected);
        Map.Targets.Clear();
        Map.Targets.UnionWith(targets);
        Map.Target = Target;
        Map.ShowCursorArrow = Selected.Count > 0;
        Map.Pin = Target != null ? Pin() : null;
        Map.OnCancel = Clear;
        Map.Refresh();
    }

    public void Dispose()
    {
        if (!_started) return;
        _client.OnLocationSelected -= HandleLocationSelected;
        Map.Changed -= HandleMapChanged;
        Map.Release(this);
    }

    private void Claim()
    {
        Map.Claim(this);
        Push();
    }

    private void HandleMapChanged()
    {
        var owns = OwnsMap;
        if (!owns)
        {
            if (Map.IsFree && Enabled())
            {
                Claim();
                owns = true;
            }
            else if (Selected.Count > 0 || Target != null)
            {
                // Another action took the map; a half-made selection would be stale.
                Selected.Clear();
                Target = null;
            }
        }

        if (owns != _owned)
        {
            _owned = owns;
            Changed?.Invoke();
        }
    }

    private void HandleLocationSelected(object? sender, Location l)
    {
        if (!OwnsMap) return;

        if (Selected.Contains(l))
        {
            Selected.Remove(l);
            Target = null;
        }
        else if (Sources().Contains(l))
        {
            // Sources are sectors of one territory; a click elsewhere starts over.
            if (Selected.Count > 0 && Selected[0].Territory != l.Territory) Selected.Clear();
            Selected.Add(l);
            Target = null;
        }
        else if (Target != null && (Target == l || (ByTerritory() && Target.Territory == l.Territory)))
        {
            Target = null;
        }
        else if (Targets().Contains(l))
        {
            Target = l;
        }
        else
        {
            return;
        }

        Push();
        Changed?.Invoke();
    }
}
