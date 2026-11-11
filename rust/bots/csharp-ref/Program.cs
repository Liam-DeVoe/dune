// C# reference values for rust/bots/tests/knowledge.rs.
//
// Usage: dotnet run -c Release -- knowledge TRACE.jsonl.gz OUT.jsonl.gz [every]
//        dotnet run -c Release -- core OUT.json
//
// core: Bot.cs and BotParameters.cs: LogError on a real (formatting) console, D, GetDefaultParameters.
//
// Replays the "step" lines of an oracle trace (it stops at the first line of any other kind) and,
// after every `every`-th step, gives each seated player a fresh ClassicBot with a fixed seed and
// calls the Bot_Knowledge members on it through reflection: the results, exceptions and draw
// counts go to OUT. OUT also holds the events, so it doesn't depend on the trace staying around.

using System.Collections;
using System.Globalization;
using System.IO.Compression;
using System.Text.Json.Nodes;
using Treachery.Bots;
using Treachery.Oracle;

CultureInfo.DefaultThreadCurrentCulture = CultureInfo.InvariantCulture;
CultureInfo.CurrentCulture = CultureInfo.InvariantCulture;
Console.SetOut(TextWriter.Null);

if (args.Length == 2 && args[0] == "core")
{
    Core.Run(args[1]);
    return 0;
}

if (args.Length == 2 && args[0] == "b6unbox")
{
    B6.Unbox(args[1]);
    return 0;
}

if (args.Length >= 3 && args[0] == "b2")
{
    B2.Run(args[1], args[2], args.Length > 3 ? int.Parse(args[3]) : 1);
    return 0;
}

if (args.Length >= 3 && args[0] == "b3")
{
    B3.Run(args[1], args[2], args.Length > 3 ? int.Parse(args[3]) : 1);
    return 0;
}

if (args.Length >= 3 && args[0] == "b4")
{
    B4.Run(args[1], args[2], args.Length > 3 ? int.Parse(args[3]) : 1);
    return 0;
}

if (args.Length >= 3 && args[0] == "b5")
{
    B5.Run(args[1], args[2], args.Length > 3 ? int.Parse(args[3]) : 1);
    return 0;
}

if (args.Length >= 3 && args[0] == "b6")
{
    B6.Run(args[1], args[2], args.Length > 3 ? int.Parse(args[3]) : 1);
    return 0;
}

if (args.Length < 3 || args[0] != "knowledge")
{
    Console.Error.WriteLine("usage: knowledge TRACE.jsonl.gz OUT.jsonl.gz [every] | core OUT.json");
    return 1;
}

var every = args.Length > 3 ? int.Parse(args[3]) : 10;
Knowledge.Run(args[1], args[2], every);
return 0;

static class Knowledge
{
    private const BindingFlags All = BindingFlags.Instance | BindingFlags.Static | BindingFlags.Public | BindingFlags.NonPublic;

    private static Game _game = null!;
    private static Player _player = null!;
    private static ClassicBot _bot = null!;
    private static LoggedRandom _random = null!;
    private static StreamWriter _out = null!;
    private static int _step;
    private static int _seed;

