using Microsoft.CodeAnalysis;
using Microsoft.CodeAnalysis.CSharp.Syntax;
using Microsoft.CodeAnalysis.Operations;

namespace CsharpGen;

// Lists every place in Shared where a value that is not a span is converted to
// Span<T> or ReadOnlySpan<T>, for rust/docs/span-binding.md.
//
//   dotnet run -- --span-binding ../tests/coverage/span_binding.txt
//
// C# 14 added first-class span conversions: an array (or a string) converts
// implicitly to a span, also as the receiver of an extension method. So
// `array.Contains(x)` and `array.SequenceEqual(y)` bind to MemoryExtensions
// rather than Enumerable, and a null array becomes an empty span instead of
// throwing ArgumentNullException. The binding is decided by the compiler, so
// this asks the semantic model which method each call binds to.
//
// Output, one tab-separated line per site:
//   <file>:<line>:<call text>   <Type.Method>   <operand type>   <operand text>
// The key (first column) is what span-binding.md rows cite. A call with two
// converted operands (SequenceEqual) is one line whose last column lists both.
public static class SpanBinding
{
    public static void Write(Model model, string outPath)
    {
        var sites = new SortedDictionary<string, (string Method, List<string> Types, List<string> Operands)>(StringComparer.Ordinal);

        static bool IsSpan(ITypeSymbol? t) =>
            t is INamedTypeSymbol { IsGenericType: true } n &&
            n.OriginalDefinition.ToDisplayString() is "System.Span<T>" or "System.ReadOnlySpan<T>";

        static string Text(SyntaxNode n) =>
            string.Join(" ", n.ToString().Split((char[])['\r', '\n', '\t'], StringSplitOptions.RemoveEmptyEntries).Select(s => s.Trim()));

        foreach (var (rel, tree) in model.Trees)
        {
            if (Model.Excluded.ContainsKey(rel)) continue;
            var sm = model.Comp.GetSemanticModel(tree);
            var ops = new List<IOperation>();
            foreach (var b in tree.GetRoot().DescendantNodes().Where(n =>
                         n is BaseMethodDeclarationSyntax or AccessorDeclarationSyntax
                             or ArrowExpressionClauseSyntax { Parent: PropertyDeclarationSyntax or IndexerDeclarationSyntax }
                             or EqualsValueClauseSyntax { Parent: VariableDeclaratorSyntax { Parent.Parent: FieldDeclarationSyntax } or PropertyDeclarationSyntax }
                             or PrimaryConstructorBaseTypeSyntax))
            {
                IOperation? root;
                try { root = sm.GetOperation(b); }
                catch (NullReferenceException)
                {
                    // The Roslyn bug on constructors of Location subclasses (see MessageCollections.cs).
                    sites[$"{rel}:{b.GetLocation().GetLineSpan().StartLinePosition.Line + 1}:UNANALYZED"] = ("Roslyn crashed on this member", [], []);
                    continue;
                }
                if (root != null) ops.AddRange(root.DescendantsAndSelf());
            }

            void Add(IOperation operand, IOperation at)
            {
                if (IsSpan(operand.Type)) return;
                // The call (or other operation) that consumes the span.
                IOperation site = at;
                for (var p = at.Parent; p != null; p = p.Parent)
                {
                    if (p is IInvocationOperation or IObjectCreationOperation) { site = p; break; }
                    if (p is IBlockOperation or IExpressionStatementOperation) break;
                }
                var method = site switch
                {
                    IInvocationOperation inv => inv.TargetMethod.ContainingType.ToDisplayString() + "." + inv.TargetMethod.Name,
                    IObjectCreationOperation oc => "new " + oc.Type?.ToDisplayString(),
                    _ => "?",
                };
                var line = site.Syntax.GetLocation().GetLineSpan().StartLinePosition.Line + 1;
                var key = $"{rel}:{line}:{Text(site.Syntax)}";
                if (!sites.TryGetValue(key, out var entry)) sites[key] = entry = (method, [], []);
                var text = Text(operand.Syntax);
                if (!entry.Operands.Contains(text))
                {
                    entry.Types.Add(operand.Type?.ToDisplayString() ?? "?");
                    entry.Operands.Add(text);
                }
            }

            foreach (var op in ops.Distinct())
            {
                switch (op)
                {
                    case IConversionOperation c when IsSpan(c.Type):
                        Add(c.Operand, c);
                        break;
                    // An argument whose conversion to a span parameter is recorded on the argument itself.
                    case IArgumentOperation a when IsSpan(a.Parameter?.Type) && a.Value is not IConversionOperation && !IsSpan(a.Value.Type):
                        Add(a.Value, a);
                        break;
                }
            }
        }

        // Cross-check against the assembly the real compiler built: every call into
        // MemoryExtensions in its IL must be one of the sites above.
        var analyzed = sites.Values.Count(s => s.Method.StartsWith("System.MemoryExtensions.", StringComparison.Ordinal));
        var compiled = CompiledMemoryExtensionsCalls(typeof(Treachery.Shared.Game).Assembly);
        if (compiled.Count != analyzed)
        {
            Console.Error.WriteLine($"Treachery.Shared.dll has {compiled.Count} calls into MemoryExtensions, Roslyn found {analyzed}:");
            foreach (var c in compiled) Console.Error.WriteLine("  " + c);
            Environment.Exit(1);
        }

        File.WriteAllLines(outPath, sites.Select(s => $"{s.Key}\t{s.Value.Method}\t{string.Join(", ", s.Value.Types)}\t{string.Join(", ", s.Value.Operands)}"));
        Console.WriteLine($"{sites.Count} span conversion sites written to {outPath} ({compiled.Count} MemoryExtensions calls in the IL):");
        foreach (var c in compiled) Console.WriteLine("  " + c);
    }

