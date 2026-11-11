using System.Reflection;
using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization.Metadata;
using Microsoft.CodeAnalysis;
using Microsoft.CodeAnalysis.CSharp.Syntax;

namespace CsharpGen;

public sealed partial class Emitter
{
    readonly Dictionary<TypeX, List<MemberX>> _members = [];
    public bool Glue;
    readonly StringBuilder _report = new();

    static readonly HashSet<string> BaseClasses = ["GameEvent", "PassableGameEvent", "PlacementEvent"];

    string TraitOf(string baseClass) => baseClass + "Methods";

    #region member collection

    List<MemberX> MembersOf(TypeX t)
    {
        if (_members.TryGetValue(t, out var list)) return list;
        list = [];
        _members[t] = list;
        if (t.Sym == null) return list;
        foreach (var rel in t.Files)
        {
            var decls = t.Sym.DeclaringSyntaxReferences.Select(r => r.GetSyntax()).OfType<TypeDeclarationSyntax>()
                .Where(d => Model.Rel(d.SyntaxTree) == rel);
            foreach (var decl in decls)
            {
                // Positional record: implicit constructor.
                if (decl.ParameterList != null && t.IsRecord)
                {
                    var ctor = t.Sym.InstanceConstructors.First(c => c.Parameters.Length == decl.ParameterList.Parameters.Count && !c.IsImplicitlyDeclared || c.Parameters.Length == decl.ParameterList.Parameters.Count);
                    list.Add(new MemberX { Sym = ctor, Method = ctor, Kind = MKind.Ctor, File = rel, Line = Model.Loc(decl).line, CsSig = OneLine($"record {decl.Identifier}{decl.ParameterList}") });
                }
                else if (decl.ParameterList != null)
                {
                    var ctor = t.Sym.InstanceConstructors.First(c => c.Parameters.Length == decl.ParameterList.Parameters.Count);
                    list.Add(new MemberX { Sym = ctor, Method = ctor, Kind = MKind.Ctor, File = rel, Line = Model.Loc(decl).line, CsSig = OneLine($"class {decl.Identifier}{decl.ParameterList} (primary constructor)") });
                }

                var sm = Model.Comp.GetSemanticModel(decl.SyntaxTree);
                foreach (var mem in decl.Members)
                {
                    var line = Model.Loc(mem).line;
                    switch (mem)
                    {
                        case FieldDeclarationSyntax fd:
                            foreach (var v in fd.Declaration.Variables)
                            {
                                var fs = (IFieldSymbol)sm.GetDeclaredSymbol(v)!;
                                if (fs.IsConst) list.Add(new MemberX { Sym = fs, Kind = MKind.Const, File = rel, Line = line, IsStatic = true, CsSig = CsSignature(fs) });
                                else if (fs.IsStatic) list.Add(new MemberX { Sym = fs, Kind = MKind.StaticField, File = rel, Line = line, IsStatic = true, CsSig = CsSignature(fs) });
                            }
                            break;
                        case PropertyDeclarationSyntax pd:
                        {
                            var ps = (IPropertySymbol)sm.GetDeclaredSymbol(pd)!;
                            var isAuto = pd.ExpressionBody == null && pd.AccessorList != null && pd.AccessorList.Accessors.All(a => a.Body == null && a.ExpressionBody == null);
                            var inInterface = t.Kind == TKind.Interface;
                            if (isAuto && !inInterface && !ps.IsAbstract)
                            {
                                if (ps.IsStatic) list.Add(new MemberX { Sym = ps, Kind = MKind.StaticField, File = rel, Line = line, IsStatic = true, CsSig = CsSignature(ps) });
                                break; // instance auto-property: a struct field
                            }
                            if (ps.GetMethod != null)
                                list.Add(new MemberX { Sym = ps, Method = ps.GetMethod, Kind = MKind.Getter, File = rel, Line = line, IsStatic = ps.IsStatic, CsSig = CsSignature(ps) });
                            var setter = pd.AccessorList?.Accessors.FirstOrDefault(a => a.Keyword.Text is "set" or "init");
                            if (ps.SetMethod != null && (setter?.Body != null || setter?.ExpressionBody != null || inInterface || ps.IsAbstract))
                                list.Add(new MemberX { Sym = ps, Method = ps.SetMethod, Kind = MKind.Setter, File = rel, Line = Model.Loc(setter!).line, IsStatic = ps.IsStatic, CsSig = CsSignature(ps) + " { set }" });
                            break;
                        }
                        case IndexerDeclarationSyntax id:
                        {
                            var ps = (IPropertySymbol)sm.GetDeclaredSymbol(id)!;
                            if (ps.GetMethod != null) list.Add(new MemberX { Sym = ps, Method = ps.GetMethod, Kind = MKind.Indexer, File = rel, Line = line, CsSig = CsSignature(ps) });
                            break;
                        }
                        case MethodDeclarationSyntax md:
                        {
                            var ms = (IMethodSymbol)sm.GetDeclaredSymbol(md)!;
                            list.Add(new MemberX { Sym = ms, Method = ms, Kind = MKind.Method, File = rel, Line = line, IsStatic = ms.IsStatic, CsSig = CsSignature(ms) });
                            break;
                        }
                        case ConstructorDeclarationSyntax cd:
                        {
                            var ms = (IMethodSymbol)sm.GetDeclaredSymbol(cd)!;
                            list.Add(new MemberX { Sym = ms, Method = ms, Kind = MKind.Ctor, File = rel, Line = line, IsStatic = ms.IsStatic, CsSig = CsSignature(ms) });
                            break;
                        }
                        case OperatorDeclarationSyntax or ConversionOperatorDeclarationSyntax:
                        {
                            var ms = (IMethodSymbol)sm.GetDeclaredSymbol(mem)!;
                            list.Add(new MemberX { Sym = ms, Method = ms, Kind = MKind.Operator, File = rel, Line = line, IsStatic = true, CsSig = CsSignature(ms) });
                            break;
                        }
                        case BaseTypeDeclarationSyntax:
                            break;
                        default:
                            Model.Warnings.Add($"unhandled member kind {mem.Kind()} in {t.Name} ({rel}:{line})");
                            break;
                    }
                }
            }
        }

        AssignNames(t, list);
        return list;
    }

