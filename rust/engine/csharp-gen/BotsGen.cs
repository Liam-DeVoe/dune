// Generates the skeleton of rust/bots from treachery-online/Bots (phase 2, see rust/docs/bots-plan.md).
//
//   dotnet run -- --bots [--force]
//
// One Rust module per C# file. ClassicBot is one struct (declared in bot.rs) with an `impl ClassicBot` block in
// every module, like Game. Every member gets a `todo!("port: File.cs:line")` body. Type mapping is the engine's (RT).
//
// Signatures:
// - Every instance method takes `game`. Bots don't keep the game; the caller passes it on each call.
// - `&mut self` when the method, or a ClassicBot method it calls or names as a method group, writes bot state
//   (a field, or anything reached through one). Draws from `_random` don't count: SharedRandom is interior-mutable.
// - `game: &mut Game` when the method reaches an engine member whose Rust signature takes `&mut Game`
//   (only Battle.DetermineBattleOutcome today, see bots-plan.md). Everything else gets `&Game`.
// - `Do` takes `&mut self` and `&mut Game`, because it runs whatever method it is given.

using System.Text;
using Microsoft.CodeAnalysis;
using Microsoft.CodeAnalysis.CSharp;
using Microsoft.CodeAnalysis.CSharp.Syntax;
using Microsoft.CodeAnalysis.Operations;

namespace CsharpGen;

public sealed partial class Emitter
{
    // Engine members that take `&mut Game` in Rust and are called by bots.
    internal static readonly HashSet<string> MutGameEngineMembers = ["Battle.DetermineBattleOutcome"];

    public static void EmitBots(Model model, string botsDir, string outDir, bool force)
    {
        var files = Directory.GetFiles(botsDir, "*.cs", SearchOption.AllDirectories)
            .Where(f => !f.Contains("/bin/") && !f.Contains("/obj/") && Path.GetFileName(f) != "GlobalUsings.cs")
            .OrderBy(f => f, StringComparer.Ordinal).ToList();
        var trees = files.Select(f => CSharpSyntaxTree.ParseText(File.ReadAllText(f), new CSharpParseOptions(LanguageVersion.Latest), f)).ToList();
        // Bots/GlobalUsings.cs plus the implicit usings of the Bots project that Shared's GlobalUsings.cs lacks.
        var usings = CSharpSyntaxTree.ParseText("global using Treachery.Shared;\nglobal using System.IO;\nglobal using System.Threading;\nglobal using System.Threading.Tasks;\n",
            new CSharpParseOptions(LanguageVersion.Latest), "BotsGlobalUsings.cs");
        model.Comp = model.Comp.AddSyntaxTrees(trees.Append(usings));
        model.GameEventSym = model.Comp.GetTypeByMetadataName("Treachery.Shared.GameEvent")!;
        model.GameSym = model.Comp.GetTypeByMetadataName("Treachery.Shared.Game")!;
        model.PlayerSym = model.Comp.GetTypeByMetadataName("Treachery.Shared.Player")!;

        var errors = trees.SelectMany(t => model.Comp.GetSemanticModel(t).GetDiagnostics().Where(d => d.Severity == DiagnosticSeverity.Error)).ToList();
        foreach (var e in errors.Take(10)) Console.Error.WriteLine("bots compile error: " + e);
        if (errors.Count > 0) Console.Error.WriteLine($"{errors.Count} Roslyn errors in Bots (types may be partially unresolved)");

        var gen = new Emitter(model, outDir, force, false);
        new BotsEmitter(gen, model, trees, botsDir, outDir, force).Run();
        foreach (var w in model.Warnings.Distinct().Where(w => w.Contains("unmapped"))) Console.Error.WriteLine("warning: " + w);
    }

    public string BotRT(ITypeSymbol t, Pos pos) => RT(t, pos);
    public static string BotCsSig(Emitter e, ISymbol s) => e.CsSignature(s);
    public static bool BotReturnsNull(IMethodSymbol m) => ReturnsNull(m);
    public static string BotOneLine(string s) => OneLine(s);
    public static string? BotConst(Emitter e, IFieldSymbol f) => e.ConstValue(f);
}