    // Lists every assignment that stores a collection-typed member of an event,
    // or of an object reached through one (which JSON can set to null), into a field or property of an object that is
    // not an event, i.e. game state. Also lists assignments of a collection-typed
    // parameter to such a field, because a caller may pass an event member on.
    //
    //   dotnet run -- --stored-event-collections ../tests/coverage/stored_event_collections.txt
    //
    // Output: <file>:<line>:<assignment text>   <target Type.Member>   <value type>   event|parameter
    public static void WriteStored(Model model, string outPath)
    {
        var lines = new SortedSet<string>(StringComparer.Ordinal);
        var enumerable = model.Comp.GetSpecialType(SpecialType.System_Collections_IEnumerable);
        bool IsCollection(ITypeSymbol? t) =>
            t != null && t.SpecialType != SpecialType.System_String &&
            (t.TypeKind == TypeKind.Array || t.AllInterfaces.Contains(enumerable, SymbolEqualityComparer.Default));
        bool IsEvent(ITypeSymbol? t)
        {
            for (var b = t; b != null; b = b.BaseType)
                if (SymbolEqualityComparer.Default.Equals(b, model.GameEventSym)) return true;
            return false;
        }
        static string Text(SyntaxNode n) =>
            string.Join(" ", n.ToString().Split((char[])['\r', '\n', '\t'], StringSplitOptions.RemoveEmptyEntries).Select(s => s.Trim()));
        static IOperation Strip(IOperation o)
        {
            while (o is IConversionOperation c) o = c.Operand;
            return o;
        }
        // The leaves a value can come from: both arms of ?: and ??.
        static IEnumerable<IOperation> Leaves(IOperation o)
        {
            o = Strip(o);
            if (o is IConditionalOperation c) return Leaves(c.WhenTrue).Concat(c.WhenFalse != null ? Leaves(c.WhenFalse) : []);
            if (o is ICoalesceOperation co) return Leaves(co.Value).Concat(Leaves(co.WhenNull));
            return [o];
        }

        // A member of an event, or a member reached from one (`Settings.InitialRules`).
        bool RootedInEvent(IMemberReferenceOperation m)
        {
            for (IOperation? o = m; o is IMemberReferenceOperation r; o = r.Instance is { } i ? Strip(i) : null)
                if (r.Member is IFieldSymbol or IPropertySymbol && IsEvent(r.Member.ContainingType)) return true;
            return false;
        }

        foreach (var (rel, tree) in model.Trees)
        {
            if (Model.Excluded.ContainsKey(rel)) continue;
            var sm = model.Comp.GetSemanticModel(tree);
            foreach (var node in tree.GetRoot().DescendantNodes().OfType<Microsoft.CodeAnalysis.CSharp.Syntax.AssignmentExpressionSyntax>())
            {
                IOperation? op;
                try { op = sm.GetOperation(node); } catch (NullReferenceException) { continue; }
                if (op is not ISimpleAssignmentOperation { Target: IMemberReferenceOperation target } assign) continue;
                if (target is not (IFieldReferenceOperation or IPropertyReferenceOperation)) continue;
                if (IsEvent(target.Member.ContainingType) || !IsCollection(target.Type)) continue;
                foreach (var leaf in Leaves(assign.Value))
                {
                    var kind = leaf switch
                    {
                        IMemberReferenceOperation { Member: IFieldSymbol or IPropertySymbol } m when IsCollection(m.Type) && RootedInEvent(m) => "event",
                        IParameterReferenceOperation p when IsCollection(p.Type) => "parameter",
                        _ => null,
                    };
                    if (kind == null) continue;
                    var line = node.GetLocation().GetLineSpan().StartLinePosition.Line + 1;
                    lines.Add($"{rel}:{line}:{Text(node)}\t{target.Member.ContainingType.Name}.{target.Member.Name}\t{leaf.Type?.ToDisplayString()}\t{kind}");
                }
            }
        }
        File.WriteAllLines(outPath, lines);
        Console.WriteLine($"{lines.Count} stored event collection sites written to {outPath}");
    }

