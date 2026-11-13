// "review-i" mode: probes for the package I review (battle actions around the fight).
// Replays the first steps of rust/traces/sample/bots-2026-00042 (stored as
// tests/review_pkg_i/bots42.jsonl), then executes hand-made events the bots never
// play and prints the result and the state hash. tests/review_pkg_i.rs hardcodes them.
//
//   dotnet run -c Release -- review-i [info]

using System.Globalization;
using System.Security.Cryptography;
using System.Text.Json;
using Treachery.Oracle.Canon;

public static class ReviewI
{
    private static readonly FieldInfo NeighbourCacheTimestamp =
        typeof(Map).GetField("_neighbourCacheTimestamp", BindingFlags.Instance | BindingFlags.NonPublic)!;

    private static string Dir => Path.Combine(AppContext.BaseDirectory, "../../../../tests/review_pkg_i");

    private static List<JsonElement> Steps() =>
        File.ReadAllLines(Path.Combine(Dir, "bots42.jsonl")).Select(l => JsonDocument.Parse(l).RootElement).ToList();

    private static Participation Part()
    {
        var p = new Participation();
        p.PlayerNames[100] = "Human100";
        p.SeatedPlayers[100] = 0;
        return p;
    }

    private static void Applicable(Game g)
    {
        var players = g.Players.ToList();
        for (var i = 0; i <= players.Count; i++)
            foreach (var host in new[] { false, true })
                try { g.GetApplicableEvents(i < players.Count ? players[i] : null, host); } catch { }
    }

    private static Message? Exec(Game g, string json, bool validate, bool isHost)
    {
        if (json == "HOOK:fd_winner")
        {
            // Tleilaxu's first face dancer becomes the winning leader, so FaceDanced can run.
            g.GetPlayer(Faction.Tleilaxu)!.FaceDancers[0] = g.WinnerHero!;
            return null;
        }
        if (json.StartsWith("HOOK:give:"))
        {
            // HOOK:give:<faction int>:<card id> puts a card in a hand.
            var parts = json.Split(':');
            g.GetPlayer((Faction)int.Parse(parts[2]))!.TreacheryCards.Add(TreacheryCardManager.Get(int.Parse(parts[3]))!);
            return null;
        }
        if (json.StartsWith("HOOK:givenull:"))
        {
            // Second review: a null card in a hand, as IxianSelectedStartingCard or RicheseGaveCard leave it.
            g.GetPlayer((Faction)int.Parse(json.Split(':')[2]))!.TreacheryCards.Add(null!);
            return null;
        }
        if (json.StartsWith("HOOK:rule:"))
        {
            g.Rules.Add((Rule)int.Parse(json.Split(':')[2]));
            return null;
        }
        if (json.StartsWith("HOOK:faction:"))
        {
            var parts = json.Split(':');
            g.GetPlayer((Faction)int.Parse(parts[2]))!.Faction = (Faction)int.Parse(parts[3]);
            return null;
        }
        if (json == "HOOK:fd_others_revealed")
        {
            var tleilaxu = g.GetPlayer(Faction.Tleilaxu)!;
            tleilaxu.RevealedFaceDancers.Add(tleilaxu.FaceDancers[1]);
            tleilaxu.RevealedFaceDancers.Add(tleilaxu.FaceDancers[2]);
            return null;
        }
        var e = Utilities.Deserialize<GameEvent>(json)!;
        e.Initialize(g);
        NeighbourCacheTimestamp.SetValue(g.Map, -1);
        try { return e.Execute(validate, isHost); }
        catch (Exception x) { return Message.Express("ESCAPED ", x.GetType().Name, ": ", x.Message); }
    }

