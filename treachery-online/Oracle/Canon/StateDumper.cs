using System.Collections;
using System.Collections.Concurrent;
using System.Globalization;
using Expression = System.Linq.Expressions.Expression;
using System.Security.Cryptography;

namespace Treachery.Oracle.Canon;

/// <summary>
/// Canonical, reflection-driven dump of a <see cref="Game"/> object graph.
/// The format is specified in rust/docs/trace-format.md ("Canonical state dump"). Any change here must be
/// reflected there and bump <see cref="FormatVersion"/>.
/// </summary>
public sealed class StateDumper
{
    public const int FormatVersion = 1;

    private const int MaxDepth = 200;

    /// <summary>Types (by name) that were dumped as "$unsupported". Should stay empty; reported at end of a run.</summary>
    public static readonly ConcurrentDictionary<string, int> UnsupportedTypes = new();

    private readonly JsonOut _o;
    private Game _root = null!;
    private readonly Dictionary<Player, int> _playerIndex = new(ReferenceEqualityComparer.Instance);
    private readonly HashSet<object> _onStack = new(ReferenceEqualityComparer.Instance);
    private object? _gameRandom;
    private int _depth;

    public StateDumper(JsonOut output)
    {
        _o = output;
    }

    /// <summary>Appends the canonical dump of <paramref name="game"/> to the output.</summary>
    public void DumpGame(Game game)
    {
        _root = game;
        _playerIndex.Clear();
        for (var i = 0; i < game.Players.Count; i++) _playerIndex.TryAdd(game.Players[i], i);
        _gameRandom = GameRandomAccessor(game);
        _onStack.Clear();
        _depth = 0;
        ExpandObject(game);
    }

    /// <summary>Appends the canonical rendering of a value that is not part of a game (for example a validation Message).</summary>
    public void DumpDetached(object? value, Game? game)
    {
        _root = game!;
        _playerIndex.Clear();
        if (game != null)
            for (var i = 0; i < game.Players.Count; i++) _playerIndex.TryAdd(game.Players[i], i);
        _gameRandom = game != null ? GameRandomAccessor(game) : null;
        _onStack.Clear();
        _depth = 0;
        Value(value);
    }

    public static string Sha256Hex(ReadOnlySpan<byte> bytes)
    {
        Span<byte> hash = stackalloc byte[32];
        SHA256.HashData(bytes, hash);
        return Convert.ToHexStringLower(hash);
    }

    #region Value dispatch

