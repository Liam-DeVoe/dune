using System.Reflection;
using Microsoft.CodeAnalysis;
using Microsoft.CodeAnalysis.CSharp;
using Microsoft.CodeAnalysis.CSharp.Syntax;
using Microsoft.CodeAnalysis.Operations;

namespace CsharpGen;

public enum TKind { Game, Player, Event, EventBase, Interface, Model, Static }

public sealed class FieldX
{
    public string Raw = "";        // reflection name
    public string CsName = "";     // normalized (StateDumper.NormalizeFieldName)
    public string Rust = "";       // rust field name
    public string RustType = "";
    public ISymbol? Sym;           // IFieldSymbol, IPropertySymbol or IParameterSymbol
    public ITypeSymbol? Type;
    public string File = "";
    public int Line;
    public string? Init;           // initializer text
    public bool Public;
    public bool Cell;              // event field mutated after construction
    public bool Omit;              // StateDumper omits it
    public string Comment = "";
}

public enum MKind { Method, Getter, Setter, Ctor, Const, StaticField, Operator, Indexer }

public sealed class MemberX
{
    public ISymbol Sym = null!;          // method/property/field symbol
    public IMethodSymbol? Method;         // the method or accessor analyzed
    public MKind Kind;
    public string File = "";
    public int Line;
    public string CsSig = "";
    public bool IsStatic;
    public string RustName = "";
    public string Signature = "";        // full rust "pub fn ..." line (without body)
    public string? Body;                 // generated body (null = todo!)
    public string? Doc;
}

public sealed class TypeX
{
    public INamedTypeSymbol Sym = null!;
    public string Name = "";
    public TKind Kind;
    public Type? Runtime;
    public List<FieldX> Fields = [];
    public List<string> Files = [];                  // declaration files, C# order
    public Dictionary<string, List<MemberX>> Members = [];   // per file, C# order
    public ISymbol? GameRef;                         // the member holding the owning Game
    public bool IsRecord;
    public bool IsStruct;
}

public sealed class Flags
{
    public bool Self;      // mutates its own instance
    public bool Game;      // mutates other game state
    public bool UsesGame;  // reads the owning Game (Player, PlayerSequence, events)
    public bool NeedsRc;   // passes `this` as a value (events)
    public bool UsesVirtual; // GameEvent members that dispatch on the concrete event
    public string? Why;      // first reason for Game (debugging)
    public HashSet<int> ParamMut = [];   // collection parameters it adds to / removes from
    public bool Same(Flags o) => Self == o.Self && Game == o.Game && UsesGame == o.UsesGame && NeedsRc == o.NeedsRc && UsesVirtual == o.UsesVirtual && ParamMut.SetEquals(o.ParamMut);
}

public enum Root { This, ThisGame, Fresh, Other, Static }

public sealed partial class Model
{
    public readonly string SharedDir;
    public CSharpCompilation Comp = null!;
    public readonly Dictionary<string, SyntaxTree> Trees = [];      // rel path -> tree
    public readonly List<string> SelectedFiles = [];
    public readonly Dictionary<string, TypeX> Types = [];           // by C# name
    public readonly Dictionary<IMethodSymbol, Flags> FlagsOf = new(SymbolEqualityComparer.Default);
    public readonly HashSet<ISymbol> CellMembers = new(SymbolEqualityComparer.Default);
    public INamedTypeSymbol GameEventSym = null!, GameSym = null!, PlayerSym = null!;
    public readonly List<string> Warnings = [];
    public readonly Dictionary<string, List<INamedTypeSymbol>> EnumsByFile = [];

    // Files ported by the foundation layer.
    public static readonly HashSet<string> Ported =
    [
        "Enumerations.cs", "ExtensionMethods.cs", "LeaderManager.cs", "LoggedRandom.cs", "Map.cs", "TreacheryCardManager.cs",
        "Model/AttachedLocation.cs", "Model/Deck.cs", "Model/DiscoveryStronghold.cs", "Model/Expression.cs", "Model/HiddenMobileStronghold.cs",
        "Model/Homeworld.cs", "Model/IFetcher.cs", "Model/IHero.cs", "Model/IIdentifiable.cs", "Model/Leader.cs", "Model/Location.cs",
        "Model/Message.cs", "Model/Messiah.cs", "Model/Payment.cs", "Model/Point.cs", "Model/Report.cs", "Model/ResourceCard.cs",
        "Model/Territory.cs", "Model/TreacheryCard.cs", "GlobalUsings.cs"
    ];

    // Files that are not part of the engine port (rendering, server, hub).
    public static readonly Dictionary<string, string> Excluded = new()
    {
        ["DefaultSkin.cs"] = "rendering (not ported, see engine-conventions.md)",
        ["Model/Skin.cs"] = "rendering (not ported, see engine-conventions.md)",
        ["Model/IDescriber.cs"] = "rendering",
        ["GameChatMessage.cs"] = "server", ["Model/ChatMessage.cs"] = "server", ["Model/GlobalChatMessage.cs"] = "server",
        ["Model/AdminInfo.cs"] = "server", ["Model/AvailableSeatInfo.cs"] = "server", ["Model/GameInitInfo.cs"] = "server", ["Model/PhaseControlInfo.cs"] = "server",
        ["Model/IBot.cs"] = "server", ["Model/IGameClient.cs"] = "server", ["Model/IGameHub.cs"] = "server",
        ["Model/LoggedInUserInfo.cs"] = "server", ["Model/LoginInfo.cs"] = "server", ["Model/ManagedGame.cs"] = "server",
        ["Model/ObserverJoined.cs"] = "server", ["Model/ObserverRejoined.cs"] = "server", ["Model/PlayerJoined.cs"] = "server",
        ["Model/PlayerRejoined.cs"] = "server", ["Model/Result.cs"] = "server", ["Model/ScheduledGame.cs"] = "server",
        ["Model/ScheduledGameInfo.cs"] = "server", ["Model/ServerInfo.cs"] = "server", ["Model/ServerStatus.cs"] = "server",
        ["Model/UserConnections.cs"] = "server", ["Model/UserInfo.cs"] = "server",
    };

