using Microsoft.CodeAnalysis;

namespace CsharpGen;

public enum Pos { Field, JsonField, Param, Ret, Elem, Key }

// C# type -> Rust type, following rust/docs/engine-conventions.md.
public sealed partial class Emitter
{
    static readonly HashSet<string> Handles = ["Location", "Homeworld", "DiscoveredLocation", "HiddenMobileStronghold", "AttachedLocation", "Territory", "Leader", "TreacheryCard", "ResourceCard"];
    static readonly HashSet<string> FoundationModels = ["Message", "MessagePart", "Payment", "Report", "Expression", "Route", "BattlePlans", "PlanSummary", "BattleFact", "PointD", "HomeworldStatus", "Map"];
    static readonly HashSet<string> EventAbstractions = ["GameEvent", "PassableGameEvent", "PlacementEvent", "IBid", "ILocationEvent", "IPlacement"];
    static readonly HashSet<string> CopyTypes = ["i32", "i64", "u8", "u32", "f32", "f64", "bool", "char", "DateTimeOffset", "TimeSpan", "PlayerIndex", "GameRef", "Hero", "GameEventType"];

    // Per-member type overrides for C# reference sharing (hazards.md section 5) and owned containers.
    static readonly Dictionary<string, string> FieldTypeOverride = new()
    {
        ["Game.Players"] = "Vec<Player>",
        ["Game.Settings"] = "Shared<GameSettings>",
        ["Game.FactionsInPlay"] = "Shared<Vec<Faction>>",
        ["GameSettings.AllowedFactionsInPlay"] = "Option<Shared<Vec<Faction>>>",
        ["EstablishPlayers.Settings"] = "Option<Shared<GameSettings>>",
        ["Game.PreviousGameState"] = "Option<Box<Game>>",
        ["Game.Random"] = "Option<SharedRandom>",
        ["Game.Map"] = "Map",
    };

    public bool IsJsonType(TypeX t) => t.Kind is TKind.Event or TKind.EventBase || t.Name == "GameSettings" || t.Name == "GameState";

    public string RT(ITypeSymbol t, Pos pos)
    {
        // Nullable<T>
        if (t is INamedTypeSymbol { OriginalDefinition.SpecialType: SpecialType.System_Nullable_T } n)
            return $"Option<{RT(n.TypeArguments[0], pos == Pos.Param ? Pos.Param : Pos.Elem)}>";

        var core = Core(t, pos);
        var nullable = t.IsReferenceType && (t.NullableAnnotation == NullableAnnotation.Annotated || pos == Pos.JsonField);
        if (t is ITypeParameterSymbol { NullableAnnotation: NullableAnnotation.Annotated }) nullable = true;
        if (nullable && core != "GameRef" && !core.StartsWith("Option<") && core != "ExpressionElement" && !core.StartsWith("&dyn") && !core.StartsWith("fn("))
            return $"Option<{core}>";
        return core;
    }

    string Elem(ITypeSymbol t, Pos outer)
    {
        var e = RT(t, outer == Pos.JsonField ? Pos.Elem : Pos.Elem);
        return e;
    }

    string KeyElem(ITypeSymbol t)
    {
        var e = RT(t, Pos.Elem);
        if (e.StartsWith("Rc<")) return "RcKey<" + e[3..^1] + ">";
        return e;
    }