    public static void Run(string tracePath, string outPath, int every)
    {
        using var input = new StreamReader(new GZipStream(File.OpenRead(tracePath), CompressionMode.Decompress));
        using var outFile = File.Create(outPath);
        using var gz = new GZipStream(outFile, CompressionLevel.SmallestSize);
        _out = new StreamWriter(gz) { NewLine = "\n" };

        string? text;
        while ((text = input.ReadLine()) != null)
        {
            var line = JsonNode.Parse(text)!.AsObject();
            var kind = (string)line["kind"]!;
            if (kind == "header")
            {
                var p = line["participation"]!.AsObject();
                // As Oracle/Verifier.cs ParseParticipation.
                var participation = new Participation { BotsSpeed = (int)p["BotsSpeed"]! };
                foreach (var kv in p["PlayerNames"]!.AsArray()) participation.PlayerNames.Add((int)kv![0]!, (string)kv[1]!);
                foreach (var kv in p["SeatedPlayers"]!.AsArray()) participation.SeatedPlayers.Add((int)kv![0]!, (int)kv[1]!);
                foreach (var v in p["Observers"]!.AsArray()) participation.Observers.Add((int)v!);
                foreach (var v in p["Hosts"]!.AsArray()) participation.Hosts.Add((int)v!);
                foreach (var v in p["AvailableSeats"]!.AsArray()) participation.AvailableSeats.Add((int)v!);
                foreach (var v in p["Kicked"]!.AsArray()) participation.Kicked.Add((int)v!);
                _game = new Game((int)line["gameVersion"]!, participation);
                Write(new JsonObject { ["k"] = "header", ["version"] = (int)line["gameVersion"]!, ["participation"] = p.DeepClone() });
                continue;
            }

            if (kind != "step") break;

            _step = (int)line["n"]!;
            var json = line["event"]!.ToJsonString();
            var validate = (bool)line["validate"]!;
            var isHost = (bool)line["isHost"]!;
            Write(new JsonObject { ["k"] = "step", ["n"] = _step, ["validate"] = validate, ["isHost"] = isHost, ["event"] = json });
            var evt = Utilities.Deserialize<GameEvent>(json)!;
            evt.Initialize(_game);
            StepExecutor.Run(_game, evt, validate, isHost);

            if (_step % every == 0) ProbeAll();
        }

        _out.Flush();
    }

    private static void Write(JsonObject o) => _out.WriteLine(o.ToJsonString());

    private static void ProbeAll()
    {
        for (var i = 0; i < _game.Players.Count; i++)
        {
            _player = _game.Players[i];
            if (_player.Faction == Faction.None) continue;
            StepExecutor.InvalidateNeighbourCache(_game);
            _seed = _step * 31 + i;
            _random = new LoggedRandom(_seed);
            _bot = new ClassicBot(_game, _player, BotParameters.GetDefaultParameters(_player.Faction), _random);
            ProbePlayer(i);
        }
    }

    private static string P(Player p) => "P" + _game.Players.IndexOf(p);
    private static string F(Faction f) => "F" + (int)f;
    private static string L(Location l) => "L" + l.Id;
    private static string T(Territory t) => "T" + t.Id;
    private static string C(TreacheryCard c) => "C" + c.Id;

