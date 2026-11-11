// Lists every member of Treachery.Shared that the Blazor client (treachery-online/Client) uses,
// per .razor component or .cs file, so the per-player view (rust/view) can be checked against it.
//
//   rust/view/csharp-inventory/run.sh
//
// The razor files are compiled by the Razor source generator. run.sh builds the client with
// EmitCompilerGeneratedFiles so the generated C# lands in a temp dir, and asks MSBuild for the
// reference list. This program then compiles the client's own .cs files plus the generated ones
// with Roslyn, resolves every name, and keeps those whose symbol lives in Treachery.Shared.
// A use in generated code is charged to the .razor file named in its #pragma checksum line.
//
// Args: <clientDir> <generatedDir> <refs.txt> <out.json> <out.md> <areas.tsv>

using System.Text;
using System.Text.Json;
using System.Text.RegularExpressions;
using Microsoft.CodeAnalysis;
using Microsoft.CodeAnalysis.CSharp;
using Microsoft.CodeAnalysis.CSharp.Syntax;

var clientDir = Path.GetFullPath(args[0]);
var genDir = Path.GetFullPath(args[1]);
var refs = File.ReadAllLines(args[2]).Where(l => l.Trim().Length > 0).ToList();
var outJson = args[3];
var outMd = args[4];
var areaRules = File.ReadAllLines(args[5])
    .Where(l => l.Trim().Length > 0 && !l.StartsWith('#'))
    .Select(l => l.Split('\t', 2))
    .Select(p => (re: new Regex("^(" + p[0] + ")$"), area: p[1].Trim()))
    .ToList();

var parse = new CSharpParseOptions(LanguageVersion.Preview);
var trees = new List<(SyntaxTree tree, string owner)>();

foreach (var f in Directory.EnumerateFiles(clientDir, "*.cs", SearchOption.AllDirectories))
{
    var rel = Path.GetRelativePath(clientDir, f);
    if (rel.StartsWith("obj") || rel.StartsWith("bin")) continue;
    trees.Add((CSharpSyntaxTree.ParseText(File.ReadAllText(f), parse, f), rel));
}

var checksum = new Regex("#pragma checksum \"([^\"]+)\"");
foreach (var f in Directory.EnumerateFiles(genDir, "*.cs", SearchOption.AllDirectories))
{
    var text = File.ReadAllText(f);
    var m = checksum.Match(text);
    var owner = m.Success ? Path.GetRelativePath(clientDir, m.Groups[1].Value) : Path.GetRelativePath(genDir, f);
    trees.Add((CSharpSyntaxTree.ParseText(text, parse, f), owner));
}

// The global usings that the SDK would generate for the client project.
var globalUsings = "global using global::System; global using global::System.Collections.Generic; global using global::System.IO; " +
                   "global using global::System.Linq; global using global::System.Net.Http; global using global::System.Net.Http.Json; " +
                   "global using global::System.Threading; global using global::System.Threading.Tasks;";
trees.Add((CSharpSyntaxTree.ParseText(globalUsings, parse, "GlobalUsings.g.cs"), "(sdk)"));

var comp = CSharpCompilation.Create("Client", trees.Select(t => t.tree),
    refs.Select(r => MetadataReference.CreateFromFile(r)),
    new CSharpCompilationOptions(OutputKind.DynamicallyLinkedLibrary, nullableContextOptions: NullableContextOptions.Enable));

var errors = comp.GetDiagnostics().Where(d => d.Severity == DiagnosticSeverity.Error).ToList();
foreach (var e in errors.Take(20)) Console.Error.WriteLine(e);
if (errors.Count > 0) Console.Error.WriteLine($"{errors.Count} compile errors (uses near them may be missing)");

// owner file -> member key -> list of lines
var uses = new SortedDictionary<string, SortedDictionary<string, SortedSet<int>>>(StringComparer.Ordinal);
var memberKinds = new Dictionary<string, string>();
var signatures = new Dictionary<string, string>();

static bool IsShared(ISymbol? s) => s?.ContainingAssembly?.Name == "Treachery.Shared";

