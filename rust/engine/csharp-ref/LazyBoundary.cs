// The "lazy" section of boundary.json: every lazy IEnumerable producer in
// Shared that takes a Game or a Player (rust/docs/lazy-queries.md), called on
// game states whose collections hold a null or throwing element after an
// element that matches.
//
// For each call it records the items the query yields, in order, up to the
// first exception, and that exception. A consumer that stops early (Any,
// First, Contains) sees only a prefix of this, so a port that builds the
// whole list first shows up as a missing prefix: it throws before yielding
// anything. tests/boundary.rs (lazy_queries_yield_like_csharp) rebuilds each
// state from the "scenarios" spec below and compares.
//
// Scenarios, applied to a fresh six-player game:
//   hands      every hand is [Karama, Useless, Mercenary, a Richese card, Distrans, null, Karama];
//              traitors and face dancers are [a leader, null].
//   ally       hands, plus forces on the planet, plus every player allied to Harkonnen,
//              which nobody plays (HasAlly is true, AlliedPlayer is null).
//   discovery  ally, plus two just-revealed discovery strongholds: the first is
//              attached next to forces, the second is not attached.
// Each producer is called for every player and for a null player.

using System.Text.Json.Nodes;
using Treachery.Shared;

public static class LazyBoundary
{
    // Six players, seeded like ReviewJ: Fremen, Atreides, Choam, Richese, Ecaz, Tleilaxu.
    public const string Establish = """{"$type":"EstablishPlayers","Settings":{"NumberOfPlayers":6,"MaximumTurns":8,"InitialRules":[301,300,65,60,20,1013,15,700,105,80,85,604,90,998,30,701,40,107,10,70,50,25,209,999,112],"AllowedFactionsInPlay":[10,20,90,100,110,80],"AutoOpenEmptySeats":true,"AsyncPlay":false,"AsyncPlayMessageIntervalMinutes":0},"Seed":1073548723,"MaximumNumberOfPlayers":6,"MaximumTurns":8,"ApplicableRules":[301,300,65,60,20,1013,15,700,105,80,85,604,90,998,30,701,40,107,10,70,50,25,209,999,112],"_players":"","_factionsInPlay":"Fremen,Atreides,Choam,Richese,Ecaz,Tleilaxu","Initiator":0,"Time":"2024-01-01T12:02:17+00:00"}""";

    private static Game NewGame()
    {
        var p = new Participation();
        p.PlayerNames[100] = "Human100";
        p.SeatedPlayers[100] = 2;
        var g = new Game(187, p);
        var e = Utilities.Deserialize<GameEvent>(Establish)!;
        e.Initialize(g);
        var result = e.Execute(true, true);
        if (result != null) throw new Exception("establish rejected: " + result);
        return g;
    }

    private static JsonArray Ids(IEnumerable<IIdentifiable?> items) => new(items.Select(i => (JsonNode?)(i == null ? null : JsonValue.Create(i.Id))).ToArray());

    // The spec Rust rebuilds: scenario name -> what to change on a fresh game.
    private static JsonObject Spec(Game g, string name)
    {
        var cards = TreacheryCardManager.GetCardsInAndOutsidePlay();
        var karama = cards.First(c => c.Type == TreacheryCardType.Karma);
        var useless = cards.First(c => c.Type == TreacheryCardType.Useless);
        var mercenary = cards.First(c => c.Type == TreacheryCardType.Mercenary);
        var richese = cards.First(c => c.Rules.Contains(Rule.RicheseTreacheryCards));
        var distrans = cards.First(c => c.Type == TreacheryCardType.Distrans);
        var spec = new JsonObject
        {
            ["name"] = name,
            ["hand"] = Ids([karama, useless, mercenary, richese, distrans, null, karama]),
            // Every player gets the first leader of its own faction, then null.
            ["heroes"] = new JsonArray(g.Players.Select(p => (JsonNode?)Ids([LeaderManager.GetLeaders(p.Faction).First(), null])).ToArray())
        };
        if (name is "ally" or "discovery")
        {
            spec["ally"] = (int)Faction.Harkonnen;
            var locations = g.Map.Locations(false).Where(l => !l.IsStronghold && l is not AttachedLocation).ToList();
            var forces = new JsonArray();
            for (var i = 0; i < g.Players.Count; i++)
            foreach (var k in new[] { 3, 17, 29 })
                forces.Add(new JsonArray(i, locations[(k + 5 * i) % locations.Count].Id, 2));
            spec["forces"] = forces;
        }
        if (name == "discovery")
        {
            // Every player has forces where the first stronghold is attached, so it matches and the
            // second one, unattached, is reached.
            var target = g.Map.Locations(false).Where(l => !l.IsStronghold && l is not AttachedLocation).ToList()[3];
            spec["discovery"] = new JsonArray(new JsonArray(g.Map.Shrine.Id, target.Id), new JsonArray(g.Map.Cistern.Id, null));
            var forces = spec["forces"]!.AsArray();
            for (var i = 0; i < g.Players.Count; i++) forces.Add(new JsonArray(i, target.Id, 1));
        }
        return spec;
    }

