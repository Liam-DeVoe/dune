using System.Globalization;

CultureInfo.DefaultThreadCurrentCulture = CultureInfo.InvariantCulture;
CultureInfo.CurrentCulture = CultureInfo.InvariantCulture;

switch (args.Length > 0 ? args[0] : "")
{
    case "skin-json":
        File.WriteAllText(args[1], DefaultSkin.Default.SkinToString());
        break;
    case "static":
        Static.Run(args[1]);
        break;
    case "traces":
        Traces.Run(args[1], args[2]);
        break;
    case "load":
        Load.Run(args[1]);
        break;
    default:
        Console.Error.WriteLine("usage: skin-json <out> | static <out> | format <out> | traces <in> <out>");
        Environment.Exit(2);
        break;
}
