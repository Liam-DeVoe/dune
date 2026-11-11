using Microsoft.CodeAnalysis;
using Microsoft.CodeAnalysis.CSharp.Syntax;
using Microsoft.CodeAnalysis.Operations;

namespace CsharpGen;

// Decides, for every method and accessor in Shared, whether it mutates its own
// object, other game state, reads the owning Game, or passes `this` as a value.
// Conservative: anything unclear counts as a mutation.
public sealed partial class Model
{
    readonly Dictionary<IMethodSymbol, Flags> _direct = new(SymbolEqualityComparer.Default);
    readonly Dictionary<IMethodSymbol, List<(IMethodSymbol callee, Root root, (Root root, int param)[] args)>> _edges = new(SymbolEqualityComparer.Default);
    readonly Dictionary<ISymbol, HashSet<ISymbol>> _setterWrites = new(SymbolEqualityComparer.Default);
    public readonly HashSet<ISymbol> GameRefMembers = new(SymbolEqualityComparer.Default);
    readonly List<INamedTypeSymbol> _allSourceTypes = [];

    static readonly HashSet<string> Mutators =
    [
        "Add", "AddRange", "Remove", "RemoveAt", "RemoveAll", "RemoveWhere", "RemoveRange", "Clear", "Insert", "InsertRange",
        "Push", "Pop", "Enqueue", "Dequeue", "TryDequeue", "TryPop", "Sort", "Reverse", "TryAdd", "UnionWith", "ExceptWith", "IntersectWith",
        "SymmetricExceptWith", "AddLast", "AddFirst", "RemoveFirst", "RemoveLast", "TrimExcess", "EnsureCapacity", "CopyTo", "Set", "Fill"
    ];

    static readonly HashSet<string> FreshProducers =
    [
        "Express", "ExpressTo", "ExpressIf", "WithRoute", "WithLog", "WithPlans", "WithFact", "AsHeading",
        "ToList", "ToArray", "ToDictionary", "ToHashSet", "CloneList", "CloneObjectDictionary", "CloneEnumDictionary", "MemberwiseClone", "Clone",
        "ToLookup", "Split", "ToString", "Substring", "Replace"
    ];

    bool ExcludedField(ISymbol f)
    {
        var t = f.ContainingType?.Name;
        if (t == "Map" && f.Name is "_neighbourCache" or "_neighbourCacheTimestamp") return true;
        if (t == "LoggedRandom") return true;
        if (t == "Message" && f.IsStatic) return true;
        return false;
    }

    bool ExcludedMethod(IMethodSymbol m) => m.ContainingType?.Name == "LoggedRandom";

