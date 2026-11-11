// Dumps the static game data of Treachery.Shared (enums, map, leaders, cards,
// pathfinding results, deck shuffles) to a JSON golden file. The Rust tests in
// rust/engine/tests/static_data.rs rebuild the same JSON from the Rust port and
// compare it section by section.

using System.Reflection;
using System.Text.Json;
using System.Text.Json.Nodes;
using Treachery.Shared;

if (args.Length > 1 && args[0] == "sync-v188")
{
    SyncV188.Run(args[1]);
    return;
}

if (args.Length > 0 && args[0] == "review-k")
{
    ReviewK.Run();
    return;
}

if (args.Length > 0 && args[0] == "review-j")
{
    ReviewJ.Run(args);
    return;
}

if (args.Length > 0 && args[0] == "review-i")
{
    ReviewI.Run(args);
    return;
}

if (args.Length > 0 && args[0] == "review-d")
{
    ReviewD.Run();
    return;
}

if (args.Length > 1 && args[0] == "undo-identity")
{
    UndoIdentity.Run(args[1]);
    return;
}

if (args.Length > 0 && args[0] == "review-a-part2")
{
    ReviewA2.Run();
    return;
}

if (args.Length > 0 && args[0] == "review-g")
{
    ReviewG.Run();
    return;
}

if (args.Length > 0 && args[0] == "review")
{
    Review.Run();
    return;
}

if (args.Length > 1 && args[0] == "review-b")
{
    ReviewB.Run(args[1]);
    return;
}

if (args.Length > 0 && args[0] == "review-f")
{
    ReviewF.Run();
    return;
}

if (args.Length > 0 && args[0] == "review-e")
{
    ReviewE.Run();
    return;
}

if (args.Length > 0 && args[0] == "review-c")
{
    if (args.Length > 1 && args[1] == "validate-off") ReviewC.RunValidationOff();
    else ReviewC.Run();
    return;
}

if (args.Length > 0 && args[0] == "review-a")
{
    ReviewA.Run();
    return;
}

if (args.Length > 1 && args[0] == "--canon-schema")
{
    CanonSchema.Write(args[1]);
    return;
}

if (args.Length > 1 && args[0] == "--events")
{
    EventCorpus.Write(args[1]);
    return;
}

// The argument is the path of static_data.json; the other golden files are
// written next to it. tests/coverage/ported_types.txt (relative to the golden
// directory's parent) lists the C# types whose members boundary.json covers.
var output = args.Length > 0 ? args[0] : "static_data.json";
var goldenDir = Path.GetDirectoryName(Path.GetFullPath(output))!;
var portedTypes = Golden.ReadPortedTypes(Path.Combine(goldenDir, "..", "coverage", "ported_types.txt"));
var root = new JsonObject
{
    ["enums"] = DumpEnums(),
    ["map"] = DumpMap(new Map()),
    ["map_attached"] = DumpAttached(),
    ["pathfinding"] = DumpPathfinding(),
    ["resource_cards"] = DumpResourceCards(),
    ["leaders"] = DumpLeaders(),
    ["cards"] = DumpCards(),
    ["random"] = DumpRandom()
};
Golden.Write(output, root);
Golden.Write(Path.Combine(goldenDir, "boundary.json"), Boundary.Dump(portedTypes));
Golden.Write(Path.Combine(goldenDir, "random_helpers.json"), RandomGrid.Dump());
Golden.Write(Path.Combine(goldenDir, "linq.json"), LinqTable.Dump());
Golden.Write(Path.Combine(goldenDir, "messages.json"), MessageCorpus.Dump());
return;

static JsonArray Ids(IEnumerable<IIdentifiable?> items) => new(items.Select(i => (JsonNode?)(i == null ? null : JsonValue.Create(i.Id))).ToArray());

static JsonNode? Id(IIdentifiable? i) => i == null ? null : JsonValue.Create(i.Id);

// The value, "null" for null, or "exception:<Type>:<Message>".
static JsonNode Try(Func<JsonNode?> f)
{
    try
    {
        return f() ?? JsonValue.Create("null")!;
    }
    catch (Exception e)
    {
        return JsonValue.Create("exception:" + e.GetType().Name + ":" + e.Message)!;
    }
}

