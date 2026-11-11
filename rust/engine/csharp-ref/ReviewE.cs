// "review-e" mode: probes for the package E (bidding) review. Each line is
// "name = value"; tests/review_pkg_e.rs hardcodes the values.
//
//   dotnet run -c Release -- review-e

using Treachery.Shared;

public static class ReviewE
{
    private static string Try(Func<object?> f)
    {
        try
        {
            var r = f();
            return r == null ? "null" : r.ToString()!;
        }
        catch (Exception e)
        {
            return "exception:" + e.GetType().Name + ":" + e.Message.Replace("\n", "\\n");
        }
    }

    private static void P(string name, Func<object?> f) => Console.WriteLine($"{name} = {Try(f)}");

    public static void Run()
    {
        // EmperorBidSupport.GetMessage holds a lazy Where/Select over the event's dictionary.
        var e = new EmperorBidSupport { Initiator = Faction.Emperor, Amounts = new Dictionary<Faction, int> { [Faction.Atreides] = 3, [Faction.Harkonnen] = 0, [Faction.Fremen] = 2 } };
        P("emperor_bid_support.message", () => e.GetMessage().ToString());

        var none = new EmperorBidSupport { Initiator = Faction.Emperor, Amounts = new Dictionary<Faction, int> { [Faction.Atreides] = 0 } };
        P("emperor_bid_support.message_none", () => none.GetMessage().ToString());

        // The silent-auction log line of Bid.HandleRicheseBid / BlackMarketBid: a List<MessagePart> element.
        var parts = new List<MessagePart>
        {
            MessagePart.Express(Faction.Atreides, Payment.Of(3), " "),
            MessagePart.Express(Faction.Harkonnen, Payment.Of(0), " ")
        };
        P("silent_bids.richese", () => Message.Express("Bids: ", parts).ToString());
        P("silent_bids.black_market", () => Message.Express(parts).ToString());

        // Bureaucracy logs its dynamic message as the only element of a new message.
        // EmperorBidSupport.Amounts is a Dictionary<Faction,int> read from JSON. Two keys that name the same faction.
        foreach (var (name, json) in new[] {
            ("dup_case", """{"$type":"EmperorBidSupport","Amounts":{"Atreides":1,"Harkonnen":5,"atreides":2},"Initiator":10}"""),
            ("dup_number", """{"$type":"EmperorBidSupport","Amounts":{"Atreides":1,"Harkonnen":5,"20":2},"Initiator":10}"""),
            ("dup_exact", """{"$type":"EmperorBidSupport","Amounts":{"Atreides":1,"Harkonnen":5,"Atreides":2},"Initiator":10}""") })
        {
            P("emperor_bid_support.json." + name, () =>
            {
                var ev = (EmperorBidSupport)Utilities.Deserialize<GameEvent>(json)!;
                return string.Join(",", ev.Amounts.Select(kv => $"{kv.Key}={kv.Value}"));
            });
        }

        P("bureaucracy.nested", () => Message.Express(Message.Express(Faction.Atreides, " apply Bureaucracy → ", Faction.Emperor, " lose ", Payment.Of(2))).ToString());
    }
}