sealed class BotsEmitter(Emitter gen, Model model, List<SyntaxTree> trees, string botsDir, string outDir, bool force)
{
    INamedTypeSymbol _bot = null!;
    readonly HashSet<IMethodSymbol> _mutSelf = new(SymbolEqualityComparer.Default);
    readonly HashSet<IMethodSymbol> _mutGame = new(SymbolEqualityComparer.Default);
    readonly Dictionary<IMethodSymbol, HashSet<IMethodSymbol>> _calls = new(SymbolEqualityComparer.Default);
    readonly Dictionary<ISymbol, string> _names = new(SymbolEqualityComparer.Default);
    readonly HashSet<string> _botTypes = [];
    int _todos, _methods;

    string Rel(SyntaxTree t) => Path.GetRelativePath(botsDir, t.FilePath).Replace('\\', '/');
    string FileName(SyntaxTree t) => Path.GetFileName(t.FilePath);
    static string Module(SyntaxTree t) => Emitter.Snake(Path.GetFileNameWithoutExtension(t.FilePath));
    string Todo(SyntaxTree t, int line) { _todos++; return $"todo!(\"port: {FileName(t)}:{line}\")"; }
    static int Line(SyntaxNode n) => n.GetLocation().GetLineSpan().StartLinePosition.Line + 1;

    public void Run()
    {
        _bot = model.Comp.GetTypeByMetadataName("Treachery.Bots.ClassicBot")!;
        // Bot-defined types must map to themselves.
        foreach (var t in trees)
        {
            var sm = model.Comp.GetSemanticModel(t);
            foreach (var d in t.GetRoot().DescendantNodes().OfType<BaseTypeDeclarationSyntax>())
            {
                var s = (INamedTypeSymbol)sm.GetDeclaredSymbol(d)!;
                if (SymbolEqualityComparer.Default.Equals(s, _bot)) continue;
                _botTypes.Add(s.Name);
                if (s.TypeKind != TypeKind.Enum && !model.Types.ContainsKey(s.Name))
                    model.Types[s.Name] = new TypeX { Sym = s, Name = s.Name, Kind = TKind.Model };
                if (s.TypeKind == TypeKind.Enum) Emitter.EnumNames.Add(s.Name);
            }
        }
        Analyze();
        AssignNames();

        Directory.CreateDirectory(outDir);
        var modules = new List<string>();
        var stateFields = new StringBuilder();
        foreach (var t in trees)
        {
            var root = t.GetRoot();
            var sm = model.Comp.GetSemanticModel(t);
            foreach (var cls in root.DescendantNodes().OfType<ClassDeclarationSyntax>().Where(c => c.Identifier.Text == "ClassicBot"))
                foreach (var member in cls.Members)
                    if (StateField(member, sm) is { } f) stateFields.Append(f);
        }

        foreach (var t in trees)
        {
            var sb = new StringBuilder();
            var mod = Module(t);
            modules.Add(mod);
            sb.AppendLine($"//! Port of `Bots/Treachery.Bots/{Rel(t)}`. Skeleton generated by `engine/csharp-gen --bots`; porters own this file now.");
            sb.AppendLine();
            sb.AppendLine("use crate::prelude::*;");
            sb.AppendLine();
            var sm = model.Comp.GetSemanticModel(t);
            var root = t.GetRoot();
            foreach (var type in root.ChildNodes().OfType<BaseNamespaceDeclarationSyntax>().SelectMany(n => n.Members).Concat(root.ChildNodes().OfType<MemberDeclarationSyntax>()).OfType<BaseTypeDeclarationSyntax>())
            {
                if (type.Identifier.Text == "ClassicBot") EmitBotPartial(t, (ClassDeclarationSyntax)type, sm, sb, mod == "bot" ? stateFields.ToString() : null);
                else EmitType(t, type, sm, sb);
            }
            Write(mod + ".rs", sb.ToString());
        }

        var lib = new StringBuilder();
        lib.AppendLine("//! Port of `treachery-online/Bots` (phase 2). See rust/docs/bots-plan.md.");
        lib.AppendLine("//!");
        lib.AppendLine("//! The module list and the skeleton were generated by `engine/csharp-gen --bots`.");
        lib.AppendLine();
        lib.AppendLine("// The skeleton's todo!() bodies leave parameters unused; porters remove these as files are finished.");
        lib.AppendLine("#![allow(unused_variables, dead_code, unused_imports, clippy::too_many_arguments)]");
        lib.AppendLine();
        lib.AppendLine("pub mod i_bot;");
        lib.AppendLine("pub mod prelude;");
        foreach (var m in modules) lib.AppendLine($"pub mod {m};");
        lib.AppendLine();
        lib.AppendLine("pub use bot::ClassicBot;");
        lib.AppendLine("pub use bot_parameters::BotParameters;");
        lib.AppendLine("pub use i_bot::IBot;");
        Write("lib.rs", lib.ToString());

        var prelude = new StringBuilder();
        prelude.AppendLine("//! Everything a bot module needs: `use crate::prelude::*;`. Support module (no C# counterpart).");
        prelude.AppendLine();
        prelude.AppendLine("pub use treachery_engine::prelude::*;");
        prelude.AppendLine("pub use treachery_engine::events::GameEventType;");
        prelude.AppendLine("pub use treachery_engine::game::Game;");
        prelude.AppendLine("pub use treachery_engine::player::{Player, PlayerIndex};");
        prelude.AppendLine("pub use treachery_engine::object_counter::ObjectCounter;");
        prelude.AppendLine();
        prelude.AppendLine("pub use crate::bot::ClassicBot;");
        prelude.AppendLine("pub use crate::bot_parameters::BotParameters;");
        foreach (var t in trees)
        {
            var sm = model.Comp.GetSemanticModel(t);
            foreach (var d in t.GetRoot().DescendantNodes().OfType<BaseTypeDeclarationSyntax>())
            {
                var s = (INamedTypeSymbol)sm.GetDeclaredSymbol(d)!;
                if (s.Name is "ClassicBot" or "BotParameters") continue;
                prelude.AppendLine($"pub use crate::{Module(t)}::{s.Name};");
            }
        }
        Write("prelude.rs", prelude.ToString());

        Write("i_bot.rs", IBotModule());
        Console.WriteLine($"bots skeleton: {modules.Count} modules, {_methods} members, {_todos} todo!() bodies, {_mutSelf.Count} methods with &mut self, {_mutGame.Count} with &mut Game");
        Console.WriteLine("  &mut Game: " + string.Join(", ", _mutGame.Select(m => m.Name).OrderBy(x => x)));
    }