    private void Value(object? v)
    {
        if (v == null)
        {
            _o.Null();
            return;
        }

        var shape = Shape.Of(v.GetType());
        switch (shape.Kind)
        {
            case Kind.String: _o.WriteString((string)v); return;
            case Kind.Bool: _o.Bool((bool)v); return;
            case Kind.SignedInt: _o.Int(Convert.ToInt64(v, CultureInfo.InvariantCulture)); return;
            case Kind.UnsignedInt: _o.UInt(Convert.ToUInt64(v, CultureInfo.InvariantCulture)); return;
            case Kind.Single: _o.Single((float)v); return;
            case Kind.Double: _o.Double((double)v); return;
            case Kind.Decimal: _o.WriteString(((decimal)v).ToString(CultureInfo.InvariantCulture)); return;
            case Kind.Char: _o.WriteString(((char)v).ToString()); return;
            case Kind.Enum: _o.WriteString(EnumText(v, shape)); return;
            case Kind.DateTimeOffset: _o.WriteString(((DateTimeOffset)v).ToString("o", CultureInfo.InvariantCulture)); return;
            case Kind.DateTime: _o.WriteString(((DateTime)v).ToString("o", CultureInfo.InvariantCulture)); return;
            case Kind.TimeSpan: _o.Int(((TimeSpan)v).Ticks); return;
            case Kind.ClrType:
                _o.ObjStart();
                _o.Key("$clrType");
                _o.WriteString(Shape.TypeName((Type)v));
                _o.ObjEnd();
                return;
            case Kind.Exception:
                var ex = (Exception)v;
                _o.ObjStart();
                _o.Key("$type"); _o.WriteString("$Exception");
                _o.Comma(); _o.Key("message"); _o.WriteString(ex.Message);
                _o.Comma(); _o.Key("type"); _o.WriteString(ex.GetType().FullName);
                _o.ObjEnd();
                return;
            case Kind.Game:
                Ref(ReferenceEquals(v, _root) ? "Game" : "OtherGame");
                return;
            case Kind.Map:
                Ref(_root != null && ReferenceEquals(v, _root.Map) ? "Map" : "OtherMap");
                return;
            case Kind.Player:
                if (_playerIndex.TryGetValue((Player)v, out var index))
                {
                    _o.ObjStart();
                    _o.Key("$ref"); _o.WriteString("Player");
                    _o.Comma(); _o.Key("index"); _o.Int(index);
                    _o.ObjEnd();
                }
                else
                {
                    ExpandObject(v);
                }
                return;
            case Kind.Leader:
                EntityRef("Leader", ((Leader)v).Id, ReferenceEquals(LeaderManager.LeaderLookup.Find(((Leader)v).Id), v));
                return;
            case Kind.Messiah:
                EntityRef("Messiah", ((Messiah)v).Id, ReferenceEquals(LeaderManager.Messiah, v));
                return;
            case Kind.TreacheryCard:
                EntityRef("TreacheryCard", ((TreacheryCard)v).Id, ReferenceEquals(TreacheryCardManager.Get(((TreacheryCard)v).Id), v));
                return;
            case Kind.Location:
            {
                var l = (Location)v;
                EntityRef("Location", l.Id, _root != null && IsOwnLocation(l));
                return;
            }
            case Kind.Territory:
            {
                var t = (Territory)v;
                EntityRef("Territory", t.Id, _root != null && IsOwnTerritory(t));
                return;
            }
            case Kind.LoggedRandom:
                if (ReferenceEquals(v, _gameRandom))
                {
                    Ref("GameRandom");
                }
                else
                {
                    _o.ObjStart();
                    _o.Key("$type"); _o.WriteString("LoggedRandom");
                    _o.Comma(); _o.Key("count"); _o.Int(((LoggedRandom)v).GetCount());
                    _o.ObjEnd();
                }
                return;
            case Kind.Dictionary:
                WithStack(v, () =>
                {
                    _o.ArrStart();
                    var first = true;
                    var e = ((IDictionary)v).GetEnumerator();
                    while (e.MoveNext())
                    {
                        if (!first) _o.Comma();
                        first = false;
                        _o.ArrStart();
                        Value(e.Key);
                        _o.Comma();
                        Value(e.Value);
                        _o.ArrEnd();
                    }
                    _o.ArrEnd();
                });
                return;
            case Kind.Sequence:
                WithStack(v, () => Sequence((IEnumerable)v));
                return;
            case Kind.Unsupported:
                UnsupportedTypes.AddOrUpdate(shape.Name, 1, (_, c) => c + 1);
                _o.ObjStart();
                _o.Key("$unsupported"); _o.WriteString(shape.Name);
                _o.ObjEnd();
                return;
            case Kind.Object:
                ExpandObject(v);
                return;
            default:
                throw new InvalidOperationException("Unhandled kind " + shape.Kind);
        }
    }

    private void Sequence(IEnumerable items)
    {
        _o.ArrStart();
        var first = true;
        try
        {
            foreach (var item in items)
            {
                if (!first) _o.Comma();
                first = false;
                Value(item);
            }
        }
        catch (Exception e) when (e is not InvalidOperationException { Source: "Treachery.Oracle" })
        {
            // A lazy query stored in a message can throw when enumerated. Record where it stopped.
            if (!first) _o.Comma();
            _o.ObjStart();
            _o.Key("$enumerationError");
            _o.ObjStart();
            _o.Key("message"); _o.WriteString(e.Message);
            _o.Comma(); _o.Key("type"); _o.WriteString(e.GetType().FullName);
            _o.ObjEnd();
            _o.ObjEnd();
        }
        _o.ArrEnd();
    }

    private void Ref(string what)
    {
        _o.ObjStart();
        _o.Key("$ref");
        _o.WriteString(what);
        _o.ObjEnd();
    }

    private void EntityRef(string type, int id, bool canonicalInstance)
    {
        _o.ObjStart();
        _o.Key("$ref"); _o.WriteString(type);
        _o.Comma(); _o.Key("id"); _o.Int(id);
        if (!canonicalInstance)
        {
            _o.Comma(); _o.Key("foreign"); _o.Bool(true);
        }
        _o.ObjEnd();
    }

    private bool IsOwnLocation(Location l)
    {
        var all = MapLocations(_root.Map);
        return l.Id >= 0 && l.Id < all.Count && ReferenceEquals(all[l.Id], l);
    }

    private bool IsOwnTerritory(Territory t)
    {
        foreach (var l in MapLocations(_root.Map))
            if (ReferenceEquals(l.Territory, t)) return true;
        return false;
    }

