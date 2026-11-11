using System.Collections;
using System.Collections.Concurrent;
using System.Text.Json.Serialization.Metadata;
using Treachery.Oracle.Canon;

namespace Treachery.Oracle;

/// <summary>
/// Steered event generation. Instead of filling an event with values from broad domains, it draws values from what is
/// currently allowed, using the engine's own static helpers on the event type (<c>Donated.ValidTargets</c>,
/// <c>Battle.ValidWeapons</c>, <c>Retreat.MaxForces</c>, ...), found by reflection. It tries several candidates and
/// prefers one that passes <c>Validate()</c>, and it prefers event types this game has not executed often.
///
/// Everything here only reads engine state. The helpers and <c>Validate()</c> are the same calls bots and the client
/// make; none of them reach <c>Game.NextPlayerToBattle</c> (hazards.md 4.3) or <c>Game.Random</c>. Run a fuzz batch
/// with <c>--check-bot-purity true</c> and <c>verify</c> to check that this stays true.
/// </summary>
public sealed partial class Fuzzer
{
    /// <summary>Probability that a fuzz step is steered (all players queried, novelty-weighted type, valid values).</summary>
    private const double SteerProbability = 0.65;

    /// <summary>Candidates tried per steered event, looking for one that passes Validate().</summary>
    private const int Attempts = 8;

    /// <summary>Probability of submitting the valid candidate when one was found (otherwise the last, invalid one).</summary>
    private const double SubmitValidProbability = 0.9;

    // Executions and opportunistic tries per (event type, phase it was submitted in). The same event type does
    // different things in different phases, so novelty is judged per phase.
    private readonly Dictionary<(Type, Phase), int> _accepted = new();
    private readonly Dictionary<(Type, Phase), int> _opportunisticTries = new();

    /// <summary>Set by --check-bot-purity: hashes the game state, to check that generation after the queries changes nothing.</summary>
    public Func<Game, string>? StateHasher { get; set; }

    /// <summary>True if the state hash changed while an event was being built (after the queries). Reset by the caller.</summary>
    public bool StateChangedDuringGeneration { get; set; }

    private string? HashBefore() => StateHasher?.Invoke(_game);

    private void CheckAfter(string? before)
    {
        if (before != null && StateHasher!(_game) != before) StateChangedDuringGeneration = true;
    }

    /// <summary>An event of any type for a random player (or nobody), filled with steered values, without querying applicability.</summary>
    public GameEvent AnyTypeEvent(Game game, out int? actor, out bool isHost)
    {
        _game = game;
        isHost = rng.NextDouble() < 0.5;
        actor = game.Players.Count > 0 && rng.NextDouble() < 0.85 ? rng.Next(game.Players.Count) : null;
        _player = actor is { } a ? game.Players[a] : null;
        return BuildSteered(EventTypes[rng.Next(EventTypes.Length)], false);
    }

    /// <summary>
    /// Looks at the event types each player may submit (as listed for the bots) and, if one has rarely been executed in
    /// this game, builds a steered event of that type. Each type gets a limited number of such tries.
    /// </summary>
    public GameEvent? Opportunistic(Game game, List<Type>[] applicable, bool isHost, out int actor)
    {
        _game = game;
        actor = -1;
        var candidates = new List<(int Actor, Type Type)>();
        for (var i = 0; i < applicable.Length && i < game.Players.Count; i++)
            foreach (var t in applicable[i])
                if (_accepted.GetValueOrDefault((t, game.CurrentPhase)) < 2 && _opportunisticTries.GetValueOrDefault((t, game.CurrentPhase)) < 15)
                    candidates.Add((i, t));

        if (candidates.Count == 0) return null;
        var pick = candidates[rng.Next(candidates.Count)];
        var key = (pick.Type, game.CurrentPhase);
        _opportunisticTries[key] = _opportunisticTries.GetValueOrDefault(key) + 1;
        actor = pick.Actor;
        _player = game.Players[actor];
        return BuildSteered(pick.Type, true);
    }

    /// <summary>Tells the fuzzer an event of this type was executed in this phase, by any source. Rare pairs get picked more often.</summary>
    public void NoteAccepted(Type t, Phase phase) => _accepted[(t, phase)] = _accepted.GetValueOrDefault((t, phase)) + 1;

    #region Reflection model

    /// <summary>A value of an event the generator can set: a JSON member, or a [JsonIgnore] property whose setter writes one (Card, Target, ForceLocations, ...).</summary>
    private sealed record Slot(string Name, Type Type, Func<object, object?> Get, Action<object, object?> Set, int Order);

