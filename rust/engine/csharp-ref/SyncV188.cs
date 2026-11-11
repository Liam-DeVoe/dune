// "sync-v188" mode: the C# side of tests/sync_v188.rs, which checks the rules
// added in game version 188 (the alliance phase that ends when everyone is done,
// AlliancePassed, PrescienceSeenPlans) on both sides of the version check.
//
//   dotnet run -c Release -- sync-v188 ../tests/golden/sync_v188.txt
//
// Each line is "name = value". The Rust test runs the same steps and compares
// every line. The game is ReviewJ.Establish: Choam, Atreides and Richese.

public static class SyncV188
{
    private static readonly List<string> Lines = [];

    private static void P(string name, object? v) => Lines.Add($"{name} = {v ?? "null"}");

    private static void Try(string name, Func<object?> f)
    {
        try { P(name, f()); }
        catch (TargetInvocationException ex) when (ex.InnerException != null) { P(name, "exception " + ex.InnerException.GetType().Name + ": " + ex.InnerException.Message); }
        catch (Exception ex) { P(name, "exception " + ex.GetType().Name + ": " + ex.Message); }
    }

    private static void Set(object target, string property, object? value) =>
        target.GetType().GetProperty(property, BindingFlags.Instance | BindingFlags.Public | BindingFlags.NonPublic)!.SetValue(target, value);

    private static object? Call(object target, string method, params object?[] args) =>
        target.GetType().GetMethod(method, BindingFlags.Instance | BindingFlags.Public | BindingFlags.NonPublic)!.Invoke(target, args);

    private static Game NewGame(int version)
    {
        var p = new Participation();
        p.PlayerNames[100] = "Human100";
        p.SeatedPlayers[100] = 2;
        var g = new Game(version, p);
        var e = Utilities.Deserialize<GameEvent>(ReviewJ.Establish)!;
        e.Initialize(g);
        if (e.Execute(true, true) != null) throw new Exception("establish rejected");
        return g;
    }

    private static string Factions(IEnumerable<Faction> fs) => "[" + string.Join(",", fs) + "]";

    private static string Applicable(Game g)
    {
        var parts = new List<string> { "host:" + string.Join("+", g.GetApplicableEvents(null, true).Select(t => t.Name)) };
        foreach (var p in g.Players)
            parts.Add(p.Faction + ":" + string.Join("+", g.GetApplicableEvents(p, false).Select(t => t.Name)));
        return string.Join(" ", parts);
    }

    private static void State(Game g, string at)
    {
        P(at + ".phase", g.CurrentPhase);
        P(at + ".acted", Factions(g.HasActedOrPassed));
        P(at + ".offers", string.Join(",", g.CurrentAllianceOffers.Select(o => o.Initiator + ">" + o.Target)));
        P(at + ".allies", string.Join(",", g.Players.Select(p => p.Faction + ":" + p.Ally)));
        P(at + ".nexus_occured", typeof(Game).GetProperty("NexusHasOccured", BindingFlags.Instance | BindingFlags.Public | BindingFlags.NonPublic)!.GetValue(g));
        P(at + ".applicable", Applicable(g));
        foreach (var p in g.Players)
        {
            var s = GameStatus.DetermineStatus(g, p, true);
            P(at + ".status." + p.Faction, s.GetMessage(p, false) + " | waiting " + Factions(s.WaitingForPlayers.Select(w => w.Faction)));
        }
    }

    private static void Exec(Game g, string at, GameEvent e)
    {
        e.Initialize(g);
        Try(at + ".applicable", () => e.IsApplicable(false));
        Try(at + ".validate", () => e.Validate()?.ToString() ?? "valid");
        Try(at + ".message", () => e.GetMessage().ToString());
        // Without the applicability check, so the v187 game runs the same steps.
        Try(at + ".execute", () => e.Execute(false, true)?.ToString() ?? "ok");
        State(g, at);
    }

    public static void Run(string output)
    {
        foreach (var v in new[] { 187, 188 })
        {
            AlliancePhase(v);
            BreakInAlliancePhase(v);
            Prescience(v);
        }
        File.WriteAllLines(output, Lines);
        Console.WriteLine($"wrote {Lines.Count} lines to {output}");
    }

