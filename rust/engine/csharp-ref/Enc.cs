// Value encoding shared by the golden files that compare results value by
// value (boundary.json, random_helpers.json, linq.json). The Rust tests
// decode and produce the same encoding (tests/common/mod.rs).
//
//   null                    -> JSON null
//   bool, int, long         -> JSON bool / number
//   float                   -> "f:" + ToString(InvariantCulture)   ("f:NaN", "f:1E-05")
//   double                  -> "d:" + ToString(InvariantCulture)
//   string                  -> "s:" + value
//   enum                    -> "e:" + type name + ":" + numeric value
//   IIdentifiable           -> "o:" + runtime type name + "#" + Id
//   ResourceCard            -> "rc:" + index in Map.GetResourceCardsInAndOutsidePlay
//   PointD                  -> "pt:" + X + "," + Y
//   Message, MessagePart    -> "m:" + ToString()
//   IEnumerable             -> JSON array
//   anything else           -> "?:" + runtime type name
//   a thrown exception      -> "x:" + type name + ":" + Message

using System.Collections;
using System.Globalization;
using System.Reflection;
using System.Text.Json.Nodes;
using Treachery.Shared;

public static class Enc
{
    private static List<ResourceCard>? _resourceCards;

    public static List<ResourceCard> ResourceCards
    {
        get
        {
            if (_resourceCards == null)
            {
                var method = typeof(Map).GetMethod("GetResourceCardsInAndOutsidePlay", BindingFlags.NonPublic | BindingFlags.Static)!;
                _resourceCards = ((IEnumerable<ResourceCard>)method.Invoke(null, [Golden.SampleMap])!).ToList();
            }
            return _resourceCards;
        }
    }

    public static JsonNode? Value(object? v)
    {
        switch (v)
        {
            case null: return null;
            case bool b: return JsonValue.Create(b);
            case int i: return JsonValue.Create(i);
            case long l: return JsonValue.Create(l);
            case float f: return JsonValue.Create("f:" + f.ToString(CultureInfo.InvariantCulture));
            case double d: return JsonValue.Create("d:" + d.ToString(CultureInfo.InvariantCulture));
            case string s: return JsonValue.Create("s:" + s);
            case Enum e: return JsonValue.Create("e:" + e.GetType().Name + ":" + Convert.ToInt64(e).ToString(CultureInfo.InvariantCulture));
            case IIdentifiable o: return JsonValue.Create("o:" + o.GetType().Name + "#" + o.Id);
            case ResourceCard rc: return JsonValue.Create("rc:" + ResourceCards.IndexOf(rc));
            case PointD p: return JsonValue.Create("pt:" + p.X.ToString(CultureInfo.InvariantCulture) + "," + p.Y.ToString(CultureInfo.InvariantCulture));
            case Message m: return JsonValue.Create("m:" + m.ToString());
            case MessagePart m: return JsonValue.Create("m:" + m.ToString());
            case IEnumerable list:
            {
                var a = new JsonArray();
                foreach (var x in list) a.Add(Value(x));
                return a;
            }
            default: return JsonValue.Create("?:" + v.GetType().Name);
        }
    }

    public static string Exception(Exception e)
    {
        if (e is TargetInvocationException { InnerException: not null } t) e = t.InnerException!;
        return "x:" + e.GetType().Name + ":" + e.Message;
    }

    // Runs f; the encoded result, or the encoded exception.
    public static JsonNode? Try(Func<object?> f)
    {
        try
        {
            return Value(f());
        }
        catch (Exception e)
        {
            return JsonValue.Create(Exception(e));
        }
    }
}
