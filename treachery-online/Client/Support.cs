/*
 * Copyright (C) 2020-2025 Ronald Ossendrijver (admin@treachery.online)
 * This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License
 * as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version. This
 * program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details. You should have
 * received a copy of the GNU General Public License along with this program. If not, see <http://www.gnu.org/licenses/>.
 */

using System.Globalization;
using System.Linq;
using System.Security.Cryptography;
using System.Text;

namespace Treachery.Client;

public static class Support
{
    public static void Log(object o)
    {
        Console.WriteLine(o);
    }

    public static void Log(string msg, params object[] o)
    {
        Console.WriteLine(msg, o);
    }

    public static string GetHash(string input)
    {
        using SHA256 sha256Hash = SHA256.Create();
        // ComputeHash - returns byte array
        byte[] bytes = sha256Hash.ComputeHash(Encoding.UTF8.GetBytes(input));

        // Convert byte array to a string
        StringBuilder builder = new StringBuilder();
        foreach (var t in bytes)
        {
            builder.Append(t.ToString("x2"));
        }
        return builder.ToString();
    }

    // The markup SpiceComponent renders, for code that builds popover HTML as a string.
    public static string SpiceHtml(Skin skin, object amount, int size) => skin.SpiceHtml(amount, size);

    // A region narrower than this fraction of the planet glyph's disc is scaled up to it.
    private const double PlaceGlyphMinimumSize = 0.20;

    private static int _placeChipCounter;

    // The markup of a place chip (PlaceChip.razor renders this): a pill with a
    // glyph of the planet showing the place's real outline where it lies, the
    // territory name, and for a sector its compass word in the label face. A
    // region too small to see at glyph size is scaled up about its own centre
    // until it is a fifth of the disc wide. The hover script in index.html reads
    // data-ref-locations and lights those sectors on the map while the mouse is
    // over the chip. Place is a Location or a Territory.
    public static string PlaceChipHtml(Skin skin, object place)
    {
        var id = "place" + System.Threading.Interlocked.Increment(ref _placeChipCounter);
        var territory = place is Location l ? l.Territory : (Territory)place;
        var locations = place is Location loc ? [loc] : territory.Locations;
        var sector = place is Location { Sector: >= 0 } sl ? sl.Sector : -1;

        var (cx, cy) = MapComponents.RegionGeometry.PlanetCenter(skin);
        var r = MapComponents.RegionGeometry.PlanetRadius(skin);
        var view = $"{Round(cx - r - 3)} {Round(cy - r - 3)} {Round(2 * r + 6)} {Round(2 * r + 6)}";
        var border = skin.GetTerritoryBorder(territory);

        var html = new StringBuilder();
        html.Append($"<span class=\"chip place\" data-ref-locations=\"{string.Join(",", locations.Select(x => x.Id))}\" title=\"{skin.Describe(place)}\">");
        html.Append($"<svg class=\"wedge\" viewBox=\"{view}\" aria-hidden=\"true\"><defs>");
        html.Append($"<clipPath id=\"{id}-disc\"><circle cx=\"{Round(cx)}\" cy=\"{Round(cy)}\" r=\"{Round(r)}\"/></clipPath>");
        if (sector >= 0)
            html.Append($"<clipPath id=\"{id}-sector\"><path d=\"{MapComponents.SectorGeometry.WedgePath(skin, sector)}\"/></clipPath>");
        html.Append($"</defs><g clip-path=\"url(#{id}-disc)\"><circle class=\"disc\" cx=\"{Round(cx)}\" cy=\"{Round(cy)}\" r=\"{Round(r)}\"/>");

        if (border != "")
        {
            var piece = MapComponents.RegionGeometry.Measure(skin, border, sector);
            var grow = piece.Size > 0 ? Math.Max(1, PlaceGlyphMinimumSize * 2 * r / piece.Size) : 1;
            var clip = sector >= 0 ? $" clip-path=\"url(#{id}-sector)\"" : "";
            html.Append($"<g transform=\"translate({Round(piece.Center.X)},{Round(piece.Center.Y)}) scale({Round(grow)}) translate({Round(-piece.Center.X)},{Round(-piece.Center.Y)})\">");
            html.Append($"<g{clip}><path class=\"w\" d=\"{border}\"/></g></g>");
        }

        html.Append($"</g><circle class=\"ring\" cx=\"{Round(cx)}\" cy=\"{Round(cy)}\" r=\"{Round(r)}\" vector-effect=\"non-scaling-stroke\"/></svg>");
        html.Append($"<span class=\"name\">{skin.Describe(territory)}</span>");
        if (place is Location { Orientation: not "" } oriented)
            html.Append($"<span class=\"sector\">{oriented.Orientation}</span>");
        html.Append("</span>");

        return html.ToString();
    }

    // A spice card's popover: the card at 200px in a frame, with the place
    // chip of its territory and its spice amount under it. Worm cards
    // (Shai-Hulud, Great Maker, Sand Trout) are the card alone. Styled by
    // .pop-spice in _pop.scss. This lives in the client rather than on Skin
    // because the place chip needs the map geometry here.
    public static string GetPopup(this Skin skin, ResourceCard? c)
    {
        if (c == null) return "";

        var html = $"<div class=\"pop-spice\"><img src=\"{skin.GetImageUrl(c)}\"/>";
        if (c.Location != null)
        {
            html += $"<div class=\"pop-spice-foot\">{PlaceChipHtml(skin, c.Location)}{SpiceHtml(skin, c.Location.SpiceBlowAmount, 26)}</div>";
        }

        return html + "</div>";
    }