    private static void ProbePlayer(int me)
    {
        var my = "P" + me;
        string[] none = [];

        foreach (var name in new[]
                 {
                     "MaxExpectedStormMoves", "MayFlipToAdvisors", "ForcesOnPlanet", "Others", "Opponents", "MeAndMyAlly", "LastTurn",
                     "AlmostLastTurn", "OpponentsToShipAndMove", "NrOfNonWinningPlayersToShipAndMoveIncludingMe", "IAmWinning",
                     "OpponentsAreWinning", "MyPrescience", "MyVoice", "MayUseUselessAsKarma", "IAmDesperateForResources",
                     "ResourcesIncludingAllyContribution", "ResourcesIncludingAllyAndEmperorContribution", "ResourcesFromAlly",
                     "ResourcesFromEmperor", "AllyResources", "BattalionThatShouldBeMovedDueToAllyPresence",
                     "BiggestBattalionThreatenedByStormWithoutSpice", "BiggestBattalionInSpicelessNonStrongholdLocationOnRock",
                     "BiggestBattalionInSpicelessNonStrongholdLocationInSandOrNotNearStronghold",
                     "BiggestBattalionInSpicelessNonStrongholdLocationNotNearStrongholdAndSpice",
                     "BiggestLargeUnthreatenedMovableBattalionInStrongholdNearVacantStronghold",
                     "BiggestMovableStackOfAdvisorsInStrongholdNearVacantStronghold",
                     "BiggestLargeUnthreatenedMovableBattalionInStrongholdNearSpice", "IMustPayForForcesInBattle", "NrOfBattlesToFight",
                     "SafeOrKnownTraitorLeaders"
                 })
            Probe(name, none);

        foreach (var name in new[]
                 {
                     "MyClairvoyanceAboutEnemyDefenseInCurrentBattle", "PlayerClairvoyanceAboutEnemyWeaponInCurrentBattle",
                     "CardsUnknownToPlayer", "CardsPlayerHasOrMightHave", "CanShip"
                 })
            Probe(name, ["G", my]);

        Probe("Weapons", ["G", my, "N", "N", "N"]);
        Probe("Defenses", ["G", my, "N", "N"]);
        Probe("UselessAsWeapon", ["G", my, "N"]);
        Probe("UselessAsDefense", ["G", my, "N"]);
        Probe("MayPlayNoWeapon", ["G", my, "N"]);
        Probe("MayPlayNoDefense", ["G", my, "N"]);
        Probe("CountDifferentWeaponTypes", ["Hand" + my]);
        Probe("CountDifferentDefenseTypes", ["Hand" + my]);

        var strongholds = _game.Map.Strongholds.ToList();

        foreach (var o in _game.Players)
        {
            if (o == _player || o.Faction == Faction.None) continue;
            var op = P(o);
            Probe("WinWasPredictedByMeThisTurn", [F(o.Faction)]);
            Probe("IsWinning", [op]);
            Probe("IsWinning", [F(o.Faction)]);
            Probe("IsWinningOpponent", [op]);
            Probe("IsWinningOpponent", [F(o.Faction)]);
            Probe("IsAlmostWinningOpponent", [op]);
            Probe("IWillBeAggressorAgainst", [op]);
            Probe("OpponentCardsUnknownToPlayer", ["G", my, op]);
            Probe("KnownCardsHeldByOtherPlayer", ["G", my, op]);
            Probe("KnownOpponentWeapons", ["G", my, op]);
            Probe("KnownOpponentCards", ["G", my, op]);
            Probe("KnownOpponentDefenses", ["G", my, op]);
            foreach (var s in strongholds.Take(2)) Probe("MaxReinforcedDialTo", ["G", my, op, T(s.Territory)]);
            foreach (var c in o.TreacheryCards) Probe("CardQuality", [C(c), op]);
        }

        foreach (var o in _game.Players)
        foreach (var c in o.TreacheryCards)
        {
            Probe("CardQuality", [C(c), "N"]);
            Probe("IsKnownTo", ["G", my, C(c)]);
        }

        var locations = strongholds.Concat(_player.ForcesOnPlanet.Keys).Concat(_game.ResourcesOnPlanet.Keys).Distinct().ToList();
        var firstOther = _game.Players.FirstOrDefault(p => p != _player && p.Faction != Faction.None);
        foreach (var l in locations)
        {
            var ls = L(l);
            var ts = T(l.Territory);
            Probe("ResourcesIn", [ls]);
            Probe("HasResources", [ls]);
            Probe("IsStronghold", [ls]);
            Probe("NotOccupiedByOthers", [ls]);
            Probe("NotOccupiedByOthers", [ts]);
            Probe("NotOccupied", [ts]);
            Probe("NotOccupied", [ls]);
            Probe("Vacant", [ls]);
            Probe("Vacant", [ts]);
            Probe("OccupiedByOpponent", [ts]);
            Probe("OccupiedByOpponent", [ls]);
            Probe("GetOpponentThatOccupies", [ls]);
            Probe("GetOpponentThatOccupies", [ts]);
            Probe("StormWillProbablyHit", [ls]);
            Probe("VacantAndSafeFromStorm", [ls]);
            Probe("IDontHaveAdvisorsIn", [ls]);
            Probe("AllyDoesntBlock", [ts]);
            Probe("AllyDoesntBlock", [ls]);
            Probe("InStorm", [ls]);
            Probe("MayFleeOutOf", [ls]);
            Probe("WithinDistance", [ls, L(_game.Map.Arrakeen), "i2"]);
            Probe("ProbablySafeFromMonster", [ts]);
            Probe("OccupyingOpponentIn", ["G", my, ts]);
            Probe("OccupyingOpponentsIn", ["G", my, ts]);
            Probe("OccupyingOpponentsIn", ["G", my, ls]);
            Probe("MaxDial", ["G", my, ts, "N", "false"]);
            if (firstOther != null) Probe("MaxDial", ["G", P(firstOther), ts, my, "true"]);
            Probe("TotalMaxDialOfOpponents", [ts]);
        }

        foreach (var l in _player.ForcesOnPlanet.Keys)
        {
            var ls = L(l);
            var b = "Bat" + ls;
            Probe("ValidMovementLocations", [ls, b]);
            Probe("VacantAndSafeNearbyStronghold", [ls, b]);
            Probe("UnthreatenedAndSafeNearbyStronghold", [ls, b]);
            Probe("WeakAndSafeNearbyStronghold", [ls, b]);
            Probe("NearbyStrongholdOfWinningOpponent", [ls, b, "true"]);
            Probe("NearbyStrongholdOfWinningOpponent", [ls, b, "false"]);
            Probe("NearbyStrongholdOfAlmostWinningOpponent", [ls, b, "true"]);
            Probe("NearbyStrongholdOfAlmostWinningOpponent", [ls, b, "false"]);
            Probe("BestSafeAndNearbyResources", [ls, b, "false"]);
            Probe("BestSafeAndNearbyDiscovery", [ls, b, "false"]);
            Probe("NearbyBattalionsOutsideStrongholds", [ls]);
            foreach (var s in strongholds)
            {
                Probe("WithinRange", [ls, L(s), b]);
                Probe("IsSafeAndNearby", [ls, L(s), b, "false"]);
            }

            Probe("MaxDial", ["G", "i" + _player.Resources, b, F(Faction.Harkonnen)]);
        }
    }

