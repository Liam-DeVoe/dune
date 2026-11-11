// "review-k" mode: probes for the package K review (alliances, deals, discards,
// status). Each line is "name = value"; tests/review_pkg_k.rs hardcodes the values.
//
//   dotnet run -c Release -- review-k
//
// The game is ReviewJ.Establish: Choam, Atreides and Richese, all seated.

public static class ReviewK
{
    private static void P(string name, object? v) => Console.WriteLine($"{name} = {v ?? "null"}");

    private static Game NewGame()
    {
        var p = new Participation();
        p.PlayerNames[100] = "Human100";
        p.SeatedPlayers[100] = 2;
        var g = new Game(187, p);
        var e = Utilities.Deserialize<GameEvent>(ReviewJ.Establish)!;
        e.Initialize(g);
        if (e.Execute(true, true) != null) throw new Exception("establish rejected");
        return g;
    }

    private static GameEvent Ev(Game g, string json)
    {
        var e = Utilities.Deserialize<GameEvent>(json)!;
        e.Initialize(g);
        return e;
    }

    private static void Try(string name, Func<object?> f)
    {
        try { P(name, f()); }
        catch (Exception ex) { P(name + ".exception", ex.GetType().Name + ": " + ex.Message); }
    }

    public static void Run()
    {
        var useless = TreacheryCardManager.GetCardsInAndOutsidePlay().First(c => c.Type == TreacheryCardType.Useless);
        P("useless.id", useless.Id);

        // 1. CardTraded.Validate: ValidCards(Player) is the hand itself, so
        //    Enumerable.Contains goes to List<T>.Contains, which finds a null Card
        //    when the hand holds a null.
        {
            var g = NewGame();
            var choam = g.GetPlayer(Faction.Choam)!;
            var atreides = g.GetPlayer(Faction.Atreides)!;
            choam.Ally = Faction.Atreides;
            atreides.Ally = Faction.Choam;
            choam.TreacheryCards.Clear();
            choam.TreacheryCards.Add(useless);
            choam.TreacheryCards.Add(null!);
            atreides.TreacheryCards.Clear();
            atreides.TreacheryCards.Add(useless);
            var e = Ev(g, """{"$type":"CardTraded","Target":20,"_cardId":-1,"_requestedCardId":-1,"Initiator":90,"Time":"2024-01-01T12:02:40+00:00"}""");
            Try("card_traded_null.validate", () => e.Validate()?.ToString() ?? "valid");
        }

        // 2. DiscardedSearched.Validate: the same List<T>.Contains(null) on the discard pile.
        {
            var g = NewGame();
            var choam = g.GetPlayer(Faction.Choam)!;
            g.TreacheryDiscardPile!.Items.Clear();
            g.TreacheryDiscardPile.Items.Add(null!);
            var e = Ev(g, """{"$type":"DiscardedSearched","_cardId":-1,"Initiator":90,"Time":"2024-01-01T12:02:40+00:00"}""");
            Try("discarded_searched_null.validate", () => e.Validate()?.ToString() ?? "valid");
        }

        // 3. DealOffered cancel with To = null. Same() calls To.SequenceEqual(To) on
        //    two Faction[]; C# 14 binds that to MemoryExtensions.SequenceEqual on spans,
        //    where a null array is an empty span, so nothing throws.
        {
            var g = NewGame();
            var offer = Ev(g, """{"$type":"DealOffered","To":[],"Type":0,"Text":"x","EndPhase":0,"Price":0,"Benefit":0,"Cancel":false,"Initiator":90,"Time":"2024-01-01T12:02:40+00:00"}""");
            P("deal_cancel_null_to.offer_result", offer.Execute(false, true)?.ToString() ?? "null");
            P("deal_cancel_null_to.offers_before", g.DealOffers.Count);
            var cancel = Ev(g, """{"$type":"DealOffered","To":null,"Type":0,"Text":"x","EndPhase":0,"Price":0,"Benefit":0,"Cancel":true,"Initiator":90,"Time":"2024-01-01T12:02:41+00:00"}""");
            Try("deal_cancel_null_to.same", () => g.DealOffers[0].Same((DealOffered)cancel));
            var r = cancel.Execute(false, true);
            P("deal_cancel_null_to.cancel_result", r?.ToString() ?? "null");
            P("deal_cancel_null_to.offers_after", g.DealOffers.Count);
            P("deal_cancel_null_to.history", g.History.Count);
        }
        // 4. MakeAlliance hand limit with null cards in the hand. RandomOrDefault can pick
        //    a null; C# Discard(player, null) does nothing and the loop draws again.
        {
            var g = NewGame();
            var atreides = g.GetPlayer(Faction.Atreides)!;
            var cards = TreacheryCardManager.GetCardsInAndOutsidePlay().Take(4).ToList();
            P("make_alliance_null.card_ids", string.Join(",", cards.Select(c => c.Id)));
            atreides.TreacheryCards.Clear();
            foreach (var c in cards) atreides.TreacheryCards.Add(c);
            for (int i = 0; i < 4; i++) atreides.TreacheryCards.Add(null!);
            var o1 = Ev(g, """{"$type":"AllianceOffered","Target":20,"Initiator":90,"Time":"2024-01-01T12:02:40+00:00"}""");
            P("make_alliance_null.offer_result", o1.Execute(false, true)?.ToString() ?? "null");
            var o2 = Ev(g, """{"$type":"AllianceOffered","Target":90,"Initiator":20,"Time":"2024-01-01T12:02:41+00:00"}""");
            P("make_alliance_null.accept_result", o2.Execute(false, true)?.ToString() ?? "null");
            P("make_alliance_null.atreides_hand", string.Join(",", atreides.TreacheryCards.Select(c => c == null ? -1 : c.Id)));
            P("make_alliance_null.atreides_ally", atreides.Ally);
            P("make_alliance_null.discard_pile", g.TreacheryDiscardPile!.Items.Count);
        }

        // 5. C# 14 binds extension calls on arrays to the MemoryExtensions span overloads,
        //    where a null array is an empty span. Checks the claim in pkg-k.md item 1 for
        //    Contains and for SequenceEqual in the other direction (offer To = null).
        {
            Faction[] nullArray = null!;
            Try("span_binding.contains_on_null_array", () => nullArray.Contains(Faction.Emperor));
            Try("span_binding.sequence_equal_null_null", () => nullArray.SequenceEqual(nullArray));
            var g = NewGame();
            var offer = Ev(g, """{"$type":"DealOffered","To":null,"Type":0,"Text":"x","EndPhase":0,"Price":0,"Benefit":0,"Cancel":false,"Initiator":90,"Time":"2024-01-01T12:02:40+00:00"}""");
            P("deal_offer_null_to.offer_result", offer.Execute(false, true)?.ToString() ?? "null");
            var cancel = Ev(g, """{"$type":"DealOffered","To":[],"Type":0,"Text":"x","EndPhase":0,"Price":0,"Benefit":0,"Cancel":true,"Initiator":90,"Time":"2024-01-01T12:02:41+00:00"}""");
            P("deal_offer_null_to.cancel_result", cancel.Execute(false, true)?.ToString() ?? "null");
            P("deal_offer_null_to.offers_after", g.DealOffers.Count);
        }
    }
}
