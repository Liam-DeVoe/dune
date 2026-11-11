// Writes the field list the oracle's state dumper (Oracle/Canon/StateDumper.cs) uses for every
// object type reachable from Game, plus every other concrete class or struct in Treachery.Shared.
// The Rust test rust/differ/tests/field_mapping.rs compares it with the Rust Canon derive, so a
// field that is missing or misnamed in Rust fails before the logic that fills it is ported.
//
// The rules copy StateDumper: every instance field of the type and its bases, with backing-field
// names normalized, minus the fields the dumper omits. Types the dumper writes as references or
// specially (Game's special fields, Leader, TreacheryCard, LoggedRandom, collections) are not
// expanded here.

using System.Collections;
using System.Reflection;
using System.Text.Json;
using System.Text.Json.Nodes;
using Treachery.Shared;

static class CanonSchema
{
    private static readonly Assembly Shared = typeof(Game).Assembly;

    public static void Write(string path)
    {
        var types = new SortedDictionary<string, JsonObject>(StringComparer.Ordinal);
        var reachable = new HashSet<string>();
        var queue = new Queue<Type>();
        queue.Enqueue(typeof(Game));
        // Message.Log, Expression.Elements and BattleFact.Cause are `object`; the dumper meets
        // log records, messages, message parts and payments through them.
        foreach (var t in Shared.GetTypes().Where(t => t.Namespace == "Treachery.Shared" && IsLogRecord(t))) queue.Enqueue(t);
        queue.Enqueue(typeof(Message));
        queue.Enqueue(typeof(MessagePart));
        queue.Enqueue(typeof(Payment));
        foreach (var t in Shared.GetTypes().Where(t => typeof(GameEvent).IsAssignableFrom(t) && !t.IsAbstract)) queue.Enqueue(t);

        var seen = new HashSet<Type>();
        while (queue.Count > 0)
        {
            var t = queue.Dequeue();
            if (!seen.Add(t)) continue;
            if (!Expands(t)) continue;
            var name = TypeName(t);
            reachable.Add(name);
            types[name] = Describe(t, queue);
        }

        // Everything else, for Rust types that the walk above does not reach.
        foreach (var t in Shared.GetTypes())
        {
            if (t.IsGenericTypeDefinition || t.IsEnum || t.IsInterface || t.IsAbstract || !Expands(t)) continue;
            if (t.IsDefined(typeof(System.Runtime.CompilerServices.CompilerGeneratedAttribute))) continue;
            if (t.Name.Contains('<') || t.IsNested && t.DeclaringType!.IsDefined(typeof(System.Runtime.CompilerServices.CompilerGeneratedAttribute))) continue;
            if (t.IsAbstract && t.IsSealed) continue; // static class
            var name = TypeName(t);
            if (!types.ContainsKey(name)) types[name] = Describe(t, new Queue<Type>());
        }

        var root = new JsonObject();
        foreach (var (name, o) in types)
        {
            o["reachable"] = reachable.Contains(name);
            root[name] = o;
        }

        File.WriteAllText(path, root.ToJsonString(new JsonSerializerOptions { WriteIndented = true }) + "\n");
        Console.Error.WriteLine($"canon schema: {types.Count} types, {reachable.Count} reachable from Game");
    }

    private static bool IsLogRecord(Type t) => LogNames.Contains(t.Name);

    // Records declared in Shared/Model/Log (they share the Treachery.Shared namespace).
    private static readonly HashSet<string> LogNames = Directory.Exists(LogDir())
        ? Directory.GetFiles(LogDir(), "*.cs").SelectMany(File.ReadAllLines)
            .Select(l => l.Trim())
            .Select(l => System.Text.RegularExpressions.Regex.Match(l, @"^public (?:sealed )?(?:record|class) (\w+)"))
            .Where(m => m.Success)
            .Select(m => m.Groups[1].Value)
            .ToHashSet()
        : [];