    /// <summary>Replays steps 0..last, as the oracle did: queries, the event, then GetApplicableEvents for everyone.</summary>
    public static Game Prefix(int last)
    {
        var g = new Game(Game.LatestVersion, Part());
        foreach (var s in Steps())
        {
            var n = s.GetProperty("n").GetInt32();
            if (n > last) break;
            foreach (var q in s.GetProperty("queries").EnumerateArray())
            {
                var p = q[0].ValueKind == JsonValueKind.Null ? null : g.Players.ToList()[q[0].GetInt32()];
                try { g.GetApplicableEvents(p, q[1].GetBoolean()); } catch { }
            }
            Exec(g, s.GetProperty("event").GetRawText(), s.GetProperty("validate").GetBoolean(), s.GetProperty("isHost").GetBoolean());
            Applicable(g);
        }
        return g;
    }

    /// <summary>Replays the trace steps after `after` on a game that has left the trace, and counts the outcomes.</summary>
    private static string Continue(Game g, int after)
    {
        var outcomes = new List<string>();
        foreach (var s in Steps())
        {
            if (s.GetProperty("n").GetInt32() <= after) continue;
            foreach (var q in s.GetProperty("queries").EnumerateArray())
            {
                var p = q[0].ValueKind == JsonValueKind.Null ? null : g.Players.ToList()[q[0].GetInt32()];
                try { g.GetApplicableEvents(p, q[1].GetBoolean()); } catch { }
            }
            var before = g.History.Count;
            var m = Exec(g, s.GetProperty("event").GetRawText(), s.GetProperty("validate").GetBoolean(), s.GetProperty("isHost").GetBoolean());
            outcomes.Add(g.History.Count > before ? "a" : m == null ? "?" : "r");
            Applicable(g);
        }
        return "continued:" + string.Join("", outcomes);
    }

    private static string Hash(Game g)
    {
        var o = new JsonOut();
        new StateDumper(o).DumpGame(g);
        return StateDumper.Sha256Hex(o.WrittenSpan);
    }

    private static string DumpMessage(Message? m, Game g)
    {
        if (m == null) return "null";
        var o = new JsonOut();
        new StateDumper(o).DumpDetached(m, g);
        return Encoding.UTF8.GetString(o.WrittenSpan);
    }

    private static void WriteDump(Game g, string name)
    {
        var dir = Environment.GetEnvironmentVariable("REVIEW_I_DUMPS");
        if (dir == null) return;
        var o = new JsonOut();
        new StateDumper(o).DumpGame(g);
        File.WriteAllBytes(Path.Combine(dir, name + ".cs.json"), o.WrittenSpan.ToArray());
    }

