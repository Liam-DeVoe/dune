namespace Treachery.Shared;

// A faction collecting spice in the Collection phase. Field is the location
// the spice lay on when it was collected from the desert, so the log can name
// the sector when two fields in one territory paid out; it is null for a
// divided collection. Income is spice paid by a stronghold (Increased Spice
// Flow) or an occupied homeworld rather than collected from a field.
public record SpiceCollected(Faction Faction, Territory Territory, int Amount, Location? Field, bool Income);