    public void Analyze()
    {
        // Members that point at the owning game (dereferencing them is "uses game").
        foreach (var t in new[] { "Player", "PlayerSequence", "GameEvent" })
        {
            var s = FindType(t);
            foreach (var m in s.GetMembers())
                if (m is IFieldSymbol { IsImplicitlyDeclared: false } f && f.Type.Name == "Game" || m is IPropertySymbol p && p.Type.Name == "Game")
                    GameRefMembers.Add(m);
            if (t.EndsWith("GameEvent")) GameRefMembers.Add(s.GetMembers("Player").First());
        }

        foreach (var tree in Comp.SyntaxTrees)
        {
            var sm = Comp.GetSemanticModel(tree);
            foreach (var decl in tree.GetRoot().DescendantNodes().OfType<TypeDeclarationSyntax>())
                _allSourceTypes.Add((INamedTypeSymbol)sm.GetDeclaredSymbol(decl)!);
        }

        // Repeated so that parameter escapes discovered late reach earlier callers.
        for (var pass = 0; pass < 3; pass++)
        {
        if (pass > 0) { _direct.Clear(); _edges.Clear(); _setterWrites.Clear(); CellMembers.Clear(); }
        foreach (var tree in Comp.SyntaxTrees)
        {
            var sm = Comp.GetSemanticModel(tree);
            foreach (var node in tree.GetRoot().DescendantNodes())
            {
                IMethodSymbol? m = null;
                SyntaxNode? body = null;
                switch (node)
                {
                    case BaseMethodDeclarationSyntax md:
                        m = sm.GetDeclaredSymbol(md) as IMethodSymbol;
                        body = (SyntaxNode?)md.Body ?? md.ExpressionBody;
                        if (md is ConstructorDeclarationSyntax cd && cd.Initializer != null)
                            try { AnalyzeBody(m!, cd.Initializer, sm); } catch (Exception) { Warnings.Add($"analysis crashed in ctor initializer of {m!.ContainingType.Name}"); }
                        break;
                    case AccessorDeclarationSyntax ad:
                        m = sm.GetDeclaredSymbol(ad) as IMethodSymbol;
                        body = (SyntaxNode?)ad.Body ?? ad.ExpressionBody;
                        if (body == null && m != null)
                        {
                            // auto accessor
                            var f = Get(m);
                            if (m.MethodKind == MethodKind.PropertySet) f.Self = true;
                        }
                        break;
                    case PropertyDeclarationSyntax { ExpressionBody: not null } pd:
                        m = (sm.GetDeclaredSymbol(pd) as IPropertySymbol)?.GetMethod;
                        body = pd.ExpressionBody;
                        break;
                    case IndexerDeclarationSyntax { ExpressionBody: not null } idx:
                        m = (sm.GetDeclaredSymbol(idx) as IPropertySymbol)?.GetMethod;
                        body = idx.ExpressionBody;
                        break;
                }
                if (m == null || body == null) continue;
                try { AnalyzeBody(m, body, sm); }
                catch (Exception e) { Warnings.Add($"analysis crashed in {m.ContainingType.Name}.{m.Name}: {e.GetType().Name}"); Get(m).Game = true; Get(m).Self = true; }
            }
        }
        }

        // Fixpoint.
        foreach (var m in _direct.Keys) FlagsOf[m] = Copy(_direct[m]);
        bool changed;
        do
        {
            changed = false;
            foreach (var (m, edges) in _edges)
            {
                var f = FlagsOf.TryGetValue(m, out var ff) ? ff : FlagsOf[m] = new Flags();
                var before = Copy(f);
                foreach (var (c, r, args) in edges)
                {
                    foreach (var target in Expand(c))
                    {
                        if (!FlagsOf.TryGetValue(target, out var cf)) continue;
                        foreach (var pi in cf.ParamMut)
                        {
                            if (pi >= args.Length) continue;
                            var (ar, ap) = args[pi];
                            if (ap >= 0) f.ParamMut.Add(ap);
                            else if (ar == Root.This) f.Self = true;
                            else if (ar is Root.Other or Root.ThisGame) { if (!f.Game) f.Why = $"{target.Name} mutates argument {pi}"; f.Game = true; }
                        }
                        switch (r)
                        {
                            case Root.This:
                                if (!f.Game && cf.Game) f.Why = $"{target.ContainingType.Name}.{target.Name} on this";
                                f.Self |= cf.Self; f.Game |= cf.Game; f.UsesGame |= cf.UsesGame; f.NeedsRc |= cf.NeedsRc; f.UsesVirtual |= cf.UsesVirtual;
                                break;
                            case Root.ThisGame:
                                if (!f.Game && (cf.Self | cf.Game)) f.Why = $"{target.ContainingType.Name}.{target.Name} via game";
                                f.Game |= cf.Self | cf.Game; f.UsesGame = true;
                                break;
                            case Root.Other:
                                if (!f.Game && (cf.Self | cf.Game)) f.Why = $"{target.ContainingType.Name}.{target.Name} on other";
                                f.Game |= cf.Self | cf.Game;
                                break;
                            default:
                                if (!f.Game && cf.Game) f.Why = $"{target.ContainingType.Name}.{target.Name} static/fresh";
                                f.Game |= cf.Game;
                                break;
                        }
                    }
                }
                if (!f.Same(before)) changed = true;
            }
        } while (changed);

        // Written outside Shared: the server's ValidateAndExecute (Server/GameHub_GameEvents.cs) sets
        // e.Time after Execute, when the event is already in History.
        CellMembers.Add(FindType("GameEvent").GetMembers("Time").Single().OriginalDefinition);

        // Event fields written after construction: properties pull in what their setters write.
        bool grew;
        do
        {
            grew = false;
            foreach (var p in CellMembers.ToList())
                if (_setterWrites.TryGetValue(p, out var writes))
                    foreach (var w in writes)
                        grew |= CellMembers.Add(w);
        } while (grew);
    }