    // name, prefix step, events (json, validate, isHost)
    public static readonly (string Name, int After, (string Json, bool Validate, bool Host)[] Events)[] Probes =
    [
        ("prefix127", 127, []),
        ("prefix129", 129, []),
        ("ecazdial", 127, [("""{"$type":"KarmaEcazDial","Initiator":80,"Time":"2024-01-01T21:56:00+00:00"}""", false, true)]),
        ("juice_aggressor", 127, [("""{"$type":"JuicePlayed","Type":30,"Initiator":100,"Time":"2024-01-01T21:56:00+00:00"}""", false, true)]),
        ("juice_aggressor_then_battle", 127, [
            ("""{"$type":"JuicePlayed","Type":30,"Initiator":100,"Time":"2024-01-01T21:56:00+00:00"}""", false, true),
            ("""{"$type":"TreacheryCalled","TraitorCalled":false,"Initiator":100,"Time":"2024-01-01T21:57:06+00:00"}""", true, false),
            ("""{"$type":"TreacheryCalled","TraitorCalled":false,"Initiator":80,"Time":"2024-01-01T21:57:07+00:00"}""", true, false)]),
        ("juice_swap", 127, [
            ("HOOK:give:100:48", false, false),
            ("""{"$type":"JuicePlayed","Type":30,"Initiator":100,"Time":"2024-01-01T21:56:00+00:00"}""", false, true),
            ("""{"$type":"TreacheryCalled","TraitorCalled":false,"Initiator":100,"Time":"2024-01-01T21:57:06+00:00"}""", true, false),
            ("""{"$type":"TreacheryCalled","TraitorCalled":false,"Initiator":80,"Time":"2024-01-01T21:57:07+00:00"}""", true, false)]),
        ("juice_gofirst_at_battle", 124, [
            ("HOOK:give:50:48", false, false),
            ("""{"$type":"JuicePlayed","Type":10,"Initiator":50,"Time":"2024-01-01T21:54:00+00:00"}""", false, true)]),
        ("residual_richese", 127, [
            ("HOOK:give:100:54", false, false),
            ("""{"$type":"ResidualPlayed","Initiator":100,"Time":"2024-01-01T21:56:00+00:00"}""", false, true)]),
        ("residual_tleilaxu_card", 127, [
            ("HOOK:give:80:54", false, false),
            ("""{"$type":"ResidualPlayed","Initiator":80,"Time":"2024-01-01T21:56:00+00:00"}""", false, true)]),
        ("ecazdial_karma", 127, [
            ("HOOK:give:80:23", false, false),
            ("""{"$type":"KarmaEcazDial","Initiator":80,"Time":"2024-01-01T21:56:00+00:00"}""", false, true)]),
        ("residual", 127, [("""{"$type":"ResidualPlayed","Initiator":100,"Time":"2024-01-01T21:56:00+00:00"}""", false, true)]),
        ("residual_tleilaxu", 127, [("""{"$type":"ResidualPlayed","Initiator":80,"Time":"2024-01-01T21:56:00+00:00"}""", false, true)]),
        ("rock_kill", 127, [("""{"$type":"RockWasMelted","Kill":true,"Initiator":80,"Time":"2024-01-01T21:56:00+00:00"}""", false, true)]),
        ("antidote", 127, [("""{"$type":"PortableAntidoteUsed","Initiator":80,"Time":"2024-01-01T21:56:00+00:00"}""", false, true)]),
        ("diplomacy", 127, [("""{"$type":"Diplomacy","_cardId":32,"Initiator":80,"Time":"2024-01-01T21:56:00+00:00"}""", false, true)]),
        ("karma_prescience", 127, [("""{"$type":"KarmaPrescience","Initiator":100,"Time":"2024-01-01T21:56:00+00:00"}""", false, true)]),
        ("thought", 127, [
            ("""{"$type":"Thought","_cardId":6,"Initiator":80,"Time":"2024-01-01T21:56:00+00:00"}""", false, true),
            ("""{"$type":"ThoughtAnswered","_cardId":6,"Initiator":100,"Time":"2024-01-01T21:56:30+00:00"}""", true, true)]),
        ("thought_answer_none", 127, [("""{"$type":"ThoughtAnswered","_cardId":6,"Initiator":100,"Time":"2024-01-01T21:56:30+00:00"}""", false, true)]),
        ("audit_cancel", 127, [("""{"$type":"AuditCancelled","Cancelled":true,"Initiator":100,"Time":"2024-01-01T21:56:00+00:00"}""", false, true)]),
        ("audit_cancel_after", 129, [("""{"$type":"AuditCancelled","Cancelled":true,"Initiator":80,"Time":"2024-01-01T21:58:50+00:00"}""", false, true)]),
        ("audit_keep_after", 129, [("""{"$type":"AuditCancelled","Cancelled":false,"Initiator":80,"Time":"2024-01-01T21:58:50+00:00"}""", false, true)]),
        ("audited_after", 129, [("""{"$type":"Audited","Initiator":90,"Time":"2024-01-01T21:58:50+00:00"}""", false, true)]),
        ("voice", 127, [("""{"$type":"Voice","Must":true,"Type":30,"Initiator":60,"Time":"2024-01-01T21:56:00+00:00"}""", false, true)]),
        ("poison_tooth", 127, [("""{"$type":"PoisonToothCancelled","Initiator":80,"Time":"2024-01-01T21:56:00+00:00"}""", false, true)]),
        ("fd_reveal", 129, [("""{"$type":"FaceDancerRevealed","Passed":false,"Initiator":80,"Time":"2024-01-01T21:59:02+00:00"}""", true, false)]),
        ("fd_replace", 129, [("""{"$type":"FaceDancerReplaced","Passed":false,"dancerId":-1,"Initiator":80,"Time":"2024-01-01T21:59:02+00:00"}""", false, true)]),
        ("fd_danced", 129, [
            ("HOOK:fd_winner", false, false),
            ("""{"$type":"FaceDancerRevealed","Passed":false,"Initiator":80,"Time":"2024-01-01T21:59:02+00:00"}""", true, false),
            ("""{"$type":"FaceDanced","_targetForceLocations":"8:2|0","ForcesFromReserve":2,"FaceDancerCalled":false,"_toId":-1,"_forceLocations":"","Passed":false,"Initiator":80,"Time":"2024-01-01T21:59:30+00:00"}""", true, false)]),
        ("fd_danced_sources", 129, [
            ("HOOK:fd_winner", false, false),
            ("""{"$type":"FaceDancerRevealed","Passed":false,"Initiator":80,"Time":"2024-01-01T21:59:02+00:00"}""", false, true),
            ("""{"$type":"FaceDanced","_targetForceLocations":"8:2|0","ForcesFromReserve":1,"FaceDancerCalled":false,"_toId":-1,"_forceLocations":"95:1|0,5:0|0","Passed":false,"Initiator":80,"Time":"2024-01-01T21:59:30+00:00"}""", false, true)]),
        ("fd_danced_last", 129, [
            ("HOOK:fd_winner", false, false),
            ("HOOK:fd_others_revealed", false, false),
            ("""{"$type":"FaceDancerRevealed","Passed":false,"Initiator":80,"Time":"2024-01-01T21:59:02+00:00"}""", true, false),
            ("""{"$type":"FaceDanced","_targetForceLocations":"8:2|0","ForcesFromReserve":2,"FaceDancerCalled":false,"_toId":-1,"_forceLocations":"","Passed":false,"Initiator":80,"Time":"2024-01-01T21:59:30+00:00"}""", true, false)]),
        ("fd_danced_no_match", 129, [
            ("""{"$type":"FaceDanced","_targetForceLocations":"8:2|0","ForcesFromReserve":2,"FaceDancerCalled":false,"_toId":-1,"_forceLocations":"","Passed":false,"Initiator":80,"Time":"2024-01-01T21:59:30+00:00"}""", false, true)]),
        ("traitor_discard", 127, [("""{"$type":"TraitorDiscarded","_traitorId":1016,"Initiator":100,"Time":"2024-01-01T21:56:00+00:00"}""", false, true)]),
        ("fd_replace_one", 129, [("""{"$type":"FaceDancerReplaced","Passed":false,"dancerId":1042,"Initiator":80,"Time":"2024-01-01T21:59:02+00:00"}""", false, true)]),
        ("ecazdial_no_battle", 124, [("""{"$type":"KarmaEcazDial","Initiator":80,"Time":"2024-01-01T21:54:00+00:00"}""", false, true)]),
        ("residual_no_battle", 124, [("HOOK:give:100:54", false, false), ("""{"$type":"ResidualPlayed","Initiator":100,"Time":"2024-01-01T21:54:00+00:00"}""", false, true)]),
        ("juice_golast_cont", 124, [("HOOK:give:50:48", false, false), ("""{"$type":"JuicePlayed","Type":20,"Initiator":50,"Time":"2024-01-01T21:54:00+00:00"}""", false, true), ("CONTINUE", false, false)]),
        ("juice_gofirst_cont", 124, [("HOOK:give:80:48", false, false), ("""{"$type":"JuicePlayed","Type":10,"Initiator":80,"Time":"2024-01-01T21:54:00+00:00"}""", false, true), ("CONTINUE", false, false)]),
        ("juice_swap_cont", 127, [("HOOK:give:100:48", false, false), ("""{"$type":"JuicePlayed","Type":30,"Initiator":100,"Time":"2024-01-01T21:56:00+00:00"}""", false, true), ("CONTINUE", false, false)]),
        ("juice_aggr_before_battle_cont", 125, [("HOOK:give:100:48", false, false), ("""{"$type":"JuicePlayed","Type":30,"Initiator":100,"Time":"2024-01-01T21:54:30+00:00"}""", false, true), ("CONTINUE", false, false)]),
        ("fd_danced_cont", 129, [
            ("HOOK:fd_winner", false, false),
            ("""{"$type":"FaceDancerRevealed","Passed":false,"Initiator":80,"Time":"2024-01-01T21:59:02+00:00"}""", true, false),
            ("""{"$type":"FaceDanced","_targetForceLocations":"8:2|0","ForcesFromReserve":2,"FaceDancerCalled":false,"_toId":-1,"_forceLocations":"","Passed":false,"Initiator":80,"Time":"2024-01-01T21:59:30+00:00"}""", true, false),
            ("CONTINUE", false, false)]),
        ("fd_validate", 129, [
            ("HOOK:fd_winner", false, false),
            ("""{"$type":"FaceDancerRevealed","Passed":false,"Initiator":80,"Time":"2024-01-01T21:59:02+00:00"}""", true, false),
            ("""VALIDATE:{"$type":"FaceDanced","_targetForceLocations":"8:3|0","ForcesFromReserve":3,"_forceLocations":"","Passed":false,"Initiator":80,"Time":"2024-01-01T21:59:30+00:00"}""", false, false),
            ("""VALIDATE:{"$type":"FaceDanced","_targetForceLocations":"8:1|0","ForcesFromReserve":2,"_forceLocations":"","Passed":false,"Initiator":80,"Time":"2024-01-01T21:59:30+00:00"}""", false, false),
            ("""VALIDATE:{"$type":"FaceDanced","_targetForceLocations":"8:x|0","ForcesFromReserve":2,"_forceLocations":"","Passed":false,"Initiator":80,"Time":"2024-01-01T21:59:30+00:00"}""", false, false),
            ("""VALIDATE:{"$type":"FaceDanced","_targetForceLocations":null,"ForcesFromReserve":2,"_forceLocations":"","Passed":false,"Initiator":80,"Time":"2024-01-01T21:59:30+00:00"}""", false, false),
            ("""VALIDATE:{"$type":"FaceDanced","_targetForceLocations":"8:2|0","ForcesFromReserve":2147483647,"_forceLocations":"95:1|0","Passed":false,"Initiator":80,"Time":"2024-01-01T21:59:30+00:00"}""", false, false),
            ("""VALIDATE:{"$type":"FaceDanced","_targetForceLocations":"8:2|0","ForcesFromReserve":2,"_forceLocations":"","Passed":false,"Initiator":100,"Time":"2024-01-01T21:59:30+00:00"}""", false, false),
            ("""VALIDATE:{"$type":"FaceDancerReplaced","Passed":false,"dancerId":1042,"Initiator":100,"Time":"2024-01-01T21:59:30+00:00"}""", false, false),
            ("""VALIDATE:{"$type":"FaceDancerReplaced","Passed":false,"dancerId":1001,"Initiator":80,"Time":"2024-01-01T21:59:30+00:00"}""", false, false),
            ("""VALIDATE:{"$type":"FaceDancerReplaced","Passed":false,"dancerId":9999,"Initiator":80,"Time":"2024-01-01T21:59:30+00:00"}""", false, false),
            ("""VALIDATE:{"$type":"TraitorDiscarded","_traitorId":1016,"Initiator":80,"Time":"2024-01-01T21:59:30+00:00"}""", false, false),
            ("""VALIDATE:{"$type":"Voice","Must":true,"Type":30,"Initiator":60,"Time":"2024-01-01T21:59:30+00:00"}""", false, false),
            ("""VALIDATE:{"$type":"AuditCancelled","Cancelled":true,"Initiator":80,"Time":"2024-01-01T21:59:30+00:00"}""", false, false),
            ("""VALIDATE:{"$type":"BeneGesseritBattleAnnouncement","_territoryId":4,"Initiator":60,"Time":"2024-01-01T21:59:30+00:00"}""", false, false),
            ("""VALIDATE:{"$type":"BeneGesseritBattleAnnouncement","_territoryId":4,"Initiator":20,"Time":"2024-01-01T21:59:30+00:00"}""", false, false),
            ("""VALIDATE:{"$type":"BeneGesseritBattleAnnouncement","_territoryId":999,"Initiator":60,"Time":"2024-01-01T21:59:30+00:00"}""", false, false),
            ("""VALIDATE:{"$type":"ThoughtAnswered","_cardId":2,"Initiator":100,"Time":"2024-01-01T21:59:30+00:00"}""", false, false),
            ("""VALIDATE:{"$type":"Thought","_cardId":2,"Initiator":80,"Time":"2024-01-01T21:59:30+00:00"}""", false, false),
            ("""VALIDATE:{"$type":"Thought","_cardId":41,"Initiator":80,"Time":"2024-01-01T21:59:30+00:00"}""", false, false)]),
        // Second review round (see docs/reviews/pkg-i.md, "Second review").
        ("r2_voice_nexus_partial", 127, [
            ("HOOK:faction:60:998", false, false),
            ("""{"$type":"Voice","Must":true,"Type":30,"Initiator":997,"Time":"2024-01-01T21:56:00+00:00"}""", false, true)]),
        ("r2_thought_answer_null", 127, [
            ("HOOK:givenull:100", false, false),
            ("""{"$type":"Thought","_cardId":33,"Initiator":80,"Time":"2024-01-01T21:56:00+00:00"}""", false, true),
            ("""VALIDATE:{"$type":"ThoughtAnswered","_cardId":-1,"Initiator":100,"Time":"2024-01-01T21:56:30+00:00"}""", false, false),
            ("""{"$type":"ThoughtAnswered","_cardId":-1,"Initiator":100,"Time":"2024-01-01T21:56:30+00:00"}""", true, true)]),
        ("r2_audit_null", 125, [
            ("""{"$type":"Battle","Messiah":false,"Forces":1,"ForcesAtHalfStrength":0,"SpecialForces":0,"SpecialForcesAtHalfStrength":0,"AllyContributionAmount":0,"BankerBonus":0,"_heroId":1046,"_weaponCardId":6,"_defenseCardId":31,"Initiator":100,"Time":"2024-01-01T21:55:03+00:00"}""", false, true),
            ("""{"$type":"Battle","Messiah":false,"Forces":5,"ForcesAtHalfStrength":0,"SpecialForces":0,"SpecialForcesAtHalfStrength":0,"AllyContributionAmount":0,"BankerBonus":0,"_heroId":1040,"_weaponCardId":32,"_defenseCardId":-1,"Initiator":80,"Time":"2024-01-01T21:55:52+00:00"}""", true, false),
            ("HOOK:rule:500", false, false),
            ("HOOK:givenull:80", false, false),
            ("CALL:audit", false, false),
            ("CONTINUE:127", false, false)]),
        ("r2_karma_prescience_null", 127, [
            ("HOOK:give:100:23", false, false),
            ("HOOK:givenull:100", false, false),
            ("""{"$type":"KarmaPrescience","Initiator":100,"Time":"2024-01-01T21:56:00+00:00"}""", false, true)]),
        ("r2_ecazdial_null", 127, [
            ("HOOK:give:80:23", false, false),
            ("HOOK:givenull:80", false, false),
            ("""{"$type":"KarmaEcazDial","Initiator":80,"Time":"2024-01-01T21:56:00+00:00"}""", false, true)]),
        ("r2_applicable_karma_null", 127, [
            ("HOOK:give:100:23", false, false),
            ("HOOK:givenull:100", false, false),
            ("CALL:applicable:100", false, false)]),
        ("bene_gesserit_announce", 127, [("""{"$type":"BeneGesseritBattleAnnouncement","_territoryId":6,"Initiator":60,"Time":"2024-01-01T21:56:00+00:00"}""", false, true)]),
    ];

