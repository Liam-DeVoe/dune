// "review-b" mode: probes for the package B review (Player.cs, PlayerSequence.cs,
// Battalion.cs, Game_Rules.cs, Game_Events.cs). tests/review_pkg_b.rs replays the same
// operations on the Rust engine and compares with the lines written here.
//
//   dotnet run -c Release -- review-b ../tests/review_pkg_b/probes.txt

using System.Reflection;
using System.Text;
using Treachery.Shared;
using Treachery.Shared.Model;

public static class ReviewB
{
    // xorshift32, identical in tests/review_pkg_b.rs.
    private sealed class Rng(uint seed)
    {
        private uint _s = seed == 0 ? 1u : seed;
        public uint Next() { _s ^= _s << 13; _s ^= _s >> 17; _s ^= _s << 5; return _s; }
        public int Below(int n) => (int)(Next() % (uint)n);
        public bool Bool() => Below(2) == 1;
    }

    public static readonly Faction[] Factions = [Faction.Emperor, Faction.BeneGesserit, Faction.Richese, Faction.Fremen, Faction.Ixian, Faction.Choam];

    private static void Set(object o, string prop, object? v) =>
        o.GetType().GetProperty(prop, BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Instance)!.SetValue(o, v);

    public static Game NewGame(int version)
    {
        var g = new Game(version, new Participation());
        g.Rules.Add(Rule.Homeworlds);
        g.Rules.Add(Rule.EmperorSpecialForces);
        g.Rules.Add(Rule.FremenSpecialForces);
        for (var i = 0; i < Factions.Length; i++)
        {
            var p = new Player(g, Factions[i]) { Seat = i };
            g.Players.Add(p);
        }
        foreach (var p in g.Players)
            foreach (var hw in g.Map.Homeworlds.Where(w => w.Faction == p.Faction))
                p.InitializeHomeworld(hw, hw.IsHomeOfNormalForces ? 10 : 0, hw.IsHomeOfSpecialForces ? 4 : 0);
        return g;
    }

    public static List<Location> LocationPool(Game g) =>
        g.Map.Locations(true).Where(l => l.Id % 9 == 0 || l is Homeworld).ToList();

    public static List<Territory> TerritoryPool(Game g) =>
        LocationPool(g).Select(l => l.Territory).Distinct().ToList();

    private static string State(Game g, Player p)
    {
        var sb = new StringBuilder();
        sb.Append(p.Faction).Append(" [");
        sb.Append(string.Join(",", p.ForcesInLocations.Select(kv => $"{kv.Key.Id}:{kv.Value.AmountOfForces}/{kv.Value.AmountOfSpecialForces}")));
        sb.Append($"] k={p.ForcesKilled}/{p.SpecialForcesKilled}/{p.TotalForcesKilledInBattle}");
        sb.Append($" res={p.ForcesInReserve}/{p.SpecialForcesInReserve}");
        sb.Append($" hi={p.HasHighThreshold()} lo={p.HasLowThreshold()}");
        foreach (var hw in p.HomeWorlds) sb.Append($" w{(int)hw.World}={p.HasHighThreshold(hw.World)}/{p.HasLowThreshold(hw.World)}/{p.GetHomeworldBattleContributionAndLasgunShieldLimit(hw.Territory)}");
        sb.Append($" nf={p.NoFieldIsActive}");
        sb.Append(" occ=").Append(string.Join(",", p.OccupiedTerritories.Select(t => t.Id)));
        sb.Append(" twf=").Append(string.Join(",", p.TerritoriesWithForces.Select(t => t.Id)));
        sb.Append(" planet=").Append(string.Join(",", p.ForcesOnPlanet.Keys.Select(l => l.Id)));
        sb.Append(" each=").Append(string.Join("", g.Players.Select(o => p.HaveForcesOnEachOthersHomeWorlds(o) ? "1" : "0")));
        return sb.ToString();
    }