static string TypeName(ITypeSymbol t) => t switch
{
    INamedTypeSymbol n when n.ContainingType != null => TypeName(n.ContainingType) + "." + n.Name,
    _ => t.Name,
};

foreach (var (tree, owner) in trees)
{
    var sm = comp.GetSemanticModel(tree);
    foreach (var node in tree.GetRoot().DescendantNodes().OfType<SimpleNameSyntax>())
    {
        var info = sm.GetSymbolInfo(node);
        var sym = info.Symbol ?? info.CandidateSymbols.FirstOrDefault();
        if (sym is IMethodSymbol { ReducedFrom: { } emperor }) sym = emperor;
        if (sym is IMethodSymbol { IsGenericMethod: true } gm) sym = gm.OriginalDefinition;
        if (sym == null) continue;
        string? key = null, kind = null;
        switch (sym)
        {
            case INamedTypeSymbol t when IsShared(t):
                key = TypeName(t); kind = t.TypeKind == TypeKind.Enum ? "enum" : "type";
                break;
            case IFieldSymbol { ContainingType.TypeKind: TypeKind.Enum } ef when IsShared(ef):
                key = TypeName(ef.ContainingType) + "." + ef.Name; kind = "enum member";
                break;
            case IMethodSymbol { MethodKind: MethodKind.Constructor } c when IsShared(c):
                key = TypeName(c.ContainingType) + "..ctor"; kind = "ctor";
                break;
            case IMethodSymbol ms when IsShared(ms):
                key = TypeName(ms.ContainingType) + "." + ms.Name + "()"; kind = ms.IsStatic ? (ms.IsExtensionMethod ? "extension" : "static method") : "method";
                signatures[key] = ms.ReturnType.ToDisplayString(SymbolDisplayFormat.MinimallyQualifiedFormat) + " " + ms.Name + "(" + string.Join(", ", ms.Parameters.Select(p => p.Type.ToDisplayString(SymbolDisplayFormat.MinimallyQualifiedFormat) + " " + p.Name)) + ")";
                break;
            case IPropertySymbol ps when IsShared(ps):
                key = TypeName(ps.ContainingType) + "." + ps.Name; kind = ps.IsStatic ? "static property" : "property";
                break;
            case IFieldSymbol fs when IsShared(fs):
                key = TypeName(fs.ContainingType) + "." + fs.Name; kind = fs.IsConst ? "const" : fs.IsStatic ? "static field" : "field";
                break;
            case IEventSymbol es when IsShared(es):
                key = TypeName(es.ContainingType) + "." + es.Name; kind = "event";
                break;
        }
        if (key == null) continue;
        // Inherited members: name the type that declares them, but also note the receiver type for events.
        memberKinds[key] = kind!;
        var span = node.GetLocation().GetMappedLineSpan();
        var line = span.HasMappedPath || !owner.EndsWith(".razor") ? span.StartLinePosition.Line + 1 : 0;
        if (!uses.TryGetValue(owner, out var byMember)) uses[owner] = byMember = new(StringComparer.Ordinal);
        if (!byMember.TryGetValue(key, out var lines)) byMember[key] = lines = [];
        if (line > 0) lines.Add(line);
    }
}

string AreaOf(string owner)
{
    var o = owner.Replace('\\', '/');
    foreach (var (re, area) in areaRules) if (re.IsMatch(o)) return area;
    return "Unassigned";
}

// JSON: { owner: { area, members: { key: [lines] } } }
var json = new SortedDictionary<string, object>(StringComparer.Ordinal);
foreach (var (owner, byMember) in uses)
    json[owner] = new { area = AreaOf(owner), members = byMember.ToDictionary(kv => kv.Key, kv => kv.Value.ToArray()) };
File.WriteAllText(outJson, JsonSerializer.Serialize(new { kinds = new SortedDictionary<string, string>(memberKinds), files = json }, new JsonSerializerOptions { WriteIndented = true }));

// Markdown
static string Owning(string key)
{
    var i = key.IndexOf("..ctor", StringComparison.Ordinal);
    if (i >= 0) return key[..i];
    var dot = key.LastIndexOf('.', key.EndsWith("()") ? key.Length - 3 : key.Length - 1);
    return dot < 0 ? key : key[..dot];
}