    // A faction's popover on the map: its sigil, name and player, its leaders,
    // where it starts, its free revival, the sheet's theme line, and the names
    // of its advantages as they head the sheet's paragraphs. All of it is from
    // the faction's rules sheet (Skin.GetFactionSheet). The full sheet is in
    // the popover too, hidden until Shift is held (see shift-held in
    // index.html). Styled by .pop-faction in _pop.scss.
    public static string FactionPopupHtml(Skin skin, Game game, Player player)
    {
        var sheet = skin.GetFactionSheet(game, player.Faction);
        if (sheet == null) return "";

        var html = new StringBuilder("<div class=\"pop-faction\">");
        html.Append($"<div class=\"pop-faction-head\"><img src=\"{skin.GetImageUrl(player.Faction)}\" alt=\"\"/>");
        html.Append($"<div><h6>{skin.Describe(player.Faction)}</h6><div class=\"who\">{player.Name}</div></div></div>");

        html.Append("<div class=\"pop-faction-brief\">");
        html.Append("<div class=\"pop-faction-leaders\">");
        foreach (var leader in LeaderManager.Leaders.Where(l => l.Faction == player.Faction))
            html.Append($"<img src=\"{skin.GetImageUrl(leader)}\" alt=\"\"/>");
        html.Append("</div>");

        // The sheet's own wording: "<at start>. Start with <n> <spice>."
        var atStart = $"{sheet.AtStart}. Start with {sheet.Resources} {{16}}.";
        atStart = System.Text.RegularExpressions.Regex.Replace(atStart, @"\{(\d+)\}", m =>
        {
            var value = sheet.Parameters[int.Parse(m.Groups[1].Value)];
            return value is Territory or Location ? PlaceChipHtml(skin, value) : skin.Describe(value);
        });
        atStart = skin.WithSpiceIcons(atStart, 18, encode: false);
        html.Append($"<dl><dt>At start</dt><dd>{atStart}</dd><dt>Free revival</dt><dd>{sheet.FreeRevival}</dd></dl>");

        html.Append($"<div class=\"theme\">{sheet.Theme}</div>");
        html.Append(AdvantageNames(sheet.Advantages));
        if (sheet.AdvancedAdvantages.Count > 0)
            html.Append("<div class=\"section\">Advanced Game Advantages</div>").Append(AdvantageNames(sheet.AdvancedAdvantages));

        html.Append("<div class=\"foot\">Hold <kbd>Shift</kbd> for full rules</div></div>");
        html.Append($"<div class=\"pop-faction-full\">{skin.GetFactionInfo_HTML(game, player.Faction, "")}</div></div>");
        return html.ToString();
    }

    private static string AdvantageNames(IEnumerable<string> names)
        => $"<ul>{string.Concat(names.Select(n => $"<li>{n}</li>"))}</ul>";

    // Special forces whose token differs from the faction's normal token only by a small star in the art:
    // Fedaykin, Sardaukar and cyborgs. These get a star badge so they can be told apart at map size.
    public static bool HasStarredSpecialForce(Faction faction) => faction is Faction.Fremen or Faction.Emperor or Faction.Ixian;

    // The markup ForceToken renders, for code that builds popover HTML as a string.
    public static string ForceTokenHtml(Skin skin, Faction faction, bool special, int size, string title = "")
    {
        var src = special ? skin.GetFactionSpecialForceImageUrl(faction) : skin.GetFactionForceImageUrl(faction);
        var starred = special && HasStarredSpecialForce(faction) ? " starred" : "";
        return $"<span class='force-token{starred}' style='--d:{size}px' title='{title}'><img src='{src}' width='{size}' height='{size}' alt='{title}'/></span>";
    }

    // A faction's spice line, "5, 2 at end of turn, 3 at Mentat Pause", with every
    // number in it drawn on the harvester icon: the first, the spice in hand, at
    // `size` and the rest smaller. The words between them are bold.
    public static string SpiceLineHtml(Skin skin, string text, int size)
    {
        var html = new StringBuilder();
        var first = true;
        foreach (var part in System.Text.RegularExpressions.Regex.Split(text, @"(\d+)"))
        {
            if (part.Length == 0) continue;
            if (char.IsDigit(part[0]))
            {
                html.Append(skin.SpiceHtml(part, first ? size : size - 6));
                first = false;
            }
            else
            {
                html.Append($"<strong>{System.Net.WebUtility.HtmlEncode(part)}</strong>");
            }
        }
        return html.ToString();
    }

    public static string TextBorder(int borderWidth, string borderColor)
    {
        return
            $"-webkit-text-stroke: {Px(0.3f * borderWidth)} {borderColor}; text-shadow: 0 0 {2 * borderWidth}px {borderColor}";
    }

    public static string Round(double x)
    {
        return Math.Round(x, 3).ToString(CultureInfo.InvariantCulture);
    }

    public static string Px(double x)
    {
        return "" + Round(x) + "px";
    }
}