    public INamedTypeSymbol FindType(string name)
    {
        foreach (var tree in Comp.SyntaxTrees)
        {
            var sm = Comp.GetSemanticModel(tree);
            foreach (var decl in tree.GetRoot().DescendantNodes().OfType<TypeDeclarationSyntax>())
                if (decl.Identifier.Text == name) return (INamedTypeSymbol)sm.GetDeclaredSymbol(decl)!;
        }
        throw new Exception("type not found: " + name);
    }

    readonly Dictionary<IMethodSymbol, List<IMethodSymbol>> _expandCache = new(SymbolEqualityComparer.Default);

    // A call to a virtual, abstract or interface member reaches every override.
    List<IMethodSymbol> Expand(IMethodSymbol m)
    {
        if (_expandCache.TryGetValue(m, out var r)) return r;
        r = [m];
        if (m.IsAbstract || m.IsVirtual || m.ContainingType.TypeKind == TypeKind.Interface)
        {
            foreach (var t in _allSourceTypes)
            {
                if (m.ContainingType.TypeKind == TypeKind.Interface)
                {
                    if (t.FindImplementationForInterfaceMember(m) is IMethodSymbol impl) r.Add(impl.OriginalDefinition);
                    continue;
                }
                foreach (var cand in t.GetMembers(m.Name).OfType<IMethodSymbol>())
                    for (var o = cand.OverriddenMethod; o != null; o = o.OverriddenMethod)
                        if (SymbolEqualityComparer.Default.Equals(o.OriginalDefinition, m.OriginalDefinition)) { r.Add(cand); break; }
                if (m.AssociatedSymbol is IPropertySymbol prop)
                    foreach (var cp in t.GetMembers(prop.Name).OfType<IPropertySymbol>())
                        for (var o = cp.OverriddenProperty; o != null; o = o.OverriddenProperty)
                            if (SymbolEqualityComparer.Default.Equals(o.OriginalDefinition, prop.OriginalDefinition))
                            {
                                var acc = m.MethodKind == MethodKind.PropertyGet ? cp.GetMethod : cp.SetMethod;
                                if (acc != null) r.Add(acc);
                                break;
                            }
            }
        }
        _expandCache[m] = r;
        return r;
    }

    static Flags Copy(Flags f) => new() { Self = f.Self, Game = f.Game, UsesGame = f.UsesGame, NeedsRc = f.NeedsRc, UsesVirtual = f.UsesVirtual, Why = f.Why, ParamMut = [.. f.ParamMut] };

    static bool IsCollection(ITypeSymbol t) =>
        t is IArrayTypeSymbol || t is INamedTypeSymbol { IsGenericType: true } n && n.ContainingNamespace?.ToDisplayString().StartsWith("System.Collections") == true;

    // The collection parameter a receiver directly is, or -1.
    static int ParamOf(IOperation? op)
    {
        while (op is IConversionOperation c) op = c.Operand;
        return op is IParameterReferenceOperation pr && IsCollection(pr.Parameter.Type) && pr.Parameter.ContainingSymbol is IMethodSymbol { MethodKind: not MethodKind.AnonymousFunction and not MethodKind.LambdaMethod }
            ? pr.Parameter.Ordinal : -1;
    }