    private void WithStack(object v, Action body)
    {
        if (!v.GetType().IsValueType)
        {
            if (!_onStack.Add(v))
            {
                _o.ObjStart();
                _o.Key("$cycle");
                _o.WriteString(Shape.Of(v.GetType()).Name);
                _o.ObjEnd();
                return;
            }
        }

        if (++_depth > MaxDepth) throw new InvalidOperationException("State dump exceeded maximum depth at " + v.GetType());
        try
        {
            body();
        }
        finally
        {
            _depth--;
            if (!v.GetType().IsValueType) _onStack.Remove(v);
        }
    }

    #endregion

    #region Objects

    private void ExpandObject(object v)
    {
        WithStack(v, () =>
        {
            var shape = Shape.Of(v.GetType());
            _o.ObjStart();
            _o.Key("$type");
            _o.WriteString(shape.Name);
            foreach (var f in shape.Fields)
            {
                if (f.Special == Special.Omit) continue;
                _o.Comma();
                _o.Key(f.Name);
                switch (f.Special)
                {
                    case Special.None:
                        Value(f.Get(v));
                        break;
                    case Special.HistoryCount:
                        _o.ObjStart();
                        _o.Key("$count");
                        _o.Int(((ICollection)f.Get(v)!).Count);
                        _o.ObjEnd();
                        break;
                    case Special.Presence:
                        _o.ObjStart();
                        _o.Key("$present");
                        _o.Bool(f.Get(v) != null);
                        _o.ObjEnd();
                        break;
                    case Special.ExpandPlayers:
                        WithStack(f.Get(v)!, () =>
                        {
                            _o.ArrStart();
                            var first = true;
                            foreach (var p in (IEnumerable<Player>)f.Get(v)!)
                            {
                                if (!first) _o.Comma();
                                first = false;
                                ExpandObject(p);
                            }
                            _o.ArrEnd();
                        });
                        break;
                    case Special.ExpandMap:
                        var map = (Map)f.Get(v)!;
                        if (ReferenceEquals(v, _root)) ExpandObject(map);
                        else Value(map);
                        break;
                    case Special.ExpandRandom:
                        if (f.Get(v) is LoggedRandom random && ReferenceEquals(v, _root))
                        {
                            _o.ObjStart();
                            _o.Key("$type"); _o.WriteString("LoggedRandom");
                            _o.Comma(); _o.Key("count"); _o.Int(random.GetCount());
                            _o.ObjEnd();
                        }
                        else
                        {
                            Value(f.Get(v));
                        }
                        break;
                    case Special.ExpandLocations:
                        WithStack(f.Get(v)!, () =>
                        {
                            _o.ArrStart();
                            var first = true;
                            foreach (var l in (IEnumerable<Location>)f.Get(v)!)
                            {
                                if (!first) _o.Comma();
                                first = false;
                                ExpandObject(l);
                            }
                            _o.ArrEnd();
                        });
                        break;
                    case Special.Territories:
                        _o.ArrStart();
                        var firstT = true;
                        var seen = new HashSet<Territory>(ReferenceEqualityComparer.Instance);
                        foreach (var l in MapLocations((Map)v))
                        {
                            if (!seen.Add(l.Territory)) continue;
                            if (!firstT) _o.Comma();
                            firstT = false;
                            ExpandObject(l.Territory);
                        }
                        _o.ArrEnd();
                        break;
                }
            }
            _o.ObjEnd();
        });
    }

    #endregion

    #region Enums

    private static readonly ConcurrentDictionary<(Type, ulong), string> EnumTexts = new();

    private static string EnumText(object v, Shape shape)
    {
        var raw = shape.EnumUnsigned
            ? Convert.ToUInt64(v, CultureInfo.InvariantCulture)
            : unchecked((ulong)Convert.ToInt64(v, CultureInfo.InvariantCulture));
        return EnumTexts.GetOrAdd((shape.Type, raw), _ =>
        {
            var number = shape.EnumUnsigned
                ? raw.ToString(CultureInfo.InvariantCulture)
                : unchecked((long)raw).ToString(CultureInfo.InvariantCulture);
            return v + "(" + number + ")";
        });
    }

    #endregion

    #region Accessors for internals

    private static readonly Func<Game, object?> GameRandomAccessor = BuildPropertyGetter<Game>("Random");

    private static readonly FieldInfo MapLocationsField = typeof(Map).GetField("_locations", BindingFlags.Instance | BindingFlags.NonPublic)!;