    void Write(string rel, string content)
    {
        var path = Path.Combine(outDir, rel);
        if (File.Exists(path) && !force)
        {
            Console.WriteLine($"exists, not overwritten: {path}");
            return;
        }
        File.WriteAllText(path, content);
    }

    #region analysis

    IEnumerable<IMethodSymbol> BotMethods() => _bot.GetMembers().SelectMany(m => m switch
    {
        IMethodSymbol ms when ms.MethodKind is MethodKind.Ordinary or MethodKind.Constructor => [ms],
        IPropertySymbol p => new[] { p.GetMethod, p.SetMethod }.Where(x => x != null).Cast<IMethodSymbol>(),
        _ => []
    });

    bool IsBotMember(ISymbol? s) => s != null && !s.IsStatic && SymbolEqualityComparer.Default.Equals(s.ContainingType?.OriginalDefinition, _bot);

    // Bot state: instance fields and auto-properties of ClassicBot, except Game and Player (engine objects).
    bool IsBotState(ISymbol? s) => IsBotMember(s) && s is IFieldSymbol or IPropertySymbol && s.Name is not ("Game" or "Player" or "Param") && !IsComputed(s);

    static bool IsComputed(ISymbol s) => s is IPropertySymbol p && p.DeclaringSyntaxReferences.FirstOrDefault()?.GetSyntax() is PropertyDeclarationSyntax pd &&
        (pd.ExpressionBody != null || pd.AccessorList?.Accessors.Any(a => a.Body != null || a.ExpressionBody != null) == true);

