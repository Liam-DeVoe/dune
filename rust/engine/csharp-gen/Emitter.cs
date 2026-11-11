using System.Reflection;
using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization.Metadata;
using Microsoft.CodeAnalysis;
using Microsoft.CodeAnalysis.CSharp;
using Microsoft.CodeAnalysis.CSharp.Syntax;

namespace CsharpGen;

public sealed partial class Emitter
{
    public readonly Model Model;
    readonly string _out;
    readonly bool _force, _dry;
    readonly Dictionary<string, int> _todoCount = [];      // rust module path -> todo count
    readonly Dictionary<string, string> _moduleOf = [];    // C# rel path -> rust path (relative to src)
    readonly List<(string module, string rel)> _modules = [];
    public int MethodCount, FieldCount, TypeCount, TodoCount;
    readonly HashSet<string> _skippedExisting = [];

    // Members whose signature must be identical across overrides / implementations.
    readonly Dictionary<ISymbol, Flags> _groupFlags = new(SymbolEqualityComparer.Default);
    readonly Dictionary<ISymbol, ISymbol> _groupOf = new(SymbolEqualityComparer.Default);

    INamedTypeSymbol GameEvent => Model.GameEventSym;
    public readonly List<string> GroupLog = [];

    public Emitter(Model m, string outDir, bool force, bool dry)
    {
        Model = m;
        _out = outDir;
        _force = force;
        _dry = dry;
        foreach (var l in m.EnumsByFile.Values)
            foreach (var e in l) EnumNames.Add(e.Name);
        foreach (var t in typeof(Treachery.Shared.Game).Assembly.GetTypes().Where(t => t.IsEnum)) EnumNames.Add(t.Name);
    }

    public static string ModulePath(string rel)
    {
        var file = Path.GetFileNameWithoutExtension(rel);
        var snake = file == "ClairVoyanceQandA" ? "clair_voyance_q_and_a" : Snake(file);
        if (rel.StartsWith("Events/")) return "events/" + snake;
        if (rel.StartsWith("Model/Log/")) return "log/" + snake;
        return snake;
    }

    #region names

    static readonly HashSet<string> Keywords =
    [
        "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod",
        "move", "mut", "pub", "ref", "return", "self", "Self", "static", "struct", "super", "trait", "true", "type", "unsafe", "use", "where", "while",
        "async", "await", "dyn", "abstract", "become", "box", "do", "final", "macro", "override", "priv", "typeof", "unsized", "virtual", "yield", "try", "gen"
    ];

    public static string Snake(string s)
    {
        s = s.TrimStart('_');
        var sb = new StringBuilder();
        for (var i = 0; i < s.Length; i++)
        {
            var c = s[i];
            if (char.IsUpper(c))
            {
                if (i > 0 && sb.Length > 0 && sb[^1] != '_' &&
                    (char.IsLower(s[i - 1]) || char.IsDigit(s[i - 1]) || i + 1 < s.Length && char.IsLower(s[i + 1]) && char.IsUpper(s[i - 1])))
                    sb.Append('_');
                sb.Append(char.ToLowerInvariant(c));
            }
            else sb.Append(c);
        }
        return sb.ToString();
    }

    public static string Ident(string snake) => Keywords.Contains(snake) ? snake + "_" : snake;

    public static string Pascal(string snake)
    {
        snake = snake.TrimEnd('_');
        return string.Concat(snake.Split('_').Where(p => p.Length > 0).Select(p => char.ToUpperInvariant(p[0]) + p[1..]));
    }

    static string Lit(string s) => "\"" + s.Replace("\\", "\\\\").Replace("\"", "\\\"") + "\"";

    #endregion

    #region fields

