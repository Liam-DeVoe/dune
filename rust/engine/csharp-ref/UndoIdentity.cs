// "undo-identity" mode: what C# does with game identity around Undo, RestoreRecentlyUndone and
// DismissRecentlyUndone. Writes tests/golden/undo_identity.json: name -> value, where dumps are
// the oracle's canonical state dump (Oracle/Canon/StateDumper.cs). tests/undo_identity.rs runs
// the same scenarios in Rust and compares.
//
//   dotnet run -c Release -- undo-identity ../tests/golden/undo_identity.json
//
// The "site" scenarios put an event that belongs to another game into the restored game's
// state (as Undo does to every replayed history event) and call the code listed in
// rust/docs/reviews/pkg-a.md item 1: reference comparisons with the event's Player, and writes
// that go to that Player.

using System.Reflection;
using System.Text;
using System.Text.Json.Nodes;
using Treachery.Oracle.Canon;
using Treachery.Shared;

public static class UndoIdentity
{
    // Step 0 of fuzz seed 100 game 64: eight players including BeneGesserit, with BeneGesseritAdvisors.
    public const string BeneGesseritEstablish =
        """{"$type":"EstablishPlayers","Settings":{"NumberOfPlayers":8,"MaximumTurns":5,"InitialRules":[209,1013,600,110,998,300,10000,10001,212,70],"AllowedFactionsInPlay":[60,100,50,120,80,40,110,90,20,10],"AutoOpenEmptySeats":true,"AsyncPlay":false,"AsyncPlayMessageIntervalMinutes":0},"Seed":1450467322,"MaximumNumberOfPlayers":8,"MaximumTurns":5,"ApplicableRules":[209,1013,600,110,998,300,10000,10001,212,70],"_players":"","_factionsInPlay":"BeneGesserit,Richese,Guild,Moritani,Tleilaxu,Emperor,Ecaz,Choam,Atreides,Fremen","Initiator":0,"Time":"2024-01-01T12:00:55+00:00"}""";

    private static readonly SortedDictionary<string, string> Out = new(StringComparer.Ordinal);

    private static void P(string name, object? v) => Out[name] = v switch
    {
        null => "null",
        bool b => b ? "true" : "false",
        _ => v.ToString()!,
    };

    private static string Dump(Game g)
    {
        var o = new JsonOut();
        new StateDumper(o).DumpGame(g);
        return Encoding.UTF8.GetString(o.WrittenSpan);
    }

    // As the oracle runs a step: Initialize, then Execute(validate: true, isHost), keeping
    // whatever an escaping exception left behind.
    private static Game Run(Game g, IEnumerable<(string Json, bool IsHost)> events)
    {
        foreach (var (json, isHost) in events)
        {
            var e = Utilities.Deserialize<GameEvent>(json)!;
            e.Initialize(g);
            try
            {
                e.Execute(true, isHost);
            }
            catch (Exception)
            {
                // EstablishPlayers overrides Execute without a try/catch.
            }
        }
        return g;
    }

    private static Participation ReviewParticipation()
    {
        var p = new Participation();
        p.PlayerNames[100] = "Human100";
        p.SeatedPlayers[100] = 2;
        return p;
    }

    // The two events of ReviewA (3 players, version 187).
    private static Game Base() => Run(new Game(Game.LatestVersion, ReviewParticipation()), [(ReviewA.Establish, true), (ReviewA.EndPhase, true)]);

    private static void Set(object target, string property, object? value) =>
        target.GetType().GetProperty(property, BindingFlags.Instance | BindingFlags.Public | BindingFlags.NonPublic)!.SetValue(target, value);

    private static object? Call(object target, string method, params object?[] args) =>
        target.GetType().GetMethod(method, BindingFlags.Instance | BindingFlags.Public | BindingFlags.NonPublic)!.Invoke(target, args);

    private static TreacheryCard FirstCard(TreacheryCardType t) => TreacheryCardManager.Items.First(c => c.Type == t);