    private static readonly ConcurrentDictionary<Type, Slot[]> SlotCache = new();
    private static readonly ConcurrentDictionary<Type, MethodInfo[]> HelperCache = new();

    private static Slot[] SlotsOf(Type t) => SlotCache.GetOrAdd(t, type =>
    {
        var result = new List<Slot>();
        foreach (var p in Settable[type])
        {
            // Raw ids and id strings are set through their typed property.
            if (p.Name.StartsWith('_')) continue;
            result.Add(new Slot(p.Name, p.PropertyType, o => p.Get!(o), (o, v) => p.Set!(o, v), OrderOf(p.PropertyType)));
        }

        foreach (var p in type.GetProperties(BindingFlags.Public | BindingFlags.Instance).OrderBy(p => p.Name, StringComparer.Ordinal).ThenBy(p => p.MetadataToken))
        {
            if (p.DeclaringType == typeof(GameEvent) || p.GetIndexParameters().Length > 0) continue;
            if (p.GetCustomAttribute<JsonIgnoreAttribute>() == null) continue;
            if (p.GetMethod?.IsPublic != true || p.SetMethod?.IsPublic != true) continue;
            if (p.PropertyType == typeof(Game) || p.PropertyType == typeof(Player)) continue;
            if (result.Any(s => s.Name == p.Name)) continue;
            result.Add(new Slot(p.Name, p.PropertyType, o => p.GetValue(o), (o, v) => p.SetValue(o, v), OrderOf(p.PropertyType)));
        }

        return result.OrderBy(s => s.Order).ThenBy(s => s.Name, StringComparer.Ordinal).ToArray();
    });

    // Values other values depend on come first: a hero before its weapon, forces before their destination, amounts last.
    private static int OrderOf(Type t)
    {
        if (t == typeof(bool)) return 0;
        if (t == typeof(Dictionary<Location, Battalion>)) return 1;
        if (typeof(IHero).IsAssignableFrom(t)) return 2;
        if (t == typeof(TreacheryCard) || t == typeof(IEnumerable<TreacheryCard>)) return 3;
        if (t == typeof(Faction) || t.IsEnum) return 4;
        if (typeof(Territory).IsAssignableFrom(t)) return 5;
        if (typeof(Location).IsAssignableFrom(t)) return 6;
        if (t == typeof(int)) return 8;
        return 7;
    }

    /// <summary>Public static helpers of an event type (and its base types) that list or bound valid values.</summary>
    private static MethodInfo[] HelpersOf(Type t) => HelperCache.GetOrAdd(t, type =>
        type.GetMethods(BindingFlags.Public | BindingFlags.Static | BindingFlags.FlattenHierarchy)
            .Where(m => !m.IsGenericMethodDefinition && !m.IsSpecialName)
            .Where(m => (m.Name.StartsWith("Valid", StringComparison.Ordinal) && !m.Name.StartsWith("Validate", StringComparison.Ordinal))
                        || m.Name.StartsWith("Max", StringComparison.Ordinal)
                        || m.Name.StartsWith("GetValid", StringComparison.Ordinal)
                        || m.Name.StartsWith("GetAvailable", StringComparison.Ordinal)
                        || m.Name.StartsWith("GetStandard", StringComparison.Ordinal))
            .Where(m => m.ReturnType == typeof(int) || ElementType(m.ReturnType) != null)
            .OrderBy(m => m.Name, StringComparer.Ordinal).ThenBy(m => m.GetParameters().Length).ThenBy(m => m.MetadataToken)
            .ToArray());

    private static Type? ElementType(Type t)
    {
        if (t == typeof(string)) return null;
        if (t.IsGenericType && t.GetGenericTypeDefinition() == typeof(IEnumerable<>)) return t.GetGenericArguments()[0];
        return t.GetInterfaces().FirstOrDefault(i => i.IsGenericType && i.GetGenericTypeDefinition() == typeof(IEnumerable<>))?.GetGenericArguments()[0];
    }

    #endregion

    #region Steered events

