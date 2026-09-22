using System.Linq;
using Treachery.Client.MapComponents;

namespace Treachery.Client.Scene;

// One step of a scene's script: a click on a map location with modifier
// keys, or a click on a button by its label.
public sealed record SceneStep(string Kind, string Target)
{
    public bool IsButton => Kind == "button";
    public bool Shift => Kind == "shift";
    public bool Ctrl => Kind == "ctrl";
    public bool Right => Kind == "right";

    private static readonly string[] Kinds = ["click", "shift", "ctrl", "right", "button"];

    // Parses "click:Carthag|shift:Polar Sink|button:Ride!". Throws on a step
    // it does not understand, naming the step.
    public static List<SceneStep> ParseAll(string? script)
    {
        var result = new List<SceneStep>();
        if (string.IsNullOrWhiteSpace(script)) return result;

        foreach (var part in script.Split('|', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries))
        {
            var colon = part.IndexOf(':');
            var kind = colon < 0 ? "" : part[..colon].Trim().ToLowerInvariant();
            var target = colon < 0 ? "" : part[(colon + 1)..].Trim();
            if (!Kinds.Contains(kind) || target.Length == 0)
                throw new ArgumentException($"Cannot read step '{part}'. Steps look like click:<location>, shift:<location>, ctrl:<location>, right:<location> or button:<label>.");
            result.Add(new SceneStep(kind, target));
        }

        return result;
    }
}

public static class SceneScript
{
    // Accepts the faction's enum name (which is its colour), the number of
    // its image in the default skin (1 = Green .. 8 = Purple), or the name the
    // default skin gives it (Atreides, Harkonnen, ...).
    public static Faction? ParseFaction(string? text)
    {
        if (string.IsNullOrWhiteSpace(text)) return null;
        text = text.Trim();

        // Enum.TryParse would also take a number as the enum's own value.
        if (!char.IsDigit(text[0]) && Enum.TryParse<Faction>(text, true, out var byName) && byName != Faction.None)
            return byName;

        Faction[] bySkinNumber = [Faction.Green, Faction.Black, Faction.Yellow, Faction.Red, Faction.Orange, Faction.Blue, Faction.Grey, Faction.Purple];
        if (int.TryParse(text, out var number) && number >= 1 && number <= bySkinNumber.Length)
            return bySkinNumber[number - 1];

        foreach (var f in Enum.GetValues<Faction>())
        {
            if (f != Faction.None && string.Equals(DefaultSkin.Default.Describe(f), text, StringComparison.OrdinalIgnoreCase))
                return f;
        }

        return null;
    }

    // Every location the skin can place, by the name the default skin gives
    // it: "Carthag", "False Wall South (East Sector)", "Polar Sink".
    public static Dictionary<string, Location> NamedLocations(Game game)
    {
        var skin = DefaultSkin.Default;
        var result = new Dictionary<string, Location>(StringComparer.OrdinalIgnoreCase);
        foreach (var l in game.Map.Locations(false).Where(l => SectorGeometry.HasCenter(skin, l)))
            result.TryAdd(skin.Describe(l), l);
        return result;
    }

    // The same locations with their centres in map coordinates, for the
    // page's JavaScript to expose.
    public static Dictionary<string, object> LocationCentres(Game game)
    {
        var skin = DefaultSkin.Default;
        var result = new Dictionary<string, object>();
        foreach (var (name, l) in NamedLocations(game))
        {
            var c = skin.GetCenter(l, game.Map);
            result[name] = new { id = l.Id, x = Math.Round(c.X, 1), y = Math.Round(c.Y, 1) };
        }
        return result;
    }

    // Finds a location by its skin name or its numeric id.
    public static Location? FindLocation(Game game, string nameOrId)
    {
        if (int.TryParse(nameOrId, out var id))
            return game.Map.Locations(true).FirstOrDefault(l => l.Id == id);
        return NamedLocations(game).GetValueOrDefault(nameOrId.Trim());
    }
}
