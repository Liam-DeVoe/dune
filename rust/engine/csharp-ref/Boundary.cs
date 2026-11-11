// boundary.json: every public member of every ported C# type, called with
// boundary inputs. The list of ported types is tests/coverage/ported_types.txt;
// add a type there when its Rust port lands, regenerate, and
// tests/boundary.rs fails until every member has a Rust entry or a
// "not ported" note in tests/coverage/not_ported.txt.
//
// Inputs per parameter type:
//   int      IntSamples (-1, 0, 13, int.MinValue, int.MaxValue and ids around the table ends)
//   enum     every defined value, plus -1, 0, 13, int.MinValue, int.MaxValue
//   bool     false, true
//   float    0, -0, 1.5, -1, NaN, infinity (double likewise)
//   game objects  a sample of instances (Golden.ArgSamples)
//   plus null wherever the parameter is nullable
// Receivers of instance members are all instances (Golden.Receivers).
// Members with a parameter of another type (Game, IEnumerable<T>, Func, ...)
// are listed with "unsupported" and no cases; the Rust table must point to
// the test that covers them instead.
//
// The "lazy" section is separate: every lazy IEnumerable producer that takes a
// Game or Player, called on game states that hold a null or throwing element
// after a matching one, with the items yielded up to the exception
// (LazyBoundary.cs).

using System.Reflection;
using System.Runtime.CompilerServices;
using System.Text.Json.Nodes;
using Treachery.Shared;

public static class Boundary
{
    private const int MaxCasesPerMember = 6000;
    private static readonly NullabilityInfoContext Nullability = new();

    public static readonly int[] IntSamples =
        [int.MinValue, -2, -1, 0, 1, 2, 13, 42, 105, 106, 107, 999, 1000, 1001, 1069, 1070, int.MaxValue];

    // int parameters that bound a loop (a search depth): int.MaxValue would
    // run for hours in both languages (the searches are exponential), so these get small values instead.
    private static readonly HashSet<string> BoundedIntParameters = ["distance", "maxDistance"];
    private static readonly int[] BoundedIntSamples = [int.MinValue, -1, 0, 1, 2, 3];

    public static string TypeName(Type t)
    {
        if (Nullable.GetUnderlyingType(t) is { } u) return TypeName(u) + "?";
        if (t.IsArray) return TypeName(t.GetElementType()!) + "[]";
        if (t.IsByRef) return TypeName(t.GetElementType()!) + "&";
        if (t.IsGenericType)
        {
            var name = t.Name[..t.Name.IndexOf('`')];
            return name + "<" + string.Join(",", t.GetGenericArguments().Select(TypeName)) + ">";
        }
        if (t.IsNested && !t.IsGenericParameter) return TypeName(t.DeclaringType!) + "." + t.Name;
        return t.Name;
    }

    public static string Signature(Type declaring, MemberInfo m) => m switch
    {
        ConstructorInfo c => TypeName(declaring) + "..ctor(" + string.Join(",", c.GetParameters().Select(p => TypeName(p.ParameterType))) + ")",
        MethodInfo mi => TypeName(declaring) + "." + mi.Name + (mi.IsGenericMethodDefinition ? "<" + string.Join(",", mi.GetGenericArguments().Select(a => a.Name)) + ">" : "") +
                         "(" + string.Join(",", mi.GetParameters().Select(p => TypeName(p.ParameterType))) + ")",
        _ => TypeName(declaring) + "." + m.Name
    };

    // Parameters declared non-nullable that callers do pass null to
    // ("Type.Member.parameter"). They get a null input like nullable ones.
    private static readonly HashSet<string> NullInPractice =
    [
        "TreacheryCard.CounteredBy.defense",            // Bot_Other.cs:1053 passes a plan's Defense
        "TreacheryCard.CounteredBy.combinedWithWeapon"  // Battle code passes null
    ];

