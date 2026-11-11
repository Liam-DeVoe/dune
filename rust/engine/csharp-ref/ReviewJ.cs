// "review-j" mode: probes for the package J review (faction specials: Karma, Nexus,
// Moritani, Ecaz, Choam, clairvoyance). Each line is "name = value";
// tests/review_pkg_j.rs hardcodes the values.
//
//   dotnet run -c Release -- review-j
//
// The probes put a null card into a hand. C# reaches that state through
// RicheseGaveCard with an unknown card id (its Validate always returns null and
// TreacheryCardManager.Lookup.Find(-1) is null), see probe 0.

using Treachery.Oracle.Canon;

public static class ReviewJ
{
    private static void P(string name, object? v) => Console.WriteLine($"{name} = {v ?? "null"}");

    // ReviewA.Establish with only Choam, Atreides and Richese allowed, so all three are seated.
    public const string Establish = """{"$type":"EstablishPlayers","Settings":{"NumberOfPlayers":3,"MaximumTurns":8,"InitialRules":[301,300,65,60,20,1013,15,700,105,80,85,604,90,998,30,701,40,107,10,70,50,25,209,999,112],"AllowedFactionsInPlay":[90,20,100],"AutoOpenEmptySeats":true,"AsyncPlay":false,"AsyncPlayMessageIntervalMinutes":0},"Seed":1073548723,"MaximumNumberOfPlayers":3,"MaximumTurns":8,"ApplicableRules":[301,300,65,60,20,1013,15,700,105,80,85,604,90,998,30,701,40,107,10,70,50,25,209,999,112],"_players":"","_factionsInPlay":"Choam,Atreides,Richese","Initiator":0,"Time":"2024-01-01T12:02:17+00:00"}""";

    private static Game NewGame()
    {
        var p = new Participation();
        p.PlayerNames[100] = "Human100";
        p.SeatedPlayers[100] = 2;
        var g = new Game(187, p);
        var e = Utilities.Deserialize<GameEvent>(Establish)!;
        e.Initialize(g);
        if (e.Execute(true, true) != null) throw new Exception("establish rejected");
        return g;
    }

    private static Message? Exec(Game g, string json, bool validate = true)
    {
        var e = Utilities.Deserialize<GameEvent>(json)!;
        e.Initialize(g);
        return e.Execute(validate, true);
    }

    private static void Try(string name, Func<object?> f)
    {
        try { P(name, f()); }
        catch (Exception ex)
        {
            P(name + ".exception", ex.GetType().Name + ": " + ex.Message);
            if (Environment.GetEnvironmentVariable("REVIEW_J_STACK") != null) Console.Error.WriteLine(ex.StackTrace);
        }
    }

    private static string Hand(Player p) => "[" + string.Join(",", p.TreacheryCards.Select(c => c == null ? "null" : c.Id.ToString())) + "]";

    private static string Hash(Game g)
    {
        var o = new JsonOut();
        new StateDumper(o).DumpGame(g);
        return StateDumper.Sha256Hex(o.WrittenSpan);
    }

    // Game.CurrentMainPhase and CurrentPhase have non-public setters. Rust sets the fields.
    private static void SetPhase(Game g, MainPhase main, Phase phase)
    {
        typeof(Game).GetProperty("CurrentMainPhase")!.SetValue(g, main);
        typeof(Game).GetProperty("CurrentPhase")!.SetValue(g, phase);
    }