    public List<FieldX> FieldsOf(TypeX t)
    {
        if (t.Fields.Count > 0 || t.Runtime == null) return t.Fields;
        foreach (var fi in t.Runtime.GetFields(BindingFlags.Instance | BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.DeclaredOnly))
        {
            var cs = Model.NormalizeFieldName(fi.Name);
            var fx = new FieldX { Raw = fi.Name, CsName = cs };
            ISymbol? sym = t.Sym.GetMembers(cs).FirstOrDefault(m => m is IFieldSymbol or IPropertySymbol);
            if (sym == null && fi.Name.EndsWith(">P"))
                sym = t.Sym.InstanceConstructors.SelectMany(c => c.Parameters).FirstOrDefault(p => p.Name == cs);
            if (sym == null)
            {
                Model.Warnings.Add($"no symbol for field {t.Name}.{fi.Name}");
                continue;
            }
            fx.Sym = sym;
            fx.Type = sym switch { IFieldSymbol f => f.Type, IPropertySymbol p => p.Type, IParameterSymbol p => p.Type, _ => null };
            (fx.File, fx.Line) = Model.Loc(sym);
            fx.Public = fi.IsPublic;
            var syntax = sym.DeclaringSyntaxReferences.FirstOrDefault()?.GetSyntax();
            fx.Init = syntax switch
            {
                VariableDeclaratorSyntax v => v.Initializer?.Value.ToString(),
                PropertyDeclarationSyntax p => p.Initializer?.Value.ToString(),
                _ => null
            };
            var key = t.Name + "." + cs;
            fx.Cell = Model.CellMembers.Contains(sym.OriginalDefinition);
            if (FieldTypeOverride.TryGetValue(key, out var ov)) fx.RustType = ov;
            else fx.RustType = RT(fx.Type!, IsJsonType(t) ? Pos.JsonField : Pos.Field);
            if (fx.RustType == "GameRef") fx.Cell = false;
            if (fx.Cell)
                fx.RustType = IsCopy(fx.RustType) ? $"Cell<{fx.RustType}>" : $"RefCell<{fx.RustType}>";
            fx.Omit = key is "LeaderState.TimeOfDeath";
            fx.Rust = Ident(Snake(cs));
            t.Fields.Add(fx);
        }

        // Disambiguate rust names (e.g. `_x` and `X`).
        foreach (var g in t.Fields.GroupBy(f => f.Rust).Where(g => g.Count() > 1))
            foreach (var f in g.Where(f => f.CsName.StartsWith('_')))
                f.Rust = Ident(Snake(f.CsName) + "_field");
        return t.Fields;
    }

    static string DeclText(ISymbol s)
    {
        var syn = s.DeclaringSyntaxReferences.FirstOrDefault()?.GetSyntax();
        if (syn is VariableDeclaratorSyntax v && v.Parent?.Parent is FieldDeclarationSyntax fd)
            return OneLine(string.Join(" ", fd.Modifiers.Select(m => m.Text)) + " " + fd.Declaration.Type + " " + v);
        if (syn is PropertyDeclarationSyntax p)
            return OneLine(string.Join(" ", p.Modifiers.Select(m => m.Text)) + " " + p.Type + " " + p.Identifier + " " + (p.AccessorList?.ToString() ?? "") + (p.Initializer?.ToString() ?? ""));
        if (syn is ParameterSyntax ps) return OneLine(ps.ToString()) + " (primary constructor)";
        return syn != null ? OneLine(syn.ToString()) : s.Name;
    }

    static string OneLine(string s)
    {
        s = System.Text.RegularExpressions.Regex.Replace(s, @"\s+", " ").Trim();
        if (s.Length > 160) s = s[..157] + "...";
        return s.Replace("`", "'");
    }

    // C# initializer -> Rust expression; null if not mechanical.
    string? Init(FieldX f)
    {
        var rt = f.RustType;
        var inner = rt;
        string Wrap(string e) => f.Cell ? (IsCopy(rt[5..^1]) ? $"Cell::new({e})" : $"RefCell::new({e})") : e;
        if (f.Cell) inner = rt[(rt.IndexOf('<') + 1)..^1];
        var isOpt = inner.StartsWith("Option<");
        var init = f.Init?.Trim();
        if (init == null || init is "null" or "null!" or "default" or "default!")
            return DefaultOk(inner) ? Wrap("Default::default()") : null;
        if (init is "[]" or "new()" || init.StartsWith("new List<") && init.EndsWith("()") || init.StartsWith("new HashSet<") && init.EndsWith("()") ||
            init.StartsWith("new Dictionary<") && init.EndsWith("()") || init.StartsWith("Array.Empty<"))
        {
            if (inner.StartsWith("Shared<")) return Wrap("Shared::new(Default::default())");
            if (isOpt && inner.Contains("Shared<")) return Wrap("Some(Shared::new(Default::default()))");
            if (isOpt) return Wrap("Some(Default::default())");
            return DefaultOk(inner) ? Wrap("Default::default()") : null;
        }
        string? v = null;
        if (System.Text.RegularExpressions.Regex.IsMatch(init, @"^-?\d+$")) v = init;
        else if (System.Text.RegularExpressions.Regex.IsMatch(init, @"^-?\d+(\.\d+)?[fFdD]?$")) v = init.TrimEnd('f', 'F', 'd', 'D') + (init.Contains('.') ? "" : ".0");
        else if (init is "true" or "false") v = init;
        else if (init is "string.Empty" or "String.Empty") v = "String::new()";
        else if (init.StartsWith('"') && init.EndsWith('"') && !init.Contains('\\')) v = init + ".to_string()";
        else if (System.Text.RegularExpressions.Regex.IsMatch(init, @"^[A-Z]\w*\.[A-Z]\w*$") && EnumNames.Contains(init.Split('.')[0])) v = init.Replace(".", "::");
        if (v == null) return null;
        return Wrap(isOpt ? $"Some({v})" : v);
    }

