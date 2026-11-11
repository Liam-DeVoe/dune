// Writes the event JSON corpus for rust/engine/tests/event_json.rs:
//
//   dotnet run -c Release -- --events ../tests/golden/events.jsonl
//
// Every event type with a [JsonDerivedType] gets several instances whose JSON
// members are set to random values (including nulls, undefined enum values and
// strings that need escaping), serialized with Utilities.Serialize<GameEvent>.
// Then malformed variants of those lines are fed to Utilities.Deserialize to
// record which ones .NET accepts. Each output line is
//   {"json": <input text>, "ok": <bool>, "out": <re-serialization or null>}

using System.Reflection;
using System.Text.Json;
using System.Text.Json.Nodes;
using System.Text.Json.Serialization;
using System.Text.Json.Serialization.Metadata;
using Treachery.Shared;

public static class EventCorpus
{
    const int PerType = 6;

    static readonly JsonSerializerOptions Stj = new() { IncludeFields = true, TypeInfoResolver = new DefaultJsonTypeInfoResolver() };

    static readonly string[] Strings =
    [
        "", "plain", "1:2|3,4:0|1", "12;7;0", "quote\" back\\slash", "tab\tnew\nline\r\u0001\u001f\u007f", "<a href='x'>&+`</a>",
        "\u00dcn\u00efc\u00f6d\u00e9 \u2713", "emoji \ud83d\ude00 pair", "/slash", "Treachery.online", "\u0085  ", "  spaced  "
    ];

    public static void Write(string path)
    {
        var rnd = new Random(12345);
        var lines = new List<string>();
        var eventTypes = typeof(GameEvent).GetCustomAttributes<JsonDerivedTypeAttribute>().Select(a => a.DerivedType).OrderBy(t => t.Name, StringComparer.Ordinal).ToList();
        var valid = new List<string>();
        foreach (var t in eventTypes)
        {
            for (var i = 0; i < PerType; i++)
            {
                var e = (GameEvent)Activator.CreateInstance(t)!;
                if (i > 0) Randomize(e, rnd); // instance 0 keeps the C# defaults
                var json = Utilities.Serialize<GameEvent>(e);
                valid.Add(json);
                lines.Add(Line(json));
            }
        }

        // A savegame wrapper.
        var state = new GameState { Version = 184, Events = valid.Take(12).Select(j => Utilities.Deserialize<GameEvent>(j)!).ToList() };
        lines.Add(LineState(Utilities.Serialize(state)));

        // Malformed and unusual inputs.
        foreach (var json in valid.Where((_, i) => i % 7 == 3))
            foreach (var variant in Variants(JsonNode.Parse(json)!.AsObject(), rnd))
                lines.Add(Line(variant));

        // `$` members inside non-polymorphic objects (GameSettings) are unknown members to
        // .NET and are skipped; in an event they are rejected. Every event with a nested
        // object gets them, as does the savegame wrapper.
        foreach (var json in valid)
            foreach (var variant in DollarVariants(JsonNode.Parse(json)!.AsObject()))
                lines.Add(Line(variant));
        foreach (var key in new[] { "$foo", "$id", "$ref", "$type", "$values" })
        {
            var stateNode = JsonNode.Parse(Utilities.Serialize(state))!.AsObject();
            stateNode[key] = key == "$values" ? new JsonArray() : "1";
            lines.Add(LineState(stateNode.ToJsonString()));
        }

        File.WriteAllLines(path, lines);
        Console.WriteLine($"wrote {lines.Count} lines to {path}");

        var errorsPath = Path.Combine(Path.GetDirectoryName(Path.GetFullPath(path))!, "event_errors.jsonl");
        var errorLines = UnknownDiscriminators().Select(c => ErrorLine(c.Kind, c.Json)).ToList();
        File.WriteAllLines(errorsPath, errorLines);
        Console.WriteLine($"wrote {errorLines.Count} lines to {errorsPath}");
    }