    // The member chain of an assignment target or receiver: is it rooted in bot state?
    bool RootedInBotState(IOperation? op)
    {
        for (var o = op; o != null;)
        {
            switch (o)
            {
                case IMemberReferenceOperation mr:
                    if (IsBotState(mr.Member)) return mr.Member.Name != "_random";
                    o = mr.Instance;
                    break;
                case IArrayElementReferenceOperation ae: o = ae.ArrayReference; break;
                case IConversionOperation c: o = c.Operand; break;
                default: return false;
            }
        }
        return false;
    }

    static readonly HashSet<string> Mutators = ["Add", "AddRange", "Remove", "RemoveAll", "RemoveAt", "Clear", "Insert", "TryAdd", "Push", "Pop", "Enqueue", "Dequeue", "Sort", "Reverse"];

    void Analyze()
    {
        foreach (var m in BotMethods())
        {
            var calls = _calls[m] = new HashSet<IMethodSymbol>(SymbolEqualityComparer.Default);
            var syn = m.DeclaringSyntaxReferences.FirstOrDefault()?.GetSyntax();
            if (syn == null) continue;
            var sm = model.Comp.GetSemanticModel(syn.SyntaxTree);
            var bodies = syn switch
            {
                MethodDeclarationSyntax md => new SyntaxNode?[] { md.Body, md.ExpressionBody },
                PropertyDeclarationSyntax pd => [pd.ExpressionBody, pd.AccessorList],
                AccessorDeclarationSyntax ad => [ad.Body, ad.ExpressionBody],
                _ => []
            };
            foreach (var body in bodies.Where(b => b != null))
                foreach (var node in body!.DescendantNodesAndSelf())
                {
                    var op = sm.GetOperation(node);
                    switch (op)
                    {
                        case IAssignmentOperation a when RootedInBotState(a.Target):
                        case IIncrementOrDecrementOperation id when RootedInBotState(id.Target):
                            _mutSelf.Add(m);
                            break;
                        case IInvocationOperation inv:
                        {
                            var target = inv.TargetMethod.OriginalDefinition;
                            if (IsBotMember(target)) calls.Add(target);
                            if (Mutators.Contains(target.Name) && RootedInBotState(inv.Instance)) _mutSelf.Add(m);
                            if (inv.Arguments.Any(arg => arg.Parameter?.RefKind is RefKind.Ref or RefKind.Out && RootedInBotState(arg.Value))) _mutSelf.Add(m);
                            if (Emitter.MutGameEngineMembers.Contains(target.ContainingType.Name + "." + target.Name)) _mutGame.Add(m);
                            break;
                        }
                        case IMethodReferenceOperation mref when IsBotMember(mref.Method):
                            calls.Add(mref.Method.OriginalDefinition);
                            break;
                        case IPropertyReferenceOperation pr when IsBotMember(pr.Property) && IsComputed(pr.Property) && pr.Property.GetMethod != null:
                            calls.Add(pr.Property.GetMethod.OriginalDefinition);
                            break;
                    }
                }
            if (m.Name is "SetGameAndPlayer") _mutSelf.Add(m);
        }

        // Do runs the method it is given.
        foreach (var m in BotMethods().Where(m => m.Name == "Do")) { _mutSelf.Add(m); _mutGame.Add(m); }

        bool changed;
        do
        {
            changed = false;
            foreach (var (m, callees) in _calls)
            {
                if (!_mutSelf.Contains(m) && callees.Any(_mutSelf.Contains)) changed |= _mutSelf.Add(m);
                if (!_mutGame.Contains(m) && callees.Any(_mutGame.Contains)) changed |= _mutGame.Add(m);
            }
        } while (changed);
    }

    // Overloads: the first keeps the name, later ones get their parameter types appended (engine-conventions.md).
    void AssignNames()
    {
        var used = new Dictionary<string, int>();
        foreach (var t in trees)
        {
            var sm = model.Comp.GetSemanticModel(t);
            foreach (var md in t.GetRoot().DescendantNodes().OfType<MethodDeclarationSyntax>())
            {
                var s = sm.GetDeclaredSymbol(md)!;
                var owner = s.ContainingType.Name;
                var name = Emitter.Snake(s.Name);
                var key = owner + "." + name;
                if (used.ContainsKey(key))
                    name = name + "_" + string.Join("_", s.Parameters.Select(p => Emitter.Snake(TypeWord(p.Type))));
                used[owner + "." + name] = 1;
                _names[s] = Emitter.Ident(name);
            }
        }
    }

