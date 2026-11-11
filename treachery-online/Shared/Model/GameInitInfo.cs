namespace Treachery.Shared;

public class GameInitInfo
{
    public string GameId { get; init; }
    public string GameState { get; init; }
    public string GameName { get; init; }
    public Participation Participation { get; init; }

    /// <summary>What the current pause between phases waits for.</summary>
    public PhaseControlInfo? PhaseControl { get; init; }
}