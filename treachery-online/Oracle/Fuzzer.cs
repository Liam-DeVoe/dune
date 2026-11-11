using System.Globalization;
using System.Text.Json.Serialization.Metadata;
using Treachery.Oracle.Canon;

namespace Treachery.Oracle;

/// <summary>
/// Builds random events and perturbs bot events. Values are generated per JSON-serialized member of the event,
/// found through System.Text.Json metadata, so every event type is covered without per-type code.
/// Member names select a value domain (card ids, hero ids, locations, ...); see <see cref="GenInt"/> and <see cref="GenString"/>.
/// </summary>
public sealed partial class Fuzzer(Random rng)
{
    private static readonly JsonSerializerOptions Options = new() { IncludeFields = true, TypeInfoResolver = new DefaultJsonTypeInfoResolver() };

    public static readonly Type[] EventTypes = typeof(GameEvent).Assembly.GetTypes()
        .Where(t => t.IsSubclassOf(typeof(GameEvent)) && !t.IsAbstract)
        .OrderBy(t => t.Name, StringComparer.Ordinal)
        .ToArray();

    private static readonly Dictionary<Type, JsonPropertyInfo[]> Settable = EventTypes.ToDictionary(t => t, t =>
        Options.GetTypeInfo(t).Properties
            .Where(p => p.Get != null && p.Set != null && p.Name != nameof(GameEvent.Time) && p.Name != nameof(GameEvent.Initiator))
            .ToArray());

    private static readonly int[] AllCardIds = TreacheryCardManager.GetCardsInAndOutsidePlay().Select(c => c.Id).ToArray();

    private Game _game = null!;
    private Player? _player;

    /// <summary>Creates a random event for a random player (or for nobody, as an observer/host would).</summary>
    public GameEvent? RandomEvent(Game game, Func<Player?, bool, List<Type>> query, out int? actor, out bool isHost)
    {
        _game = game;
        isHost = rng.NextDouble() < 0.5;
        actor = null;
        _player = null;

        if (ClientOnly || rng.NextDouble() < SteerProbability)
        {
            var steered = SteeredEvent(game, query, isHost, out actor);
            if (steered != null || ClientOnly) return steered;
            actor = null;
            _player = null;
        }

        if (game.Players.Count > 0 && rng.NextDouble() < 0.85)
        {
            actor = rng.Next(game.Players.Count);
            _player = game.Players[actor.Value];
        }

        Type type;
        List<Type> applicable;
        try
        {
            applicable = query(_player, isHost);
        }
        catch
        {
            applicable = [];
        }

        if (applicable.Count > 0 && rng.NextDouble() < 0.85)
            type = applicable[rng.Next(applicable.Count)];
        else
            type = EventTypes[rng.Next(EventTypes.Length)];

        var before = HashBefore();
        var initiator = _player?.Faction ?? Faction.None;
        if (rng.NextDouble() < 0.05) initiator = GenFaction();
        var evt = Construct(type, initiator);

        foreach (var p in Settable[type])
        {
            if (rng.NextDouble() < 0.25) continue;
            TrySet(evt, p);
        }

        if (rng.NextDouble() < 0.4) FillSteered(evt);

        CheckAfter(before);
        return evt;
    }

    /// <summary>Returns a copy of <paramref name="original"/> (via its JSON form) with one member set to a random value.</summary>
    public GameEvent? Mutate(Game game, GameEvent original, out string? field)
    {
        _game = game;
        _player = game.GetPlayer(original.Initiator);
        field = null;
        var copy = Utilities.Deserialize<GameEvent>(Utilities.Serialize<GameEvent>(original));
        if (copy == null) return null;
        copy.Initialize(game);

        // The client sends events only as the player's own faction, with values it offers.
        if (ClientOnly)
        {
            field = MutateSteered(copy);
            return field != null ? copy : null;
        }

        var props = Settable[copy.GetType()];
        if (props.Length == 0 || rng.NextDouble() < 0.1)
        {
            field = nameof(GameEvent.Initiator);
            copy.Initialize(game, GenFaction());
            return copy;
        }

        // Half the time the new value comes from the currently valid choices, so the mutant is a different legal move.
        if (rng.NextDouble() < 0.5)
        {
            field = MutateSteered(copy);
            if (field != null) return copy;
        }

        var prop = props[rng.Next(props.Length)];
        field = prop.Name;
        TrySet(copy, prop);
        return copy;
    }

    private void TrySet(GameEvent evt, JsonPropertyInfo p)
    {
        object? value;
        try
        {
            if (!TryGen(p.Name, p.PropertyType, out value)) return;
        }
        catch
        {
            return;
        }

        try
        {
            p.Set!(evt, value);
        }
        catch
        {
            // Some setters parse or validate; an exception here just means this value is skipped.
        }
    }

