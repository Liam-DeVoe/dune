using System.Globalization;
using System.Linq;

namespace Treachery.Client.MapComponents;

// The shape of a location's region on the map, for the glyph on a place chip:
// a territory outline from the skin, cut to the location's storm sector. The
// outlines use only M, L, C and Z, so the path is flattened to a polygon and
// the piece inside the sector is measured by sampling points.
public static class RegionGeometry
{
    private const double ReferenceWidth = 563d;
    private const double ReferenceHeight = 626d;
    private const int Samples = 40;

    // The side of a square with the piece's area, and the piece's centre of
    // mass, in the outline's coordinates.
    public record Piece(double Size, PointD Center);

    private static readonly Dictionary<(string, int), Piece> Cache = [];

    public static Piece Measure(Skin skin, string path, int sector)
    {
        var key = (path, sector);
        if (Cache.TryGetValue(key, out var cached)) return cached;

        var polygon = Flatten(path);
        var (minX, minY, maxX, maxY) = (polygon.Min(p => p.X), polygon.Min(p => p.Y), polygon.Max(p => p.X), polygon.Max(p => p.Y));
        var step = Math.Max(maxX - minX, maxY - minY) / Samples;
        var (cx, cy) = PlanetCenter(skin);

        var inside = 0;
        double sumX = 0, sumY = 0;
        for (var x = minX + step / 2; x < maxX; x += step)
        for (var y = minY + step / 2; y < maxY; y += step)
        {
            if (!Contains(polygon, x, y)) continue;
            if (sector >= 0 && SectorOf(x - cx, y - cy) != sector) continue;
            inside++;
            sumX += x;
            sumY += y;
        }

        var piece = inside == 0
            ? new Piece(0, new PointD((float)((minX + maxX) / 2), (float)((minY + maxY) / 2)))
            : new Piece(Math.Sqrt(inside) * step, new PointD((float)(sumX / inside), (float)(sumY / inside)));
        Cache[key] = piece;
        return piece;
    }

    // The planet's centre in the outline's coordinates, as SectorGeometry.WedgePath scales it.
    public static (double X, double Y) PlanetCenter(Skin skin) =>
        (skin.PlanetCenter.X * ReferenceWidth / skin.MapDimensions.X, skin.PlanetCenter.Y * ReferenceHeight / skin.MapDimensions.Y);

    public static double PlanetRadius(Skin skin) => skin.PlanetRadius * ReferenceWidth / skin.MapDimensions.X;

    // The storm sector of a point relative to the planet's centre, by the same angles as the wedges.
    private static int SectorOf(double dx, double dy)
    {
        var angle = Math.Atan2(dy, dx);
        var sector = (int)Math.Floor(6 - angle * Map.NumberOfSectors / (2 * Math.PI));
        return ((sector % Map.NumberOfSectors) + Map.NumberOfSectors) % Map.NumberOfSectors;
    }

    private static bool Contains(List<(double X, double Y)> polygon, double x, double y)
    {
        var inside = false;
        for (int i = 0, j = polygon.Count - 1; i < polygon.Count; j = i++)
        {
            var (xi, yi) = polygon[i];
            var (xj, yj) = polygon[j];
            if (yi > y != yj > y && x < (xj - xi) * (y - yi) / (yj - yi) + xi) inside = !inside;
        }
        return inside;
    }

    // Absolute M, L, C and Z only, which is all the shipped outlines use. Each curve becomes eight segments.
    private static List<(double X, double Y)> Flatten(string path)
    {
        var points = new List<(double X, double Y)>();
        var numbers = new List<double>();
        var command = ' ';

        void Run()
        {
            if (command == 'M' || command == 'L')
            {
                for (var i = 0; i + 1 < numbers.Count; i += 2) points.Add((numbers[i], numbers[i + 1]));
            }
            else if (command == 'C')
            {
                for (var i = 0; i + 5 < numbers.Count; i += 6)
                {
                    var (x0, y0) = points.Count > 0 ? points[^1] : (numbers[i], numbers[i + 1]);
                    for (var k = 1; k <= 8; k++)
                    {
                        var t = k / 8d;
                        var u = 1 - t;
                        points.Add((
                            u * u * u * x0 + 3 * u * u * t * numbers[i] + 3 * u * t * t * numbers[i + 2] + t * t * t * numbers[i + 4],
                            u * u * u * y0 + 3 * u * u * t * numbers[i + 1] + 3 * u * t * t * numbers[i + 3] + t * t * t * numbers[i + 5]));
                    }
                }
            }
            numbers.Clear();
        }

        foreach (var token in path.Replace(",", " ").Split(' ', StringSplitOptions.RemoveEmptyEntries))
        {
            var t = token;
            if (char.IsLetter(t[0]))
            {
                Run();
                command = char.ToUpperInvariant(t[0]);
                t = t[1..];
                if (t.Length == 0) continue;
            }
            if (double.TryParse(t, NumberStyles.Float, CultureInfo.InvariantCulture, out var value)) numbers.Add(value);
        }
        Run();
        return points;
    }
}