    // "Type.Method -> MemoryExtensions.X" for every call or callvirt into System.MemoryExtensions.
    static List<string> CompiledMemoryExtensionsCalls(System.Reflection.Assembly asm)
    {
        var codes = new Dictionary<short, System.Reflection.Emit.OpCode>();
        foreach (var f in typeof(System.Reflection.Emit.OpCodes).GetFields())
            if (f.GetValue(null) is System.Reflection.Emit.OpCode oc) codes[oc.Value] = oc;

        var result = new List<string>();
        const System.Reflection.BindingFlags all = System.Reflection.BindingFlags.Public | System.Reflection.BindingFlags.NonPublic |
                                                   System.Reflection.BindingFlags.Instance | System.Reflection.BindingFlags.Static | System.Reflection.BindingFlags.DeclaredOnly;
        foreach (var type in asm.GetTypes())
        foreach (var m in type.GetMethods(all).Cast<System.Reflection.MethodBase>().Concat(type.GetConstructors(all)))
        {
            var il = m.GetMethodBody()?.GetILAsByteArray();
            if (il == null) continue;
            for (var i = 0; i < il.Length;)
            {
                short v = il[i] == 0xFE ? (short)(0xFE00 | il[i + 1]) : il[i];
                var oc = codes[v];
                i += oc.Size;
                var operand = oc.OperandType switch
                {
                    System.Reflection.Emit.OperandType.InlineNone => 0,
                    System.Reflection.Emit.OperandType.ShortInlineBrTarget or System.Reflection.Emit.OperandType.ShortInlineI or System.Reflection.Emit.OperandType.ShortInlineVar => 1,
                    System.Reflection.Emit.OperandType.InlineVar => 2,
                    System.Reflection.Emit.OperandType.InlineI8 or System.Reflection.Emit.OperandType.InlineR => 8,
                    System.Reflection.Emit.OperandType.InlineSwitch => 4 + 4 * BitConverter.ToInt32(il, i),
                    _ => 4,
                };
                if ((oc == System.Reflection.Emit.OpCodes.Call || oc == System.Reflection.Emit.OpCodes.Callvirt) &&
                    m.Module.ResolveMethod(BitConverter.ToInt32(il, i), type.IsGenericType ? type.GetGenericArguments() : null,
                        m.IsGenericMethod ? m.GetGenericArguments() : null) is { DeclaringType.FullName: "System.MemoryExtensions" } target)
                    result.Add($"{type.FullName}.{m.Name} -> MemoryExtensions.{target.Name}");
                i += operand;
            }
        }
        result.Sort(StringComparer.Ordinal);
        return result;
    }
}