    private static (Type, object?) Decode(string a)
    {
        if (a == "G") return (typeof(Game), _game);
        if (a == "N") return (typeof(object), null);
        if (a is "true" or "false") return (typeof(bool), a == "true");
        if (a.StartsWith("Hand")) return (typeof(List<TreacheryCard>), _game.Players[int.Parse(a[5..])].TreacheryCards);
        if (a.StartsWith("BatL")) return (typeof(Battalion), _player.ForcesOnPlanet[_game.Map.LocationLookup.Find(int.Parse(a[4..]))]);
        var n = int.Parse(a[1..]);
        return a[0] switch
        {
            'P' => (typeof(Player), _game.Players[n]),
            'F' => (typeof(Faction), (Faction)n),
            'L' => (typeof(Location), _game.Map.LocationLookup.Find(n)),
            'T' => (typeof(Territory), _game.Map.TerritoryLookup.Find(n)),
            'C' => (typeof(TreacheryCard), TreacheryCardManager.Lookup.Find(n)),
            'i' => (typeof(int), n),
            _ => throw new ArgumentException(a)
        };
    }

    private static MemberInfo Resolve(string name, Type[] types)
    {
        var prop = typeof(ClassicBot).GetProperty(name, All);
        if (prop != null && types.Length == 0) return prop;

        // Pick the overload whose parameters accept the decoded arguments (null matches any reference type).
        var candidates = typeof(ClassicBot).GetMethods(All).Where(m => m.Name == name).Where(m =>
        {
            var ps = m.GetParameters();
            if (ps.Length != types.Length) return false;
            for (var i = 0; i < ps.Length; i++)
            {
                if (types[i] == typeof(object)) { if (ps[i].ParameterType.IsValueType && Nullable.GetUnderlyingType(ps[i].ParameterType) == null) return false; }
                else if (!ps[i].ParameterType.IsAssignableFrom(types[i])) return false;
            }
            return true;
        }).ToList();
        if (candidates.Count != 1) throw new InvalidOperationException($"{name}: {candidates.Count} overloads match");
        return candidates[0];
    }