    static string TypeShort(ITypeSymbol t) => t switch
    {
        IArrayTypeSymbol a => TypeShort(a.ElementType) + "s",
        INamedTypeSymbol { OriginalDefinition.SpecialType: SpecialType.System_Nullable_T } n => TypeShort(n.TypeArguments[0]),
        INamedTypeSymbol { IsGenericType: true } n => n.Name + string.Concat(n.TypeArguments.Select(TypeShort)),
        _ => t.Name
    };

    void AssignNames(TypeX t, List<MemberX> list)
    {
        foreach (var m in list)
        {
            m.RustName = m.Kind switch
            {
                MKind.Ctor => "new",
                MKind.Setter => "set_" + Snake(m.Sym.Name),
                MKind.Const => Snake(m.Sym.Name).ToUpperInvariant(),
                MKind.Indexer => "index",
                MKind.Operator => "op_" + Snake(((IMethodSymbol)m.Sym).Name.Replace("op_", "")),
                _ => m.Sym.Name == "Clone" ? "cs_clone" : Ident(Snake(m.Sym.Name))
            };
            if (m.Kind == MKind.StaticField) m.RustName = Ident(Snake(m.Sym.Name));
        }

        // Parameterless constructors of events and defaultable types are `Default`.
        foreach (var g in list.GroupBy(m => m.RustName).Where(g => g.Count() > 1))
        {
            var first = true;
            var used = new HashSet<string>();
            foreach (var m in g)
            {
                if (first) { first = false; used.Add(m.RustName); continue; }
                var ps = m.Method?.Parameters ?? [];
                var suffix = ps.Length == 0 ? "0" : string.Join("_", ps.Select(p => Snake(TypeShort(p.Type))));
                var n = m.RustName + "_" + suffix;
                var i = 2;
                while (used.Contains(n) || list.Any(o => o != m && o.RustName == n)) n = m.RustName + "_" + suffix + i++;
                used.Add(n);
                m.RustName = n;
            }
        }
    }

    #endregion

    #region emit

    public void EmitAll()
    {
        foreach (var t in Model.Types.Values) { FieldsOf(t); MembersOf(t); }

        foreach (var rel in Model.SelectedFiles)
        {
            var path = ModulePath(rel);
            _moduleOf[rel] = path;
            var sb = new StringBuilder();
            var todosBefore = TodoCount;
            EmitFile(rel, sb);
            _todoCount[path] = TodoCount - todosBefore;
            Write(path + ".rs", sb.ToString());
            _modules.Add((path, rel));
        }

        EmitEventsMod();
        EmitLogMod();
        EmitPrelude();
        EmitJsonGlue();
    }

    void Write(string relPath, string content)
    {
        var full = Path.Combine(_out, relPath);
        if (_dry) return;
        var glueFile = relPath is "events/mod.rs" or "log/mod.rs" or "prelude.rs" or "json_members.rs";
        if (File.Exists(full) && !_force && !(Glue && glueFile))
        {
            _skippedExisting.Add(relPath);
            return;
        }
        Directory.CreateDirectory(Path.GetDirectoryName(full)!);
        File.WriteAllText(full, content);
    }

    const string GenNote = "//! Skeleton generated by engine/csharp-gen from the C# source. Bodies are ported by hand;\n//! the generator never overwrites an existing file (see engine-conventions.md).\n";