    /// <summary>
    /// Queries every player (and nobody), picks a (player, event type) pair weighted towards types this game has rarely
    /// executed, and fills the event from currently valid values. Returns null when nothing is applicable.
    /// </summary>
    private GameEvent? SteeredEvent(Game game, Func<Player?, bool, List<Type>> query, bool isHost, out int? actor)
    {
        actor = null;
        var candidates = new List<(int? Actor, Type Type)>();
        for (var i = 0; i < game.Players.Count; i++)
            foreach (var t in SafeQuery(query, game.Players[i], isHost))
                candidates.Add((i, t));
        foreach (var t in SafeQuery(query, null, isHost))
            candidates.Add((null, t));

        if (candidates.Count == 0) return null;

        var weights = candidates.Select(c => 1.0 / Math.Pow(1 + _accepted.GetValueOrDefault((c.Type, game.CurrentPhase)), 1.5)).ToArray();
        var pick = candidates[WeightedIndex(weights)];
        actor = pick.Actor;
        _player = actor is { } a ? game.Players[a] : null;
        var before = HashBefore();
        var result = BuildSteered(pick.Type, true);
        CheckAfter(before);
        return result;
    }

    private static List<Type> SafeQuery(Func<Player?, bool, List<Type>> query, Player? p, bool isHost)
    {
        try
        {
            return query(p, isHost);
        }
        catch
        {
            return [];
        }
    }

    private int WeightedIndex(double[] weights)
    {
        var total = weights.Sum();
        var x = rng.NextDouble() * total;
        for (var i = 0; i < weights.Length; i++)
        {
            x -= weights[i];
            if (x < 0) return i;
        }

        return weights.Length - 1;
    }

    /// <summary>Builds up to <see cref="Attempts"/> candidates and returns a valid one if found (usually).</summary>
    private GameEvent BuildSteered(Type type, bool search)
    {
        var initiator = _player?.Faction ?? Faction.None;
        GameEvent? last = null;
        var tries = search ? Attempts : 1;
        for (var i = 0; i < tries; i++)
        {
            var evt = Construct(type, initiator);
            // A share of raw values keeps invalid and boundary values in the mix (wide profile only).
            if (!ClientOnly && rng.NextDouble() < 0.3)
                foreach (var p in Settable[type])
                    if (rng.NextDouble() < 0.3) TrySet(evt, p);

            FillSteered(evt);
            last = evt;
            if (search && IsValid(evt))
            {
                if (rng.NextDouble() < SubmitValidProbability) return evt;
                search = false;
            }
        }

        return last!;
    }

    /// <summary>Creates the event with its (Game, Faction) constructor when it has one, as bots and the client do.</summary>
    private GameEvent Construct(Type type, Faction initiator)
    {
        var ctor = type.GetConstructor([typeof(Game), typeof(Faction)]);
        if (ctor != null && rng.NextDouble() < 0.5)
        {
            try
            {
                return (GameEvent)ctor.Invoke([_game, initiator]);
            }
            catch
            {
                // Fall back to the parameterless constructor.
            }
        }

        var evt = (GameEvent)Activator.CreateInstance(type)!;
        evt.Initialize(_game, initiator);
        return evt;
    }

    private static bool IsValid(GameEvent evt)
    {
        try
        {
            return evt.Validate() == null;
        }
        catch
        {
            return false;
        }
    }

    private void FillSteered(GameEvent evt)
    {
        foreach (var slot in SlotsOf(evt.GetType()))
        {
            if (rng.NextDouble() < 0.1) continue;
            TrySetSteered(evt, slot);
        }
    }

    private void TrySetSteered(GameEvent evt, Slot slot)
    {
        object? value;
        try
        {
            if (!TryGenSteered(evt, slot, out value)) return;
        }
        catch
        {
            return;
        }

        try
        {
            slot.Set(evt, value);
        }
        catch
        {
            // Setters that parse or look up values may throw; the value is skipped.
        }
    }

    private bool TryGenSteered(GameEvent evt, Slot slot, out object? value)
    {
        value = null;
        var t = slot.Type;

        if (t == typeof(bool))
        {
            value = rng.NextDouble() < 0.5;
            return true;
        }

        if (t == typeof(int)) return TryGenSteeredInt(evt, slot, out value);

        if (t == typeof(Dictionary<Location, Battalion>))
        {
            value = GenForces(evt);
            return true;
        }

        var elem = t.IsGenericType && (t.GetGenericTypeDefinition() == typeof(IEnumerable<>) || t.GetGenericTypeDefinition() == typeof(List<>)) ? t.GetGenericArguments()[0] : null;
        if (elem != null)
        {
            var pool = Pool(evt, elem, slot.Name);
            if (pool == null) return false;
            var subset = pool.Where(_ => rng.NextDouble() < 0.5).ToList();
            if (subset.Count == 0 && pool.Count > 0 && rng.NextDouble() < 0.7) subset.Add(pool[rng.Next(pool.Count)]);
            var list = (IList)Activator.CreateInstance(typeof(List<>).MakeGenericType(elem))!;
            foreach (var x in subset) list.Add(x);
            value = list;
            return true;
        }

        if (t.IsValueType && !t.IsEnum) return false;

        var items = Pool(evt, t, slot.Name);
        if (items == null) return false;
        if (items.Count == 0 || (!ClientOnly && !t.IsValueType && rng.NextDouble() < 0.05))
        {
            value = t.IsValueType ? Activator.CreateInstance(t) : null;
            return true;
        }

        value = items[rng.Next(items.Count)];
        return true;
    }