    private static string LogDir()
    {
        var dir = AppContext.BaseDirectory;
        for (var i = 0; i < 10 && dir != null; i++)
        {
            var candidate = Path.Combine(dir, "treachery-online", "Shared", "Model", "Log");
            if (Directory.Exists(candidate)) return candidate;
            dir = Path.GetDirectoryName(dir);
        }

        return "";
    }

    private static JsonObject Describe(Type t, Queue<Type> queue)
    {
        var fields = new SortedDictionary<string, string>(StringComparer.Ordinal);
        for (var cur = t; cur != null && cur != typeof(object) && cur != typeof(ValueType); cur = cur.BaseType)
        {
            foreach (var fi in cur.GetFields(BindingFlags.Instance | BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.DeclaredOnly))
            {
                if (Omitted(cur, fi.Name)) continue;
                fields[Normalize(fi.Name)] = TypeName(fi.FieldType);
                if (!SpecialGameField(cur, fi.Name)) Walk(fi.FieldType, queue);
            }
        }

        var o = new JsonObject();
        var f = new JsonObject();
        foreach (var (k, v) in fields) f[k] = v;
        o["fields"] = f;
        return o;
    }

    /// Queues the object types a field of this declared type can hold.
    private static void Walk(Type t, Queue<Type> queue)
    {
        if (Nullable.GetUnderlyingType(t) is { } u) t = u;
        if (t.IsArray)
        {
            Walk(t.GetElementType()!, queue);
            return;
        }

        if (t.IsGenericType && t.Namespace != null && t.Namespace.StartsWith("System"))
        {
            foreach (var a in t.GetGenericArguments()) Walk(a, queue);
            return;
        }

        if (t.Assembly != Shared) return;
        if (t.IsGenericType)
            foreach (var a in t.GetGenericArguments()) Walk(a, queue);

        // A field of a base or interface type can hold any concrete subtype.
        foreach (var s in Shared.GetTypes())
            if (!s.IsAbstract && !s.IsInterface && !s.IsGenericTypeDefinition && t.IsAssignableFrom(s))
                queue.Enqueue(s);
        if (!t.IsAbstract && !t.IsInterface) queue.Enqueue(t);
    }

    /// Whether the dumper writes values of this type as objects with fields.
    private static bool Expands(Type t)
    {
        if (t.Assembly != Shared) return false;
        if (t.IsEnum || t.IsInterface || t.IsGenericTypeDefinition) return false;
        if (typeof(Delegate).IsAssignableFrom(t) || typeof(Exception).IsAssignableFrom(t)) return false;
        if (typeof(Leader).IsAssignableFrom(t) || typeof(Messiah).IsAssignableFrom(t) || typeof(TreacheryCard).IsAssignableFrom(t)) return false;
        if (typeof(LoggedRandom).IsAssignableFrom(t)) return false;
        if (typeof(IEnumerable).IsAssignableFrom(t) && t.Namespace != null && t.Namespace.StartsWith("System")) return false;
        return true;
    }

    private static bool SpecialGameField(Type declaring, string raw) =>
        declaring == typeof(Game) && raw is "<History>k__BackingField" or "<PreviousGameState>k__BackingField" or "<Random>k__BackingField";

    private static bool Omitted(Type declaring, string raw) =>
        (declaring == typeof(Message) && raw == "<Nr>k__BackingField") ||
        (declaring == typeof(LeaderState) && raw == "<TimeOfDeath>k__BackingField") ||
        (declaring == typeof(Map) && raw is "_neighbourCache" or "_neighbourCacheTimestamp");

    private static string Normalize(string raw)
    {
        if (raw.StartsWith('<'))
        {
            var close = raw.IndexOf('>');
            var suffix = raw[(close + 1)..];
            if (suffix is "k__BackingField" or "P") return raw[1..close];
        }

        return raw;
    }

    private static string TypeName(Type t)
    {
        if (t.IsArray) return TypeName(t.GetElementType()!) + "[]";
        if (!t.IsGenericType) return t.Name;
        var name = t.Name;
        var tick = name.IndexOf('`');
        if (tick >= 0) name = name[..tick];
        return name + "<" + string.Join(",", t.GetGenericArguments().Select(TypeName)) + ">";
    }
}