    private bool TryGen(string name, Type t, out object? value)
    {
        value = null;
        if (t == typeof(bool))
        {
            value = rng.NextDouble() < 0.5;
            return true;
        }

        if (t == typeof(Faction))
        {
            value = GenFaction();
            return true;
        }

        if (t.IsEnum)
        {
            value = GenEnum(t);
            return true;
        }

        if (t == typeof(int))
        {
            value = GenInt(name);
            return true;
        }

        if (t == typeof(string))
        {
            value = GenString(name);
            return true;
        }

        if (t == typeof(Faction[]))
        {
            value = GenFactions().ToArray();
            return true;
        }

        if (t == typeof(List<Faction>))
        {
            value = GenFactions();
            return true;
        }

        if (t == typeof(Rule[]))
        {
            value = GenRules().ToArray();
            return true;
        }

        if (t == typeof(Dictionary<Faction, int>))
        {
            var d = new Dictionary<Faction, int>();
            foreach (var f in GenFactions()) d.TryAdd(f, GenAmount());
            value = d;
            return true;
        }

        if (t == typeof(GameSettings))
        {
            value = new GameSettings
            {
                NumberOfPlayers = rng.Next(-1, 14),
                MaximumTurns = rng.Next(-1, 21),
                InitialRules = GenRules(),
                AllowedFactionsInPlay = GenFactions(),
                AutoOpenEmptySeats = rng.NextDouble() < 0.5
            };
            return true;
        }

        return false;
    }

    #region Domains

    private Faction GenFaction()
    {
        var r = rng.NextDouble();
        var inPlay = _game.Players.Select(p => p.Faction).Where(f => f != Faction.None).ToList();
        if (r < 0.55 && inPlay.Count > 0) return inPlay[rng.Next(inPlay.Count)];
        if (r < 0.65) return Faction.None;
        if (r < 0.75 && _player != null) return _player.Ally;
        if (r < 0.95) return (Faction)GenEnum(typeof(Faction));
        return rng.NextDouble() < 0.5 ? (Faction)99 : (Faction)(-1);
    }

    private object GenEnum(Type t)
    {
        var values = Enum.GetValues(t);
        var r = rng.NextDouble();
        if (r < 0.85) return values.GetValue(rng.Next(values.Length))!;
        if (r < 0.95) return Enum.ToObject(t, 0);
        return Enum.ToObject(t, 999);
    }

    private List<Faction> GenFactions()
    {
        var result = new List<Faction>();
        var count = rng.Next(0, 4);
        for (var i = 0; i < count; i++) result.Add(GenFaction());
        return result;
    }

    private List<Rule> GenRules()
    {
        var all = Enum.GetValues<Rule>();
        var result = new List<Rule>();
        var count = rng.Next(0, 12);
        for (var i = 0; i < count; i++) result.Add(all[rng.Next(all.Length)]);
        return result;
    }

    private int GenAmount()
    {
        var r = rng.NextDouble();
        if (r < 0.45) return rng.Next(0, 6);
        if (r < 0.7) return rng.Next(0, 4);
        if (r < 0.88) return rng.Next(6, 21);
        int[] boundary = [-1, -5, 100, 1000, int.MaxValue, int.MinValue];
        return boundary[rng.Next(boundary.Length)];
    }

    private int GenInt(string name)
    {
        var n = name.ToLowerInvariant();
        // EstablishPlayers loops up to the player count (FillSeats), so huge values only exhaust memory. 0 divides by zero later.
        if (n is "maximumnumberofplayers" or "numberofplayers") return rng.Next(-1, 14);
        if (n == "maximumturns") return rng.Next(-1, 21);
        if (n == "automationruleid" && _game.AutomationRules.Count > 0 && rng.NextDouble() < 0.8)
            return _game.AutomationRules[rng.Next(_game.AutomationRules.Count)].AutomationRuleId;
        if (n.Contains("card")) return GenCardId();
        if (n.Contains("hero") || n.Contains("traitor") || n.Contains("leader") || n.Contains("dancer")) return GenHeroId();
        if (n.Contains("territory") || n.Contains("stronghold")) return GenTerritoryId();
        if (n.StartsWith('_') && (n.Contains("to") || n.Contains("from") || n.Contains("location") || n.Contains("target") || n.Contains("sneak"))) return GenLocationId();
        if (n == "seat") return rng.Next(-1, _game.Players.Count + 2);
        if (n == "turn") return rng.Next(-1, 13);
        if (n == "direction") return rng.Next(-1, 3);
        return GenAmount();
    }

