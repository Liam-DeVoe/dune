namespace Treachery.Shared;

// The two storm dials, in the order they were logged.
public record StormDials(Faction First, int FirstDial, Faction Second, int SecondDial);

// The first storm landing in a sector (turn 1).
public record StormPlaced(int Sector);

// The distance the storm will move, announced before cards may change it.
public record StormForecast(int Distance);

// The storm moving Distance sectors from the sector it stood in. By and Card
// are set when a player moved it with Weather Control.
public record StormMoved(int From, int Distance, Faction By = Faction.None, TreacheryCard? Card = null);

// Forces the storm killed in a location. Also logged when a faction chose its
// storm losses (Fremen losing half, Choam).
public record StormKilled(Faction Faction, Location Location, int Forces, int SpecialForces);

// Spice the storm destroyed in a location.
public record StormDestroyedSpice(Location Location, int Amount);

// A player using Family Atomics on the Shield Wall.
public record FamilyAtomicsUsed(Faction Faction, TreacheryCard? Card);

// Forces Family Atomics killed on the Shield Wall.
public record FamilyAtomicsKilled(Faction Faction, int Forces, int SpecialForces);

// Bene Gesserit advisors flipping to fighters because they are alone.
public record BeneGesseritFlipped(Territory Territory);
