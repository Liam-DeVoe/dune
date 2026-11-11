using Microsoft.CodeAnalysis;
using Microsoft.CodeAnalysis.CSharp.Syntax;
using Microsoft.CodeAnalysis.Operations;

namespace CsharpGen;

// Lists every member of Shared that returns a lazy IEnumerable, every call
// site of those members, every local that holds a lazy LINQ query, and how
// each site consumes the query. For rust/docs/lazy-queries.md.
//
//   dotnet run -- --lazy-queries ../tests/coverage/lazy_queries.txt
//
// A member is lazy when its declared return type is an IEnumerable interface
// and it is an iterator, or one of its return values is a deferred
// System.Linq operator, a call of another lazy member, or a conditional with
// a lazy branch. A virtual or interface member is lazy when one of its
// implementations is.
//
// Output, one tab-separated line per entry:
//   <file>:<line>:<Type.Member>   producer   <shape of the returned query>
//   <file>:<line>:<Type.Member>   <consumer> <call text>        (a call site)
//   <file>:<line>:local <name>    <consumer> <initializer text> (a lazy local)
// The consumer says what stops or drives the enumeration:
//   short:Op     Any, All, Contains, First, FirstOrDefault, Single, ElementAt...
//                (stops early);
//   full:Op      ToList, Count, Sum, a List constructor... (enumerates all once);
//   foreach      a foreach whose body never leaves the loop early;
//   foreach-exit(...) a foreach whose body can break, return or throw;
//   return M     the query is the return value of the enclosing member M (a
//                producer with its own sites);
//   arg:M[...]   passed to a Shared method; the brackets hold how M consumes it;
//   message      converted to object (a message argument, see live-lists.md);
//   store:X      assigned to a field or property;
//   local x{...} assigned to a local, with each use of the local; prefixed with
//                "double" when the local is enumerated more than once.
// " via A>B" lists the deferred operators applied between the call and the consumer.
public static class LazyQueries
{
    public static readonly HashSet<string> Deferred =
    [
        "Where", "Select", "SelectMany", "OrderBy", "OrderByDescending", "ThenBy", "ThenByDescending", "Distinct", "DistinctBy",
        "Union", "UnionBy", "Concat", "Except", "ExceptBy", "Intersect", "IntersectBy", "Skip", "SkipWhile", "SkipLast", "Take",
        "TakeWhile", "TakeLast", "Cast", "OfType", "Reverse", "GroupBy", "Zip", "Append", "Prepend", "DefaultIfEmpty", "Chunk",
        "Order", "OrderDescending", "Join", "GroupJoin", "AsEnumerable", "Index", "CountBy", "AggregateBy"
    ];

    public static readonly HashSet<string> Short =
    [
        "Any", "All", "Contains", "First", "FirstOrDefault", "Single", "SingleOrDefault", "ElementAt", "ElementAtOrDefault",
        "SequenceEqual", "TryGetNonEnumeratedCount"
    ];

    // Second-source operators that pull their second sequence lazily, as part of the chain.
    static readonly HashSet<string> SecondLazy = ["Concat", "Union", "UnionBy", "Zip"];

    sealed class Body
    {
        public string Rel = "";
        public IOperation Root = null!;
        public IMethodSymbol? Method;
        public SyntaxNode Decl = null!;
    }

    sealed class Producer
    {
        public IMethodSymbol Method = null!;
        public string Shape = "";
        public string Key = "";
    }

    static Model _model = null!;
    static Body? _current;
    static INamedTypeSymbol _enumerable = null!, _ienum = null!;
    static readonly Dictionary<IMethodSymbol, Producer> Producers = new(SymbolEqualityComparer.Default);
    static readonly Dictionary<IMethodSymbol, Body> BodyOf = new(SymbolEqualityComparer.Default);

    static IMethodSymbol Norm(IMethodSymbol m) => (m.ReducedFrom ?? m).OriginalDefinition;