    static string TypeWord(ITypeSymbol t) => t switch
    {
        IArrayTypeSymbol a => TypeWord(a.ElementType) + "s",
        INamedTypeSymbol { OriginalDefinition.SpecialType: SpecialType.System_Nullable_T } n => TypeWord(n.TypeArguments[0]),
        INamedTypeSymbol { IsGenericType: true } n => n.Name + string.Concat(n.TypeArguments.Select(TypeWord)),
        _ => t.SpecialType switch { SpecialType.System_Int32 => "Int32", SpecialType.System_Boolean => "Bool", SpecialType.System_Single => "Single", _ => t.Name }
    };

    #endregion

    #region emit

    string? StateField(MemberDeclarationSyntax member, SemanticModel sm)
    {
        ISymbol? s = member switch
        {
            FieldDeclarationSyntax fd when !fd.Modifiers.Any(x => x.Text is "const" or "static") => sm.GetDeclaredSymbol(fd.Declaration.Variables[0]),
            PropertyDeclarationSyntax pd when !pd.Modifiers.Any(x => x.Text == "static") && pd.ExpressionBody == null &&
                                             pd.AccessorList?.Accessors.All(a => a.Body == null && a.ExpressionBody == null) == true => sm.GetDeclaredSymbol(pd),
            _ => null
        };
        if (s == null) return null;
        var type = s is IFieldSymbol f ? f.Type : ((IPropertySymbol)s).Type;
        var sb = new StringBuilder();
        var tree = member.SyntaxTree;
        sb.AppendLine($"    /// C#: `{Emitter.BotCsSig(gen, s)}` ({FileName(tree)}:{Line(member)})");
        if (s.Name == "Game")
        {
            sb.AppendLine("    // Not stored: every method takes the game as a parameter. SetGameAndPlayer only rebinds `player`.");
            return sb.ToString();
        }
        var name = Emitter.Ident(Emitter.Snake(s.Name));
        sb.AppendLine($"    pub {name}: {gen.BotRT(type, Pos.Field)},");
        return sb.ToString();
    }

    void EmitBotPartial(SyntaxTree t, ClassDeclarationSyntax cls, SemanticModel sm, StringBuilder sb, string? stateFields)
    {
        if (stateFields != null)
        {
            sb.AppendLine("/// C#: `public partial class ClassicBot(Game game, Player player, BotParameters param, LoggedRandom? random = null) : IBot` (Bot.cs:12)");
            sb.AppendLine("///");
            sb.AppendLine("/// The fields of every partial file, in file order. One bot per faction lives for the whole game (hazards.md 16.8).");
            sb.AppendLine("pub struct ClassicBot {");
            sb.Append(stateFields);
            sb.AppendLine("}");
            sb.AppendLine();
        }

        var nested = new List<BaseTypeDeclarationSyntax>();
        sb.AppendLine("impl ClassicBot {");
        if (stateFields != null && cls.ParameterList != null)
        {
            _methods++;
            sb.AppendLine("    /// C#: `ClassicBot(Game game, Player player, BotParameters param, LoggedRandom? random = null)` (primary constructor, Bot.cs:12)");
            sb.AppendLine("    pub fn new(game: &Game, player: PlayerIndex, param: BotParameters, random: Option<SharedRandom>) -> Self {");
            sb.AppendLine($"        {Todo(t, Line(cls))}");
            sb.AppendLine("    }");
            sb.AppendLine();
        }
        foreach (var member in cls.Members)
        {
            switch (member)
            {
                case BaseTypeDeclarationSyntax nt: nested.Add(nt); break;
                case MethodDeclarationSyntax md: EmitMethod(t, sm.GetDeclaredSymbol(md)!, md, sb); break;
                case PropertyDeclarationSyntax pd when IsComputed(sm.GetDeclaredSymbol(pd)!):
                    EmitGetter(t, sm.GetDeclaredSymbol(pd)!, pd, sb, true);
                    break;
                case FieldDeclarationSyntax fd when fd.Modifiers.Any(x => x.Text == "const"):
                    EmitConst(t, (IFieldSymbol)sm.GetDeclaredSymbol(fd.Declaration.Variables[0])!, fd, sb);
                    break;
            }
        }
        sb.AppendLine("}");
        sb.AppendLine();
        foreach (var nt in nested) EmitType(t, nt, sm, sb);
    }