    public static void Run(string[] args)
    {
        CultureInfo.CurrentCulture = CultureInfo.InvariantCulture;
        if (args.Length > 1 && args[1] == "info")
        {
            foreach (var at in new[] { 127, 129 })
            {
                var g = Prefix(at);
                Console.WriteLine($"== after {at}: phase {g.CurrentPhase} winner {g.BattleWinner} battle {g.CurrentBattle?.Initiator}->{g.CurrentBattle?.Target} in {g.CurrentBattle?.Territory?.Id} winnerHero {g.WinnerHero}");
                Console.WriteLine($"   aggr {g.AggressorPlan?.Initiator} hero {g.AggressorPlan?.Hero} w {g.AggressorPlan?.Weapon} d {g.AggressorPlan?.Defense}; def {g.DefenderPlan?.Initiator} hero {g.DefenderPlan?.Hero} w {g.DefenderPlan?.Weapon} d {g.DefenderPlan?.Defense}");
                foreach (var p in g.Players)
                    Console.WriteLine($"   {p.Faction} res {p.Resources} reserves {p.ForcesInReserve}/{p.SpecialForcesInReserve} cards [{string.Join(",", p.TreacheryCards.Select(c => c.Id + ":" + c.Type))}] traitors [{string.Join(",", p.Traitors.Select(t => t.Id))}] fd [{string.Join(",", p.FaceDancers.Select(t => t.Id))}] revealedFd [{string.Join(",", p.RevealedFaceDancers.Select(t => t.Id))}] forces [{string.Join(",", p.ForcesInLocations.Select(kv => kv.Key.Id + ":" + kv.Value.AmountOfForces + "|" + kv.Value.AmountOfSpecialForces))}]");
                if (g.CurrentBattle?.Territory != null)
                    Console.WriteLine("   territory locations " + string.Join(",", g.CurrentBattle.Territory.Locations.Select(l => l.Id)));
            }
            return;
        }

        var only = args.Length > 1 ? args[1] : null;
        foreach (var (name, after, events) in Probes)
        {
            if (only != null && !name.StartsWith(only)) continue;
            var g = Prefix(after);
            var results = new List<string>();
            foreach (var (json, validate, host) in events)
            {
                if (json == "CONTINUE")
                {
                    results.Add(Continue(g, after));
                    continue;
                }
                if (json.StartsWith("CONTINUE:"))
                {
                    results.Add(Continue(g, int.Parse(json["CONTINUE:".Length..])));
                    continue;
                }
                if (json == "CALL:audit")
                {
                    try { results.Add("audit " + AuditCancelled.GetNumberOfCardsThatMayBeAudited(g)); }
                    catch (Exception x) { results.Add("THROWS " + x.GetType().Name + ": " + x.Message); }
                    continue;
                }
                if (json.StartsWith("CALL:applicable:"))
                {
                    var who = g.GetPlayer((Faction)int.Parse(json.Split(':')[2]));
                    try { results.Add("applicable " + string.Join(",", g.GetApplicableEvents(who, false).Select(t => t.Name))); }
                    catch (Exception x) { results.Add("THROWS " + x.GetType().Name + ": " + x.Message); if (Environment.GetEnvironmentVariable("REVIEW_I_STACK") != null) Console.Error.WriteLine(x.StackTrace); }
                    continue;
                }
                if (json.StartsWith("VALIDATE:"))
                {
                    var e = Utilities.Deserialize<GameEvent>(json["VALIDATE:".Length..])!;
                    e.Initialize(g);
                    string r;
                    try { r = DumpMessage(e.Validate(), g); }
                    catch (Exception x) { r = "THROWS " + x.GetType().Name + ": " + x.Message; }
                    results.Add(r);
                    continue;
                }
                results.Add(DumpMessage(Exec(g, json, validate, host), g));
                Applicable(g);
            }
            WriteDump(g, name);
            Console.WriteLine($"{name}.hash = {Hash(g)}");
            Console.WriteLine($"{name}.results = {string.Join(" | ", results)}");
        }
    }
}
