using System.Collections.Concurrent;
using System.Diagnostics;

namespace Treachery.Oracle;

/// <summary>Result of submitting one event to the engine.</summary>
public sealed class StepOutcome
{
    /// <summary>The Message returned by GameEvent.Execute (null when the event was executed).</summary>
    public Message? Result { get; init; }

    /// <summary>An exception that GameEvent.Execute caught and turned into a "Game Error" message, or that escaped Execute.</summary>
    public Exception? Error { get; init; }

    /// <summary>"validate" or "execute"; null when there was no exception.</summary>
    public string? ErrorPhase { get; init; }

    /// <summary>True when the exception escaped Execute (only possible for event types that override Execute).</summary>
    public bool Escaped { get; init; }

    /// <summary>True when Game.History grew, i.e. the event was executed to completion.</summary>
    public bool Accepted { get; init; }
}

public static class StepExecutor
{
    /// <summary>
    /// A Milestone value that does not exist in the enum. It is appended to Game.RecentMilestones before Execute
    /// and removed afterwards. GameEvent.Execute clears RecentMilestones exactly when it enters its execution branch,
    /// so the sentinel's survival tells us whether an exception came from validation or from execution.
    /// RecentMilestones is only read by GameStatus (client UI), never by validation.
    /// </summary>
    private const Milestone Sentinel = (Milestone)int.MinValue;

    private static readonly FieldInfo NeighbourCacheTimestamp =
        typeof(Map).GetField("_neighbourCacheTimestamp", BindingFlags.Instance | BindingFlags.NonPublic)!;

    private static readonly ConcurrentDictionary<Type, bool> OverridesExecuteCache = new();

    public static bool OverridesExecute(Type t) => OverridesExecuteCache.GetOrAdd(t, type =>
        type.GetMethod(nameof(GameEvent.Execute), [typeof(bool), typeof(bool)])!.DeclaringType != typeof(GameEvent));

    /// <summary>Marks Map's neighbour cache as stale, so it is rebuilt from the current state on first use.</summary>
    public static void InvalidateNeighbourCache(Game game) => NeighbourCacheTimestamp.SetValue(game.Map, -1);

    public static StepOutcome Run(Game game, GameEvent evt, bool performValidation, bool isHost)
    {
        InvalidateNeighbourCache(game);

        var historyBefore = game.History.Count;
        var overrides = OverridesExecute(evt.GetType());
        var milestones = game.RecentMilestones;
        if (!overrides) milestones.Add(Sentinel);

        Message? result = null;
        Exception? escaped = null;
        try
        {
            result = evt.Execute(performValidation, isHost);
        }
        catch (Exception e)
        {
            escaped = e;
        }

        var executionEntered = true;
        if (!overrides)
        {
            var idx = milestones.LastIndexOf(Sentinel);
            executionEntered = idx < 0;
            if (idx >= 0) milestones.RemoveAt(idx);
        }

        var error = escaped ?? FindException(result);
        string? phase = null;
        if (error != null)
        {
            if (escaped != null || overrides)
                phase = StackPhase(error);
            else
                phase = executionEntered ? "execute" : "validate";
        }

        return new StepOutcome
        {
            Result = result,
            Error = error,
            ErrorPhase = phase,
            Escaped = escaped != null,
            Accepted = game.History.Count > historyBefore
        };
    }

    private static Exception? FindException(Message? m)
    {
        if (m == null) return null;
        foreach (var e in m.Expression.Elements)
            if (e is Exception ex) return ex;
        return null;
    }

    /// <summary>Classifies an exception by the GameEvent method it passed through.</summary>
    private static string StackPhase(Exception e)
    {
        var frames = new StackTrace(e, false).GetFrames();
        foreach (var f in frames)
        {
            var m = f.GetMethod();
            if (m?.DeclaringType == null || !typeof(GameEvent).IsAssignableFrom(m.DeclaringType)) continue;
            if (m.Name is "Validate" or "IsApplicable") return "validate";
            if (m.Name is "ExecuteConcreteEvent") return "execute";
        }

        return "execute";
    }
}