    void EmitFile(string rel, StringBuilder sb)
    {
        sb.AppendLine($"//! Port of {rel}.");
        sb.AppendLine("//!");
        sb.Append(GenNote);
        sb.AppendLine();
        sb.AppendLine("#![allow(unused_variables, unused_imports, unused_mut, dead_code, non_snake_case, clippy::all)]");
        sb.AppendLine();
        sb.AppendLine("use crate::prelude::*;");
        sb.AppendLine();

        if (Model.EnumsByFile.TryGetValue(rel, out var enums))
            foreach (var e in enums)
                sb.AppendLine($"// C# enum `{e.Name}` is generated into enumerations.rs (tools/gen_enumerations.py).\n");

        // Types declared in this file, in source order.
        var tree = Model.Trees[rel];
        var sm = Model.Comp.GetSemanticModel(tree);
        foreach (var decl in tree.GetRoot().DescendantNodes().OfType<TypeDeclarationSyntax>())
        {
            var sym = (INamedTypeSymbol)sm.GetDeclaredSymbol(decl)!;
            if (!Model.Types.TryGetValue(sym.Name, out var t)) continue;
            var primary = t.Files[0] == rel && decl == t.Sym.DeclaringSyntaxReferences.Select(r => r.GetSyntax()).OfType<TypeDeclarationSyntax>().First(d => Model.Rel(d.SyntaxTree) == t.Files[0]);
            if (t.Kind == TKind.Game) primary = rel == "Game.cs";
            if (primary)
            {
                TypeCount++;
                EmitTypeDef(t, sb);
            }
            EmitImpl(t, rel, sb, primary);
        }
    }

    string TypeHeader(TypeX t)
    {
        var syn = t.Sym.DeclaringSyntaxReferences.Select(r => r.GetSyntax()).OfType<TypeDeclarationSyntax>().First();
        var head = OneLine($"{string.Join(" ", syn.Modifiers.Select(m => m.Text))} {syn.Keyword.Text} {syn.Identifier}{syn.TypeParameterList}{syn.ParameterList}{syn.BaseList}");
        var (f, l) = Model.Loc(syn);
        return $"/// C#: `{head}` ({f}:{l})";
    }

    string Generics(TypeX t) => t.Sym.TypeParameters.Length > 0 ? "<" + string.Join(", ", t.Sym.TypeParameters.Select(p => p.Name)) + ">" : "";

    string RustTypeName(TypeX t) => t.Name == "GameEvent" ? "GameEventBase" : t.Name;

    void EmitField(FieldX f, StringBuilder sb, string indent = "    ")
    {
        FieldCount++;
        sb.AppendLine($"{indent}/// `{DeclText(f.Sym!)}` ({f.File}:{f.Line})");
        if (f.Cell) sb.AppendLine($"{indent}/// Interior mutability: C# writes this member after the event is stored in game state.");
        var attrs = new List<string>();
        if (Pascal(f.Rust) != f.CsName) attrs.Add($"name = {Lit(f.CsName)}");
        if (f.Omit) attrs.Add("omit");
        if (attrs.Count > 0) sb.AppendLine($"{indent}#[canon({string.Join(", ", attrs)})]");
        sb.AppendLine($"{indent}pub {f.Rust}: {f.RustType},");
    }

    void EmitTypeDef(TypeX t, StringBuilder sb)
    {
        sb.AppendLine(TypeHeader(t));
        var name = RustTypeName(t);
        switch (t.Kind)
        {
            case TKind.Interface:
                EmitInterfaceTrait(t, sb);
                return;
            case TKind.Static:
                sb.AppendLine($"pub struct {name};");
                sb.AppendLine();
                return;
            case TKind.EventBase when t.Name != "GameEvent":
                sb.AppendLine($"/// Holds the static members of `{t.Name}`. Its fields are inlined into every derived event struct;");
                sb.AppendLine($"/// its instance members are the default methods of [`{TraitOf(t.Name)}`].");
                sb.AppendLine($"pub struct {name};");
                sb.AppendLine();
                EmitBaseTrait(t, sb);
                return;
        }

        if (t.Kind == TKind.EventBase)
        {
            sb.AppendLine("/// The fields of the C# base class `GameEvent`, embedded as `base` in every event struct.");
            sb.AppendLine("/// The polymorphic `GameEvent` itself is the enum in `events/mod.rs`.");
        }

        var fields = FieldsOf(t);
        var derives = new List<string>();
        if (t.Kind != TKind.Game) { derives.Add("Clone"); derives.Add("Debug"); }
        if (t.IsRecord) derives.Add("PartialEq");
        derives.Add("Canon");
        sb.AppendLine($"#[derive({string.Join(", ", derives)})]");
        var gen = Generics(t);
        var where = t.Sym.TypeParameters.Length > 0 ? "" : "";
        sb.AppendLine($"pub struct {name}{gen} {{");
        if (t.Kind == TKind.Game)
        {
            foreach (var g in fields.GroupBy(f => f.File))
            {
                sb.AppendLine($"    // ---- {g.Key} ----");
                sb.AppendLine();
                foreach (var f in g) EmitField(f, sb);
                sb.AppendLine();
            }
        }
        else
        {
            foreach (var f in fields) EmitField(f, sb);
        }

        if (t.Kind == TKind.Event)
        {
            foreach (var b in EventBaseChain(t).Where(b => b.Name != "GameEvent"))
            {
                sb.AppendLine($"    // ---- inherited from {b.Name} ----");
                foreach (var f in FieldsOf(b)) EmitField(f, sb);
            }
            sb.AppendLine("    /// The fields of `GameEvent` (Initiator, Time, Game, Player).");
            sb.AppendLine("    #[canon(flatten)]");
            sb.AppendLine("    pub base: GameEventBase,");
        }
        sb.AppendLine("}");
        sb.AppendLine();

        if (Defaultable(t) || t.Kind == TKind.Event)
        {
            var bounds = t.Sym.TypeParameters.Length > 0 ? "<" + string.Join(", ", t.Sym.TypeParameters.Select(p => p.Name + ": Clone + Eq + std::hash::Hash")) + ">" : "";
            sb.AppendLine($"impl{bounds} Default for {name}{gen} {{");
            sb.AppendLine("    /// The C# parameterless constructor with its field initializers.");
            sb.AppendLine("    fn default() -> Self {");
            sb.AppendLine($"        {name} {{");
            foreach (var f in fields) sb.AppendLine($"            {f.Rust}: {Init(f) ?? Todo(f.File, f.Line, "initializer")},");
            if (t.Kind == TKind.Event)
            {
                foreach (var b in EventBaseChain(t).Where(b => b.Name != "GameEvent"))
                    foreach (var f in FieldsOf(b)) sb.AppendLine($"            {f.Rust}: {Init(f) ?? Todo(f.File, f.Line, "initializer")},");
                sb.AppendLine("            base: Default::default(),");
            }
            sb.AppendLine("        }");
            sb.AppendLine("    }");
            sb.AppendLine("}");
            sb.AppendLine();
        }
    }