    public static void Forces(TextWriter w)
    {
        foreach (var version in new[] { 163, 187 })
        {
            for (uint seed = 1; seed <= 20; seed++)
            {
                var g = NewGame(version);
                var locs = LocationPool(g);
                var terrs = TerritoryPool(g);
                var hws = g.Map.Homeworlds.ToList();
                if (seed == 1) w.WriteLine($"pool {version} " + string.Join(",", locs.Select(l => l.Id)) + " | " + string.Join(",", terrs.Select(t => t.Id)));
                var r = new Rng(seed * 2654435761u);
                for (var step = 0; step < 120; step++)
                {
                    var p = g.Players[r.Below(g.Players.Count)];
                    var op = r.Below(25);
                    var l = locs[r.Below(locs.Count)];
                    var l2 = locs[r.Below(locs.Count)];
                    var t = terrs[r.Below(terrs.Count)];
                    var hw = hws[r.Below(hws.Count)];
                    var n = r.Below(9) - 2;
                    var m = r.Below(9) - 2;
                    var b = r.Bool();
                    string res;
                    try
                    {
                        object? ret = null;
                        switch (op)
                        {
                            case 0: p.AddForces(l, n, b); break;
                            case 1: p.AddSpecialForces(l, n, b); break;
                            case 2: p.AddForcesToReserves(n); break;
                            case 3: p.AddSpecialForcesToReserves(n); break;
                            case 4: p.RemoveForces(l, n); break;
                            case 5: p.RemoveSpecialForces(l, n); break;
                            case 6: p.ForcesToReserves(l, n); break;
                            case 7: p.SpecialForcesToReserves(l, n); break;
                            case 8: p.ForcesToReserves(t, n, b); break;
                            case 9: p.ForcesToReserves(t); break;
                            case 10: ret = p.KillAllForces(l, b); break;
                            case 11: p.KillAllForces(t, b); break;
                            case 12: ret = p.KillForces(l, n, m, b); break;
                            case 13: p.KillForces(t, n, b, r.Bool()); break;
                            case 14: p.ReviveForces(n); break;
                            case 15: p.ReviveSpecialForces(n); break;
                            case 16: p.ShipForces(l, n); break;
                            case 17: p.ShipForces(l, hw, n); break;
                            case 18: p.ShipSpecialForces(l, n); break;
                            case 19: p.ShipSpecialForces(l, hw, n); break;
                            case 20: p.ShipAdvisors(l, n); break;
                            case 21: p.MoveForces(l, l2, n); break;
                            case 22: p.MoveSpecialForces(l, l2, n); break;
                            case 23: p.FlipForces(l, b); break;
                            case 24: p.FlipForces(t, b); break;
                        }
                        res = ret?.ToString() ?? "-";
                    }
                    catch (Exception e)
                    {
                        res = "exception:" + e.GetType().Name + ":" + e.Message;
                    }
                    w.WriteLine($"{version} {seed} {step} op{op} {res} {State(g, p)}");
                }
            }
        }
    }

    // PlayerSequence.DetermineFirstPlayer over storm sectors and player counts, with
    // some seats left empty.
    public static void FirstPlayer(TextWriter w)
    {
        for (var players = 1; players <= 12; players++)
        {
            for (var sector = -2; sector <= 19; sector++)
            {
                var g = new Game(187, new Participation());
                var settings = new GameSettings { NumberOfPlayers = players };
                Set(g, "Settings", settings);
                for (var i = 0; i < players; i++)
                {
                    if (i % 3 == 1) continue;
                    g.Players.Add(new Player(g, Factions[i % Factions.Length]) { Seat = i });
                }
                Set(g, "SectorInStorm", sector);
                string res;
                try
                {
                    var fp = PlayerSequence.DetermineFirstPlayer(g);
                    res = fp.Seat.ToString();
                }
                catch (Exception e)
                {
                    res = "exception:" + e.GetType().Name + ":" + e.Message;
                }
                w.WriteLine($"first {players} {sector} {res}");
            }
        }
    }


    private static object? Get(object o, string member)
    {
        var t = o.GetType();
        var f = t.GetField(member, BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Instance);
        if (f != null) return f.GetValue(o);
        return t.GetProperty(member, BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Instance)!.GetValue(o);
    }

    private static string Seat(Player? p) => p == null ? "null" : p.Seat.ToString();

    private static string Try(Func<string> f)
    {
        try { return f(); }
        catch (Exception e) { return "exception:" + e.GetType().Name + ":" + e.Message; }
    }