    /// <summary>Candidate values of type <paramref name="t"/>: usually from a matching Valid* helper, otherwise from the game state.</summary>
    private List<object?>? Pool(GameEvent evt, Type t, string slotName)
    {
        var helpers = HelpersOf(evt.GetType()).Where(m => ElementType(m.ReturnType) is { } e && Compatible(t, e)).ToList();
        if (helpers.Count > 0 && rng.NextDouble() < 0.85)
        {
            var named = helpers.Where(m => NameMatches(m.Name, slotName)).ToList();
            var chosen = named.Count > 0 && rng.NextDouble() < 0.8 ? named[rng.Next(named.Count)] : helpers[rng.Next(helpers.Count)];
            var result = Invoke(evt, chosen) is IEnumerable e ? Materialize(e) : null;
            if (result is { Count: > 0 }) return result;
        }

        // The client offers objects only through such helpers. Without one, leave the value as constructed, except
        // for the player's own cards, leaders and force locations, which the client lists anyway.
        if (ClientOnly) return OwnPool(t);
        return FallbackPool(t);
    }

    private static bool Compatible(Type slot, Type element)
    {
        var s = Nullable.GetUnderlyingType(slot) ?? slot;
        var e = Nullable.GetUnderlyingType(element) ?? element;
        return s.IsAssignableFrom(e);
    }

    private static bool NameMatches(string method, string slot)
    {
        var core = method.StartsWith("GetValid", StringComparison.Ordinal) ? method[8..]
            : method.StartsWith("GetAvailable", StringComparison.Ordinal) ? method[12..]
            : method.StartsWith("GetStandard", StringComparison.Ordinal) ? method[11..]
            : method.StartsWith("Valid", StringComparison.Ordinal) ? method[5..] : method;
        return core.Contains(slot, StringComparison.OrdinalIgnoreCase) || (core.Length > 3 && slot.Contains(core.TrimEnd('s'), StringComparison.OrdinalIgnoreCase));
    }

    private static List<object?>? Materialize(IEnumerable e)
    {
        try
        {
            var result = new List<object?>();
            foreach (var x in e)
            {
                result.Add(x);
                if (result.Count > 500) break;
            }

            return result;
        }
        catch
        {
            return null;
        }
    }

    private List<object?>? OwnPool(Type t)
    {
        var me = _player;
        if (me == null) return null;
        List<object?> list;
        if (t == typeof(TreacheryCard)) list = [.. me.TreacheryCards];
        else if (typeof(IHero).IsAssignableFrom(t) || t == typeof(IHero)) list = me.Leaders.Where(t.IsInstanceOfType).Cast<object?>().ToList();
        else if (typeof(Location).IsAssignableFrom(t)) list = me.ForcesInLocations.Keys.Where(t.IsInstanceOfType).Cast<object?>().ToList();
        else if (t == typeof(Faction)) list = _game.Players.Select(p => p.Faction).Where(f => f != Faction.None && f != me.Faction).Cast<object?>().ToList();
        else if (t.IsEnum) list = Enum.GetValues(t).Cast<object?>().ToList();
        else return null;
        return list.Count > 0 ? list : null;
    }