    string Todo(string file, int line, string what = "")
    {
        TodoCount++;
        return $"todo!(\"port: {file}:{line}{(what != "" ? " " + what : "")}\")";
    }

    // Return type of a member.
    string RetOf(TypeX owner, MemberX m)
    {
        switch (m.Kind)
        {
            case MKind.Ctor: return "Self";
            case MKind.Setter: return "";
            case MKind.Getter or MKind.Indexer:
            {
                var p = (IPropertySymbol)m.Sym;
                while (p.OverriddenProperty != null) p = p.OverriddenProperty;
                return EventProp(owner, p.Type, RT(p.Type, Pos.Ret));
            }
            case MKind.Const or MKind.StaticField:
                return m.Sym switch { IFieldSymbol f => RT(f.Type, Pos.Ret), IPropertySymbol p => RT(p.Type, Pos.Ret), _ => "()" };
            default:
            {
                var ms = (IMethodSymbol)m.Sym;
                var self = ms;
                while (ms.OverriddenMethod != null) ms = ms.OverriddenMethod;
                if (ms.ReturnsVoid) return "";
                var rt = EventProp(owner, ms.ReturnType, RT(ms.ReturnType, Pos.Ret));
                if (ms.ReturnType.IsReferenceType && !rt.StartsWith("Option<") && ReturnsNull(self)) rt = $"Option<{rt}>";
                return rt;
            }
        }
    }

    // `return null` in a method whose return type is not annotated nullable.
    static bool ReturnsNull(IMethodSymbol m)
    {
        var syn = m.DeclaringSyntaxReferences.FirstOrDefault()?.GetSyntax();
        if (syn == null) return false;
        return syn.DescendantNodes().OfType<ReturnStatementSyntax>().Any(r => r.Expression is LiteralExpressionSyntax { RawKind: (int)Microsoft.CodeAnalysis.CSharp.SyntaxKind.NullLiteralExpression } &&
                   !r.Ancestors().TakeWhile(a => a != syn).Any(a => a is AnonymousFunctionExpressionSyntax or LocalFunctionStatementSyntax)) ||
               syn is MethodDeclarationSyntax { ExpressionBody.Expression: LiteralExpressionSyntax { RawKind: (int)Microsoft.CodeAnalysis.CSharp.SyntaxKind.NullLiteralExpression } };
    }

    // Event members that resolve ids return Option even where C# says non-null (lookups can fail).
    string EventProp(TypeX owner, ITypeSymbol t, string rt)
    {
        if (owner.Kind is not (TKind.Event or TKind.EventBase or TKind.Interface)) return rt;
        if (rt.StartsWith("Option<")) return rt;
        if (Handles.Contains(rt) || rt == "Hero" || rt.StartsWith("Rc<") || rt == "GameEvent" || rt == "PlayerIndex") return $"Option<{rt}>";
        return rt;
    }

    Sig SigOf(TypeX owner, MemberX m)
    {
        var ret = RetOf(owner, m);
        var f = FlagsFor(m.Method);
        if (m.Kind == MKind.Ctor)
        {
            var s = BuildSig(owner, m.Method!, "Self", true, f, owned: owner.IsRecord && m.Sym.DeclaringSyntaxReferences.FirstOrDefault()?.GetSyntax() is Microsoft.CodeAnalysis.CSharp.Syntax.RecordDeclarationSyntax);
            return s;
        }
        if (m.Kind is MKind.Const or MKind.StaticField) return new Sig("", [], ret, "");
        if (m.Kind == MKind.Setter)
        {
            var p = (IPropertySymbol)m.Sym;
            var s = BuildSig(owner, m.Method!, "", m.IsStatic, f);
            s.Params[^1] = "value: " + EventProp(owner, p.Type, RT(p.Type, Pos.Param));
            return s;
        }
        if (m.Kind == MKind.Indexer)
            return BuildSig(owner, m.Method!, ret, false, f);
        return BuildSig(owner, m.Method!, ret, m.IsStatic, f);
    }