    private static void Probe(string name, string[] encodedArgs)
    {
        var decoded = encodedArgs.Select(Decode).ToArray();
        var member = Resolve(name, decoded.Select(d => d.Item1).ToArray());
        string result;
        try
        {
            var value = member is PropertyInfo p
                ? p.GetValue(p.GetMethod!.IsStatic ? null : _bot)
                : ((MethodInfo)member).Invoke(((MethodInfo)member).IsStatic ? null : _bot, decoded.Select(d => d.Item2).ToArray());
            result = Fmt(value);
        }
        catch (TargetInvocationException e)
        {
            var inner = e.InnerException!;
            result = "EX:" + inner.GetType().FullName + ": " + inner.Message;
        }

        var args = new JsonArray();
        foreach (var a in encodedArgs) args.Add(a);
        Write(new JsonObject
        {
            ["k"] = "probe", ["n"] = _step, ["seed"] = _seed, ["p"] = _game.Players.IndexOf(_player), ["m"] = name, ["a"] = args,
            ["r"] = result, ["d"] = _random.GetCount()
        });
    }

    private static string Fmt(object? v) => v switch
    {
        null => "null",
        bool b => b ? "true" : "false",
        int i => i.ToString(CultureInfo.InvariantCulture),
        float f => f.ToString(CultureInfo.InvariantCulture),
        Faction f => F(f),
        Player p => P(p),
        Location l => L(l),
        Territory t => T(t),
        TreacheryCard c => C(c),
        IHero h => "H" + h.Id,
        Battalion b => $"B{(int)b.Faction}:{b.AmountOfForces}:{b.AmountOfSpecialForces}",
        BattalionInLocation bl => $"{L(bl.Location)}={Fmt(bl.Battalion)}",
        KeyValuePair<Location, Battalion> kv => $"{L(kv.Key)}={Fmt(kv.Value)}",
        ClairVoyanceQandA => "QandA",
        GameEvent e => e.GetType().Name,
        IEnumerable e => "[" + string.Join(",", e.Cast<object?>().Select(Fmt)) + "]",
        _ => throw new ArgumentException("no format for " + v.GetType())
    };
}

static class Core
{
    private const BindingFlags All = BindingFlags.Instance | BindingFlags.Static | BindingFlags.Public | BindingFlags.NonPublic;