    static string Text(SyntaxNode n)
    {
        var s = string.Join(" ", n.ToString().Split((char[])['\r', '\n', '\t'], StringSplitOptions.RemoveEmptyEntries).Select(x => x.Trim()));
        return s.Length > 160 ? s[..157] + "..." : s;
    }

    static int Line(SyntaxNode n) => n.GetLocation().GetLineSpan().StartLinePosition.Line + 1;

    static string Name(IMethodSymbol m)
    {
        var n = m.AssociatedSymbol is IPropertySymbol p ? p.Name : m.Name;
        return m.ContainingType.Name + "." + n;
    }

    static bool IsEnumerableInterface(ITypeSymbol? t) =>
        t is { TypeKind: TypeKind.Interface, SpecialType: not SpecialType.System_String } &&
        (SymbolEqualityComparer.Default.Equals(t, _ienum) || t.AllInterfaces.Contains(_ienum, SymbolEqualityComparer.Default));

    static bool IsLinq(IMethodSymbol m) => SymbolEqualityComparer.Default.Equals(m.ContainingType, _enumerable);

    static IOperation Strip(IOperation e)
    {
        while (true)
        {
            if (e is IConversionOperation c) e = c.Operand;
            else if (e is IParenthesizedOperation p) e = p.Operand;
            else return e;
        }
    }

    // Whether `e` evaluates to a lazy query; `shape` describes it.
    static bool IsLazy(IOperation e, out string shape)
    {
        e = Strip(e);
        switch (e)
        {
            case IInvocationOperation inv when IsLinq(inv.TargetMethod) && Deferred.Contains(inv.TargetMethod.Name):
            {
                var src = inv.Arguments.FirstOrDefault(a => a.Parameter?.Ordinal == 0)?.Value;
                var inner = src != null && IsLazy(src, out var s) ? s : src != null ? SourceKind(src) : "?";
                shape = inv.TargetMethod.Name + ">" + inner;
                return true;
            }
            case IInvocationOperation inv when Producers.TryGetValue(Norm(inv.TargetMethod), out var p):
                shape = "call " + Name(p.Method);
                return true;
            case IPropertyReferenceOperation pr when pr.Property.GetMethod is { } g && Producers.TryGetValue(Norm(g), out var p2):
                shape = "call " + Name(p2.Method);
                return true;
            case IConditionalOperation c:
            {
                var a = c.WhenTrue != null && IsLazy(c.WhenTrue, out var s1) ? s1 : null;
                var b = c.WhenFalse != null && IsLazy(c.WhenFalse, out var s2) ? s2 : null;
                shape = "(" + (a ?? "eager") + " | " + (b ?? "eager") + ")";
                return a != null || b != null;
            }
            case ISwitchExpressionOperation sw:
            {
                var arms = sw.Arms.Select(a => IsLazy(a.Value, out var sa) ? sa : null).ToList();
                shape = "switch(" + string.Join(" | ", arms.Select(a => a ?? "eager")) + ")";
                return arms.Any(a => a != null);
            }
            case ICoalesceOperation co:
            {
                var a = IsLazy(co.Value, out var s1) ? s1 : null;
                var b = IsLazy(co.WhenNull, out var s2) ? s2 : null;
                shape = "(" + (a ?? "eager") + " ?? " + (b ?? "eager") + ")";
                return a != null || b != null;
            }
        }
        shape = "";
        return false;
    }

    static string SourceKind(IOperation src)
    {
        src = Strip(src);
        var t = src.Type;
        var tn = t is IArrayTypeSymbol ? "array" : t?.Name ?? "?";
        return src switch
        {
            IInvocationOperation i => tn + "(" + i.TargetMethod.Name + ")",
            IPropertyReferenceOperation p => tn + "(" + p.Property.Name + ")",
            IFieldReferenceOperation f => tn + "(" + f.Field.Name + ")",
            IParameterReferenceOperation pr => tn + "(param " + pr.Parameter.Name + ")",
            ILocalReferenceOperation l => tn + "(local " + l.Local.Name + ")",
            _ => tn
        };
    }