    // Types whose Rust port is a handle into a static table: their
    // constructors run only while building that table.
    private static readonly HashSet<string> HandleTypes =
        ["Location", "Homeworld", "AttachedLocation", "DiscoveredLocation", "HiddenMobileStronghold", "Territory", "Leader", "Messiah", "TreacheryCard", "ResourceCard"];

    // Every parameter of a game-object type gets null too: C# collections of game
    // objects can hold null (a hand, a deck, a list of territories), and code passes
    // their elements on, whatever the parameter's annotation says. The Rust handles
    // have a null value for this (engine-conventions.md, "Null handles").
    private static bool IsNullable(ParameterInfo p) =>
        HandleTypes.Contains((Nullable.GetUnderlyingType(p.ParameterType) ?? p.ParameterType).Name) ||
        p.ParameterType == typeof(IHero) ||
        Nullable.GetUnderlyingType(p.ParameterType) != null ||
        (!p.ParameterType.IsValueType && Nullability.Create(p).WriteState == NullabilityState.Nullable) ||
        NullInPractice.Contains(p.Member.DeclaringType!.Name + "." + p.Member.Name + "." + p.Name);

    // Sample inputs for one parameter, or null if the type is unsupported.
    private static List<object?>? Inputs(ParameterInfo p)
    {
        var t = Nullable.GetUnderlyingType(p.ParameterType) ?? p.ParameterType;
        List<object?>? values = null;
        if (t == typeof(int)) values = (BoundedIntParameters.Contains(p.Name!) ? BoundedIntSamples : IntSamples).Cast<object?>().ToList();
        else if (t == typeof(bool)) values = [false, true];
        else if (t == typeof(float)) values = [0f, -0f, 1.5f, -1f, float.NaN, float.PositiveInfinity];
        else if (t == typeof(double)) values = [0d, -0d, 1.5d, -1d, double.NaN, double.PositiveInfinity];
        else if (t == typeof(string)) values = ["", "a"];
        else if (t.IsEnum) values = EnumSamples(t).ToList();
        else if (Golden.ArgSamples(t) is { } samples) values = samples.Cast<object?>().ToList();
        if (values == null) return null;
        if (IsNullable(p)) values.Add(null);
        return values;
    }

    public static IEnumerable<object?> EnumSamples(Type t)
    {
        var defined = Enum.GetValues(t).Cast<object>().Select(Convert.ToInt64).ToList();
        foreach (var v in defined.Concat(new long[] { -1, 0, 13, int.MinValue, int.MaxValue }).Distinct())
            yield return Enum.ToObject(t, v);
    }

    // The cartesian product of the inputs, thinned to at most max entries by
    // taking every k-th combination (decoded from its index, so a huge
    // product is never built).
    private static List<object?[]> Product(List<List<object?>> inputs, int max)
    {
        var total = inputs.Aggregate(1L, (acc, o) => acc * o.Count);
        var stride = Math.Max(1L, (total + max - 1) / max);
        var result = new List<object?[]>();
        for (var index = 0L; index < total; index += stride)
        {
            var args = new object?[inputs.Count];
            var rest = index;
            for (var i = inputs.Count - 1; i >= 0; i--)
            {
                args[i] = inputs[i][(int)(rest % inputs[i].Count)];
                rest /= inputs[i].Count;
            }
            result.Add(args);
        }
        return result;
    }

    // Keeps every k-th item so at most max remain (deterministic).
    private static List<T> Thin<T>(List<T> items, int max)
    {
        if (items.Count <= max) return items;
        var stride = (items.Count + max - 1) / max;
        return items.Where((_, i) => i % stride == 0).ToList();
    }

    private static IEnumerable<MemberInfo> PublicMembers(Type t)
    {
        const BindingFlags flags = BindingFlags.Public | BindingFlags.Instance | BindingFlags.Static | BindingFlags.DeclaredOnly;
        foreach (var m in t.GetMembers(flags).OrderBy(m => m.MemberType).ThenBy(m => m.Name, StringComparer.Ordinal).ThenBy(m => m.ToString(), StringComparer.Ordinal))
        {
            if (m.Name.StartsWith('<')) continue;
            switch (m)
            {
                case MethodInfo mi when mi.IsSpecialName && !mi.Name.StartsWith("op_"):
                    continue; // property accessors (the property itself is listed)
                case MethodInfo or PropertyInfo or FieldInfo or ConstructorInfo:
                    yield return m;
                    break;
            }
        }
    }

