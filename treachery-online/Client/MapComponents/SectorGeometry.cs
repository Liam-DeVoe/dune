namespace Treachery.Client.MapComponents;

// The pie-slice outline of one storm sector, in the same coordinate space as
// the territory borders in the skin. Planet.razor uses the same numbers for
// its per-sector clip paths.
public static class SectorGeometry
{
    private const double ReferenceWidth = 563d;
    private const double ReferenceHeight = 626d;

    public static string WedgePath(Skin skin, int sector)
    {
        var scaleX = ReferenceWidth / skin.MapDimensions.X;
        var scaleY = ReferenceHeight / skin.MapDimensions.Y;
        var angle = 360f / Map.NumberOfSectors;
        var r = skin.PlanetRadius;
        var cx = skin.PlanetCenter.X;
        var cy = skin.PlanetCenter.Y;

        var startX = Math.Cos(2f * Math.PI * (6 - sector) / Map.NumberOfSectors) * r + cx;
        var startY = Math.Sin(2f * Math.PI * (6 - sector) / Map.NumberOfSectors) * r + cy;
        var endX = Math.Cos(2f * Math.PI * (5 - sector) / Map.NumberOfSectors) * r + cx;
        var endY = Math.Sin(2f * Math.PI * (5 - sector) / Map.NumberOfSectors) * r + cy;

        return $"M {Utilities.Round(scaleX * cx)},{Utilities.Round(scaleY * cy)} " +
               $"L {Utilities.Round(scaleX * startX)},{Utilities.Round(scaleY * startY)} " +
               $"A {Utilities.Round(scaleX * r)} {Utilities.Round(scaleY * r)} {angle} 0 0 {Utilities.Round(scaleX * endX)},{Utilities.Round(scaleY * endY)} Z";
    }

    // The parts of a sector's wedge outline drawn separately, so a hover
    // shape can drop the radial side it shares with a lit neighbour in the
    // same territory. "start" is the side shared with sector - 1, "end" the
    // side shared with sector + 1.
    public static (string Start, string End, string Arc) WedgeEdges(Skin skin, int sector)
    {
        var scaleX = ReferenceWidth / skin.MapDimensions.X;
        var scaleY = ReferenceHeight / skin.MapDimensions.Y;
        var angle = 360f / Map.NumberOfSectors;
        var r = skin.PlanetRadius;
        var cx = skin.PlanetCenter.X;
        var cy = skin.PlanetCenter.Y;

        var startX = Math.Cos(2f * Math.PI * (6 - sector) / Map.NumberOfSectors) * r + cx;
        var startY = Math.Sin(2f * Math.PI * (6 - sector) / Map.NumberOfSectors) * r + cy;
        var endX = Math.Cos(2f * Math.PI * (5 - sector) / Map.NumberOfSectors) * r + cx;
        var endY = Math.Sin(2f * Math.PI * (5 - sector) / Map.NumberOfSectors) * r + cy;

        var c = $"{Utilities.Round(scaleX * cx)},{Utilities.Round(scaleY * cy)}";
        var s = $"{Utilities.Round(scaleX * startX)},{Utilities.Round(scaleY * startY)}";
        var e = $"{Utilities.Round(scaleX * endX)},{Utilities.Round(scaleY * endY)}";

        return ($"M {c} L {s}", $"M {c} L {e}",
                $"M {s} A {Utilities.Round(scaleX * r)} {Utilities.Round(scaleY * r)} {angle} 0 0 {e}");
    }

    // Whether the skin knows where to draw this location. Homeworlds are drawn
    // relative to the player's token and have no fixed centre.
    public static bool HasCenter(Skin skin, Location l) =>
        l is HiddenMobileStronghold || l is DiscoveredLocation || skin.LocationCenter_Point.ContainsKey(l.Id);

    // Centre of a player's token on the ring around the planet.
    public static PointD PlayerTokenPosition(Skin skin, Game game, int seat)
    {
        var positionSector = Math.Floor((float)seat * Map.NumberOfSectors / game.MaximumPlayers);
        var angle = (4.5 - positionSector) * 2 * Math.PI / Map.NumberOfSectors;
        var x = skin.PlanetCenter.X + (float)(skin.MapRadius * Math.Cos(angle));
        var y = skin.PlanetCenter.Y + (float)(skin.MapRadius * Math.Sin(angle));
        return new PointD(x, y);
    }

    // Planet.razor defines one clip path per sector with this id.
    public static string SectorClipId(int sector) => "clipOfSector" + sector;
}