    public static List<Location> MapLocations(Map m) => (List<Location>)MapLocationsField.GetValue(m)!;

    public static LoggedRandom? GameRandom(Game g) => (LoggedRandom?)GameRandomAccessor(g);

    private static Func<T, object?> BuildPropertyGetter<T>(string name)
    {
        var prop = typeof(T).GetProperty(name, BindingFlags.Instance | BindingFlags.Public | BindingFlags.NonPublic)
                   ?? throw new MissingMemberException(typeof(T).Name, name);
        var p = Expression.Parameter(typeof(T));
        return Expression.Lambda<Func<T, object?>>(Expression.Convert(Expression.Property(p, prop), typeof(object)), p).Compile();
    }

    #endregion

    #region Shapes

    private enum Kind
    {
        String, Bool, SignedInt, UnsignedInt, Single, Double, Decimal, Char, Enum, DateTimeOffset, DateTime, TimeSpan,
        ClrType, Exception, Game, Map, Player, Leader, Messiah, TreacheryCard, Location, Territory, LoggedRandom,
        Dictionary, Sequence, Object, Unsupported
    }

    private enum Special
    {
        None,
        Omit,
        HistoryCount,
        Presence,
        ExpandPlayers,
        ExpandMap,
        ExpandLocations,
        Territories,
        ExpandRandom
    }

    private sealed record FieldShape(string Name, Func<object, object?> Get, Special Special);

    private sealed class Shape
    {
        private static readonly ConcurrentDictionary<Type, Shape> Cache = new();

        public Type Type { get; private init; } = null!;
        public Kind Kind { get; private init; }
        public string Name { get; private init; } = "";
        public bool EnumUnsigned { get; private init; }
        public FieldShape[] Fields { get; private set; } = [];

        public static Shape Of(Type t) => Cache.GetOrAdd(t, Create);

        private static Shape Create(Type t)
        {
            var kind = Classify(t);
            var shape = new Shape
            {
                Type = t,
                Kind = kind,
                Name = TypeName(t),
                EnumUnsigned = t.IsEnum && Type.GetTypeCode(Enum.GetUnderlyingType(t)) is TypeCode.Byte or TypeCode.UInt16 or TypeCode.UInt32 or TypeCode.UInt64
            };

            if (kind is Kind.Object or Kind.Player or Kind.Location or Kind.Territory or Kind.Map or Kind.Game) shape.Fields = BuildFields(t);
            return shape;
        }

        private static Kind Classify(Type t)
        {
            if (t == typeof(string)) return Kind.String;
            if (t == typeof(bool)) return Kind.Bool;
            if (t.IsEnum) return Kind.Enum;
            if (t == typeof(int) || t == typeof(long) || t == typeof(short) || t == typeof(sbyte)) return Kind.SignedInt;
            if (t == typeof(uint) || t == typeof(ulong) || t == typeof(ushort) || t == typeof(byte)) return Kind.UnsignedInt;
            if (t == typeof(float)) return Kind.Single;
            if (t == typeof(double)) return Kind.Double;
            if (t == typeof(decimal)) return Kind.Decimal;
            if (t == typeof(char)) return Kind.Char;
            if (t == typeof(DateTimeOffset)) return Kind.DateTimeOffset;
            if (t == typeof(DateTime)) return Kind.DateTime;
            if (t == typeof(TimeSpan)) return Kind.TimeSpan;
            if (typeof(Type).IsAssignableFrom(t)) return Kind.ClrType;
            if (typeof(Exception).IsAssignableFrom(t)) return Kind.Exception;
            if (typeof(Game).IsAssignableFrom(t)) return Kind.Game;
            if (typeof(Map).IsAssignableFrom(t)) return Kind.Map;
            if (typeof(Player).IsAssignableFrom(t)) return Kind.Player;
            if (typeof(Leader).IsAssignableFrom(t)) return Kind.Leader;
            if (typeof(Messiah).IsAssignableFrom(t)) return Kind.Messiah;
            if (typeof(TreacheryCard).IsAssignableFrom(t)) return Kind.TreacheryCard;
            if (typeof(Location).IsAssignableFrom(t)) return Kind.Location;
            if (typeof(Territory).IsAssignableFrom(t)) return Kind.Territory;
            if (typeof(LoggedRandom).IsAssignableFrom(t)) return Kind.LoggedRandom;
            if (typeof(Delegate).IsAssignableFrom(t) || t.IsPointer || t == typeof(IntPtr) || t == typeof(UIntPtr)) return Kind.Unsupported;
            if (t.IsArray) return Kind.Sequence;
            if (IsBclCollection(t))
            {
                if (typeof(IDictionary).IsAssignableFrom(t)) return Kind.Dictionary;
                if (typeof(IEnumerable).IsAssignableFrom(t)) return Kind.Sequence;
            }

            if (t.Namespace != null && t.Namespace.StartsWith("System", StringComparison.Ordinal) && !IsValueTupleOrKvp(t))
            {
                // Lazy LINQ queries kept inside log messages: dumped as the sequence they produce now.
                if (typeof(IDictionary).IsAssignableFrom(t)) return Kind.Dictionary;
                if (typeof(IEnumerable).IsAssignableFrom(t)) return Kind.Sequence;
                return Kind.Unsupported;
            }

            return Kind.Object;
        }