    private static Game Build(JsonObject spec)
    {
        var g = NewGame();
        var hand = spec["hand"]!.AsArray().Select(n => n == null ? null : TreacheryCardManager.Lookup.Find((int)n)).ToList();
        var heroes = spec["heroes"]!.AsArray();
        for (var i = 0; i < g.Players.Count; i++)
        {
            var p = g.Players[i];
            p.TreacheryCards.Clear();
            p.TreacheryCards.AddRange(hand!);
            var h = heroes[i]!.AsArray().Select(n => n == null ? null : LeaderManager.HeroLookup.Find((int)n)).ToList();
            p.Traitors.Clear();
            p.Traitors.AddRange(h!);
            p.FaceDancers.Clear();
            p.FaceDancers.AddRange(h!);
            if (spec["ally"] is { } ally) p.Ally = (Faction)(int)ally;
        }
        if (spec["forces"] is JsonArray forces)
            foreach (var f in forces)
            {
                var a = f!.AsArray();
                g.Players[(int)a[0]!].AddForces(g.Map.LocationLookup.Find((int)a[1]!), (int)a[2]!, false);
            }
        if (spec["discovery"] is JsonArray discovery)
            foreach (var d in discovery)
            {
                var a = d!.AsArray();
                var ds = (DiscoveredLocation)g.Map.LocationLookup.Find((int)a[0]!);
                if (a[1] != null) ds.PointAt(g, g.Map.LocationLookup.Find((int)a[1]!));
                g.JustRevealedDiscoveryStrongholds.Add(ds);
            }
        return g;
    }