    static bool InsideNestedFunction(IOperation op, IOperation root)
    {
        for (var p = op.Parent; p != null && p != root; p = p.Parent)
            if (p is IAnonymousFunctionOperation or ILocalFunctionOperation) return true;
        return false;
    }

    public static void Write(Model model, string outPath)
    {
        _model = model;
        _enumerable = model.Comp.GetTypeByMetadataName("System.Linq.Enumerable")!;
        _ienum = model.Comp.GetSpecialType(SpecialType.System_Collections_IEnumerable);

        var bodies = new List<Body>();
        foreach (var (rel, tree) in model.Trees)
        {
            if (Model.Excluded.ContainsKey(rel)) continue;
            var sm = model.Comp.GetSemanticModel(tree);
            foreach (var b in tree.GetRoot().DescendantNodes())
            {
                IMethodSymbol? method = null;
                switch (b)
                {
                    case BaseMethodDeclarationSyntax or AccessorDeclarationSyntax:
                        method = sm.GetDeclaredSymbol(b) as IMethodSymbol;
                        break;
                    case ArrowExpressionClauseSyntax { Parent: PropertyDeclarationSyntax or IndexerDeclarationSyntax } arrow:
                        method = (sm.GetDeclaredSymbol(arrow.Parent!) as IPropertySymbol)?.GetMethod;
                        break;
                    case EqualsValueClauseSyntax { Parent: VariableDeclaratorSyntax { Parent.Parent: FieldDeclarationSyntax } or PropertyDeclarationSyntax }:
                    case PrimaryConstructorBaseTypeSyntax:
                        break;
                    default:
                        continue;
                }
                IOperation? root;
                try { root = sm.GetOperation(b); }
                catch (NullReferenceException) { continue; }   // Roslyn bug on a few Location constructors; none returns a query
                if (root == null) continue;
                var body = new Body { Rel = rel, Root = root, Method = method, Decl = b };
                bodies.Add(body);
                if (method != null && !BodyOf.ContainsKey(Norm(method))) BodyOf[Norm(method)] = body;
            }
        }

        // Producers, to a fixpoint (a member returning another lazy member's result is lazy).
        bool changed;
        do
        {
            changed = false;
            foreach (var b in bodies)
            {
                if (b.Method == null || !IsEnumerableInterface(b.Method.ReturnType)) continue;
                var key = Norm(b.Method);
                if (Producers.ContainsKey(key)) continue;
                string? shape = null;
                if (b.Decl.DescendantNodes().OfType<YieldStatementSyntax>().Any(y => !y.Ancestors().TakeWhile(a => a != b.Decl).Any(a => a is LocalFunctionStatementSyntax or AnonymousFunctionExpressionSyntax)))
                    shape = "iterator";
                else
                {
                    var shapes = new List<string>();
                    var lazy = false;
                    foreach (var r in b.Root.DescendantsAndSelf().OfType<IReturnOperation>())
                    {
                        if (r.ReturnedValue == null || InsideNestedFunction(r, b.Root)) continue;
                        if (IsLazy(r.ReturnedValue, out var s)) { lazy = true; shapes.Add(s); }
                        else shapes.Add("eager " + SourceKind(r.ReturnedValue));
                    }
                    if (lazy) shape = string.Join(" ; ", shapes.Distinct());
                }
                if (shape == null) continue;
                var (file, line) = model.Loc(b.Decl);
                Producers[key] = new Producer { Method = b.Method, Shape = shape, Key = $"{file}:{line}:{Name(b.Method)}" };
                changed = true;
            }
            // A virtual or interface member is lazy when an implementation is.
            foreach (var p in Producers.Values.ToList())
            {
                var bases = new List<IMethodSymbol>();
                for (var o = p.Method.OverriddenMethod; o != null; o = o.OverriddenMethod) bases.Add(o);
                foreach (var i in p.Method.ContainingType.AllInterfaces)
                    foreach (var im in i.GetMembers().OfType<IMethodSymbol>())
                        if (SymbolEqualityComparer.Default.Equals(p.Method.ContainingType.FindImplementationForInterfaceMember(im), p.Method))
                            bases.Add(im);
                foreach (var bm in bases)
                {
                    var k = Norm(bm);
                    if (Producers.ContainsKey(k)) continue;
                    var (file, line) = model.Loc(bm);
                    Producers[k] = new Producer { Method = bm, Shape = "virtual, lazy in " + Name(p.Method), Key = $"{file}:{line}:{Name(bm)}" };
                    changed = true;
                }
            }
        } while (changed);

        var lines = new List<(string key, string rest)>();
        foreach (var p in Producers.Values) lines.Add((p.Key, "producer\t" + p.Shape));

        foreach (var b in bodies)
        {
            _current = b;
            foreach (var op in b.Root.DescendantsAndSelf())
            {
                IMethodSymbol? target = op switch
                {
                    IInvocationOperation inv => inv.TargetMethod,
                    IPropertyReferenceOperation pr when pr.Property.GetMethod != null && !(pr.Parent is ISimpleAssignmentOperation sa && sa.Target == pr) => pr.Property.GetMethod,
                    _ => null
                };
                if (target != null && Producers.TryGetValue(Norm(target), out var prod))
                {
                    lines.Add(($"{b.Rel}:{Line(op.Syntax)}:{Name(prod.Method)}", Consumer(op, b.Root, 0, []) + "\t" + Text(op.Syntax)));
                    continue;
                }
                // A local initialized with an inline lazy query (a producer call is listed above as its own site).
                // A foreach over an inline lazy query: the body runs between predicate calls (hazards 4.1).
                if (op is IForEachLoopOperation { Collection: var coll } fe && IsLazy(coll, out _) &&
                    Strip(coll) is IInvocationOperation { TargetMethod: var ftm } && IsLinq(ftm))
                    lines.Add(($"{b.Rel}:{Line(op.Syntax)}:foreach {(fe.Locals.FirstOrDefault()?.Name ?? "?")}", ForeachKind(fe) + "\t" + Text(coll.Syntax)));
                if (op is IVariableDeclaratorOperation { Initializer.Value: { } init } d && IsLazy(init, out _) &&
                    Strip(init) is IInvocationOperation { TargetMethod: var tm } && IsLinq(tm))
                    lines.Add(($"{b.Rel}:{Line(op.Syntax)}:local {d.Symbol.Name}", LocalUses(d.Symbol, b.Root, 0, []) + "\t" + Text(init.Syntax)));
            }
        }

        // Keys are unique: a second site with the same key gets "#2".
        var seen = new Dictionary<string, int>();
        var output = new List<string>();
        foreach (var (key, rest) in lines.OrderBy(l => l.key, StringComparer.Ordinal).ThenBy(l => l.rest, StringComparer.Ordinal))
        {
            var n = seen[key] = seen.GetValueOrDefault(key) + 1;
            output.Add((n == 1 ? key : key + "#" + n) + "\t" + rest);
        }
        File.WriteAllLines(outPath, output);
        Console.WriteLine($"{Producers.Count} lazy producers, {output.Count - Producers.Count} sites written to {outPath}");
    }