    // Choam offers to Atreides, Richese passes, Atreides offers to Richese (rejected since v188
    // because Richese passed), Atreides accepts Choam: everyone is done.
    private static void AlliancePhase(int v)
    {
        var at = $"v{v}.phase";
        var g = NewGame(v);
        g.HasActedOrPassed.Add(Faction.Atreides);
        Call(g, "EnterAlliancePhase", true);
        State(g, at + ".enter");

        Exec(g, at + ".choam_offers", new AllianceOffered(g, Faction.Choam) { Target = Faction.Atreides });
        Exec(g, at + ".richese_passes", new AlliancePassed(g, Faction.Richese));
        Exec(g, at + ".atreides_offers_richese", new AllianceOffered(g, Faction.Atreides) { Target = Faction.Richese });
        Exec(g, at + ".atreides_accepts", new AllianceOffered(g, Faction.Atreides) { Target = Faction.Choam });

        // KarmaMonster in BlowReport enters AllianceB through EnterAlliancePhase.
        var g2 = NewGame(v);
        g2.HasActedOrPassed.Add(Faction.Richese);
        Call(g2, "EnterAlliancePhase", false);
        State(g2, at + ".enter_b");
        Exec(g2, at + ".b_choam_passes", new AlliancePassed(g2, Faction.Choam));
        Exec(g2, at + ".b_atreides_passes", new AlliancePassed(g2, Faction.Atreides));
        Exec(g2, at + ".b_richese_passes", new AlliancePassed(g2, Faction.Richese));
    }

    // Choam and Atreides are allied. Atreides passes (keeps the alliance), then Choam breaks it:
    // since v188 Atreides may act again.
    private static void BreakInAlliancePhase(int v)
    {
        var at = $"v{v}.break";
        var g = NewGame(v);
        g.GetPlayer(Faction.Choam)!.Ally = Faction.Atreides;
        g.GetPlayer(Faction.Atreides)!.Ally = Faction.Choam;
        Call(g, "EnterAlliancePhase", true);
        Exec(g, at + ".richese_offers_atreides", new AllianceOffered(g, Faction.Richese) { Target = Faction.Atreides });
        Exec(g, at + ".atreides_passes", new AlliancePassed(g, Faction.Atreides));
        Exec(g, at + ".choam_breaks", new AllianceBroken(g, Faction.Choam));
        Exec(g, at + ".atreides_offers_richese", new AllianceOffered(g, Faction.Atreides) { Target = Faction.Richese });
    }

    private static string Seen(Game g, Battle?[] plans) => "[" + string.Join(",", g.PrescienceSeenPlans.Select(kv =>
        $"{kv.Key}:{kv.Value.Initiator}:plan{Array.IndexOf(plans, kv.Value)}")) + "]";