    Flags Get(IMethodSymbol m)
    {
        m = (IMethodSymbol)m.OriginalDefinition;
        if (!_direct.TryGetValue(m, out var f)) _direct[m] = f = new Flags();
        return f;
    }

    void Edge(IMethodSymbol from, IMethodSymbol to, Root r, (Root, int)[]? args = null)
    {
        from = (IMethodSymbol)from.OriginalDefinition;
        to = (IMethodSymbol)(to.ReducedFrom ?? to).OriginalDefinition;
        if (!_edges.TryGetValue(from, out var l)) _edges[from] = l = [];
        l.Add((to, r, args ?? []));
    }

    static bool InSource(ISymbol s) => s.Locations.Any(l => l.IsInSource);

    void AnalyzeBody(IMethodSymbol m, SyntaxNode body, SemanticModel sm)
    {
        if (ExcludedMethod(m)) { Get(m); return; }
        var f = Get(m);
        var op = sm.GetOperation(body);
        if (op == null) return;
        var fresh = new HashSet<ILocalSymbol>(SymbolEqualityComparer.Default);
        var all = op.DescendantsAndSelf().ToList();
        foreach (var d in all.OfType<IVariableDeclaratorOperation>())
            if (d.Initializer?.Value is { } v && R(v, fresh) == Root.Fresh) fresh.Add(d.Symbol);

        var isCtor = m.MethodKind == MethodKind.Constructor;
        IPropertySymbol? setterOf = m.MethodKind == MethodKind.PropertySet ? m.AssociatedSymbol as IPropertySymbol : null;

        IOperation? cur = null;
        void Mut(Root r)
        {
            if (r == Root.This) f.Self = true;
            else if (r is Root.ThisGame or Root.Other) { if (!f.Game) f.Why = cur?.Syntax.ToString(); f.Game = true; }
        }

        void RecordCell(ISymbol member, Root r)
        {
            if (!IsEventType(member.ContainingType) || r == Root.Fresh) return;
            if (isCtor && r == Root.This) return;
            if (setterOf != null && r == Root.This)
            {
                if (!_setterWrites.TryGetValue(setterOf, out var set)) _setterWrites[setterOf] = set = new(SymbolEqualityComparer.Default);
                set.Add(member.OriginalDefinition);
                return;
            }
            CellMembers.Add(member.OriginalDefinition);
        }

        void Target(IOperation t)
        {
            switch (t)
            {
                case IFieldReferenceOperation fr:
                    if (ExcludedField(fr.Field)) return;
                    if (fr.Instance is IInstanceReferenceOperation { ReferenceKind: InstanceReferenceKind.ImplicitReceiver }) return;
                    {
                        var r = fr.Field.IsStatic ? Root.Static : R(fr.Instance!, fresh);
                        Mut(r);
                        RecordCell(fr.Field, r);
                    }
                    return;
                case IPropertyReferenceOperation pr:
                    if (pr.Instance is IInstanceReferenceOperation { ReferenceKind: InstanceReferenceKind.ImplicitReceiver }) return;
                    {
                        var r = pr.Property.IsStatic ? Root.Static : R(pr.Instance!, fresh);
                        Mut(r);
                        if (!pr.Property.IsIndexer) RecordCell(pr.Property, r);
                        else if (pr.Instance != null) RecordReceiver(pr.Instance);
                        if (pr.Property.SetMethod != null && InSource(pr.Property)) Edge(m, pr.Property.SetMethod, r);
                    }
                    return;
                case IArrayElementReferenceOperation ar:
                    Mut(R(ar.ArrayReference, fresh));
                    RecordReceiver(ar.ArrayReference);
                    return;
                case IParameterReferenceOperation { Parameter.RefKind: RefKind.Ref or RefKind.Out }:
                    return; // a `&mut T` parameter in Rust
                case ITupleOperation tu:
                    foreach (var e in tu.Elements) Target(e);
                    return;
            }
        }

        void RecordReceiver(IOperation recv)
        {
            switch (recv)
            {
                case IFieldReferenceOperation fr when !fr.Field.IsStatic:
                    RecordCell(fr.Field, R(fr.Instance!, fresh));
                    break;
                case IPropertyReferenceOperation pr when !pr.Property.IsStatic && !pr.Property.IsIndexer:
                    RecordCell(pr.Property, R(pr.Instance!, fresh));
                    break;
                case IConversionOperation c:
                    RecordReceiver(c.Operand);
                    break;
            }
        }

        foreach (var o in all)
        {
            cur = o;
            switch (o)
            {
                case IAssignmentOperation a when a is not IDeconstructionAssignmentOperation:
                    Target(a.Target);
                    break;
                case IDeconstructionAssignmentOperation da:
                    Target(da.Target);
                    break;
                case IIncrementOrDecrementOperation inc:
                    Target(inc.Target);
                    break;
                case IInvocationOperation inv:
                {
                    var tm = inv.TargetMethod;
                    if (ExcludedMethod(tm)) break;
                    var r = inv.Instance == null ? Root.Static : R(inv.Instance, fresh);
                    if (InSource(tm.OriginalDefinition) || tm.ReducedFrom != null && InSource(tm.ReducedFrom))
                    {
                        var args = inv.Arguments.OrderBy(a => a.Parameter?.Ordinal ?? 0).Select(a => (R(a.Value, fresh), ParamOf(a.Value))).ToArray();
                        Edge(m, tm, r, args);
                        if (tm.Name is "Validate" or "ExecuteConcreteEvent" or "GetMessage" or "GetShortMessage" && r == Root.This && IsEventType(m.ContainingType))
                            f.UsesVirtual = true;
                    }
                    else if (tm.Name == "GetType" && r == Root.This && IsEventType(m.ContainingType))
                    {
                        f.UsesVirtual = true;
                    }
                    else if (Mutators.Contains(tm.Name) && inv.Instance is IFieldReferenceOperation efr && ExcludedField(efr.Field))
                    {
                        // interior mutability in Rust (Map's neighbour cache)
                    }
                    else if (Mutators.Contains(tm.Name) && inv.Instance != null && ParamOf(inv.Instance) >= 0)
                    {
                        f.ParamMut.Add(ParamOf(inv.Instance));
                    }
                    else if (Mutators.Contains(tm.Name) && inv.Instance != null && r != Root.Fresh)
                    {
                        Mut(r);
                        RecordReceiver(inv.Instance);
                    }
                    break;
                }
                case IPropertyReferenceOperation pr:
                {
                    if (pr.Property.IsStatic || pr.Instance == null) { if (pr.Property.GetMethod != null && InSource(pr.Property)) Edge(m, pr.Property.GetMethod, Root.Static); break; }
                    var r = R(pr.Instance, fresh);
                    if (GameRefMembers.Contains(pr.Property.OriginalDefinition) && r == Root.This) f.UsesGame = true;
                    if (pr.Property.GetMethod != null && InSource(pr.Property)) Edge(m, pr.Property.GetMethod, r);
                    break;
                }
                case IFieldReferenceOperation fr:
                {
                    if (!fr.Field.IsStatic && fr.Instance != null && GameRefMembers.Contains(fr.Field.OriginalDefinition) && R(fr.Instance, fresh) == Root.This) f.UsesGame = true;
                    break;
                }
                case IInstanceReferenceOperation ir when ir.ReferenceKind == InstanceReferenceKind.ContainingTypeInstance:
                {
                    var p = ir.Parent;
                    var asReceiver = p is IMemberReferenceOperation mr && mr.Instance == ir || p is IInvocationOperation iv && iv.Instance == ir;
                    if (!asReceiver && Escapes(ir))
                    {
                        f.NeedsRc = true;
                        if (IsEventType(m.ContainingType)) f.UsesVirtual = true;
                    }
                    break;
                }
                case IParameterReferenceOperation prm when IsConcreteEvent(prm.Parameter.Type):
                {
                    var p = prm.Parent;
                    var asReceiver = p is IMemberReferenceOperation mr && mr.Instance == prm || p is IInvocationOperation iv && iv.Instance == prm;
                    if (!asReceiver && Escapes(prm)) EscapingParams.Add(prm.Parameter.OriginalDefinition);
                    break;
                }
            }
        }
    }