    void EmitConst(SyntaxTree t, IFieldSymbol f, MemberDeclarationSyntax decl, StringBuilder sb)
    {
        sb.AppendLine($"    /// C#: `{Emitter.BotCsSig(gen, f)}` ({FileName(t)}:{Line(decl)})");
        var v = Emitter.BotConst(gen, f) ?? Todo(t, Line(decl));
        sb.AppendLine($"    pub const {Emitter.Snake(f.Name).ToUpperInvariant()}: {gen.BotRT(f.Type, Pos.Field)} = {v};");
        sb.AppendLine();
    }

    string Ret(IMethodSymbol m, ITypeSymbol type)
    {
        if (type.SpecialType == SpecialType.System_Void) return "";
        var r = gen.BotRT(type, Pos.Ret);
        if (Emitter.BotReturnsNull(m) && !r.StartsWith("Option<") && r != "ExpressionElement" && !(type is ITypeParameterSymbol)) r = $"Option<{r}>";
        return r;
    }

    List<string> Params(IMethodSymbol m, bool instance, bool mutGame)
    {
        var ps = new List<string>();
        var gameRef = mutGame ? "&mut Game" : "&Game";
        var hasGame = m.Parameters.Any(p => p.Type.Name == "Game");
        if (instance)
        {
            ps.Add(_mutSelf.Contains(m.OriginalDefinition) ? "&mut self" : "&self");
            if (!hasGame) ps.Add("game: " + gameRef);
        }
        else if (!hasGame && m.Parameters.Any(p => p.Type.Name == "Player" || p.Type.ToDisplayString().Contains("Player")))
            ps.Add("game: " + gameRef);
        foreach (var p in m.Parameters)
        {
            var name = Emitter.Ident(Emitter.Snake(p.Name));
            string type;
            if (p.Type.Name == "Game") type = gameRef;
            else if (p.RefKind is RefKind.Ref or RefKind.Out) type = "&mut " + gen.BotRT(p.Type, Pos.Ret);
            else if (p.IsParams && p.Type is IArrayTypeSymbol at) type = $"Vec<{gen.BotRT(at.ElementType, Pos.Elem)}>";
            else type = gen.BotRT(p.Type, Pos.Param);
            ps.Add($"{name}: {type}");
        }
        return ps;
    }

    void EmitMethod(SyntaxTree t, IMethodSymbol m, MethodDeclarationSyntax md, StringBuilder sb)
    {
        _methods++;
        var isBot = SymbolEqualityComparer.Default.Equals(m.ContainingType, _bot);
        var instance = !m.IsStatic;
        var ps = Params(m, instance, _mutGame.Contains(m.OriginalDefinition));
        if (instance && !isBot)
        {
            // Methods of bot helper types: plain receivers.
            ps[0] = "&self";
            ps.RemoveAt(1);
        }
        var generics = m.TypeParameters.Length > 0 ? "<" + string.Join(", ", m.TypeParameters.Select(tp => tp.Name)) + ">" : "";
        var ret = Ret(m, m.ReturnType);
        sb.AppendLine($"    /// C#: `{Emitter.BotCsSig(gen, m)}` ({FileName(t)}:{Line(md)})");
        sb.AppendLine($"    pub fn {_names[m]}{generics}({string.Join(", ", ps)}){(ret == "" ? "" : " -> " + ret)} {{");
        sb.AppendLine($"        {Todo(t, Line(md))}");
        sb.AppendLine("    }");
        sb.AppendLine();
    }