    // Savegames written before an event class was renamed (Blue* to BeneGesserit*,
    // PerformYellowSetup) no longer load. Rust must throw the same exception, with the same
    // message, path and position. Written to event_errors.jsonl as
    //   {"kind": "event"|"state", "json": <input>, "error": "<exception type>: <message>"}
    static IEnumerable<(string Kind, string Json)> UnknownDiscriminators()
    {
        yield return ("event", "{\"$type\":\"BluePrediction\",\"ToWin\":30,\"Turn\":7,\"Initiator\":60}");
        yield return ("event", "{\"$type\":\"BlueBattleAnnouncement\"}");
        yield return ("event", " { \"$type\" : \"BluePrediction\" , \"ToWin\" : 30 }");
        yield return ("event", "{\n  \"$type\": \"BluePrediction\",\n  \"ToWin\": 30\n}");
        yield return ("event", "{\r\n  \"$type\": \"BluePrediction\",\r\n  \"ToWin\": 30\r\n}");
        yield return ("event", "{\"$type\":\"BluePrediction\",\"$foo\":1}");
        yield return ("event", "{\"$type\":\"Bl\\u00e4h\u00e4\",\"ToWin\":\"\u00e4\"}");
        yield return ("event", "{\"$type\":\"Blue\",\"Time\":\"2026-09-21T03:33:01.491994-04:00\"}");
        yield return ("state", "{\"Version\":187,\"_events\":[{\"$type\":\"BluePrediction\",\"ToWin\":30}]}");
        yield return ("state", "{\"Version\":187,\"_events\":[{\"$type\":\"EndPhase\",\"Initiator\":20},{\"$type\":\"PerformYellowSetup\",\"Initiator\":40}]}");
        yield return ("state", "{\"_events\":[{\"$type\":\"Bid\",\"Amount\":3},{\"$type\":\"EndPhase\",\"Initiator\":20,\"Time\":\"2026-09-21T03:33:01.491994-04:00\"},{\"$type\":\"BlueAccompanies\"}],\"Version\":187}");
        yield return ("state", "{\n \"Version\": 187,\n \"_events\": [\n  {\"$type\":\"EndPhase\",\"Note\":\"a]}\\\"[{\"},\n  {\"$type\":\"BluePrediction\"}\n ]\n}");
        yield return ("state", "{\"Version\":187,\"Other\":{\"_events\":[1]},\"_events\":[{\"$type\":\"EndPhase\",\"Initiator\":20},{\"$type\":\"BluePrediction\"}]}");
    }

    static string ErrorLine(string kind, string json)
    {
        string error;
        try
        {
            if (kind == "state") GameState.Load(json);
            else Utilities.Deserialize<GameEvent>(json);
            error = "";
        }
        catch (Exception e)
        {
            error = e.GetType().Name + ": " + e.Message;
        }
        return new JsonObject { ["kind"] = kind, ["json"] = json, ["error"] = error }.ToJsonString();
    }

    static string Line(string json)
    {
        string? output = null;
        bool ok;
        try
        {
            var e = Utilities.Deserialize<GameEvent>(json);
            ok = e != null;
            if (e != null) output = Utilities.Serialize<GameEvent>(e);
        }
        catch (Exception)
        {
            ok = false;
        }
        return new JsonObject { ["kind"] = "event", ["json"] = json, ["ok"] = ok, ["out"] = output }.ToJsonString();
    }

    static string LineState(string json)
    {
        string? output = null;
        var ok = true;
        try
        {
            output = Utilities.Serialize(Utilities.Deserialize<GameState>(json)!);
        }
        catch (Exception)
        {
            ok = false;
        }
        return new JsonObject { ["kind"] = "state", ["json"] = json, ["ok"] = ok, ["out"] = output }.ToJsonString();
    }

    static IEnumerable<string> DollarVariants(JsonObject o)
    {
        foreach (var p in o.Select(kv => kv.Key).ToList())
        {
            if (o[p] is not JsonObject) continue;
            foreach (var key in new[] { "$foo", "$id", "$ref", "$type", "$values" })
            {
                var c = JsonNode.Parse(o.ToJsonString())!.AsObject();
                var inner = c[p]!.AsObject();
                inner[key] = key == "$values" ? new JsonArray() : "1";
                yield return c.ToJsonString();
                // The same key first, before the known members.
                var reordered = new JsonObject { [key] = key == "$values" ? new JsonArray() : "1" };
                foreach (var kv in c[p]!.AsObject().ToList())
                    if (kv.Key != key) reordered[kv.Key] = kv.Value?.DeepClone();
                c[p] = reordered;
                yield return c.ToJsonString();
            }
        }
    }

