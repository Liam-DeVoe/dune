using System.Globalization;
using System.Xml.Linq;

namespace Treachery.Oracle;

/// <summary>Summarizes a Cobertura report: overall line coverage of Treachery.Shared and the least-covered files.</summary>
public static class CoverageReport
{
    // Files that only the client or server use. They are reported separately because no trace can cover them.
    private static readonly string[] UiOnly = ["DefaultSkin.cs", "Skin.cs", "GameStatus.cs", "GameStatistics.cs"];

    public static int Print(string path, int top)
    {
        var doc = XDocument.Load(path);
        var lines = new Dictionary<string, Dictionary<int, bool>>();
        foreach (var cls in doc.Descendants("class"))
        {
            var file = (string)cls.Attribute("filename")!;
            if (!lines.TryGetValue(file, out var perLine)) lines[file] = perLine = new Dictionary<int, bool>();
            foreach (var line in cls.Descendants("line"))
            {
                var nr = (int)line.Attribute("number")!;
                var hit = int.Parse((string)line.Attribute("hits")!, CultureInfo.InvariantCulture) > 0;
                perLine[nr] = perLine.GetValueOrDefault(nr) || hit;
            }
        }

        var rows = lines.Select(kv => (File: Shorten(kv.Key), Covered: kv.Value.Count(x => x.Value), Total: kv.Value.Count)).ToList();
        Report("Treachery.Shared", rows);
        Report("Treachery.Shared without UI-only files (" + string.Join(", ", UiOnly) + ")",
            rows.Where(r => !UiOnly.Contains(Path.GetFileName(r.File))).ToList());

        Console.WriteLine();
        Console.WriteLine($"Most uncovered lines (excluding UI-only files):");
        foreach (var r in rows.Where(r => !UiOnly.Contains(Path.GetFileName(r.File))).OrderByDescending(r => r.Total - r.Covered).Take(top))
            Console.WriteLine($"  {r.Total - r.Covered,5} uncovered  {Pct(r.Covered, r.Total),6}  {r.File}");

        Console.WriteLine();
        Console.WriteLine($"Lowest coverage, files with at least 20 lines (excluding UI-only files):");
        foreach (var r in rows.Where(r => r.Total >= 20 && !UiOnly.Contains(Path.GetFileName(r.File))).OrderBy(r => (double)r.Covered / r.Total).Take(top))
            Console.WriteLine($"  {Pct(r.Covered, r.Total),6}  {r.Covered,5}/{r.Total,-5}  {r.File}");
        return 0;
    }

    private static void Report(string name, List<(string File, int Covered, int Total)> rows)
    {
        var covered = rows.Sum(r => r.Covered);
        var total = rows.Sum(r => r.Total);
        Console.WriteLine($"{name}: {Pct(covered, total)} of lines ({covered}/{total})");
    }

    private static string Pct(int covered, int total) => total == 0 ? "-" : (100.0 * covered / total).ToString("F1", CultureInfo.InvariantCulture) + "%";

    private static string Shorten(string file)
    {
        var i = file.Replace('\\', '/').LastIndexOf("/Shared/", StringComparison.Ordinal);
        return i >= 0 ? file[(i + "/Shared/".Length)..] : file;
    }
}