    public static void Run(string[] args)
    {
        var info = args.Length > 1 && args[1] == "info";

        if (info)
        {
            var g = NewGame();
            P("info.phase", g.CurrentPhase);
            P("info.mainphase", g.CurrentMainPhase);
            foreach (var pl in g.Players) P("info.player", pl.Faction);
            foreach (var t in g.GetApplicableEvents(g.Players[0], false)) P("info.applicable0", t.Name);
            return;
        }

        var karama = TreacheryCardManager.GetCardsInAndOutsidePlay().First(c => c.Type == TreacheryCardType.Karma);
        var useless = TreacheryCardManager.GetCardsInAndOutsidePlay().First(c => c.Type == TreacheryCardType.Useless);
        P("karama.id", karama.Id);
        P("useless.id", useless.Id);

        // 1. Karma.ValidKarmaCards / CanKarma stop at the first match. A null card later in
        //    the hand is never looked at.
        {
            var g = NewGame();
            var p = g.Players[0];
            p.TreacheryCards.Clear();
            p.TreacheryCards.Add(karama);
            p.TreacheryCards.Add(null!);
            Try("karma_null.can_karma", () => Karma.CanKarma(g, p));
            Try("karma_null.first", () => Karma.ValidKarmaCards(g, p).FirstOrDefault()?.Id);
            Try("karma_null.has_karma", () => p.HasKarma(g));
        }

        // 2. NexusPlayed betrayal of Choam: Choam's hand is [null]. RandomOrDefault returns
        //    the null and C# skips the discard. Executed with validation.
        {
            var g = NewGame();
            var choam = g.GetPlayer(Faction.Choam)!;
            var actor = g.Players.First(x => x.Faction != Faction.Choam);
            actor.Nexus = Faction.Choam;
            choam.TreacheryCards.Clear();
            choam.TreacheryCards.Add(null!);
            P("nexus_choam.actor", actor.Faction);
            P("nexus_choam.phase", g.CurrentPhase);
            P("nexus_choam.applicable", NexusPlayed.IsApplicable(g, actor));
            var r = Exec(g, $$"""{"$type":"NexusPlayed","Faction":90,"Initiator":{{(int)actor.Faction}},"Time":"2024-01-01T12:02:40+00:00"}""");
            P("nexus_choam.result", r?.ToString());
            P("nexus_choam.history", g.History.Count);
            P("nexus_choam.choam_hand", Hand(choam));
            P("nexus_choam.discard_pile", g.TreacheryDiscardPile?.Items.Count);
            P("nexus_choam.actor_nexus", actor.Nexus);
            P("nexus_choam.hash", Hash(g));
        }

        // 3. ChoamDiscarded.ValidCards(p).Contains(card) stops at the first match.
        {
            var g = NewGame();
            var choam = g.GetPlayer(Faction.Choam)!;
            choam.TreacheryCards.Clear();
            choam.TreacheryCards.Add(useless);
            choam.TreacheryCards.Add(null!);
            Try("choam_discarded_null.contains", () => ChoamDiscarded.ValidCards(choam).Contains(useless));
            Try("choam_discarded_null.any", () => ChoamDiscarded.ValidCards(choam).Any());
            Try("nexus_valid_choam_cards.any", () => NexusPlayed.ValidChoamCards(choam).Any());
        }

        // 4. The same with validation on for every event. Richese gives its ally Choam an
        //    unknown card (RicheseGaveCard._cardId = -1), then Atreides betrays Choam with Nexus.
        {
            var g = NewGame();
            var richese = g.GetPlayer(Faction.Richese)!;
            var choam = g.GetPlayer(Faction.Choam)!;
            var atreides = g.GetPlayer(Faction.Atreides)!;
            richese.Ally = Faction.Choam;
            choam.Ally = Faction.Richese;
            richese.TreacheryCards.Clear();
            richese.TreacheryCards.Add(TreacheryCardManager.Get(48)!);
            choam.TreacheryCards.Clear();
            atreides.Nexus = Faction.Choam;
            SetPhase(g, MainPhase.Contemplate, Phase.Contemplate);
            P("richese_gave.applicable", string.Join(",", g.GetApplicableEvents(richese, false).Select(t => t.Name)));
            var r = Exec(g, """{"$type":"RicheseGaveCard","_cardId":-1,"Initiator":100,"Time":"2024-01-01T12:02:40+00:00"}""");
            P("richese_gave.result", r?.ToString());
            P("richese_gave.choam_hand", Hand(choam));
            P("richese_gave.richese_hand", Hand(richese));
            P("richese_gave.hash", Hash(g));
            Try("richese_gave.atreides_applicable", () => string.Join(",", g.GetApplicableEvents(atreides, false).Select(t => t.Name)));
            r = Exec(g, """{"$type":"NexusPlayed","Faction":90,"Initiator":20,"Time":"2024-01-01T12:02:41+00:00"}""");
            P("richese_gave.nexus_result", r?.ToString());
            P("richese_gave.nexus_history", g.History.Count);
            P("richese_gave.nexus_choam_hand", Hand(choam));
            P("richese_gave.nexus_atreides_nexus", atreides.Nexus);
            P("richese_gave.nexus_hash", Hash(g));
        }
    }
}