    // The producers, by the name lazy-queries.md uses (Type.Member).
    private static readonly (string Name, Func<Game, Player?, System.Collections.IEnumerable> Call)[] Producers =
    [
        ("AllianceOffered.ValidTargets", (g, p) => AllianceOffered.ValidTargets(g, p!)),
        ("AmbassadorActivated.ValidFremenTargets", (g, p) => AmbassadorActivated.ValidFremenTargets(g, p!)),
        ("AmbassadorActivated.ValidGuildTargets", (g, p) => AmbassadorActivated.ValidGuildTargets(g, p!)),
        ("AmbassadorPlaced.ValidStrongholds", (g, p) => AmbassadorPlaced.ValidStrongholds(g, p!)),
        ("BattleConcluded.ValidTraitorsToReplace", (g, p) => BattleConcluded.ValidTraitorsToReplace(p!)),
        ("Bid.ValidKarmaCards", (g, p) => Bid.ValidKarmaCards(g, p)),
        ("BeneGesseritBattleAnnouncement.ValidTerritories", (g, p) => BeneGesseritBattleAnnouncement.ValidTerritories(g, p!)),
        ("BeneGesseritPrediction.ValidTargets", (g, p) => BeneGesseritPrediction.ValidTargets(g, p!)),
        ("ChoamDiscarded.ValidCards", (g, p) => ChoamDiscarded.ValidCards(p!)),
        ("ChoamMovePrevention.ValidTerritories", (g, p) => ChoamMovePrevention.ValidTerritories(p!)),
        ("ChoamRemoveForce.ValidLocations", (g, p) => ChoamRemoveForce.ValidLocations(g)),
        ("ChoamRemoveForce.ValidFactions", (g, p) => ChoamRemoveForce.ValidFactions(g, p!, null)),
        ("ClairVoyancePlayed.ValidTargets", (g, p) => ClairVoyancePlayed.ValidTargets(g, p!)),
        ("DealAccepted.AcceptableDeals", (g, p) => DealAccepted.AcceptableDeals(g, p)),
        ("DealAccepted.CancellableDeals", (g, p) => DealAccepted.CancellableDeals(g, p)),
        ("DiscardedTaken.ValidCards", (g, p) => DiscardedTaken.ValidCards(g, p!)),
        ("DiscoveryEntered.ValidTargets", (g, p) => DiscoveryEntered.ValidTargets(g, p!)),
        ("DiscoveryRevealed.GetLocations", (g, p) => DiscoveryRevealed.GetLocations(g, p!)),
        ("DistransUsed.ValidCards", (g, p) => DistransUsed.ValidCards(p!)),
        ("DistransUsed.ValidTargets", (g, p) => DistransUsed.ValidTargets(g, p!)),
        ("Donated.ValidTargets", (g, p) => Donated.ValidTargets(g, p!)),
        ("FaceDanced.ValidSourceLocations", (g, p) => FaceDanced.ValidSourceLocations(g, p!)),
        ("FaceDanced.ValidTargetLocations", (g, p) => FaceDanced.ValidTargetLocations(g)),
        ("FaceDancerReplaced.ValidFaceDancers", (g, p) => FaceDancerReplaced.ValidFaceDancers(p!)),
        ("FactionSelected.ValidFactions", (g, p) => FactionSelected.ValidFactions(g)),
        ("FactionTradeOffered.ValidTargets", (g, p) => FactionTradeOffered.ValidTargets(g, p!)),
        ("Karma.ValidKarmaCards", (g, p) => Karma.ValidKarmaCards(g, p!)),
        ("KarmaHmsMovement.ValidLocations", (g, p) => KarmaHmsMovement.ValidLocations(g)),
        ("KarmaMonster.ValidTargets", (g, p) => KarmaMonster.ValidTargets(g)),
        ("KarmaRevivalPrevention.GetValidTargets", (g, p) => KarmaRevivalPrevention.GetValidTargets(g, p!)),
        ("KarmaShipmentPrevention.GetValidTargets", (g, p) => KarmaShipmentPrevention.GetValidTargets(g, p!)),
        ("NexusPlayed.ValidChoamCards", (g, p) => NexusPlayed.ValidChoamCards(p!)),
        ("NexusPlayed.ValidEcazTerritories", (g, p) => NexusPlayed.ValidEcazTerritories(g)),
        ("NexusPlayed.ValidEcazFactions", (g, p) => NexusPlayed.ValidEcazFactions(g)),
        ("NexusPlayed.ValidMoritaniTerritories", (g, p) => NexusPlayed.ValidMoritaniTerritories(g)),
        ("PerformBeneGesseritPlacement.ValidLocations", (g, p) => PerformBeneGesseritPlacement.ValidLocations(g)),
        ("PerformMoritaniSetup.ValidLocations", (g, p) => PerformMoritaniSetup.ValidLocations(g)),
        ("PerformHmsMovement.ValidLocations", (g, p) => PerformHmsMovement.ValidLocations(g)),
        ("PerformHmsPlacement.ValidLocations", (g, p) => PerformHmsPlacement.ValidLocations(g, p!)),
        ("PlayerReplaced.ValidTargets", (g, p) => PlayerReplaced.ValidTargets(g)),
        ("RequestTleilaxuRevival.ValidTargets", (g, p) => RequestTleilaxuRevival.ValidTargets(g, p!)),
        ("ResourcesAudited.ValidFactions", (g, p) => ResourcesAudited.ValidFactions(g, p!)),
        ("Retreat.ValidTargets", (g, p) => Retreat.ValidTargets(g, p!)),
        ("Revival.ValidRevivalHeroes", (g, p) => Revival.ValidRevivalHeroes(g, p!)),
        ("Revival.ValidRevivedForceLocations", (g, p) => Revival.ValidRevivedForceLocations(g, p!)),
        ("SetIncreasedRevivalLimits.ValidTargets", (g, p) => SetIncreasedRevivalLimits.ValidTargets(g, p!)),
        ("SetShipmentPermission.ValidTargets", (g, p) => SetShipmentPermission.ValidTargets(g, p!)),
        ("Shipment.ValidShipmentLocations", (g, p) => Shipment.ValidShipmentLocations(g, p!, false, false)),
        ("Shipment.ValidSourceLocations", (g, p) => Shipment.ValidSourceLocations(g, p!)),
        ("TerrorPlanted.ValidStrongholds", (g, p) => TerrorPlanted.ValidStrongholds(g, p!)),
        ("TerrorPlanted.ValidTerrorTypes", (g, p) => TerrorPlanted.ValidTerrorTypes(g, false)),
        ("TerrorRevealed.GetTypes", (g, p) => TerrorRevealed.GetTypes(g)),
        ("TerrorRevealed.ValidSneakAttackTargets", (g, p) => TerrorRevealed.ValidSneakAttackTargets(g, p!)),
        ("Thought.ValidCards", (g, p) => Thought.ValidCards(g)),
        ("RicheseGaveCard.ValidCards", (g, p) => RicheseGaveCard.ValidCards(p!)),
        ("FremenRidesMonster.ValidSources", (g, p) => FremenRidesMonster.ValidSources(g)),
        ("FremenRidesMonster.ValidTargets", (g, p) => FremenRidesMonster.ValidTargets(g, p!)),
        ("FremenSentMonster.ValidTargets", (g, p) => FremenSentMonster.ValidTargets(g)),
        ("Game.LocationsWithAnyForcesNotInStorm", (g, p) => g.LocationsWithAnyForcesNotInStorm(p!)),
        ("Game.GetSkilledLeaders", (g, p) => g.GetSkilledLeaders(p!)),
        ("Game.PlayersOtherThan", (g, p) => g.PlayersOtherThan(p!)),
        ("Map.GetResourceCardsInPlay", (g, p) => Map.GetResourceCardsInPlay(g)),
        ("Player.TerritoriesWithForces", (g, p) => p!.TerritoriesWithForces),
        ("Player.UnrevealedTraitors", (g, p) => p!.UnrevealedTraitors),
        ("Player.UnrevealedFaceDancers", (g, p) => p!.UnrevealedFaceDancers),
        ("TreacheryCardManager.GetCardsInPlay", (g, p) => TreacheryCardManager.GetCardsInPlay(g)),
    ];