    public static void Run(string outPath)
    {
        var result = new JsonObject();

        // LogError as in production: Console.Out formats (TextWriter.Null would not).
        var logError = typeof(ClassicBot).GetMethod("LogError", All)!;
        var format = new JsonArray();
        string[] cases =
        [
            "plain", "{", "}", "{{", "}}", "{0}", "a{x}b", "{ 0}", "{0 }", "{-1}", "{1,2}", "{0:x}", "x}y", "{}", "{{}}", "{a", "{0",
            "abc{{def}}g", "}}{{", "{0,", "{0,-", "{0,a}", "{ }", "{0:}", "{0:{{}", "{0:}}}", "}a", "a{", "{1000000}", "{9999999999}",
            "{0,9999999}", "{0,1000000}", "x{ 0 , 5 : y}z", "{0 x}", "{0:a{b}", "\u00e9}x", "\ud83d\ude00}x", "{0,5 x}", "{0,  5}", "{0, -5}",
            "{0,- 5}", "{00}", "{0 ,1}", "{0,1 :x}", "{0  :x}", "{1000000 }", "{0,10000000}", "{0:abc", "",
            "--error occured -->System.Collections.Generic.KeyNotFoundException: The given key 'Location: {x}' was not present in the dictionary.",
            "--invalid decision (Treachery.Shared.Bid)--> Atreides bid 3: You can't bid more than {0}"
        ];
        var stdout = Console.Out;
        foreach (var c in cases)
        {
            Console.SetOut(new StringWriter());
            string r;
            try
            {
                logError.Invoke(null, [c, Array.Empty<object?>()]);
                r = "OK";
            }
            catch (TargetInvocationException e)
            {
                r = "EX:" + e.InnerException!.GetType().FullName + ": " + e.InnerException.Message;
            }

            format.Add(new JsonArray(c, r));
        }

        Console.SetOut(stdout);
        result["logError"] = format;

        // D(amount, sides): result or exception, and the draw count after each call, on one bot.
        var game = new Game();
        var random = new LoggedRandom(12345);
        var bot = new ClassicBot(game, new Player(game, Faction.Atreides), BotParameters.GetDefaultParameters(Faction.Atreides), random);
        var d = typeof(ClassicBot).GetMethod("D", All)!;
        var dice = new JsonArray();
        foreach (var amount in new[] { 0, 1, 2, 3, 5, -1 })
        foreach (var sides in new[] { 0, 1, 2, 6, 10, 100, -1, -5 })
        {
            string r;
            try
            {
                r = ((int)d.Invoke(bot, [amount, sides])!).ToString(CultureInfo.InvariantCulture);
            }
            catch (TargetInvocationException e)
            {
                r = "EX:" + e.InnerException!.GetType().FullName + ": " + e.InnerException.Message;
            }

            dice.Add(new JsonArray(amount, sides, r, random.GetCount()));
        }

        result["d"] = dice;

        // Do<T> on a game without players: the decision, Do's result, the action afterwards, and
        // whether the method ran. Console.Out formats, as in production.
        Console.SetOut(new StringWriter());
        var doMethod = typeof(ClassicBot).GetMethod("Do", All)!;
        var scenarios = new JsonArray();

        void Scenario<T>(string name, Func<T> method, GameEvent? actionBefore, List<Type> allowed) where T : GameEvent?
        {
            var called = false;
            Func<T> wrapped = () => { called = true; return method(); };
            object?[] callArgs = [wrapped, actionBefore, allowed];
            string r;
            try
            {
                r = ((bool)doMethod.MakeGenericMethod(typeof(T)).Invoke(bot, callArgs)!) ? "true" : "false";
            }
            catch (TargetInvocationException e)
            {
                r = "EX:" + e.InnerException!.GetType().FullName + ": " + e.InnerException.Message;
            }

            var after = (GameEvent?)callArgs[1];
            scenarios.Add(new JsonObject
            {
                ["name"] = name, ["result"] = r, ["action"] = after?.GetType().Name, ["sameAction"] = ReferenceEquals(after, actionBefore),
                ["called"] = called
            });
        }

        var existing = new EndPhase(game, Faction.Atreides);
        Scenario("valid", () => new EndPhase(game, Faction.Atreides), null, [typeof(EndPhase)]);
        Scenario("notAllowed", () => new EndPhase(game, Faction.Atreides), null, [typeof(Thought)]);
        Scenario("alreadyDecided", () => new EndPhase(game, Faction.Atreides), existing, [typeof(EndPhase)]);
        Scenario("returnsNull", () => (Thought?)null, null, [typeof(Thought)]);
        Scenario<EndPhase>("throws", () => throw new InvalidOperationException("boom"), null, [typeof(EndPhase)]);
        Scenario<EndPhase>("throwsWithBrace", () => throw new InvalidOperationException("bo{om"), null, [typeof(EndPhase)]);
        Scenario("invalid", () => new Thought(game, Faction.Atreides), null, [typeof(Thought)]);
        Console.SetOut(stdout);
        result["do"] = scenarios;

        // GetDefaultParameters for every faction value, including ones outside the enum.
        var parameters = new JsonArray();
        foreach (var f in Enumerable.Range(-1, 16).Select(i => i * 10).Append(13))
        {
            var p = BotParameters.GetDefaultParameters((Faction)f);
            var o = new JsonObject { ["faction"] = f };
            foreach (var prop in typeof(BotParameters).GetProperties().OrderBy(x => x.MetadataToken))
                o[prop.Name] = prop.GetValue(p) switch
                {
                    float x => x.ToString(CultureInfo.InvariantCulture),
                    bool b => b ? "true" : "false",
                    var v => Convert.ToString(v, CultureInfo.InvariantCulture)
                };
            parameters.Add(o);
        }

        result["parameters"] = parameters;
        result["penaltyForAttackingBots"] = BotParameters.PenaltyForAttackingBots.ToString(CultureInfo.InvariantCulture);

        File.WriteAllText(outPath, result.ToJsonString(new System.Text.Json.JsonSerializerOptions { WriteIndented = true }) + "\n");
    }
}