static JsonArray DumpEnums()
{
    var result = new JsonArray();
    foreach (var t in typeof(Map).Assembly.GetTypes().Where(t => t.IsEnum && t.IsPublic).OrderBy(t => t.Name, StringComparer.Ordinal))
    {
        var declared = new JsonArray();
        foreach (var f in t.GetFields(BindingFlags.Public | BindingFlags.Static))
            declared.Add(new JsonObject { ["name"] = f.Name, ["value"] = Convert.ToInt64(f.GetValue(null)) });

        result.Add(new JsonObject
        {
            ["name"] = t.Name,
            ["flags"] = t.GetCustomAttribute<FlagsAttribute>() != null,
            ["underlying"] = Enum.GetUnderlyingType(t).Name,
            ["declared"] = declared,
            ["get_values"] = new JsonArray(System.Enum.GetValues(t).Cast<object>().Select(v => (JsonNode?)Convert.ToInt64(v)).ToArray()),
            ["get_names"] = new JsonArray(Enum.GetNames(t).Select(n => (JsonNode?)n).ToArray()),
            // ToString of undefined values and flag combinations
            ["to_string"] = new JsonArray(new long[] { -1, 1, 3, 5, 7, 11, 999999 }.Select(v => (JsonNode?)(v + "=" + Enum.ToObject(t, v))).ToArray())
        });
    }
    return result;
}

static JsonObject DumpLocation(Location l)
{
    var o = new JsonObject
    {
        ["id"] = l.Id,
        ["kind"] = l.GetType().Name,
        ["sector"] = l.Sector,
        ["territory"] = l.Territory.Id,
        ["visible"] = l.Visible,
        ["neighbours"] = Ids(l.Neighbours),
        ["orientation"] = l.Orientation,
        ["spice_blow_amount"] = l.SpiceBlowAmount,
        ["discovery_token_type"] = (int)l.DiscoveryTokenType,
        ["is_stronghold"] = l.IsStronghold,
        ["is_homeworld"] = l.IsHomeworld,
        ["is_protected_from_storm"] = l.IsProtectedFromStorm,
        ["advantage"] = (int)l.Advantage,
        ["hash"] = l.GetHashCode()
    };
    if (l is Homeworld hw)
    {
        o["world"] = (int)hw.World;
        o["faction"] = (int)hw.Faction;
        o["threshold"] = hw.Threshold;
        o["resource_amount"] = hw.ResourceAmount;
        o["is_home_of_normal_forces"] = hw.IsHomeOfNormalForces;
        o["is_home_of_special_forces"] = hw.IsHomeOfSpecialForces;
        o["battle_bonus_high"] = hw.BattleBonusAndLasgunShieldLimitAtHighThreshold;
        o["battle_bonus_low"] = hw.BattleBonusAndLasgunShieldLimitAtLowThreshold;
    }
    if (l is AttachedLocation al) o["attached_to"] = Id(al.AttachedToLocation);
    if (l is DiscoveredLocation dl) o["discovery"] = (int)dl.Discovery;
    return o;
}

static JsonObject DumpTerritory(Territory t) => new()
{
    ["id"] = t.Id,
    ["skin_id"] = t.SkinId,
    ["is_homeworld"] = t.IsHomeworld,
    ["has_reduced_shipping_cost"] = t.HasReducedShippingCost,
    ["is_discovery"] = t.IsDiscovery,
    ["is_discovered_stronghold"] = t.IsDiscoveredStronghold,
    ["is_visible"] = t.IsVisible,
    ["is_stronghold"] = t.IsStronghold,
    ["is_protected_from_storm"] = t.IsProtectedFromStorm,
    ["is_protected_from_worm"] = t.IsProtectedFromWorm,
    ["advantage"] = (int)t.Advantage,
    ["locations"] = Ids(t.Locations),
    ["middle_location"] = Id(t.MiddleLocation),
    ["resource_blow_location"] = Id(t.ResourceBlowLocation),
    ["discovery_token_location"] = Id(t.DiscoveryTokenLocation)
};