    public static JsonObject Dump(IReadOnlyList<string> portedTypes)
    {
        var members = new JsonArray();
        foreach (var name in portedTypes)
        {
            var t = Golden.FindType(name);
            if (t.IsEnum) continue; // enums are covered by static_data.json and messages.json
            foreach (var m in PublicMembers(t))
                members.Add(DumpMember(t, m));
        }
        return new JsonObject
        {
            ["ported_types"] = new JsonArray(portedTypes.Select(n => (JsonNode?)n).ToArray()),
            ["members"] = members,
            // Lazy producers on states with a null or throwing element after a match (LazyBoundary.cs).
            ["lazy"] = LazyBoundary.Dump()
        };
    }

    private static JsonObject DumpMember(Type t, MemberInfo m)
    {
        var sig = Signature(t, m);
        if (Environment.GetEnvironmentVariable("BOUNDARY_VERBOSE") != null) Console.Error.WriteLine(sig);
        var o = new JsonObject { ["sig"] = sig, ["kind"] = m.MemberType.ToString() };

        var isStatic = m switch
        {
            MethodInfo mi => mi.IsStatic,
            FieldInfo fi => fi.IsStatic,
            PropertyInfo pi => (pi.GetMethod ?? pi.SetMethod)!.IsStatic,
            _ => true // constructors need no receiver
        };
        o["static"] = isStatic;

        string? unsupported = null;
        var parameters = m switch
        {
            MethodBase mb => mb.GetParameters(),
            PropertyInfo pi => pi.GetIndexParameters(),
            _ => []
        };
        if (m is MethodInfo { ContainsGenericParameters: true } || t.ContainsGenericParameters) unsupported = "generic";
        else if (m is PropertyInfo { GetMethod: null or { IsPublic: false } }) unsupported = "no public getter";
        else if (m is MethodBase { IsAbstract: true } && !t.IsInterface) unsupported = "abstract";
        else if (m is ConstructorInfo && HandleTypes.Contains(t.Name)) unsupported = "constructor of a static-table type";

        var inputs = new List<List<object?>>();
        if (unsupported == null)
        {
            foreach (var p in parameters)
            {
                var values = Inputs(p);
                if (values == null)
                {
                    unsupported = "parameter type " + TypeName(p.ParameterType);
                    break;
                }
                inputs.Add(values);
            }
        }

        List<(JsonNode? Label, Func<object?> Make)> receivers = [(null, () => null)];
        if (unsupported == null && !isStatic)
        {
            var r = Golden.Receivers(t);
            if (r == null) unsupported = "receiver type " + TypeName(t);
            else receivers = r;
        }

        if (unsupported != null)
        {
            o["unsupported"] = unsupported;
            return o;
        }

        var argLists = Product(inputs, Math.Max(1, MaxCasesPerMember / Math.Max(1, receivers.Count)));
        receivers = Thin(receivers, Math.Max(1, MaxCasesPerMember / argLists.Count));

        var cases = new JsonArray();
        foreach (var (label, make) in receivers)
        foreach (var args in argLists)
        {
            var result = Enc.Try(() =>
            {
                var recv = make();
                return m switch
                {
                    ConstructorInfo c => c.Invoke(args),
                    MethodInfo mi => mi.Invoke(recv, args),
                    PropertyInfo pi => pi.GetValue(recv, args.Length == 0 ? null : args),
                    FieldInfo fi => fi.GetValue(recv),
                    _ => throw new NotSupportedException()
                };
            });
            cases.Add(new JsonArray(label?.DeepClone(), new JsonArray(args.Select(Enc.Value).ToArray()), result));
        }
        o["cases"] = cases;
        return o;
    }
}