    void EmitMember(TypeX owner, MemberX m, StringBuilder sb, string indent, bool inTrait, bool pub = true)
    {
        var sig = SigOf(owner, m);
        sb.AppendLine($"{indent}/// C#: `{m.CsSig}` ({m.File}:{m.Line})");
        switch (m.Kind)
        {
            case MKind.Const:
            {
                var f = (IFieldSymbol)m.Sym;
                var v = ConstValue(f);
                if (v != null)
                {
                    sb.AppendLine($"{indent}pub const {m.RustName}: {sig.Ret} = {v};");
                    sb.AppendLine();
                    return;
                }
                sb.AppendLine($"{indent}pub fn {m.RustName.ToLowerInvariant()}() -> {sig.Ret} {{");
                sb.AppendLine($"{indent}    {Todo(m.File, m.Line)}");
                sb.AppendLine($"{indent}}}");
                sb.AppendLine();
                MethodCount++;
                return;
            }
            case MKind.StaticField:
                sb.AppendLine($"{indent}/// A C# static field: decide per site whether it is a constant table (a fn or a `static`) or process-wide state (hazards.md section 6.2).");
                sb.AppendLine($"{indent}pub fn {m.RustName}() -> {sig.Ret} {{");
                sb.AppendLine($"{indent}    {Todo(m.File, m.Line)}");
                sb.AppendLine($"{indent}}}");
                sb.AppendLine();
                MethodCount++;
                return;
        }

        MethodCount++;
        var body = m.Body ?? GeneratedBody(owner, m) ?? Todo(m.File, m.Line);
        var abstractMember = inTrait && (m.Method?.IsAbstract ?? false);
        if (abstractMember)
        {
            sb.AppendLine($"{indent}{FormatSig(m.RustName, sig, false)};");
            sb.AppendLine();
            return;
        }
        sb.AppendLine($"{indent}{FormatSig(m.RustName, sig, pub && !inTrait)} {{");
        foreach (var line in body.Split('\n')) sb.AppendLine($"{indent}    {line}");
        sb.AppendLine($"{indent}}}");
        sb.AppendLine();
    }

    string? ConstValue(IFieldSymbol f)
    {
        if (!f.HasConstantValue) return null;
        return f.ConstantValue switch
        {
            int i => i.ToString(),
            bool b => b ? "true" : "false",
            string s => null,
            char c => $"'{c}'",
            float x => x.ToString("R", System.Globalization.CultureInfo.InvariantCulture) + (x % 1 == 0 ? ".0" : ""),
            double x => x.ToString("R", System.Globalization.CultureInfo.InvariantCulture) + (x % 1 == 0 ? ".0" : ""),
            _ => null
        };
    }

    // Constructors that only chain to GameEvent's constructor, and positional records.
    string? GeneratedBody(TypeX owner, MemberX m)
    {
        if (m.Kind != MKind.Ctor) return null;
        var syn = m.Sym.DeclaringSyntaxReferences.FirstOrDefault()?.GetSyntax();
        var ctor = (IMethodSymbol)m.Sym;
        if (owner.IsRecord && syn is RecordDeclarationSyntax)
        {
            var fields = FieldsOf(owner);
            return $"{owner.Name} {{ {string.Join(", ", fields.Select(f => f.Rust == Ident(Snake(f.CsName)) && ctor.Parameters.Any(p => Ident(Snake(p.Name)) == f.Rust) ? f.Rust : f.Rust + ": " + Todo(m.File, m.Line)))} }}";
        }
        if (syn is ConstructorDeclarationSyntax cd && owner.Kind == TKind.Event && (cd.Body?.Statements.Count ?? 0) == 0 && cd.ExpressionBody == null)
        {
            var init = cd.Initializer;
            if (init == null && ctor.Parameters.Length == 0) return "Self::default()";
            if (init != null && init.ThisOrBaseKeyword.Text == "base")
            {
                var args = init.ArgumentList.Arguments.Select(a => a.ToString()).ToList();
                var ps = ctor.Parameters.Select(p => p.Name).ToList();
                if (args.Count == 2 && ps.Count == 2 && args[0] == ps[0] && args[1] == ps[1])
                    return $"let e = Self::default();\ne.initialize({Ident(Snake(ps[0]))}, {Ident(Snake(ps[1]))});\ne";
                if (args.Count == 1 && ps.Count == 1 && args[0] == ps[0])
                    return $"let e = Self::default();\ne.initialize({Ident(Snake(ps[0]))}, Faction::None);\ne";
                if (args.Count == 0 && ps.Count == 0) return "Self::default()";
            }
        }
        return null;
    }