    static string Fmt(string what, List<string> via) => via.Count == 0 ? what : what + " via " + string.Join(">", via);

    // How the lazy value `v` is consumed.
    static string Consumer(IOperation v, IOperation root, int depth, HashSet<string> visiting)
    {
        var cur = v;
        var via = new List<string>();
        while (true)
        {
            var p = cur.Parent;
            switch (p)
            {
                case null:
                    return Fmt("other:root", via);
                case IConversionOperation or IParenthesizedOperation:
                    cur = p;
                    continue;
                case IConditionalOperation c when c.Condition != cur:
                    cur = p;
                    continue;
                case ICoalesceOperation or ISwitchExpressionArmOperation or ISwitchExpressionOperation:
                    cur = p;
                    continue;
                case IConditionalAccessOperation ca when ca.Operation == cur:
                {
                    var inst = ca.WhenNotNull.DescendantsAndSelf().OfType<IConditionalAccessInstanceOperation>().FirstOrDefault();
                    if (inst == null) return Fmt("other:conditional-access", via);
                    cur = inst;
                    continue;
                }
                case IArgumentOperation a:
                {
                    var call = a.Parent!;
                    var m = call switch { IInvocationOperation i => i.TargetMethod, IObjectCreationOperation o => o.Constructor, _ => null };
                    if (m == null) return Fmt("other:argument", via);
                    if (IsLinq(m))
                    {
                        var name = m.Name;
                        if (a.Parameter!.Ordinal == 0)
                        {
                            if (Deferred.Contains(name)) { via.Add(name); cur = call; continue; }
                            return Fmt((Short.Contains(name) ? "short:" : "full:") + name, via);
                        }
                        if (SecondLazy.Contains(name)) { via.Add(name + "(2nd)"); cur = call; continue; }
                        return Fmt("full:" + name + "(2nd)", via);
                    }
                    if (a.Parameter!.Type.SpecialType == SpecialType.System_Object ||
                        a.Parameter.Type is IArrayTypeSymbol { ElementType.SpecialType: SpecialType.System_Object })
                        return Fmt("message", via);
                    var key = Norm(m);
                    if (BodyOf.TryGetValue(key, out var cb) && depth < 4)
                    {
                        var param = key.Parameters[a.Parameter.Ordinal];
                        return Fmt($"arg:{Name(key)}({param.Name})[{ParamUses(key, param, cb, depth + 1, visiting)}]", via);
                    }
                    return Fmt("full:" + m.ContainingType.Name + "." + (m.MethodKind == MethodKind.Constructor ? "ctor" : m.Name), via);
                }
                case IForEachLoopOperation f when f.Collection == cur:
                    return Fmt(ForeachKind(f), via);
                case IReturnOperation r:
                {
                    // A lambda returning the query (SelectMany's selector, for example) is part of the outer query.
                    for (var q = r.Parent; q != null && q != root; q = q.Parent)
                    {
                        if (q is ILocalFunctionOperation) return Fmt("return(local function)", via);
                        if (q is IAnonymousFunctionOperation af)
                        {
                            var dc = af.Parent is IDelegateCreationOperation d ? d : null;
                            if (dc?.Parent is IArgumentOperation { Parent: IInvocationOperation outer } && IsLinq(outer.TargetMethod) && Deferred.Contains(outer.TargetMethod.Name))
                            {
                                via.Add(outer.TargetMethod.Name + "(inner)");
                                cur = outer;
                                goto next;
                            }
                            if (dc?.Parent is IArgumentOperation { Parent: IInvocationOperation outer2 } && IsLinq(outer2.TargetMethod))
                                return Fmt("full:" + outer2.TargetMethod.Name + "(inner)", via);
                            return Fmt("return(lambda)", via);
                        }
                    }
                    return Fmt(_current?.Method is { } cm ? "return " + Name(Norm(cm)) : "return", via);
                }
                case IVariableDeclaratorOperation d:
                    return Fmt(LocalUses(d.Symbol, root, depth, visiting), via);
                case IVariableInitializerOperation { Parent: IVariableDeclaratorOperation d2 }:
                    return Fmt(LocalUses(d2.Symbol, root, depth, visiting), via);
                case IVariableInitializerOperation:
                    cur = p;
                    continue;
                case ISimpleAssignmentOperation s when s.Value == cur:
                    if (s.Target is ILocalReferenceOperation lr) return Fmt(LocalUses(lr.Local, root, depth, visiting), via);
                    return Fmt("store:" + s.Target.Syntax, via);
                case IIsPatternOperation or IBinaryOperation:
                    return Fmt("none:null-check", via);
                case IConditionalAccessOperation ca2 when ca2.WhenNotNull == cur:
                    cur = p;
                    continue;
                case IInvocationOperation i when i.Instance == cur:
                {
                    // A C# 14 extension block member: the receiver is its extension parameter.
                    var em = Norm(i.TargetMethod);
                    if (em.ContainingType.ExtensionParameter is { } ep && BodyOf.TryGetValue(em, out var eb) && depth < 4)
                        return Fmt($"arg:{Name(em)}({ep.Name})[{ParamUses(em, ep, eb, depth + 1, visiting)}]", via);
                    return Fmt("other:" + i.TargetMethod.Name, via);
                }
                case IInterpolationOperation:
                    return Fmt("message", via);
                case IAnonymousFunctionOperation or IBlockOperation { Parent: IAnonymousFunctionOperation }:
                    cur = p;
                    continue;
                default:
                    return Fmt("other:" + p.Kind, via);
            }
            next: ;
        }
    }