        private static bool IsBclCollection(Type t) =>
            t.Namespace != null && t.Namespace.StartsWith("System.Collections", StringComparison.Ordinal);

        private static bool IsValueTupleOrKvp(Type t) =>
            t.IsGenericType && (t.GetGenericTypeDefinition() == typeof(KeyValuePair<,>) || t.FullName!.StartsWith("System.ValueTuple`", StringComparison.Ordinal) || t.FullName!.StartsWith("System.Tuple`", StringComparison.Ordinal));

        public static string TypeName(Type t)
        {
            if (t.IsArray) return TypeName(t.GetElementType()!) + "[]";
            if (!t.IsGenericType) return t.Name;
            var name = t.Name;
            var tick = name.IndexOf('`');
            if (tick >= 0) name = name[..tick];
            return name + "<" + string.Join(",", t.GetGenericArguments().Select(TypeName)) + ">";
        }

        public static string NormalizeFieldName(string raw)
        {
            if (raw.StartsWith('<'))
            {
                var close = raw.IndexOf('>');
                var suffix = raw[(close + 1)..];
                if (suffix is "k__BackingField" or "P") return raw[1..close];
            }

            return raw;
        }

        private static FieldShape[] BuildFields(Type t)
        {
            var result = new List<FieldShape>();
            var names = new HashSet<string>();
            for (var cur = t; cur != null && cur != typeof(object) && cur != typeof(ValueType); cur = cur.BaseType)
            {
                foreach (var fi in cur.GetFields(BindingFlags.Instance | BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.DeclaredOnly))
                {
                    var name = NormalizeFieldName(fi.Name);
                    if (!names.Add(name))
                        throw new InvalidOperationException($"Field name collision after normalization: {t}.{name}");
                    result.Add(new FieldShape(name, BuildGetter(fi), SpecialFor(cur, fi.Name)));
                }
            }

            if (t == typeof(Map)) result.Add(new FieldShape("$territories", _ => null, Special.Territories));

            return result.OrderBy(f => f.Name, StringComparer.Ordinal).ToArray();
        }

        private static Special SpecialFor(Type declaring, string rawName)
        {
            if (declaring == typeof(Game))
            {
                return rawName switch
                {
                    "<History>k__BackingField" => Special.HistoryCount,
                    "<PreviousGameState>k__BackingField" => Special.Presence,
                    "<Players>k__BackingField" => Special.ExpandPlayers,
                    "<Map>k__BackingField" => Special.ExpandMap,
                    "<Random>k__BackingField" => Special.ExpandRandom,
                    _ => Special.None
                };
            }

            if (declaring == typeof(Message) && rawName == "<Nr>k__BackingField") return Special.Omit;
            if (declaring == typeof(LeaderState) && rawName == "<TimeOfDeath>k__BackingField") return Special.Omit;
            if (declaring == typeof(Map) && rawName is "_neighbourCache" or "_neighbourCacheTimestamp") return Special.Omit;
            if (declaring == typeof(Map) && rawName == "_locations") return Special.ExpandLocations;
            if (declaring == typeof(LoggedRandom) && rawName == "_supply") return Special.Omit;
            return Special.None;
        }

        private static Func<object, object?> BuildGetter(FieldInfo fi)
        {
            var p = Expression.Parameter(typeof(object));
            var owner = fi.DeclaringType!;
            Expression instance = owner.IsValueType ? Expression.Unbox(p, owner) : Expression.Convert(p, owner);
            Expression body = Expression.Field(instance, fi);
            if (fi.FieldType.IsPointer) return _ => null;
            body = Expression.Convert(body, typeof(object));
            return Expression.Lambda<Func<object, object?>>(body, p).Compile();
        }
    }

    #endregion
}