    /// Parameters of a concrete event type that the method stores or upcasts (they need `&Rc<T>`).
    public readonly HashSet<ISymbol> EscapingParams = new(SymbolEqualityComparer.Default);

    // Whether an event reference (`this` or a parameter) is used in a way that needs the Rc:
    // stored, returned, upcast, or passed on to a parameter that needs it.
    bool Escapes(IOperation op)
    {
        var p = op.Parent;
        while (p is IConversionOperation c && !IsEventAbstraction(c.Type) && IsConcreteEvent(c.Type)) { op = p; p = p.Parent; }
        switch (p)
        {
            case IBinaryOperation: return false;                       // == / != : reference comparison
            case IIsPatternOperation or IIsTypeOperation: return false;
            case IArgumentOperation a when a.Parameter != null && IsConcreteEvent(a.Parameter.Type):
                return EscapingParams.Contains(a.Parameter.OriginalDefinition);
            case IArgumentOperation a when a.Parameter != null && a.Parameter.Type.Name == "IEnumerable":
                return true;
            case IConditionalAccessOperation ca when ca.Operation == op: return false;
            default: return true;
        }
    }

    bool IsEventAbstraction(ITypeSymbol? t) => t != null && (t.Name is "GameEvent" or "PassableGameEvent" or "PlacementEvent" || t.TypeKind == TypeKind.Interface);

