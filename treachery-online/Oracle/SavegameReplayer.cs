using Treachery.Oracle.Canon;

namespace Treachery.Oracle;

/// <summary>Replays a GameState savegame event by event, recording a trace.</summary>
public static class SavegameReplayer
{
    public static string Run(string input, string output, DumpPolicy policy)
    {
        var state = GameState.Load(File.ReadAllText(input));
        var participation = new Participation();
        var game = new Game(state.Version, participation);

        using var rec = new Recorder(output, policy);
        rec.Header(o =>
        {
            o.Comma(); o.Key("mode"); o.WriteString("replay");
            o.Comma(); o.Key("source"); o.WriteString(Path.GetFileName(input));
            o.Comma(); o.Key("gameVersion"); o.Int(state.Version);
            o.Comma(); o.Key("participation"); new StateDumper(o).DumpDetached(participation, null);
        });

        var rejected = 0;
        foreach (var original in state.Events)
        {
            var json = Utilities.Serialize<GameEvent>(original);
            var evt = Utilities.Deserialize<GameEvent>(json)!;
            evt.Initialize(game);
            var outcome = StepExecutor.Run(game, evt, true, true);
            if (!outcome.Accepted) rejected++;

            string? after;
            try
            {
                after = Utilities.Serialize<GameEvent>(evt);
            }
            catch
            {
                after = null;
            }

            rec.Step(new StepInfo { Source = "replay", Actor = null, PerformValidation = true, IsHost = true, EventJson = json }, outcome, game, after, null);
        }

        var reason = rejected == 0 ? "replayed" : "replayedWithRejections";
        rec.End(reason, rejected == 0 ? null : rejected + " events not accepted", game);
        return reason;
    }
}