    private List<object?>? FallbackPool(Type t)
    {
        var players = _game.Players;
        var me = _player;
        var ally = me?.AlliedPlayer;
        var r = rng.NextDouble();

        if (t == typeof(TreacheryCard))
        {
            if (r < 0.6 && me != null && me.TreacheryCards.Count > 0) return [.. me.TreacheryCards];
            if (r < 0.75 && ally != null && ally.TreacheryCards.Count > 0) return [.. ally.TreacheryCards];
            var held = players.SelectMany(p => p.TreacheryCards).Cast<object?>().ToList();
            return held.Count > 0 && r < 0.95 ? held : TreacheryCardManager.GetCardsInAndOutsidePlay().Cast<object?>().ToList();
        }

        if (typeof(IHero).IsAssignableFrom(t) || t == typeof(IHero))
        {
            IEnumerable<object> heroes;
            if (r < 0.5 && me != null) heroes = me.Leaders.Cast<object>().Concat(me.Traitors).Concat(me.FaceDancers);
            else if (r < 0.9) heroes = players.SelectMany(p => p.Leaders);
            else heroes = LeaderManager.Leaders.Cast<object>().Append(LeaderManager.Messiah);
            var list = heroes.Where(t.IsInstanceOfType).Cast<object?>().ToList();
            return list.Count > 0 ? list : null;
        }

        if (typeof(Location).IsAssignableFrom(t))
        {
            IEnumerable<Location> locs = r < 0.5 && me != null && me.ForcesInLocations.Count > 0
                ? me.ForcesInLocations.Keys
                : StateDumper.MapLocations(_game.Map);
            var list = locs.Where(t.IsInstanceOfType).Cast<object?>().ToList();
            return list.Count > 0 ? list : null;
        }

        if (t == typeof(Territory))
        {
            IEnumerable<Territory> ts = r < 0.5 && me != null ? me.TerritoriesWithForces : _game.Map.Territories(true);
            var list = ts.Cast<object?>().ToList();
            return list.Count > 0 ? list : _game.Map.Territories(true).Cast<object?>().ToList();
        }

        if (t == typeof(Faction))
        {
            var list = players.Select(p => p.Faction).Where(f => f != Faction.None && (me == null || f != me.Faction || r < 0.1)).Cast<object?>().ToList();
            if (list.Count == 0) list.Add(Faction.None);
            return list;
        }

        if (t.IsEnum) return Enum.GetValues(t).Cast<object?>().ToList();

        return null;
    }

    /// <summary>Ints: a value within the bound of a matching Max* helper, or from a matching IEnumerable&lt;int&gt; helper.</summary>
    private bool TryGenSteeredInt(GameEvent evt, Slot slot, out object? value)
    {
        value = null;
        var helpers = HelpersOf(evt.GetType());
        var lists = helpers.Where(m => ElementType(m.ReturnType) == typeof(int) && NameMatches(m.Name, slot.Name)).ToList();
        if (lists.Count > 0 && rng.NextDouble() < 0.7)
        {
            var items = Invoke(evt, lists[rng.Next(lists.Count)]) is IEnumerable e ? Materialize(e) : null;
            if (items is { Count: > 0 })
            {
                value = items[rng.Next(items.Count)];
                return true;
            }
        }

        var bounds = helpers.Where(m => m.ReturnType == typeof(int) && m.Name.Contains("Max", StringComparison.Ordinal)).ToList();
        var matching = bounds.Where(m => BoundMatches(m.Name, slot.Name)).ToList();
        if (matching.Count == 0 && IsAmountLike(slot.Name) && rng.NextDouble() < 0.5) matching = bounds;
        if (matching.Count == 0)
        {
            // No bound to go by: use the name-based domain (ids, seats, turns, amounts). Not in the client profile.
            if (ClientOnly || rng.NextDouble() < 0.5) return false;
            value = GenInt(slot.Name);
            return true;
        }

        var exact = matching.Where(m => BoundCore(m.Name).Equals(slot.Name, StringComparison.OrdinalIgnoreCase)).ToList();
        var chosen = exact.Count > 0 && rng.NextDouble() < 0.8 ? exact[rng.Next(exact.Count)] : matching[rng.Next(matching.Count)];
        if (Invoke(evt, chosen) is not int max) return false;

        var r = rng.NextDouble();
        value = r switch
        {
            < 0.6 => max <= 0 ? 0 : rng.Next(0, max + 1),
            < 0.85 => Math.Max(0, max),
            < 0.95 => 0,
            _ => ClientOnly ? Math.Max(0, max) : max + 1
        };
        return true;
    }

    private static string BoundCore(string method)
    {
        var core = method.StartsWith("ValidMax", StringComparison.Ordinal) ? method[8..] : method.StartsWith("Max", StringComparison.Ordinal) ? method[3..] : method;
        return core;
    }

    private static bool BoundMatches(string method, string slot)
    {
        var core = BoundCore(method);
        if (core.Length == 0) return false;
        return core.Equals(slot, StringComparison.OrdinalIgnoreCase)
               || core.Equals(slot + "Amount", StringComparison.OrdinalIgnoreCase)
               || slot.Contains(core, StringComparison.OrdinalIgnoreCase)
               || core.Contains(slot, StringComparison.OrdinalIgnoreCase);
    }

    private static bool IsAmountLike(string name) =>
        name.Contains("Amount", StringComparison.Ordinal) || name.Contains("Forces", StringComparison.Ordinal) || name.Contains("Resources", StringComparison.Ordinal) || name == "Bid" || name.Contains("Spice", StringComparison.Ordinal);