    static IEnumerable<string> Variants(JsonObject o, Random rnd)
    {
        var props = o.Select(kv => kv.Key).Where(k => k != "$type").ToList();
        var type = o["$type"]!.GetValue<string>();
        string With(Action<JsonObject> f)
        {
            var c = JsonNode.Parse(o.ToJsonString())!.AsObject();
            f(c);
            return c.ToJsonString();
        }

        string Raw(string member, string rawValue)
        {
            // Replace a member's value textually (JsonNode cannot hold 1.0 vs 1 distinctly).
            var c = JsonNode.Parse(o.ToJsonString())!.AsObject();
            c[member] = "__RAW__";
            return c.ToJsonString().Replace("\"__RAW__\"", rawValue);
        }

        yield return With(c => c.Remove("$type"));
        yield return With(c => { var v = c["$type"]!.GetValue<string>(); c.Remove("$type"); c["$type"] = v; });
        yield return With(c => c["$type"] = type.ToLowerInvariant());
        yield return With(c => c["$type"] = "NoSuchEvent");
        yield return With(c => c["$id"] = "1");
        yield return With(c => c["UnknownMember"] = 5);
        yield return "{\"$type\":\"" + type + "\"}";
        yield return With(c => c["Time"] = "2024-03-01T10:00:00Z");
        // A date without an offset is machine-local time in .NET, so it is not in the corpus.
        yield return With(c => c["Time"] = "2024-03-01T10:00:00.123456789+02:00");
        foreach (var p in props)
        {
            var v = o[p];
            if (v is JsonValue jv && jv.TryGetValue<int>(out var n))
            {
                yield return Raw(p, n + ".0");
                yield return Raw(p, "\"" + n + "\"");
                yield return Raw(p, "null");
                yield return Raw(p, "2147483648");
                yield return Raw(p, "-0");
            }
            else if (v is JsonValue jb && jb.TryGetValue<bool>(out _))
            {
                yield return Raw(p, "1");
                yield return Raw(p, "null");
            }
            else if (v is JsonValue js && js.TryGetValue<string>(out _))
            {
                yield return Raw(p, "null");
                yield return Raw(p, "5");
            }
            else if (v is JsonArray)
            {
                yield return Raw(p, "null");
                yield return Raw(p, "{}");
            }
        }
    }

    static void Randomize(object o, Random rnd)
    {
        var ti = Stj.GetTypeInfo(o.GetType());
        foreach (var p in ti.Properties)
        {
            if (p.Set == null || p.Get == null) continue;
            p.Set(o, Value(p.PropertyType, rnd));
        }
    }

    static object? Value(Type t, Random rnd)
    {
        var nullable = Nullable.GetUnderlyingType(t);
        if (nullable != null) return rnd.Next(4) == 0 ? null : Value(nullable, rnd);
        if (t == typeof(int)) return rnd.Next(6) switch { 0 => -1, 1 => 0, 2 => int.MaxValue, 3 => int.MinValue, _ => rnd.Next(-5, 120) };
        if (t == typeof(bool)) return rnd.Next(2) == 0;
        if (t.IsEnum) return Enum.ToObject(t, rnd.Next(8) == 0 ? rnd.Next(-3, 300) : rnd.Next(0, 14));
        if (t == typeof(string)) return rnd.Next(8) == 0 ? null : Strings[rnd.Next(Strings.Length)];
        if (t == typeof(DateTimeOffset))
        {
            var ticks = rnd.NextInt64(DateTimeOffset.UnixEpoch.Ticks, new DateTime(2100, 1, 1).Ticks);
            if (rnd.Next(3) == 0) ticks -= ticks % TimeSpan.TicksPerSecond;
            var offset = TimeSpan.FromMinutes(rnd.Next(-14 * 60, 14 * 60 + 1));
            return new DateTimeOffset(ticks, offset);
        }
        if (t.IsArray)
        {
            if (rnd.Next(6) == 0) return null;
            var n = rnd.Next(5);
            var arr = Array.CreateInstance(t.GetElementType()!, n);
            for (var i = 0; i < n; i++) arr.SetValue(Value(t.GetElementType()!, rnd), i);
            return arr;
        }
        if (t.IsGenericType && t.GetGenericTypeDefinition() == typeof(List<>))
        {
            if (rnd.Next(6) == 0) return null;
            var list = (System.Collections.IList)Activator.CreateInstance(t)!;
            var n = rnd.Next(5);
            for (var i = 0; i < n; i++) list.Add(Value(t.GetGenericArguments()[0], rnd));
            return list;
        }
        if (t.IsGenericType && t.GetGenericTypeDefinition() == typeof(Dictionary<,>))
        {
            if (rnd.Next(4) == 0) return null;
            var d = (System.Collections.IDictionary)Activator.CreateInstance(t)!;
            var n = rnd.Next(4);
            for (var i = 0; i < n; i++)
            {
                var k = Value(t.GetGenericArguments()[0], rnd)!;
                d[k] = Value(t.GetGenericArguments()[1], rnd);
            }
            return d;
        }
        if (t == typeof(GameSettings))
        {
            if (rnd.Next(5) == 0) return null;
            var s = new GameSettings();
            Randomize(s, rnd);
            return s;
        }
        throw new NotSupportedException("no generator for " + t);
    }
}
