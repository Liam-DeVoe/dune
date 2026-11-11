using System.Globalization;
using System.IO.Compression;

CultureInfo.DefaultThreadCurrentCulture = CultureInfo.InvariantCulture;
CultureInfo.CurrentCulture = CultureInfo.InvariantCulture;

switch (args.Length > 0 ? args[0] : "")
{
    case "log-lines":
        {
            Stream s = File.Create(args[2]);
            if (args[2].EndsWith(".gz")) s = new GZipStream(s, CompressionLevel.SmallestSize);
            using var w = new StreamWriter(s, new System.Text.UTF8Encoding(false)) { NewLine = "\n" };
            LogLines.Run(args[1], w);
            break;
        }
    case "log-lines-traces":
        {
            Stream s = File.Create(args[1]);
            if (args[1].EndsWith(".gz")) s = new GZipStream(s, CompressionLevel.SmallestSize);
            using var w = new StreamWriter(s, new System.Text.UTF8Encoding(false)) { NewLine = "\n" };
            LogLines.RunTraces(args.Skip(2), w);
            break;
        }
    default:
        Console.Error.WriteLine("usage: log-lines <fixtures dir> <out.jsonl[.gz]> | log-lines-traces <out.jsonl[.gz]> <trace files...>");
        Environment.Exit(2);
        break;
}