static JsonObject DumpMap(Map m)
{
    var named = new JsonObject();
    foreach (var p in typeof(Map).GetProperties(BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Instance)
                 .Where(p => typeof(Location).IsAssignableFrom(p.PropertyType) || p.PropertyType == typeof(Territory))
                 .OrderBy(p => p.Name, StringComparer.Ordinal))
        named[p.Name] = (p.PropertyType == typeof(Territory) ? "territory:" : "location:") + ((IIdentifiable)p.GetValue(m)!).Id;

    var homeworldByWorld = new JsonObject();
    foreach (var w in Enum.GetValues<World>()) homeworldByWorld[((int)w).ToString()] = Id(m.GetHomeWorld(w));

    var discoveryStrongholds = new JsonObject();
    foreach (var d in Enum.GetValues<DiscoveryToken>()) discoveryStrongholds[((int)d).ToString()] = Try(() => Id(m.GetDiscoveryStronghold(d)));

    var territoryLookup = new JsonArray();
    for (var i = -2; i <= 62; i++) territoryLookup.Add(Try(() => Id(m.TerritoryLookup.Find(i))));
    var locationLookup = new JsonArray();
    for (var i = -2; i <= 106; i++) locationLookup.Add(Try(() => Id(m.LocationLookup.Find(i))));

    return new JsonObject
    {
        ["number_of_sectors"] = Map.NumberOfSectors,
        ["locations"] = new JsonArray(m.Locations(true).Select(l => (JsonNode)DumpLocation(l)).ToArray()),
        ["territories"] = new JsonArray(m.Territories(true).Select(t => (JsonNode)DumpTerritory(t)).ToArray()),
        ["locations_true"] = Ids(m.Locations(true)),
        ["locations_false"] = Ids(m.Locations(false)),
        ["territories_true"] = Ids(m.Territories(true)),
        ["territories_false"] = Ids(m.Territories(false)),
        ["strongholds"] = Ids(m.Strongholds),
        ["homeworlds"] = Ids(m.Homeworlds),
        ["named"] = named,
        ["get_home_world"] = homeworldByWorld,
        ["get_discovery_stronghold"] = discoveryStrongholds,
        ["territory_lookup"] = territoryLookup,
        ["location_lookup"] = locationLookup
    };
}

static void SetSectorInStorm(Game g, int sector) =>
    typeof(Game).GetProperty("SectorInStorm")!.SetValue(g, sector);

// Moves the attached locations around and dumps the map after every step, for
// an old game version (sector stays -1) and a new one (sector follows host).
static JsonArray DumpAttached()
{
    var result = new JsonArray();
    foreach (var version in new[] { 150, 162, 163, Game.LatestVersion })
    {
        var g = new Game(version, new Participation());
        var m = g.Map;
        var steps = new (string Name, Func<AttachedLocation> Which, Func<Location> To)[]
        {
            ("hms->arrakeen", () => m.HiddenMobileStronghold, () => m.Arrakeen),
            ("hms->polar sink", () => m.HiddenMobileStronghold, () => m.PolarSink),
            ("hms->first of false wall east", () => m.HiddenMobileStronghold, () => m.FalseWallEast.Locations.First()),
            ("jacurutu->great flat", () => m.Jacurutu, () => m.TheGreatFlat),
            ("shrine->first of shield wall", () => m.Shrine, () => m.ShieldWall.Locations.First()),
            ("hms->great flat", () => m.HiddenMobileStronghold, () => m.TheGreatFlat),
            ("cistern->carthag", () => m.Cistern, () => m.Carthag),
            ("hms->carthag", () => m.HiddenMobileStronghold, () => m.Carthag),
            ("hms->carthag again", () => m.HiddenMobileStronghold, () => m.Carthag),
        };
        foreach (var (name, which, to) in steps)
        {
            which().PointAt(g, to());
            var snapshot = DumpMap(m);
            var neighbours = new JsonArray();
            foreach (var l in m.Locations(true))
            foreach (var d in new[] { 1, 2 })
                neighbours.Add(Ids(m.FindNeighbours(l, d, false, Faction.None, g, false)));
            var hms = new JsonArray();
            foreach (var l in m.Locations(true))
                hms.Add(Ids(Map.FindNeighboursForHmsMovement(l, 1, false, 9)));
            result.Add(new JsonObject
            {
                ["version"] = version,
                ["step"] = name,
                ["locations"] = new JsonArray(m.Locations(true).Select(l => (JsonNode)new JsonArray(
                    l.Sector, l.Visible, Ids(l.Neighbours), l is AttachedLocation al ? Id(al.AttachedToLocation) : null)).ToArray()),
                ["territories_visible"] = new JsonArray(m.Territories(true).Select(t => (JsonNode?)t.IsVisible).ToArray()),
                ["strongholds"] = snapshot["strongholds"]!.DeepClone(),
                ["find_neighbours"] = neighbours,
                ["find_neighbours_for_hms_movement"] = hms
            });
        }
    }
    return result;
}

