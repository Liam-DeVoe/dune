using Treachery.Oracle.Canon;

namespace Treachery.Oracle;

/// <summary>How bots and the host interact with the engine. See trace-format.md, "Host styles".</summary>
public enum HostStyle
{
    /// <summary>Like Test/Tests.cs: every bot sees GetApplicableEvents(p, true), all events are submitted with isHost = true.</summary>
    Tests,

    /// <summary>Like Server PerformBotEvent with botsActAsHosts = true: in-phase actions isHost = false, end-phase actions isHost = true, bot order shuffled every step.</summary>
    ServerBotsHost,

    /// <summary>Like Server PerformBotEvent with botsActAsHosts = false: bots never act as host; the oracle plays a human host (EndPhase etc.).</summary>
    ServerHumanHost
}

/// <summary>Everything about a game that is derived from its seed before play starts.</summary>
public sealed class GameConfig
{
    public required int Version { get; init; }
    public required Ruleset BaseRuleset { get; init; }
    public required List<Rule> Rules { get; init; }
    public required List<Faction> AllowedFactions { get; init; }
    public required int NumberOfPlayers { get; init; }
    public required int MaximumTurns { get; init; }
    public required int EngineSeed { get; init; }
    public required HostStyle HostStyle { get; init; }
    public required Dictionary<int, int> SeatedHumans { get; init; }

    private static readonly Rule[] HouseAndExceptionRules = Enum.GetValues<Rule>()
        .Where(r => Game.GetRuleGroup(r) is RuleGroup.House or RuleGroup.CoreBasicExceptions or RuleGroup.CoreAdvancedExceptions)
        .ToArray();

    private static readonly Rule[] ExpansionRules = Enum.GetValues<Rule>()
        .Where(r => Game.GetRuleGroup(r) is RuleGroup.CoreAdvanced or RuleGroup.ExpansionIxAndBtBasic or RuleGroup.ExpansionIxAndBtAdvanced
            or RuleGroup.ExpansionChoamAndRicheseBasic or RuleGroup.ExpansionChoamAndRicheseAdvanced
            or RuleGroup.ExpansionEcazAndMoritaniBasic or RuleGroup.ExpansionEcazAndMoritaniAdvanced)
        .ToArray();

    // Rules that are not in any rule group but still switch engine behavior.
    private static readonly Rule[] OtherRules =
    [
        Rule.ExpansionTreacheryCardsExceptPBandSSandAmal, Rule.ExpansionTreacheryCardsPBandSs, Rule.ExpansionTreacheryCardsAmal,
        Rule.RicheseTreacheryCards, Rule.BeneGesseritFirstForceInAnyTerritory, Rule.BeneGesseritAccompaniesToShipmentLocation
    ];

    // Rules that make the game wait for a host decision bots don't make. The oracle host handles them, but they
    // make games slower and less representative, so they are switched on less often.
    private static readonly Rule[] HostHeavyRules = [Rule.CustomDecks, Rule.CustomInitialForcesAndResources, Rule.PlayersChooseFactions, Rule.HmSwithoutIxian];

    // Every N in a "Version <op> N" check in Shared (grep -rhoE "Version *(<=|>=|<|>|==|!=) *[0-9]+" Shared). Fuzz games
    // pick N or N - 1 so that both sides of every version gate are played.
    private static readonly int[] VersionGates =
    [
        100, 102, 103, 104, 106, 107, 108, 109, 110, 113, 115, 116, 117, 120, 121, 122, 123, 124, 125, 126, 131, 132, 133, 134,
        135, 136, 138, 139, 140, 142, 144, 145, 146, 147, 148, 150, 151, 152, 153, 154, 155, 156, 157, 158, 159, 160, 161, 162,
        163, 164, 165, 166, 167, 169, 170, 171, 172, 174, 176, 177, 178, 179, 180, 181, 182, 186, 187, 188
    ];

    // Rules whose absence or presence fuzz games should vary freely (everything except the bot seating rules).
    private static readonly Rule[] SoupRules = Enum.GetValues<Rule>()
        .Where(r => r != Rule.None && Game.GetRuleGroup(r) != RuleGroup.Bots && r != Rule.FillWithBots)
        .ToArray();

