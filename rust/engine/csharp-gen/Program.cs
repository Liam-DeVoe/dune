// Generates the Rust skeleton of the engine from Treachery.Shared.
//
//   dotnet run -- [--glue] [--force] [--dry]
//   dotnet run -- --bots [--force]     the rust/bots skeleton (BotsGen.cs, rust/docs/bots-plan.md)
//
// Roslyn supplies declaration order, files, lines, signatures and a mutation
// analysis (which members need &mut). Reflection supplies the exact instance
// field list of every type and the System.Text.Json member order of every
// event. The output is written under rust/engine/src. Existing files are not
// overwritten unless --force is given: once porters edit a file, the
// generator must not touch it again. --glue overwrites only the fully generated
// files (events/mod.rs, log/mod.rs, prelude.rs, json_members.rs), which nobody edits.

using CsharpGen;

var force = args.Contains("--force");
var glue = args.Contains("--glue");
var dry = args.Contains("--dry");
var here = AppContext.BaseDirectory;
var root = Path.GetFullPath(Path.Combine(here, "../../../../../.."));   // repo root (rust/engine/csharp-gen/bin/Debug/net10.0)
if (!Directory.Exists(Path.Combine(root, "treachery-online", "Shared")))
    root = Path.GetFullPath(Path.Combine(Directory.GetCurrentDirectory(), "../../.."));
var shared = Path.Combine(root, "treachery-online", "Shared");
var outDir = Path.Combine(root, "rust", "engine", "src");

var model = new Model(shared);
model.Load();
var mc = Array.IndexOf(args, "--message-collections");
if (mc >= 0)
{
    MessageCollections.Write(model, Path.GetFullPath(args[mc + 1]));
    return;
}
var sb = Array.IndexOf(args, "--span-binding");
if (sb >= 0)
{
    SpanBinding.Write(model, Path.GetFullPath(args[sb + 1]));
    return;
}
var sc = Array.IndexOf(args, "--stored-event-collections");
if (sc >= 0)
{
    SpanBinding.WriteStored(model, Path.GetFullPath(args[sc + 1]));
    return;
}
var lq = Array.IndexOf(args, "--lazy-queries");
if (lq >= 0)
{
    LazyQueries.Write(model, Path.GetFullPath(args[lq + 1]));
    return;
}
if (args.Contains("--bots"))
{
    // The rust/bots skeleton (phase 2). Writes only files that don't exist yet, unless --force.
    Emitter.EmitBots(model, Path.Combine(root, "treachery-online", "Bots", "Treachery.Bots"), Path.Combine(root, "rust", "bots", "src"), force);
    return;
}
model.Analyze();
var gen = new Emitter(model, outDir, force, dry) { Glue = glue };
gen.EmitAll();
gen.Report(Path.Combine(root, "rust", "engine", "csharp-gen", "out"));