static JsonObject DumpPathfinding()
{
    var g = new Game(Game.LatestVersion, new Participation());
    var m = g.Map;
    var locations = m.Locations(true).ToList();
    var sectors = new[] { -1, 4, 9, 13 };

    var hms = new JsonArray();
    var within = new JsonArray();
    var neighbours = new JsonArray();
    foreach (var sector in sectors)
    {
        SetSectorInStorm(g, sector);
        foreach (var l in locations)
        foreach (var ignoreStorm in new[] { false, true })
        {
            within.Add(Ids(Map.FindNeighboursWithinTerritory(l, ignoreStorm, sector)));
            for (var d = 0; d <= 4; d++)
            {
                hms.Add(Ids(Map.FindNeighboursForHmsMovement(l, d, ignoreStorm, sector)));
                neighbours.Add(Ids(m.FindNeighbours(l, d, ignoreStorm, Faction.None, g, false)));
            }
        }
    }

    var shortest = new JsonArray();
    foreach (var sector in new[] { -1, 9 })
    {
        SetSectorInStorm(g, sector);
        foreach (var from in locations)
        foreach (var to in locations)
            shortest.Add(Try(() => Map.FindFirstShortestPath(from, to, false, Faction.Atreides, g) is { } p ? Ids(p) : null));
    }

    var paths = new JsonArray();
    var starts = new[] { 0, 5, 9, 23, 42, 57, 68, 74, 86 };
    foreach (var sector in new[] { -1, 9 })
    {
        SetSectorInStorm(g, sector);
        foreach (var s in starts)
        foreach (var to in locations)
        foreach (var d in new[] { 1, 2 })
        foreach (var ignoreStorm in new[] { false, true })
        {
            var found = Map.FindPaths(locations[s], to, d, ignoreStorm, Faction.Atreides, g);
            paths.Add(new JsonArray(found.Select(p => (JsonNode)Ids(p)).ToArray()));
        }
    }

    // The neighbour cache is keyed on History.Count, so a storm move between
    // two calls without a new event returns the stale result.
    var cache = new JsonArray();
    var g2 = new Game(Game.LatestVersion, new Participation());
    SetSectorInStorm(g2, -1);
    cache.Add(Ids(g2.Map.FindNeighbours(g2.Map.Arrakeen, 2, false, Faction.Atreides, g2)));
    SetSectorInStorm(g2, 9);
    cache.Add(Ids(g2.Map.FindNeighbours(g2.Map.Arrakeen, 2, false, Faction.Atreides, g2)));
    cache.Add(Ids(g2.Map.FindNeighbours(g2.Map.Arrakeen, 2, false, Faction.Atreides, g2, false)));
    cache.Add(Ids(g2.Map.FindNeighbours(g2.Map.Arrakeen, 2, false, Faction.Harkonnen, g2)));
    g2.History.Add(new AllianceBroken());
    cache.Add(Ids(g2.Map.FindNeighbours(g2.Map.Arrakeen, 2, false, Faction.Atreides, g2)));

    return new JsonObject
    {
        ["sectors"] = new JsonArray(sectors.Select(s => (JsonNode?)s).ToArray()),
        ["within_territory"] = within,
        ["hms_movement"] = hms,
        ["find_neighbours"] = neighbours,
        ["first_shortest_path"] = shortest,
        ["path_starts"] = new JsonArray(starts.Select(s => (JsonNode?)s).ToArray()),
        ["find_paths"] = paths,
        ["cache"] = cache
    };
}