    public Model(string sharedDir) => SharedDir = sharedDir;

    public string Rel(SyntaxTree t) => Path.GetRelativePath(SharedDir, t.FilePath).Replace('\\', '/');

    public void Load()
    {
        var files = Directory.GetFiles(SharedDir, "*.cs", SearchOption.AllDirectories)
            .Where(f => !f.Contains("/bin/") && !f.Contains("/obj/"))
            .OrderBy(f => Path.GetRelativePath(SharedDir, f), StringComparer.Ordinal).ToList();
        var trees = new List<SyntaxTree>();
        foreach (var f in files)
        {
            var t = CSharpSyntaxTree.ParseText(File.ReadAllText(f), new CSharpParseOptions(LanguageVersion.Latest), f);
            trees.Add(t);
            Trees[Rel(t)] = t;
        }

        var refs = ((string)AppContext.GetData("TRUSTED_PLATFORM_ASSEMBLIES")!).Split(Path.PathSeparator)
            .Where(p => Path.GetFileName(p).StartsWith("System") || Path.GetFileName(p).StartsWith("Microsoft") || Path.GetFileName(p) == "netstandard.dll" || Path.GetFileName(p) == "mscorlib.dll")
            .Where(p => !Path.GetFileName(p).StartsWith("Microsoft.CodeAnalysis"))
            .Select(p => MetadataReference.CreateFromFile(p));
        Comp = CSharpCompilation.Create("Shared", trees, refs,
            new CSharpCompilationOptions(OutputKind.DynamicallyLinkedLibrary, nullableContextOptions: NullableContextOptions.Enable));
        var errors = new List<Diagnostic>();
        foreach (var t in trees)
        {
            try { errors.AddRange(Comp.GetSemanticModel(t).GetDiagnostics().Where(d => d.Severity == DiagnosticSeverity.Error)); }
            catch (Exception e) { Console.Error.WriteLine($"roslyn crashed on {Rel(t)}: {e.GetType().Name}"); }
        }
        foreach (var e in errors.Take(10)) Console.Error.WriteLine("compile error: " + e);
        if (errors.Count > 0) Console.Error.WriteLine($"{errors.Count} Roslyn errors (types may be partially unresolved)");

        GameEventSym = FindType("GameEvent");
        GameSym = FindType("Game");
        PlayerSym = FindType("Player");

        foreach (var (rel, tree) in Trees)
        {
            if (Ported.Contains(rel) || Excluded.ContainsKey(rel)) continue;
            SelectedFiles.Add(rel);
            var sm = Comp.GetSemanticModel(tree);
            foreach (var decl in tree.GetRoot().DescendantNodes().OfType<BaseTypeDeclarationSyntax>())
            {
                var sym = (INamedTypeSymbol)sm.GetDeclaredSymbol(decl)!;
                if (sym.TypeKind == TypeKind.Enum)
                {
                    if (!EnumsByFile.TryGetValue(rel, out var list)) EnumsByFile[rel] = list = [];
                    list.Add(sym);
                    continue;
                }

                var name = sym.Name;
                if (!Types.TryGetValue(name, out var tx))
                {
                    tx = new TypeX { Sym = sym, Name = name, Kind = KindOf(sym), IsRecord = sym.IsRecord, IsStruct = sym.TypeKind == TypeKind.Struct };
                    Types[name] = tx;
                }
                if (!tx.Files.Contains(rel)) tx.Files.Add(rel);
            }
        }

        // Runtime types for exact field lists.
        var asm = typeof(Treachery.Shared.Game).Assembly;
        foreach (var t in Types.Values)
            t.Runtime = asm.GetTypes().FirstOrDefault(r => r.Name == MetadataName(t.Sym) && r.DeclaringType == null);
    }

    static string MetadataName(INamedTypeSymbol s) => s.MetadataName;

    public bool IsEventType(ITypeSymbol? t)
    {
        for (var c = t as INamedTypeSymbol; c != null; c = c.BaseType)
            if (SymbolEqualityComparer.Default.Equals(c.OriginalDefinition, GameEventSym)) return true;
        return false;
    }

    public bool IsConcreteEvent(ITypeSymbol? t) => IsEventType(t) && !t!.IsAbstract;

    TKind KindOf(INamedTypeSymbol s)
    {
        if (s.Name == "Game") return TKind.Game;
        if (s.Name == "Player") return TKind.Player;
        if (s.TypeKind == TypeKind.Interface) return TKind.Interface;
        if (IsEventType(s)) return s.IsAbstract ? TKind.EventBase : TKind.Event;
        if (s.IsStatic) return TKind.Static;
        return TKind.Model;
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

    public (string file, int line) Loc(ISymbol s)
    {
        var l = s.Locations.FirstOrDefault(x => x.IsInSource);
        if (l == null) return ("?", 0);
        return (Rel(l.SourceTree!), l.GetLineSpan().StartLinePosition.Line + 1);
    }

    public (string file, int line) Loc(SyntaxNode n) => (Rel(n.SyntaxTree), n.GetLocation().GetLineSpan().StartLinePosition.Line + 1);
}