    void EmitGetter(SyntaxTree t, IPropertySymbol p, PropertyDeclarationSyntax pd, StringBuilder sb, bool takesGame)
    {
        _methods++;
        var getter = p.GetMethod!;
        var recv = _mutSelf.Contains(getter.OriginalDefinition) ? "&mut self" : "&self";
        var gameParam = takesGame ? (_mutGame.Contains(getter.OriginalDefinition) ? ", game: &mut Game" : ", game: &Game") : "";
        var ret = gen.BotRT(p.Type, Pos.Ret);
        sb.AppendLine($"    /// C#: `{Emitter.BotCsSig(gen, p)}` ({FileName(t)}:{Line(pd)})");
        sb.AppendLine($"    pub fn {Emitter.Ident(Emitter.Snake(p.Name))}({recv}{gameParam}) -> {ret} {{");
        sb.AppendLine($"        {Todo(t, Line(pd))}");
        sb.AppendLine("    }");
        sb.AppendLine();
    }

    void EmitType(SyntaxTree t, BaseTypeDeclarationSyntax decl, SemanticModel sm, StringBuilder sb)
    {
        var s = (INamedTypeSymbol)sm.GetDeclaredSymbol(decl)!;
        var cs = Emitter.BotOneLine($"{string.Join(" ", decl.Modifiers.Select(x => x.Text))} {(decl switch { TypeDeclarationSyntax x => x.Keyword.Text, EnumDeclarationSyntax x => x.EnumKeyword.Text, _ => "" })} {decl.Identifier}{(decl is TypeDeclarationSyntax td && td.ParameterList != null ? td.ParameterList.ToString() : "")}");
        sb.AppendLine($"/// C#: `{cs}` ({FileName(t)}:{Line(decl)})");
        if (decl is EnumDeclarationSyntax ed)
        {
            // Bot-internal: never cast from an int or deserialized, so a closed Rust enum is exact.
            sb.AppendLine("#[derive(Clone, Copy, PartialEq, Eq, Debug)]");
            sb.AppendLine($"pub enum {s.Name} {{");
            foreach (var m in ed.Members) sb.AppendLine($"    {m.Identifier.Text},");
            sb.AppendLine("}");
            sb.AppendLine();
            return;
        }

        var type = (TypeDeclarationSyntax)decl;
        var fields = new List<(ISymbol sym, ITypeSymbol type, MemberDeclarationSyntax decl)>();
        foreach (var member in type.Members)
        {
            switch (member)
            {
                case FieldDeclarationSyntax fd when !fd.Modifiers.Any(x => x.Text is "const" or "static"):
                    foreach (var v in fd.Declaration.Variables)
                    {
                        var f = (IFieldSymbol)sm.GetDeclaredSymbol(v)!;
                        fields.Add((f, f.Type, fd));
                    }
                    break;
                case PropertyDeclarationSyntax pd when !pd.Modifiers.Any(x => x.Text == "static") && !IsComputed(sm.GetDeclaredSymbol(pd)!):
                {
                    var p = sm.GetDeclaredSymbol(pd)!;
                    fields.Add((p, p.Type, pd));
                    break;
                }
            }
        }

        var allScalar = fields.All(f => f.type.SpecialType is SpecialType.System_Int32 or SpecialType.System_Single or SpecialType.System_Boolean);
        if (allScalar && fields.Count > 0) sb.AppendLine("#[derive(Clone, Copy, PartialEq, Debug, Default)]");
        sb.AppendLine($"pub struct {s.Name} {{");
        foreach (var (sym, ft, d) in fields)
        {
            sb.AppendLine($"    /// C#: `{Emitter.BotCsSig(gen, sym)}` ({FileName(t)}:{Line(d)})");
            sb.AppendLine($"    pub {Emitter.Ident(Emitter.Snake(sym.Name))}: {gen.BotRT(ft, Pos.Field)},");
        }
        sb.AppendLine("}");
        sb.AppendLine();

        var impl = new StringBuilder();
        if (type.ParameterList != null)
        {
            _methods++;
            var ps = type.ParameterList.Parameters.Select(p =>
            {
                var ps2 = sm.GetDeclaredSymbol(p)!;
                return $"{Emitter.Ident(Emitter.Snake(ps2.Name))}: {gen.BotRT(ps2.Type, Pos.Field)}";
            });
            impl.AppendLine($"    /// C#: primary constructor `{s.Name}{Emitter.BotOneLine(type.ParameterList.ToString())}` ({FileName(t)}:{Line(type)})");
            impl.AppendLine($"    pub fn new({string.Join(", ", ps)}) -> Self {{");
            impl.AppendLine($"        {Todo(t, Line(type))}");
            impl.AppendLine("    }");
            impl.AppendLine();
        }
        foreach (var member in type.Members)
        {
            switch (member)
            {
                case FieldDeclarationSyntax fd when fd.Modifiers.Any(x => x.Text == "const"):
                    EmitConst(t, (IFieldSymbol)sm.GetDeclaredSymbol(fd.Declaration.Variables[0])!, fd, impl);
                    break;
                case FieldDeclarationSyntax fd when fd.Modifiers.Any(x => x.Text == "static"):
                    foreach (var v in fd.Declaration.Variables)
                    {
                        _methods++;
                        var f = (IFieldSymbol)sm.GetDeclaredSymbol(v)!;
                        impl.AppendLine($"    /// C#: `{Emitter.BotCsSig(gen, f).Split(" = new")[0]}` ({FileName(t)}:{Line(fd)})");
                        impl.AppendLine("    /// A C# static readonly field. Nothing mutates it, so a constant value is exact (hazards.md 16.8).");
                        impl.AppendLine($"    pub fn {Emitter.Snake(f.Name)}() -> {gen.BotRT(f.Type, Pos.Field)} {{");
                        impl.AppendLine($"        {Todo(t, Line(fd))}");
                        impl.AppendLine("    }");
                        impl.AppendLine();
                    }
                    break;
                case MethodDeclarationSyntax md: EmitMethod(t, sm.GetDeclaredSymbol(md)!, md, impl); break;
                case PropertyDeclarationSyntax pd when IsComputed(sm.GetDeclaredSymbol(pd)!):
                    EmitGetter(t, sm.GetDeclaredSymbol(pd)!, pd, impl, false);
                    break;
            }
        }
        if (impl.Length > 0)
        {
            sb.AppendLine($"impl {s.Name} {{");
            sb.Append(impl);
            sb.AppendLine("}");
            sb.AppendLine();
        }
    }