static JsonArray DumpResourceCards()
{
    var m = new Map();
    var method = typeof(Map).GetMethod("GetResourceCardsInAndOutsidePlay", BindingFlags.NonPublic | BindingFlags.Static)!;
    var cards = (IEnumerable<ResourceCard>)method.Invoke(null, [m])!;
    return new JsonArray(cards.Select(c => (JsonNode)new JsonObject
    {
        ["skin_id"] = c.SkinId,
        ["is_sand_trout"] = c.IsSandTrout,
        ["is_great_maker"] = c.IsGreatMaker,
        ["location"] = Id(c.Location),
        ["discovery_location"] = Id(c.DiscoveryLocation),
        ["is_shai_hulud"] = c.IsShaiHulud,
        ["is_spice_blow"] = c.IsSpiceBlow,
        ["is_discovery"] = c.IsDiscovery,
        ["territory"] = Id(c.Territory)
    }).ToArray());
}

static JsonObject DumpHero(IHero h, IHero[] opponents) => new()
{
    ["id"] = h.Id,
    ["skin_id"] = h.SkinId,
    ["value"] = h.Value,
    ["faction"] = (int)h.Faction,
    ["hero_type"] = (int)h.HeroType,
    ["cost_to_revive"] = h.CostToRevive,
    ["value_in_combat_against"] = new JsonArray(new IHero?[] { null }.Concat(opponents).Select(o => (JsonNode?)h.ValueInCombatAgainst(o)).ToArray()),
    ["is_traitor"] = new JsonArray(opponents.Select(o => (JsonNode?)h.IsTraitor(o)).ToArray()),
    ["is_face_dancer"] = new JsonArray(opponents.Select(o => (JsonNode?)h.IsFaceDancer(o)).ToArray()),
    ["is"] = new JsonArray(Enum.GetValues<Faction>().Select(f => (JsonNode?)h.Is(f)).ToArray())
};

static JsonObject DumpLeaders()
{
    IHero[] opponents = [LeaderManager.Leaders[0], LeaderManager.Leaders[35], LeaderManager.Leaders[56], LeaderManager.Messiah, TreacheryCardManager.Items[17], TreacheryCardManager.Items[0]];
    var byFaction = new JsonObject();
    foreach (var f in Enum.GetValues<Faction>()) byFaction[((int)f).ToString()] = Ids(LeaderManager.GetLeaders(f));
    var heroLookup = new JsonArray();
    var leaderLookup = new JsonArray();
    foreach (var i in Enumerable.Range(-3, 110).Concat(Enumerable.Range(995, 75)))
    {
        heroLookup.Add(Try(() => Id(LeaderManager.HeroLookup.Find(i))));
        leaderLookup.Add(Try(() => Id(LeaderManager.LeaderLookup.Find(i))));
    }
    return new JsonObject
    {
        ["first_id"] = LeaderManager.FirstId,
        ["leaders"] = new JsonArray(LeaderManager.Leaders.Select(l => (JsonNode)DumpHero(l, opponents)).ToArray()),
        ["messiah"] = DumpHero(LeaderManager.Messiah, opponents),
        ["get_leaders"] = byFaction,
        ["lookup_ids"] = new JsonArray(Enumerable.Range(-3, 110).Concat(Enumerable.Range(995, 75)).Select(i => (JsonNode?)i).ToArray()),
        ["hero_lookup"] = heroLookup,
        ["leader_lookup"] = leaderLookup
    };
}

