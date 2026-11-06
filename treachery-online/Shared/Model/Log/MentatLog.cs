namespace Treachery.Shared;

// A faction moving the bribes it received into its reserves.
public record BribesCollected(Faction Faction, int Amount);

// A line saying why the game was won. When the Bene Gesserit prediction takes
// over a win, the earlier reason stays in the log and the prediction comes after it.
public record WinReason(WinMethod Method);

// "The game has ended.", with the factions that won.
public record GameEnded(IReadOnlyList<Faction> Winners);

// One "[faction] win!" line after the game has ended.
public record GameWinner(Faction Faction);