    string IBotModule()
    {
        var levels = new[] { "DetermineHighestPriorityInPhaseAction", "DetermineHighPriorityInPhaseAction", "DetermineMiddlePriorityInPhaseAction", "DetermineLowPriorityInPhaseAction", "DetermineEndPhaseAction" };
        var sb = new StringBuilder();
        sb.AppendLine("//! Port of `Shared/Model/IBot.cs`, which the engine port left out (it has no engine callers).");
        sb.AppendLine("//! Generated by `engine/csharp-gen --bots`.");
        sb.AppendLine();
        sb.AppendLine("use crate::prelude::*;");
        sb.AppendLine();
        sb.AppendLine("/// C#: `public interface IBot` (Shared/Model/IBot.cs)");
        sb.AppendLine("pub trait IBot {");
        foreach (var l in levels)
        {
            sb.AppendLine($"    /// C#: `GameEvent? {l}(List<Type> events)`");
            sb.AppendLine($"    fn {Emitter.Snake(l)}(&mut self, game: &mut Game, events: &[GameEventType]) -> Option<GameEvent>;");
        }
        sb.AppendLine("    /// C#: `void SetGameAndPlayer(Game gameGame, Player player)`");
        sb.AppendLine("    fn set_game_and_player(&mut self, game: &Game, player: PlayerIndex);");
        sb.AppendLine("}");
        sb.AppendLine();
        sb.AppendLine("impl IBot for ClassicBot {");
        foreach (var l in levels)
        {
            var n = Emitter.Snake(l);
            sb.AppendLine($"    fn {n}(&mut self, game: &mut Game, events: &[GameEventType]) -> Option<GameEvent> {{");
            sb.AppendLine($"        ClassicBot::{n}(self, game, events)");
            sb.AppendLine("    }");
        }
        sb.AppendLine("    fn set_game_and_player(&mut self, game: &Game, player: PlayerIndex) {");
        sb.AppendLine("        ClassicBot::set_game_and_player(self, game, player)");
        sb.AppendLine("    }");
        sb.AppendLine("}");
        return sb.ToString();
    }

    #endregion
}
