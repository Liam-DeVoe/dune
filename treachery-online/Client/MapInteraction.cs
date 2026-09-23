using Microsoft.AspNetCore.Components;

namespace Treachery.Client;

// The map-driven selection for the action that currently uses the map. One
// game event component owns it at a time and pushes what may be clicked, what
// has been clicked, and what to show at the pinned destination. The map layers
// only read it and redraw when Changed fires.
public class MapInteraction
{
    public event Action? Changed;

    public object? Owner { get; private set; }

    public Faction Faction { get; set; }

    // Locations the owner accepts as a source. Drawn with a light tint.
    public HashSet<Location> Selectable { get; } = [];

    // Sources the player has clicked, in click order.
    public List<Location> Selected { get; } = [];

    // Destinations that are valid for the selected sources.
    public HashSet<Location> Targets { get; } = [];

    // The destination the player clicked, if any.
    public Location? Target { get; set; }

    // Where arrows start when nothing on the map is selected, such as the
    // player's token on the map's edge for a shipment from reserves.
    public PointD? Origin { get; set; }

    // Where arrows end when no destination is pinned, such as the player's
    // token for a shipment back to reserves.
    public PointD? EndPoint { get; set; }

    // Content shown next to the pinned arrow, usually a force counter and a confirm button.
    public RenderFragment? Pin { get; set; }

    // Draw an arrow from each selected source to the last target the cursor
    // was over while no destination is pinned.
    public bool ShowCursorArrow { get; set; } = true;

    // Forces the owner has placed on the map but not yet confirmed, drawn as
    // ghost tokens next to the real ones.
    public Dictionary<Location, Battalion> Staged { get; } = [];

    // What the owner does when the player cancels with Escape or a right
    // click on the map, usually dropping the selected sources and destination.
    public Action? OnCancel { get; set; }

    // Whether there is a half-made selection to cancel.
    public bool CanCancel => Owner != null && OnCancel != null && (Selected.Count > 0 || Target != null);

    public void Cancel()
    {
        if (CanCancel) OnCancel!();
    }

    public bool IsFree => Owner is null;

    public bool IsOwnedBy(object candidate) => ReferenceEquals(Owner, candidate);

    public void Claim(object owner)
    {
        Owner = owner;
        Reset();
        Changed?.Invoke();
    }

    public void Release(object owner)
    {
        if (!IsOwnedBy(owner)) return;
        Owner = null;
        Reset();
        Changed?.Invoke();
    }

    public void Refresh() => Changed?.Invoke();

    private void Reset()
    {
        Selectable.Clear();
        Selected.Clear();
        Targets.Clear();
        Target = null;
        Origin = null;
        EndPoint = null;
        Pin = null;
        ShowCursorArrow = true;
        Staged.Clear();
        OnCancel = null;
    }
}