    bool DefaultOk(string rt)
    {
        if (rt.StartsWith("Option<") || rt.StartsWith("Vec<") || rt.StartsWith("NetHashSet<") || rt.StartsWith("NetDictionary<")) return true;
        if (rt is "i32" or "i64" or "u8" or "u32" or "f32" or "f64" or "bool" or "char" or "String" or "DateTimeOffset" or "TimeSpan" or "GameRef") return true;
        if (EnumNames.Contains(rt)) return true;
        if (rt.StartsWith("Shared<")) return DefaultOk(rt[7..^1]);
        if (rt == "GameEvent" || rt.StartsWith("Rc<")) return false;
        if (rt.StartsWith("(")) return true;
        if (Model.Types.TryGetValue(rt, out var t)) return Defaultable(t);
        return false;
    }

    readonly Dictionary<TypeX, bool> _defaultable = [];

    bool Defaultable(TypeX t)
    {
        if (_defaultable.TryGetValue(t, out var d)) return d;
        _defaultable[t] = false; // cycle guard
        if (t.Kind is TKind.Game or TKind.Player or TKind.Interface or TKind.Static) return false;
        var fields = FieldsOf(t);
        var ok = fields.All(f => Init(f) != null);
        if (t.Kind == TKind.Event)
            ok = ok && EventBaseChain(t).All(b => FieldsOf(b).All(f => Init(f) != null));
        _defaultable[t] = ok;
        return ok;
    }

    #endregion

    #region members

    IEnumerable<TypeX> EventBaseChain(TypeX t)
    {
        for (var b = t.Sym.BaseType; b != null && Model.IsEventType(b); b = b.BaseType)
            if (Model.Types.TryGetValue(b.Name, out var bt)) yield return bt;
    }

    Flags FlagsFor(IMethodSymbol? m)
    {
        if (m == null) return new Flags();
        m = (IMethodSymbol)m.OriginalDefinition;
        var key = GroupKey(m);
        if (key != null)
        {
            if (!_groupFlags.TryGetValue(key, out var gf))
            {
                gf = new Flags();
                foreach (var member in GroupMembers(key))
                {
                    var f = Model.FlagsOf.TryGetValue(member, out var ff) ? ff : new Flags();
                    // An interface member implemented by an override takes the override's group signature.
                    if (!SymbolEqualityComparer.Default.Equals(member, key) && GroupKey(member) is { } k2 && !SymbolEqualityComparer.Default.Equals(k2, key))
                        f = FlagsFor(member);
                    if (f.Game || f.NeedsRc) GroupLog.Add($"{key.ContainingType.Name}.{key.Name} <- {member.ContainingType.Name}: game={f.Game} rc={f.NeedsRc} self={f.Self}");
                    gf.Self |= f.Self; gf.Game |= f.Game; gf.UsesGame |= f.UsesGame || f.Game; gf.NeedsRc |= f.NeedsRc;
                }
                _groupFlags[key] = gf;
            }
            return gf;
        }
        return Model.FlagsOf.TryGetValue(m, out var fl) ? fl : new Flags();
    }