    // Enc.Value, except for resource cards: GetResourceCardsInPlay makes new card objects on
    // every call, so they can't be found by reference. They are written by their data.
    private static JsonNode? Item(object? item) => item is ResourceCard rc
        ? JsonValue.Create($"rcd:{rc.SkinId},{rc.Location?.Id},{rc.DiscoveryLocation?.Id},{rc.IsSandTrout},{rc.IsGreatMaker}")
        : Enc.Value(item);

    public static JsonObject Dump()
    {
        var scenarios = new JsonArray();
        var cases = new JsonArray();
        foreach (var name in new[] { "hands", "ally", "discovery" })
        {
            var spec = Spec(NewGame(), name);
            scenarios.Add(spec);
            var g = Build(spec);
            var players = g.Players.Cast<Player?>().Append(null).ToList();
            foreach (var (producer, call) in Producers)
            foreach (var p in players)
            {
                var items = new JsonArray();
                JsonNode? exception = null;
                try
                {
                    foreach (var item in call(g, p)) items.Add(Item(item));
                }
                catch (Exception e)
                {
                    exception = Enc.Exception(e);
                }
                cases.Add(new JsonArray(name, producer, p == null ? null : (int)p.Faction, items, exception));
            }
        }
        return new JsonObject
        {
            ["establish"] = Establish,
            ["producers"] = new JsonArray(Producers.Select(p => (JsonNode?)p.Name).ToArray()),
            ["scenarios"] = scenarios,
            ["cases"] = cases
        };
    }
}
