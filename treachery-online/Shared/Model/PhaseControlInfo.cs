namespace Treachery.Shared;

/// <summary>
/// What the current pause between phases is waiting for. The server continues the pause once nobody is left to
/// wait for and any countdown has run out, with no player holding it. Sent to everyone in the game; every field
/// is public.
/// </summary>
public class PhaseControlInfo
{
    /// <summary>False when the host alone continues the current phase, as before.</summary>
    public bool Active { get; init; }

    /// <summary>Seats of the players who may still act or pass here.</summary>
    public int[] WaitingFor { get; init; } = [];

    /// <summary>Whether this pause runs a countdown that any player can hold.</summary>
    public bool Countdown { get; init; }

    /// <summary>When the countdown runs out, while it is running and nobody holds it.</summary>
    public DateTimeOffset? CountdownEnds { get; init; }

    /// <summary>Seats of the players holding the countdown.</summary>
    public int[] Holding { get; init; } = [];
}