    /// <summary>Forces to move or place: either some of the player's own battalions or reserves spread over candidate locations.</summary>
    private Dictionary<Location, Battalion> GenForces(GameEvent evt)
    {
        var result = new Dictionary<Location, Battalion>();
        var me = _player;
        if (me == null) return result;

        if (rng.NextDouble() < 0.6 && me.ForcesInLocations.Count > 0)
        {
            var sources = me.ForcesInLocations.Where(kv => !(kv.Key is Homeworld) || rng.NextDouble() < 0.2).ToList();
            if (sources.Count == 0) return result;
            var count = rng.Next(1, Math.Min(3, sources.Count) + 1);
            for (var i = 0; i < count; i++)
            {
                var (loc, bat) = sources[rng.Next(sources.Count)];
                if (result.ContainsKey(loc)) continue;
                var f = PartOf(bat.AmountOfForces);
                var s = PartOf(bat.AmountOfSpecialForces);
                if (f + s == 0 && rng.NextDouble() < 0.8) f = Math.Min(1, bat.AmountOfForces);
                result.Add(loc, new Battalion(me.Faction, f, s, loc));
            }

            return result;
        }

        var targets = Pool(evt, typeof(Location), "Location");
        if (targets == null || targets.Count == 0) return result;
        var n = rng.Next(1, Math.Min(3, targets.Count) + 1);
        for (var i = 0; i < n; i++)
        {
            if (targets[rng.Next(targets.Count)] is not Location loc || result.ContainsKey(loc)) continue;
            result.Add(loc, new Battalion(me.Faction, PartOf(Math.Min(me.ForcesInReserve, 5)), PartOf(Math.Min(me.SpecialForcesInReserve, 3)), loc));
        }

        return result;
    }

    private int PartOf(int available)
    {
        if (available <= 0) return !ClientOnly && rng.NextDouble() < 0.05 ? 1 : 0;
        var r = rng.NextDouble();
        if (r < 0.4) return available;
        if (r < 0.95 || ClientOnly) return rng.Next(0, available + 1);
        return available + 1;
    }

    /// <summary>Calls a static helper, filling its parameters from the game, the player and the event's current values.</summary>
    private object? Invoke(GameEvent evt, MethodInfo m)
    {
        var ps = m.GetParameters();
        var args = new object?[ps.Length];
        for (var i = 0; i < ps.Length; i++)
        {
            if (!TryArg(evt, ps[i], out args[i])) return null;
        }

        try
        {
            return m.Invoke(null, args);
        }
        catch
        {
            return null;
        }
    }

    private bool TryArg(GameEvent evt, ParameterInfo p, out object? value)
    {
        var t = p.ParameterType;
        value = null;
        if (t == typeof(Game))
        {
            value = _game;
            return true;
        }

        if (t == typeof(Player))
        {
            value = _player;
            return _player != null;
        }

        var name = p.Name ?? "";
        foreach (var slot in SlotsOf(evt.GetType()))
        {
            if (!t.IsAssignableFrom(slot.Type) && !(slot.Type.IsAssignableFrom(t) && !t.IsValueType)) continue;
            if (!slot.Name.Equals(name, StringComparison.OrdinalIgnoreCase) && !name.Contains(slot.Name, StringComparison.OrdinalIgnoreCase)) continue;
            try
            {
                var v = slot.Get(evt);
                if (v == null || t.IsInstanceOfType(v))
                {
                    value = v;
                    return true;
                }
            }
            catch
            {
                // Getters that parse may throw; try the defaults below.
            }
        }

        if (p.HasDefaultValue && rng.NextDouble() < 0.5)
        {
            value = p.DefaultValue;
            return true;
        }

        if (t == typeof(Faction))
        {
            value = _player?.Faction ?? Faction.None;
            return true;
        }

        if (t == typeof(bool))
        {
            value = rng.NextDouble() < 0.5;
            return true;
        }

        if (t == typeof(int))
        {
            value = GenSmallAmount();
            return true;
        }

        if (t == typeof(Territory))
        {
            value = _game.CurrentBattle?.Territory;
            return true;
        }

        foreach (var slot in SlotsOf(evt.GetType()))
        {
            if (!t.IsAssignableFrom(slot.Type)) continue;
            try
            {
                value = slot.Get(evt);
                return true;
            }
            catch
            {
                // Try the next one.
            }
        }

        if (t.IsValueType)
        {
            value = Activator.CreateInstance(t);
            return true;
        }

        return true;
    }