    string Core(ITypeSymbol t, Pos pos)
    {
        bool param = pos == Pos.Param;
        switch (t.SpecialType)
        {
            case SpecialType.System_Boolean: return "bool";
            case SpecialType.System_Int32: return "i32";
            case SpecialType.System_Int64: return "i64";
            case SpecialType.System_UInt32: return "u32";
            case SpecialType.System_Byte: return "u8";
            case SpecialType.System_Single: return "f32";
            case SpecialType.System_Double: return "f64";
            case SpecialType.System_Char: return "char";
            case SpecialType.System_String: return param ? "&str" : "String";
            case SpecialType.System_Object: return "ExpressionElement";
            case SpecialType.System_Void: return "()";
        }

        if (t is IArrayTypeSymbol arr)
        {
            var e = Elem(arr.ElementType, pos);
            return param ? $"&[{e}]" : $"Vec<{e}>";
        }

        if (t is ITypeParameterSymbol tp) return tp.Name;

        if (t is INamedTypeSymbol nt)
        {
            var name = nt.Name;
            var args = nt.TypeArguments;
            if (nt.IsTupleType)
                return "(" + string.Join(", ", nt.TupleElements.Select(x => RT(x.Type, Pos.Elem))) + ")";
            if (nt.IsGenericType)
            {
                switch (name)
                {
                    case "List" or "IList" or "IEnumerable" or "ICollection" or "IReadOnlyList" or "IReadOnlyCollection" or "LinkedList" or "Queue" or "Stack" or "IOrderedEnumerable":
                    {
                        var e = Elem(args[0], pos);
                        return param ? $"&[{e}]" : $"Vec<{e}>";
                    }
                    case "HashSet" or "ISet" or "IReadOnlySet":
                    {
                        var e = KeyElem(args[0]);
                        return param ? $"&NetHashSet<{e}>" : $"NetHashSet<{e}>";
                    }
                    case "Dictionary" or "IDictionary" or "IReadOnlyDictionary":
                    {
                        var k = KeyElem(args[0]);
                        var v = Elem(args[1], pos);
                        return param ? $"&NetDictionary<{k}, {v}>" : $"NetDictionary<{k}, {v}>";
                    }
                    case "KeyValuePair":
                        return $"({RT(args[0], Pos.Elem)}, {RT(args[1], Pos.Elem)})";
                    case "Func":
                    {
                        var ins = args.Take(args.Length - 1).Select(a => RT(a, Pos.Elem));
                        var r = RT(args[^1], Pos.Elem);
                        return $"&dyn Fn({string.Join(", ", ins)}) -> {r}";
                    }
                    case "Action":
                        return $"&dyn Fn({string.Join(", ", args.Select(a => RT(a, Pos.Elem)))})";
                    case "Predicate":
                        return $"&dyn Fn({RT(args[0], Pos.Elem)}) -> bool";
                    case "Comparison":
                        return $"&dyn Fn({RT(args[0], Pos.Elem)}, {RT(args[0], Pos.Elem)}) -> i32";
                    case "IFetcher":
                        return $"&dyn IFetcher<{RT(args[0], Pos.Elem)}>";
                    case "IEqualityComparer":
                        return $"&dyn Fn({RT(args[0], Pos.Elem)}, {RT(args[0], Pos.Elem)}) -> bool";
                    case "Deck" or "Timer" or "ObjectCounter":
                    {
                        var g = $"{name}<{RT(args[0], Pos.Elem)}>";
                        return param ? "&" + g : g;
                    }
                }
                Model.Warnings.Add($"unmapped generic type {t.ToDisplayString()}");
                return $"() /* C#: {t.ToDisplayString()} */";
            }

            if (nt.TypeKind == TypeKind.Enum) return name;
            if (nt.TypeKind == TypeKind.Delegate)
            {
                if (name == "Action") return "fn(&mut Game)";
            }

            switch (name)
            {
                case "Game": return "GameRef";
                case "Player": return "PlayerIndex";
                case "IHero" or "Messiah": return "Hero";
                case "LoggedRandom": return param ? "&SharedRandom" : "SharedRandom";
                case "Random": return param ? "&mut NetRandom" : "NetRandom";
                case "DateTimeOffset" or "DateTime": return "DateTimeOffset";
                case "TimeSpan": return "TimeSpan";
                case "Type": return "GameEventType";
                case "Exception": return "CsException";
                case "Action": return "fn(&mut Game)";
                case "IIdentifiable": return "&dyn IIdentifiable";
            }
            if (Handles.Contains(name)) return name;
            if (EventAbstractions.Contains(name)) return param ? "&GameEvent" : "GameEvent";
            if (Model.IsConcreteEvent(nt)) return param ? $"&{name}" : $"Rc<{name}>";
            if (FoundationModels.Contains(name) || Model.Types.ContainsKey(name))
                return param ? "&" + name : name;
        }

        Model.Warnings.Add($"unmapped type {t.ToDisplayString()}");
        return $"() /* C#: {t.ToDisplayString()} */";
    }

    public static bool IsCopy(string rt)
    {
        if (CopyTypes.Contains(rt) || Handles.Contains(rt)) return true;
        if (rt.StartsWith("Option<")) return IsCopy(rt[7..^1]);
        if (rt.Length > 0 && char.IsUpper(rt[0]) && !rt.Contains('<') && EnumNames.Contains(rt)) return true;
        return false;
    }

    public static HashSet<string> EnumNames = [];
}
