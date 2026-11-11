// Skin.Load and SkinToString: the default skin's JSON, and Load on a set of edited skins with
// DefaultSkin.Default as the donor. One JSON line per case: {"in":..., "out":...}, where out is
// SkinToString() of the loaded skin or "!ExceptionType". rust/skin/tests/skin_json.rs.

public static class Load
{
    public static void Run(string path)
    {
        using var w = Output.Open(path);
        var full = DefaultSkin.Default.SkinToString();
        string[] inputs =
        [
            full, "{}", "null", "[1]", "\"x\"", "{\"Map_URL\":null}", "{\"Map_URL\":\"\"}", "{\"Map_URL\":\"m.svg\"}",
            "{\"Concept_STR\":{\"Messiah\":\"X\"}}",
            "{\"Concept_STR\":{\"Messiah\":\"A\",\"Monster\":\"B\",\"Resource\":\"C\",\"Graveyard\":\"D\",\"BabyMonster\":\"E\",\"GreatMonster\":\"F\",\"None\":\"G\"}}",
            "{\"Concept_STR\":null}", "{\"Concept_STR\":{}}", "{\"PlanetRadius\":0,\"MapDimensions\":{\"X\":0,\"Y\":0}}",
            "{\"PlanetRadius\":5,\"MapDimensions\":{\"X\":1.5,\"Y\":0}}", "{\"Version\":0}", "{\"Version\":7}",
            "{\"Version\":\"x\"}", "{\"version\":7}", "{\"HmsDx\":5,\"HmsRadius\":1}", "{\"DrawResourceIconsOnMap\":false}",
            "{\"FactionName_STR\":{\"20\":\"G\"}}", "{\"FactionName_STR\":{\"atreides\":\"G\"}}", "{\"FactionName_STR\":{\"Atreides\":5}}",
            "{\"Sound\":{\"GameStarted\":\"a\",\"GameStarted\":\"b\"}}", "{\"PersonName_STR\":{\"x\":\"y\"}}",
            "{\"LocationCenter_Point\":{\"0\":{\"X\":1,\"Y\":2}}}", "{\"Unknown\":1,\"SHADOW\":\"s\"}",
            "{\"FACTION_INFORMATIONCARDSTYLE\":null,\"SHADOW\":null}", "{\"MapRadius\":2147483648}",
        ];
        foreach (var input in inputs)
        {
            string result;
            try
            {
                result = Skin.Load(input, DefaultSkin.Default).SkinToString();
            }
            catch (Exception e)
            {
                result = "!" + e.GetType().Name;
            }

            w.WriteLine(JsonSerializer.Serialize(new Dictionary<string, string> { ["in"] = input, ["out"] = result }));
        }
    }
}