var allFiles = Directory.EnumerateFiles(clientDir, "*.*", SearchOption.AllDirectories)
    .Select(f => Path.GetRelativePath(clientDir, f).Replace('\\', '/'))
    .Where(r => (r.EndsWith(".razor") || r.EndsWith(".cs")) && !r.StartsWith("obj/") && !r.StartsWith("bin/"))
    .ToList();

var md = new StringBuilder();
md.AppendLine("# Client inventory (generated)");
md.AppendLine();
md.AppendLine("Generated by `rust/view/csharp-inventory/run.sh` from `treachery-online/Client`. Do not edit by hand; edit `areas.tsv` or the tool and rerun.");
md.AppendLine();
md.AppendLine("Every member of `Treachery.Shared` that a client file names, resolved by Roslyn over the client's own `.cs` files and the C# that the Razor generator makes from each `.razor` file. A use in a razor file counts for that file. Uses through a client helper (`Support.cs`, `MapInteraction.cs`, ...) are listed under the helper, not under its callers. Line numbers are in the `.razor` or `.cs` file. Enum members and types are listed separately at the end of each area, because they carry no game data.");
md.AppendLine();
md.AppendLine("How to read it for the view design: a member listed under an in-game area is data the new client needs in its view, or a static rule/helper whose result the server must compute for it. [client-plan.md](client-plan.md) and the Game view section of [protocol.md](protocol.md) map each group to a view field.");
md.AppendLine();
var byArea = uses.GroupBy(kv => AreaOf(kv.Key)).OrderBy(g => g.Key, StringComparer.Ordinal).ToList();
md.AppendLine("## Areas");
md.AppendLine();
md.AppendLine("| Area | Files | Distinct Shared members (excluding types and enum members) |");
md.AppendLine("|---|---|---|");
foreach (var g in byArea)
{
    var n = g.SelectMany(kv => kv.Value.Keys).Distinct().Count(k => memberKinds[k] is not ("type" or "enum" or "enum member"));
    md.AppendLine($"| [{g.Key}](#{Anchor(g.Key)}) | {g.Count()} | {n} |");
}
md.AppendLine();
var unused = allFiles.Where(f => !uses.ContainsKey(f)).OrderBy(f => f).ToList();
md.AppendLine($"Files that use nothing from Shared: {unused.Count}. " + string.Join(", ", unused.Select(f => $"`{f}`")));
md.AppendLine();

foreach (var g in byArea)
{
    md.AppendLine($"## {g.Key}");
    md.AppendLine();
    md.AppendLine("Files: " + string.Join(", ", g.Select(kv => $"`{kv.Key}`")));
    md.AppendLine();
    // member -> files
    var members = new SortedDictionary<string, List<string>>(StringComparer.Ordinal);
    foreach (var (owner, byMember) in g)
        foreach (var (key, lines) in byMember)
        {
            if (!members.TryGetValue(key, out var l)) members[key] = l = [];
            l.Add(Path.GetFileNameWithoutExtension(owner) + (lines.Count > 0 ? ":" + string.Join(",", lines.Take(6)) + (lines.Count > 6 ? ",…" : "") : ""));
        }
    var data = members.Where(kv => memberKinds[kv.Key] is not ("type" or "enum" or "enum member")).GroupBy(kv => Owning(kv.Key)).OrderBy(x => x.Key, StringComparer.Ordinal);
    md.AppendLine("| Type | Member | Kind | Used in (file:lines) |");
    md.AppendLine("|---|---|---|---|");
    foreach (var t in data)
        foreach (var (key, files) in t)
            md.AppendLine($"| {t.Key} | `{key[(t.Key.Length)..].TrimStart('.')}` | {memberKinds[key]} | {string.Join("; ", files)} |");
    md.AppendLine();
    var types = members.Where(kv => memberKinds[kv.Key] == "type").Select(kv => kv.Key).ToList();
    if (types.Count > 0) md.AppendLine("Types named: " + string.Join(", ", types.Select(t => $"`{t}`")) + ".");
    md.AppendLine();
    var enums = members.Where(kv => memberKinds[kv.Key] == "enum member").Select(kv => kv.Key).ToList();
    if (enums.Count > 0) md.AppendLine("Enum members named: " + string.Join(", ", enums.Select(t => $"`{t}`")) + ".");
    md.AppendLine();
}

