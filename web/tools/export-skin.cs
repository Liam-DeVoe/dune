#:project ../../treachery-online/Shared/Treachery.Shared.csproj
#:property JsonSerializerIsReflectionEnabledByDefault=true
#:property NoWarn=IL2026;IL3050

// Writes the C# DefaultSkin as JSON (the same format as a custom skin file, Skin.SkinToString)
// to the path given, for the web client's skin module (web/src/skin.ts).
//
//   DOTNET_ROOT=$HOME/dotnet10 PATH=$HOME/dotnet10:$PATH dotnet run web/tools/export-skin.cs -- web/public/skin/default.json

using Treachery.Shared;

File.WriteAllText(args[0], DefaultSkin.Default.SkinToString());
Console.WriteLine($"wrote {args[0]}");