    Root R(IOperation? op, HashSet<ILocalSymbol> fresh)
    {
        switch (op)
        {
            case null: return Root.Other;
            case IInstanceReferenceOperation ir:
                return ir.ReferenceKind == InstanceReferenceKind.ImplicitReceiver ? Root.Fresh : Root.This;
            case IFieldReferenceOperation fr:
            {
                if (fr.Field.IsStatic) return Root.Static;
                var r = R(fr.Instance, fresh);
                if (r == Root.This && GameRefMembers.Contains(fr.Field.OriginalDefinition)) return Root.ThisGame;
                return r;
            }
            case IPropertyReferenceOperation pr:
            {
                if (pr.Property.IsStatic) return Root.Static;
                var r = R(pr.Instance, fresh);
                if (r == Root.This && GameRefMembers.Contains(pr.Property.OriginalDefinition)) return Root.ThisGame;
                return r;
            }
            case IInvocationOperation inv:
                if (FreshProducers.Contains(inv.TargetMethod.Name)) return Root.Fresh;
                return Root.Other;
            case ILocalReferenceOperation lr:
                return fresh.Contains(lr.Local) ? Root.Fresh : Root.Other;
            case IObjectCreationOperation or IArrayCreationOperation or ICollectionExpressionOperation or IAnonymousObjectCreationOperation or ITypeParameterObjectCreationOperation:
                return Root.Fresh;
            case IConversionOperation c:
                return R(c.Operand, fresh);
            case IArrayElementReferenceOperation ae:
                return R(ae.ArrayReference, fresh) == Root.This ? Root.This : Root.Other;
            case IConditionalAccessInstanceOperation ci:
            {
                IOperation child = ci;
                for (var p = ci.Parent; p != null; child = p, p = p.Parent)
                    if (p is IConditionalAccessOperation ca && ca.WhenNotNull == child) return R(ca.Operation, fresh);
                return Root.Other;
            }
            default:
                return Root.Other;
        }
    }
}