    public static GameConfig Randomize(Random rng, bool fuzz = false, bool latestOnly = false)
    {
        int version;
        if (!fuzz)
            version = rng.NextDouble() < 0.85 ? Game.LatestVersion : rng.Next(100, Game.LatestVersion + 1);
        else
            version = rng.NextDouble() switch
            {
                < 0.4 => Game.LatestVersion,
                < 0.85 => Math.Max(100, VersionGates[rng.Next(VersionGates.Length)] - rng.Next(2)),
                _ => rng.Next(100, Game.LatestVersion + 1)
            };
        // The version is still drawn above, so the rest of the configuration matches the default mode's.
        if (latestOnly) version = Game.LatestVersion;

        var rulesets = Game.RulesetDefinition.Keys.OrderBy(k => (int)k).ToArray();
        var ruleset = rulesets[rng.Next(rulesets.Length)];
        var rules = new List<Rule>(Game.RulesetDefinition[ruleset]);

        foreach (var r in ExpansionRules.Concat(OtherRules))
            if (rng.NextDouble() < 0.08) Toggle(rules, r);

        foreach (var r in HouseAndExceptionRules)
        {
            var p = HostHeavyRules.Contains(r) ? 0.04 : 0.15;
            if (rng.NextDouble() < p) Toggle(rules, r);
        }

        // Before version 113, these rules seated bots of fixed factions (EstablishPlayers.AddBots).
        Rule[] legacyBotRules = [Rule.GuildBot, Rule.EmperorBot, Rule.HarkonnenBot, Rule.TleilaxuBot, Rule.BeneGesseritBot, Rule.AtreidesBot, Rule.FremenBot, Rule.IxianBot];
        if (rng.NextDouble() < (version < 113 ? 0.6 : 0.03))
            foreach (var r in legacyBotRules)
                if (rng.NextDouble() < 0.3) rules.Add(r);

        // Fuzz games sometimes get a rule set drawn from every Rule value, ignoring rulesets and expansions.
        if (fuzz && rng.NextDouble() < 0.35)
        {
            var density = 0.15 + 0.5 * rng.NextDouble();
            foreach (var r in SoupRules)
                if (rng.NextDouble() < density) Toggle(rules, r);
        }

        if (!rules.Contains(Rule.FillWithBots)) rules.Add(Rule.FillWithBots);
        if (rng.NextDouble() < 0.2) rules.Add(Rule.BotsCannotAlly);

        Shuffle(rules, rng);

        var all = EstablishPlayers.AvailableFactions().ToList();
        var nrOfPlayers = rng.NextDouble() < 0.8 ? rng.Next(2, 9) : rng.Next(2, all.Count + 1);
        var factions = all.Where(_ => true).ToList();
        Shuffle(factions, rng);
        var allowedCount = rng.Next(nrOfPlayers, all.Count + 1);
        factions = factions.Take(allowedCount).ToList();

        var turns = rng.NextDouble() < 0.05 ? rng.Next(1, 3) : rng.Next(3, 11);
        if (fuzz && rng.NextDouble() < 0.25) turns = rng.Next(10, 21);

        var style = rng.NextDouble() switch
        {
            < 0.5 => HostStyle.Tests,
            < 0.75 => HostStyle.ServerBotsHost,
            _ => HostStyle.ServerHumanHost
        };

        var humans = new Dictionary<int, int>();
        if (rng.NextDouble() < 0.3)
        {
            var count = rng.Next(1, Math.Min(3, nrOfPlayers) + 1);
            var seats = Enumerable.Range(0, nrOfPlayers).ToList();
            Shuffle(seats, rng);
            for (var i = 0; i < count; i++) humans[100 + i] = seats[i];
        }

        return new GameConfig
        {
            Version = version,
            BaseRuleset = ruleset,
            Rules = rules,
            AllowedFactions = factions,
            NumberOfPlayers = nrOfPlayers,
            MaximumTurns = turns,
            EngineSeed = rng.Next(),
            HostStyle = style,
            SeatedHumans = humans
        };
    }

    private static void Toggle(List<Rule> rules, Rule r)
    {
        if (!rules.Remove(r)) rules.Add(r);
    }

    public static void Shuffle<T>(IList<T> list, Random rng)
    {
        for (var n = list.Count - 1; n > 0; n--)
        {
            var k = rng.Next(n + 1);
            (list[n], list[k]) = (list[k], list[n]);
        }
    }

    public Participation CreateParticipation()
    {
        var p = new Participation();
        foreach (var (user, seat) in SeatedHumans)
        {
            p.PlayerNames[user] = "Human" + user;
            p.SeatedPlayers[user] = seat;
        }

        return p;
    }

    public EstablishPlayers CreateEstablishPlayers(Game game)
    {
        return new EstablishPlayers(game, Faction.None)
        {
            Seed = EngineSeed,
            Settings = new GameSettings
            {
                InitialRules = Rules.ToList(),
                AllowedFactionsInPlay = AllowedFactions.ToList(),
                MaximumTurns = MaximumTurns,
                NumberOfPlayers = NumberOfPlayers
            },
            // Used by versions before 170.
            MaximumNumberOfPlayers = NumberOfPlayers,
            MaximumTurns = MaximumTurns,
            ApplicableRules = Rules.ToArray(),
            FactionsInPlay = AllowedFactions.ToList()
        };
    }

    public void WriteJson(JsonOut o)
    {
        o.ObjStart();
        o.Key("baseRuleset"); Recorder.WriteEnum(o, BaseRuleset);
        o.Comma(); o.Key("rules"); WriteEnumList(o, Rules);
        o.Comma(); o.Key("allowedFactions"); WriteEnumList(o, AllowedFactions);
        o.Comma(); o.Key("numberOfPlayers"); o.Int(NumberOfPlayers);
        o.Comma(); o.Key("maximumTurns"); o.Int(MaximumTurns);
        o.Comma(); o.Key("hostStyle"); o.WriteString(HostStyle.ToString());
        o.ObjEnd();
    }

    private static void WriteEnumList<T>(JsonOut o, IEnumerable<T> items) where T : Enum
    {
        o.ArrStart();
        var first = true;
        foreach (var i in items)
        {
            if (!first) o.Comma();
            first = false;
            Recorder.WriteEnum(o, i);
        }
        o.ArrEnd();
    }
}