    private int GenCardId()
    {
        var r = rng.NextDouble();
        if (r < 0.4 && _player != null && _player.TreacheryCards.Count > 0) return _player.TreacheryCards[rng.Next(_player.TreacheryCards.Count)].Id;
        if (r < 0.6)
        {
            var held = _game.Players.SelectMany(p => p.TreacheryCards).ToList();
            if (held.Count > 0) return held[rng.Next(held.Count)].Id;
        }

        if (r < 0.85) return AllCardIds[rng.Next(AllCardIds.Length)];
        if (r < 0.95) return -1;
        return rng.NextDouble() < 0.5 ? -2 : 999;
    }

    private int GenHeroId()
    {
        var r = rng.NextDouble();
        if (r < 0.35 && _player != null && _player.Leaders.Count > 0) return _player.Leaders[rng.Next(_player.Leaders.Count)].Id;
        if (r < 0.5 && _player != null && _player.Traitors.Count > 0) return _player.Traitors[rng.Next(_player.Traitors.Count)].Id;
        if (r < 0.75)
        {
            var leaders = _game.Players.SelectMany(p => p.Leaders).ToList();
            if (leaders.Count > 0) return leaders[rng.Next(leaders.Count)].Id;
        }

        if (r < 0.82) return LeaderManager.Leaders[rng.Next(LeaderManager.Leaders.Count)].Id;
        if (r < 0.86) return LeaderManager.Messiah.Id;
        if (r < 0.9) return rng.Next(17, 20);
        if (r < 0.97) return -1;
        return 99999;
    }

    private int GenLocationId()
    {
        var locations = StateDumper.MapLocations(_game.Map);
        var r = rng.NextDouble();
        if (r < 0.35 && _player != null && _player.ForcesInLocations.Count > 0)
            return _player.ForcesInLocations.Keys.ElementAt(rng.Next(_player.ForcesInLocations.Count)).Id;
        if (r < 0.5)
        {
            var strongholds = _game.Map.Strongholds.ToList();
            return strongholds[rng.Next(strongholds.Count)].Id;
        }

        if (r < 0.88) return locations[rng.Next(locations.Count)].Id;
        if (r < 0.96) return -1;
        return locations.Count + rng.Next(0, 5);
    }

    private int GenTerritoryId()
    {
        var r = rng.NextDouble();
        if (r < 0.9)
        {
            var territories = _game.Map.Territories(true).ToList();
            return territories[rng.Next(territories.Count)].Id;
        }

        return r < 0.97 ? -1 : 9999;
    }

    private string? GenString(string name)
    {
        var n = name.ToLowerInvariant();
        var r = rng.NextDouble();

        if (n.EndsWith("forcelocations"))
        {
            if (r < 0.08) return Garbage();
            var count = rng.Next(0, 4);
            var parts = new List<string>();
            for (var i = 0; i < count; i++)
                parts.Add(GenLocationId() + ":" + GenSmallAmount() + "|" + GenSmallAmount());
            return string.Join(',', parts);
        }

        if (n.EndsWith("ids"))
        {
            if (r < 0.05) return Garbage();
            var count = rng.Next(0, 5);
            var ids = new List<int>();
            for (var i = 0; i < count; i++) ids.Add(n.Contains("card") ? GenCardId() : GenHeroId());
            return string.Join(';', ids);
        }

        if (n == "_players")
        {
            return string.Join('>', Enumerable.Range(0, rng.Next(0, 4)).Select(i => "P" + i));
        }

        if (n == "_factionsinplay")
        {
            if (r < 0.1) return Garbage();
            return string.Join(',', GenFactions().Select(f => f.ToString()));
        }

        if (n.StartsWith("questionparameter") || n.StartsWith("dealparameter"))
        {
            return rng.Next(8) switch
            {
                0 => null,
                1 => GenEnum(typeof(TreacheryCardType)).ToString(),
                2 => GenHeroId().ToString(CultureInfo.InvariantCulture),
                3 => GenFaction().ToString(),
                4 => GenTerritoryId().ToString(CultureInfo.InvariantCulture),
                5 => (rng.Next(0, 40) / 2.0).ToString(CultureInfo.InvariantCulture),
                6 => GenAmount().ToString(CultureInfo.InvariantCulture),
                _ => Garbage()
            };
        }

        if (n == "initiatorplayername")
        {
            if (r < 0.7 && _game.Players.Count > 0) return _game.Players[rng.Next(_game.Players.Count)].Name;
            return Garbage();
        }

        return r < 0.3 ? null : Garbage();
    }

    private int GenSmallAmount()
    {
        var r = rng.NextDouble();
        if (r < 0.9) return rng.Next(0, 6);
        return r < 0.95 ? -1 : 25;
    }

    private string Garbage()
    {
        string[] options = ["", "x", "1:", ":1|1", "-1", "1;;2", "999999999999", "Ünïcødé \"quoted\"\n", "1,2", "0:0|0,0:0|0"];
        return options[rng.Next(options.Length)];
    }

    #endregion
}