    private static string SeqState(Game g, PlayerSequence s)
    {
        var played = (List<Player>)Get(s, "_played")!;
        return $"fp={Seat((Player?)Get(s, "FirstPlayer"))} cf={Seat((Player?)Get(s, "<CurrentPlayer>k__BackingField"))}" +
               $" played={string.Join(",", played.Select(Seat))} round={Get(s, "_round")} circ={Get(s, "_fullyCircled")}" +
               $" cur={Try(() => Seat(s.CurrentPlayer))} richese={Try(() => s.HasPassedRichese.ToString())}" +
               $" seq={Try(() => string.Join(",", s.GetPlayersInSequence().Select(e => Seat(e.Player) + (e.HasTurn ? "*" : ""))))}";
    }

    // PlayerSequence: random seats, full hands, juice, start player and directions, then a
    // random series of NextPlayer / NextRound / CheckCurrentPlayer.
    public static void Sequences(TextWriter w)
    {
        var cards = TreacheryCardManager.Items;
        foreach (var version in new[] { 117, 187 })
        {
            for (uint seed = 1; seed <= 150; seed++)
            {
                var r = new Rng(seed * 40503u + 7u);
                var g = new Game(version, new Participation());
                var count = 1 + r.Below(6);
                Set(g, "Settings", new GameSettings { NumberOfPlayers = count + r.Below(2) });
                var factions = new[] { Faction.Atreides, Faction.Richese, Faction.Harkonnen, Faction.Emperor, Faction.Choam, Faction.BeneGesserit, Faction.Tleilaxu };
                for (var i = 0; i < count; i++)
                {
                    var p = new Player(g, factions[(i + (int)seed) % factions.Length]) { Seat = r.Below(4) == 0 ? r.Below(count + 1) : i };
                    var full = r.Below(3) == 0 ? 8 : r.Below(5);
                    for (var c = 0; c < full; c++) p.TreacheryCards.Add(cards[c]);
                    g.Players.Add(p);
                }
                Set(g, "SectorInStorm", r.Below(18));
                var juice = r.Below(4);
                if (juice > 0)
                {
                    var j = new JuicePlayed(g, g.Players[r.Below(count)].Faction) { Type = juice == 1 ? JuiceType.GoFirst : juice == 2 ? JuiceType.GoLast : JuiceType.Aggressor };
                    Set(g, "CurrentJuice", j);
                }
                var skip = r.Bool();
                var dir = r.Bool() ? 1 : -1;
                var begin = r.Bool();
                var startIdx = r.Below(count + 1);
                var nulls = startIdx == count;
                if (nulls && !(begin && count >= 2 && !skip)) startIdx = 0;
                Player? start = startIdx == count ? null : g.Players[startIdx];
                var kind = r.Below(3);
                PlayerSequence? s = null;
                var head = $"seq {version} {seed} kind={kind} skip={skip} dir={dir} begin={begin} start={Seat(start)}";
                var ctor = Try(() =>
                {
                    s = kind switch
                    {
                        0 => new PlayerSequence(g, skip, dir, start!, begin),
                        1 => new PlayerSequence(g, skip, dir),
                        _ => new PlayerSequence(g)
                    };
                    return "ok";
                });
                w.WriteLine($"{head} {ctor}");
                if (s == null) continue;
                w.WriteLine($"{head} init {SeqState(g, s)}");
                for (var step = 0; step < 14; step++)
                {
                    var op = r.Below(5);
                    if (op == 4 && g.Players.Count > 0)
                    {
                        var p = g.Players[r.Below(count)];
                        if (p.TreacheryCards.Count > 0) p.TreacheryCards.RemoveAt(0); else p.TreacheryCards.Add(cards[0]);
                    }
                    var res = Try(() =>
                    {
                        switch (op)
                        {
                            case 0: case 1: s.NextPlayer(); break;
                            case 2: s.NextRound(); break;
                            case 3: s.CheckCurrentPlayer(); break;
                        }
                        return "ok";
                    });
                    w.WriteLine($"{head} {step} op{op} {res} {SeqState(g, s)}");
                }
                var a = g.Players[r.Below(count)];
                var bb = g.Players[r.Below(count)];
                w.WriteLine($"{head} isafter {Try(() => PlayerSequence.IsAfter(g, a, bb).ToString())}");
            }
        }
    }

    public static void Run(string path)
    {
        using var w = new StreamWriter(path) { NewLine = "\n" };
        Forces(w);
        FirstPlayer(w);
        Sequences(w);
    }
}