    // The root virtual/interface member a method belongs to, if any.
    ISymbol? GroupKey(IMethodSymbol m)
    {
        if (_groupOf.TryGetValue(m, out var k)) return k;
        ISymbol? key = null;
        IMethodSymbol r = m;
        while (r.OverriddenMethod != null) r = r.OverriddenMethod;
        if (!SymbolEqualityComparer.Default.Equals(r, m) || m.IsVirtual || m.IsAbstract) key = r.OriginalDefinition;
        if (m.AssociatedSymbol is IPropertySymbol p)
        {
            var rp = p;
            while (rp.OverriddenProperty != null) rp = rp.OverriddenProperty;
            if (!SymbolEqualityComparer.Default.Equals(rp, p) || p.IsVirtual || p.IsAbstract)
                key = m.MethodKind == MethodKind.PropertyGet ? rp.GetMethod?.OriginalDefinition : rp.SetMethod?.OriginalDefinition;
        }
        if (key == null)
        {
            // interface implementation (events only)
            foreach (var i in m.ContainingType.AllInterfaces)
                foreach (var im in i.GetMembers().OfType<IMethodSymbol>())
                    if (SymbolEqualityComparer.Default.Equals(m.ContainingType.FindImplementationForInterfaceMember(im)?.OriginalDefinition, m))
                        key = im.OriginalDefinition;
            if (m.ContainingType.TypeKind == TypeKind.Interface) key = m;
        }
        _groupOf[m] = key!;
        return key;
    }

    IEnumerable<IMethodSymbol> GroupMembers(ISymbol key)
    {
        var km = (IMethodSymbol)key;
        yield return km;
        foreach (var t in Model.Types.Values)
        {
            if (t.Sym == null) continue;
            if (km.ContainingType.TypeKind == TypeKind.Interface)
            {
                if (t.Sym.FindImplementationForInterfaceMember(km) is IMethodSymbol impl) yield return (IMethodSymbol)impl.OriginalDefinition;
                continue;
            }
            foreach (var m in t.Sym.GetMembers().OfType<IMethodSymbol>())
            {
                for (var o = m.OverriddenMethod; o != null; o = o.OverriddenMethod)
                    if (SymbolEqualityComparer.Default.Equals(o.OriginalDefinition, km)) { yield return (IMethodSymbol)m.OriginalDefinition; break; }
            }
            foreach (var p in t.Sym.GetMembers().OfType<IPropertySymbol>())
                for (var o = p.OverriddenProperty; o != null; o = o.OverriddenProperty)
                    if (SymbolEqualityComparer.Default.Equals(o.GetMethod?.OriginalDefinition, km) && p.GetMethod != null) { yield return p.GetMethod.OriginalDefinition; break; }
        }
        // interface members implemented by base classes of implementers
    }

    record struct Sig(string Receiver, List<string> Params, string Ret, string Generics);

    // Builds receiver + params for a method of `owner`.
    Sig BuildSig(TypeX owner, IMethodSymbol m, string ret, bool isStatic, Flags f, bool owned = false)
    {
        var ps = new List<string>();
        string recv = "";
        var hasGameParam = m.Parameters.Any(p => p.Type.Name == "Game");
        var gameRef = f.Game ? "&mut Game" : "&Game";
        // A Player parameter is an index: reading it needs the game.
        var playerParams = m.Parameters.Any(p => ContainsPlayer(p.Type));
        var meAfterGame = false;

        if (!isStatic)
        {
            switch (owner.Kind)
            {
                case TKind.Game:
                    recv = f.Self || f.Game ? "&mut self" : "&self";
                    break;
                case TKind.Player:
                {
                    var needsGame = f.UsesGame || hasGameParam || playerParams;
                    if (needsGame && (f.Self || f.Game) || f.Game)
                    {
                        recv = "";
                        if (!hasGameParam) { ps.Add("game: &mut Game"); ps.Add("me: PlayerIndex"); }
                        else meAfterGame = true;
                        gameRef = "&mut Game";
                    }
                    else
                    {
                        recv = f.Self ? "&mut self" : "&self";
                        if (needsGame && !hasGameParam) ps.Add("game: &Game");
                        gameRef = "&Game";
                    }
                    break;
                }
                case TKind.Event or TKind.EventBase:
                    recv = f.NeedsRc ? "self: &Rc<Self>" : "&self";
                    // A setter that only runs before the event is shared writes plain fields.
                    if (f.Self && m.MethodKind == MethodKind.PropertySet && !Model.CellMembers.Contains(m.AssociatedSymbol!.OriginalDefinition))
                        recv = "&mut self";
                    if ((f.UsesGame || f.Game || playerParams) && !hasGameParam) ps.Add("game: " + gameRef);
                    break;
                case TKind.Interface:
                    recv = f.NeedsRc ? "self: &Rc<Self>" : "&self";
                    if ((f.UsesGame || f.Game) && !hasGameParam) ps.Add("game: " + gameRef);
                    break;
                default:
                    recv = f.Self ? "&mut self" : "&self";
                    if ((owner.Sym.GetMembers().Any(x => Model.GameRefMembers.Contains(x)) && f.UsesGame || playerParams) && !hasGameParam)
                        ps.Add(f.Game ? "game: &mut Game" : "game: &Game");
                    if (hasGameParam && f.Self && f.Game)
                        Model.Warnings.Add($"borrow: {owner.Name}.{m.Name} mutates itself and the Game it is given ({Model.Loc(m).file}:{Model.Loc(m).line})");
                    break;
            }
        }

        if (isStatic && playerParams && !hasGameParam && m.MethodKind != MethodKind.Constructor)
            ps.Add("game: " + gameRef);

        foreach (var p in m.Parameters)
        {
            var name = Ident(Snake(p.Name));
            if (name == "me") name = "me_";
            string t;
            if (p.Type.Name == "Game" && p.Type.TypeKind == TypeKind.Class)
                t = p.NullableAnnotation == NullableAnnotation.Annotated ? $"Option<{gameRef}>" : gameRef;
            else if (p.IsParams)
                t = p.Type is IArrayTypeSymbol at ? $"Vec<{RT(at.ElementType, Pos.Elem)}>" : RT(p.Type, Pos.Ret);
            else
                t = RT(p.Type, owned ? Pos.Field : Pos.Param);
            if (!owned && Model.IsConcreteEvent(p.Type) && Model.EscapingParams.Contains(p.OriginalDefinition))
                t = t.Replace("&" + p.Type.Name, "&Rc<" + p.Type.Name + ">");
            if (p.RefKind is RefKind.Ref or RefKind.Out) t = "&mut " + RT(p.Type, Pos.Ret);
            ps.Add($"{name}: {t}");
            if (meAfterGame && p.Type.Name == "Game") { ps.Add("me: PlayerIndex"); meAfterGame = false; }
        }
        var generics = m.TypeParameters.Length > 0 ? "<" + string.Join(", ", m.TypeParameters.Select(tp => tp.Name)) + ">" : "";
        return new Sig(recv, ps, ret, generics);
    }