    /// <summary>Changes one slot of a copy to a steered value. Returns the JSON member that changed, or null if none did.</summary>
    private string? MutateSteered(GameEvent copy)
    {
        var slots = SlotsOf(copy.GetType());
        if (slots.Length == 0) return null;
        var slot = slots[rng.Next(slots.Length)];
        var props = Settable[copy.GetType()];
        var before = props.Select(p => SafeJson(p, copy)).ToArray();
        TrySetSteered(copy, slot);
        for (var i = 0; i < props.Length; i++)
            if (SafeJson(props[i], copy) != before[i])
                return props[i].Name;
        return null;
    }

    private static string? SafeJson(JsonPropertyInfo p, object o)
    {
        try
        {
            return JsonSerializer.Serialize(p.Get!(o), p.PropertyType, Options);
        }
        catch
        {
            return null;
        }
    }

    #endregion

    /// <summary>FuzzSettings.ClientOnly: only values the client UI could send.</summary>
    public bool ClientOnly { get; init; } = true;

    #region Battle actions

    /// <summary>The events played around a fight (package I of rust/docs/port-assignments.md), plus plan revision and traitor calls.</summary>
    private static readonly Type[] BattleActionTypes =
    [
        typeof(Voice), typeof(Prescience), typeof(Thought), typeof(ThoughtAnswered), typeof(Diplomacy), typeof(PoisonToothCancelled),
        typeof(PortableAntidoteUsed), typeof(RockWasMelted), typeof(AuditCancelled), typeof(Audited), typeof(FaceDanced),
        typeof(FaceDancerReplaced), typeof(FaceDancerRevealed), typeof(ResidualPlayed), typeof(Planetology), typeof(JuicePlayed),
        typeof(KarmaEcazDial), typeof(KarmaPrescience), typeof(TraitorDiscarded), typeof(BattleRevision), typeof(TreacheryCalled),
        typeof(HMSAdvantageChosen), typeof(SwitchedSkilledLeader), typeof(BattleClaimed)
    ];

    /// <summary>Battle actions whose execution branches the bots and plain steering rarely reach.</summary>
    private static readonly Type[] RareBattleActionTypes = [typeof(KarmaEcazDial), typeof(JuicePlayed), typeof(ResidualPlayed), typeof(AuditCancelled)];

    public static bool BattleInProgress(Game g) => g.CurrentMainPhase == MainPhase.Battle && g.CurrentBattle != null;

    /// <summary>
    /// A battle action for a player in the current battle (or the auditee). Prefers an action that is applicable now
    /// and rarely executed in this phase; sometimes picks one of <see cref="RareBattleActionTypes"/> regardless, which is
    /// rejected unless it runs without validation. Fields come from the steered helpers, then from the battle itself.
    /// </summary>
    public GameEvent? BattleAction(Game g, List<Type>[] applicable, out int actor)
    {
        _game = g;
        actor = -1;
        var battle = g.CurrentBattle!;
        var candidates = new List<(int Actor, Type Type)>();
        for (var i = 0; i < applicable.Length && i < g.Players.Count; i++)
            foreach (var t in applicable[i])
                if (Array.IndexOf(BattleActionTypes, t) >= 0)
                    candidates.Add((i, t));

        (int Actor, Type Type) pick;
        if (candidates.Count > 0 && rng.NextDouble() < 0.9)
        {
            var weights = candidates.Select(c => 1.0 / Math.Pow(1 + _accepted.GetValueOrDefault((c.Type, g.CurrentPhase)), 1.5)).ToArray();
            pick = candidates[WeightedIndex(weights)];
        }
        else
        {
            if (rng.NextDouble() > 0.15) return null;
            var involved = Enumerable.Range(0, g.Players.Count).Where(i => battle.IsAggressorOrDefender(g.Players[i]) || g.Players[i] == g.Auditee).ToList();
            if (involved.Count == 0) return null;
            pick = (involved[rng.Next(involved.Count)], RareBattleActionTypes[rng.Next(RareBattleActionTypes.Length)]);
        }

        actor = pick.Actor;
        _player = g.Players[actor];
        var evt = BuildSteered(pick.Type, true);
        FillFromBattle(evt, battle);
        return evt;
    }

