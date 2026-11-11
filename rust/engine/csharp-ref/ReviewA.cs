// "review-a" mode: probes for the package A review (Game.cs, GameEvent.cs).
// Each line is "name = value"; tests/review_pkg_a.rs hardcodes the values.
//
//   dotnet run -c Release -- review-a

using Treachery.Shared;

public static class ReviewA
{
    // Steps 1 and 2 of rust/traces/sample/fuzz-2026-00018.jsonl.gz.
    public const string Establish = """{"$type":"EstablishPlayers","Settings":{"NumberOfPlayers":3,"MaximumTurns":8,"InitialRules":[301,300,65,60,20,1013,15,700,105,80,85,604,90,998,30,701,40,107,10,70,50,25,209,999,112],"AllowedFactionsInPlay":[50,10,120,60,40,100,20,90,70,80,30,110],"AutoOpenEmptySeats":true,"AsyncPlay":false,"AsyncPlayMessageIntervalMinutes":0},"Seed":1073548723,"MaximumNumberOfPlayers":3,"MaximumTurns":8,"ApplicableRules":[301,300,65,60,20,1013,15,700,105,80,85,604,90,998,30,701,40,107,10,70,50,25,209,999,112],"_players":"","_factionsInPlay":"Guild,Fremen,Moritani,BeneGesserit,Emperor,Richese,Atreides,Choam,Ixian,Tleilaxu,Harkonnen,Ecaz","Initiator":0,"Time":"2024-01-01T12:02:17+00:00"}""";
    public const string EndPhase = """{"$type":"EndPhase","Initiator":0,"Time":"2024-01-01T12:02:32+00:00"}""";

    private static Participation TraceParticipation()
    {
        var p = new Participation();
        p.PlayerNames[100] = "Human100";
        p.SeatedPlayers[100] = 2;
        return p;
    }

    private static void P(string name, object? v) => Console.WriteLine($"{name} = {v ?? "null"}");

    private static Game Run(params string[] events)
    {
        var g = new Game(187, TraceParticipation());
        foreach (var json in events)
        {
            var e = Utilities.Deserialize<GameEvent>(json)!;
            e.Initialize(g);
            var r = e.Execute(true, true);
            if (r != null) throw new Exception("rejected: " + json);
        }
        return g;
    }

    public static void Run()
    {
        // 1. After Undo, RecentlyUndoneEvents still belong to the old game.
        {
            var g = Run(Establish, EndPhase);
            var u = g.Undo(1);
            var ev = u.RecentlyUndoneEvents[0];
            P("undo.recentlyUndone.count", u.RecentlyUndoneEvents.Count);
            P("undo.recentlyUndone[0].type", ev.GetType().Name);
            P("undo.recentlyUndone[0].game_is_result", ReferenceEquals(ev.Game, u));
            P("undo.recentlyUndone[0].game_is_old", ReferenceEquals(ev.Game, g));
            P("undo.players.count", u.Players.Count);
        }

        // 2. After Undo + RestoreRecentlyUndone, the restored (old) game's history
        //    events point at the undo game and its players.
        {
            var g = Run(Establish, EndPhase);
            var u = g.Undo(1);
            var r = u.RestoreRecentlyUndone();
            P("restore.returns_old", ReferenceEquals(r, g));
            P("restore.history[0].game_is_restored", ReferenceEquals(r.History[0].Game, r));
            P("restore.history[0].game_is_undo", ReferenceEquals(r.History[0].Game, u));
            P("restore.history[1].game_is_restored", ReferenceEquals(r.History[1].Game, r));
        }

        // 3. Participation is shared between a game and its Undo result, so a
        //    seat change after Undo shows up in the game RestoreRecentlyUndone returns.
        {
            var g = new Game(187, new Participation());
            var u = g.Undo(0);
            u.AddPlayer(5, "Late", 1);
            var r = u.RestoreRecentlyUndone();
            P("participation.restore_returns_old", ReferenceEquals(r, g));
            P("participation.shared", ReferenceEquals(r.Participation, u.Participation));
            P("participation.old.isParticipant5", r.IsParticipant(5));
            P("participation.old.seated", r.NumberOfSeatedPlayers);
        }

        // 4. The caller's Participation object is the game's.
        {
            var part = new Participation();
            var g = new Game(187, part);
            part.Hosts.Add(7);
            P("participation.caller_alias.isHost7", g.IsHost(7));
        }
    }
}