    static string ForeachKind(IForEachLoopOperation f)
    {
        var exits = new SortedSet<string>(StringComparer.Ordinal);
        foreach (var o in f.Body.DescendantsAndSelf())
        {
            if (InsideNestedFunction(o, f.Body)) continue;
            switch (o)
            {
                case IBranchOperation b when SymbolEqualityComparer.Default.Equals(b.Target, f.ExitLabel):
                    exits.Add("break");
                    break;
                case IBranchOperation { BranchKind: BranchKind.GoTo }:
                    exits.Add("goto");
                    break;
                case IReturnOperation:
                    exits.Add("return");
                    break;
                case IThrowOperation:
                    exits.Add("throw");
                    break;
            }
        }
        return exits.Count == 0 ? "foreach" : "foreach-exit(" + string.Join(",", exits) + ")";
    }

    static bool IsEnumerating(string use) => !(use.StartsWith("none") || use.StartsWith("store") || use.StartsWith("return") || use.StartsWith("message"));

    static string LocalUses(ILocalSymbol local, IOperation root, int depth, HashSet<string> visiting)
    {
        var uses = new List<string>();
        var repeated = false;
        foreach (var r in root.DescendantsAndSelf().OfType<ILocalReferenceOperation>())
        {
            if (!SymbolEqualityComparer.Default.Equals(r.Local, local)) continue;
            if (r.Parent is ISimpleAssignmentOperation sa && sa.Target == r) continue;
            var use = Consumer(r, root, depth, visiting);
            uses.Add(use);
            // A use inside a loop or lambda that does not also contain the declaration can run more than once.
            var decl = root.DescendantsAndSelf().OfType<IVariableDeclaratorOperation>().FirstOrDefault(d => SymbolEqualityComparer.Default.Equals(d.Symbol, local));
            for (var q = r.Parent; q != null && q != root; q = q.Parent)
            {
                if (decl != null && q.DescendantsAndSelf().Contains(decl)) break;
                if (q is ILoopOperation lo && !(lo is IForEachLoopOperation fe && fe.Collection.DescendantsAndSelf().Contains(r)) || q is IAnonymousFunctionOperation)
                {
                    if (IsEnumerating(use)) repeated = true;
                    break;
                }
            }
        }
        var enumerations = uses.Count(IsEnumerating);
        var prefix = enumerations > 1 || repeated ? "double " : "";
        return $"{prefix}local {local.Name}{{{string.Join("; ", uses)}}}";
    }

    static string ParamUses(IMethodSymbol m, IParameterSymbol param, Body b, int depth, HashSet<string> visiting)
    {
        var id = Name(m) + "/" + param.Name;
        if (!visiting.Add(id)) return "recursive";
        var uses = new List<string>();
        foreach (var r in b.Root.DescendantsAndSelf().OfType<IParameterReferenceOperation>())
            if (SymbolEqualityComparer.Default.Equals(r.Parameter.OriginalDefinition, param.OriginalDefinition) || r.Parameter.Name == param.Name && r.Parameter.Ordinal == param.Ordinal)
                uses.Add(Consumer(r, b.Root, depth, visiting));
        visiting.Remove(id);
        var enumerations = uses.Count(IsEnumerating);
        return (enumerations > 1 ? "double " : "") + string.Join("; ", uses);
    }
}
