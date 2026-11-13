// "review-c" mode: probes for the package C review (setup events and their static
// helpers). Each line is "name = value"; tests/review_pkg_c.rs hardcodes the values.
//
//   dotnet run -c Release -- review-c

using Treachery.Shared;

public static class ReviewC
{
    private const string T = "\"Time\":\"2024-01-01T12:00:00+00:00\"";

    // Four players, latest version, no PlayersChooseFactions: factions are dealt at once.
    public const string Establish = "{\"$type\":\"EstablishPlayers\",\"Settings\":{\"NumberOfPlayers\":4,\"MaximumTurns\":5,\"InitialRules\":[998,400,1013],\"AllowedFactionsInPlay\":[20,50,60,70,110],\"AutoOpenEmptySeats\":true},\"Seed\":31337,\"Initiator\":0," + T + "}";
    public const string EndPhase = "{\"$type\":\"EndPhase\",\"Initiator\":0," + T + "}";

    private static string Try(Func<object?> f)
    {
        try
        {
            var r = f();
            return r == null ? "null" : r.ToString()!;
        }
        catch (Exception e)
        {
            return "exception:" + e.GetType().Name + ":" + e.Message.Replace("\n", "\\n");
        }
    }

    private static void P(string name, Func<object?> f) => Console.WriteLine($"{name} = {Try(f)}");

    private static string L<X>(IEnumerable<X> xs) => string.Join(",", xs.Select(x => x is Location l ? l.Id.ToString() : x?.ToString() ?? "null"));

    private static Game Run(params string[] events)
    {
        var g = new Game(Game.LatestVersion, new Participation());
        foreach (var json in events)
        {
            var e = Utilities.Deserialize<GameEvent>(json)!;
            e.Initialize(g);
            var r = e.Execute(true, true);
            if (r != null) throw new Exception("rejected: " + json + " " + r);
        }
        return g;
    }

    public static void Run()
    {
        var g = Run(Establish);
        P("factions", () => L(g.Players.Select(p => p.Faction)));
        P("phase", () => g.CurrentPhase);

        foreach (var p in g.Players)
        {
            P($"tradeTargets.{p.Faction}", () => L(FactionTradeOffered.ValidTargets(g, p)));
            P($"ruleTypes.{p.Faction}", () => L(AutomationConfigured.GetAvailableRuleTypes(g, p)));
            P($"biddingFactions.{p.Faction}", () => L(AutomationConfigured.GetValidBiddingFactions(g, p)));
            P($"hmsLocations.{p.Faction}", () => L(PerformHmsPlacement.ValidLocations(g, p)));
            P($"validLeaders.{p.Faction}", () => L(SkillAssigned.ValidLeaders(g, p).Select(l => l == null ? "null" : l.Id.ToString())));
        }

        P("validFactions", () => L(FactionSelected.ValidFactions(g)));
        P("moritaniLocations", () => L(PerformMoritaniSetup.ValidLocations(g)));
        P("beneGesseritLocations", () => L(PerformBeneGesseritPlacement.ValidLocations(g)));
        P("replaceTargets", () => L(PlayerReplaced.ValidTargets(g)));
        P("ruleGroups", () => L(EstablishPlayers.AvailableRuleGroups()));
        P("maxPlayers", () => EstablishPlayers.GetMaximumNumberOfPlayers());
        P("mustChooseSkills", () => SkillAssigned.PlayersMustChooseLeaderSkills(g));
        P("nextCustomSetup", () => g.NextFactionToPerformCustomSetup);

        // GetDescription for each rule type, for a player with and without special forces.
        foreach (var faction in new[] { Faction.Atreides, Faction.Ixian })
        foreach (var rt in Enum.GetValues<AutomationRuleType>().Append((AutomationRuleType)999))
        {
            var e = new AutomationConfigured(g, faction) { RuleType = rt, BiddingAboveAmount = 7, BiddingWinningFaction = Faction.BeneGesserit };
            P($"description.{faction}.{(int)rt}", () => e.GetDescription()?.ToString());
        }

        // The obsolete _factionsInPlay parser (Enum.Parse<Faction> per comma-separated part).
        foreach (var s in new[] { "Atreides, Harkonnen", " Atreides ", "Atreides,", "10", "-5", "+20", "0x10", "Atreides;Harkonnen", "Fremen,fremen", "1e1", " ", "Moritani,Ecaz,None" })
        {
            var ep = new EstablishPlayers { _factionsInPlay = s };
            P($"factionsInPlay[{s}]", () => L(ep.FactionsInPlay));
        }

        // The obsolete Players getter.
        foreach (var s in new[] { "", ">", "a>>b", "a" })
        {
            var ep = new EstablishPlayers { _players = s };
            P($"players[{s}]", () => string.Join("|", ep.Players.Select(x => "<" + x + ">")));
        }

        // GameInfo
        var gi = new GameInfo { Phase = Phase.AwaitingPlayers, MaxPlayers = 2, SeatedPlayers = new() { [1] = 0, [2] = 1 } };
        P("gameinfo.canBeJoined.full", () => gi.CanBeJoined);
        P("gameinfo.seat", () => gi.YourCurrentSeat(2) + "," + gi.YourCurrentSeat(9));
    }

    // "review-c validate-off": events executed with performValidation = false, as Game.Undo
    // and the client do. Each line is "name = value"; tests/review_pkg_c.rs compares them.
    public static void RunValidationOff()
    {
        // PerformHmsPlacement twice: a real target, then none. C# detaches the HMS from the
        // first target before PointAt throws on the null target.
        {
            var g = Run(Establish);
            var ixian = g.Players.First().Faction;
            P("hms.first", () => Exec(g, "{\"$type\":\"PerformHmsPlacement\",\"_targetId\":5,\"Initiator\":" + (int)ixian + "," + T + "}", false));
            var target = g.Map.LocationLookup.Find(5)!;
            P("hms.neighbourOfTarget.before", () => target.Neighbours.Contains(g.Map.HiddenMobileStronghold));
            P("hms.second", () => Exec(g, "{\"$type\":\"PerformHmsPlacement\",\"_targetId\":-1,\"Initiator\":" + (int)ixian + "," + T + "}", false));
            P("hms.neighbourOfTarget.after", () => target.Neighbours.Contains(g.Map.HiddenMobileStronghold));
            P("hms.attachedTo", () => g.Map.HiddenMobileStronghold.AttachedToLocation?.Id);
        }
    }

    // Executes one event. Returns "accepted", the validation message, or the exception type
    // and message of a "Game Error" result.
    private static string Exec(Game g, string json, bool validate)
    {
        var e = Utilities.Deserialize<GameEvent>(json)!;
        e.Initialize(g);
        var r = e.Execute(validate, true);
        if (r == null) return "accepted";
        var ex = r.Expression.Elements.OfType<Exception>().FirstOrDefault();
        if (ex != null && Environment.GetEnvironmentVariable("REVIEWC_TRACE") != null) Console.Error.WriteLine(ex.StackTrace);
        return ex != null ? "error:" + ex.GetType().Name + ":" + ex.Message : "rejected:" + r;
    }
}