    void EmitImpl(TypeX t, string rel, StringBuilder sb, bool primary)
    {
        var members = MembersOf(t).Where(m => m.File == rel).ToList();
        if (t.Kind == TKind.Interface) return; // emitted as trait
        if (t.Kind == TKind.EventBase && t.Name != "GameEvent")
        {
            var st = members.Where(m => m.IsStatic && m.Kind != MKind.Ctor).ToList();
            if (st.Count > 0)
            {
                sb.AppendLine($"impl {t.Name} {{");
                foreach (var m in st) EmitMember(t, m, sb, "    ", false);
                sb.AppendLine("}");
                sb.AppendLine();
            }
            return;
        }

        var name = RustTypeName(t);
        var gen = Generics(t);
        var bounds = t.Sym.TypeParameters.Length > 0 ? "<" + string.Join(", ", t.Sym.TypeParameters.Select(p => p.Name + ": Clone + Eq + std::hash::Hash")) + ">" : "";

        if (t.Kind == TKind.EventBase) // GameEvent
        {
            EmitBaseTrait(t, sb);
            var statics = members.Where(m => m.IsStatic && m.Kind != MKind.Ctor).ToList();
            if (statics.Count > 0)
            {
                sb.AppendLine("/// The static members of the C# `GameEvent` class.");
                sb.AppendLine("impl GameEvent {");
                foreach (var m in statics) EmitMember(t, m, sb, "    ", false);
                sb.AppendLine("}");
                sb.AppendLine();
            }
            return;
        }

        var emitted = members.Where(m => !(t.Kind == TKind.Event && m.Kind == MKind.Ctor && m.Method!.Parameters.Length == 0 && GeneratedBody(t, m) == "Self::default()")).ToList();
        if (emitted.Count > 0 || primary)
        {
            sb.AppendLine($"impl{bounds} {name}{gen} {{");
            if (primary && t.Kind is TKind.Model && t.Sym.InstanceConstructors.All(c => c.IsImplicitlyDeclared) && !t.IsRecord && !t.Sym.IsStatic)
            {
                sb.AppendLine("    /// C#: implicit parameterless constructor.");
                sb.AppendLine("    pub fn new() -> Self {");
                sb.AppendLine("        " + (Defaultable(t) ? "Self::default()" : Todo(t.Files[0], Model.Loc(t.Sym).line, "field initializers")));
                sb.AppendLine("    }");
                sb.AppendLine();
                MethodCount++;
            }
            foreach (var m in emitted) EmitMember(t, m, sb, "    ", false);
            sb.AppendLine("}");
            sb.AppendLine();
        }

        if (t.Kind == TKind.Event && primary) EmitEventTraitImpls(t, sb);
        if (t.Kind == TKind.Game && rel == "Game.cs") EmitGameExtras(sb);
        if (t.Kind == TKind.Player && primary)
            sb.AppendLine(@"/// A C# `Player` reference: the index of the player in `Game.Players`. Players are only ever
/// appended, so the index is stable, and index equality is C# reference equality.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Canon)]
pub struct PlayerIndex(pub usize);
");
    }

    #endregion

    #region event traits

    List<MemberX> InstanceMembers(TypeX t) => MembersOf(t).Where(m => !m.IsStatic && m.Kind != MKind.Ctor).ToList();

    void EmitBaseTrait(TypeX b, StringBuilder sb)
    {
        var trait = TraitOf(b.Name);
        var super = b.Sym.BaseType!.Name == "Object" ? "" : ": " + TraitOf(b.Sym.BaseType!.Name);
        sb.AppendLine($"/// The instance members of the C# class `{b.Name}`. Every event deriving from it implements this trait");
        sb.AppendLine("/// (generated in its module); overridden virtual members are delegated to the event's inherent method.");
        sb.AppendLine($"pub trait {trait}{super} {{");
        if (b.Name == "GameEvent")
        {
            sb.AppendLine("    /// The `GameEvent` fields of this event.");
            sb.AppendLine("    fn base(&self) -> &GameEventBase;");
            sb.AppendLine();
            sb.AppendLine("    /// `GetType()`");
            sb.AppendLine("    fn event_type(&self) -> GameEventType;");
            sb.AppendLine();
            sb.AppendLine("    /// Upcast to the polymorphic event (shares the object).");
            sb.AppendLine("    fn into_event(self: Rc<Self>) -> GameEvent;");
            sb.AppendLine();
            sb.AppendLine("    /// `Initiator`");
            sb.AppendLine("    fn initiator(&self) -> Faction {");
            sb.AppendLine("        self.base().initiator.get()");
            sb.AppendLine("    }");
            sb.AppendLine();
            sb.AppendLine("    /// `Time`");
            sb.AppendLine("    fn time(&self) -> DateTimeOffset {");
            sb.AppendLine("        self.base().time" + (FieldsOf(b).First(f => f.CsName == "Time").Cell ? ".get()" : ""));
            sb.AppendLine("    }");
            sb.AppendLine();
            sb.AppendLine("    /// `Player`, dereferenced: NullReferenceException when it was never set (hazards.md 1.8).");
            sb.AppendLine("    fn player(&self) -> PlayerIndex {");
            sb.AppendLine("        self.base().player.get().nre()");
            sb.AppendLine("    }");
            sb.AppendLine();
            sb.AppendLine("    /// `Player`, possibly null.");
            sb.AppendLine("    fn player_opt(&self) -> Option<PlayerIndex> {");
            sb.AppendLine("        self.base().player.get()");
            sb.AppendLine("    }");
            sb.AppendLine();
        }
        else
        {
            foreach (var f in FieldsOf(b))
            {
                sb.AppendLine($"    /// The inherited field `{f.CsName}` (`self.{f.Rust}` on the event struct).");
                sb.AppendLine($"    fn field_{f.Rust.TrimEnd('_')}(&self) -> &{f.RustType};");
                sb.AppendLine();
            }
        }
        foreach (var m in InstanceMembers(b)) EmitMember(b, m, sb, "    ", true);
        sb.AppendLine("}");
        sb.AppendLine();
    }

    void EmitEventTraitImpls(TypeX t, StringBuilder sb)
    {
        var own = InstanceMembers(t);
        foreach (var b in EventBaseChain(t).Reverse())
        {
            sb.AppendLine($"impl {TraitOf(b.Name)} for {t.Name} {{");
            if (b.Name == "GameEvent")
            {
                sb.AppendLine("    fn base(&self) -> &GameEventBase {\n        &self.base\n    }\n");
                sb.AppendLine($"    fn event_type(&self) -> GameEventType {{\n        GameEventType::{t.Name}\n    }}\n");
                sb.AppendLine($"    fn into_event(self: Rc<Self>) -> GameEvent {{\n        GameEvent::{t.Name}(self)\n    }}\n");
            }
            else
            {
                foreach (var f in FieldsOf(b))
                    sb.AppendLine($"    fn field_{f.Rust.TrimEnd('_')}(&self) -> &{f.RustType} {{\n        &self.{f.Rust}\n    }}\n");
            }
            // Overrides of this base's virtual/abstract members.
            foreach (var bm in InstanceMembers(b))
            {
                var bsig = SigOf(b, bm);
                var over = own.FirstOrDefault(m => m.Method != null && Overrides(m.Method, bm.Method!));
                if (over == null)
                {
                    if (bm.Method!.IsAbstract) Model.Warnings.Add($"{t.Name} does not override abstract {bm.Sym.Name}");
                    continue;
                }
                sb.AppendLine($"    {FormatSig(bm.RustName, bsig, false)} {{");
                sb.AppendLine($"        {t.Name}::{over.RustName}({CallArgs(bsig, SigOf(t, over))})");
                sb.AppendLine("    }");
                sb.AppendLine();
            }
            sb.AppendLine("}");
            sb.AppendLine();
        }

        foreach (var i in t.Sym.AllInterfaces.Where(i => Model.Types.ContainsKey(i.Name)))
        {
            var it = Model.Types[i.Name];
            sb.AppendLine($"impl crate::events::{ModulePath(it.Files[0]).Split('/')[1]}::{i.Name} for {t.Name} {{");
            foreach (var im in MembersOf(it))
            {
                var isig = SigOf(it, im);
                sb.AppendLine($"    {FormatSig(im.RustName, isig, false)} {{");
                sb.AppendLine($"        {InterfaceCall(t, it, im, isig)}");
                sb.AppendLine("    }");
                sb.AppendLine();
            }
            sb.AppendLine("}");
            sb.AppendLine();
        }
    }

    static bool Overrides(IMethodSymbol m, IMethodSymbol baseM)
    {
        for (var o = m.OverriddenMethod; o != null; o = o.OverriddenMethod)
            if (SymbolEqualityComparer.Default.Equals(o.OriginalDefinition, baseM.OriginalDefinition)) return true;
        if (m.AssociatedSymbol is IPropertySymbol p && baseM.AssociatedSymbol is IPropertySymbol bp)
            for (var o = p.OverriddenProperty; o != null; o = o.OverriddenProperty)
                if (SymbolEqualityComparer.Default.Equals(o.OriginalDefinition, bp.OriginalDefinition)) return m.MethodKind == baseM.MethodKind;
        return false;
    }

    // Arguments to pass from a `caller` signature to a `callee` signature (by parameter name).
    static string CallArgs(Sig caller, Sig callee)
    {
        var args = new List<string> { "self" };
        var callerNames = caller.Params.Select(p => p.Split(':')[0].Trim()).ToList();
        foreach (var p in callee.Params)
        {
            var n = p.Split(':')[0].Trim();
            args.Add(callerNames.Contains(n) ? n : "todo!(\"argument " + n + "\")");
        }
        return string.Join(", ", args);
    }

    string InterfaceCall(TypeX t, TypeX iface, MemberX im, Sig isig)
    {
        var imSym = im.Kind is MKind.Getter or MKind.Setter ? (ISymbol)im.Sym : im.Method!;
        var impl = t.Sym.FindImplementationForInterfaceMember(imSym);
        if (impl == null) { Model.Warnings.Add($"{t.Name}: no implementation of {iface.Name}.{im.Sym.Name}"); return Todo(im.File, im.Line); }
        var declaring = Model.Types.GetValueOrDefault(impl.ContainingType.Name);
        if (declaring == null) return Todo(im.File, im.Line);

        // Auto-property -> field.
        if (impl is IPropertySymbol ip && !MembersOf(declaring).Any(m => SymbolEqualityComparer.Default.Equals(m.Sym, ip) && m.Kind == im.Kind))
        {
            if (declaring.Name == "GameEvent")
            {
                return im.Sym.Name switch
                {
                    "Initiator" => "GameEventMethods::initiator(self)",
                    "Player" => "GameEventMethods::player_opt(self)",
                    _ => Todo(im.File, im.Line)
                };
            }
            var f = FieldsOf(declaring).FirstOrDefault(x => x.CsName == ip.Name);
            if (f == null) return Todo(im.File, im.Line);
            var access = $"self.{f.Rust}";
            if (f.Cell) access += IsCopy(f.RustType[5..^1]) ? ".get()" : ".borrow().clone()";
            else if (!IsCopy(f.RustType)) access += ".clone()";
            var ret = isig.Ret;
            if (ret.StartsWith("Option<") && !f.RustType.StartsWith("Option<") && !f.RustType.Contains("Option<")) access = $"Some({access})";
            return access;
        }

        var implMember = MembersOf(declaring).FirstOrDefault(m =>
            impl is IPropertySymbol ? SymbolEqualityComparer.Default.Equals(m.Sym, impl) && m.Kind == im.Kind : SymbolEqualityComparer.Default.Equals(m.Sym, impl));
        if (implMember == null) { Model.Warnings.Add($"{t.Name}: implementation member not found for {iface.Name}.{im.Sym.Name}"); return Todo(im.File, im.Line); }
        var callee = SigOf(declaring, implMember);
        var target = declaring == t ? $"{t.Name}::{implMember.RustName}" : $"<Self as {TraitOf(declaring.Name)}>::{implMember.RustName}";
        var call = $"{target}({CallArgs(isig, callee)})";
        if (isig.Ret.StartsWith("Option<") && !callee.Ret.StartsWith("Option<")) call = $"Some({call})";
        return call;
    }

    void EmitInterfaceTrait(TypeX t, StringBuilder sb)
    {
        sb.AppendLine($"/// Implemented by the events that implement the C# interface. Not in the prelude, to avoid");
        sb.AppendLine("/// ambiguity with the base-class traits; `GameEvent` has a dispatching method for each member.");
        sb.AppendLine($"pub trait {t.Name} {{");
        foreach (var m in MembersOf(t))
        {
            var sig = SigOf(t, m);
            sb.AppendLine($"    /// C#: `{m.CsSig}` ({m.File}:{m.Line})");
            sb.AppendLine($"    {FormatSig(m.RustName, sig, false)};");
            sb.AppendLine();
            MethodCount++;
        }
        sb.AppendLine("}");
        sb.AppendLine();
    }

    void EmitGameExtras(StringBuilder sb)
    {
        sb.AppendLine(@"/// C# objects that keep a reference to their owning `Game` (Player, PlayerSequence, events)
/// hold this marker instead; their methods take the game as a parameter.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Canon)]
pub struct GameRef;

impl Game {
    /// `Players[p]`
    pub fn player(&self, p: PlayerIndex) -> &Player {
        &self.players[p.0]
    }

    /// `Players[p]`, for mutation.
    pub fn player_mut(&mut self, p: PlayerIndex) -> &mut Player {
        &mut self.players[p.0]
    }

    /// The index of a player object (C# reference identity), if it is seated in this game.
    pub fn index_of(&self, p: &Player) -> Option<PlayerIndex> {
        self.players.iter().position(|x| std::ptr::eq(x, p)).map(PlayerIndex)
    }
}

/// What `Map` needs to know about the game (see map.rs).
impl crate::map::MapGame for Game {
    fn version(&self) -> i32 {
        self.version
    }

    fn sector_in_storm(&self) -> i32 {
        self.sector_in_storm
    }

    fn history_count(&self) -> i32 {
        self.history.len() as i32
    }

    fn applicable(&self, rule: Rule) -> bool {
        Game::applicable(self, rule)
    }

    fn forces_battalion_factions(&self) -> Vec<(Location, Vec<Faction>)> {
        todo!(""port: Map.cs DetermineForceObstacles reads Game.Forces()"")
    }

    fn player_ally(&self, f: Faction) -> Option<Faction> {
        todo!(""port: Map.cs reads Game.GetPlayer(f)?.Ally"")
    }

    fn nr_of_occupants_excluding_faction(&self, l: Location, f: Faction) -> i32 {
        Game::nr_of_occupants_excluding_faction(self, l, f)
    }

    fn current_blocked_territories(&self) -> Vec<Territory> {
        self.current_blocked_territories.clone()
    }
}
");
        TodoCount += 2;
    }

    #endregion
}