    static bool ContainsPlayer(ITypeSymbol t) => t.Name == "Player" ||
        t is IArrayTypeSymbol a && ContainsPlayer(a.ElementType) ||
        t is INamedTypeSymbol { IsGenericType: true } n && n.TypeArguments.Any(ContainsPlayer);

    static string FormatSig(string name, Sig s, bool pub = true)
    {
        var all = new List<string>();
        if (s.Receiver != "") all.Add(s.Receiver);
        all.AddRange(s.Params);
        var ret = s.Ret is "" or "()" ? "" : " -> " + s.Ret;
        return $"{(pub ? "pub " : "")}fn {name}{s.Generics}({string.Join(", ", all)}){ret}";
    }

    string CsSignature(ISymbol s)
    {
        var syn = s.DeclaringSyntaxReferences.FirstOrDefault()?.GetSyntax();
        switch (syn)
        {
            case MethodDeclarationSyntax md:
                return OneLine($"{string.Join(" ", md.Modifiers.Select(x => x.Text))} {md.ReturnType} {md.Identifier}{md.TypeParameterList}{md.ParameterList}");
            case ConstructorDeclarationSyntax cd:
                return OneLine($"{string.Join(" ", cd.Modifiers.Select(x => x.Text))} {cd.Identifier}{cd.ParameterList}{(cd.Initializer != null ? " " + cd.Initializer : "")}");
            case PropertyDeclarationSyntax pd:
                return OneLine($"{string.Join(" ", pd.Modifiers.Select(x => x.Text))} {pd.Type} {pd.Identifier}");
            case IndexerDeclarationSyntax id:
                return OneLine($"{string.Join(" ", id.Modifiers.Select(x => x.Text))} {id.Type} this{id.ParameterList}");
            case AccessorDeclarationSyntax ad when ad.Parent?.Parent is PropertyDeclarationSyntax pd2:
                return OneLine($"{string.Join(" ", pd2.Modifiers.Select(x => x.Text))} {pd2.Type} {pd2.Identifier} {{ {ad.Keyword.Text} }}");
            case VariableDeclaratorSyntax v when v.Parent?.Parent is FieldDeclarationSyntax fd:
                return OneLine($"{string.Join(" ", fd.Modifiers.Select(x => x.Text))} {fd.Declaration.Type} {v}");
            case OperatorDeclarationSyntax od:
                return OneLine($"{string.Join(" ", od.Modifiers.Select(x => x.Text))} {od.ReturnType} operator {od.OperatorToken}{od.ParameterList}");
        }
        return s.ToDisplayString();
    }

    #endregion
}