static JsonObject DumpCards()
{
    IHero[] opponents = [LeaderManager.Leaders[0], LeaderManager.Leaders[35], LeaderManager.Messiah, TreacheryCardManager.Items[17], TreacheryCardManager.Items[0]];
    var cards = TreacheryCardManager.Items;
    var items = new JsonArray();
    foreach (var c in cards)
    {
        var o = DumpHero(c, opponents);
        o["type"] = (int)c.Type;
        o["rules"] = new JsonArray(c.Rules.Select(r => (JsonNode?)(int)r).ToArray());
        o["is_poison_weapon"] = c.IsPoisonWeapon;
        o["is_projectile_weapon"] = c.IsProjectileWeapon;
        o["is_poison_defense"] = c.IsPoisonDefense;
        o["is_non_antidote_poison_defense"] = c.IsNonAntidotePoisonDefense;
        o["is_shield"] = c.IsShield;
        o["is_projectile_defense"] = c.IsProjectileDefense;
        o["is_laser"] = c.IsLaser;
        o["is_poison_tooth"] = c.IsPoisonTooth;
        o["is_artillery"] = c.IsArtillery;
        o["is_rock_melter"] = c.IsRockMelter;
        o["is_mirror_weapon"] = c.IsMirrorWeapon;
        o["is_portable_antidote"] = c.IsPortableAntidote;
        o["is_weapon"] = c.IsWeapon;
        o["is_defense"] = c.IsDefense;
        o["is_useless"] = c.IsUseless;
        o["is_mercenary"] = c.IsMercenary;
        o["is_atreides"] = c.IsAtreides;
        items.Add(o);
    }

    // CounteredBy(defense, combinedWithWeapon) for one card of every type; one
    // string of 0/1 (S: infinite recursion, stack overflow) per (weapon, defense) pair, indexed by [null, types...].
    var reps = cards.GroupBy(c => c.Type).Select(g => g.First()).ToList();
    var countered = new JsonArray();
    foreach (var w in reps)
    foreach (var d in reps)
        countered.Add(string.Concat(new TreacheryCard?[] { null }.Concat(reps).Select(c => w.IsMirrorWeapon && c is { IsMirrorWeapon: true } ? 'S' : w.CounteredBy(d, c!) ? '1' : '0')));

    var byRule = new JsonObject();
    var getByRule = typeof(TreacheryCardManager).GetMethod("GetCardsInPlay", BindingFlags.NonPublic | BindingFlags.Static, [typeof(Rule)])!;
    foreach (var r in Enum.GetValues<Rule>()) byRule[((int)r).ToString()] = Ids((IEnumerable<TreacheryCard>)getByRule.Invoke(null, [r])!);

    var lookup = new JsonArray();
    var get = new JsonArray();
    for (var i = -3; i <= 105; i++)
    {
        lookup.Add(Try(() => Id(TreacheryCardManager.Lookup.Find(i))));
        get.Add(Try(() => Id(TreacheryCardManager.Get(i))));
    }

    return new JsonObject
    {
        ["items"] = items,
        ["countered_by_reps"] = Ids(reps),
        ["countered_by"] = countered,
        ["get_cards_in_play_by_rule"] = byRule,
        ["richese_cards"] = Ids(TreacheryCardManager.GetRicheseCards()),
        ["lookup"] = lookup,
        ["get"] = get,
        ["constants"] = new JsonArray(TreacheryCard.None, TreacheryCard.Unknown, TreacheryCardManager.CardBaliset, TreacheryCardManager.CardJubbaCloak, TreacheryCardManager.CardKulon, TreacheryCardManager.CardLalala, TreacheryCardManager.CardTripToGamont, TreacheryCardManager.CardKullWahad)
    };
}

static JsonObject DumpRandom()
{
    var shuffles = new JsonArray();
    foreach (var seed in new[] { 0, 1, 42, 123456, int.MaxValue })
    foreach (var n in new[] { 0, 1, 2, 5, 33 })
    {
        var random = new LoggedRandom(seed);
        var deck = new Deck<int>(Enumerable.Range(0, n), random);
        deck.Shuffle();
        deck.Shuffle();
        var pick = Enumerable.Range(0, n).RandomOrDefault(random);
        shuffles.Add(new JsonObject
        {
            ["seed"] = seed,
            ["n"] = n,
            ["items"] = new JsonArray(deck.Items.Select(i => (JsonNode?)i).ToArray()),
            ["random_or_default"] = pick,
            ["count"] = random.GetCount()
        });
    }
    return new JsonObject { ["shuffles"] = shuffles };
}