// Action forms: for each component that builds an event, the static helpers of event classes it calls.
// A helper whose parameters go beyond (Game, Player) depends on other inputs of the form: the new client
// gets its result through a draft query (protocol.md, Game view, Dependent inputs).
md.AppendLine("## Action forms");
md.AppendLine();
md.AppendLine("Static members of event classes (and `Game`/`Player` methods taking more than the viewer) that each action component calls to offer choices. A helper marked **dep** takes an argument besides the game and the acting player, usually another input of the same form, so its result is a dependent option: the server recomputes it for each draft (see the Game view section of protocol.md). This table is the checklist for the per-event option builders in `rust/view`.");
md.AppendLine();
md.AppendLine("| Component | Event helpers called (signature) |");
md.AppendLine("|---|---|");
static bool IsEventType(string owner) => owner is not ("Game" or "Player" or "Map" or "Skin" or "Message" or "Expression" or "MessagePart" or "Battalion" or "Location" or "Territory" or "LeaderState" or "TreacheryCard" or "Utilities" or "LeaderManager" or "TreacheryCardManager" or "Payment" or "Deck" or "IGameHub" or "GameStatus" or "Report" or "Participation" or "GameSettings" or "GameEvent" or "PassableGameEvent" or "PlacementEvent" or "Enumerations" or "EnumerationExtensions" or "IFetcher") && !owner.StartsWith("Skin.");
foreach (var (owner, byMember) in uses.Where(kv => kv.Key.Contains("GameEventComponents/") || AreaOf(kv.Key).StartsWith("Action panel") || kv.Key.Contains("BiddingPanel") || kv.Key.Contains("CardPlays")))
{
    var helpers = byMember.Keys
        .Where(k => memberKinds[k] == "static method" && IsEventType(Owning(k)))
        .Select(k =>
        {
            var sig = signatures.GetValueOrDefault(k, k);
            var ps = sig[(sig.IndexOf('(') + 1)..^1].Split(", ", StringSplitOptions.RemoveEmptyEntries)
                .Select(x => x.Split(' ')[0].TrimEnd('?'));
            var dep = ps.Any(t => t is not ("Game" or "Player"));
            return $"`{Owning(k)}.{sig[(sig.IndexOf(' ') + 1)..]}`" + (dep ? " **dep**" : "");
        }).ToList();
    if (helpers.Count == 0) continue;
    md.AppendLine($"| {Path.GetFileNameWithoutExtension(owner)} | {string.Join("<br>", helpers)} |");
}
md.AppendLine();

// Reverse index: every Shared member with the areas that use it.
md.AppendLine("## By Shared type");
md.AppendLine();
md.AppendLine("Every data member and method, with the areas that use it. Use this to check that a view field exists for each.");
md.AppendLine();
var rev = new SortedDictionary<string, SortedSet<string>>(StringComparer.Ordinal);
foreach (var (owner, byMember) in uses)
    foreach (var key in byMember.Keys)
    {
        if (memberKinds[key] is "type" or "enum" or "enum member") continue;
        if (!rev.TryGetValue(key, out var s)) rev[key] = s = new(StringComparer.Ordinal);
        s.Add(AreaOf(owner));
    }
foreach (var t in rev.GroupBy(kv => Owning(kv.Key)).OrderBy(x => x.Key, StringComparer.Ordinal))
{
    md.AppendLine($"### {t.Key}");
    md.AppendLine();
    foreach (var (key, areas) in t)
        md.AppendLine($"- `{key[(t.Key.Length)..].TrimStart('.')}` ({memberKinds[key]}): {string.Join(", ", areas)}");
    md.AppendLine();
}

File.WriteAllText(outMd, md.ToString());
Console.WriteLine($"{uses.Count} files, {rev.Count} members, {errors.Count} compile errors");

static string Anchor(string s) => Regex.Replace(s.ToLowerInvariant(), "[^a-z0-9 -]", "").Replace(' ', '-');
