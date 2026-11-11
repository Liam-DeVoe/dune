using Microsoft.CodeAnalysis;
using Microsoft.CodeAnalysis.Operations;

namespace CsharpGen;

// Lists every place in Shared where a collection object can end up inside a
// message or log record, for rust/docs/live-lists.md (hazards 4.5).
//
//   dotnet run -- --message-collections ../tests/coverage/message_collections.txt
//
// A site is one of:
//   - a collection (anything IEnumerable except string) converted to `object`,
//     which is how it enters a `params object?[]` expression or an `object` field
//     such as BattleFact.Cause;
//   - an array passed in normal form to a `params object?[]` parameter, which
//     becomes the expression's element array itself;
//   - a collection passed to a constructor of, or assigned to a property of, a
//     record from Model/Message.cs or Model/Log/.
// The output is one tab-separated line per site: key, enclosing call, C# type, argument text.
// The key is "<file>:<line>:<argument text>", which is what live-lists.md rows cite.
public static class MessageCollections
{
    public static void Write(Model model, string outPath)
    {
        var lines = new SortedSet<string>(StringComparer.Ordinal);
        var enumerable = model.Comp.GetSpecialType(SpecialType.System_Collections_IEnumerable);

        // A type parameter could be instantiated with a collection, so it counts too.
        bool IsCollection(ITypeSymbol? t) =>
            t is ITypeParameterSymbol || t != null && t.SpecialType != SpecialType.System_String &&
            (t.TypeKind == TypeKind.Array || SymbolEqualityComparer.Default.Equals(t, enumerable) || t.AllInterfaces.Contains(enumerable, SymbolEqualityComparer.Default));

        bool IsMessageRecord(ITypeSymbol? t)
        {
            if (t == null) return false;
            var file = t.Locations.FirstOrDefault()?.SourceTree is { } tree ? model.Rel(tree) : "";
            return file == "Model/Message.cs" || file.StartsWith("Model/Log/", StringComparison.Ordinal);
        }

        foreach (var (rel, tree) in model.Trees)
        {
            if (Model.Excluded.ContainsKey(rel)) continue;
            var sm = model.Comp.GetSemanticModel(tree);
            // Operation trees of every member body; lambdas and local functions are inside them.
            var bodies = tree.GetRoot().DescendantNodes().Where(n =>
                n is Microsoft.CodeAnalysis.CSharp.Syntax.BaseMethodDeclarationSyntax or Microsoft.CodeAnalysis.CSharp.Syntax.AccessorDeclarationSyntax
                    or Microsoft.CodeAnalysis.CSharp.Syntax.ArrowExpressionClauseSyntax { Parent: Microsoft.CodeAnalysis.CSharp.Syntax.PropertyDeclarationSyntax or Microsoft.CodeAnalysis.CSharp.Syntax.IndexerDeclarationSyntax }
                    or Microsoft.CodeAnalysis.CSharp.Syntax.EqualsValueClauseSyntax { Parent: Microsoft.CodeAnalysis.CSharp.Syntax.VariableDeclaratorSyntax { Parent.Parent: Microsoft.CodeAnalysis.CSharp.Syntax.FieldDeclarationSyntax } or Microsoft.CodeAnalysis.CSharp.Syntax.PropertyDeclarationSyntax }
                    or Microsoft.CodeAnalysis.CSharp.Syntax.PrimaryConstructorBaseTypeSyntax);
            var ops = new List<IOperation>();
            foreach (var b in bodies)
            {
                IOperation? root;
                try { root = sm.GetOperation(b); }
                catch (NullReferenceException)
                {
                    // A Roslyn bug on a few constructors of Location subclasses. Listed so they are checked by hand.
                    lines.Add($"{rel}:{b.GetLocation().GetLineSpan().StartLinePosition.Line + 1}:UNANALYZED\tRoslyn crashed on this member\t");
                    continue;
                }
                if (root != null) ops.AddRange(root.DescendantsAndSelf());
            }
            foreach (var op in ops.Distinct())
            {

                string Text(SyntaxNode n) => string.Join(" ", n.ToString().Split((char[])['\r', '\n', '\t'], StringSplitOptions.RemoveEmptyEntries).Select(s => s.Trim()));
                string Enclosing(IOperation o)
                {
                    for (var p = o.Parent; p != null; p = p.Parent)
                    {
                        if (p is IInvocationOperation inv) return inv.TargetMethod.ContainingType.Name + "." + inv.TargetMethod.Name;
                        if (p is IObjectCreationOperation oc) return "new " + oc.Type?.Name;
                        if (p is ISimpleAssignmentOperation { Target: IPropertyReferenceOperation pr }) return pr.Property.ContainingType.Name + "." + pr.Property.Name;
                        if (p is IReturnOperation) return "return";
                    }
                    return "?";
                }
                void Add(IOperation value, SyntaxNode at, string kind)
                {
                    var line = at.GetLocation().GetLineSpan().StartLinePosition.Line + 1;
                    lines.Add($"{rel}:{line}:{Text(at)}\t{kind} {Enclosing(value)}\t{value.Type?.ToDisplayString()}");
                }

                switch (op)
                {
                    // `x == null` and `x != null` convert to object too; they store nothing.
                    case IConversionOperation c
                        when c.Type?.SpecialType == SpecialType.System_Object && IsCollection(c.Operand.Type) && c.Parent is not IBinaryOperation:
                        Add(c.Operand, c.Operand.Syntax, "object");
                        break;
                    case IArgumentOperation { ArgumentKind: ArgumentKind.Explicit, Parameter.IsParams: true } a
                        when a.Parameter.Type is IArrayTypeSymbol { ElementType.SpecialType: SpecialType.System_Object } && IsCollection(a.Value.Type):
                        Add(a.Value, a.Value.Syntax, "params-array");
                        break;
                    case IArgumentOperation { ArgumentKind: ArgumentKind.Explicit, Parameter.IsParams: false } a when IsMessageRecord(a.Parameter.ContainingType) && IsCollection(a.Value.Type):
                        Add(a.Value, a.Value.Syntax, "record");
                        break;
                    case ISimpleAssignmentOperation { Target: IPropertyReferenceOperation pr } s
                        when IsMessageRecord(pr.Property.ContainingType) && IsCollection(s.Value.Type):
                        Add(s.Value, s.Value.Syntax, "record");
                        break;
                }
            }
        }

        File.WriteAllLines(outPath, lines);
        Console.WriteLine($"{lines.Count} message collection sites written to {outPath}");
    }
}