    /// <summary>
    /// From version 188, a plan withdrawn with BattleRevision after the opponent saw parts of it through prescience must
    /// keep those parts (Game.PrescienceSeenPlans). When such a player may submit Battle again, resubmit the seen plan
    /// with the seen element changed to another valid choice; the engine has to reject it. Returns null when there is
    /// no such situation.
    /// </summary>
    public GameEvent? ChangedSeenPlan(Game g, List<Type>[] applicable, out int actor)
    {
        _game = g;
        actor = -1;
        if (g.Version < 188 || g.PrescienceSeenPlans.Count == 0 || g.CurrentBattle == null) return null;
        var entries = g.PrescienceSeenPlans.OrderBy(e => (int)e.Key).ToList();
        var (aspect, seen) = entries[rng.Next(entries.Count)];
        var p = g.Players.FirstOrDefault(x => x.Faction == seen.Initiator);
        if (p == null) return null;
        actor = g.Players.IndexOf(p);
        if (actor >= applicable.Length || !applicable[actor].Contains(typeof(Battle))) return null;
        _player = p;

        if (Utilities.Deserialize<GameEvent>(Utilities.Serialize<GameEvent>(seen)) is not Battle plan) return null;
        plan.Initialize(g, p.Faction);
        var territory = g.CurrentBattle.Territory;
        try
        {
            switch (aspect)
            {
                case PrescienceAspect.Leader:
                    var heroes = Battle.ValidBattleHeroes(g, p).Where(h => h != seen.Hero).ToList();
                    if (heroes.Count == 0) return null;
                    plan.Hero = heroes[rng.Next(heroes.Count)];
                    break;
                case PrescienceAspect.Weapon:
                    var weapons = Battle.ValidWeapons(g, p, plan.Defense, plan.Hero, territory, true).Where(c => c != seen.Weapon).ToList();
                    if (weapons.Count == 0) return null;
                    plan.Weapon = weapons[rng.Next(weapons.Count)];
                    break;
                case PrescienceAspect.Defense:
                    var defenses = Battle.ValidDefenses(g, p, plan.Weapon, territory, true).Where(c => c != seen.Defense).ToList();
                    if (defenses.Count == 0) return null;
                    plan.Defense = defenses[rng.Next(defenses.Count)];
                    break;
                case PrescienceAspect.Dial:
                    var max = Battle.MaxForces(g, p, false);
                    var other = Enumerable.Range(0, Math.Max(0, max) + 1).Where(f => f != plan.Forces).ToList();
                    if (other.Count == 0) return null;
                    plan.Forces = other[rng.Next(other.Count)];
                    break;
                default:
                    return null;
            }
        }
        catch
        {
            return null;
        }

        return plan;
    }

    /// <summary>Sets some members from the battle: plan cards, the opponent's leader, the battle territory, Aggressor juice, a paid audit.</summary>
    private void FillFromBattle(GameEvent evt, BattleInitiated battle)
    {
        switch (evt)
        {
            case JuicePlayed j when rng.NextDouble() < 0.7 && (!ClientOnly || (_player != null && JuicePlayed.ValidTypes(_game, _player).Contains(JuiceType.Aggressor))):
                j.Type = JuiceType.Aggressor;
                break;
            case AuditCancelled a when rng.NextDouble() < 0.7:
                a.Cancelled = true;
                break;
        }

        // Values taken from the battle rather than from a helper: wide profile only.
        if (ClientOnly || rng.NextDouble() >= 0.3) return;

        var me = _player;
        var myPlan = battle.PlanOf(me);
        var theirPlan = battle.PlanOfOpponent(me);
        var opponent = me == null ? null : battle.OpponentOf(me);
        var cards = new List<object?> { myPlan?.Weapon, myPlan?.Defense, theirPlan?.Weapon, theirPlan?.Defense }.Where(c => c != null).ToList();
        var heroes = new List<object?> { theirPlan?.Hero, myPlan?.Hero }.Where(h => h != null).Concat(opponent?.Leaders.Cast<object?>() ?? []).ToList();
        var territory = battle.Territory;
        var locations = territory?.Locations.Cast<object?>().ToList() ?? [];

        foreach (var slot in SlotsOf(evt.GetType()))
        {
            if (rng.NextDouble() < 0.5) continue;
            List<object?>? pool = null;
            if (slot.Type == typeof(TreacheryCard)) pool = cards;
            else if (typeof(IHero).IsAssignableFrom(slot.Type) || slot.Type == typeof(IHero)) pool = heroes.Where(slot.Type.IsInstanceOfType).ToList();
            else if (slot.Type == typeof(Territory) && territory != null) pool = [territory];
            else if (typeof(Location).IsAssignableFrom(slot.Type)) pool = locations.Where(slot.Type.IsInstanceOfType).ToList();
            if (pool is not { Count: > 0 }) continue;
            try
            {
                slot.Set(evt, pool[rng.Next(pool.Count)]);
            }
            catch
            {
                // A setter that rejects the value: keep the steered one.
            }
        }
    }

    #endregion
}