    // Atreides (aggressor) fights Choam in Arrakeen. Atreides's prescience sees part of Choam's plan;
    // Choam revises, and its next plan must keep what Atreides saw (since v188).
    private static void Prescience(int v)
    {
        var at = $"v{v}.prescience";
        var g = NewGame(v);
        var choam = g.GetPlayer(Faction.Choam)!;
        var weapons = TreacheryCardManager.GetCardsInAndOutsidePlay().Where(c => c.IsProjectileWeapon).Take(2).ToArray();
        var defense = TreacheryCardManager.GetCardsInAndOutsidePlay().First(c => c.IsProjectileDefense);
        P(at + ".cards", string.Join(",", weapons.Select(c => c.Id)) + "," + defense.Id);
        choam.TreacheryCards.Clear();
        choam.TreacheryCards.AddRange(weapons);
        choam.TreacheryCards.Add(defense);
        P(at + ".leaders_before", choam.Leaders.Count);
        choam.Leaders.AddRange(LeaderManager.GetLeaders(Faction.Choam));
        var leader0 = choam.Leaders[0];
        var leader1 = choam.Leaders[1];
        P(at + ".leaders", leader0.Id + "," + leader1.Id);
        choam.AddForces(g.Map.Arrakeen, 3, false);
        var battle = new BattleInitiated(g, Faction.Atreides) { Target = Faction.Choam, Territory = g.Map.Arrakeen.Territory };
        Set(g, "CurrentBattle", battle);

        Battle Plan(IHero hero, TreacheryCard? weapon, TreacheryCard? def, int forcesAtHalfStrength)
        {
            var b = new Battle(g, Faction.Choam) { Hero = hero, Weapon = weapon, Defense = def, ForcesAtHalfStrength = forcesAtHalfStrength };
            return b;
        }

        var plans = new Battle?[6];
        plans[0] = Plan(leader0, weapons[0], defense, 0);
        Try(at + ".plan0.validate", () => plans[0]!.Validate()?.ToString() ?? "valid");
        Set(g, "DefenderPlan", plans[0]);

        // Atreides sees Choam's weapon.
        Set(g, "CurrentPrescience", new Prescience(g, Faction.Atreides) { Aspect = PrescienceAspect.Weapon });
        Try(at + ".aspects", () => string.Join(",", Battle.AspectsSeenByOpponent(g, choam)));
        Try(at + ".aspects_of_atreides", () => string.Join(",", Battle.AspectsSeenByOpponent(g, g.GetPlayer(Faction.Atreides)!)));
        var revision = new BattleRevision(g, Faction.Choam);
        Try(at + ".revise1", () => revision.Execute(false, true)?.ToString() ?? "ok");
        P(at + ".revise1.seen", Seen(g, plans));
        P(at + ".revise1.defender_plan_is_null", g.DefenderPlan == null);

        plans[1] = Plan(leader0, weapons[1], defense, 0);
        Try(at + ".other_weapon.validate", () => plans[1]!.Validate()?.ToString() ?? "valid");
        plans[2] = Plan(leader1, weapons[0], null, 0);
        Try(at + ".same_weapon.validate", () => plans[2]!.Validate()?.ToString() ?? "valid");

        // Choam submits plan 2 and revises again; this time Atreides also has karma prescience,
        // so it sees every aspect. Weapon is re-set in place, the others are added in order.
        Set(g, "DefenderPlan", plans[2]);
        Set(g, "AtreidesKarma", true);
        Set(g, "CurrentPrescience", new Prescience(g, Faction.Atreides) { Aspect = PrescienceAspect.Dial });
        Try(at + ".aspects_karma", () => string.Join(",", Battle.AspectsSeenByOpponent(g, choam)));
        Try(at + ".revise2", () => new BattleRevision(g, Faction.Choam).Execute(false, true)?.ToString() ?? "ok");
        P(at + ".revise2.seen", Seen(g, plans));

        plans[3] = Plan(leader1, weapons[0], null, 0);
        Try(at + ".identical.validate", () => plans[3]!.Validate()?.ToString() ?? "valid");
        plans[4] = Plan(leader0, weapons[0], defense, 0);
        Try(at + ".leader_and_defense_changed.validate", () => plans[4]!.Validate()?.ToString() ?? "valid");
        plans[5] = Plan(leader1, weapons[0], null, 1);
        Try(at + ".dial_changed.validate", () => plans[5]!.Validate()?.ToString() ?? "valid");

        // A revision with no plan on file records nothing.
        Set(g, "DefenderPlan", null);
        Try(at + ".revise_without_plan", () => new BattleRevision(g, Faction.Choam).Execute(false, true)?.ToString() ?? "ok");
        P(at + ".revise_without_plan.seen", Seen(g, plans));

        // Richese is not in this battle: OpponentOf(Richese) is null, at every version.
        var richese = new Battle(g, Faction.Richese) { Hero = null, Weapon = null, Defense = null };
        Try(at + ".outsider.validate", () => richese.Validate()?.ToString() ?? "valid");

        // EndPhase.ResetBattle clears the seen plans.
        Try(at + ".reset", () => { Call(new EndPhase(g, Faction.Choam), "ResetBattle"); return "ok"; });
        P(at + ".reset.seen", Seen(g, plans));
    }
}
