// "review-g" mode: probes for the package G review (shipment and movement).
// Each line is "name = value"; tests/review_pkg_g.rs hardcodes the values.
//
//   dotnet run -c Release -- review-g

using System.Reflection;
using Treachery.Shared;

public static class ReviewG
{
    private static void P(string name, object? v) => Console.WriteLine($"{name} = {v ?? "null"}");

    private static string Try(Func<object?> f)
    {
        try { return f()?.ToString() ?? "null"; }
        catch (Exception e) { return e.GetType().Name + ": " + e.Message; }
    }

    private static Game Setup()
    {
        var p = new Participation();
        p.PlayerNames[100] = "Human100";
        p.SeatedPlayers[100] = 2;
        var g = new Game(187, p);
        var e = Utilities.Deserialize<GameEvent>(ReviewA.Establish)!;
        e.Initialize(g);
        if (e.Execute(true, true) != null) throw new Exception("rejected");
        return g;
    }

    public static void Run()
    {
        var g = Setup();
        var parse = typeof(Shipment).GetMethod("ParseForceLocations", BindingFlags.NonPublic | BindingFlags.Static)!;
        string Parse(string s) => Try(() =>
        {
            try
            {
                var d = (Dictionary<Location, Battalion>)parse.Invoke(null, [g, Faction.Emperor, s])!;
                return string.Join(",", d.Select(kv => kv.Key.Id + ":" + kv.Value.AmountOfForces + "|" + kv.Value.AmountOfSpecialForces));
            }
            catch (TargetInvocationException t) { throw t.InnerException!; }
        });
        P("parse.format", Parse("abc:1|1"));
        P("parse.dup", Parse("42:1|0,42:2|0"));
        P("parse.noamounts", Parse("42:1"));
        P("parse.badid", Parse("9999:1|0"));
        P("parse.neg", Parse("-5:1|0,42:3|4"));
        P("parse.overflow", Parse("42:99999999999|0"));

        var pl = g.Players.First(x => x.Faction != Faction.Fremen);
        var to = g.Map.Locations(false).First(l => l.Id != 0 && !l.Territory.HasReducedShippingCost && !l.Territory.IsStronghold);
        P("player0", pl.Faction);
        P("to", to.Id);
        P("cost.wrap", Try(() => Shipment.DetermineCost(g, pl, int.MaxValue, 1, to, false, false, false, false, false)));
        P("cost.big", Try(() => Shipment.DetermineCost(g, pl, 1073741824, 0, to, false, false, false, false, false)));
        P("cost.back.odd", Try(() => Shipment.DetermineCost(g, pl, 16777217, 0, to, false, true, false, false, false)));
        P("cost.back.max", Try(() => Shipment.DetermineCost(g, pl, int.MaxValue, 0, to, false, true, false, false, false)));
        P("cost.back.neg", Try(() => Shipment.DetermineCost(g, pl, -3, 0, to, false, true, false, false, false)));
        P("cost.karma.odd", Try(() => Shipment.DetermineCost(g, pl, 3, 0, to, true, false, false, false, false)));
        var s = new Shipment(g, pl.Faction) { ShipmentType = ShipmentType.ShipmentBack, ForceAmount = int.MinValue, To = to };
        P("cost.minvalue", Try(() => Shipment.DetermineCost(g, pl, s)));
        P("validate.minvalue", Try(() => s.Validate()));
        var s2 = new Shipment(g, pl.Faction) { ShipmentType = ShipmentType.Unknown, ForceAmount = int.MaxValue, SpecialForceAmount = 1, To = to };
        P("validate.unknown.wrapneg", Try(() => s2.Validate()));
        P("validLocations.count", Shipment.ValidShipmentLocations(g, pl, false, false).Count());
        P("validLocations.first", Shipment.ValidShipmentLocations(g, pl, false, false).First().Id);

        // ChoamRemoveForce executed without validation (Game.TryLoad with performValidation: false)
        // and no location: C# returns forces to reserves before ChangeForces(null) throws.
        foreach (var special in new[] { false, true })
        {
            var g2 = Setup();
            var target = g2.Players.First(x => x.Faction != Faction.BeneGesserit && x.Faction != Faction.Richese);
            var actor = g2.Players.First(x => x != target);
            var before = target.ForcesInReserve + "/" + target.SpecialForcesInReserve;
            var json = "{\"$type\":\"ChoamRemoveForce\",\"Target\":" + (int)target.Faction + ",\"SpecialForce\":" + (special ? "true" : "false") + ",\"_locationId\":-1,\"Initiator\":" + (int)actor.Faction + ",\"Time\":\"2024-01-01T12:03:00+00:00\"}";
            var e = Utilities.Deserialize<GameEvent>(json)!;
            e.Initialize(g2);
            var r = e.Execute(false, true);
            var tag = special ? "choamRemoveForce.special" : "choamRemoveForce.normal";
            P(tag + ".json", json);
            P(tag + ".target", target.Faction);
            P(tag + ".before", before);
            P(tag + ".after", target.ForcesInReserve + "/" + target.SpecialForcesInReserve);
            P(tag + ".result", r?.ToString());
        }
    }
}