    public static void Run(string path)
    {
        // Undo, restore and dismiss on the ReviewA game.
        {
            var g = Base();
            var u = g.Undo(1);
            P("undo.dump", Dump(u));
            var r = u.RestoreRecentlyUndone();
            P("restore.returns_old", ReferenceEquals(r, g));
            P("restore.dump", Dump(r));
        }
        {
            var u = Base().Undo(1);
            P("dismiss.dump", Dump(u.DismissRecentlyUndone()));
        }
        {
            // The replaying branch of RestoreRecentlyUndone (PreviousGameState is null).
            var u = Base().Undo(1);
            Set(u, "PreviousGameState", null);
            P("restore_replay.dump", Dump(u.RestoreRecentlyUndone()));
        }
        {
            // Two undos, then two restores: each restore returns the game before.
            var g = Base();
            var u1 = g.Undo(2);
            var u2 = u1.Undo(1);
            P("undo2.dump", Dump(u2));
            var r1 = u2.RestoreRecentlyUndone();
            P("undo2.restore1.is_u1", ReferenceEquals(r1, u1));
            P("undo2.restore1.dump", Dump(r1));
            var r2 = r1.RestoreRecentlyUndone();
            P("undo2.restore2.is_g", ReferenceEquals(r2, g));
            P("undo2.restore2.dump", Dump(r2));
        }

        // pkg-a item 1: code that reads .Player of an event the restored game holds.
        {
            var g = Base();
            var u = g.Undo(1);
            var r = u.RestoreRecentlyUndone();
            var f = r.Players.First(x => x.Faction != Faction.Fremen && x.Ally == Faction.None).Faction;
            var p = r.GetPlayer(f)!;

            // Game.cs:977 HasOrnithopters: CurrentFlightUsed.Player == p
            Set(r, "CurrentFlightUsed", new FlightUsed(u, f) { MoveThreeTerritories = true });
            P("site.ornithopters.foreign", r.HasOrnithopters(p));
            Set(r, "CurrentFlightUsed", new FlightUsed(r, f) { MoveThreeTerritories = true });
            P("site.ornithopters.own", r.HasOrnithopters(p));
            Set(r, "CurrentFlightUsed", null);

            // Game_Blow.cs:508 ProtectedFromMonster and Game_Revival.cs:80 FreeRevivals:
            // CurrentFremenSecretAlly.Player == p
            Set(r, "CurrentFremenSecretAlly", new NexusPlayed(u, f));
            P("site.protected_from_monster.foreign", r.ProtectedFromMonster(p));
            P("site.free_revivals.foreign", r.FreeRevivals(p, false));
            Set(r, "CurrentFremenSecretAlly", new NexusPlayed(r, f));
            P("site.protected_from_monster.own", r.ProtectedFromMonster(p));
            P("site.free_revivals.own", r.FreeRevivals(p, false));
            Set(r, "CurrentFremenSecretAlly", null);

            // Game_Bidding.cs:636 GetCardSetAsideForBid, and Game_Bidding.cs:178
            // ReturnKarmaCardUsedForBid, which gives the card to CurrentBid.Player.
            var karma = FirstCard(TreacheryCardType.Karma);
            Set(r, "CurrentBid", new Bid(r, f) { KarmaCard = karma });
            P("site.card_set_aside.own", r.GetCardSetAsideForBid(p)?.Id);
            Set(r, "CurrentBid", new Bid(u, f) { KarmaCard = karma });
            P("site.card_set_aside.foreign", r.GetCardSetAsideForBid(p)?.Id);
            Call(r, "ReturnKarmaCardUsedForBid");
            P("site.return_karma.dump", Dump(r));
            P("site.return_karma.own_has_card", p.TreacheryCards.Contains(karma));
            Set(r, "CurrentBid", null);

            // Members of a stored BattleInitiated read its own Game (the undo game):
            // AggressivePlayer is the undo game's player, PlanOf reads its AggressorPlan.
            var other = r.Players.First(x => x.Faction != f).Faction;
            Set(r, "AggressorPlan", new Battle(r, f));
            Set(r, "CurrentBattle", new BattleInitiated(u, f) { Target = other, Territory = u.Map.Arrakeen.Territory });
            P("site.battle.territory_is_this_maps", ReferenceEquals(r.CurrentBattle!.Territory, r.Map.Arrakeen.Territory));
            P("site.battle.aggressive_player_is_undo_games", u.Players.Contains(r.CurrentBattle!.AggressivePlayer!));
            P("site.battle.plan_of_is_null", r.CurrentBattle!.PlanOf(p) == null);
            P("site.battle.dump", Dump(r));
            Set(r, "CurrentBattle", null);
            Set(r, "AggressorPlan", null);

            // Game_Battle.cs:220-228 HandleReinforcements writes to plan.Player.
            var plan = new Battle(u, f) { Weapon = FirstCard(TreacheryCardType.Reinforcements) };
            Set(r, "AggressorPlan", plan);
            Call(r, "HandleReinforcements", plan);
            P("site.reinforcements.dump", Dump(r));
        }

        // Map objects of another game handed back to this one.
        {
            var bg = Base();
            var bu = bg.Undo(1);
            var br = bu.RestoreRecentlyUndone();
            var up = bu.Players[0];
            Call(up, "AddForces", bu.Map.Arrakeen, 3, false);
            var l = up.ForcesInLocations.Keys.First();
            P("handed_back.territory_is_this_maps", ReferenceEquals(l.Territory, br.Map.TerritoryLookup.Find(l.Territory.Id)));
            var leader = LeaderManager.Leaders[0];
            br.LeaderState[leader].CurrentTerritory = l.Territory;
            P("handed_back.dump", Dump(br));

        }

        // Members of stored events that read their own Game, after a restore.
        {
            var g = Base();
            var u = g.Undo(1);
            var r = u.RestoreRecentlyUndone();
            var f = r.Players.First(x => x.Faction != Faction.Fremen && x.Ally == Faction.None).Faction;
            var other = r.Players.First(x => x.Faction != f).Faction;

            // ClairVoyancePlayed.Parameter1 reads Game.Map for WillAttackX.
            var arrakeen = r.Map.Arrakeen.Territory;
            var cv = new ClairVoyancePlayed(u, f) { Question = ClairvoyanceQuestion.WillAttackX, QuestionParameter1 = arrakeen.Id.ToString() };
            P("event_game.parameter1_is_this_maps", ReferenceEquals(cv.Parameter1, arrakeen));

            // DealOffered.Acceptance/Rejection build the answer for the offer's Game.
            var offer = new DealOffered(u, f);
            P("event_game.acceptance_game_is_undo", ReferenceEquals(offer.Acceptance(other).Game, u));
            P("event_game.rejection_game_is_undo", ReferenceEquals(offer.Rejection(other).Game, u));
            P("event_game.acceptance_player_is_undo_games", u.Players.Contains(offer.Acceptance(other).Player));

            // Battle.ActivateDynamicWeapons logs to, and reads, the plan's own game.
            var plan = new Battle(u, f) { Weapon = FirstCard(TreacheryCardType.MirrorWeapon) };
            Set(r, "AggressorPlan", plan);
            Call(plan, "ActivateDynamicWeapons", LeaderManager.Leaders[0], FirstCard(TreacheryCardType.Projectile), null);
            P("event_game.dynamic_weapons.dump", Dump(r));
            P("event_game.dynamic_weapons.undo_report_messages", u.CurrentReport.Messages.Count);
            Set(r, "AggressorPlan", null);
        }

        // GameStatus reads LastShipmentOrMovement.To through the event's own Game
        // (DetermineHighlights and the BeneGesserit intrusion messages).
        {
            var g = Base();
            var u = g.Undo(1);
            var r = u.RestoreRecentlyUndone();
            var f = r.Players.First(x => x.Faction != Faction.Fremen && x.Ally == Faction.None).Faction;
            Set(r, "LastShipmentOrMovement", new Move(u, f) { _toId = r.Map.Arrakeen.Id });
            foreach (var phase in new[] { Phase.BeneGesseritIntrudedByFremenRidingMonsterA, Phase.BeneGesseritIntrudedByNonGuildMove })
            {
                Set(r, "CurrentPhase", phase);
                var o = new JsonOut();
                new StateDumper(o).DumpDetached(GameStatus.DetermineStatus(r, r.Players[0], true), r);
                P($"status.{phase}", Encoding.UTF8.GetString(o.WrittenSpan));
            }
        }

        // BeneGesseritFlip after an undo: Player.FlipForces(Territory) gets a territory of the undo
        // game's map (LastShipmentOrMovement.To read through the event's own Game). Where
        // the flip empties a battalion, ChangeForces removes it and ChangeSpecialForces
        // creates a new one keyed by the other map's location; where it doesn't, the
        // existing key of this map stays. Fuzz seed 100 game 64, step 561.
        {
            var g = Run(new Game(Game.LatestVersion, new Participation()), [(BeneGesseritEstablish, true), (ReviewA.EndPhase, true)]);
            var u = g.Undo(1);
            var r = u.RestoreRecentlyUndone();
            var bene_gesserit = r.GetPlayer(Faction.BeneGesserit)!;
            bene_gesserit.AddForces(r.Map.Arrakeen, 2, false);
            bene_gesserit.FlipForces(u.Map.Arrakeen.Territory, true);
            bene_gesserit.AddForces(r.Map.Carthag, 1, false);
            bene_gesserit.AddSpecialForces(r.Map.Carthag, 1, false);
            bene_gesserit.FlipForces(u.Map.Carthag.Territory, true);
            // The other Player methods that take a Territory and change forces.
            bene_gesserit.KillForces(u.Map.Carthag.Territory, 1, true, false);
            bene_gesserit.AddForces(r.Map.TueksSietch, 3, false);
            bene_gesserit.ForcesToReserves(u.Map.TueksSietch.Territory, 1, false);
            bene_gesserit.KillAllForces(u.Map.TueksSietch.Territory, true);
            bene_gesserit.AddForces(r.Map.SietchTabr, 2, false);
            bene_gesserit.ForcesToReserves(u.Map.SietchTabr.Territory);
            bene_gesserit.AddForces(r.Map.HabbanyaSietch, 2, false);
            bene_gesserit.KillForces(u.Map.HabbanyaSietch.Territory, 2, false, true);
            P("bene_gesserit_flip.arrakeen_key_is_this_maps", bene_gesserit.ForcesInLocations.Keys.Any(l => ReferenceEquals(l, r.Map.Arrakeen)));
            P("bene_gesserit_flip.carthag_key_is_this_maps", bene_gesserit.ForcesInLocations.Keys.Any(l => ReferenceEquals(l, r.Map.Carthag)));
            P("bene_gesserit_flip.dump", Dump(r));
        }

        // An exception out of Undo leaves the caller with the old game.
        {
            var g = Base();
            try
            {
                g = g.Undo(-1);
                P("undo_throws.exception", "none");
            }
            catch (Exception e)
            {
                P("undo_throws.exception", e.GetType().Name + ": " + e.Message);
            }
            P("undo_throws.dump", Dump(g));
        }

        // pkg-a item 2: Participation is shared, not copied.
        {
            var part = new Participation();
            var g = new Game(Game.LatestVersion, part);
            part.Hosts.Add(7);
            P("participation.caller_alias.is_host7", g.IsHost(7));
            var u = g.Undo(0);
            u.AddPlayer(5, "Late", 1);
            var r = u.RestoreRecentlyUndone();
            P("participation.restore.is_participant5", r.IsParticipant(5));
            P("participation.restore.dump", Dump(r));
            var d = r.DismissRecentlyUndone();
            d.SetOrUnsetHost(9);
            P("participation.dismiss.caller_sees_host9", part.Hosts.Contains(9));
        }

        var root = new JsonObject();
        foreach (var (k, v) in Out) root[k] = v;
        File.WriteAllText(path, root.ToJsonString(new System.Text.Json.JsonSerializerOptions { WriteIndented = true }) + "\n");
        Console.WriteLine($"wrote {Out.Count} values to {path}");
    }
}
